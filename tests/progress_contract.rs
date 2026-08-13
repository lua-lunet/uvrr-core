//! Contract for `vrr::progress`, `vrr::invariant::legal`, and `vrr::observe`.
//!
//! Spec §1.3 (current vs retained view, frontier chain), §5 (the progress record and
//! its sticky fault), §6 (the delta transition), §8.7.3 (era/slot discipline), §12
//! (the serialized transition interval), and decisions B1 (seqlock), S3 (only
//! `Indeterminate` persistence faults), W1 (`ViewId`).
//!
//! The properties pinned here, each of which the rest of the crate is entitled to
//! assume without re-checking:
//!
//! 1. a `Progress` violating the frontier chain `checkpoint <= applied <= committed
//!    <= accepted` is unrepresentable, exhaustively over a small frontier space;
//! 2. the §1.3 status/view relations hold on every representable value;
//! 3. a fault is sticky: a faulted `Progress` admits no transition at all, and
//!    `legal` reports the existing fault rather than a fresh one;
//! 4. `revision` advances by exactly one per published transition, so a stale plan
//!    (§12) is rejected by comparison;
//! 5. `invariant::legal` enforces each numbered rule of the transition contract,
//!    one violating and one passing triple per rule;
//! 6. era/slot discipline: `era(accepted)` is `era(current)` or `era(current) + 1`
//!    — the `+1` boundary (overlap mode) passes, `+2` is refused at construction;
//! 7. the seqlock never returns a torn read, under a writer/reader race;
//! 8. `ViewId::INITIAL` is the genesis view: `Progress::genesis` advertises it,
//!    fenced and recovering, per §5.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vrr::configuration::{EraTable, SystemOperation};
use vrr::ids::{Era, NodeId, Slot, View, ViewId};
use vrr::invariant::{Fault, HeaderSlotRole, InputKind, header_slot_role, legal};
use vrr::observe::Observation;
use vrr::progress::{Progress, ProgressError, Status};
use vrr::wire::Tag;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A table with every era through `era` established: `Void` at slot 1, `Init` at
/// slot 2, and one `Increment` per further era, so era `k >= 1` is established at
/// slot `k + 1`. The retention window keeps only the last two eras.
fn table_upto(era: u32) -> Arc<EraTable> {
    let node = NodeId(0);
    let mut table = EraTable::genesis();
    table = table
        .extend(&SystemOperation::Void, Slot(1))
        .expect("Void at VOID_SLOT");
    if era >= 1 {
        table = table
            .extend(&SystemOperation::Init { order: vec![node] }, Slot(2))
            .expect("Init at INIT_SLOT");
    }
    for established in 2..=era {
        let slot = Slot(u64::from(established) + 1);
        table = table
            .extend(&SystemOperation::Increment(node), slot)
            .expect("Increment establishes one era per slot");
    }
    Arc::new(table)
}

fn genesis_table() -> Arc<EraTable> {
    Arc::new(EraTable::genesis())
}

fn view(era: u32, view: u32) -> ViewId {
    ViewId {
        era: Era(era),
        view: View(view),
    }
}

/// A `Normal`-status progress with `current == retained`, valid by construction in
/// the domains these tests use.
#[allow(clippy::too_many_arguments)]
fn normal(
    at: ViewId,
    accepted: u64,
    committed: u64,
    applied: u64,
    checkpoint: u64,
    revision: u64,
    table: &Arc<EraTable>,
) -> Progress {
    Progress::reconstitute(
        at,
        at,
        Status::Normal,
        Slot(accepted),
        Slot(committed),
        Slot(applied),
        Slot(checkpoint),
        revision,
        Arc::clone(table),
        None,
    )
    .expect("fixture must satisfy the progress invariants")
}

// ---------------------------------------------------------------------------
// 1. Frontier chain — exhaustive over a small space
// ---------------------------------------------------------------------------

/// Every tuple `(checkpoint, applied, committed, accepted)` over `0..=3` is either
/// constructible or refused with `FrontierChain`, exactly according to the §1.3
/// chain. 256 cases, total coverage: the domain is tiny and sampling is strictly
/// weaker. The genesis table keeps era/slot discipline silent here (era 0 covers
/// every slot), so the chain is the only rule that can fire.
#[test]
fn frontier_chain_is_exhaustively_enforced() {
    let table = genesis_table();
    for checkpoint in 0u64..=3 {
        for applied in 0u64..=3 {
            for committed in 0u64..=3 {
                for accepted in 0u64..=3 {
                    let result = Progress::reconstitute(
                        ViewId::INITIAL,
                        ViewId::INITIAL,
                        Status::Normal,
                        Slot(accepted),
                        Slot(committed),
                        Slot(applied),
                        Slot(checkpoint),
                        0,
                        Arc::clone(&table),
                        None,
                    );
                    let chain_holds =
                        checkpoint <= applied && applied <= committed && committed <= accepted;
                    if chain_holds {
                        assert!(
                            result.is_ok(),
                            "({checkpoint}, {applied}, {committed}, {accepted}) satisfies the chain"
                        );
                    } else {
                        assert_eq!(
                            result.unwrap_err(),
                            ProgressError::FrontierChain,
                            "({checkpoint}, {applied}, {committed}, {accepted})"
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Status/view relations (§1.3)
// ---------------------------------------------------------------------------

/// `Normal` requires `current == retained`; `ViewChange` requires `current >=
/// retained`. `Recovering` and `Replaying` carry no relation — §1.3 states that in
/// those statuses `current` is not an authority to participate, so constraining it
/// would forbid states a recovering node legitimately holds.
#[test]
fn status_view_relation_is_enforced() {
    let table = genesis_table();
    let build = |current: ViewId, retained: ViewId, status: Status| {
        Progress::reconstitute(
            current,
            retained,
            status,
            Slot(0),
            Slot(0),
            Slot(0),
            Slot(0),
            0,
            Arc::clone(&table),
            None,
        )
    };

    assert_eq!(
        build(view(0, 1), view(0, 0), Status::Normal).unwrap_err(),
        ProgressError::StatusViewRelation
    );
    assert!(build(view(0, 1), view(0, 1), Status::Normal).is_ok());

    assert_eq!(
        build(view(0, 0), view(0, 1), Status::ViewChange).unwrap_err(),
        ProgressError::StatusViewRelation
    );
    assert!(build(view(0, 1), view(0, 0), Status::ViewChange).is_ok());
    assert!(build(view(0, 1), view(0, 1), Status::ViewChange).is_ok());

    assert!(build(view(0, 0), view(0, 1), Status::Recovering).is_ok());
    assert!(build(view(0, 0), view(0, 1), Status::Replaying).is_ok());
}

// ---------------------------------------------------------------------------
// 3. Sticky fault (§5 invariant 5, S3)
// ---------------------------------------------------------------------------

/// A faulted progress refuses every transition, reporting the fault it already
/// holds — never a new one. This is what S3 buys: an indeterminate persistence
/// result stops the node instead of letting it guess what its durable state is.
#[test]
fn fault_is_sticky_across_every_transition() {
    let table = genesis_table();
    let faulted = Progress::reconstitute(
        ViewId::INITIAL,
        ViewId::INITIAL,
        Status::Normal,
        Slot(2),
        Slot(1),
        Slot(1),
        Slot(1),
        3,
        Arc::clone(&table),
        Some(Fault::IndeterminatePersistence),
    )
    .expect("a faulted value is still structurally valid");

    let expected = ProgressError::AlreadyFaulted(Fault::IndeterminatePersistence);
    assert_eq!(faulted.with_accepted(Slot(3)).unwrap_err(), expected);
    assert_eq!(faulted.with_committed(Slot(2)).unwrap_err(), expected);
    assert_eq!(faulted.with_applied(Slot(2)).unwrap_err(), expected);
    assert_eq!(faulted.with_checkpoint(Slot(2)).unwrap_err(), expected);
    assert_eq!(faulted.with_view_change(view(0, 1)).unwrap_err(), expected);
    assert_eq!(
        faulted
            .with_view_installed(view(0, 1), Slot(2))
            .unwrap_err(),
        expected
    );
    assert_eq!(
        faulted.with_status(Status::Recovering).unwrap_err(),
        expected
    );
    assert_eq!(faulted.with_config(table_upto(1)).unwrap_err(), expected);
    // A second fault does not replace the first: the node faults once, on the
    // first thing it could not determine.
    assert_eq!(
        faulted.with_fault(Fault::HostDeclared).unwrap_err(),
        expected
    );

    // `legal` reports the existing fault, not `IllegalTransition`: the old value
    // admits no candidate at all, whatever the candidate looks like.
    let candidate = normal(ViewId::INITIAL, 3, 1, 1, 1, 4, &table);
    assert_eq!(
        legal(&faulted, &candidate, &InputKind::ClientRequest),
        Some(Fault::IndeterminatePersistence)
    );
}

// ---------------------------------------------------------------------------
// 4. Revision discipline (§12: exactly one outstanding transition)
// ---------------------------------------------------------------------------

/// `revision` exists so the host's serialized transition interval can reject a
/// stale plan: a plan computed against revision `r` is publishable only against a
/// state still at revision `r`. The discipline is exactly +1 per published
/// transition.
#[test]
fn revision_advances_by_exactly_one() {
    let table = genesis_table();
    let genesis = Progress::genesis(Arc::clone(&table)).expect("genesis");
    assert_eq!(genesis.revision(), 0);
    let one = genesis.with_accepted(Slot(1)).expect("first transition");
    assert_eq!(one.revision(), 1);
    let two = one.with_committed(Slot(1)).expect("second transition");
    assert_eq!(two.revision(), 2);

    // legal() rule 4: 0, +2, and regression are each `IllegalTransition`.
    let old = normal(ViewId::INITIAL, 4, 2, 1, 1, 7, &table);
    for revision in [7u64, 9, 6, u64::MAX] {
        let candidate = normal(ViewId::INITIAL, 5, 2, 1, 1, revision, &table);
        assert_eq!(
            legal(&old, &candidate, &InputKind::ClientRequest),
            Some(Fault::IllegalTransition),
            "revision {revision} from 7"
        );
    }
    let borderline = normal(ViewId::INITIAL, 5, 2, 1, 1, 8, &table);
    assert_eq!(legal(&old, &borderline, &InputKind::ClientRequest), None);

    // The revision space does not wrap: a progress at `u64::MAX` refuses to
    // transition rather than publish revision 0 again.
    let exhausted = normal(ViewId::INITIAL, 0, 0, 0, 0, u64::MAX, &table);
    assert_eq!(
        exhausted.with_accepted(Slot(1)).unwrap_err(),
        ProgressError::RevisionExhausted
    );
}

// ---------------------------------------------------------------------------
// 5. The `legal` rule table
// ---------------------------------------------------------------------------

/// Rule 1 — monotone frontiers never regress; `accepted` may shorten only across
/// a history re-selection (rule 3's set), because §9.1 ranks `retained_view`
/// first: a shorter history retained from a later view displaces a longer one
/// from an earlier view.
#[test]
fn rule1_frontiers_never_regress() {
    let table = genesis_table();
    let old = normal(ViewId::INITIAL, 5, 2, 1, 1, 0, &table);

    // Committed regression.
    let candidate = normal(ViewId::INITIAL, 5, 1, 1, 1, 1, &table);
    assert_eq!(
        legal(&old, &candidate, &InputKind::ClientRequest),
        Some(Fault::IllegalTransition)
    );
    // Applied regression.
    let candidate = normal(ViewId::INITIAL, 5, 2, 0, 0, 1, &table);
    assert_eq!(
        legal(&old, &candidate, &InputKind::ClientRequest),
        Some(Fault::IllegalTransition)
    );
    // `accepted` regression with `retained` unchanged: not a re-selection.
    let candidate = Progress::reconstitute(
        view(0, 1),
        ViewId::INITIAL,
        Status::ViewChange,
        Slot(3),
        Slot(2),
        Slot(1),
        Slot(1),
        1,
        Arc::clone(&table),
        None,
    )
    .expect("valid candidate");
    assert_eq!(
        legal(
            &old,
            &candidate,
            &InputKind::PeerMessage {
                tag: Tag::StartViewChange,
                slot: Slot(0),
            },
        ),
        Some(Fault::IllegalTransition)
    );

    // Borderline pass: equal frontiers are not a regression.
    let candidate = normal(ViewId::INITIAL, 5, 2, 1, 1, 1, &table);
    assert_eq!(legal(&old, &candidate, &InputKind::Tick), None);
}

/// The `accepted`-shortening re-selection: legal exactly when `retained` changed
/// under an input that can re-select history.
#[test]
fn rule1_accepted_may_shorten_only_on_reselection() {
    let table = genesis_table();
    let old = normal(view(0, 3), 5, 2, 1, 1, 0, &table);
    // View 4 installs a history whose frontier (3) is shorter than the local
    // one (5) but still covers `committed` (2).
    let installed = normal(view(0, 4), 3, 2, 1, 1, 1, &table);
    assert_eq!(
        legal(
            &old,
            &installed,
            &InputKind::PeerMessage {
                tag: Tag::StartView,
                slot: Slot(3),
            },
        ),
        None
    );
}

/// Rule 2 — `current` never regresses, and a view change is a legal successor:
/// view strictly up, era equal or +1 (delegated to `ViewId::is_legal_successor`).
#[test]
fn rule2_view_succession() {
    let table = genesis_table();
    let old = normal(view(0, 3), 5, 2, 1, 1, 0, &table);

    // View regression.
    let candidate = normal(view(0, 2), 5, 2, 1, 1, 1, &table);
    assert_eq!(
        legal(&old, &candidate, &InputKind::Tick),
        Some(Fault::IllegalTransition)
    );

    // Era jump of two, with internally valid values on both sides.
    let old_e1 = normal(view(1, 5), 2, 0, 0, 0, 0, &table_upto(1));
    let jumped = normal(view(3, 9), 4, 0, 0, 0, 1, &table_upto(3));
    assert_eq!(
        legal(
            &old_e1,
            &jumped,
            &InputKind::PeerMessage {
                tag: Tag::StartView,
                slot: Slot(4),
            },
        ),
        Some(Fault::IllegalTransition)
    );

    // Borderline pass: era +1 (the overlap-mode successor).
    let successor = normal(view(2, 9), 3, 0, 0, 0, 1, &table_upto(2));
    assert_eq!(
        legal(
            &old_e1,
            &successor,
            &InputKind::PeerMessage {
                tag: Tag::StartView,
                slot: Slot(3),
            },
        ),
        None
    );
}

/// Rule 3 — `retained` identifies the provenance of the retained history (§1.3);
/// it changes only when that history was re-selected: recovery, or a peer message
/// that installs one (`DoViewChange` completing the new primary's quorum,
/// `StartView`, `NewState`).
#[test]
fn rule3_retained_changes_only_on_reselection() {
    let table = genesis_table();
    let old = normal(view(0, 3), 5, 2, 1, 1, 0, &table);
    let installed = normal(view(0, 4), 5, 2, 1, 1, 1, &table);

    // Retained changed during normal operation.
    assert_eq!(
        legal(&old, &installed, &InputKind::ClientRequest),
        Some(Fault::IllegalTransition)
    );
    assert_eq!(
        legal(&old, &installed, &InputKind::Tick),
        Some(Fault::IllegalTransition)
    );
    // A peer message that does not install history.
    assert_eq!(
        legal(
            &old,
            &installed,
            &InputKind::PeerMessage {
                tag: Tag::Prepare,
                slot: Slot(5),
            },
        ),
        Some(Fault::IllegalTransition)
    );

    // Borderline passes: each re-selection input.
    assert_eq!(legal(&old, &installed, &InputKind::Recovery), None);
    for tag in [Tag::DoViewChange, Tag::StartView, Tag::NewState] {
        assert_eq!(
            legal(
                &old,
                &installed,
                &InputKind::PeerMessage { tag, slot: Slot(5) },
            ),
            None,
            "{tag:?} may re-select history"
        );
    }
}

/// Rule 7 — the per-tag header-slot table (item02d, resolved here). The table
/// itself is pinned exhaustively; each role then gets its violating and passing
/// transitions below.
#[test]
fn rule7_header_slot_table_is_the_documented_one() {
    let expected: [(Tag, HeaderSlotRole); 13] = [
        (Tag::Request, HeaderSlotRole::Absent),
        (Tag::Prepare, HeaderSlotRole::Operation),
        (Tag::PrepareOk, HeaderSlotRole::Operation),
        (Tag::Commit, HeaderSlotRole::Frontier),
        (Tag::StartViewChange, HeaderSlotRole::Absent),
        (Tag::DoViewChange, HeaderSlotRole::Frontier),
        (Tag::StartView, HeaderSlotRole::Frontier),
        (Tag::PlannedViewChange, HeaderSlotRole::Absent),
        (Tag::Recovery, HeaderSlotRole::Absent),
        (Tag::RecoveryResponse, HeaderSlotRole::Frontier),
        (Tag::GetState, HeaderSlotRole::Frontier),
        (Tag::NewState, HeaderSlotRole::Frontier),
        (Tag::Reply, HeaderSlotRole::Absent),
    ];
    for (tag, role) in expected {
        assert_eq!(header_slot_role(tag), role, "{tag:?}");
        // The role predicate and the table agree by construction; pin the
        // predicate's edge behaviour too.
        match role {
            HeaderSlotRole::Operation => {
                assert!(!role.admits(Slot(0)));
                assert!(role.admits(Slot(1)));
            }
            HeaderSlotRole::Frontier => {
                assert!(role.admits(Slot(0)));
                assert!(role.admits(Slot(u64::MAX)));
            }
            HeaderSlotRole::Absent => {
                assert!(role.admits(Slot(0)));
                assert!(!role.admits(Slot(1)));
            }
        }
    }
}

/// Rule 7, `Operation` class: `Prepare`/`PrepareOk` name a log position, and
/// position 0 holds nothing — the first operation of a legitimate history is
/// `Void` at slot 1 (§8.7.2).
#[test]
fn rule7_operation_tags_must_name_a_slot() {
    let table = genesis_table();
    let old = normal(ViewId::INITIAL, 4, 2, 1, 1, 0, &table);
    let candidate = normal(ViewId::INITIAL, 5, 2, 1, 1, 1, &table);

    for tag in [Tag::Prepare, Tag::PrepareOk] {
        let violating = InputKind::PeerMessage { tag, slot: Slot(0) };
        assert_eq!(
            legal(&old, &candidate, &violating),
            Some(Fault::IllegalTransition),
            "{tag:?} at slot 0"
        );
        let passing = InputKind::PeerMessage { tag, slot: Slot(5) };
        assert_eq!(legal(&old, &candidate, &passing), None, "{tag:?} at slot 5");
    }
}

/// Rule 7, `Absent` class: these tags carry no meaningful slot (item02d), so the
/// only legal value is the sentinel `Slot(0)` — never a fabricated position.
#[test]
fn rule7_absent_tags_must_send_the_sentinel() {
    let table = genesis_table();
    let old = normal(ViewId::INITIAL, 4, 2, 1, 1, 0, &table);
    let candidate = normal(ViewId::INITIAL, 5, 2, 1, 1, 1, &table);

    for tag in [
        Tag::Request,
        Tag::StartViewChange,
        Tag::PlannedViewChange,
        Tag::Recovery,
        Tag::Reply,
    ] {
        let violating = InputKind::PeerMessage { tag, slot: Slot(1) };
        assert_eq!(
            legal(&old, &candidate, &violating),
            Some(Fault::IllegalTransition),
            "{tag:?} at slot 1"
        );
        let passing = InputKind::PeerMessage { tag, slot: Slot(0) };
        assert_eq!(legal(&old, &candidate, &passing), None, "{tag:?} sentinel");
    }
}

/// Rule 7, `Frontier` class: these tags carry the frontier they speak about, and
/// every slot value is meaningful — `Slot(0)` is the empty frontier a genesis
/// node legitimately reports. The class has no violating value at the header
/// level by design: whether a claimed frontier is *believable* is a transition
/// rule of the replica (item07), not a property of the header.
#[test]
fn rule7_frontier_tags_admit_any_slot() {
    let table = genesis_table();
    let old = normal(ViewId::INITIAL, 4, 2, 1, 1, 0, &table);
    let candidate = normal(ViewId::INITIAL, 5, 2, 1, 1, 1, &table);

    for tag in [
        Tag::Commit,
        Tag::DoViewChange,
        Tag::StartView,
        Tag::RecoveryResponse,
        Tag::GetState,
        Tag::NewState,
    ] {
        for slot in [Slot(0), Slot(7), Slot(u64::MAX)] {
            let input = InputKind::PeerMessage { tag, slot };
            assert_eq!(legal(&old, &candidate, &input), None, "{tag:?} at {slot:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Era/slot discipline (W1, §8.7.3)
// ---------------------------------------------------------------------------

/// The boundary case: `era(accepted) == era(current) + 1` is overlap mode and is
/// legal; `+2` skips a configuration whose intersection obligations were never
/// checked (Q1) and is refused. Enforcement is at construction — an undisciplined
/// value is unrepresentable — with `legal` rule 6 as the belt-and-braces check on
/// every candidate.
#[test]
fn era_slot_discipline_boundary() {
    // Era 1 view, accepted frontier in era 2 (established at slot 3): legal.
    let overlap = normal(view(1, 5), 3, 0, 0, 0, 0, &table_upto(2));
    let advanced = normal(view(1, 5), 3, 1, 0, 0, 1, &table_upto(2));
    assert_eq!(legal(&overlap, &advanced, &InputKind::ClientRequest), None);

    // Era 1 view, accepted frontier in era 3 (established at slot 4): refused.
    assert_eq!(
        Progress::reconstitute(
            view(1, 5),
            view(1, 5),
            Status::Normal,
            Slot(4),
            Slot(0),
            Slot(0),
            Slot(0),
            0,
            table_upto(3),
            None,
        )
        .unwrap_err(),
        ProgressError::EraSlotDiscipline
    );

    // A transition that would carry the accepted frontier out of the window is
    // refused rather than validated after the fact.
    let in_era = normal(view(1, 5), 2, 0, 0, 0, 0, &table_upto(2));
    assert!(in_era.with_accepted(Slot(3)).is_ok(), "overlap is legal");
    // With only eras <= 2 established, slot 4 still attributes to era 2: the
    // table is the authority on which era a slot belongs to.
    assert!(in_era.with_accepted(Slot(4)).is_ok());

    // A configuration swap that orphans the accepted frontier is refused: with
    // era 3 established at slot 4, an accepted frontier of 2 belongs to no era
    // the table can still name.
    assert_eq!(
        in_era.with_config(table_upto(3)).unwrap_err(),
        ProgressError::EraSlotDiscipline
    );
    // A backwards table is refused on its own terms.
    assert_eq!(
        normal(view(2, 7), 3, 0, 0, 0, 0, &table_upto(2))
            .with_config(table_upto(1))
            .unwrap_err(),
        ProgressError::ConfigRegress
    );
}

// ---------------------------------------------------------------------------
// 7. The seqlock (B1)
// ---------------------------------------------------------------------------

/// A snapshot whose two fields satisfy an invariant a torn read would violate.
#[derive(Clone, Copy, Debug)]
struct Pair {
    value: u64,
    doubled: u64,
}

/// Deterministic single-threaded behaviour: `new` publishes the initial value,
/// `write` replaces it, `read` returns the latest published value.
#[test]
fn observation_read_returns_latest_write() {
    let observation = Observation::new(Pair {
        value: 0,
        doubled: 0,
    });
    let initial = observation.read();
    assert_eq!(initial.value, 0);
    assert_eq!(initial.doubled, 0);

    observation.write(Pair {
        value: 7,
        doubled: 14,
    });
    let latest = observation.read();
    assert_eq!(latest.value, 7);
    assert_eq!(latest.doubled, 14);
}

/// The race B1 exists to win: one writer publishing an incrementing counter,
/// readers asserting `doubled == value * 2` on every read. A torn read — half of
/// one write, half of another — violates that relation, so any seqlock bug that
/// returns one fails here. Run under `--release` per the item brief; debug runs
/// exercise the same interleavings more slowly.
#[test]
fn observation_never_returns_a_torn_read() {
    const WRITES: u64 = 100_000;

    let observation = Arc::new(Observation::new(Pair {
        value: 0,
        doubled: 0,
    }));
    let done = Arc::new(AtomicBool::new(false));

    let readers: Vec<_> = (0..3)
        .map(|_| {
            let observation = Arc::clone(&observation);
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                let mut reads = 0u64;
                while !done.load(Ordering::Relaxed) {
                    let snapshot = observation.read();
                    assert_eq!(
                        snapshot.doubled,
                        snapshot.value * 2,
                        "torn read: {snapshot:?}"
                    );
                    reads += 1;
                }
                reads
            })
        })
        .collect();

    let writer = {
        let observation = Arc::clone(&observation);
        std::thread::spawn(move || {
            for value in 1..=WRITES {
                observation.write(Pair {
                    value,
                    doubled: value * 2,
                });
            }
        })
    };

    writer.join().expect("writer panicked");
    done.store(true, Ordering::Relaxed);
    for reader in readers {
        let reads = reader.join().expect("reader panicked");
        assert!(reads > 0, "reader observed nothing");
    }

    let final_snapshot = observation.read();
    assert_eq!(final_snapshot.value, WRITES);
    assert_eq!(final_snapshot.doubled, WRITES * 2);
}

// ---------------------------------------------------------------------------
// 8. Genesis and the `ViewId::INITIAL` ruling
// ---------------------------------------------------------------------------

/// The ruling (item01 follow-on, resolved here): `ViewId::INITIAL` is pinned as
/// the **genesis view** — the `(era 0, view 0)` pair a freshly provisioned node
/// advertises. Era 0 is the void configuration, quorum-impossible by arithmetic
/// (see `Configuration::void`), and view 0 is the first primary term once `Init`
/// commits. It is not "no view": a freshly provisioned node has a real genesis
/// view, so no `Option<ViewId>` appears anywhere. Per §5 the genesis node is
/// fenced and recovering until it proves its state current.
#[test]
fn genesis_advertises_view_id_initial_fenced() {
    let table = genesis_table();
    let genesis = Progress::genesis(Arc::clone(&table)).expect("genesis");

    assert_eq!(genesis.current(), ViewId::INITIAL);
    assert_eq!(genesis.retained(), ViewId::INITIAL);
    assert_eq!(genesis.status(), Status::Recovering);
    assert_eq!(genesis.accepted(), Slot::FIRST);
    assert_eq!(genesis.committed(), Slot::FIRST);
    assert_eq!(genesis.applied(), Slot::FIRST);
    assert_eq!(genesis.checkpoint(), Slot::FIRST);
    assert_eq!(genesis.revision(), 0);
    assert_eq!(genesis.fault(), None);
    assert!(Arc::ptr_eq(genesis.config(), &table));

    // A table that has moved past genesis is not a genesis table: the empty
    // frontier belongs to no era such a table can still name.
    assert_eq!(
        Progress::genesis(table_upto(2)).unwrap_err(),
        ProgressError::EraSlotDiscipline
    );
}

/// The snapshot is the flat POD the seqlock publishes (B1): every field copied
/// out by value, status encoded as a word that round-trips.
#[test]
fn snapshot_is_flat_and_decodes() {
    for (status, word) in [
        (Status::Normal, 0),
        (Status::ViewChange, 1),
        (Status::Recovering, 2),
        (Status::Replaying, 3),
    ] {
        assert_eq!(status.to_word(), word);
        assert_eq!(Status::from_word(word), Some(status));
    }
    assert_eq!(Status::from_word(4), None);

    let progress = normal(view(2, 9), 3, 2, 1, 1, 9, &table_upto(2));
    let snapshot = progress.to_snapshot();
    assert_eq!(snapshot.era, 2);
    assert_eq!(snapshot.view, 9);
    assert_eq!(snapshot.retained_era, 2);
    assert_eq!(snapshot.retained_view, 9);
    assert_eq!(snapshot.status, Status::Normal.to_word());
    assert!(!snapshot.faulted);
    assert_eq!(snapshot.accepted, 3);
    assert_eq!(snapshot.committed, 2);
    assert_eq!(snapshot.applied, 1);
    assert_eq!(snapshot.checkpoint, 1);
    assert_eq!(snapshot.revision, 9);

    let faulted = progress
        .with_fault(Fault::HostDeclared)
        .expect("unfaulted progress accepts a fault");
    assert!(faulted.to_snapshot().faulted);
}

/// The transition functions refuse what the checker would refuse, so an invalid
/// value never exists to be checked: regression, non-successor views, and
/// faulted receivers are all refused at the constructor boundary.
#[test]
fn transitions_validate_their_results() {
    let table = genesis_table();
    let progress = normal(view(0, 3), 5, 2, 1, 1, 0, &table);

    assert_eq!(
        progress.with_accepted(Slot(4)).unwrap_err(),
        ProgressError::FrontierRegress
    );
    assert_eq!(
        progress.with_committed(Slot(6)).unwrap_err(),
        ProgressError::FrontierChain
    );
    assert_eq!(
        progress.with_applied(Slot(3)).unwrap_err(),
        ProgressError::FrontierChain
    );
    assert_eq!(
        progress.with_checkpoint(Slot(2)).unwrap_err(),
        ProgressError::FrontierChain
    );
    assert_eq!(
        progress.with_view_change(view(0, 3)).unwrap_err(),
        ProgressError::ViewSuccessor
    );
    assert_eq!(
        progress.with_view_change(view(0, 2)).unwrap_err(),
        ProgressError::ViewSuccessor
    );

    // View installation completes a change: retained and current join, status
    // Normal, and the installed frontier may shorten — but never below
    // `committed`.
    let installed = progress
        .with_view_change(view(0, 4))
        .expect("legal successor")
        .with_view_installed(view(0, 4), Slot(3))
        .expect("installed history still covers committed");
    assert_eq!(installed.status(), Status::Normal);
    assert_eq!(installed.current(), view(0, 4));
    assert_eq!(installed.retained(), view(0, 4));
    assert_eq!(installed.accepted(), Slot(3));
    assert_eq!(installed.revision(), 2);

    assert_eq!(
        progress
            .with_view_installed(view(0, 4), Slot(1))
            .unwrap_err(),
        ProgressError::FrontierChain
    );
    assert_eq!(
        progress
            .with_view_installed(view(0, 2), Slot(5))
            .unwrap_err(),
        ProgressError::ViewSuccessor
    );
}
