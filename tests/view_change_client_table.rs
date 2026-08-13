//! The client table as view-change evidence (§9.2): the table hole
//! reproduced and closed.
//!
//! The hole: the client table is volatile, and nothing carried it through a
//! view change, so the new primary's table was empty and a retry of a
//! COMMITTED request was accepted as new — a second log entry for the same
//! logical request, the duplicate §9.2's two-exchange design exists to
//! prevent. The closure: the table rides `DoViewChange`, the new primary
//! merges the quorum's tables (per client, greatest `last_request`; a tie
//! prefers the row WITH a cached result), and `StartView` distributes the
//! merged table, which recipients install in place of their own.
//!
//! The soundness claim under test: **a request is replied-to at most once
//! per unique result, and the log holds at most one entry per accepted
//! request number per client.** The unknown-result case (the row survived
//! but the result died with the old primary) is answered by re-driving
//! `Effect::Apply` for the committed slot — never by re-appending — because
//! §11 already puts idempotence on the host: the host recognises the
//! re-drive by the slot being at or below its own applied frontier, so the
//! effect vocabulary needs no schema change.

mod harness;

use std::collections::BTreeMap;

use harness::{Harness, StepOutcome};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::effects::Effect;
use vrr::ids::{ClientId, Era, NodeId, RequestNumber, Slot, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, ClientRow, EraProof, EvidenceKind, Message};
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::{PlanRejection, ViewChangeKnobs};
use vrr::wire::{Header, Pack, Tag};

/// Node id shorthand (the harness's own pattern).
fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// A view in era 1 — every scenario here is same-era (W1).
fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// The snapshot of a live node (tests never snapshot a crashed one).
fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

/// The node's current view as a `ViewId`.
fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

/// The node's status, decoded from the observation word.
fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

/// The primary of a view in era 1 under the genesis order of `members`
/// nodes (§1.2).
fn primary_of(view: ViewId, members: u32) -> NodeId {
    n(view.view.0 % members)
}

/// A client-table row as the wire carries it.
fn row(client: u128, last_request: u64, result: Option<&[u8]>) -> ClientRow {
    ClientRow {
        client: ClientId(client),
        last_request: RequestNumber(last_request),
        result: result.map(|bytes| bytes.to_vec().into_boxed_slice()),
    }
}

/// Every reply released for one (client, request), in release order.
fn replies_for(h: &Harness, client: ClientId, request: u64) -> Vec<Box<[u8]>> {
    h.replies()
        .iter()
        .filter(|(_, c, r, _)| *c == client && *r == RequestNumber(request))
        .map(|(_, _, _, result)| result.clone())
        .collect()
}

/// Executes every pending `Apply` at the given nodes (one harness step per
/// effect), the way a host would.
fn apply_all(h: &mut Harness, ids: [u32; 3]) {
    for id in ids {
        h.execute_apply_effects(n(id));
    }
}

/// The timeout knob used by every test in this suite.
const TIMEOUT: u64 = 3;

/// A three-node cluster with the view-change knobs set explicitly.
fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// Bootstraps the cluster: the genesis primary promotes itself and the
/// backups adopt view (1, 0).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// Ticks a node until its timeout fires (one tick past the knob), asserting
/// it fenced into exactly `target`.
fn tick_into_view_change(h: &mut Harness, id: NodeId, target: ViewId) {
    for _ in 0..=TIMEOUT {
        h.tick(id);
    }
    assert_eq!(status_of(h, id), Status::ViewChange);
    assert_eq!(current_view(h, id), target);
}

/// The era-1 proof every fabricated message in this suite carries (§8.7.8).
/// The era-1 proof fabricated messages carry (§8.7.8): the genesis `Init`
/// of a `members`-node cluster.
fn era_proof_for(members: u32) -> EraProof {
    EraProof {
        op: SystemOperation::Init {
            order: (0..members).map(n).collect(),
        },
        committed_at: INIT_SLOT,
    }
}

/// Drives a complete view change to `target` among the two live nodes
/// `prime` (which must be `primary_of(target)` and times out first) and
/// `voter`. The view-change churn helper, unchanged.
fn drive_view_change(h: &mut Harness, prime: NodeId, voter: NodeId, target: ViewId) {
    assert_eq!(
        primary_of(target, 3),
        prime,
        "the prime must own the target"
    );
    tick_into_view_change(h, prime, target);
    assert!(h.peek_queued(voter, Tag::StartViewChange).is_some());
    h.deliver_all();
    for id in [prime, voter] {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target);
    }
    h.assert_safety();
}

/// The structural half of the soundness claim, checked over the whole
/// trace: replies for one (client, request) never carry two different
/// results, and no live node's journal holds two entries for one
/// (client, request).
fn assert_client_safety(h: &Harness) {
    let mut results: BTreeMap<(ClientId, RequestNumber), &[u8]> = BTreeMap::new();
    for (node, client, request, result) in h.replies() {
        match results.get(&(*client, *request)) {
            Some(previous) => assert_eq!(
                *previous,
                result.as_ref(),
                "two different results for ({client:?}, {request:?}); {node:?} replied {result:?}"
            ),
            None => {
                results.insert((*client, *request), result);
            }
        }
    }
    for id in [n(0), n(1), n(2)] {
        if !h.is_up(id) {
            continue;
        }
        let mut slots: BTreeMap<(ClientId, RequestNumber), Slot> = BTreeMap::new();
        for entry in h.journal_entries(id) {
            if let Payload::Client {
                client, request, ..
            } = &entry.payload
            {
                assert!(
                    slots.insert((*client, *request), entry.slot).is_none(),
                    "{id:?} holds two entries for ({client:?}, {request:?}): slots {:?} and {:?}",
                    slots[&(*client, *request)],
                    entry.slot
                );
            }
        }
    }
}

// 1. The hole, reproduced then closed: the request commits and replies, the
//    primary dies, the view changes, and the SAME request number retried
//    against the new primary is answered from the merged table — no second
//    log entry, exactly one additional Reply, the same result bytes (the
//    backup that applied the entry cached the result, and its DoViewChange
//    supplied the row to the merge).
#[test]
fn retry_of_a_committed_request_survives_the_view_change() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Slot 3 commits on {n0, n1} and both apply it; n2 only prepared it.
    h.client_request(n(0), ClientId(1), b"a");
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("n1 prepares");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("n2 prepares");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("the commit quorum lands");
    h.execute_apply_effects(n(0)); // the first Reply
    assert_eq!(replies_for(&h, ClientId(1), 1).len(), 1);
    h.deliver_tag(n(1), Tag::Commit)
        .expect("n1 learns the commit");
    h.execute_apply_effects(n(1)); // n1's row caches the result

    // The primary dies with its table. The view change completes among
    // {n1, n2}; n1's DoViewChange carries the row WITH the cached result.
    h.crash(n(0));
    drive_view_change(&mut h, n(1), n(2), view(1));
    assert_eq!(snap(&h, n(1)).committed, 3);

    // The retry: answered from the merged table. No second log entry,
    // exactly one additional Reply, the same result bytes.
    let outcome = h.client_request_numbered(n(1), ClientId(1), 1, b"a");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the cached reply publishes: {outcome:?}");
    };
    assert_eq!(
        effects,
        vec![Effect::Reply {
            client: ClientId(1),
            request: RequestNumber(1),
            result: b"a".to_vec().into_boxed_slice(),
        }],
        "the merged row supplies the cached result"
    );
    assert_eq!(snap(&h, n(1)).accepted, 3, "no second log entry");
    assert_eq!(h.journal_entries(n(1)).len(), 3);
    let replies = replies_for(&h, ClientId(1), 1);
    assert_eq!(replies.len(), 2, "the original reply and the cached one");
    assert!(
        replies.iter().all(|result| result.as_ref() == b"a"),
        "one unique result: {replies:?}"
    );
    h.assert_safety();
    assert_client_safety(&h);
}

// 2. The unknown-result re-drive: the committed entry's result is cached by
//    NO surviving quorum member (n1 applied it but restarted — the table is
//    volatile — and n0 crashed with its copy). The merged row knows the
//    request but not the result. The retry must NOT re-append (the entry is
//    committed in the installed history); the reply is produced by
//    re-driving `Effect::Apply` for the existing slot, which the host
//    recognises as a re-drive because the slot is at or below its applied
//    frontier (§11: idempotence is the host's friend — no schema change).
#[test]
fn unknown_result_retry_re_drives_the_committed_entry() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Slot 3 commits on {n0, n1}; n0 and n1 both apply it; n2 prepared it
    // but never learned the commit.
    h.client_request(n(0), ClientId(1), b"a");
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("n1 prepares");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("n2 prepares");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("the commit quorum lands");
    h.execute_apply_effects(n(0)); // the first Reply
    assert_eq!(replies_for(&h, ClientId(1), 1).len(), 1);
    h.deliver_tag(n(1), Tag::Commit)
        .expect("n1 learns the commit");
    h.execute_apply_effects(n(1)); // n1 applies: applied == 3

    // Every volatile copy of the result dies: n1 restarts from disk (the
    // journal — and the applied frontier — survive; the table does not),
    // and n0 crashes outright. n2's row carries the request but no result.
    h.crash(n(1));
    h.restart_with(n(1)).expect("the journal survived");
    h.crash(n(0));

    // n1 is the designated primary of view 1 but reopens Recovering, so
    // n2's timeout drives the change. The merged table is n1's empty table
    // plus n2's (c1, 1, None) — the row survives, the result does not.
    tick_into_view_change(&mut h, n(2), view(1));
    h.deliver_all();
    for id in [n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(&h, id), view(1));
        assert_eq!(snap(&h, id).committed, 3, "the committed entry survives");
    }

    // The retry: no re-append. The new primary re-drives Apply for the
    // committed slot; the host re-executes (the echo is idempotent) and the
    // normal Applied path produces the Reply.
    let outcome = h.client_request_numbered(n(1), ClientId(1), 1, b"a");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the re-drive publishes: {outcome:?}");
    };
    assert_eq!(
        effects,
        vec![Effect::Apply {
            slot: Slot(3),
            payload: b"a".to_vec().into_boxed_slice(),
        }],
        "the reply is produced by re-driving the committed slot, not by re-appending"
    );
    assert_eq!(snap(&h, n(1)).accepted, 3, "no second log entry");
    assert_eq!(h.journal_entries(n(1)).len(), 3);
    h.execute_apply_effects(n(1));
    let replies = replies_for(&h, ClientId(1), 1);
    assert_eq!(replies.len(), 2, "the original reply and the re-driven one");
    assert!(
        replies.iter().all(|result| result.as_ref() == b"a"),
        "one unique result: {replies:?}"
    );
    h.assert_safety();
    assert_client_safety(&h);
}

// 3. The uncommitted-tail retry: the entry died uncommitted with the old
//    primary (no surviving quorum member ever saw it, so the merged table
//    has no row and the installed history has no entry). The retry is
//    genuinely new: accepted, appended once, committed, replied once.
#[test]
fn uncommitted_tail_retry_is_genuinely_new() {
    let mut h = cluster();
    bootstrap(&mut h);

    // The old primary accepts the request alone, under partition.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.client_request_numbered(n(0), ClientId(1), 1, b"x");
    assert_eq!(snap(&h, n(0)).accepted, 3);

    // The change completes without n0's tail: the installed history ends at
    // the genesis slot and the merged table is empty.
    drive_view_change(&mut h, n(1), n(2), view(1));
    assert_eq!(snap(&h, n(1)).accepted, 2);

    // The retry is genuinely new — the old entry never committed, so the
    // client never got a reply and exactly-once is not at stake.
    let outcome = h.client_request_numbered(n(1), ClientId(1), 1, b"x");
    assert!(
        matches!(outcome, StepOutcome::Published { ref effects, .. } if effects.len() == 2),
        "accepted fresh: a Prepare per backup, got {outcome:?}"
    );
    h.deliver_all();
    apply_all(&mut h, [1, 2, 0]);
    assert_eq!(snap(&h, n(1)).committed, 3);
    assert_eq!(replies_for(&h, ClientId(1), 1).len(), 1, "replied once");
    for id in [n(1), n(2)] {
        assert_eq!(h.journal_entries(id).len(), 3, "appended once on {id:?}");
    }

    // The old primary heals and adopts: its private tail is replaced by an
    // identical entry (the retry found its way to the same slot), and every
    // journal holds the request exactly once.
    h.heal();
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    assert_client_safety(&h);
    h.assert_safety();
}

/// A fabricated `DoViewChange` for `target` carrying `table`: empty suffix,
/// `accepted == committed == frontier`, ordinary evidence (§13.1 permits no
/// suffix at all, and these scenarios never install history from it).
fn dvc(target: ViewId, frontier: Slot, table: Vec<ClientRow>, members: u32) -> Message {
    Message {
        header: Header {
            tag: Tag::DoViewChange,
            view: target,
            slot: frontier,
        },
        body: Body::DoViewChange {
            retained: view(0),
            accepted: frontier,
            committed: frontier,
            suffix: Vec::new(),
            client_table: table,
            evidence: EvidenceKind::Ordinary,
            era_proof: era_proof_for(members),
        },
    }
}

/// The merged table a five-node view change produces when the designated
/// new primary (n1, whose own table is empty) is fed `from_n0` and then
/// `from_n2` as fabricated evidence. A five-node cluster's evidence quorum
/// is three, so BOTH fabricated tables land before the quorum completes and
/// the delivery order is genuinely under test. The answer is read off the
/// winning `StartView`.
fn merged_table(from_n0: &[ClientRow], from_n2: &[ClientRow]) -> Vec<ClientRow> {
    let mut h = Harness::with_knobs(
        5,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    );
    bootstrap(&mut h);
    tick_into_view_change(&mut h, n(1), view(1));
    // Complete n1's fence quorum ({n1, n0, n2}): n0 and n2 join on n1's
    // StartViewChange, and only their fence votes (the re-broadcast copies)
    // are delivered back to n1 — the real DoViewChanges stay queued, so the
    // evidence quorum below is exactly the fabricated tables.
    h.deliver_to(n(0)).expect("n0 joins");
    h.deliver_to(n(2)).expect("n2 joins");
    h.deliver_tag(n(1), Tag::StartViewChange)
        .expect("n0's fence vote");
    h.deliver_tag(n(1), Tag::StartViewChange)
        .expect("n2's fence vote completes the quorum");
    h.inject(n(0), n(1), dvc(view(1), Slot(2), from_n0.to_vec(), 5));
    h.inject(n(2), n(1), dvc(view(1), Slot(2), from_n2.to_vec(), 5));
    let start_view = h
        .peek_queued(n(3), Tag::StartView)
        .expect("the evidence quorum completed and installed");
    let Body::StartView { client_table, .. } = start_view.body else {
        panic!("expected StartView");
    };
    client_table
}

// 4. Merge determinism: the same evidence set delivered in different orders
//    produces the identical merged table — same evidence, same table, on
//    every node.
#[test]
fn the_merge_is_a_total_deterministic_function_of_the_evidence() {
    let a = vec![row(1, 3, Some(b"r1")), row(2, 1, Some(b"r2"))];
    let b = vec![row(1, 3, None), row(3, 2, Some(b"r3"))];
    let expected = vec![
        // The tie on (c1, 3) breaks toward the row WITH the cached result.
        row(1, 3, Some(b"r1")),
        row(2, 1, Some(b"r2")),
        row(3, 2, Some(b"r3")),
    ];
    assert_eq!(merged_table(&a, &b), expected, "n0's table first");
    assert_eq!(merged_table(&b, &a), expected, "n2's table first");
}

// 5. Merge precedence, resolved: the GREATEST `last_request` wins wholesale;
//    result-preference breaks ties only. A higher request number with no
//    result beats a lower one WITH a result, because the row the merge keeps
//    is the row the client protocol will actually retry against: keeping the
//    lower row would strand the client (its retry of the newer request would
//    be judged ahead of the table and dropped forever), and it cannot
//    re-execute anything — the lower row's request was superseded before it
//    ever committed, or its reply is simply unreachable. The invariant "no
//    committed-and-replied request is ever re-executed" is preserved either
//    way for the TIE (both rows name the same request, so keeping the cached
//    result can never lose a produced reply), and for the non-tie the newer
//    row's request is the only one a retry can name.
#[test]
fn merge_precedence_greatest_request_wins_and_result_breaks_ties() {
    // Non-tie: (c1, 5, no result) beats (c1, 4, cached result).
    let merged = merged_table(&[row(1, 5, None)], &[row(1, 4, Some(b"r4"))]);
    assert_eq!(merged, vec![row(1, 5, None)]);
    // Tie, both orders: the row WITH the cached result wins.
    let expected = vec![row(1, 5, Some(b"r5"))];
    assert_eq!(
        merged_table(&[row(1, 5, Some(b"r5"))], &[row(1, 5, None)]),
        expected
    );
    assert_eq!(
        merged_table(&[row(1, 5, None)], &[row(1, 5, Some(b"r5"))]),
        expected
    );
}

// 6. Budget accounting, ruled and pinned: the table rides INSIDE the §13.1
//    budget — everything in the message counts, a budget is a budget. The
//    table is charged FIRST and is never truncated (it is safety evidence);
//    the suffix shrinks to make room, down to empty, which §13.1 explicitly
//    permits (the range is reconstructible via the state-transfer fetch,
//    §10).
#[test]
fn the_table_counts_against_the_suffix_budget_and_is_never_truncated() {
    let probe = LogEntry {
        slot: Slot::FIRST,
        era: Era::INITIAL,
        payload: Payload::Client {
            client: ClientId(1),
            request: RequestNumber(1),
            payload: b"pp".to_vec().into_boxed_slice(),
        },
    };
    let per_entry = probe.packed_len();
    let table_len = 4 + row(1, 1, None).packed_len(); // u32 count + one row

    /// Drives the view-change budget scenario — slot 3 committed by {n0, n1},
    /// n2 behind at genesis, view change among {n1, n2} with n0 held away —
    /// and returns the winning StartView's suffix and table.
    fn start_view_with_budget(budget: usize) -> (Vec<LogEntry>, Vec<ClientRow>) {
        let mut h = Harness::with_knobs(
            3,
            ViewChangeKnobs {
                primary_timeout: TIMEOUT,
                view_change_budget: budget,
            },
        );
        bootstrap(&mut h);
        h.client_request(n(0), ClientId(1), b"pp"); // slot 3
        h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
            .expect("n1 prepares: its row is (c1, 1, None)");
        h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
            .expect("the commit quorum lands");
        h.drop_queued(n(2));
        h.deliver_tag(n(1), Tag::Commit)
            .expect("n1 learns the commit");
        h.partition(vec![n(0)], vec![n(1), n(2)]);
        tick_into_view_change(&mut h, n(1), view(1));
        h.deliver_all();
        assert_eq!(status_of(&h, n(1)), Status::Normal);
        // n0's copy was partition-held at broadcast time; heal surfaces it.
        h.heal();
        let start_view = h
            .peek_queued(n(0), Tag::StartView)
            .expect("the new primary broadcasts StartView");
        let Body::StartView {
            suffix,
            client_table,
            ..
        } = &start_view.body
        else {
            panic!("expected StartView");
        };
        (suffix.clone(), client_table.clone())
    }

    // The budget fits exactly one entry. The table is charged first, so no
    // entry rides — and the table is complete regardless.
    let (suffix, table) = start_view_with_budget(per_entry);
    assert_eq!(
        table,
        vec![row(1, 1, None)],
        "the table is safety evidence: never truncated"
    );
    assert!(
        suffix.is_empty(),
        "the suffix shrinks first, down to empty (§13.1 permits it): {suffix:?}"
    );

    // One entry's worth of headroom above the table, and exactly the newest
    // entry rides again.
    let (suffix, table) = start_view_with_budget(per_entry + table_len);
    assert_eq!(table, vec![row(1, 1, None)]);
    assert_eq!(suffix.len(), 1);
    assert_eq!(suffix[0].slot, Slot(3), "the newest entry, ascending");
}

// 7. StartView installs the table: recipients REPLACE their own table with
//    the merged one (the merged table dominates — it saw a view-change
//    quorum's rows), and the installation survives the next view change: a
//    duplicate retry at the post-change primary gets the cached reply, and
//    at a backup it still gets the named redirection.
#[test]
fn start_view_installs_the_merged_table() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.client_request(n(0), ClientId(1), b"a"); // slot 3
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    assert_eq!(replies_for(&h, ClientId(1), 1).len(), 1);

    // View 1 among {n1, n2}: n2 installs the merged table from StartView.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    drive_view_change(&mut h, n(1), n(2), view(1));

    // A retry at the backup is still a client-protocol refusal, with the
    // named redirection — the table does not turn backups into primaries.
    assert_eq!(
        h.client_request_numbered(n(2), ClientId(1), 1, b"a"),
        StepOutcome::PlanRefused(PlanRejection::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        })
    );

    // The healed old primary adopts and installs the merged table too.
    h.heal();
    h.deliver_all();
    assert_eq!(status_of(&h, n(0)), Status::Normal);

    // A retry at the view-1 primary: the cached reply, no new entry.
    h.client_request_numbered(n(1), ClientId(1), 1, b"a");
    assert_eq!(snap(&h, n(1)).accepted, 3);

    // Depose n1: view 2 completes among {n0, n2}, and the table survives
    // the second change — every evidence row carried the cached result.
    h.partition(vec![n(1)], vec![n(0), n(2)]);
    drive_view_change(&mut h, n(2), n(0), view(2));

    // A duplicate retry at the post-change primary gets the cached reply.
    let outcome = h.client_request_numbered(n(2), ClientId(1), 1, b"a");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the cached reply publishes: {outcome:?}");
    };
    assert_eq!(
        effects,
        vec![Effect::Reply {
            client: ClientId(1),
            request: RequestNumber(1),
            result: b"a".to_vec().into_boxed_slice(),
        }]
    );
    assert_eq!(snap(&h, n(2)).accepted, 3, "no new entry, three views in");
    let replies = replies_for(&h, ClientId(1), 1);
    assert_eq!(replies.len(), 3, "one fresh reply, two cached re-emissions");
    assert!(replies.iter().all(|result| result.as_ref() == b"a"));
    h.assert_safety();
    assert_client_safety(&h);
}

// 8. Legality + safety under churn: the view-change churn shape with client
//    retries interleaved through the view changes. The gate stays clean (no
//    fault is ever declared), `assert_safety()` runs at every quiesce, and
//    the structural half of the soundness claim is asserted over the whole
//    trace at every quiesce and at the end.
#[test]
fn churn_with_retries_never_replies_twice_with_different_results() {
    let mut h = cluster();
    bootstrap(&mut h);
    // A small client pool, so retries have something to hit. `issued[c]` is
    // the greatest request number submitted for client `c`.
    let mut issued: BTreeMap<u64, u64> = BTreeMap::new();
    let mut submissions = 0u64;

    for round in 0..40u64 {
        match round % 8 {
            0 => {
                // A client request at the current primary, if one is live
                // and Normal. Even rounds submit a fresh request for a
                // rotating pool client; odd rounds RETRY the pool client's
                // last request — through whatever view the churn has
                // reached, table or no table.
                if let Some(primary) = current_primary(&h) {
                    let client = 1 + (round / 8) % 4;
                    let last = issued.get(&client).copied().unwrap_or(0);
                    if round % 16 == 0 || last == 0 {
                        let next = last + 1;
                        h.client_request_numbered(
                            primary,
                            ClientId(u128::from(client)),
                            next,
                            b"c",
                        );
                        issued.insert(client, next);
                        submissions += 1;
                    } else {
                        h.client_request_numbered(
                            primary,
                            ClientId(u128::from(client)),
                            last,
                            b"c",
                        );
                    }
                }
            }
            1 => {
                h.deliver_all();
                apply_all(&mut h, [0, 1, 2]);
                assert_client_safety(&h);
            }
            2 => {
                for id in [n(0), n(1), n(2)] {
                    if h.is_up(id) {
                        h.tick(id);
                    }
                }
            }
            3 => {
                // Rotate partition shapes, then let the live nodes suspect:
                // the two-node side completes a view change while the
                // isolated node is held away.
                match (round / 8) % 3 {
                    0 => h.partition(vec![n(0)], vec![n(1), n(2)]),
                    1 => h.partition(vec![n(1)], vec![n(0), n(2)]),
                    _ => h.partition(vec![n(2)], vec![n(0), n(1)]),
                }
                for id in [n(0), n(1), n(2)] {
                    if h.is_up(id) {
                        for _ in 0..=TIMEOUT {
                            h.tick(id);
                        }
                    }
                }
                h.deliver_all();
            }
            4 => {
                h.heal();
                h.deliver_all();
            }
            5 => {
                // Crash and restore (volatile wipe — the client table dies;
                // the disk survives).
                let id = n(u32::try_from((round / 8) % 3).expect("small"));
                if h.is_up(id) {
                    h.crash(id);
                    h.restart_with(id).expect("the journal survived");
                }
            }
            6 => {
                // Another retry shape: resubmit the pool client's last
                // request at the primary of the HIGHEST live view, whatever
                // the last round did to the cluster.
                let client = 1 + (round / 8) % 4;
                if let Some(&last) = issued.get(&client)
                    && let Some(primary) = current_primary(&h)
                {
                    h.client_request_numbered(primary, ClientId(u128::from(client)), last, b"c");
                }
            }
            _ => {
                h.deliver_all();
                apply_all(&mut h, [0, 1, 2]);
                h.assert_safety();
                assert_client_safety(&h);
            }
        }
    }
    assert!(submissions > 0, "the churn submitted requests");

    // Final quiesce: heal everything, then tick-and-deliver long enough for
    // suspicion to fire and the primary rotation to land on a completable
    // view.
    h.heal();
    for _ in 0..16 {
        h.tick_all();
        h.deliver_all();
    }
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();

    // Every node is Normal at one view, and the structural claim holds over
    // the whole trace: per client, per request number, never two different
    // results and never two log entries in one history.
    let aligned = current_view(&h, n(0));
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            current_view(&h, id),
            aligned,
            "{id:?} is at the cluster view"
        );
        assert_eq!(status_of(&h, id), Status::Normal);
        assert!(!snap(&h, id).faulted);
    }
    h.assert_safety();
    assert_client_safety(&h);
}

/// The primary of the highest view any live node reports, if that node is
/// itself live and Normal (the churn helper, unchanged).
fn current_primary(h: &Harness) -> Option<NodeId> {
    let mut highest: Option<ViewId> = None;
    for id in [n(0), n(1), n(2)] {
        if h.is_up(id) {
            let view = current_view(h, id);
            if highest.is_none_or(|incumbent| view > incumbent) {
                highest = Some(view);
            }
        }
    }
    let view = highest?;
    let candidate = primary_of(view, 3);
    (h.is_up(candidate)
        && status_of(h, candidate) == Status::Normal
        && current_view(h, candidate) == view)
        .then_some(candidate)
}
