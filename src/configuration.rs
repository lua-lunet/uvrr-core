//! Configuration, era, membership, voting weights, and reconfiguration operations.
//!
//! Spec §8.7.1 (configuration), §8.7.2 (reconfiguration operations), §8.7.5 (closure of
//! weighted-majority configurations), §8.7.6 (pivot condition), §8.7.7 (non-stop
//! transition), §8.7.8 (primary failure during overlap mode).
//!
//! A configuration is an era, an ordered member list, and a non-negative integer weight
//! per member. Primary selection is `config(era).order[index mod len(order)]`. Weight
//! zero grants no voting authority, so a newly added member is a learner until a later
//! committed `INCREMENT` promotes it — which is what makes state transfer a precondition
//! of authority rather than a courtesy.
//!
//! Reconfiguration operations (`VOID`, `INIT`, `ADD`, `REMOVE`, `INCREMENT`,
//! `DECREMENT`, `DOUBLE`, `HALVE`) are replicated history, not ambient state, so initial
//! configuration construction is itself recoverable.
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
//! operation establishes `era + 1`. Era `e` is therefore established by the `(e+1)`-th
//! committed configuration operation, and §8.7.8's rule that a replica may propose era
//! `e` only if it holds the operation establishing `e` is checkable by construction:
//! era and establishing operation are in one-to-one correspondence, so there is no
//! bookkeeping to drift.
//!
//! # Persistence
//!
//! Nothing here is mutated. [`Configuration::apply`] folds one configuration into the
//! next and [`EraTable::extend`] returns a new table that shares the retained `Arc`s
//! with its receiver, so later consumers can hand configuration history out by `Arc`
//! without copying and without locking.

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

/// Voting weight of one member.
///
/// Weight `0` is a **learner** (§8.4): it receives history and may occupy a position in
/// `order`, but it contributes nothing to any quorum. That is the property that makes
/// `Add` and `Remove` at weight zero safe — neither changes any quorum family, so
/// neither needs an overlap argument — and it is what makes state transfer a
/// precondition of authority rather than a courtesy: a member votes only after a
/// committed `Increment` promotes it.
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
/// committed, no duplicate `NodeId`, every weight and the total representable — must
/// hold for *every* value that exists, and the only way to guarantee that is to make
/// [`Configuration::void`] and [`Configuration::apply`] the only ways in. There is no
/// public constructor that can produce an invalid `Configuration`.
///
/// For the same reason there are no serde impls: `Deserialize` would be an unchecked
/// constructor, and Q1's quorum families are stated over configurations *reachable by
/// the fold* — a value that bypassed the fold would be outside every closure argument
/// in §8.7.5.
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
    /// position the host named in `Add`. A host that wants deterministic
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
    /// Total **by cap, not by hope**: the fold refuses `Init` and `Add` past
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
    /// member. The position is what `Add { position }` named and what [`primary`]
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
    /// operation establishes `era + 1` — see the module docs for why that makes
    /// §8.7.8's proposing rule checkable by construction. `at` is consulted only for
    /// the two genesis ordinals; no other precondition is slot-sensitive.
    ///
    /// # Preconditions (§8.7.2), each with its own refusal
    ///
    /// | Operation | Preconditions |
    /// |---|---|
    /// | `Void` | receiver is void; `at == `[`VOID_SLOT`] |
    /// | `Init { order }` | receiver is void; `at == `[`INIT_SLOT`]; `order` non-empty, duplicate-free, and at most [`MAX_MEMBERS`] long; all weights become `1` |
    /// | `Increment(n)` | `n ∈ order`; `W(n) + 1` representable in `u32` |
    /// | `Decrement(n)` | `n ∈ order`; `W(n) >= 1`; resulting `total() >= 1` |
    /// | `Double` | every `W(n) * 2` representable in `u32` |
    /// | `Halve` | every `W(n)` even |
    /// | `Add { node, position }` | `node ∉ order`; `len() < `[`MAX_MEMBERS`]; `position <= len()`; inserted at weight `0` |
    /// | `Remove(n)` | `n ∈ order`; `W(n) == 0` |
    ///
    /// Every operation must leave `order` non-empty and `total() >= 1`, and only
    /// `Decrement` needs a check to guarantee it: `Halve`'s all-even parity implies
    /// some weight is at least 2 and halves to at least 1, and `Remove`'s zero-weight
    /// precondition makes "remove the last member" unreachable — a sole member at
    /// weight 0 is a zero total, which the `Decrement` that would have produced it
    /// already refused. Rejection is total: nothing saturates, nothing rounds, and a
    /// refused operation produces no configuration at all.
    ///
    /// # The one departure route
    ///
    /// `Decrement` to zero, then `Remove`, is the only way a member leaves.
    /// `Decrement` on a weight-0 member is refused ([`ConfigError::WeightUnderflow`]),
    /// which is distinct from `Remove`, and `Remove` on a positive-weight member is
    /// refused ([`ConfigError::NonZeroWeight`]). Departure is therefore a sequence of
    /// individually overlap-safe steps — a unit weight change preserves the §8.7.5
    /// consecutive-era intersection, and a weight-0 removal changes no quorum family at
    /// all — rather than one unsafe step.
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
            SystemOperation::Increment(node) => {
                let era = self.successor_era()?;
                let index = match self.order.iter().position(|m| m.node == *node) {
                    Some(index) => index,
                    None => return Err(ConfigError::NotAMember(*node)),
                };
                let mut order = self.order.clone();
                let member = &mut order[index];
                member.weight = match member.weight.0.checked_add(1) {
                    Some(weight) => Weight(weight),
                    None => return Err(ConfigError::WeightOverflow),
                };
                Ok(Configuration { era, order })
            }
            SystemOperation::Decrement(node) => {
                let era = self.successor_era()?;
                let index = match self.order.iter().position(|m| m.node == *node) {
                    Some(index) => index,
                    None => return Err(ConfigError::NotAMember(*node)),
                };
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
                Ok(Configuration { era, order })
            }
            SystemOperation::Double => {
                let era = self.successor_era()?;
                // Refusal is per configuration, not per member: the operation is
                // atomic, so one unrepresentable weight refuses the whole fold rather
                // than doubling the others and silently changing every ratio §8.7.5
                // reasons about.
                let order = self
                    .order
                    .iter()
                    .map(|member| match member.weight.0.checked_mul(2) {
                        Some(weight) => Ok(Member {
                            node: member.node,
                            weight: Weight(weight),
                        }),
                        None => Err(ConfigError::WeightOverflow),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Configuration { era, order })
            }
            SystemOperation::Halve => {
                let era = self.successor_era()?;
                // Every weight must be even (§8.7.2): refused, never rounded down. The
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
                Ok(Configuration { era, order })
            }
            SystemOperation::Add { node, position } => {
                let era = self.successor_era()?;
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
                Ok(Configuration { era, order })
            }
            SystemOperation::Remove(node) => {
                let era = self.successor_era()?;
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
                Ok(Configuration { era, order })
            }
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
    /// A [`NodeId`] appeared twice in an `Init` order, or `Add` named a node already
    /// present. `order` is a list of *distinct* identifiers (§8.7.1); a duplicate would
    /// make `weight_of` ambiguous and let one node's weight be counted twice by a
    /// quorum evaluator that indexed by position — the double vote the distinctness
    /// requirement exists to forbid.
    DuplicateNode(NodeId),
    /// The operation names a node that is not in `order`.
    NotAMember(NodeId),
    /// `Increment` would take a weight past `u32::MAX`, or `Double` would take any
    /// weight past it. Refused, never saturated: a clamped weight would make two
    /// distinct configurations indistinguishable and quietly break the §8.7.5 closure
    /// arguments, which are arithmetic identities over the *actual* weights.
    WeightOverflow,
    /// `Decrement` on a member already at weight 0. A learner has no weight to give
    /// up; the departure route ends here and continues with `Remove`. Distinct from
    /// [`ConfigError::NotAMember`] because the host's next action differs.
    WeightUnderflow(NodeId),
    /// `Halve` on a configuration holding an odd weight. Refused, never rounded down:
    /// a rounded halve changes the total by an amount that depends on how many odd
    /// members there were, which is not a quantity §8.7.5's `T -> floor(T/2)` argument
    /// admits. Carries the first odd member in `order`.
    OddWeight(NodeId),
    /// `Remove` on a member whose weight is not 0. The member still votes; the host
    /// must `Decrement` it to 0 first. Distinct from [`ConfigError::NotAMember`]
    /// because the host's next action differs.
    NonZeroWeight(NodeId),
    /// `Add` named a position beyond `len()`. Legal positions are `0..=len()`; anything
    /// further is a gap the succession sequence cannot express.
    PositionOutOfRange {
        /// The position the caller named.
        position: u32,
        /// The membership count at the time of the refusal.
        len: u32,
    },
    /// `Init` or `Add` would take the membership past [`MAX_MEMBERS`]. The cap is a
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
}

/// The reconfiguration operation alphabet of §8.7.2, exhaustive.
///
/// Operations are replicated history, not ambient state: they travel in the log as
/// typed payloads ([`crate::journal::LogEntry`] carries one), so configuration
/// construction is
/// itself recoverable and two replicas cannot disagree about genesis without the
/// disagreement appearing in the history itself.
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
    /// Raises one member's weight by 1.
    Increment(NodeId),
    /// Lowers one member's weight by 1. The first half of the only departure route;
    /// see [`Configuration::apply`].
    Decrement(NodeId),
    /// Doubles every weight.
    Double,
    /// Halves every weight; legal only when every weight is even.
    Halve,
    /// Inserts a new member at weight 0 — a learner (§8.4), so the addition changes no
    /// quorum family.
    Add {
        /// The node to insert; must not already be a member.
        node: NodeId,
        /// The succession position to insert at; must satisfy `position <= len()`.
        position: u32,
    },
    /// Removes a member whose weight is 0. The second half of the only departure
    /// route; see [`Configuration::apply`].
    Remove(NodeId),
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
            SystemOperation::Add { .. } => 7,
            SystemOperation::Remove(_) => 8,
        }
    }
}

impl Pack for SystemOperation {
    fn packed_len(&self) -> usize {
        // One byte of discriminant, then fixed-width fields (W4). `Init` adds a `u32`
        // count and one `u32` id per member; a `Vec` cannot hold enough 4-byte ids for
        // the multiplication to overflow `usize`.
        match self {
            SystemOperation::Void | SystemOperation::Double | SystemOperation::Halve => 1,
            SystemOperation::Init { order } => 1 + 4 + 4 * order.len(),
            SystemOperation::Increment(_) | SystemOperation::Decrement(_) => 1 + 4,
            SystemOperation::Add { .. } => 1 + 4 + 4,
            SystemOperation::Remove(_) => 1 + 4,
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
            | SystemOperation::Remove(node) => node.pack(w),
            SystemOperation::Add { node, position } => {
                node.pack(w);
                w.u32(*position);
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
                Ok(SystemOperation::Add { node, position })
            }
            8 => Ok(SystemOperation::Remove(NodeId::unpack(c)?)),
            _ => Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        }
    }
}

/// One era of configuration history.
///
/// Carries the configuration by `Arc` so a table hands history out without copying and
/// without locking, and precomputes `total` so a quorum evaluator never walks the
/// membership to rediscover it.
///
/// A `pivot: Option<Pivot>` field recording the concrete `qI`/`qII` vote
/// sets that carried a non-stop transition (§8.7.6–§8.7.7) is deliberately not
/// pre-declared here.
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
    /// [`Slot::FIRST`] as a placeholder: era 0 is established by construction, not by
    /// an operation. The `Void` operation at [`VOID_SLOT`] records the first real
    /// establishing slot on top of it.
    #[must_use]
    pub fn genesis() -> EraTable {
        EraTable {
            records: vec![EraRecord {
                era: Era::INITIAL,
                config: Arc::new(Configuration::void()),
                total: 0,
                established_by: Slot::FIRST,
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
        });
        // Evict everything older than `current - 1`. `saturating_sub` rather than a
        // guard: at era 0 the cutoff is era 0 and nothing is evicted.
        let cutoff = era.0.saturating_sub(1);
        let first_kept = records.partition_point(|record| record.era.0 < cutoff);
        records.drain(..first_kept);
        Ok(EraTable { records })
    }
}
