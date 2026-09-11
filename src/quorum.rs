//! Quorum families by semantic role, and the `QuorumStrategy` extension point.
//!
//! Spec §8.1 (baseline majority), §8.2 (families, not counts), §8.3 (diskless overlap),
//! §8.4 (weighted quorums), §8.5 (even-sized configurations), §8.6 (reconfiguration
//! overlap), §8.7.5 (closure). Decision Q1.
//!
//! Quorums are named by role, not by size: `C_g` commit, `V_g` view change, `F_g`
//! `StartViewChange` fence, `R_g` restart. Distinguishing the roles is what lets a host
//! adopt the §8.5 even-node split (`V_g = k+1`, `C_g = k`) or a §8.4 weighted family
//! without editing the protocol.
//!
//! `QuorumStrategy` is an extension point; the intersection obligations it must satisfy
//! are not. The obligations — `R1: QI_e ⌢ QII_e` and `R2: QI_e ⌢ QII_(e+1)` (§8.7.4),
//! plus the diskless `F_g ⌢ R_g` and `V_g ⌢ V_g` (§8.3) — are discharged by
//! [`validate_era`] and [`validate_transition`], **free functions the core calls**, not
//! trait methods. That placement is the Q1 ruling made concrete: a strategy value
//! decides only [`QuorumStrategy::is_quorum`], and no strategy, feature flag or host
//! can override, skip or weaken the gate, because the gate is not part of the thing a
//! host supplies.
//!
//! The discharge is exhaustive subset enumeration over the membership, using the
//! complement identity: two families fail to intersect iff some quorum of one has a
//! complement that is a quorum of the other. Every refusal carries the **witness** —
//! the two disjoint subsets that violate the obligation — because a refusal that
//! cannot show the operator *why* is a refusal nobody can act on.
//! [`configuration::MAX_MEMBERS`] bounds the enumeration at `2^16` predicate
//! evaluations, which is what makes the cap a validation-cost bound rather than a
//! protocol limit.
//!
//! We ship `WeightedMajority` as the default because §8.7.5 proves its closure across
//! consecutive eras, and `EvenSplit` ships as a non-default *reference*
//! strategy to demonstrate that the extension point genuinely admits the six-node
//! three-datacentre profile. Shipping only a default would leave the extension point
//! unexercised and therefore unproven.
//!
//! We ship a default and a trait. We do not ship a choice.
//!
//! [`configuration::MAX_MEMBERS`]: crate::configuration::MAX_MEMBERS

use crate::configuration::{Configuration, MAX_MEMBERS};
use crate::ids::NodeId;

/// The semantic role a quorum serves. QII, QI, R_g and F_g respectively (§8.2, §8.3).
///
/// Exhaustive on purpose: there is no wildcard arm over `Role` anywhere in the crate,
/// so a fifth role added here is a compile error at every site that must take a
/// position on it — the gate, the shipped strategy, and every test-local strategy —
/// rather than a silently defaulted case.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    /// QII: the normal-operation commit family (§8.7.4).
    Commit,
    /// QI: the view-change family (§8.7.4).
    ViewChange,
    /// R_g: the restart family (§8.3).
    Restart,
    /// F_g: the `StartViewChange` fence family (§8.3).
    Fence,
}

/// A quorum policy. Open for extension (Q1): a host supplies one value at
/// construction. What a host may NOT do is weaken the closed obligations —
/// [`validate_era`] and [`validate_transition`] are run by the core on every
/// configuration and every proposed reconfiguration, and a refusal is final.
///
/// # Contract a strategy must honour
///
/// - **Monotonicity.** A superset of a quorum is a quorum. The gate's complement
///   identity — two families fail to intersect iff some quorum's complement is a
///   quorum of the other family — is only valid for upward-closed families, and every
///   quorum family in §8.2–§8.5 is upward-closed by construction.
/// - **Strictness about membership.** Unknown or duplicated nodes in `members` are
///   refused (`false`), which [`Configuration::weight_of_set`] already implements for
///   weight-based strategies. The gate enumerates subsets of a configuration union,
///   and a cross-era subset can legitimately contain nodes one of the two
///   configurations does not know; those subsets must simply fail the predicate.
pub trait QuorumStrategy {
    /// Is `members` a quorum for `role` under `config`? The ONLY thing a strategy
    /// decides. Everything else — intersection, cross-era closure — is checked
    /// mechanically from this predicate.
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool;

    /// Smallest quorum weight threshold for `role` under `config`, for diagnostics
    /// and for tests that build minimal quorums. Strategies whose quorums are not
    /// threshold-expressible return `None`; the exhaustive gate does not use this.
    fn threshold(&self, role: Role, config: &Configuration) -> Option<u64>;
}

/// Which direction of an era boundary a cross-era refusal was found on.
///
/// Both directions are gated because overlap mode (§8.7.7) sends era `e` to one set
/// of members and era `e+1` to another in the same instant; safety at the boundary is
/// a property of the pair of directions, not of the forward one alone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum R2Direction {
    /// `ViewChange_current ⌢ Commit_next` — R2 exactly as §8.7.4 states it: a view
    /// change in era `e` must not select a history that omits an operation committed
    /// under era `e+1`. This is the direction Q1's six-node counterexample violates.
    Forward,
    /// `ViewChange_next ⌢ Commit_current` — `PrepareOk` evidence gathered under era
    /// `e+1` must intersect a commit quorum under era `e`, or a view change in the
    /// new era can install a history the old era's committers never saw. Not a
    /// consequence of the forward direction when the two eras weight members
    /// differently; `tests/quorum_contract.rs` constructs a strategy that passes
    /// forward and is refused here.
    Reverse,
}

/// Why a configuration or a proposed reconfiguration was refused by the gate.
///
/// Every intersection refusal carries the **witness**: the two disjoint subsets that
/// violate the obligation. A refusal that cannot show the operator *why* is a refusal
/// nobody can act on, so the witness is part of the error type, not a log line. The
/// subsets are inclusion-minimal — neither has a proper subset that is a quorum of
/// its claimed family — so the witness names exactly the members whose votes cannot
/// both be legal.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum QuorumError {
    /// `R1` (§8.7.4) failed within one era: a commit quorum and a view-change quorum
    /// of the same configuration are disjoint.
    R1Violation {
        /// A `Role::Commit` quorum under the configuration.
        commit: Vec<NodeId>,
        /// A `Role::ViewChange` quorum under the same configuration, disjoint from
        /// `commit`.
        view_change: Vec<NodeId>,
    },
    /// `R2` (§8.7.4) failed across an era boundary, in either direction
    /// ([`R2Direction`]).
    R2Violation {
        /// Which direction of the boundary was violated.
        direction: R2Direction,
        /// A `Role::ViewChange` quorum under the era `direction` names.
        view_change: Vec<NodeId>,
        /// A `Role::Commit` quorum under the other era, disjoint from `view_change`.
        commit: Vec<NodeId>,
    },
    /// A family does not self-intersect (§8.3's `V_g ⌢ V_g`): two quorums of the same
    /// role in the same era are disjoint. Required because the fence family equals
    /// the view family in diskless VRR, so a recovering replica must encounter the
    /// volatile evidence that an earlier view was fenced; not required between
    /// arbitrary phase-one quorums in the classical protocol family, which is why
    /// a flexible-quorum policy satisfying `R1` can still fail here.
    SelfIntersectionViolation {
        /// The role whose family does not self-intersect.
        role: Role,
        /// A quorum of `role` under the configuration.
        first: Vec<NodeId>,
        /// A second quorum of `role` under the same configuration, disjoint from
        /// `first`.
        second: Vec<NodeId>,
    },
    /// §8.3's `F_g ⌢ R_g` failed: a fence quorum and a restart quorum of the same
    /// era are disjoint, so a recovering replica could miss the evidence that a view
    /// was fenced.
    FenceRestartViolation {
        /// A `Role::Fence` quorum under the configuration.
        fence: Vec<NodeId>,
        /// A `Role::Restart` quorum under the same configuration, disjoint from
        /// `fence`.
        restart: Vec<NodeId>,
    },
    /// A configuration presented to the gate exceeds [`MAX_MEMBERS`]. Unreachable
    /// through the fold — `Init` and `Join` refuse past the cap first — but the gate
    /// does not trust its callers: its cost analysis assumes the bound, so it
    /// restates it rather than enumerating an unbounded membership.
    MembershipCapExceeded {
        /// The cap that was exceeded: always [`MAX_MEMBERS`].
        cap: u32,
    },
}

/// The members of `universe` selected by `mask`, in `universe` order.
fn members_of(universe: &[NodeId], mask: u32) -> Vec<NodeId> {
    universe
        .iter()
        .zip(0u32..)
        .filter(|(_, bit)| mask & (1u32 << bit) != 0)
        .map(|(node, _)| *node)
        .collect()
}

/// The number of members, as the mask width. Total: computed by a `u32` fold, and the
/// callers cap `universe` at [`MAX_MEMBERS`], so the shifts below cannot overflow.
fn width_of(universe: &[NodeId]) -> u32 {
    universe.iter().fold(0u32, |count, _| count + 1)
}

/// Does some quorum of `family_a` have a complement (within `universe`) that is a
/// quorum of `family_b`? By the complement identity that is exactly the failure of
/// the two families to intersect — for the upward-closed families the trait contract
/// requires.
///
/// This is the fast scan: `2^N` masks, two predicate evaluations each, which is the
/// entire cost of validating a *legal* configuration. The two families may belong to
/// different configurations (cross-era): subsets containing nodes a configuration
/// does not know simply fail its predicate.
fn disjoint_quorums_exist(
    strategy: &dyn QuorumStrategy,
    family_a: (Role, &Configuration),
    family_b: (Role, &Configuration),
    universe: &[NodeId],
) -> bool {
    let n = width_of(universe);
    debug_assert!(n <= MAX_MEMBERS);
    let full = (1u32 << n) - 1;
    (0..=full).any(|mask| {
        strategy.is_quorum(family_a.0, family_a.1, &members_of(universe, mask))
            && strategy.is_quorum(family_b.0, family_b.1, &members_of(universe, full ^ mask))
    })
}

/// The witness: an inclusion-minimal disjoint quorum pair, or `None` if the families
/// intersect universally.
///
/// Runs the fast scan first, then — only on the refusal path — a thorough pass that
/// enumerates subsets in increasing size and, for each `family_a` quorum, searches
/// its complement for a smallest `family_b` quorum. The pair returned is minimal
/// *unconditionally*, with no monotonicity assumption: the first subset `S` returned
/// cannot have a proper subset that is an `A` quorum, because that subset was
/// enumerated earlier and its complement contains the `B` quorum found here, so it
/// would have been returned instead; and `T` is a smallest `B` quorum inside the
/// complement of `S`, so no proper subset of `T` qualifies either.
///
/// Cost: the valid path is the fast scan above. The refusal path is bounded by `3^N`
/// predicate evaluations in the worst case — at `N <= 16` still well under a second,
/// and a refusal is a terminal preflight event, not steady state.
fn find_disjoint_pair(
    strategy: &dyn QuorumStrategy,
    family_a: (Role, &Configuration),
    family_b: (Role, &Configuration),
    universe: &[NodeId],
) -> Option<(Vec<NodeId>, Vec<NodeId>)> {
    if !disjoint_quorums_exist(strategy, family_a, family_b, universe) {
        return None;
    }

    let n = width_of(universe);
    let full = (1u32 << n) - 1;
    // Subsets in increasing size: the first `S` that yields a pair has no proper
    // subset that is an `A` quorum (see the minimality argument above).
    for size in 0..=n {
        for mask in 0..=full {
            if mask.count_ones() != size {
                continue;
            }
            if !strategy.is_quorum(family_a.0, family_a.1, &members_of(universe, mask)) {
                continue;
            }
            let complement = full ^ mask;
            // Smallest `B` quorum inside the complement, again by increasing size.
            for inner in 0..=(n - size) {
                for sub in 0..=complement {
                    if sub & !complement != 0 || sub.count_ones() != inner {
                        continue;
                    }
                    if strategy.is_quorum(family_b.0, family_b.1, &members_of(universe, sub)) {
                        return Some((members_of(universe, mask), members_of(universe, sub)));
                    }
                }
            }
        }
    }
    // The fast scan proved a disjoint pair exists, and the thorough pass enumerates
    // every subset against every subset of its complement, so it cannot miss it.
    unreachable!("the fast scan and the thorough pass decide the same predicate")
}

/// Refuses a configuration whose membership exceeds the enumeration budget. See
/// [`QuorumError::MembershipCapExceeded`] for why the gate restates the fold's cap.
fn check_cap(config: &Configuration) -> Result<(), QuorumError> {
    if config.len() > MAX_MEMBERS {
        return Err(QuorumError::MembershipCapExceeded { cap: MAX_MEMBERS });
    }
    Ok(())
}

/// Validates one era's configuration against the within-era obligations (§8.3 +
/// §8.7.4, Q1): `Commit ⌢ ViewChange` (R1), `ViewChange ⌢ ViewChange`
/// (self-intersection, required because the fence family equals the view family in
/// diskless VRR), and `Fence ⌢ Restart`.
///
/// Each named obligation is discharged separately even though they coincide for
/// [`WeightedMajority`] — where all four roles are the strict majority — because a
/// future strategy that splits roles is still fully gated, and a refusal must name
/// the obligation it found, not the one that happens to share its arithmetic.
///
/// Cost: three obligations, each a `2^N` subset scan with two predicate evaluations
/// per subset on the valid path; `N <= `[`MAX_MEMBERS`], so at most 3 × 65536 × 2
/// evaluations per call — microseconds. A refusal additionally runs the
/// minimal-witness pass documented on [`find_disjoint_pair`].
pub fn validate_era(
    strategy: &dyn QuorumStrategy,
    config: &Configuration,
) -> Result<(), QuorumError> {
    check_cap(config)?;
    let universe: Vec<NodeId> = config.order().iter().map(|member| member.node).collect();

    if let Some((commit, view_change)) = find_disjoint_pair(
        strategy,
        (Role::Commit, config),
        (Role::ViewChange, config),
        &universe,
    ) {
        return Err(QuorumError::R1Violation {
            commit,
            view_change,
        });
    }
    if let Some((first, second)) = find_disjoint_pair(
        strategy,
        (Role::ViewChange, config),
        (Role::ViewChange, config),
        &universe,
    ) {
        return Err(QuorumError::SelfIntersectionViolation {
            role: Role::ViewChange,
            first,
            second,
        });
    }
    if let Some((fence, restart)) = find_disjoint_pair(
        strategy,
        (Role::Fence, config),
        (Role::Restart, config),
        &universe,
    ) {
        return Err(QuorumError::FenceRestartViolation { fence, restart });
    }
    Ok(())
}

/// Validates a proposed era transition against the cross-era obligation `R2`
/// (§8.7.4), in **both** directions: `ViewChange_current ⌢ Commit_next`
/// ([`R2Direction::Forward`], Q1's counterexample direction) and `ViewChange_next ⌢
/// Commit_current` ([`R2Direction::Reverse`]). Overlap mode sends era `e` to one set
/// and era `e+1` to another, so both directions of the boundary must be safe, not
/// only the forward one: `PrepareOk` evidence gathered under `e+1` must intersect a
/// commit quorum under `e` just as much as the converse.
///
/// The enumeration universe is the union of the two memberships, sorted for
/// determinism; nodes a configuration does not know simply fail its predicate. The
/// union of two fold-consecutive configurations never exceeds [`MAX_MEMBERS`] (the
/// one membership-changing operations, `Join` and `Leave`, move exactly one member at
/// weight 0), so the union check cannot fire on a legitimate transition — it exists
/// because the gate does not assume its callers only present fold-consecutive pairs.
///
/// Cost: two directions, each a `2^N` scan with two predicate evaluations per subset
/// on the valid path; at most 2 × 65536 × 2 evaluations — microseconds.
pub fn validate_transition(
    strategy: &dyn QuorumStrategy,
    current: &Configuration,
    next: &Configuration,
) -> Result<(), QuorumError> {
    check_cap(current)?;
    check_cap(next)?;
    let mut universe: Vec<NodeId> = current
        .order()
        .iter()
        .chain(next.order().iter())
        .map(|member| member.node)
        .collect();
    universe.sort();
    universe.dedup();
    if width_of(&universe) > MAX_MEMBERS {
        return Err(QuorumError::MembershipCapExceeded { cap: MAX_MEMBERS });
    }

    if let Some((view_change, commit)) = find_disjoint_pair(
        strategy,
        (Role::ViewChange, current),
        (Role::Commit, next),
        &universe,
    ) {
        return Err(QuorumError::R2Violation {
            direction: R2Direction::Forward,
            view_change,
            commit,
        });
    }
    if let Some((view_change, commit)) = find_disjoint_pair(
        strategy,
        (Role::ViewChange, next),
        (Role::Commit, current),
        &universe,
    ) {
        return Err(QuorumError::R2Violation {
            direction: R2Direction::Reverse,
            view_change,
            commit,
        });
    }
    Ok(())
}

/// Why a host-supplied pivot was refused (§8.7.6).
///
/// One variant per precondition of the pivot condition, so a refusal names
/// exactly which leg failed. A refusal is bad input, never a fault: the
/// reconfigure request is rejected before the operation enters the log.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PivotError {
    /// A member appears twice in `qI`, twice in `qII`, or once in each
    /// beyond the shared leader. The pivot condition requires `qI ∩ qII =
    /// {L}` exactly — no duplicates within a set, no second shared member.
    DuplicateMember(NodeId),
    /// The intersection of `qI` and `qII` is not exactly the leader.
    IntersectionNotLeader,
    /// The leader is absent from `qI` or `qII`.
    LeaderAbsent,
    /// `qI` is not a view-change quorum under `config(e)`.
    QiNotLegal,
    /// `qII` is not a commit quorum under `config(e)`.
    QiiNotLegalCurrent,
    /// `qII` is not a commit quorum under `config(e+1)`.
    QiiNotLegalNext,
}

/// Constructs the concrete pivot for a non-stop reconfiguration (§8.7.6).
///
/// From `config(e)` and `config(e+1)`, finds `qI`, `qII` with `qI ∩ qII =
/// {L}`, `qI` a view-change quorum under `config(e)`, `qII` a commit
/// quorum under both configs, and `|qI| + |qII| = N + 1` (the unweighted
/// cardinality rule, which forces `qI ∪ qII` to cover the membership).
///
/// The algorithm enumerates subsets of the union of both memberships in
/// increasing bitmask order, returning the first `qII` that satisfies
/// every leg. It **terminates** because the powerset of a membership
/// capped at [`MAX_MEMBERS`] is finite (`2^16` subsets at most). It is
/// **deterministic** because the enumeration order is fixed and the first
/// match is returned — same inputs, same pivot, every time.
///
/// A leader that finds no split (non-pivotal, low-weight) gets `None`:
/// the fallback to stop-the-world is a latency outcome, not an error.
pub fn construct_pivot(
    strategy: &dyn QuorumStrategy,
    current: &Configuration,
    next: &Configuration,
    leader: NodeId,
) -> Option<crate::replica::Pivot> {
    let mut universe: Vec<NodeId> = current
        .order()
        .iter()
        .chain(next.order().iter())
        .map(|member| member.node)
        .collect();
    universe.sort();
    universe.dedup();
    let n = width_of(&universe);
    if n > MAX_MEMBERS {
        return None;
    }
    let full = (1u32 << n) - 1;
    // Enumerate subsets in increasing bitmask order: deterministic.
    for mask in 0..=full {
        let q_ii = members_of(&universe, mask);
        if !q_ii.contains(&leader) {
            continue;
        }
        if !strategy.is_quorum(Role::Commit, current, &q_ii) {
            continue;
        }
        if !strategy.is_quorum(Role::Commit, next, &q_ii) {
            continue;
        }
        // qI = (universe \ qII) ∪ {leader}
        let mut q_i: Vec<NodeId> = universe
            .iter()
            .filter(|node| !q_ii.contains(node))
            .copied()
            .collect();
        q_i.push(leader);
        q_i.sort();
        q_i.dedup();
        if !strategy.is_quorum(Role::ViewChange, current, &q_i) {
            continue;
        }
        // Cardinality: |qI| + |qII| = N + 1
        if q_i.len() + q_ii.len() != universe.len() + 1 {
            continue;
        }
        return Some(crate::replica::Pivot { q_i, q_ii });
    }
    None
}

/// Validates a host-supplied pivot against the pivot condition (§8.7.6).
///
/// Every leg is checked independently and the first failure is named.
/// A refusal is bad input, never a fault: the reconfigure request is
/// rejected before the operation enters the log.
pub fn validate_pivot(
    strategy: &dyn QuorumStrategy,
    current: &Configuration,
    next: &Configuration,
    leader: NodeId,
    pivot: &crate::replica::Pivot,
) -> Result<(), PivotError> {
    // Unique members within each set.
    for (index, node) in pivot.q_i.iter().enumerate() {
        if pivot.q_i[..index].contains(node) {
            return Err(PivotError::DuplicateMember(*node));
        }
    }
    for (index, node) in pivot.q_ii.iter().enumerate() {
        if pivot.q_ii[..index].contains(node) {
            return Err(PivotError::DuplicateMember(*node));
        }
    }
    // Leader present in both.
    if !pivot.q_i.contains(&leader) || !pivot.q_ii.contains(&leader) {
        return Err(PivotError::LeaderAbsent);
    }
    // Intersection exactly {leader}.
    let shared: Vec<NodeId> = pivot
        .q_i
        .iter()
        .filter(|node| pivot.q_ii.contains(node))
        .copied()
        .collect();
    if shared != [leader] {
        return Err(PivotError::IntersectionNotLeader);
    }
    // qI legal under config(e).
    if !strategy.is_quorum(Role::ViewChange, current, &pivot.q_i) {
        return Err(PivotError::QiNotLegal);
    }
    // qII legal under config(e).
    if !strategy.is_quorum(Role::Commit, current, &pivot.q_ii) {
        return Err(PivotError::QiiNotLegalCurrent);
    }
    // qII legal under config(e+1).
    if !strategy.is_quorum(Role::Commit, next, &pivot.q_ii) {
        return Err(PivotError::QiiNotLegalNext);
    }
    Ok(())
}

/// The shipped default: strict weighted majority, `floor(T/2) + 1`, for every role
/// (§8.4). All four roles coincide — the match is exhaustive rather than a wildcard
/// so that a fifth role is a compile error here, not a silently defaulted case.
///
/// §8.7.5 proves its closure across consecutive eras under the §8.7.2 operation
/// alphabet, and `tests/quorum_contract.rs` re-runs that proof computationally over
/// every configuration reachable at small sizes, which is why this strategy can be
/// the default rather than merely a suggestion.
#[derive(Clone, Copy, Debug, Default)]
pub struct WeightedMajority;

impl QuorumStrategy for WeightedMajority {
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool {
        let threshold = match role {
            Role::Commit | Role::ViewChange | Role::Restart | Role::Fence => config.total() / 2 + 1,
        };
        // Duplicates and unknown members are refused by `weight_of_set`, which is the
        // single copy of that rule — a strategy that deduplicated for itself would
        // hold a second one.
        match config.weight_of_set(members) {
            Some(weight) => weight >= threshold,
            None => false,
        }
    }

    fn threshold(&self, role: Role, config: &Configuration) -> Option<u64> {
        match role {
            Role::Commit | Role::ViewChange | Role::Restart | Role::Fence => {
                Some(config.total() / 2 + 1)
            }
        }
    }
}
