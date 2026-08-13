//! Contract for `vrr::journal` — the §4 logical journal capabilities, `LogEntry`, and
//! the default `SegmentedLog`.
//!
//! Spec §4 (the four capabilities, and the explicit absence of any fifth) and decisions
//! S1 (reclamation is host policy, absent from the portable traits), W3/W4 (the binary
//! codec is normative and `packed_len` is exact), W5 (no datagram-size opinion).
//!
//! What the rest of the crate is entitled to assume after this file passes:
//!
//! 1. **Contiguity is the journal's structural invariant, not the caller's memory.**
//!    `accept` refuses a non-contiguous or regressive batch and leaves the log
//!    unchanged when it refuses.
//! 2. **A view is a snapshot.** Sealed slabs are shared by `Arc` — asserted by pointer
//!    identity, not by contents — so a later append, suffix install, or reclaim cannot
//!    reach into a view already handed to a planned transition.
//! 3. **`install_suffix` is copy-on-write at slab granularity.** A wholly superseded
//!    slab is dropped (refcount observed); a partially superseded slab is copied up to
//!    the boundary and re-sealed (new `Arc` identity, same retained contents), never
//!    mutated in place, because other views may hold the `Arc`.
//! 4. **Reclamation is inherent, gated, lazy.** `SegmentedLog::reclaim_through` drops
//!    whole covered slabs only; a partially covered slab is retained whole; reclaimed
//!    slots read as absent while the logical accepted frontier is unaffected. A run
//!    that never reclaims is a legitimate configuration (S1).
//! 5. **The wire form of an entry is exact.** `packed_len` is the byte count, the
//!    payload discriminant reserves 0, and an untrusted length prefix yields
//!    `Incomplete`, never a pre-allocation (mirroring the `Init` decode decision).
//! 6. **The traits carry no reclamation vocabulary, mechanically** — see
//!    `s1_gate_traits_carry_no_reclamation_vocabulary`.
//!
//! The deterministic groups are exhaustive loops over small domains rather than
//! samplers; the interleaving property and the wire round trip are proptest.

use std::sync::Arc;

use proptest::prelude::*;

use vrr::configuration::{SystemOperation, VOID_SLOT};
use vrr::ids::{ClientId, Era, NodeId, RequestNumber, Slot};
use vrr::journal::{
    Journal, JournalError, JournalView, LogEntry, Payload, RangeOutcome, SegmentedLog,
};
use vrr::wire::{Malformed, Pack, Unpack, UnpackError};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A client-payload entry at `slot`, in the initial era, carrying one byte derived
/// from the slot so entries are distinguishable after copies.
fn client(slot: Slot) -> LogEntry {
    let byte = u8::try_from(slot.0 % 251).expect("mod 251 fits a u8");
    LogEntry {
        slot,
        era: Era::INITIAL,
        payload: Payload::Client {
            client: ClientId(1),
            request: RequestNumber(slot.0),
            payload: Box::new([byte; 4]),
        },
    }
}

/// A contiguous run of client entries, `from..from + len`.
fn run(from: u64, len: u64) -> Vec<LogEntry> {
    (0..len).map(|i| client(Slot(from + i))).collect()
}

/// A genesis `Void` at [`VOID_SLOT`] — slot 1, the first entry of a legitimate
/// history (§8.7.2's fixed ordinals; slot 0 is the sentinel, never a position).
fn genesis() -> LogEntry {
    LogEntry {
        slot: VOID_SLOT,
        era: Era::INITIAL,
        payload: Payload::System(SystemOperation::Void),
    }
}

/// A default log holding client entries at slots `1..=count` (no genesis payload;
/// these tests exercise journal structure, not the configuration fold).
fn log_with(count: u64) -> SegmentedLog {
    let mut log = SegmentedLog::new();
    if count > 0 {
        log.accept(&run(1, count)).expect("contiguous append");
    }
    log
}

// ---------------------------------------------------------------------------
// 1. Append ordering and contiguity
// ---------------------------------------------------------------------------

#[test]
fn append_single_and_batch_advance_the_frontier() {
    let mut log = SegmentedLog::new();
    assert_eq!(log.view().accepted(), None);

    log.accept(&[client(Slot(1))]).expect("first append");
    assert_eq!(log.view().accepted(), Some(Slot(1)));

    log.accept(&run(2, 5)).expect("contiguous batch");
    assert_eq!(log.view().accepted(), Some(Slot(6)));
    for slot in 1..=6u64 {
        assert_eq!(log.view().get(Slot(slot)), Some(&client(Slot(slot))));
    }

    // An empty batch is a no-op, not an error.
    log.accept(&[]).expect("empty batch");
    assert_eq!(log.view().accepted(), Some(Slot(6)));
}

/// The refusal a contiguous-frontier log must produce for `batch`: the first entry
/// that is not exactly the expected successor. A gap is `NonContiguous`; a regressive
/// slot names a position already accepted and is `SlotOccupied` — the causes differ
/// and so do the variants.
fn expected_refusal(next: Slot, batch: &[LogEntry]) -> JournalError {
    let mut expected = next;
    for entry in batch {
        if entry.slot < expected {
            return JournalError::SlotOccupied(entry.slot);
        }
        if entry.slot != expected {
            return JournalError::NonContiguous {
                expected,
                got: entry.slot,
            };
        }
        expected = expected.next().expect("test slots far from exhaustion");
    }
    panic!("fixture bug: batch is contiguous");
}

#[test]
fn noncontiguous_and_regressive_appends_are_refused_atomically() {
    // Every failure shape: gap at the front, regression at the front, gap inside,
    // regression inside, plateau inside. Each refusal leaves the log untouched.
    let mut log = log_with(6); // slots 1..=6, next expected is 7

    let cases: Vec<Vec<LogEntry>> = vec![
        vec![client(Slot(8))],
        vec![client(Slot(6))],
        vec![client(Slot(7)), client(Slot(9))],
        vec![client(Slot(7)), client(Slot(6))],
        vec![client(Slot(7)), client(Slot(8)), client(Slot(8))],
    ];
    for batch in &cases {
        let refusal = expected_refusal(Slot(7), batch);
        assert_eq!(log.accept(batch), Err(refusal), "batch {batch:?}");
        assert_eq!(
            log.view().accepted(),
            Some(Slot(6)),
            "a refused batch changes nothing"
        );
        for slot in 1..=6u64 {
            assert_eq!(log.view().get(Slot(slot)), Some(&client(Slot(slot))));
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Variable slab sizes and boundary-exact lookup
// ---------------------------------------------------------------------------

#[test]
fn variable_slab_sizes_lookup_is_exact_across_every_boundary() {
    // Tail capacity 4: appends of 4 seal into slabs of exactly 4; one batch of 9
    // bypasses the tail and seals as a single slab of 9. Slab sizes are therefore
    // [4, 4, 4, 9] and the layout exercises every boundary shape.
    let mut log = SegmentedLog::with_tail_capacity(4);
    for batch in 0..3u64 {
        log.accept(&run(1 + batch * 4, 4))
            .expect("tail-filling batch");
    }
    log.accept(&run(13, 9))
        .expect("bulk batch seals as one slab");
    assert_eq!(log.view().accepted(), Some(Slot(21)));

    // Exhaustive: every slot reads back the entry written there. A boundary search
    // that read a neighbouring slab's entry would surface as a wrong payload byte.
    let view = log.view();
    for slot in 1..=21u64 {
        assert_eq!(
            view.get(Slot(slot)),
            Some(&client(Slot(slot))),
            "slot {slot}"
        );
    }
    assert_eq!(view.get(Slot(22)), None, "past the tail");
}

#[test]
fn zero_tail_capacity_is_clamped_not_spinning() {
    // Zero asks for a slab that can hold nothing; the log clamps to 1 rather than
    // spinning on a seal that can never drain.
    let mut log = SegmentedLog::with_tail_capacity(0);
    log.accept(&run(1, 3))
        .expect("clamped capacity still appends");
    assert_eq!(log.view().accepted(), Some(Slot(3)));
    for slot in 1..=3u64 {
        assert_eq!(log.view().get(Slot(slot)), Some(&client(Slot(slot))));
    }
}

// ---------------------------------------------------------------------------
// 3. Double-ended iteration
// ---------------------------------------------------------------------------

#[test]
fn iter_range_is_ordered_and_double_ended_across_slab_boundaries() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");
    log.accept(&run(9, 2)).expect("tail of 2"); // slabs [4, 4], tail of 2

    let cases: [(u64, u64); 9] = [
        (1, 10),  // full range
        (1, 1),   // single slot at the start
        (10, 10), // single slot at the end
        (3, 6),   // ending mid-slab (slab boundary at 4/5)
        (4, 7),   // starting at a slab boundary, ending mid-slab
        (5, 8),   // starting on a slab boundary
        (2, 9),   // mid-slab to mid-slab across two boundaries
        (6, 9),   // slab interior into the tail
        (8, 10),  // tail only
    ];
    for (from, to) in cases {
        let view = log.view();
        let forward: Vec<LogEntry> = view.iter_range(Slot(from), Slot(to)).cloned().collect();
        assert_eq!(forward, run(from, to - from + 1), "forward {from}..={to}");

        let reverse: Vec<LogEntry> = view
            .iter_range(Slot(from), Slot(to))
            .rev()
            .cloned()
            .collect();
        let mut expect = run(from, to - from + 1);
        expect.reverse();
        assert_eq!(reverse, expect, "reverse {from}..={to}");

        // Interleaved next/next_back consumes each element exactly once.
        let mut it = view.iter_range(Slot(from), Slot(to));
        let mut woven = Vec::new();
        let mut take_front = true;
        loop {
            let next = if take_front {
                it.next()
            } else {
                it.next_back()
            };
            match next {
                Some(entry) => woven.push(entry.clone()),
                None => break,
            }
            take_front = !take_front;
        }
        woven.sort_by_key(|entry| entry.slot);
        assert_eq!(woven, forward, "interleaved {from}..={to} is complete");
    }

    // Degenerate ranges are empty, not panics: clipped at the tail, wholly past the
    // frontier, inverted.
    let view = log.view();
    let clipped: Vec<LogEntry> = view.iter_range(Slot(9), Slot(100)).cloned().collect();
    assert_eq!(clipped, run(9, 2), "clipped at the frontier");
    assert_eq!(view.iter_range(Slot(50), Slot(60)).count(), 0);
    assert_eq!(
        view.iter_range(Slot(10), Slot(4)).count(),
        0,
        "inverted is empty"
    );
}

// ---------------------------------------------------------------------------
// 4. A view is a snapshot, and sharing is by Arc
// ---------------------------------------------------------------------------

#[test]
fn view_is_a_snapshot_and_sealed_slabs_are_shared_by_arc() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");

    let before = log.view();
    assert_eq!(before.accepted(), Some(Slot(8)));

    // Sharing is by pointer, not by contents: a deep copy would pass every
    // behavioural assertion below while costing a per-view copy of history.
    for slab in before.slabs() {
        assert!(
            log.slabs()
                .iter()
                .any(|held| Arc::as_ptr(held) == Arc::as_ptr(slab)),
            "a view holds the log's own slab Arcs"
        );
    }

    // Append more, on both sides of a seal.
    log.accept(&run(9, 6)).expect("append past a seal");
    assert_eq!(
        before.accepted(),
        Some(Slot(8)),
        "the old view is unchanged"
    );
    assert_eq!(before.get(Slot(9)), None);
    let after = log.view();
    assert_eq!(after.accepted(), Some(Slot(14)));
    assert_eq!(after.get(Slot(14)), Some(&client(Slot(14))));

    // Reclaim does not reach into a held view either.
    log.reclaim_through(Slot(4));
    assert_eq!(
        before.get(Slot(1)),
        Some(&client(Slot(1))),
        "the old view still reads reclaimed slots"
    );
    assert_eq!(
        log.view().get(Slot(1)),
        None,
        "the log itself has dropped them"
    );
}

// ---------------------------------------------------------------------------
// 5. install_suffix divergence and the copy-on-write boundary
// ---------------------------------------------------------------------------

#[test]
fn install_suffix_replaces_the_divergent_tail_copy_on_write() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");
    log.accept(&run(9, 2)).expect("tail"); // slabs [1..4], [5..8]; tail 9,10

    // A competing view won with a different history from slot 7 onward.
    let mut suffix = Vec::new();
    for slot in 7..=13u64 {
        suffix.push(LogEntry {
            slot: Slot(slot),
            era: Era::INITIAL,
            payload: Payload::Client {
                client: ClientId(2),
                request: RequestNumber(slot),
                payload: Box::new([0xEE; 2]),
            },
        });
    }

    // Sealed slab [5..8] is partially retained: 5,6 stay, 7,8 go. It must be copied,
    // not mutated, because views may hold the Arc. Sealed slab [1..4] is wholly
    // retained and must not be copied. Scoped so the pre-install view's Arc handles
    // drop before the refcount assertions below.
    {
        let pre_install_view = log.view();
        let partial_arc = pre_install_view.slabs()[1].clone();
        let retained_arc = pre_install_view.slabs()[0].clone();

        log.install_suffix(Slot(7), &suffix)
            .expect("install the winning suffix");

        // The installed history is exactly [1..=6 from the base] ++ suffix.
        let view = log.view();
        assert_eq!(view.accepted(), Some(Slot(13)));
        for slot in 1..=6u64 {
            assert_eq!(view.get(Slot(slot)), Some(&client(Slot(slot))));
        }
        for entry in &suffix {
            assert_eq!(view.get(entry.slot), Some(entry));
        }

        // The wholly retained slab keeps its Arc identity; the partially retained
        // one was re-sealed under a new Arc; the pre-install view is intact.
        assert!(
            log.slabs()
                .iter()
                .any(|s| Arc::as_ptr(s) == Arc::as_ptr(&retained_arc)),
            "wholly retained slabs are shared, not copied"
        );
        assert!(
            !log.slabs()
                .iter()
                .any(|s| Arc::as_ptr(s) == Arc::as_ptr(&partial_arc)),
            "a partially retained slab is re-sealed under a new Arc"
        );
        assert_eq!(
            pre_install_view.get(Slot(7)),
            Some(&client(Slot(7))),
            "the old view is undisturbed"
        );
        assert_eq!(
            Arc::strong_count(&partial_arc),
            2,
            "the superseded slab is held by the old view and this test only"
        );
    }

    // A wholly superseded slab is dropped outright. Install again from the anchor
    // and watch the refcount of the log's current first slab.
    let first_slab = log.slabs()[0].clone();
    let view = log.view();
    assert_eq!(
        Arc::strong_count(&first_slab),
        3,
        "held by the log, the fresh view, and this test"
    );
    drop(view);
    log.install_suffix(Slot(1), &run(1, 2))
        .expect("replace from genesis");
    assert_eq!(
        Arc::strong_count(&first_slab),
        1,
        "a wholly superseded slab's memory is released once the log lets go"
    );
    assert_eq!(log.view().accepted(), Some(Slot(2)));
    assert_eq!(log.view().get(Slot(1)), Some(&client(Slot(1))));
    assert_eq!(log.view().get(Slot(3)), None);

    // A cut that lands inside the tail retains the tail's prefix: entries below
    // `from` are history, entries at or after it are replaced.
    let tail_suffix = vec![
        LogEntry {
            slot: Slot(2),
            era: Era::INITIAL,
            payload: Payload::Client {
                client: ClientId(3),
                request: RequestNumber(1),
                payload: Box::new([0x11; 1]),
            },
        },
        client(Slot(3)),
        client(Slot(4)),
    ];
    log.install_suffix(Slot(2), &tail_suffix)
        .expect("cut inside the tail");
    let view = log.view();
    assert_eq!(view.accepted(), Some(Slot(4)));
    assert_eq!(
        view.get(Slot(1)),
        Some(&client(Slot(1))),
        "tail prefix retained"
    );
    assert_eq!(
        view.get(Slot(2)),
        Some(&tail_suffix[0]),
        "divergent tail replaced"
    );
    assert_eq!(view.get(Slot(4)), Some(&client(Slot(4))));
    assert_eq!(view.get(Slot(5)), None);
}

#[test]
fn install_suffix_refusals_are_total_and_named() {
    let mut log = log_with(5); // slots 1..=5
    let before: Vec<LogEntry> = log.view().iter_range(Slot(1), Slot(5)).cloned().collect();

    // A gap between the frontier and the installed suffix.
    assert_eq!(
        log.install_suffix(Slot(10), &run(10, 1)),
        Err(JournalError::NonContiguous {
            expected: Slot(6),
            got: Slot(10),
        })
    );
    // A suffix that starts before the cut it claims to replace from names slots
    // already accepted.
    assert_eq!(
        log.install_suffix(Slot(5), &run(4, 2)),
        Err(JournalError::SlotOccupied(Slot(4)))
    );
    // No selected history to install.
    assert_eq!(
        log.install_suffix(Slot(4), &[]),
        Err(JournalError::EmptySuffix)
    );
    // A gap inside the suffix itself.
    assert_eq!(
        log.install_suffix(Slot(6), &[client(Slot(6)), client(Slot(8))]),
        Err(JournalError::NonContiguous {
            expected: Slot(7),
            got: Slot(8),
        })
    );

    let after: Vec<LogEntry> = log.view().iter_range(Slot(1), Slot(5)).cloned().collect();
    assert_eq!(before, after, "a refused install changes nothing");
}

// ---------------------------------------------------------------------------
// 6. copy_out
// ---------------------------------------------------------------------------

#[test]
fn copy_out_reports_exactly_what_is_physically_present() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");
    log.accept(&run(9, 2)).expect("tail of 2"); // 1..=10 present

    let mut into = Vec::new();

    // Full range.
    assert_eq!(
        log.view().copy_out(Slot(1), Slot(10), &mut into),
        RangeOutcome::Complete
    );
    assert_eq!(into, run(1, 10));

    // Interior range.
    into.clear();
    assert_eq!(
        log.view().copy_out(Slot(4), Slot(7), &mut into),
        RangeOutcome::Complete
    );
    assert_eq!(into, run(4, 4));

    // Range extending past the tail: returns what exists, reports the shortfall.
    // Decision, pinned: the request is clipped to the frontier, never invented.
    into.clear();
    assert_eq!(
        log.view().copy_out(Slot(8), Slot(20), &mut into),
        RangeOutcome::Short { through: Slot(10) }
    );
    assert_eq!(into, run(8, 3));

    // Entirely past the frontier.
    into.clear();
    assert_eq!(
        log.view().copy_out(Slot(11), Slot(20), &mut into),
        RangeOutcome::Short { through: Slot(10) }
    );
    assert!(into.is_empty());

    // Below retention: refused, nothing written. Decision, pinned: a range that
    // starts below the retained frontier is `BelowRetention` rather than a silent
    // partial, because §4 requires the journal to *report* unavailable history and
    // a partial copy would masquerade as a complete prefix.
    log.reclaim_through(Slot(4)); // slab [1..4] covered: dropped
    let view = log.view();
    assert_eq!(view.retained(), (Slot(5), Slot(10)));
    into.clear();
    assert_eq!(
        view.copy_out(Slot(3), Slot(7), &mut into),
        RangeOutcome::BelowRetention {
            slot: Slot(3),
            first: Slot(5),
        }
    );
    assert!(into.is_empty(), "a refused copy writes nothing");

    // At the retained frontier is fine.
    into.clear();
    assert_eq!(
        view.copy_out(Slot(5), Slot(7), &mut into),
        RangeOutcome::Complete
    );
    assert_eq!(into, run(5, 3));

    // Empty log: nothing is physically present.
    let empty = SegmentedLog::new();
    let mut sink = Vec::new();
    assert_eq!(
        empty.view().copy_out(Slot(1), Slot(5), &mut sink),
        RangeOutcome::Empty
    );
    assert!(sink.is_empty());
}

// ---------------------------------------------------------------------------
// 7. Reclamation
// ---------------------------------------------------------------------------

#[test]
fn reclaim_drops_whole_covered_slabs_only() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");
    log.accept(&run(9, 4)).expect("slab three"); // [1..4][5..8][9..12]

    // A checkpoint mid-slab retains the whole slab: nothing moves.
    log.reclaim_through(Slot(3));
    assert_eq!(
        log.view().retained(),
        (Slot(1), Slot(12)),
        "mid-slab: the whole slab is retained"
    );
    for slot in 1..=12u64 {
        assert_eq!(log.view().get(Slot(slot)), Some(&client(Slot(slot))));
    }

    // A checkpoint exactly on a slab's last slot drops exactly that slab.
    log.reclaim_through(Slot(4));
    assert_eq!(log.view().retained(), (Slot(5), Slot(12)));
    for slot in 1..5u64 {
        assert_eq!(
            log.view().get(Slot(slot)),
            None,
            "reclaimed slot reads absent"
        );
    }
    for slot in 5..=12u64 {
        assert_eq!(log.view().get(Slot(slot)), Some(&client(Slot(slot))));
    }

    // The *logical* accepted frontier is unaffected by reclamation (§4: the
    // frontier is a protocol fact; retention is physical).
    assert_eq!(log.view().accepted(), Some(Slot(12)));

    // `retained()` first advances only across whole-slab boundaries.
    log.reclaim_through(Slot(7));
    assert_eq!(log.view().retained(), (Slot(5), Slot(12)), "still mid-slab");
    log.reclaim_through(Slot(8));
    assert_eq!(
        log.view().retained(),
        (Slot(9), Slot(12)),
        "boundary crossed"
    );

    // Reclaiming through the frontier empties the physical log; the logical
    // frontier still stands.
    log.reclaim_through(Slot(12));
    assert_eq!(
        log.view().retained(),
        (Slot(13), Slot(12)),
        "empty window: first exceeds last"
    );
    assert_eq!(log.view().accepted(), Some(Slot(12)));
    assert_eq!(log.view().get(Slot(12)), None);
}

#[test]
fn reclaim_through_a_checkpoint_past_the_tail_is_total() {
    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("sealed slab");
    log.accept(&run(5, 2)).expect("tail of 2"); // [1..4], tail 5,6
    // A checkpoint at or past the frontier drains the tail into a covered slab too:
    // "final slot at or below checkpoint" is the only rule, and the tail is not
    // exempt from it.
    log.reclaim_through(Slot(6));
    assert_eq!(log.view().retained(), (Slot(7), Slot(6)), "fully reclaimed");
    assert_eq!(log.view().accepted(), Some(Slot(6)));
    // Appending after a full reclaim continues the history at the frontier.
    log.accept(&[client(Slot(7))])
        .expect("append continues after a full reclaim");
    assert_eq!(log.view().retained(), (Slot(7), Slot(7)));
    assert_eq!(log.view().get(Slot(7)), Some(&client(Slot(7))));
}

#[derive(Debug, Clone)]
enum Op {
    /// Append 1..=4 contiguous client entries.
    Append(u8),
    /// Install a suffix replacing the last 0..=3 slots and extending past the
    /// frontier by up to 3 more.
    Install { cut_back: u8, len: u8 },
    /// Reclaim through a slot at or below the current frontier.
    Reclaim(u8),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        5 => (1u8..=4).prop_map(Op::Append),
        3 => (0u8..=3, 1u8..=4).prop_map(|(cut_back, len)| Op::Install { cut_back, len }),
        2 => (0u8..=8).prop_map(Op::Reclaim),
    ]
}

proptest! {
    /// After any interleaving of appends, suffix installs and reclaims, `get(s)`
    /// for every physically retained `s` returns the entry last installed at `s`,
    /// and every slot outside the retained window reads absent. The shadow model is
    /// the slot-indexed history of last writers; the model cannot know slab
    /// boundaries, so the retained window is read from the log itself.
    #[test]
    fn interleaving_preserves_last_writer_per_slot(ops in proptest::collection::vec(op_strategy(), 1..60)) {
        let mut log = SegmentedLog::with_tail_capacity(3);
        // Shadow history: index `i` holds the entry last installed at slot `i + 1`
        // — history begins at slot 1, the genesis anchor.
        let mut shadow: Vec<LogEntry> = Vec::new();

        for op in ops {
            match op {
                Op::Append(len) => {
                    let first = u64::try_from(shadow.len()).expect("small history") + 1;
                    let batch = run(first, u64::from(len));
                    log.accept(&batch).expect("shadow appends are contiguous by construction");
                    shadow.extend(batch);
                }
                Op::Install { cut_back, len } => {
                    if shadow.is_empty() {
                        continue;
                    }
                    // Slots 1..=frontier are occupied; the cut stays at or above 1.
                    let frontier = u64::try_from(shadow.len()).expect("small history");
                    let from = frontier - u64::from(cut_back).min(frontier - 1);
                    // The cut cannot name a position the log has let go of: clamp to
                    // the retained window, which is the model's way of knowing where
                    // reclamation actually landed.
                    let (retained_first, _) = log.view().retained();
                    let from = from.max(retained_first.0);
                    // The winning suffix carries a distinct payload byte so a stale
                    // entry surviving an install is visible.
                    let suffix: Vec<LogEntry> = (0..u64::from(len))
                        .map(|i| LogEntry {
                            slot: Slot(from + i),
                            era: Era::INITIAL,
                            payload: Payload::Client {
                                client: ClientId(4),
                                request: RequestNumber(from + i),
                                payload: Box::new([0xAB, u8::try_from(i).expect("len <= 4")]),
                            },
                        })
                        .collect();
                    log.install_suffix(Slot(from), &suffix)
                        .expect("shadow installs are well-formed");
                    // Entries below slot `from` are slots 1..=from - 1: `from - 1`
                    // shadow entries.
                    shadow.truncate(usize::try_from(from - 1).expect("from >= 1"));
                    shadow.extend(suffix);
                }
                Op::Reclaim(back) => {
                    if shadow.is_empty() {
                        continue;
                    }
                    let frontier = u64::try_from(shadow.len()).expect("small history");
                    let checkpoint = frontier - u64::from(back).min(frontier - 1);
                    log.reclaim_through(Slot(checkpoint));
                }
            }

            // The logical frontier always agrees with the shadow.
            let expected_frontier = if shadow.is_empty() {
                None
            } else {
                Some(Slot(u64::try_from(shadow.len()).expect("small history")))
            };
            prop_assert_eq!(log.view().accepted(), expected_frontier);

            // Every slot inside the retained window reads the shadow's last writer;
            // every slot outside it reads absent.
            let view = log.view();
            let (first, last) = view.retained();
            for (index, entry) in shadow.iter().enumerate() {
                let slot = Slot(u64::try_from(index).expect("small history") + 1);
                let read = view.get(slot);
                if first <= slot && slot <= last {
                    prop_assert_eq!(read, Some(entry), "retained slot {}", index + 1);
                } else {
                    prop_assert_eq!(read, None, "slot {} outside the retained window", index + 1);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. retained() truthfulness
// ---------------------------------------------------------------------------

#[test]
fn retained_brackets_exactly_the_physically_present_slots() {
    let log = SegmentedLog::new();
    assert_eq!(
        log.view().retained(),
        (VOID_SLOT, Slot(u64::MAX)),
        "empty: first exceeds last (the anchor has no predecessor position)"
    );

    let mut log = SegmentedLog::with_tail_capacity(4);
    log.accept(&run(1, 4)).expect("slab one");
    log.accept(&run(5, 4)).expect("slab two");
    log.accept(&run(9, 2)).expect("tail of 2");
    assert_eq!(log.view().retained(), (Slot(1), Slot(10)));

    log.reclaim_through(Slot(4));
    let view = log.view();
    let (first, last) = view.retained();
    assert_eq!((first, last), (Slot(5), Slot(10)));
    // Exhaustive bracket: inside is present, outside is absent, up to the frontier.
    for slot in 1..=10u64 {
        let present = view.get(Slot(slot)).is_some();
        assert_eq!(
            present,
            first <= Slot(slot) && Slot(slot) <= last,
            "slot {slot}"
        );
    }
}

// ---------------------------------------------------------------------------
// 9. Empty and genesis states
// ---------------------------------------------------------------------------

#[test]
fn genesis_begins_at_slot_one_and_is_explicit() {
    // Decision, pinned: history begins at slot 1, the single named anchor shared
    // with `configuration::VOID_SLOT` (§8.7.2's fixed ordinals). Slot 0 is the
    // crate-wide "no slot" sentinel (`invariant::header_slot_role`), never a
    // position. There is no `first_slot` constructor knob; a state-transfer
    // restore is a journal whose prefix has been reclaimed, achieved by `accept`
    // + `reclaim_through`, so `accepted() == None` is unambiguous.
    let mut log = SegmentedLog::new();
    assert_eq!(log.view().accepted(), None);

    // The sentinel is refused outright: slot 0 can never hold an entry, and an
    // offer below the anchor reports the slot as occupied — the caller drops it
    // rather than fetching history for a position that does not exist.
    assert_eq!(
        log.accept(&[client(Slot(0))]),
        Err(JournalError::SlotOccupied(Slot(0)))
    );
    // Past the anchor is a gap, not a genesis.
    assert_eq!(
        log.accept(&[client(Slot(2))]),
        Err(JournalError::NonContiguous {
            expected: VOID_SLOT,
            got: Slot(2),
        })
    );
    log.accept(&[genesis()]).expect("Void at VOID_SLOT");
    assert_eq!(log.view().accepted(), Some(VOID_SLOT));

    // A state-transfer-restored log: history accepted, prefix reclaimed. The
    // frontier is honest and reads below retention report unavailability.
    let mut restored = SegmentedLog::with_tail_capacity(8);
    for batch in 0..5u64 {
        restored
            .accept(&run(1 + batch * 8, 8))
            .expect("transferred history");
    }
    restored.reclaim_through(Slot(30)); // slabs [1..8][9..16][17..24] covered
    assert_eq!(restored.view().accepted(), Some(Slot(40)));
    assert_eq!(restored.view().retained(), (Slot(25), Slot(40)));
    assert_eq!(restored.view().get(Slot(10)), None, "below retention");
    assert_eq!(
        restored.view().get(Slot(25)),
        Some(&client(Slot(25))),
        "the retained window is intact"
    );
}

// ---------------------------------------------------------------------------
// 10. Wire round trip
// ---------------------------------------------------------------------------

/// Entry shapes exhaustive over the payload variants and boundary lengths.
fn entry_cases() -> Vec<LogEntry> {
    vec![
        LogEntry {
            slot: Slot::FIRST,
            era: Era::INITIAL,
            payload: Payload::Client {
                client: ClientId(5),
                request: RequestNumber(1),
                payload: Box::new([]),
            },
        },
        client(Slot(3)),
        LogEntry {
            slot: Slot(u64::MAX),
            era: Era(u32::MAX),
            payload: Payload::Client {
                client: ClientId(6),
                request: RequestNumber(9),
                payload: vec![0xCD; 300].into_boxed_slice(),
            },
        },
        LogEntry {
            slot: Slot(9),
            era: Era(1),
            payload: Payload::System(SystemOperation::Increment(NodeId(7))),
        },
        genesis(),
        LogEntry {
            slot: Slot(2),
            era: Era(1),
            payload: Payload::System(SystemOperation::Init {
                order: vec![NodeId(1), NodeId(2), NodeId(3)],
            }),
        },
        LogEntry {
            slot: Slot(4),
            era: Era(2),
            payload: Payload::System(SystemOperation::Add {
                node: NodeId(9),
                position: 3,
            }),
        },
    ]
}

#[test]
fn wire_round_trip_is_exact_and_packed_len_is_normative() {
    for entry in entry_cases() {
        let len = entry.packed_len();
        let mut buf = vec![0u8; len];
        let written = entry
            .pack_into(&mut buf)
            .expect("buffer is exactly packed_len");
        assert_eq!(written, len, "packed_len is the byte count (W3)");
        let decoded = LogEntry::unpack_from(&buf).expect("round trip");
        assert_eq!(decoded, entry);

        // Every proper prefix is Incomplete, never a partial decode or a panic.
        for cut in 0..len {
            match LogEntry::unpack_from(&buf[..cut]) {
                Err(UnpackError::Incomplete { .. }) => {}
                other => panic!("{entry:?} truncated to {cut}: expected Incomplete, got {other:?}"),
            }
        }

        // A byte short of buffer space is BufferTooSmall and writes nothing.
        let mut short = vec![0xAAu8; len - 1];
        assert!(entry.pack_into(&mut short).is_err());
        assert!(
            short.iter().all(|&b| b == 0xAA),
            "nothing written on refusal"
        );
    }
}

#[test]
fn wire_refuses_reserved_and_unknown_discriminants() {
    // Discriminant 0 is reserved: an all-zero buffer is not a valid entry.
    let zeros = [0u8; 16];
    assert_eq!(
        LogEntry::unpack_from(&zeros),
        Err(UnpackError::Malformed(Malformed::OutOfDomain))
    );

    // Unknown discriminants are refused, never guessed. slot(8) ++ era(4), then
    // the discriminant byte.
    for discriminant in [3u8, 4, 17, 255] {
        let mut buf = vec![0u8; 13];
        buf[12] = discriminant;
        match LogEntry::unpack_from(&buf) {
            Err(UnpackError::Malformed(Malformed::OutOfDomain)) => {}
            other => panic!("discriminant {discriminant}: expected OutOfDomain, got {other:?}"),
        }
    }

    // A client payload length prefix far beyond the input is Incomplete, and the
    // decode must not pre-allocate from the untrusted prefix (the `Init`
    // decode decision, mirrored): this buffer is 41 bytes and claims a 4 GiB payload.
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u64.to_be_bytes()); // slot
    buf.extend_from_slice(&0u32.to_be_bytes()); // era
    buf.push(1); // Payload::Client
    buf.extend_from_slice(&0u128.to_be_bytes()); // client
    buf.extend_from_slice(&0u64.to_be_bytes()); // request
    buf.extend_from_slice(&u32::MAX.to_be_bytes()); // declared payload length
    match LogEntry::unpack_from(&buf) {
        Err(UnpackError::Incomplete { .. }) => {}
        other => panic!("oversized prefix: expected Incomplete, got {other:?}"),
    }

    // Trailing bytes are refused: a message with a suffix is not a message.
    let entry = client(Slot(1));
    let mut buf = vec![0u8; entry.packed_len() + 1];
    entry.pack_into(&mut buf).expect("fits");
    assert_eq!(
        LogEntry::unpack_from(&buf),
        Err(UnpackError::Malformed(Malformed::TrailingBytes {
            unread: 1
        }))
    );
}

proptest! {
    /// Round trip over arbitrary entries: slots and eras anywhere in range, client
    /// payloads of arbitrary length up to a host-plausible size, and both payload
    /// variants.
    #[test]
    fn wire_round_trip_proptest(
        slot in any::<u64>(),
        era in any::<u32>(),
        client in any::<u128>(),
        request in any::<u64>(),
        client_payload in proptest::collection::vec(any::<u8>(), 0..512),
        is_system in any::<bool>(),
        node in any::<u32>(),
    ) {
        let payload = if is_system {
            Payload::System(SystemOperation::Increment(NodeId(node)))
        } else {
            Payload::Client {
                client: ClientId(client),
                request: RequestNumber(request),
                payload: client_payload.into_boxed_slice(),
            }
        };
        let entry = LogEntry { slot: Slot(slot), era: Era(era), payload };
        let mut buf = vec![0u8; entry.packed_len()];
        let written = entry.pack_into(&mut buf).expect("exact buffer");
        prop_assert_eq!(written, entry.packed_len());
        let decoded = LogEntry::unpack_from(&buf).expect("round trip");
        prop_assert_eq!(decoded, entry);
    }
}

// ---------------------------------------------------------------------------
// 11. The S1 mechanical gate
// ---------------------------------------------------------------------------

/// The compile-time contract that `Journal` and `JournalView` carry no reclamation
/// vocabulary (spec §4 names four capabilities and no fifth; decision S1 makes that
/// a ruling).
///
/// Mechanism, chosen and documented: a source-text scan of the trait definition
/// slices, pinned as a test. The alternative — a const-assertion or trait-bound
/// pattern — cannot express "this method does not exist", because Rust has no
/// negative trait bound and no way to name a method that must not be there. A text
/// scan can, and it fails the build at `cargo test` the moment someone adds
/// `reclaim`, `truncate` or `retain(` to either trait. Its known hole is cosmetic:
/// it would also fire on those words inside a doc comment within the trait body.
/// That hole is accepted deliberately, because a false positive on a comment costs
/// a reworded sentence while a false negative costs the S1 split itself.
#[test]
fn s1_gate_traits_carry_no_reclamation_vocabulary() {
    let source = include_str!("../src/journal.rs");

    for trait_decl in ["pub trait JournalView {", "pub trait Journal {"] {
        let start = source
            .find(trait_decl)
            .unwrap_or_else(|| panic!("{trait_decl} must exist in src/journal.rs"));
        let rest = &source[start..];
        let end = rest
            .find("\n}\n")
            .unwrap_or_else(|| panic!("{trait_decl} must have a body"));
        // Case-folded: the gate guards meaning, not capitalisation.
        let body = rest[..end].to_lowercase();
        for forbidden in ["reclaim", "truncate", "retain("] {
            assert!(
                !body.contains(forbidden),
                "{trait_decl} must not mention `{forbidden}`: \
                 reclamation is host policy and absent from the portable trait (S1)"
            );
        }
    }

    // The capability count itself is pinned: `Journal` has exactly `view`, `accept`,
    // `install_suffix`; `JournalView` exactly `accepted`, `get`, `iter_range`,
    // `retained`, `copy_out`. A fifth capability added to either is as much an S1
    // breach as a reclamation method is.
    let journal_start = source.find("pub trait Journal {").expect("Journal trait");
    let journal_rest = &source[journal_start..];
    let journal_body = &journal_rest[..journal_rest.find("\n}\n").expect("body")];
    assert_eq!(
        journal_body.matches("\n    fn ").count(),
        3,
        "Journal carries exactly the two §4 mutations plus view()"
    );
    let view_start = source
        .find("pub trait JournalView {")
        .expect("JournalView trait");
    let view_rest = &source[view_start..];
    let view_body = &view_rest[..view_rest.find("\n}\n").expect("body")];
    assert_eq!(
        view_body.matches("\n    fn ").count(),
        5,
        "JournalView carries exactly the §4 read capabilities"
    );
}

/// The inherent escape hatch exists on `SegmentedLog` only — that placement is the
/// split made physical. Asserted by calling it through the concrete type (this test
/// fails to compile if the method moves or is renamed) and by the trait scan above
/// (which fails if it is ever promoted into a trait).
#[test]
fn s1_gate_reclaim_is_inherent_not_trait() {
    fn assert_journal<J: Journal>(journal: &J) -> &J {
        journal
    }
    let mut log = SegmentedLog::new();
    log.accept(&run(1, 4)).expect("genesis run");
    // Reachable only as an inherent method on the concrete type.
    SegmentedLog::reclaim_through(&mut log, Slot(0));
    // And the value still satisfies the portable traits afterwards.
    let _ = assert_journal(&log);
    let view = <SegmentedLog as Journal>::view(&log);
    let _ = JournalView::accepted(&view);
}
