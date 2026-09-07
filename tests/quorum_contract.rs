//! Contract for `vrr::quorum` — the `QuorumStrategy` extension point and the closed
//! gate that no strategy can override.
//!
//! Spec §8.3 (the diskless obligations `F_g ⌢ R_g` and `V_g ⌢ V_g`), §8.4 (weighted
//! quorums and the strict-majority family), §8.7.4 (`R1`/`R2`), §8.7.5 (closure of
//! weighted-majority configurations), and decision Q1 (families are open, intersection
//! is closed).
//!
//! What is gated here:
//!
//! 1. **The extension point is real but the gate is closed.** A host-supplied strategy
//!    decides only `is_quorum`; `validate_era`/`validate_transition` are free functions
//!    that mechanically discharge the intersection obligations, and a refusal is final.
//!    Test 1 is Q1's six-node counterexample, detected before the unsafe `INCREMENT`
//!    can be proposed — the whole reason the gate exists.
//! 2. **`WeightedMajority` re-proves §8.7.5 computationally.** Every configuration
//!    reachable from genesis by at most four operations on 3–6 initial members, and
//!    every single-step transition between them, validates. A proof we can re-run is
//!    worth more than a proof we cite.
//! 3. **Every refusal carries a genuine witness.** The two subsets in a `QuorumError`
//!    are disjoint, each is a quorum of its claimed family under its claimed
//!    configuration — re-checked here through the strategy's own `is_quorum` — and
//!    neither has a proper subset that is a quorum (inclusion-minimality), so an
//!    operator is shown exactly the two vote sets that cannot both be legal.
//! 4. **Both directions of the era boundary are checked.** Overlap mode sends era `e`
//!    to one set and era `e+1` to another, so `ViewChange_next ⌢ Commit_current` is
//!    gated alongside the forward `R2`; test 4 constructs a strategy that passes the
//!    forward direction and is refused on the reverse.

use std::collections::BTreeSet;

use vrr::configuration::{Configuration, MAX_MEMBERS, SystemOperation};
use vrr::ids::NodeId;
use vrr::quorum::{
    QuorumError, QuorumStrategy, R2Direction, Role, WeightedMajority, validate_era,
    validate_transition,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const N0: NodeId = NodeId(0);
const N1: NodeId = NodeId(1);
const N2: NodeId = NodeId(2);
const N3: NodeId = NodeId(3);
const N4: NodeId = NodeId(4);
const N5: NodeId = NodeId(5);

/// The six unit-weight members of Q1's counterexample profile.
const SIX: [NodeId; 6] = [N0, N1, N2, N3, N4, N5];

/// A void configuration followed by `Init` over `order`, at the two genesis slots —
/// the only way a `Configuration` can come to exist, which is the property under test
/// in `configuration_contract.rs` and the precondition for every argument here.
fn initialised(order: &[NodeId]) -> Configuration {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, vrr::configuration::VOID_SLOT)
        .expect("Void at slot 1 on the void configuration");
    void.apply(
        &SystemOperation::Init {
            order: order.to_vec(),
        },
        vrr::configuration::INIT_SLOT,
    )
    .expect("Init at slot 2 immediately after Void")
}

/// A test-local threshold strategy: each role admits exactly the sets whose weight
/// meets a fixed threshold. This is the shape of every deliberate violation below —
/// the gate must catch unsafe *policies*, and a threshold table is how an operator
/// would actually write one.
struct Thresholds {
    /// Weight threshold for `Role::Commit` (QII).
    commit: u64,
    /// Weight threshold for `Role::ViewChange` (QI).
    view_change: u64,
    /// Weight threshold for `Role::Recovery` (R_g).
    recovery: u64,
    /// Weight threshold for `Role::Fence` (F_g).
    fence: u64,
}

impl Thresholds {
    fn threshold_of(&self, role: Role) -> u64 {
        match role {
            Role::Commit => self.commit,
            Role::ViewChange => self.view_change,
            Role::Recovery => self.recovery,
            Role::Fence => self.fence,
        }
    }
}

impl QuorumStrategy for Thresholds {
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool {
        match config.weight_of_set(members) {
            Some(weight) => weight >= self.threshold_of(role),
            None => false,
        }
    }

    fn threshold(&self, role: Role, _config: &Configuration) -> Option<u64> {
        Some(self.threshold_of(role))
    }
}

/// The members of `subset` that are present, as a mask over `universe`.
fn members_of(universe: &[NodeId], mask: u32) -> Vec<NodeId> {
    universe
        .iter()
        .zip(0u32..)
        .filter(|(_, bit)| mask & (1u32 << bit) != 0)
        .map(|(node, _)| *node)
        .collect()
}

/// Re-checks a gate-produced witness through the strategy's own predicate: the two
/// subsets must be disjoint, each must genuinely be a quorum of its claimed family
/// under its claimed configuration, and no proper subset of either may be a quorum —
/// a witness that could be trimmed would send an operator chasing members that are
/// not part of the violation.
fn assert_genuine_witness(
    strategy: &dyn QuorumStrategy,
    claimed_a: (Role, &Configuration, &[NodeId]),
    claimed_b: (Role, &Configuration, &[NodeId]),
) {
    let (role_a, config_a, a) = claimed_a;
    let (role_b, config_b, b) = claimed_b;

    assert!(
        a.iter().all(|node| !b.contains(node)),
        "witness subsets must be disjoint: {a:?} vs {b:?}"
    );
    assert!(
        strategy.is_quorum(role_a, config_a, a),
        "{a:?} must be a {role_a:?} quorum"
    );
    assert!(
        strategy.is_quorum(role_b, config_b, b),
        "{b:?} must be a {role_b:?} quorum"
    );

    for (role, config, witness) in [(role_a, config_a, a), (role_b, config_b, b)] {
        let full = (1u32 << witness.len()) - 1;
        for mask in 0..full {
            let subset = members_of(witness, mask);
            assert!(
                !strategy.is_quorum(role, config, &subset),
                "proper subset {subset:?} of witness {witness:?} must not be a {role:?} quorum"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Q1's counterexample, mechanically detected
// ---------------------------------------------------------------------------

/// The six-node profile of Q1: commit threshold 3, view-change threshold 4, unit
/// weights, `T = 6`. Within era `e` the families intersect (`3 + 4 > 6`), so
/// `validate_era` accepts the strategy — the danger is exclusively cross-era, which is
/// why a per-era check cannot substitute for the transition gate.
fn q1_scenario() -> (Thresholds, Configuration, Configuration) {
    let strategy = Thresholds {
        commit: 3,
        view_change: 4,
        recovery: 4,
        fence: 4,
    };
    let current = initialised(&SIX);
    let next = current
        .apply(&SystemOperation::Increment(N0), vrr::ids::Slot(3))
        .expect("Increment on a member");
    (strategy, current, next)
}

/// `INCREMENT(n0)` against the six-node profile must be refused as `R2Violation` with
/// the witness Q1 worked out by hand: view-change quorum `{n1,n2,n3,n4}` under era `e`
/// against commit quorum `{n0,n5}` under era `e+1`. Detecting this after the
/// reconfiguration commits is worthless — the divergence is already reachable.
#[test]
fn q1_counterexample_is_refused_with_its_witness() {
    let (strategy, current, next) = q1_scenario();

    validate_era(&strategy, &current).expect("3 + 4 > 6: the era itself is legal");

    let refusal = validate_transition(&strategy, &current, &next)
        .expect_err("Q1's unsafe INCREMENT must be refused");
    match refusal {
        QuorumError::R2Violation {
            direction,
            view_change,
            commit,
        } => {
            assert_eq!(direction, R2Direction::Forward);
            assert_eq!(view_change, vec![N1, N2, N3, N4]);
            assert_eq!(commit, vec![N0, N5]);
        }
        other => panic!("expected R2Violation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. WeightedMajority always validates — §8.7.5, re-run
// ---------------------------------------------------------------------------

/// Every configuration reachable from genesis by at most four §8.7.2 operations on
/// 3–6 initial members, together with every single-step edge between a reached
/// configuration and its successor. This is the domain over which §8.7.5's closure
/// lemmas are re-verified computationally.
///
/// `Join` inserts at position 0 only, and always the same fresh id per depth: the
/// succession position and the identity of a zero-weight learner change no quorum
/// family (§8.4: weight 0 grants no voting authority), so enumerating positions and
/// fresh ids would multiply the work without enlarging the set of *families* under
/// test. Everything else — the member every weight operation names — is enumerated in
/// full.
fn reachable() -> (Vec<Configuration>, Vec<(Configuration, Configuration)>) {
    let mut configurations = Vec::new();
    let mut edges = Vec::new();

    for size in 3u32..=6 {
        let nodes: Vec<NodeId> = (0..size).map(NodeId).collect();
        let genesis = initialised(&nodes);

        let mut seen: BTreeSet<Vec<(NodeId, u32)>> = BTreeSet::new();
        let shape = |config: &Configuration| -> Vec<(NodeId, u32)> {
            config
                .order()
                .iter()
                .map(|member| (member.node, member.weight.0))
                .collect()
        };
        seen.insert(shape(&genesis));
        configurations.push(genesis.clone());

        // (configuration, depth): depth counts operations from genesis.
        let mut frontier: Vec<(Configuration, u32)> = vec![(genesis, 0)];
        while let Some((config, depth)) = frontier.pop() {
            if depth == 4 {
                continue;
            }
            let fresh = NodeId(1000 + depth);
            let mut alphabet: Vec<SystemOperation> = vec![
                SystemOperation::Double,
                SystemOperation::Halve,
                SystemOperation::Join {
                    node: fresh,
                    position: 0,
                },
            ];
            for member in config.order() {
                alphabet.push(SystemOperation::Increment(member.node));
                alphabet.push(SystemOperation::Decrement(member.node));
                alphabet.push(SystemOperation::Leave(member.node));
            }
            for op in &alphabet {
                // Any non-genesis slot: the fold consults `at` only for the two
                // genesis ordinals.
                let Ok(next) = config.apply(op, vrr::ids::Slot(3)) else {
                    continue;
                };
                edges.push((config.clone(), next.clone()));
                if seen.insert(shape(&next)) {
                    configurations.push(next.clone());
                    frontier.push((next, depth + 1));
                }
            }
        }
    }

    (configurations, edges)
}

/// The computational re-verification of §8.7.5: `validate_era` passes on every
/// reachable configuration and `validate_transition` passes on every single-step edge
/// — both directions of the boundary, since the gate checks both.
#[test]
fn weighted_majority_validates_every_reachable_era_and_transition() {
    let strategy = WeightedMajority;
    let (configurations, edges) = reachable();

    assert!(
        configurations.len() > 100,
        "the enumeration must be substantive, got {}",
        configurations.len()
    );
    for config in &configurations {
        validate_era(&strategy, config)
            .unwrap_or_else(|refusal| panic!("era refused for {config:?}: {refusal:?}"));
    }
    for (current, next) in &edges {
        validate_transition(&strategy, current, next).unwrap_or_else(|refusal| {
            panic!("transition refused for {current:?} -> {next:?}: {refusal:?}")
        });
    }
}

// ---------------------------------------------------------------------------
// 3. Obligation isolation — one violated obligation, one specific variant
// ---------------------------------------------------------------------------

/// A commit family that admits singletons violates `R1` even when every other family
/// is a strict majority: `{n0}` commits while `{n1,n2}` changes the view. The refusal
/// must be `R1Violation`, and `validate_era` must reach it — the self-intersection and
/// fence/recovery checks hold for this strategy, so the variant cannot be arriving
/// from a different obligation.
#[test]
fn undersized_commit_family_is_an_r1_violation() {
    let strategy = Thresholds {
        commit: 1,
        view_change: 2,
        recovery: 2,
        fence: 2,
    };
    let config = initialised(&[N0, N1, N2]);

    let refusal =
        validate_era(&strategy, &config).expect_err("a singleton commit family must be refused");
    match refusal {
        QuorumError::R1Violation {
            commit,
            view_change,
        } => {
            assert_eq!(commit, vec![N0]);
            assert_eq!(view_change, vec![N1, N2]);
        }
        other => panic!("expected R1Violation, got {other:?}"),
    }
}

/// Over four unit members a view threshold of 2 satisfies `R1` against a commit
/// threshold of 3 (`2 + 3 > 4`) but does not self-intersect: `{n0,n1}` and `{n2,n3}`
/// are disjoint view-change quorums. This is the §8.3 diskless obligation a
/// flexible-quorum reading of `QI ⌢ QII` would miss, and it must surface as
/// `SelfIntersectionViolation`, not as a generic failure.
#[test]
fn non_self_intersecting_view_family_is_named() {
    let strategy = Thresholds {
        commit: 3,
        view_change: 2,
        recovery: 3,
        fence: 3,
    };
    let config = initialised(&[N0, N1, N2, N3]);

    let refusal = validate_era(&strategy, &config)
        .expect_err("a split view family must be refused even with R1 intact");
    match refusal {
        QuorumError::SelfIntersectionViolation {
            role,
            first,
            second,
        } => {
            assert_eq!(role, Role::ViewChange);
            assert_eq!(first, vec![N0, N1]);
            assert_eq!(second, vec![N2, N3]);
        }
        other => panic!("expected SelfIntersectionViolation, got {other:?}"),
    }
}

/// Fence and recovery families of threshold 1 over three members leave `R1` and the
/// view self-intersection intact (both majorities) but admit a disjoint fence/recovery
/// pair — a recovering replica could miss the volatile evidence that a view was fenced
/// (§8.3). The refusal must be `FenceRecoveryViolation`.
#[test]
fn disjoint_fence_and_recovery_families_are_named() {
    let strategy = Thresholds {
        commit: 2,
        view_change: 2,
        recovery: 1,
        fence: 1,
    };
    let config = initialised(&[N0, N1, N2]);

    let refusal = validate_era(&strategy, &config)
        .expect_err("disjoint fence and recovery families must be refused");
    match refusal {
        QuorumError::FenceRecoveryViolation { fence, recovery } => {
            assert_eq!(fence, vec![N0]);
            assert_eq!(recovery, vec![N1]);
        }
        other => panic!("expected FenceRecoveryViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. The reverse direction of R2 is checked
// ---------------------------------------------------------------------------

/// A deliberately constructed strategy that is safe in the forward direction and
/// unsafe in the reverse. View-change threshold is 4; the commit threshold is 3 on
/// even totals and 4 on odd totals.
///
/// Six unit members, `INCREMENT(n0)`: era `e` has `T = 6` (commit threshold 3), era
/// `e+1` has `T = 7` (commit threshold 4).
///
/// Forward (`ViewChange_e ⌢ Commit_{e+1}`) holds: a view quorum carries weight at
/// least 4 under `e`, a commit quorum at least 4 under `e+1`, and disjoint sets would
/// need `4 + 4 = 8` against a combined weight of at most `6 + 1 = 7`.
///
/// Reverse (`ViewChange_{e+1} ⌢ Commit_e`) fails: `{n0,n1,n2}` has weight `2+1+1 = 4`
/// under `e+1` and is disjoint from `{n3,n4,n5}`, weight 3 under `e`. The spec-level
/// argument for gating this direction: `PrepareOk` evidence gathered under era `e+1`
/// must intersect a commit quorum under era `e`, or a view change in the new era can
/// select a history that omits an operation the old era committed.
struct ParityCommit;

impl QuorumStrategy for ParityCommit {
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool {
        let threshold = match role {
            Role::Commit => {
                if config.total() % 2 == 0 {
                    3
                } else {
                    4
                }
            }
            Role::ViewChange | Role::Recovery | Role::Fence => 4,
        };
        match config.weight_of_set(members) {
            Some(weight) => weight >= threshold,
            None => false,
        }
    }

    fn threshold(&self, role: Role, config: &Configuration) -> Option<u64> {
        let threshold = match role {
            Role::Commit => {
                if config.total() % 2 == 0 {
                    3
                } else {
                    4
                }
            }
            Role::ViewChange | Role::Recovery | Role::Fence => 4,
        };
        Some(threshold)
    }
}

/// Both eras are individually legal under `ParityCommit` — `R1`, self-intersection and
/// fence/recovery all hold in each — and the forward transition check passes. The
/// refusal must come from the reverse direction alone, witnessed by `{n0,n1,n2}`
/// against `{n3,n4,n5}`.
#[test]
fn forward_safe_reverse_unsafe_transition_is_refused() {
    let strategy = ParityCommit;
    let current = initialised(&SIX);
    let next = current
        .apply(&SystemOperation::Increment(N0), vrr::ids::Slot(3))
        .expect("Increment on a member");

    validate_era(&strategy, &current).expect("era e is internally legal");
    validate_era(&strategy, &next).expect("era e+1 is internally legal");

    let refusal = validate_transition(&strategy, &current, &next)
        .expect_err("the reverse direction must be refused");
    match refusal {
        QuorumError::R2Violation {
            direction,
            view_change,
            commit,
        } => {
            assert_eq!(direction, R2Direction::Reverse);
            assert_eq!(view_change, vec![N0, N1, N2]);
            assert_eq!(commit, vec![N3, N4, N5]);
        }
        other => panic!("expected R2Violation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Threshold agreement
// ---------------------------------------------------------------------------

/// For `WeightedMajority`, `threshold()` must agree with the quorum family found by
/// brute-force enumeration — the declared threshold and the actual family may not
/// drift apart, because diagnostics and minimal-quorum construction trust the
/// declared value while safety rests on the actual one.
///
/// The precise universal statement is **family agreement**: a set is a quorum iff its
/// weight meets `threshold()`, checked here over every subset. Plain equality between
/// `threshold()` and the smallest weight of any quorum does *not* hold under
/// indivisible weights — `(2,2,0)` is reachable (`DECREMENT`, then `DOUBLE`), has
/// `T = 4` and threshold 3, yet no subset weighs exactly 3, so the lightest quorum
/// weighs 4. §8.4 flags exactly this: the threshold inequality is sufficient but not
/// necessary for every indivisible weight assignment. What is asserted, beyond family
/// agreement, is that the enumerated lightest quorum weighs exactly the smallest
/// *attainable* weight at or above the declared threshold — so the two coincide
/// whenever `floor(T/2) + 1` is attainable, which is every unit-weight case.
#[test]
fn weighted_majority_threshold_matches_enumerated_minimum() {
    let strategy = WeightedMajority;
    let (configurations, _) = reachable();

    for config in &configurations {
        let members: Vec<NodeId> = config.order().iter().map(|member| member.node).collect();
        let full = (1u32 << members.len()) - 1;
        for role in [Role::Commit, Role::ViewChange, Role::Recovery, Role::Fence] {
            let declared = strategy
                .threshold(role, config)
                .expect("WeightedMajority is threshold-expressible");
            let mut lightest_quorum: Option<u64> = None;
            let mut lightest_attainable_at_or_above: Option<u64> = None;
            for mask in 0..=full {
                let subset = members_of(&members, mask);
                let weight = config
                    .weight_of_set(&subset)
                    .expect("an enumerated subset is known and duplicate-free");
                assert_eq!(
                    strategy.is_quorum(role, config, &subset),
                    weight >= declared,
                    "{role:?} family disagrees with threshold {declared} for {subset:?} in {config:?}"
                );
                if weight >= declared {
                    lightest_attainable_at_or_above = Some(
                        lightest_attainable_at_or_above.map_or(weight, |best| best.min(weight)),
                    );
                }
                if strategy.is_quorum(role, config, &subset) {
                    lightest_quorum = Some(lightest_quorum.map_or(weight, |best| best.min(weight)));
                }
            }
            assert_eq!(
                lightest_quorum, lightest_attainable_at_or_above,
                "{role:?}: lightest quorum must weigh the smallest attainable weight \
                 at or above the threshold, for {config:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Cap enforcement (via the fold)
// ---------------------------------------------------------------------------

/// The membership cap is a validation-cost bound, not a protocol limit: 17-member
/// `Init` and `Join` past 16 are refused by the fold with a named cap, so no
/// over-cap `Configuration` can ever reach the gate. At the cap itself, the gate's
/// `2^16` enumeration is expected to be routine — validated here, once, on the
/// boundary.
#[test]
fn membership_cap_is_enforced_by_the_fold_and_the_gate_runs_at_the_cap() {
    let seventeen: Vec<NodeId> = (0..17).map(NodeId).collect();
    let void = Configuration::void()
        .apply(&SystemOperation::Void, vrr::configuration::VOID_SLOT)
        .expect("Void");
    assert_eq!(
        void.apply(
            &SystemOperation::Init {
                order: seventeen.clone()
            },
            vrr::configuration::INIT_SLOT,
        ),
        Err(vrr::configuration::ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS })
    );

    let sixteen: Vec<NodeId> = seventeen[..16].to_vec();
    let at_cap = void
        .apply(
            &SystemOperation::Init {
                order: sixteen.clone(),
            },
            vrr::configuration::INIT_SLOT,
        )
        .expect("Init at exactly the cap");
    assert_eq!(at_cap.len(), MAX_MEMBERS);
    for (position, node) in sixteen.iter().enumerate() {
        assert_eq!(
            at_cap.index_of(*node),
            Some(u32::try_from(position).expect("position < 16"))
        );
    }

    assert_eq!(
        at_cap.apply(
            &SystemOperation::Join {
                node: NodeId(1000),
                position: 0,
            },
            vrr::ids::Slot(3),
        ),
        Err(vrr::configuration::ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS })
    );

    validate_era(&WeightedMajority, &at_cap).expect("the gate runs at the cap");
}

// ---------------------------------------------------------------------------
// 7. Witnesses are real
// ---------------------------------------------------------------------------

/// Every refusal produced above is re-examined: the witness subsets are disjoint,
/// each is a quorum of its claimed family under its claimed configuration — through
/// the strategy's own `is_quorum`, not the gate's word — and neither can be trimmed.
/// A refusal an operator cannot act on is worse than none, and an un-trimmed or
/// fabricated witness is one nobody can act on.
#[test]
fn every_refusal_witness_is_genuine_and_minimal() {
    // Test 1: Q1's counterexample.
    let (q1_strategy, q1_current, q1_next) = q1_scenario();
    match validate_transition(&q1_strategy, &q1_current, &q1_next) {
        Err(QuorumError::R2Violation {
            view_change,
            commit,
            ..
        }) => assert_genuine_witness(
            &q1_strategy,
            (Role::ViewChange, &q1_current, &view_change),
            (Role::Commit, &q1_next, &commit),
        ),
        other => panic!("expected R2Violation, got {other:?}"),
    }

    // Test 3a: undersized commit family.
    let r1_strategy = Thresholds {
        commit: 1,
        view_change: 2,
        recovery: 2,
        fence: 2,
    };
    let three = initialised(&[N0, N1, N2]);
    match validate_era(&r1_strategy, &three) {
        Err(QuorumError::R1Violation {
            commit,
            view_change,
        }) => assert_genuine_witness(
            &r1_strategy,
            (Role::Commit, &three, &commit),
            (Role::ViewChange, &three, &view_change),
        ),
        other => panic!("expected R1Violation, got {other:?}"),
    }

    // Test 3b: non-self-intersecting view family.
    let split_view = Thresholds {
        commit: 3,
        view_change: 2,
        recovery: 3,
        fence: 3,
    };
    let four = initialised(&[N0, N1, N2, N3]);
    match validate_era(&split_view, &four) {
        Err(QuorumError::SelfIntersectionViolation {
            role,
            first,
            second,
        }) => {
            assert_eq!(role, Role::ViewChange);
            assert_genuine_witness(&split_view, (role, &four, &first), (role, &four, &second));
        }
        other => panic!("expected SelfIntersectionViolation, got {other:?}"),
    }

    // Test 3c: disjoint fence/recovery families.
    let split_fr = Thresholds {
        commit: 2,
        view_change: 2,
        recovery: 1,
        fence: 1,
    };
    match validate_era(&split_fr, &three) {
        Err(QuorumError::FenceRecoveryViolation { fence, recovery }) => {
            assert_genuine_witness(
                &split_fr,
                (Role::Fence, &three, &fence),
                (Role::Recovery, &three, &recovery),
            );
        }
        other => panic!("expected FenceRecoveryViolation, got {other:?}"),
    }

    // Test 4: the reverse-direction refusal.
    let reverse_current = initialised(&SIX);
    let reverse_next = reverse_current
        .apply(&SystemOperation::Increment(N0), vrr::ids::Slot(3))
        .expect("Increment on a member");
    match validate_transition(&ParityCommit, &reverse_current, &reverse_next) {
        Err(QuorumError::R2Violation {
            direction,
            view_change,
            commit,
        }) => {
            assert_eq!(direction, R2Direction::Reverse);
            assert_genuine_witness(
                &ParityCommit,
                (Role::ViewChange, &reverse_next, &view_change),
                (Role::Commit, &reverse_current, &commit),
            );
        }
        other => panic!("expected R2Violation, got {other:?}"),
    }
}
