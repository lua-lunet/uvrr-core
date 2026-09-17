//! Witness stream plays: the two exact walk-throughs of
//! `docs/uvrr-reincarnation.md` §7–§8 as message choreography.
//!
//! * **Play A (three nodes)** — n1 isolated, n3 crashes and reincarnates
//!   as a witness; the leader streams phase-2 but cannot commit; the
//!   network heals, the isolated node gap-detects and fetches, its
//!   cumulative ack completes every outstanding slot, the leader commits
//!   in slot order, and the streamed join batches carry the witness to
//!   full membership — it leaves the leader's witness list.
//! * **Play B (five nodes)** — an isolated leader in a minority keeps its
//!   witness current on uncommitted slots; the leader crashes, the
//!   majority elects a new leader; the witness re-announces on its own
//!   timer, the new leader's stream OVERRIDES the dead view's uncommitted
//!   slots, and the recomputed join commits at the new leader's slots.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::observe::Diagnostic;
use vrr::progress::Status;

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// The timeout knob for the view-change scripts (S4).
const TIMEOUT: u64 = 3;

fn knobs() -> vrr::replica::ViewChangeKnobs {
    vrr::replica::ViewChangeKnobs {
        primary_timeout: TIMEOUT,
        view_change_budget: usize::MAX,
    }
}

fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_era(h: &Harness, id: NodeId) -> Era {
    h.era_table(id).expect("the node is live").current().era
}

/// The member weights of the node's current configuration, in order.
fn current_weights(h: &Harness, id: NodeId) -> Vec<u64> {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .order()
        .iter()
        .map(|member| u64::from(member.weight.0))
        .collect()
}

/// The weight the node's current configuration gives `member`, if present.
fn weight_of(h: &Harness, id: NodeId, member: NodeId) -> Option<u64> {
    h.era_table(id)?
        .current()
        .config
        .weight_of(member)
        .map(|weight| u64::from(weight.0))
}

/// The view-change target the fence machinery itself chooses for `node`.
fn fence_target(h: &Harness, node: NodeId) -> ViewId {
    let current = current_view(h, node);
    ViewId {
        era: current_era(h, node),
        view: View(current.view.0 + 1),
    }
}

/// Drives the view change the fence machinery targets, asserting every
/// node in `live` installs it.
fn drive_view_change(h: &mut Harness, live: &[NodeId]) -> ViewId {
    let target = fence_target(h, live[0]);
    let driver = live
        .iter()
        .copied()
        .find(|&id| status_of(h, id) == Status::Normal)
        .expect("a live Normal member drives the fence");
    for _ in 0..=TIMEOUT {
        h.tick(driver);
    }
    h.deliver_all();
    for &id in live {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target, "{id:?} is in the target");
    }
    h.assert_safety();
    target
}

/// Play A: three nodes, n1 isolated, n3 crashes and reincarnates. The
/// leader streams phase-2 to the witness yet cannot commit; the heal
/// brings a gap-detect, a fetch, a cumulative ack, contiguous slot-order
/// commits, and the streamed join batches land the witness as a voter.
///
/// The user's walk, mapped: n1 = n(0), n2(l) = n(1), n3 = n(2),
/// reincarnated n3' = n(3).
#[test]
fn play_a_isolated_backup_reincarnated_witness_heal_and_slot_order_commit() {
    let mut h = Harness::with_knobs(3, knobs());
    bootstrap(&mut h);
    // Leadership on n(1) — the walk's "n2(l)": the first fence moves the
    // cluster to view 1, whose primary is position 1.
    drive_view_change(&mut h, &[n(0), n(1), n(2)]);
    let outcome = h.propose(n(1), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(snap(&h, n(1)).committed, 3, "op1 committed everywhere");

    // n1 isolated; n3 crashes dirty and reincarnates as n(3).
    h.partition(vec![n(0)], vec![n(1), n(2), n(3)]);
    h.crash(n(2));
    h.restart_as(n(2), n(3)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    // The leader acked and streamed the first forced batch
    // `[Decrement(n2), Join(n3)]` at slot 4: the witness folded it; the
    // isolated n(0) never saw it; no quorum exists.
    assert_eq!(
        snap(&h, n(3)).accepted,
        4,
        "the witness folded the streamed batch"
    );
    assert_eq!(
        snap(&h, n(1)).committed,
        3,
        "the leader cannot commit: its only stream partner votes nowhere"
    );
    // More phase-2 streams: a client op at slot 5, same shape.
    let outcome = h.propose(n(1), op_id(2), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(snap(&h, n(3)).accepted, 5, "the witness stays current");
    assert_eq!(snap(&h, n(1)).committed, 3, "still no quorum");

    // The slot-4/5 prepares to n(0) are lost; the heal delivers nothing
    // stale. The next proposal (slot 6) is n(0)'s first sight of the
    // stream: past its frontier, so the contiguity gap rule drops it and
    // opens the fetch — the walk's "requests retransmission".
    h.drop_held();
    h.heal();
    let outcome = h.propose(n(1), op_id(3), b"z");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(0));
    assert!(
        matches!(h.diagnostic(n(0)), Some(Diagnostic::GapDetected { .. })),
        "n(0) gap-detects the high slot: {:?}",
        h.diagnostic(n(0))
    );
    assert_eq!(
        snap(&h, n(1)).committed,
        3,
        "a majority response for the high slot does not exist yet"
    );

    // The fetch is served: the contiguous range 4..=6 folds at n(0).
    h.deliver_all();
    assert_eq!(
        snap(&h, n(0)).accepted,
        6,
        "the fetch filled the gap in order"
    );
    // The next proposal lands cleanly; n(0)'s ack vouches cumulatively
    // for every outstanding slot it covers, and the leader commits the
    // whole tail in slot order — the establishing batch first.
    let outcome = h.propose(n(1), op_id(4), b"w");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        snap(&h, n(1)).committed,
        7,
        "the commit cascade took the contiguous tail in slot order"
    );
    assert_eq!(current_era(&h, n(1)), Era(2), "the join batch committed");
    assert_eq!(
        snap(&h, n(0)).committed,
        7,
        "the healed member holds the same committed prefix"
    );
    assert_eq!(
        snap(&h, n(3)).committed,
        7,
        "the witness holds the live commit stream — current indefinitely"
    );
    assert_eq!(
        current_era(&h, n(3)),
        Era(2),
        "the witness folded the era that will admit it"
    );
    for slot in 1..=7u64 {
        assert_eq!(
            h.journal_entry(n(0), Slot(slot)),
            h.journal_entry(n(1), Slot(slot)),
            "slot {slot}: the healed member's journal is the leader's"
        );
        assert_eq!(
            h.journal_entry(n(1), Slot(slot)),
            h.journal_entry(n(3), Slot(slot)),
            "slot {slot}: the witness's journal is the leader's"
        );
    }

    // The join completes over the stream: the view change into era 2,
    // the re-announce, and the second batch `[Increment(n3), Leave(n2)]`
    // commits — n(3) is a voter and n(2) is gone.
    drive_view_change(&mut h, &[n(0), n(1)]);
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(3), "the promotion committed");
    assert_eq!(
        current_weights(&h, n(0)),
        vec![1, 1, 1],
        "n(3) joined at weight 1; n(2) left"
    );

    // "Left the leader's witness list": its vote is now REQUIRED. In the
    // era-3 unit cluster, isolate one incumbent — the leader and the
    // promoted member are exactly a quorum, and the commit waits on the
    // promoted member's ack.
    drive_view_change(&mut h, &[n(0), n(1), n(3)]);
    let leader = h
        .era_table(n(0))
        .expect("live")
        .current()
        .config
        .primary(current_view(&h, n(0)).view)
        .expect("a primary exists");
    let promoted = n(3);
    let incumbent = [n(0), n(1)]
        .into_iter()
        .find(|&id| id != leader)
        .expect("one incumbent is not the leader");
    h.partition(vec![incumbent], vec![leader, promoted]);
    let before = snap(&h, leader).committed;
    let outcome = h.propose(leader, op_id(5), b"v");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(promoted);
    assert_eq!(
        snap(&h, leader).committed,
        before,
        "no quorum without the promoted member's ack"
    );
    h.deliver_to(leader);
    assert_eq!(
        snap(&h, leader).committed,
        before + 1,
        "the promoted member's vote completes the quorum — it is no witness"
    );
    h.assert_safety();
}

/// Play B: five nodes, the isolated leader and its witness in the
/// minority, keep-up with no commits; the leader crashes, the majority
/// elects; the witness re-announces on its own timer; and the dead view's
/// uncommitted slots on the witness are OVERRIDDEN — not by the memo
/// stream (a boot-fenced witness drops a higher view's stream by design:
/// its only route back is its own fetch through the boot fence), but by
/// the fence into the era that admits it: the StartView suffix install
/// replaces the poisoned tail with the majority's history, and the join
/// lands at slots the new leader chose.
#[test]
fn play_b_isolated_leader_dies_new_leader_overrides_witness_tail() {
    let mut h = Harness::with_knobs(5, knobs());
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 3);

    // n(4) crashes dirty and reincarnates as n(5); the leader acks and
    // arms the machine. The first forced step (DOUBLE) is proposed but
    // not yet delivered anywhere.
    h.crash(n(4));
    h.restart_as(n(4), n(5)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(0));

    // The minority: old leader and witness. The witness plays keep-up on
    // the dead view's uncommitted slots — the DOUBLE at slot 4 and a
    // client op "z" at slot 5.
    h.partition(vec![n(0), n(5)], vec![n(1), n(2), n(3)]);
    h.deliver_all();
    let outcome = h.propose(n(0), op_id(2), b"z");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        snap(&h, n(5)).accepted,
        5,
        "the witness kept up with the isolated leader"
    );
    assert_eq!(
        snap(&h, n(5)).committed,
        3,
        "keep-up is not commit: nothing commits in the minority"
    );

    // The majority suspects and elects: view 1, leader n(1), the
    // surviving committed history (accepted = 3).
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    h.deliver_all();
    for id in [n(1), n(2), n(3)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(&h, id).view, View(1), "{id:?} in view 1");
    }

    // The old leader crashes; the network heals. The witness's own timer
    // re-announces; the new leader acks and recomputes the sequence from
    // the committed configuration (era 1): the first step is DOUBLE again,
    // at slot 4 — the SAME slot the witness filled for the dead view.
    h.crash(n(0));
    h.heal();
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(1));
    // The new leader's slot 5 is a client op "y" — a DIFFERENT value
    // from the dead view's uncommitted "z" the witness holds there.
    let outcome = h.propose(n(1), op_id(3), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        snap(&h, n(1)).committed,
        5,
        "the majority commits the recomputed DOUBLE and the new slot 5"
    );

    // The witness CANNOT fold the new view's memo stream: it is
    // boot-fenced at the dead view, and a higher view's Prepare/Commit is
    // a ViewMismatch drop by design (its route back is its own fetch, not
    // view adoption). Its announced frontier (5) is AHEAD of the leader's
    // committed frontier, so the ack's missed-range push is empty — the
    // divergent tail is invisible to the leader. For now the witness is
    // simply behind; it votes nowhere, so this is availability, not safety.
    assert_eq!(
        snap(&h, n(5)).committed,
        3,
        "the memo stream is same-view-only: the witness cannot ride it across the view change"
    );

    // The recomputed sequence commits at the majority: the view change
    // into era 2, the re-announce, and `[Join(n5), Increment(n5)]`
    // establishes era 3 — the new join attempt at slots the new leader
    // chose.
    drive_view_change(&mut h, &[n(1), n(2), n(3)]);
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        weight_of(&h, n(1), n(5)),
        Some(1),
        "the new join attempt committed at the new leader's slots"
    );

    // The override. The memo stream is same-view-only, so the fence into
    // the era the committed join established is the witness's convergence
    // point: the era_proof on the view-change evidence lets it fold the
    // establishing batch, the StartView suffix install REPLACES the dead
    // view's uncommitted tail with the majority's history — slot 5's "z"
    // becomes the committed "y" — and the witness lands as a voter.
    drive_view_change(&mut h, &[n(1), n(2), n(3)]);
    for _ in 0..4 {
        h.tick_all();
        h.deliver_all();
    }
    assert_eq!(
        status_of(&h, n(5)),
        Status::Normal,
        "the witness converged through the fence: {:?}",
        status_of(&h, n(5))
    );
    for slot in 1..=5u64 {
        assert_eq!(
            h.journal_entry(n(1), Slot(slot)),
            h.journal_entry(n(5), Slot(slot)),
            "slot {slot}: the majority's history overrode the dead view's uncommitted tail"
        );
    }
    h.assert_safety();
}

/// The continuation Play B does not narrate: the forced sequence is NOT
/// done at the join — the old identity still owes its eviction batches
/// (Decrement, Leave, …). The next re-announce arms the next batch while
/// the witness (a voter since the join) has not yet folded the era the
/// next fence targets. The fence must still complete: a reincarnation
/// sequence that wedges the cluster one era after the join is a liveness
/// bug, not a scheduling artefact.
#[test]
fn play_b_tail_sequence_fence_completes_with_promoted_witness_behind() {
    let mut h = Harness::with_knobs(5, knobs());
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();

    h.crash(n(4));
    h.restart_as(n(4), n(5)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(0));
    h.partition(vec![n(0), n(5)], vec![n(1), n(2), n(3)]);
    h.deliver_all();
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    h.deliver_all();
    h.crash(n(0));
    h.heal();
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(1));
    let outcome = h.propose(n(1), op_id(3), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    drive_view_change(&mut h, &[n(1), n(2), n(3)]);
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        weight_of(&h, n(1), n(5)),
        Some(1),
        "setup: the join attempt committed at the new leader's slots"
    );

    // The tail: re-announce to arm the old identity's next eviction batch,
    // drive the next fence before the witness has folded the new era, and
    // the fence must still complete within bounded driving.
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    drive_view_change(&mut h, &[n(1), n(2), n(3)]);
    {
        let table = h.era_table(n(1)).expect("n1 live");
        for era in [3u32, 4] {
            if let Some(record) = table.record(Era(era)) {
                let order: Vec<_> = record
                    .config
                    .order()
                    .iter()
                    .map(|m| (m.node, u64::from(m.weight.0)))
                    .collect();
                eprintln!(
                    "PROBE4 era {era} order {order:?} primary(4) {:?}",
                    record.config.primary(View(4))
                );
            } else {
                eprintln!("PROBE4 era {era}: no record at n1");
            }
        }
        let t5 = h.era_table(n(5)).expect("n5 live");
        eprintln!(
            "PROBE4 n5 current {:?} era4 record {:?}",
            t5.current().era,
            t5.record(Era(4)).map(|_| "held")
        );
    }
    for round in 0..8u64 {
        h.tick_all();
        h.deliver_all();
        // The cluster serves through the converged view: the S4 tick
        // suspicion deposes an unserved primary, so rounds beyond the
        // suspicion horizon would fence the serving cluster into the dead
        // old identity's succession slot. The convergence must hold while
        // the cluster serves — the shape the play witnesses.
        let observer = [n(1), n(2), n(3)]
            .iter()
            .copied()
            .find(|&id| status_of(&h, id) == Status::Normal);
        if let Some(observer) = observer {
            let view = current_view(&h, observer);
            if let Some(primary) = h
                .era_table(observer)
                .and_then(|table| table.current().config.primary(view.view))
                .filter(|&primary| status_of(&h, primary) == Status::Normal)
            {
                let _ = h.propose(primary, op_id(100 + round), b"t");
                h.deliver_all();
            }
        }
    }
    for id in [n(1), n(2), n(3)] {
        assert_eq!(
            status_of(&h, id),
            Status::Normal,
            "{id:?}: the fence completes while the promoted witness is an era behind: {:?}",
            status_of(&h, id)
        );
    }
    h.assert_safety();
}
