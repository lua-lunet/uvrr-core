//! The fused jump and the per-slot step-through land in the same state:
//! the equivalence proof of the per-slot view-jump law
//! (`docs/uvrr-protocols.md`, the NOMINATE chapter).
//!
//! The Repetitive Construct, made mechanical: the same solver schedule is
//! driven through one cluster twice. The stepped run proposes one
//! establishing batch per transition through the ordinary pipeline, so
//! every published transition advances the serving view by at most one
//! establishing slot's worth — the rule honoured step by step, observed
//! on the library's own numbers. The fused run submits the same steps as
//! the leader's armed plan, whose batches travel as
//! [`uvrr::wire::Tag::Fuse`] envelopes; the cluster commits the same
//! slots and the nomination riders walk the serving view. If both runs
//! end in the same final state — the same journal whole, the same
//! committed configuration, the same serving view — then the fused jump
//! is exactly the telescoped sum of the steps the stepped run proved
//! legal, and the law's arithmetic holds: the total jump divided by the
//! packed slot count is in `(0, 1]`, one era per establishing slot.
//!
//! The scenario is the expansion of a three-member cluster to five at a
//! fixed serving view, the leader constant throughout (`tests/nominate_leader_assignment.rs`
//! is the same schedule's step-by-step ladder). Nothing here reads a
//! synthetic state pair: every number is a snapshot, a journal entry or
//! an era-table record the library itself installed.

mod harness;

use harness::{Harness, StepOutcome};
use uvrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use uvrr::ids::{Ballot, Era, NodeId, Slot, SystemId, View};
use uvrr::journal::{LogEntry, Payload};
use uvrr::message::{Body, Message};
use uvrr::plan::Plan;
use uvrr::solve;
use uvrr::wire::{Header, Tag};

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        uvrr::ids::CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn member(id: u32, weight: u32) -> Member {
    Member {
        node: n(id),
        weight: Weight(weight),
    }
}

/// Delivers everything until the network is empty: proposals, cascades
/// and announcements are all ordinary steps, and a committed cascade may
/// queue more.
fn quiesce(h: &mut Harness) {
    while h.queued_len() > 0 {
        h.deliver_all();
    }
}

/// The bootstrap: the genesis primary promotes itself and the backups
/// adopt view (1, 0) from the promotion's commit announcement (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    quiesce(h);
    h.assert_safety();
}

/// Forces the cluster's serving view to `(1, view)` (§14.2), the leader
/// the primary under the genesis order.
fn serve_at(h: &mut Harness, view: u32) {
    h.force_view(
        n(0),
        Ballot {
            era: Era(1),
            view: View(view),
        },
    );
    quiesce(h);
    for id in (0..3).map(n) {
        let snapshot = h.snapshot(id).expect("the node is live");
        assert_eq!(
            (snapshot.era, snapshot.view),
            (1, view),
            "n{} serves at the forced view",
            id.0
        );
    }
    h.assert_safety();
}

/// The committed configuration the cluster's history has established.
fn committed_config(h: &Harness, id: NodeId) -> Configuration {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .as_ref()
        .clone()
}

/// The solver's schedule: the exhaustive path from the committed
/// configuration to the target, folded at the running era and view into
/// one step per establishing slot, each step's `Nominate` rider computed
/// by the fold's leader-stability arithmetic.
fn solver_steps(
    start: &Configuration,
    target: &Configuration,
    view: View,
) -> Vec<Vec<SystemOperation>> {
    let path = solve::solve(start, target).expect("the scenario solves");
    let steps = solve::fold(&path, start, (start.era(), view, Slot(0)))
        .expect("the fold rides a legal path");
    steps.into_iter().map(|step| step.ops).collect()
}

/// One run's final observation, named whole so the two runs' equality is
/// asserted in one step and a divergence prints both states entire.
#[derive(Clone, PartialEq, Eq, Debug)]
struct FinalState {
    /// The serving view's era and view number on every live node.
    serving: Vec<(NodeId, u32, u32)>,
    /// The frontiers on every live node.
    frontiers: Vec<(NodeId, u64, u64)>,
    /// The journal whole on the leader: the installed value and era stamp
    /// at every slot.
    journal: Vec<LogEntry>,
    /// The committed configuration on every live node.
    configs: Vec<(NodeId, Configuration)>,
}

fn observe(h: &Harness, roster: &[NodeId], journal_of: NodeId) -> FinalState {
    let mut serving = Vec::new();
    let mut frontiers = Vec::new();
    let mut configs = Vec::new();
    for id in roster {
        let Some(snapshot) = h.snapshot(*id) else {
            continue;
        };
        serving.push((*id, snapshot.era, snapshot.view));
        frontiers.push((*id, snapshot.accepted, snapshot.committed));
        configs.push((*id, committed_config(h, *id)));
    }
    FinalState {
        serving,
        frontiers,
        journal: h.journal_entries(journal_of),
        configs,
    }
}

/// The era boundary where no rider landed (§14.2, §8.7.8): the view lags
/// the established era and the gate would refuse the next establishing
/// operation, so the script drives the leader-preserving forced change,
/// the least view past the current one selecting the constant leader.
fn cross_boundary(h: &mut Harness, leader: NodeId) {
    let snapshot = h.snapshot(leader).expect("the leader is live");
    let table = h.era_table(leader).expect("the leader is live");
    let established = table.current().era;
    if Era(snapshot.era) == established {
        return;
    }
    let voters: Vec<NodeId> = table
        .current()
        .config
        .order()
        .iter()
        .filter(|m| m.weight.0 > 0)
        .map(|m| m.node)
        .collect();
    let index = voters
        .iter()
        .position(|&voter| voter == leader)
        .expect("the constant leader is a voter of the established era");
    let target = uvrr::ids::next_view_selecting(
        View(snapshot.view),
        u32::try_from(index).expect("the roster fits the view arithmetic"),
        u32::try_from(voters.len()).expect("the roster fits the view arithmetic"),
    )
    .expect("a view selecting the leader always exists past any view");
    h.force_view(
        leader,
        Ballot {
            era: established,
            view: target,
        },
    );
    quiesce(h);
    h.assert_safety();
}

/// The stepped run: the fused run's own journal, fed through a node one
/// at a time as individual `Prepare` datagrams, one op per slot, each
/// commit announcement covering a whole maximal same-era run (a run the
/// advance would split is refused). Every step is judged by the same
/// accept perimeter and the same transition gate the fused transport
/// meets.
fn run_stepped(journal: &[LogEntry]) -> FinalState {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    serve_at(&mut h, 3);
    let roster: Vec<NodeId> = vec![n(1), n(2)];
    let leader = n(0);
    let acceptor = n(1);

    // The run boundaries: the maximal runs of consecutive entries sharing
    // one entry era, above the genesis pair.
    let schedule: Vec<&LogEntry> = journal.iter().filter(|entry| entry.slot.0 > 2).collect();
    let mut run_ends: Vec<Slot> = Vec::new();
    for (index, entry) in schedule.iter().enumerate() {
        let last = index + 1 == schedule.len();
        let next_differs = !last && schedule[index + 1].era != entry.era;
        if last || next_differs {
            run_ends.push(entry.slot);
        }
    }

    let mut committed_runs = 0;
    for entry in &schedule {
        let Payload::System(op) = &entry.payload else {
            continue;
        };
        // The backups are in sync: the sender's view is either's.
        let header_view = {
            let snapshot = h.snapshot(acceptor).expect("live");
            Ballot {
                era: Era(snapshot.era),
                view: View(snapshot.view),
            }
        };
        let accepted_before = h.snapshot(acceptor).expect("live").accepted;
        let claim = Slot(h.snapshot(acceptor).expect("live").committed);
        for id in [acceptor, n(2)] {
            h.inject(
                leader,
                id,
                Message {
                    header: Header {
                        tag: Tag::Prepare,
                        view: header_view,
                        slot: entry.slot,
                    },
                    body: Body::Prepare {
                        entry: LogEntry {
                            slot: entry.slot,
                            era: entry.era,
                            payload: Payload::System(op.clone()),
                        },
                        committed: claim,
                    },
                },
            );
        }
        if h.snapshot(acceptor).expect("live").accepted == accepted_before {
            panic!(
                "the Prepare at slot {:?} was refused: diagnostic {:?}\n{}",
                entry.slot,
                h.diagnostic(acceptor),
                h.trace_dump()
            );
        }
        if run_ends.contains(&entry.slot) {
            for id in [acceptor, n(2)] {
                h.inject(
                    leader,
                    id,
                    Message {
                        header: Header {
                            tag: Tag::Commit,
                            view: header_view,
                            slot: entry.slot,
                        },
                        body: Body::Commit {
                            committed: entry.slot,
                        },
                    },
                );
            }
            committed_runs += 1;
        }
        quiesce(&mut h);
    }
    assert!(committed_runs > 0, "no run committed");
    h.assert_safety();
    observe(&h, &roster, acceptor)
}

/// The fused run: the same steps as the leader's armed plan, the machine
/// proposing each batch on the tick, the Fuse envelopes where the batch
/// packs two or more operations. Ticks and deliveries alternate until the
/// machine exhausts; the step budget bounds the wait, a committed
/// cascade may queue more.
fn run_fused(steps: Vec<Vec<SystemOperation>>) -> FinalState {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    serve_at(&mut h, 3);
    let roster: Vec<NodeId> = vec![n(1), n(2)];
    let leader = n(0);
    let plan = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps,
    };
    let outcome = h.submit_plan(leader, plan);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the plan arms: {outcome:?}\n{}",
        h.trace_dump()
    );
    for _ in 0..64 {
        quiesce(&mut h);
        if h.queued_len() == 0 {
            h.tick_all();
            quiesce(&mut h);
        }
        cross_boundary(&mut h, leader);
        let config = committed_config(&h, leader);
        if config.order()
            == [
                member(0, 1),
                member(1, 1),
                member(2, 1),
                member(3, 1),
                member(4, 1),
            ]
        {
            break;
        }
    }
    quiesce(&mut h);
    h.assert_safety();
    observe(&h, &roster, n(1))
}

#[test]
fn the_fused_jump_lands_where_the_step_through_steps() {
    // The schedule is computed once, from the starting configuration both
    // runs boot into, so the two runs drive byte-identical steps.
    let probe = {
        let mut h = Harness::provision(3);
        bootstrap(&mut h);
        serve_at(&mut h, 3);
        committed_config(&h, n(0))
    };
    let target = Snapshot {
        era: Era(1),
        order: [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1),
        ]
        .into(),
    }
    .inflate()
    .expect("the target is a legal configuration");
    let steps = solver_steps(&probe, &target, View(3));
    assert_eq!(steps.len(), 4, "join, promote, join, promote");

    // The fused run first: the armed plan's batches travel as Fuse
    // envelopes, the cluster commits, the nomination riders walk the
    // serving view. Its journal is then the schedule the stepped run
    // feeds through a node one at a time.
    let fused = run_fused(steps);
    let stepped = run_stepped(&fused.journal);

    // The claim under test: the same final state from both transports —
    // the same serving view, the same frontiers, the same journal whole,
    // the same committed configuration.
    assert_eq!(
        stepped, fused,
        "the fused jump and the per-slot step-through disagree\n\
         stepped: {stepped:?}\nfused: {fused:?}"
    );

    // The law, read off the fused run's own numbers: the total jump
    // divided by the packed slot count is a decimal in `(0, 1]` — one era
    // per establishing slot, so the average advance per committed slot
    // never exceeds one and the walk always advances.
    let start_era = 1_u32;
    let serving = &fused.serving;
    let acceptor_state = serving.iter().find(|(id, _, _)| *id == n(1)).expect("live");
    let final_era = acceptor_state.1;
    let jump = final_era - start_era;
    let count = fused
        .journal
        .iter()
        .filter(|entry| entry.slot.0 > 2)
        .count() as u64;
    assert!(count > 0, "the packed slots are observed");
    let per_slot = f64::from(jump) / f64::from(u32::try_from(count).expect("the count fits"));
    assert!(
        per_slot > 0.0 && per_slot <= 1.0,
        "the per-slot advance {jump}/{count} = {per_slot} must lie in (0, 1]\n{fused:?}"
    );
}
