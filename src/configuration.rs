//! Configuration, era, membership, voting weights, and reconfiguration operations.
//!
//! Spec §8.7.1 (configuration), §8.7.2 (reconfiguration operations), §8.7.5 (closure of
//! weighted-majority configurations), §8.7.6 (pivot condition), §8.7.7 (non-stop
//! transition), §8.7.8 (primary failure during overlap mode); and
//! `docs/uvrr-reconfiguration-rules.md` §1–§4 (the command alphabet and its
//! boundaries, R1–R15), §9 (the snapshot and the operation WAL).
//!
//! A configuration is an era, an ordered member list, and a non-negative integer weight
//! per member. Primary selection is `config(era).order[index mod len(order)]`. Weight
//! zero grants no voting authority, so a newly joined member is a learner until a later
//! committed `Increment` promotes it — which is what makes state transfer a precondition
//! of authority rather than a courtesy.
//!
//! # The weight domain {0, 1, 2} (rules §1, R1)
//!
//! Voting weights are common factors: any uniform scaling is a zero effect on quorums,
//! so no node ever needs a weight above [`MAX_WEIGHT`] and no operation may produce any
//! other value — the fold refuses. A weight-0 member is a **learner**: it never votes
//! and is counted against no quorum (rules §2, R4), which is what makes `Join` and
//! `Leave` at weight zero safe — neither changes any quorum family, so neither needs an
//! overlap argument.
//!
//! # The operation alphabet, the batch, and the era rule (rules §3–§4, R7–R15)
//!
//! Reconfiguration operations (`VOID`, `INIT`, `JOIN`, `LEAVE`, `INCREMENT`,
//! `DECREMENT`, `DOUBLE`, `HALVE`) are replicated history, not ambient state, so initial
//! configuration construction is itself recoverable. One reconfiguration commits a
//! [`SystemOperation::Batch`]: operations applied together, in order, establishing one
//! era. A batch containing `Double` or `Halve` contains nothing else (R13); every other
//! batch is a unit batch moving at most one unit of per-node mass (R14); genesis and
//! nested batches are refused (R15). The planner that partitions an operation stream
//! into legal batches is [`crate::reconfiguration`].
//!
//! The relation `era(view) + 1 >= era(slot) >= era(view)` bounds which configuration may
//! authorize a slot. The `+1` case is overlap mode: an era-`e` view committing operations
//! against the era-`e+1` replication quorum.
//!
//! Failure to construct the §8.7.6 pivot is **not** a protocol fault. A low-weight
//! primary may have no legal split; the core falls back to the ordinary stop-the-world
//! view change. Treating a missing pivot as an error would make weight assignment a
//! correctness concern for the host instead of a latency concern.
//!
//! # Era assignment
//!
//! `Void` establishes era 0, `Init` establishes era 1, and every subsequent accepted
//! operation — single or [`SystemOperation::Batch`] — establishes `era + 1`. Era `e` is
//! therefore established by the `(e+1)`-th committed configuration operation, and
//! §8.7.8's rule that a replica may propose era `e` only if it holds the operation
//! establishing `e` is checkable by construction: era and establishing operation are in
//! one-to-one correspondence, so there is no bookkeeping to drift.
//!
//! # Persistence (rules §9)
//!
//! Nothing here is mutated. [`Configuration::apply`] folds one configuration into the
//! next and [`EraTable::extend`] returns a new table that shares the retained `Arc`s
//! with its receiver, so later consumers can hand configuration history out by `Arc`
//! without copying and without locking. The durable record is a [`Snapshot`] plus a WAL
//! of [`SystemOperation`] values — the WAL payload type — and replay is the fold.

use std::sync::Arc;

use crate::ids::{Era, NodeId, Slot, View};
use crate::wire::{Malformed, Pack, PackWriter, Unpack, UnpackCursor, UnpackError};

/// The log slot `Void` occupies: 1, the first position of a legitimate history
/// (§8.7.2: "operation number 1 ... with an otherwise empty log"). The slot is the only
/// evidence the fold has that this is genesis, so the ordinal is part of the
/// precondition, not a convention.
pub const VOID_SLOT: Slot = Slot(1);

/// The log slot `Init` occupies: 2, immediately after [`VOID_SLOT`] (§8.7.2: "immediately
/// preceded by `VOID`"). Fixing both ordinals is what makes a second genesis
/// unreachable: any later `Void` or `Init` is on the wrong slot before its payload is
/// even examined.
pub const INIT_SLOT: Slot = Slot(2);

/// The largest membership the fold will establish: 16 members.
///
/// The cap is a **validation-cost bound, not a protocol limit**. The
/// cross-era quorum discharge in [`crate::quorum`] enumerates all `2^N` subsets of the
/// membership; at `N = 16` that is 65536 predicate evaluations per reconfiguration
/// preflight — microseconds. Raising the cap is a one-constant change whose cost
/// consequence is exactly that the enumeration doubles per added member, and the
/// quorum gate's documented asymptotics assume this bound.
///
/// Two further consequences are deliberate. [`Configuration::len`] and
/// [`Configuration::index_of`] are total *by cap*: a membership of at most 16 fits a
/// `u32` with room to spare, so there is no conversion failure to handle. And target
/// deployments are 3–6 nodes, so 16 is generous headroom rather than a constraint a
/// host will ever meet.
pub const MAX_MEMBERS: u32 = 16;

/// [`MAX_MEMBERS`] as `usize`, so length checks against a `Vec` need no fallible
/// conversion. Pinned to the same value by the compile-time assertion below; the two
/// constants exist because comparing `usize` against `u32` would otherwise require a
/// cast this crate does not use.
const MAX_MEMBERS_USIZE: usize = 16;

const _: () = assert!(MAX_MEMBERS == 16 && MAX_MEMBERS_USIZE == 16);

/// The largest voting weight any member may hold: 2 (rules §1, R1).
///
/// Voting weights are common factors. A configuration whose weights are `9, 9, 9`
/// decides with two of three nodes — `18/27 = 2/3`, the same split as `(1, 1, 1)` — so
/// any uniform scaling is a zero effect on quorums, and no node ever needs a weight
/// above 2. The fold refuses every operation that would produce a weight outside
/// {0, 1, 2}: `Increment` past 2 names the member (R7), and `Double` with any member at
/// 2 names the first such member (R9). `Double` and `Halve` are therefore usable exactly
/// once per doubling cycle (rules §3), which keeps every worked schedule inside the
/// domain the closure arguments were checked over.
pub const MAX_WEIGHT: u32 = 2;

/// Voting weight of one member.
///
/// Weight `0` is a **learner** (§8.4; rules §1): it receives history and may occupy a
/// position in `order`, but it contributes nothing to any quorum. That is the property
/// that makes `Join` and `Leave` at weight zero safe — neither changes any quorum
/// family, so neither needs an overlap argument — and it is what makes state transfer a
/// precondition of authority rather than a courtesy: a member votes only after a
/// committed `Increment` promotes it. The domain is {0, 1, 2} ([`MAX_WEIGHT`], R1);
/// every operation that would leave it is refused by the fold.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Weight(pub u32);

/// One member of a configuration: who it is, and what it votes with.
///
/// `Copy` because it is two words and is passed by value everywhere; the identity it
/// carries is the host-assigned [`NodeId`], which the core never interprets beyond
/// equality (§8.7.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Member {
    /// The member's identity, assigned by the administrator.
    pub node: NodeId,
    /// The member's voting weight; `0` is a learner (§8.4).
    pub weight: Weight,
}

/// An era, an ordered member list, and a weight per member (§8.7.1).
///
/// The fields are private because the invariants — non-empty `order` once `Init` has
/// committed, no duplicate `NodeId`, every weight inside {0, 1, 2} (R1) and the total
/// at least 1 — must hold for *every* value that exists, and the only way to guarantee
/// that is to make [`Configuration::void`] and [`Configuration::apply`] the only ways
/// in. There is no public constructor that can produce an invalid `Configuration`.
///
/// For the same reason there are no serde impls: `Deserialize` would be an unchecked
/// constructor, and Q1's quorum families are stated over configurations *reachable by
/// the fold* — a value that bypassed the fold would be outside every closure argument
/// in §8.7.5. Serialization exists through [`Snapshot`] (rules §9), whose
/// [`Snapshot::inflate`] re-checks every invariant before yielding a value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Configuration {
    era: Era,
    order: Vec<Member>,
}

// The `total()` accumulator cannot overflow: the membership cap bounds the count at
// `MAX_MEMBERS`, each weight is at most `u32::MAX`, and even the pre-cap bound of
// `u32::MAX` members times `u32::MAX` weight stays below `u64::MAX`. Stated as a
// compile-time assertion so a widened `Weight` or `NodeId` breaks the build here rather
// than quietly invalidating the return type of `total()`.
const _: () = assert!((u32::MAX as u64) * (u32::MAX as u64) < u64::MAX);

impl Configuration {
    /// The void configuration: era 0, empty `order`, total weight 0.
    ///
    /// Era 0 is quorum-impossible by **arithmetic, not by guard**. Every quorum
    /// condition in §8.4 and §8.7.5 bottoms out in `sum(W(n) for n in S) >=
    /// floor(T/2) + 1`; on the void configuration `T` is 0 and the right-hand side is
    /// 1, so the empty set and every other set fail it. Nothing can commit before
    /// `Init` commits, and that impossibility cannot be forgotten the way a guard can,
    /// because it is a property of the numbers rather than a check someone remembered
    /// to write.
    #[must_use]
    pub const fn void() -> Configuration {
        Configuration {
            era: Era::INITIAL,
            order: Vec::new(),
        }
    }

    /// The era this configuration establishes (§8.7.1).
    #[must_use]
    pub fn era(&self) -> Era {
        self.era
    }

    /// The primary-succession sequence, exactly as the host supplied it.
    ///
    /// `order` is host convention, not core policy. The core sorts nothing and resolves
    /// nothing: it stores the sequence the host supplied in `Init` and inserts at the
    /// position the host named in `Join`. A host that wants deterministic
    /// cross-datacentre failover — for example by naming nodes so that a left-padded
    /// ordinal, a datacentre code and a left-padded node id sort into the succession it
    /// wants — is free to do that, and the core neither implements nor precludes it.
    /// Assigning node ids is the administrator's job; mapping a node id to an address
    /// is the host's job; security is the host's job.
    #[must_use]
    pub fn order(&self) -> &[Member] {
        &self.order
    }

    /// The number of members.
    ///
    /// Total **by cap, not by hope**: the fold refuses `Init` and `Join` past
    /// [`MAX_MEMBERS`], so the count is at most 16 and the `u32` return cannot
    /// truncate. The count is computed by a `u32` fold rather than a `usize`
    /// conversion, so there is no conversion failure path to document away.
    #[must_use]
    pub fn len(&self) -> u32 {
        debug_assert!(self.order.len() <= MAX_MEMBERS_USIZE);
        self.order.iter().fold(0u32, |count, _| count + 1)
    }

    /// Whether the membership is empty — true exactly for the void configuration.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The sum of all member weights.
    ///
    /// Returns `u64`, not `Option<u64>`, because the sum cannot overflow: at most
    /// `u32::MAX` members each at most `u32::MAX` weight, and the compile-time
    /// assertion above pins `u32::MAX * u32::MAX < u64::MAX`. There is no case to
    /// report, so there is no `Option` to thread through every quorum evaluation that
    /// builds on this.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.order
            .iter()
            .map(|member| u64::from(member.weight.0))
            .sum()
    }

    /// The weight `node` votes with, or `None` if it is not a member.
    #[must_use]
    pub fn weight_of(&self, node: NodeId) -> Option<Weight> {
        self.order
            .iter()
            .find(|member| member.node == node)
            .map(|member| member.weight)
    }

    /// The position of `node` in the succession sequence, or `None` if it is not a
    /// member. The position is what `Join { position }` named and what [`primary`]
    /// indexes; it is not derivable from the node id, because the core sorts nothing.
    ///
    /// Total by cap: [`MAX_MEMBERS`] bounds the membership at 16, so the position is
    /// at most 15 and the `u32` result cannot truncate. The index is produced by
    /// zipping with a `u32` counter rather than converting a `usize`, so there is no
    /// fallible conversion here at all.
    ///
    /// [`primary`]: Configuration::primary
    #[must_use]
    pub fn index_of(&self, node: NodeId) -> Option<u32> {
        self.order
            .iter()
            .zip(0u32..)
            .find(|(member, _)| member.node == node)
            .map(|(_, index)| index)
    }

    /// `order[view mod len]` (§1.2), or `None` on the void configuration.
    ///
    /// The index is into the host-supplied sequence, unsorted and unmodified: view 0
    /// selects `order[0]`, and the host chose who that is when it wrote `Init`. `None`
    /// is the total answer for the void configuration — there is no index into an
    /// empty sequence — not a defensive fallback.
    #[must_use]
    pub fn primary(&self, view: View) -> Option<NodeId> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        let index = usize::try_from(view.0 % len).ok()?;
        self.order.get(index).map(|member| member.node)
    }

    /// The sum of the weights of `members`, or `None` if any is unknown or named twice.
    ///
    /// Quorum evaluation builds on this. Duplicate rejection here is what
    /// stops a set from voting twice; a caller that deduplicated for itself would hold
    /// a second copy of the rule, and two copies of an intersection rule are two
    /// chances to get it wrong. The sum of distinct members' weights is bounded by
    /// [`total`], so it cannot overflow.
    ///
    /// Allocation-free and quadratic in the length of `members`: quorum sets are
    /// membership-sized, and a hash set's per-query allocation would dominate the scan
    /// it exists to speed up.
    ///
    /// [`total`]: Configuration::total
    #[must_use]
    pub fn weight_of_set(&self, members: &[NodeId]) -> Option<u64> {
        let mut sum = 0u64;
        for (index, node) in members.iter().enumerate() {
            if members[..index].contains(node) {
                return None;
            }
            sum += u64::from(self.weight_of(*node)?.0);
        }
        Some(sum)
    }

    /// The era a post-genesis operation establishes, or the refusal.
    ///
    /// `NotInitialised` when the receiver is the void configuration: only `Void` and
    /// `Init` may be applied before `Init`, and reporting `NotAMember` for a cluster
    /// with no membership would suggest a member could be supplied. `EraExhausted`
    /// when the era space is spent: §8.7.3 forbids wraparound, so the fold stops rather
    /// than reuses a generation.
    fn successor_era(&self) -> Result<Era, ConfigError> {
        if self.order.is_empty() {
            return Err(ConfigError::NotInitialised);
        }
        self.era.next().ok_or(ConfigError::EraExhausted)
    }

    /// Folds `op`, committed at log slot `at`, into the next configuration.
    ///
    /// `Void` establishes era 0, `Init` establishes era 1, and every other accepted
    /// operation — single or [`SystemOperation::Batch`] — establishes `era + 1` — see
    /// the module docs for why that makes §8.7.8's proposing rule checkable by
    /// construction. `at` is consulted only for the two genesis ordinals; no other
    /// precondition is slot-sensitive.
    ///
    /// # Preconditions, each with its own refusal
    ///
    /// Per operation (§8.7.2; rules §3, R7–R12), and per batch (rules §4, R13–R15):
    ///
    /// | Operation | Preconditions |
    /// |---|---|
    /// | `Void` | receiver is void; `at == `[`VOID_SLOT`] |
    /// | `Init { order }` | receiver is void; `at == `[`INIT_SLOT`]; `order` non-empty, duplicate-free, and at most [`MAX_MEMBERS`] long; all weights become `1` |
    /// | `Increment(n)` | `n ∈ order`; `W(n) + 1 <= `[`MAX_WEIGHT`] (R7) |
    /// | `Decrement(n)` | `n ∈ order`; `W(n) >= 1`; resulting `total() >= 1` (R8) |
    /// | `Double` | every `W(n) <= 1`, so every doubled weight stays in the domain (R9); the first member that would pass 2 is named |
    /// | `Halve` | every `W(n)` even (R10); no rounding |
    /// | `Join { node, position }` | `node ∉ order`; `len() < `[`MAX_MEMBERS`]; `position <= len()`; inserted at weight `0` (R11) |
    /// | `Leave(n)` | `n ∈ order`; `W(n) == 0` (R12) |
    /// | `Batch(ops)` | non-empty; no genesis op and no nested batch (R15); a batch containing `Double` or `Halve` is exactly that one op (R13); otherwise the unit rule — over the union of before/after memberships, `Σ_a |W_before(a) − W_after(a)| <= 1` (R14) |
    ///
    /// Every operation must leave `order` non-empty and `total() >= 1`, and only
    /// `Decrement` needs a check to guarantee it: `Halve`'s all-even parity implies
    /// some weight is at least 2 and halves to at least 1, and `Leave`'s zero-weight
    /// precondition makes "leave the last member" unreachable — a sole member at
    /// weight 0 is a zero total, which the `Decrement` that would have produced it
    /// already refused. Rejection is total: nothing saturates, nothing rounds, and a
    /// refused operation produces no configuration at all.
    ///
    /// # The one departure route
    ///
    /// `Decrement` to zero, then `Leave`, is the only way a member leaves.
    /// `Decrement` on a weight-0 member is refused ([`ConfigError::WeightUnderflow`]),
    /// which is distinct from `Leave`, and `Leave` on a positive-weight member is
    /// refused ([`ConfigError::NonZeroWeight`]). Departure is therefore a sequence of
    /// individually overlap-safe steps — a unit weight change preserves the §8.7.5
    /// consecutive-era intersection, and a weight-0 removal changes no quorum family at
    /// all — rather than one unsafe step. Inside a [`SystemOperation::Batch`] the same
    /// steps compose under the unit rule R14: zero-mass changes move nothing, so any
    /// number of them may share one era.
    ///
    /// # What this function deliberately does not check
    ///
    /// §8.7.2 also requires that `Init` not be proposed after any view change. That is
    /// a proposal-time condition on the proposer's state, not a property of the fold:
    /// the fold sees slots and configurations, not views. It belongs to the planned
    /// view-change proposal path, and its absence here is deliberate, not an omission.
    pub fn apply(&self, op: &SystemOperation, at: Slot) -> Result<Configuration, ConfigError> {
        match op {
            SystemOperation::Void => {
                if !self.order.is_empty() {
                    return Err(ConfigError::AlreadyInitialised);
                }
                if at != VOID_SLOT {
                    return Err(ConfigError::WrongGenesisSlot {
                        expected: VOID_SLOT,
                        actual: at,
                    });
                }
                Ok(Configuration::void())
            }
            SystemOperation::Init { order } => {
                if !self.order.is_empty() {
                    return Err(ConfigError::AlreadyInitialised);
                }
                if at != INIT_SLOT {
                    return Err(ConfigError::WrongGenesisSlot {
                        expected: INIT_SLOT,
                        actual: at,
                    });
                }
                if order.is_empty() {
                    return Err(ConfigError::EmptyInitOrder);
                }
                // The membership cap bounds the quorum gate's subset
                // enumeration; see `MAX_MEMBERS`.
                if order.len() > MAX_MEMBERS_USIZE {
                    return Err(ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS });
                }
                // Quadratic duplicate scan, allocation-free. This runs once, at
                // genesis, over the founding membership; a set structure would
                // allocate to save nothing.
                for (index, node) in order.iter().enumerate() {
                    if order[..index].contains(node) {
                        return Err(ConfigError::DuplicateNode(*node));
                    }
                }
                Ok(Configuration {
                    era: Era(1),
                    order: order
                        .iter()
                        .map(|&node| Member {
                            node,
                            weight: Weight(1),
                        })
                        .collect(),
                })
            }
            // A batch is the establishing operation of exactly one era: the
            // rules §4 applier checks R13–R15 and folds the sub-operations
            // in order within that one era.
            SystemOperation::Batch(ops) => self.apply_batch(ops),
            // A single operation is a one-element era of its own, exactly as
            // before the batch form existed: era first, then the per-op
            // preconditions of the in-era applier.
            single => {
                let era = self.successor_era()?;
                let mut next = self.apply_in_era(single)?;
                next.era = era;
                Ok(next)
            }
        }
    }

    /// The in-era applier: folds one non-genesis operation into the configuration
    /// **of the same era**. Every per-op precondition of the table above is checked
    /// here, at the op's point in the sequence — which is what lets
    /// [`SystemOperation::Batch`] fold its sub-operations sequentially and still
    /// hold each one's boundary at its point, while establishing exactly one era.
    ///
    /// The returned configuration carries the receiver's era; the dispatchers
    /// ([`Configuration::apply`] for singles, [`Configuration::apply_batch`] for
    /// batches) establish the successor era.
    fn apply_in_era(&self, op: &SystemOperation) -> Result<Configuration, ConfigError> {
        match op {
            SystemOperation::Increment(node) => {
                let index = match self.order.iter().position(|m| m.node == *node) {
                    Some(index) => index,
                    None => return Err(ConfigError::NotAMember(*node)),
                };
                // R7: the member would leave the domain {0, 1, 2} — refused by
                // name, never saturated. A clamped weight would make two
                // distinct configurations indistinguishable and quietly break
                // the §8.7.5 closure arguments, which are arithmetic identities
                // over the *actual* weights.
                if self.order[index].weight.0 >= MAX_WEIGHT {
                    return Err(ConfigError::WeightCapExceeded {
                        node: *node,
                        cap: MAX_WEIGHT,
                    });
                }
                let mut order = self.order.clone();
                order[index].weight = Weight(self.order[index].weight.0 + 1);
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            SystemOperation::Decrement(node) => {
                let index = match self.order.iter().position(|m| m.node == *node) {
                    Some(index) => index,
                    None => return Err(ConfigError::NotAMember(*node)),
                };
                // R8: a learner has no weight to give; the departure route ends
                // here and continues with `Leave`. Distinct from
                // [`ConfigError::NotAMember`] because the host's next action differs.
                let weight = self.order[index].weight.0;
                if weight == 0 {
                    return Err(ConfigError::WeightUnderflow(*node));
                }
                // `weight >= 1` here, so the member contributes at least 1 to the
                // total; a total of 1 means this member alone carries it and the
                // result would be 0.
                if self.total() == 1 {
                    return Err(ConfigError::TotalWouldBeZero);
                }
                let mut order = self.order.clone();
                order[index].weight = Weight(weight - 1);
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            SystemOperation::Double => {
                // R9: every weight must be at most 1, so every doubled weight stays
                // in the domain; the first member that would pass 2 is named.
                // Refusal is per configuration, not per member: the operation is
                // atomic, so one out-of-domain weight refuses the whole fold rather
                // than doubling the others and silently changing every ratio §8.7.5
                // reasons about. `Double` and `Halve` are therefore usable exactly
                // once per doubling cycle (rules §1).
                let order = self
                    .order
                    .iter()
                    .map(|member| match member.weight.0.checked_mul(2) {
                        Some(weight) if weight <= MAX_WEIGHT => Ok(Member {
                            node: member.node,
                            weight: Weight(weight),
                        }),
                        _ => Err(ConfigError::WeightCapExceeded {
                            node: member.node,
                            cap: MAX_WEIGHT,
                        }),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            SystemOperation::Halve => {
                // R10: every weight must be even — refused, never rounded down. The
                // first odd member in `order` is named. The resulting total is at
                // least 1 without a check: all-even weights and a positive total imply
                // some weight is at least 2, which halves to at least 1.
                let order = self
                    .order
                    .iter()
                    .map(|member| {
                        if member.weight.0 % 2 == 0 {
                            Ok(Member {
                                node: member.node,
                                weight: Weight(member.weight.0 / 2),
                            })
                        } else {
                            Err(ConfigError::OddWeight(member.node))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            SystemOperation::Join { node, position } => {
                // R11: the join inserts a learner (R2) — there is no operation
                // that joins at any other weight.
                if self.order.iter().any(|member| member.node == *node) {
                    return Err(ConfigError::DuplicateNode(*node));
                }
                // One more member would take the membership past the cap;
                // see `MAX_MEMBERS`.
                if self.order.len() >= MAX_MEMBERS_USIZE {
                    return Err(ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS });
                }
                let len = self.len();
                if *position > len {
                    return Err(ConfigError::PositionOutOfRange {
                        position: *position,
                        len,
                    });
                }
                let mut order = self.order.clone();
                // Unreachable conversion failure, documented rather than removed:
                // `position <= len() <= MAX_MEMBERS = 16`, and every target this crate
                // supports has a `usize` of at least 16 bits, so the `u32` always fits.
                let index = usize::try_from(*position).expect("position <= len() <= MAX_MEMBERS");
                order.insert(
                    index,
                    Member {
                        node: *node,
                        weight: Weight(0),
                    },
                );
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            SystemOperation::Leave(node) => {
                // R12: the member must have been driven to weight 0 first; a
                // member still voting refuses by name.
                let index = match self.order.iter().position(|m| m.node == *node) {
                    Some(index) => index,
                    None => return Err(ConfigError::NotAMember(*node)),
                };
                if self.order[index].weight.0 != 0 {
                    return Err(ConfigError::NonZeroWeight(*node));
                }
                // The result is non-empty without a check: a sole member at weight 0
                // is a zero total, which the fold has never produced.
                let mut order = self.order.clone();
                order.remove(index);
                Ok(Configuration {
                    era: self.era,
                    order,
                })
            }
            // Unreachable from the two dispatchers: `apply` routes genesis and
            // batch operations elsewhere before the in-era applier runs, and
            // `apply_batch` refuses such sub-operations first (R15).
            _ => unreachable!("genesis and batch operations never reach the in-era applier"),
        }
    }

    /// The batch applier (rules §4): `ops` fold in order within ONE era, and the
    /// batch is the establishing operation of exactly one successor era (§9: one
    /// batch, one WAL entry, one era).
    ///
    /// # Preconditions (R13–R15), each with its own refusal
    ///
    /// * Non-empty ([`ConfigError::EmptyBatch`]).
    /// * No genesis operation and no nested batch (R15:
    ///   [`ConfigError::GenesisNotPlannable`], [`ConfigError::NestedBatch`]).
    /// * A batch containing `Double` or `Halve` is exactly that one op (R13:
    ///   [`ConfigError::ScalingNotSolitary`]). A scaling op changes no quorum family
    ///   — common-factor normalization (rules §8) — so it is exempt from the unit
    ///   rule and solitary by construction.
    /// * Every other batch is a **unit batch**: over the union of before/after
    ///   memberships (missing nodes weigh 0), `Σ_a |W_before(a) − W_after(a)| <= 1`
    ///   (R14, [`ConfigError::BatchMassMoved`] naming the mass moved). The rule is
    ///   deliberately over per-node mass moved, not over the net total change: the
    ///   identity swap has net change 0 yet moves mass 2, and its endpoint's
    ///   consecutive-era majorities are disjoint (rules §4). Zero-weight joins and
    ///   leaves move no mass, so any number of them is legal in one era.
    ///
    /// Each sub-operation's own preconditions hold at its point in the sequence —
    /// the fold is sequential, and a refused sub-operation refuses the whole batch.
    fn apply_batch(&self, ops: &[SystemOperation]) -> Result<Configuration, ConfigError> {
        if ops.is_empty() {
            return Err(ConfigError::EmptyBatch);
        }
        for op in ops {
            match op {
                SystemOperation::Void | SystemOperation::Init { .. } => {
                    return Err(ConfigError::GenesisNotPlannable);
                }
                SystemOperation::Batch(_) => return Err(ConfigError::NestedBatch),
                _ => {}
            }
        }
        let solitary_scaling =
            ops.len() == 1 && matches!(ops[0], SystemOperation::Double | SystemOperation::Halve);
        if !solitary_scaling
            && ops
                .iter()
                .any(|op| matches!(op, SystemOperation::Double | SystemOperation::Halve))
        {
            return Err(ConfigError::ScalingNotSolitary);
        }
        let era = self.successor_era()?;
        let mut current = Configuration {
            era: self.era,
            order: self.order.clone(),
        };
        for op in ops {
            current = current.apply_in_era(op)?;
        }
        // R14: the unit rule, over per-node mass moved, not over the net total
        // change — the net total of any legal batch is −1, 0, or +1, but the
        // converse is false, and the identity swap is the counterexample that
        // would be admitted by a net check.
        if !solitary_scaling {
            let moved = self.mass_moved(&current);
            if moved > 1 {
                return Err(ConfigError::BatchMassMoved { moved });
            }
        }
        current.era = era;
        Ok(current)
    }

    /// The per-node mass R14 bounds between two configurations: over the union of
    /// both memberships, `Σ_a |W_before(a) − W_after(a)|`, with a node absent on
    /// one side weighing 0. Each term is at most 2 and the count is capped at
    /// [`MAX_MEMBERS`], so the sum cannot overflow.
    fn mass_moved(&self, after: &Configuration) -> u64 {
        let mut nodes: Vec<NodeId> = self.order.iter().map(|m| m.node).collect();
        nodes.extend(after.order.iter().map(|m| m.node));
        nodes.sort_unstable();
        nodes.dedup();
        nodes
            .into_iter()
            .map(|node| {
                let before = u64::from(self.weight_of(node).map_or(0, |weight| weight.0));
                let after = u64::from(after.weight_of(node).map_or(0, |weight| weight.0));
                before.abs_diff(after)
            })
            .sum()
    }

    /// The serializable projection of this configuration (rules §9): the era and the
    /// ordered membership, nothing else. `Configuration` itself carries no serde impls
    /// — `Deserialize` would be an unchecked constructor — so the durable record holds
    /// a [`Snapshot`], and re-inflation goes through [`Snapshot::inflate`], which
    /// re-checks every invariant the fold guarantees.
    #[must_use]
    pub fn to_snapshot(&self) -> Snapshot {
        Snapshot {
            era: self.era,
            order: self.order.clone(),
        }
    }
}

/// Why a reconfiguration operation was refused.
///
/// One variant per precondition of §8.7.2 — deliberately no shared "invalid" variant.
/// A host reacting to a refusal needs to know *which* precondition failed, because the
/// responses differ: retry with a corrected id, `Decrement` first, name a position
/// inside the sequence. And a test that can only observe `is_err()` passes when the
/// fold refuses for the wrong reason, which is precisely the failure mode a
/// precondition table exists to prevent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfigError {
    /// The receiver is the void configuration and the operation requires a live one.
    /// Only `Void` and `Init` may be applied before `Init` commits.
    NotInitialised,
    /// `Void` or `Init` was applied to a configuration that already has members.
    /// Genesis happens once; a second genesis would replace membership wholesale with
    /// no overlap argument, so it is refused on the receiver's state.
    AlreadyInitialised,
    /// A genesis operation occupied a slot other than its fixed ordinal. The slot is
    /// the only evidence the fold has that this is genesis, so accepting it elsewhere
    /// would let a replayed or replayed-out-of-order operation rewrite the founding
    /// eras.
    WrongGenesisSlot {
        /// The ordinal the operation must occupy: [`VOID_SLOT`] or [`INIT_SLOT`].
        expected: Slot,
        /// The slot the caller supplied.
        actual: Slot,
    },
    /// `Init` with an empty `order`. Every operation must leave `order` non-empty
    /// (§8.7.2); an empty genesis could never commit and never be repaired, because
    /// repair itself needs a quorum.
    EmptyInitOrder,
    /// A [`NodeId`] appeared twice in an `Init` order, or `Join` named a node already
    /// present. `order` is a list of *distinct* identifiers (§8.7.1); a duplicate would
    /// make `weight_of` ambiguous and let one node's weight be counted twice by a
    /// quorum evaluator that indexed by position — the double vote the distinctness
    /// requirement exists to forbid.
    DuplicateNode(NodeId),
    /// The operation names a node that is not in `order`.
    NotAMember(NodeId),
    /// `Increment` would take a weight past [`MAX_WEIGHT`], or `Double` would take
    /// any weight past it (R7, R9). Refused, never saturated: a clamped weight would
    /// make two distinct configurations indistinguishable and quietly break the
    /// §8.7.5 closure arguments, which are arithmetic identities over the *actual*
    /// weights. Carries the offending member — for `Double`, the first member that
    /// would pass the cap — and the cap itself.
    WeightCapExceeded {
        /// The member whose weight would leave the domain {0, 1, 2}.
        node: NodeId,
        /// The cap that was exceeded: always [`MAX_WEIGHT`].
        cap: u32,
    },
    /// `Decrement` on a member already at weight 0. A learner has no weight to give
    /// up; the departure route ends here and continues with `Leave`. Distinct from
    /// [`ConfigError::NotAMember`] because the host's next action differs.
    WeightUnderflow(NodeId),
    /// `Halve` on a configuration holding an odd weight. Refused, never rounded down:
    /// a rounded halve changes the total by an amount that depends on how many odd
    /// members there were, which is not a quantity §8.7.5's `T -> floor(T/2)` argument
    /// admits. Carries the first odd member in `order`.
    OddWeight(NodeId),
    /// `Leave` on a member whose weight is not 0. The member still votes; the host
    /// must `Decrement` it to 0 first. Distinct from [`ConfigError::NotAMember`]
    /// because the host's next action differs.
    NonZeroWeight(NodeId),
    /// `Join` named a position beyond `len()`. Legal positions are `0..=len()`; anything
    /// further is a gap the succession sequence cannot express.
    PositionOutOfRange {
        /// The position the caller named.
        position: u32,
        /// The membership count at the time of the refusal.
        len: u32,
    },
    /// `Init` or `Join` would take the membership past [`MAX_MEMBERS`]. The cap is a
    /// validation-cost bound for the quorum gate's subset enumeration, not a protocol
    /// limit — see the constant's documentation — so this refusal names the cap rather
    /// than pretending the membership is malformed.
    MembershipCapExceeded {
        /// The cap that was exceeded: always [`MAX_MEMBERS`].
        cap: u32,
    },
    /// `Decrement` would take the total weight to 0. A zero-total configuration is
    /// quorum-impossible in exactly the way era 0 is, but unlike era 0 it would be
    /// reachable *after* commitment, with no repair path that does not itself need a
    /// quorum.
    TotalWouldBeZero,
    /// The era space is exhausted: establishing the next era would wrap the `u32` era.
    /// §8.7.3 forbids wraparound — a reused era makes `config(e)` ambiguous and breaks
    /// the `R1`/`R2` intersection argument (§8.7.4), which is stated over consecutive
    /// eras — so the fold stops rather than reuses a generation.
    EraExhausted,
    /// A batch carried no operations. An empty batch is not an era: it would advance
    /// the era counter while establishing nothing, breaking the one-to-one
    /// correspondence between eras and establishing operations the §8.7.8 proposing
    /// rule is checked by.
    EmptyBatch,
    /// A batch carried a genesis operation (R15). `Void` and `Init` never appear in a
    /// batch: a second genesis inside a batch would replace membership wholesale with
    /// no overlap argument, and genesis is not plannable.
    GenesisNotPlannable,
    /// A batch carried another batch (R15). One reconfiguration commits one era; a
    /// nested batch has no era semantics the fold could give it, and the planner
    /// never proposes one.
    NestedBatch,
    /// A batch mixed a scaling operation (`Double` or `Halve`) with company (R13). A
    /// scaling op is solitary: it rescales every quorum threshold at once, and the
    /// intersection argument it rests on — common-factor normalization (rules §8) —
    /// is stated for the rescaling alone, not for a compound batch.
    ScalingNotSolitary,
    /// A batch moved more than one unit of per-node mass (R14). The variant names
    /// the mass moved: over the union of before/after memberships, the total
    /// `Σ_a |W_before(a) − W_after(a)|`. Deliberately the per-node mass, not the
    /// net total change — the net change of any legal batch is `−1`, `0`, or `+1`,
    /// but the converse is false, and the identity swap is the counterexample a net
    /// check would admit (rules §4).
    BatchMassMoved {
        /// The per-node mass the batch would have moved.
        moved: u64,
    },
    /// A snapshot claimed era 0 — the void — while carrying members. Era 0 is empty
    /// by definition, and a value that claims both is outside every configuration
    /// the fold can produce.
    SnapshotVoidWithMembers,
    /// A snapshot claimed a positive era with an empty `order`. A positive era is
    /// established by an operation that leaves `order` non-empty; an empty one is
    /// quorum-impossible with no repair path that does not itself need a quorum.
    SnapshotEmptyOrder {
        /// The era the snapshot claimed.
        era: Era,
    },
    /// A snapshot claimed a positive era whose total weight is 0. A zero-total
    /// configuration is quorum-impossible in exactly the way era 0 is, but unlike
    /// era 0 it could never be repaired.
    SnapshotZeroTotal {
        /// The era the snapshot claimed.
        era: Era,
    },
}

/// The reconfiguration operation alphabet of §8.7.2, exhaustive.
///
/// Operations are replicated history, not ambient state: they travel in the log as
/// typed payloads ([`crate::journal::LogEntry`] carries one), so configuration
/// construction is
/// itself recoverable and two replicas cannot disagree about genesis without the
/// disagreement appearing in the history itself. This type is the **WAL payload**
/// (rules §9): the durable record is a [`Snapshot`] plus a WAL of these values, and
/// replay is the fold.
///
/// `Init` carries only node ids: every initial weight is `1`. That removes the whole
/// class of "did the hosts agree on the genesis weights" divergence — the operation
/// cannot carry weights, so there is nothing to disagree about — and it matches
/// §8.7.5's closure proofs, which start from unit weights.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SystemOperation {
    /// Establishes era 0: the void configuration, with an otherwise empty log
    /// (§8.7.2). Legal only at [`VOID_SLOT`].
    Void,
    /// Establishes era 1: the founding membership, every weight `1`. Legal only at
    /// [`INIT_SLOT`], immediately after `Void`.
    Init {
        /// The primary-succession sequence; the core stores it verbatim and sorts
        /// nothing (see [`Configuration::order`]).
        order: Vec<NodeId>,
    },
    /// Raises one member's weight by 1; legal only while the member stays inside the
    /// domain {0, 1, 2} (R7).
    Increment(NodeId),
    /// Lowers one member's weight by 1. The first half of the only departure route;
    /// see [`Configuration::apply`].
    Decrement(NodeId),
    /// Doubles every weight; legal only while every weight stays inside the domain
    /// (R9). Solitary in a batch (R13).
    Double,
    /// Halves every weight; legal only when every weight is even (R10). Solitary in
    /// a batch (R13).
    Halve,
    /// Inserts a new member at weight 0 — a learner (§8.4; rules §2, R2), so the
    /// join changes no quorum family.
    Join {
        /// The node to insert; must not already be a member.
        node: NodeId,
        /// The succession position to insert at; must satisfy `position <= len()`.
        position: u32,
    },
    /// Removes a member whose weight is 0 (R12). The second half of the only
    /// departure route; see [`Configuration::apply`].
    Leave(NodeId),
    /// One reconfiguration: the sub-operations fold in order within ONE era, and
    /// the batch is that era's establishing operation (rules §4). Legal only when
    /// non-empty, free of genesis and nested batches (R15), solitary if it carries a
    /// scaling op (R13), and a unit batch otherwise (R14) — see
    /// [`Configuration::apply`].
    Batch(
        /// The sub-operations, in application order.
        Vec<SystemOperation>,
    ),
}

impl SystemOperation {
    /// The wire discriminant.
    ///
    /// A `match` rather than a cast, on the same reasoning as [`crate::wire::Tag`]: an
    /// explicit table states the numbering in one place, the compiler checks that every
    /// variant has one, and reordering this source cannot renumber the wire. `0` is
    /// reserved so that an all-zero buffer decodes as malformed rather than as a valid
    /// `Void`.
    fn discriminant(&self) -> u8 {
        match self {
            SystemOperation::Void => 1,
            SystemOperation::Init { .. } => 2,
            SystemOperation::Increment(_) => 3,
            SystemOperation::Decrement(_) => 4,
            SystemOperation::Double => 5,
            SystemOperation::Halve => 6,
            SystemOperation::Join { .. } => 7,
            SystemOperation::Leave(_) => 8,
            SystemOperation::Batch(_) => 9,
        }
    }
}

impl Pack for SystemOperation {
    fn packed_len(&self) -> usize {
        // One byte of discriminant, then fixed-width fields (W4). `Init` adds a `u32`
        // count and one `u32` id per member; a `Vec` cannot hold enough 4-byte ids for
        // the multiplication to overflow `usize`. A `Batch` adds a `u32` count and the
        // sub-operations' own encodings; the membership cap bounds a legal batch, and
        // even an adversarially long one is length-checked before it is decoded.
        match self {
            SystemOperation::Void | SystemOperation::Double | SystemOperation::Halve => 1,
            SystemOperation::Init { order } => 1 + 4 + 4 * order.len(),
            SystemOperation::Increment(_) | SystemOperation::Decrement(_) => 1 + 4,
            SystemOperation::Join { .. } => 1 + 4 + 4,
            SystemOperation::Leave(_) => 1 + 4,
            SystemOperation::Batch(ops) => 1 + 4 + ops.iter().map(Pack::packed_len).sum::<usize>(),
        }
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        w.u8(self.discriminant());
        match self {
            SystemOperation::Void | SystemOperation::Double | SystemOperation::Halve => {}
            SystemOperation::Init { order } => {
                let count = u32::try_from(order.len())
                    .expect("an Init order beyond u32::MAX members cannot be framed");
                w.u32(count);
                for node in order {
                    node.pack(w);
                }
            }
            SystemOperation::Increment(node)
            | SystemOperation::Decrement(node)
            | SystemOperation::Leave(node) => node.pack(w),
            SystemOperation::Join { node, position } => {
                node.pack(w);
                w.u32(*position);
            }
            SystemOperation::Batch(ops) => {
                let count = u32::try_from(ops.len())
                    .expect("a batch beyond u32::MAX operations cannot be framed");
                w.u32(count);
                for op in ops {
                    op.pack(w);
                }
            }
        }
    }
}

impl Unpack for SystemOperation {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        // Discriminant first, so an all-zero buffer fails before any field is read. An
        // unknown discriminant is `OutOfDomain`, not `UnknownTag`: that variant carries
        // a `u32` for the header's tag field, and this one is a `u8`.
        match c.u8()? {
            1 => Ok(SystemOperation::Void),
            2 => {
                let count = c.u32()?;
                // No `with_capacity(count)`: the count is untrusted, and reserving
                // against it would let a four-byte prefix demand a 16 GiB allocation.
                // Growth is bounded by the input actually consumed.
                let mut order = Vec::new();
                for _ in 0..count {
                    order.push(NodeId::unpack(c)?);
                }
                Ok(SystemOperation::Init { order })
            }
            3 => Ok(SystemOperation::Increment(NodeId::unpack(c)?)),
            4 => Ok(SystemOperation::Decrement(NodeId::unpack(c)?)),
            5 => Ok(SystemOperation::Double),
            6 => Ok(SystemOperation::Halve),
            7 => {
                let node = NodeId::unpack(c)?;
                let position = c.u32()?;
                Ok(SystemOperation::Join { node, position })
            }
            8 => Ok(SystemOperation::Leave(NodeId::unpack(c)?)),
            9 => {
                let count = c.u32()?;
                // No `with_capacity(count)`, for the same reason as `Init`: the
                // count is untrusted. Growth is bounded by the input consumed,
                // and a batch the fold cannot legally produce is refused there
                // rather than here — the codec's job is framing, not policy.
                let mut ops = Vec::new();
                for _ in 0..count {
                    ops.push(SystemOperation::unpack(c)?);
                }
                Ok(SystemOperation::Batch(ops))
            }
            _ => Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        }
    }
}

/// The durable state object (rules §9): an era and an ordered membership, exactly
/// what a checkpoint holds and nothing else.
///
/// `Configuration` carries no serde impls — `Deserialize` would be an unchecked
/// constructor, and Q1's quorum families are stated over configurations *reachable
/// by the fold*, so a value that bypassed the fold would be outside every closure
/// argument in §8.7.5. The snapshot is the serializable form instead: it round-trips
/// through serde (behind the `serde` feature) and re-inflates only through
/// [`Snapshot::inflate`], a CHECKED constructor that re-checks every
/// invariant the fold guarantees: the weight domain {0, 1, 2} (R1), duplicate-free
/// membership, the era/void correspondence (era 0 is the empty void; a positive era
/// is non-empty), and the total floor `total() >= 1` on a positive era.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snapshot {
    /// The era this snapshot establishes.
    pub era: Era,
    /// The membership in primary-succession order, with each member's weight.
    pub order: Vec<Member>,
}

impl Snapshot {
    /// Inflates the snapshot into a `Configuration`, re-checking every invariant
    /// (rules §9). This is the only way a stored state re-enters the fold's type
    /// system, and it refuses by name — one variant per precondition — anything the
    /// fold could not have produced: a weight outside {0, 1, 2}
    /// ([`ConfigError::WeightCapExceeded`]), a duplicated identity
    /// ([`ConfigError::DuplicateNode`]), era 0 carrying members
    /// ([`ConfigError::SnapshotVoidWithMembers`]), a positive era with an empty
    /// membership ([`ConfigError::SnapshotEmptyOrder`]), or a positive era with a
    /// zero total ([`ConfigError::SnapshotZeroTotal`]).
    ///
    /// The void snapshot itself — era 0, empty — inflates to
    /// [`Configuration::void`], whose era-0 quorum impossibility is arithmetic
    /// (see the void constructor's documentation).
    pub fn inflate(&self) -> Result<Configuration, ConfigError> {
        // Quadratic duplicate scan, allocation-free, mirroring the genesis fold:
        // `order` is a list of distinct identifiers, and a duplicate would let one
        // node's weight be counted twice.
        for (index, member) in self.order.iter().enumerate() {
            if self.order[..index]
                .iter()
                .any(|other| other.node == member.node)
            {
                return Err(ConfigError::DuplicateNode(member.node));
            }
        }
        // The weight domain (R1): every member inside {0, 1, 2}, refused by name.
        for member in &self.order {
            if member.weight.0 > MAX_WEIGHT {
                return Err(ConfigError::WeightCapExceeded {
                    node: member.node,
                    cap: MAX_WEIGHT,
                });
            }
        }
        if self.era == Era::INITIAL {
            if !self.order.is_empty() {
                return Err(ConfigError::SnapshotVoidWithMembers);
            }
            return Ok(Configuration::void());
        }
        if self.order.is_empty() {
            return Err(ConfigError::SnapshotEmptyOrder { era: self.era });
        }
        // The total floor: a zero-total positive era is quorum-impossible after
        // commitment, with no repair path that does not itself need a quorum.
        let total: u64 = self
            .order
            .iter()
            .map(|member| u64::from(member.weight.0))
            .sum();
        if total == 0 {
            return Err(ConfigError::SnapshotZeroTotal { era: self.era });
        }
        Ok(Configuration {
            era: self.era,
            order: self.order.clone(),
        })
    }
}

/// One era of configuration history.
///
/// Carries the configuration by `Arc` so a table hands history out without copying and
/// without locking, and precomputes `total` so a quorum evaluator never walks the
/// membership to rediscover it.
///
/// The `pivot` field records the concrete `qI`/`qII` vote sets that carried a
/// non-stop transition (§8.7.6–§8.7.7), when one was used. `None` on the
/// stop-the-world path (§8.7.4). The `RecordedQuorums` discipline requires the
/// record to name the vote sets that carried the transition, so a later era
/// proof can show exactly which members' votes established it.
#[derive(Clone, Debug)]
pub struct EraRecord {
    /// The era this configuration establishes.
    pub era: Era,
    /// The configuration of this era, shared.
    pub config: Arc<Configuration>,
    /// `config.total()`, precomputed.
    pub total: u64,
    /// The log slot of the operation that established this era.
    pub established_by: Slot,
    /// The operation that established this era.
    ///
    /// Retained with the bounded era window because the journal may reclaim
    /// the physical entry while peers still need an era proof (§8.7.8, S1).
    pub establishing_operation: SystemOperation,
    /// The concrete pivot vote sets that carried this transition, when the
    /// non-stop path was used (§8.7.6). `None` on the stop-the-world path.
    pub pivot: Option<crate::replica::Pivot>,
}

/// The derived, log-independent record of configuration history.
///
/// The journal may reclaim the physical slabs that carried the establishing operations
/// (S1); this table is what remains correct afterwards, which is its reason to exist.
/// It is persistent: [`EraTable::extend`] returns a new table that shares the retained
/// `Arc`s with its receiver, so a table held by a diagnostic reader or a `Progress`
/// snapshot stays valid while the replica advances, with no copy and no lock.
///
/// # Retention window: exactly three eras
///
/// The resident set is `{current - 1, current, current + 1}` as available — in
/// practice `current - 1` and `current`, since `current + 1` is not yet established.
/// Overlap mode (§8.7.7) evaluates quorums under era `e` and era `e+1` in the same
/// instant, so both must be resident. The previous era is retained so that in-flight
/// messages from a peer that has not yet learned of the era change remain *evaluable*
/// rather than merely undecidable. A message naming an era outside the window is not
/// evaluable and is **dropped**, not faulted — a peer that far behind must obtain a
/// current state by recovery or state transfer (§10, §14.2), which is the mechanism
/// VRR-2012 already requires for a stale peer. The window is deliberately small: an
/// unbounded table would make configuration history a memory leak, and a window of one
/// would make the overlap mode unexpressible.
///
/// The safety of *dropping* an out-of-window message is exercised end to end by the
/// crash matrix (`docs/architecture.md`); that claim has a named owner and is not
/// asserted by this module's own tests.
#[derive(Clone, Debug)]
pub struct EraTable {
    /// Retained records, non-decreasing in era; the last is current. Never empty: the
    /// table is born with the genesis record and the retention window always covers
    /// the current era.
    records: Vec<EraRecord>,
}

impl EraTable {
    /// The void era 0 alone.
    ///
    /// The genesis record predates the log, so its `established_by` is
    /// [`Slot::NONE`] as a placeholder: era 0 is established by construction, not by
    /// an operation. The `Void` operation at [`VOID_SLOT`] records the first real
    /// establishing slot on top of it.
    #[must_use]
    pub fn genesis() -> EraTable {
        EraTable {
            records: vec![EraRecord {
                era: Era::INITIAL,
                config: Arc::new(Configuration::void()),
                total: 0,
                established_by: Slot::NONE,
                establishing_operation: SystemOperation::Void,
                pivot: None,
            }],
        }
    }

    /// The most recently established era's record.
    ///
    /// Always present, by the invariant on `records`; there is no empty table to
    /// report.
    #[must_use]
    pub fn current(&self) -> &EraRecord {
        self.records
            .last()
            .expect("an EraTable always holds at least its current era")
    }

    /// The record for `era`, if it is inside the retention window.
    ///
    /// `None` means either the era was evicted or it was never established, and the
    /// caller need not distinguish: both make a message naming that era unevaluable,
    /// and an unevaluable message is dropped (see the type docs). Where two records
    /// exist for one era — possible only for era 0, established by both genesis and
    /// `Void` — the latest is returned.
    #[must_use]
    pub fn record(&self, era: Era) -> Option<&EraRecord> {
        self.records.iter().rev().find(|record| record.era == era)
    }

    /// Returns the table with `pivot` recorded on `era`'s record
    /// (§8.7.6–§8.7.7): the concrete vote sets that carried the non-stop
    /// transition into that era. Persistent like [`EraTable::extend`].
    /// `None` when `era` is outside the retention window.
    ///
    /// The pivot is leader-local transition evidence: only the leader that
    /// ran the planned view change knows the sets, and §8.7.7 carries them
    /// on no message, so a peer that folds the same era records `None`.
    #[must_use]
    pub fn with_transition_pivot(
        &self,
        era: Era,
        pivot: crate::replica::Pivot,
    ) -> Option<EraTable> {
        let mut records = self.records.clone();
        let record = records.iter_mut().rev().find(|record| record.era == era)?;
        record.pivot = Some(pivot);
        Some(EraTable { records })
    }

    /// Folds `op` at slot `at` onto the current configuration and returns the
    /// resulting table.
    ///
    /// Persistent: the receiver is unchanged, the retained records share their `Arc`s
    /// with the receiver rather than being re-created, and a refused operation leaves
    /// no trace. The retention window of the type applies to the result.
    pub fn extend(&self, op: &SystemOperation, at: Slot) -> Result<EraTable, ConfigError> {
        let config = self.current().config.apply(op, at)?;
        let era = config.era();
        let total = config.total();
        let mut records = self.records.clone();
        records.push(EraRecord {
            era,
            total,
            config: Arc::new(config),
            established_by: at,
            establishing_operation: op.clone(),
            pivot: None,
        });
        // Evict everything older than `current - 1`. `saturating_sub` rather than a
        // guard: at era 0 the cutoff is era 0 and nothing is evicted.
        let cutoff = era.0.saturating_sub(1);
        let first_kept = records.partition_point(|record| record.era.0 < cutoff);
        records.drain(..first_kept);
        Ok(EraTable { records })
    }
}
