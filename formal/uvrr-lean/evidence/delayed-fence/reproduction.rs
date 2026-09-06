//! Public-path reproduction for delayed first-round evidence across serial
//! amnesiac recoveries. All injected packets were emitted by the live core.
mod harness;

use harness::Harness;
use vrr::ids::{NodeId, OperationId};
use vrr::progress::Status;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::Tag;

fn n(id: u32) -> NodeId {
    NodeId(id)
}
fn discard(h: &mut Harness) {
    for id in 0..3 {
        h.drop_queued(n(id));
    }
}
fn normal(h: &Harness, id: u32, number: u32) {
    let s = h.snapshot(n(id)).unwrap();
    assert_eq!(
        Status::from_word(s.status),
        Some(Status::Normal),
        "{}",
        h.trace_dump()
    );
    assert_eq!(s.view, number, "{}", h.trace_dump());
}
fn reopen(h: &mut Harness, id: u32) {
    h.crash(n(id));
    h.restart_amnesiac(n(id)).unwrap();
    h.recover(n(id));
}
fn finish_recovery(h: &mut Harness, id: u32, others: [u32; 2]) {
    for other in others {
        h.deliver_tag(n(other), Tag::Recovery).unwrap();
    }
    h.deliver_tag(n(id), Tag::RecoveryResponse).unwrap();
    h.deliver_tag(n(id), Tag::RecoveryResponse).unwrap();
}

#[test]
fn delayed_first_round_evidence_across_serial_recoveries_preserves_commits() {
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: 3,
            view_change_budget: usize::MAX,
        },
    );
    h.tick_all();
    h.deliver_all();
    for id in 0..3 {
        normal(&h, id, 0);
    }
    for _ in 0..4 {
        h.tick(n(1));
    }
    let first_fence = h.peek_queued(n(2), Tag::StartViewChange).unwrap();
    discard(&mut h);

    // n1's fresh recovery replies are sent while both responders remain in v0.
    reopen(&mut h, 1);
    h.deliver_tag(n(0), Tag::Recovery).unwrap();
    let primary_reply = h.peek_queued(n(1), Tag::RecoveryResponse).unwrap();
    h.drop_queued(n(1));
    h.deliver_tag(n(2), Tag::Recovery).unwrap();
    let backup_reply = h.peek_queued(n(1), Tag::RecoveryResponse).unwrap();
    h.drop_queued(n(1));

    // The delayed n1 fence reaches n2 after n2 sent its recovery reply.
    h.inject(n(1), n(2), first_fence);
    let second_fence = h.peek_queued(n(1), Tag::StartViewChange).unwrap();
    let second_report = h.peek_queued(n(1), Tag::DoViewChange).unwrap();
    discard(&mut h);
    h.inject(n(0), n(1), primary_reply);
    h.inject(n(2), n(1), backup_reply);
    normal(&h, 1, 0);

    // There is only one recovering replica at any instant.
    reopen(&mut h, 2);
    finish_recovery(&mut h, 2, [0, 1]);
    normal(&h, 2, 0);
    discard(&mut h);
    h.propose(n(0), OperationId { msb: 0, lsb: 1 }, b"x");
    h.deliver_tag(n(2), Tag::Prepare).unwrap();
    h.deliver_tag(n(0), Tag::PrepareOk).unwrap();
    h.deliver_tag(n(2), Tag::Commit).unwrap();
    h.execute_apply_effects(n(0));
    h.execute_apply_effects(n(2));
    assert_eq!(h.snapshot(n(0)).unwrap().committed, 3);
    h.assert_safety();
    discard(&mut h);

    for _ in 0..4 {
        h.tick(n(1));
    }
    h.inject(n(2), n(1), second_fence);
    h.inject(n(2), n(1), second_report);
    normal(&h, 1, 1);
    discard(&mut h);

    // Recover n2 from the latest-view primary, then drive a client operation.
    reopen(&mut h, 2);
    finish_recovery(&mut h, 2, [0, 1]);
    normal(&h, 2, 1);
    discard(&mut h);
    h.propose(n(1), OperationId { msb: 0, lsb: 2 }, b"y");
    h.deliver_tag(n(2), Tag::Prepare).unwrap();
    h.deliver_tag(n(1), Tag::PrepareOk).unwrap();
    h.assert_safety();
}
