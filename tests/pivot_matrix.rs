//! Pivot construction and validation matrix (§8.7.6).
//!
//! The pivot condition splits the old and new memberships into two concrete
//! vote sets whose intersection is exactly the leader:
//!
//! ```text
//! qI ∩ qII = {L}
//! qI  legal under config(e)   — view-change family
//! qII legal under config(e)   — commit family
//! qII legal under config(e+1) — commit family
//! ```
//!
//! For unweighted threshold quorums the cardinality rule is `|qI| + |qII| =
//! N + 1`. A leader that finds no split falls back to stop-the-world without
//! fault. A valid pivot never weakens the closed `validate_transition` gate.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{Configuration, SystemOperation};
use vrr::effects::Stability;
use vrr::ids::{Era, NodeId, View, ViewId};
use vrr::journal::{Journal, SegmentedLog};
use vrr::quorum::{QuorumStrategy, Role, WeightedMajority};
use vrr::replica::{Input, Pivot, PlanRefusal, Replica, ViewChangeKnobs};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// A view in era 1 — the era every node here bootstraps into.
#[allow(dead_code)]
fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// The timeout knob for the view-change scripts.
const TIMEOUT: u64 = 3;

/// A three-node cluster whose view-change machinery is live.
fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// Bootstraps the cluster: the genesis primary promotes itself and both
/// backups adopt view (1, 0) from the promotion's `Commit` announcement.
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

// ---------------------------------------------------------------------
// Construction: the W/X/Y/Z membership sequence from §8.7.6
// ---------------------------------------------------------------------

/// Builds a configuration by applying a sequence of operations to genesis.
fn config_with_weights(order: &[NodeId], weights: &[(NodeId, u32)]) -> Configuration {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, vrr::configuration::VOID_SLOT)
        .expect("Void at slot 1");
    let mut config = void
        .apply(
            &SystemOperation::Init {
                order: order.to_vec(),
            },
            vrr::configuration::INIT_SLOT,
        )
        .expect("Init at slot 2");
    let mut slot = vrr::configuration::INIT_SLOT;
    for &(node, target_weight) in weights {
        let current = config.weight_of(node).expect("member exists").0;
        for _ in current..target_weight {
            slot = slot.next().expect("slot space");
            config = config
                .apply(&SystemOperation::Increment(node), slot)
                .expect("Increment folds");
        }
    }
    config
}

/// The W/X/Y/Z sequence: four nodes, era e has W=2 X=1 Y=1 Z=1, era e+1
/// has W=1 X=1 Y=2 Z=1 (weights inside the {0, 1, 2} domain, R1). The
/// leader W is pivotal: its weight drops from 2 to 1 while Y's rises from
/// 1 to 2, and the pivot split must account for both configurations.
#[test]
fn construction_finds_pivot_in_wxyz_sequence() {
    let w = n(0);
    let x = n(1);
    let y = n(2);
    let z = n(3);

    let current = config_with_weights(&[w, x, y, z], &[(w, 2)]);
    let next = config_with_weights(&[w, x, y, z], &[(y, 2)]);

    // The construction must find a pivot for leader W.
    let pivot = vrr::replica::construct_pivot(&WeightedMajority, &current, &next, w)
        .expect("a pivotal leader finds a split");

    // qI ∩ qII = {W}
    let intersection: Vec<NodeId> = pivot
        .q_i
        .iter()
        .filter(|node| pivot.q_ii.contains(node))
        .copied()
        .collect();
    assert_eq!(
        intersection,
        vec![w],
        "the intersection is exactly the leader"
    );

    // qI is a view-change quorum under config(e)
    assert!(
        WeightedMajority.is_quorum(Role::ViewChange, &current, &pivot.q_i),
        "qI is legal under config(e)"
    );

    // qII is a commit quorum under both configs
    assert!(
        WeightedMajority.is_quorum(Role::Commit, &current, &pivot.q_ii),
        "qII is legal under config(e)"
    );
    assert!(
        WeightedMajority.is_quorum(Role::Commit, &next, &pivot.q_ii),
        "qII is legal under config(e+1)"
    );

    // Cardinality: |qI| + |qII| = N + 1
    assert_eq!(
        pivot.q_i.len() + pivot.q_ii.len(),
        5,
        "unweighted cardinality rule"
    );
}

/// The construction is deterministic: same inputs, same pivot, every time.
#[test]
fn construction_is_deterministic() {
    let w = n(0);
    let x = n(1);
    let y = n(2);
    let z = n(3);

    let current = config_with_weights(&[w, x, y, z], &[(w, 2)]);
    let next = config_with_weights(&[w, x, y, z], &[(y, 2)]);

    let first = vrr::replica::construct_pivot(&WeightedMajority, &current, &next, w);
    let second = vrr::replica::construct_pivot(&WeightedMajority, &current, &next, w);
    assert_eq!(first, second, "same inputs produce the same pivot");
}

/// A low-weight leader with no legal split gets `None` — fallback to
/// stop-the-world, never a fault.
#[test]
fn construction_falls_back_without_fault() {
    let w = n(0);
    let x = n(1);
    let y = n(2);
    let z = n(3);

    // W has weight 1: not pivotal, no legal split exists.
    let current = config_with_weights(&[w, x, y, z], &[]);
    let next = config_with_weights(&[w, x, y, z], &[(y, 2)]);

    let pivot = vrr::replica::construct_pivot(&WeightedMajority, &current, &next, w);
    assert_eq!(pivot, None, "a non-pivotal leader falls back");
}

// ---------------------------------------------------------------------
// Validation: each invalid host pivot shape rejected by name
// ---------------------------------------------------------------------

/// A valid pivot for the W/X/Y/Z sequence, for mutation in the rejection
/// tests below.
fn valid_wxyz_pivot() -> (Configuration, Configuration, Pivot) {
    let w = n(0);
    let x = n(1);
    let y = n(2);
    let z = n(3);

    let current = config_with_weights(&[w, x, y, z], &[(w, 2)]);
    let next = config_with_weights(&[w, x, y, z], &[(y, 2)]);
    let pivot = vrr::replica::construct_pivot(&WeightedMajority, &current, &next, w)
        .expect("construction succeeds");
    (current, next, pivot)
}

#[test]
fn validation_rejects_duplicate_members() {
    let w = n(0);
    let (current, next, mut pivot) = valid_wxyz_pivot();
    pivot.q_i.push(w); // duplicate the leader in qI

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("duplicate members are refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::DuplicateMember(w),
        "named by the duplicated member"
    );
}

#[test]
fn validation_rejects_intersection_not_exactly_leader() {
    let y = n(2);
    let (current, next, mut pivot) = valid_wxyz_pivot();
    // Add Y to qI: Y is already in qII from construction, so the
    // intersection becomes {W, Y}, not {W}.
    pivot.q_i.push(y);

    let w = n(0);
    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("an intersection past the leader is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::IntersectionNotLeader,
        "named by the violation"
    );
}

#[test]
fn validation_rejects_leader_absent_from_qi() {
    let (current, next, mut pivot) = valid_wxyz_pivot();
    let w = n(0);
    pivot.q_i.retain(|&node| node != w);

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("leader absent from qI is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::LeaderAbsent,
        "named by the violation"
    );
}

#[test]
fn validation_rejects_leader_absent_from_qii() {
    let (current, next, mut pivot) = valid_wxyz_pivot();
    let w = n(0);
    pivot.q_ii.retain(|&node| node != w);

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("leader absent from qII is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::LeaderAbsent,
        "named by the violation"
    );
}

#[test]
fn validation_rejects_qi_not_legal_under_current() {
    let (current, next, mut pivot) = valid_wxyz_pivot();
    // Strip qI down to just the leader: weight 2 < threshold 3.
    let w = n(0);
    pivot.q_i = vec![w];

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("a qI that is not a view quorum is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::QiNotLegal,
        "named by the violation"
    );
}

#[test]
fn validation_rejects_qii_not_legal_under_current() {
    let (current, next, mut pivot) = valid_wxyz_pivot();
    // Strip qII down to just the leader: weight 2 < threshold 3 in era e.
    let w = n(0);
    pivot.q_ii = vec![w];

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("a qII that is not a commit quorum under config(e) is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::QiiNotLegalCurrent,
        "named by the violation"
    );
}

#[test]
fn validation_rejects_qii_not_legal_under_next() {
    let (current, next, mut pivot) = valid_wxyz_pivot();
    // A qII legal under config(e) but not config(e+1): {W, X} has weight
    // 2+1=3 in era e (threshold 3) but 1+1=2 in era e+1 (threshold 3).
    let w = n(0);
    let x = n(1);
    pivot.q_ii = vec![w, x];
    // Fix qI to keep the intersection exactly {W}: qI = (members \ qII) ∪ {W}
    pivot.q_i = vec![n(2), n(3), w];

    let refusal = vrr::replica::validate_pivot(&WeightedMajority, &current, &next, w, &pivot)
        .expect_err("a qII that is not a commit quorum under config(e+1) is refused");
    assert_eq!(
        refusal,
        vrr::replica::PivotError::QiiNotLegalNext,
        "named by the violation"
    );
}

// ---------------------------------------------------------------------
// Non-bypass: a valid pivot never weakens validate_transition
// ---------------------------------------------------------------------

/// A test-local threshold strategy: each role admits exactly the sets
/// whose weight meets a fixed threshold. This is the shape of Q1's
/// six-node counterexample — the gate must catch unsafe *policies*.
struct Thresholds {
    commit: u64,
    view_change: u64,
    restart: u64,
    fence: u64,
}

impl Thresholds {
    fn threshold_of(&self, role: Role) -> u64 {
        match role {
            Role::Commit => self.commit,
            Role::ViewChange => self.view_change,
            Role::Restart => self.restart,
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

#[test]
fn valid_pivot_does_not_bypass_transition_gate() {
    // Q1's six-node counterexample: thresholds commit=3, view=4, fence=4,
    // restart=4. INCREMENT of n0 takes the next config to weights
    // [2,1,1,1,1,1] (total 7), where a disjoint view-change quorum
    // {n1,n2,n3,n4} (weight 4, under era e) and commit quorum {n0,n5}
    // (weight 3, under era e+1) exist — the R2 forward violation
    // validate_transition must refuse.
    let order: Vec<NodeId> = (0..6).map(n).collect();
    let strategy = Thresholds {
        commit: 3,
        view_change: 4,
        restart: 4,
        fence: 4,
    };
    let mut replica = Replica::provision(
        n(0),
        order,
        strategy,
        SegmentedLog::new(),
        Stability::Volatile,
        ViewChangeKnobs {
            primary_timeout: 0,
            view_change_budget: usize::MAX,
        },
    )
    .expect("the fixed thresholds are legal WITHIN era 1");
    // Bootstrap the view like the harness does: the genesis primary
    // promotes itself on the first tick.
    let promoted = replica
        .plan(
            &vrr::replica::TimedInput {
                at: vrr::ids::Tick(1),
                event: Input::Tick,
            },
            &replica.journal().view(),
        )
        .expect("the bootstrap tick plans");
    replica
        .publish(promoted)
        .expect("the bootstrap tick installs");

    // A valid pivot for the current configuration (era 1, all weights 1,
    // thresholds commit=3, view=4): qI={n0,n1,n2,n3} (weight 4, view
    // quorum), qII={n0,n4,n5} (weight 3, commit quorum). The intersection
    // is exactly {n0}, and |qI| + |qII| = 7 = N+1.
    let pivot = Pivot {
        q_i: vec![n(0), n(1), n(2), n(3)],
        q_ii: vec![n(0), n(4), n(5)],
    };

    // The unsafe operation with a valid pivot: the pivot validation
    // passes, but the transition gate still refuses.
    let view = replica.journal().view();
    let refusal = replica
        .plan(
            &vrr::replica::TimedInput {
                at: vrr::ids::Tick(2),
                event: Input::Reconfigure {
                    op: SystemOperation::Increment(n(0)),
                    pivot: Some(pivot),
                },
            },
            &view,
        )
        .expect_err("the transition gate refuses, pivot or no pivot");
    assert!(
        matches!(refusal, PlanRefusal::ReconfigureQuorum(_)),
        "the refusal is the transition gate's, not the pivot's: {refusal:?}"
    );
}

// ---------------------------------------------------------------------
// Integration: plan_reconfigure with a valid pivot
// ---------------------------------------------------------------------

#[test]
fn plan_reconfigure_accepts_valid_pivot() {
    let mut h = cluster();
    bootstrap(&mut h);

    // A legal reconfiguration with a valid pivot: Increment(n1) takes the
    // next config to weights [1, 2, 1] (total 4, threshold 3). The pivot
    // qI={n0,n2}, qII={n0,n1} has qII weight 3 under the next config and
    // weight 2 under the current, qI weight 2 under the current, and
    // |qI| + |qII| = 4 = N+1.
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Increment(n(1)),
        Some(Pivot {
            q_i: vec![n(0), n(2)],
            q_ii: vec![n(0), n(1)],
        }),
    );
    // The operation is legal (Increment of a member), so the transition
    // gate passes. The pivot is valid for this configuration. The proposal
    // should be planned, not refused.
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "a valid pivot on a legal transition plans: {outcome:?}"
    );
}

/// Enumerates both quorum families through their public predicates. The next
/// configuration may admit or remove an identity, so unknown members must be
/// rejected by the predicates rather than treated as zero-weight members.
fn legal_boundary_quorum_pairs(current: &Configuration, next: &Configuration) -> Vec<Pivot> {
    let mut members: Vec<_> = current
        .order()
        .iter()
        .chain(next.order())
        .map(|member| member.node)
        .collect();
    members.sort();
    members.dedup();
    let subsets: Vec<Vec<NodeId>> = (0..(1usize << members.len()))
        .map(|mask| {
            members
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, node)| *node)
                .collect()
        })
        .collect();
    let mut pairs = Vec::new();
    for q_i in &subsets {
        if !WeightedMajority.is_quorum(Role::ViewChange, current, q_i) {
            continue;
        }
        for q_ii in &subsets {
            if WeightedMajority.is_quorum(Role::Commit, current, q_ii)
                && WeightedMajority.is_quorum(Role::Commit, next, q_ii)
            {
                pairs.push(Pivot {
                    q_i: q_i.clone(),
                    q_ii: q_ii.clone(),
                });
            }
        }
    }
    pairs
}

fn minimum_overlap_without(pairs: &[Pivot], failed: NodeId) -> Option<usize> {
    pairs
        .iter()
        .filter(|pair| !pair.q_i.contains(&failed) && !pair.q_ii.contains(&failed))
        .map(|pair| {
            pair.q_i
                .iter()
                .filter(|node| pair.q_ii.contains(node))
                .count()
        })
        .min()
}

/// Turner gives the seven weighted rows for a three-node replacement in his
/// consensus-membership reconfiguration notes (raft-like-reconfiguration
/// branch, the weighted-rows section).
/// This checks their five-node extension against this crate's pivot contract:
/// qI is legal in the current era; qII is legal in both eras. Safety of all
/// six boundaries is distinct from availability of a live singleton split.
/// In particular, doubling cannot restore such a split after the old node
/// fails: both quorums still need three of the four original survivors.
#[test]
fn five_node_weighted_replacement_checks_safety_and_live_pivots_at_every_boundary() {
    let old = n(4);
    let new = n(5);
    let survivors = [n(0), n(1), n(2), n(3)];
    let initial = config_with_weights(&[n(0), n(1), n(2), n(3), old], &[]);
    let steps = vrr::replica::forced_steps(&initial, old, new);
    assert_eq!(
        steps.len(),
        6,
        "the full weighted replacement has six boundaries"
    );

    let mut current = initial.clone();
    let mut slot = vrr::configuration::INIT_SLOT;
    let expected_failed_overlap = [2, 2, 2, 2, 1, 1];
    for (boundary, step) in steps.iter().enumerate() {
        slot = slot.next().expect("slot space");
        let next = current.apply(step, slot).expect("the weighted step folds");
        vrr::quorum::validate_transition(&WeightedMajority, &current, &next)
            .expect("every weighted boundary preserves agreement safety");
        let pairs = legal_boundary_quorum_pairs(&current, &next);
        for leader in survivors {
            let valid_count = pairs
                .iter()
                .filter(|pivot| {
                    vrr::replica::validate_pivot(&WeightedMajority, &current, &next, leader, pivot)
                        .is_ok()
                })
                .count();
            assert!(
                valid_count > 0,
                "healthy survivor {leader:?} has a singleton split at boundary {boundary}"
            );
        }
        assert_eq!(
            minimum_overlap_without(&pairs, old),
            Some(expected_failed_overlap[boundary]),
            "minimum live overlap after old fails at boundary {boundary}"
        );
        current = next;
    }

    // The shortened unit schedule is safe too, but its first boundary has
    // the same obstruction: no singleton split among the four survivors.
    let shortened = initial
        .apply(
            &SystemOperation::Batch(vec![
                SystemOperation::Decrement(old),
                SystemOperation::Join {
                    node: new,
                    position: 4,
                },
            ]),
            vrr::configuration::INIT_SLOT.next().expect("slot space"),
        )
        .expect("the shortened first boundary folds");
    vrr::quorum::validate_transition(&WeightedMajority, &initial, &shortened)
        .expect("the shortened first boundary preserves agreement safety");
    assert_eq!(
        minimum_overlap_without(&legal_boundary_quorum_pairs(&initial, &shortened), old),
        Some(2),
        "safety alone does not establish a live singleton split"
    );
}
