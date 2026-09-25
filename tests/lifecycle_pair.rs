//! The marker carries the durable identity pair (`docs/uvrr-boot-gate.md`
//! §5): the sysadmin-assigned system identifier as the high half, the crash
//! counter as the low half, packed into the boundary `NodeId` the wire, the
//! disk, and the C ABI all see. These tests pin the format contract of the
//! reference marker files: the lawful pair survives write and read, a zero
//! half is a corrupt marker and refuses, an old-format marker refuses with
//! the incompatible refusal, the bump at the sixteenth bit of lives is the
//! exhaustion refusal rather than a wrap, and the machine never lets a
//! crash self-reset to zero: it picks up its actual identity or it refuses.

use std::fs;
use std::path::PathBuf;

mod harness;

use harness::{TmpGate, mint_pair};
use vrr::ids::{CrashCounter, NodeId, SystemId};
use vrr::lifecycle::{
    CopyState, LifecycleStore, Marker, RestartClass, RestartDecision, RestartRefusal,
    SuperblockCopies,
};

/// A fresh gate directory.
fn gate_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "uvrr-lifecycle-pair-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// The pair's identity, packed.
fn packed(system: SystemId, crash: CrashCounter) -> NodeId {
    NodeId::new(system, crash)
}

/// Four uniform copies of one marker state.
fn copies(identity: NodeId, marker: Marker) -> SuperblockCopies {
    SuperblockCopies {
        copies: core::array::from_fn(|_| CopyState { identity, marker }),
    }
}

#[test]
fn the_lawful_pair_survives_the_marker_write_and_read() {
    // Minted once, owned for the whole run, checked at the end: the
    // identity that went in is the identity that came back, halves intact.
    let (system, crash) = mint_pair();
    let id = packed(system, crash);
    let dir = gate_dir("survives");
    let mut gate = TmpGate::open(dir.clone()).expect("the gate opens");
    let committed = copies(id, Marker::Joining);
    gate.commit(&committed).expect("the pair commits");
    let read = gate.read_copies().expect("the pair reads back");
    let read_copies = read.expect("a committed pair is present");
    assert_eq!(read_copies.copies[0].identity, id);
    assert_eq!(read_copies.copies[3].identity, id);
    let read_system = id.system_id().expect("the system half decodes");
    let read_crash = id.crash_counter().expect("the counter half decodes");
    assert_eq!(read_system, system, "the system half is the minted one");
    assert_eq!(read_crash, crash, "the counter half is the minted one");
    // The raw files name the pair in the stamped format.
    let raw = fs::read_to_string(dir.join("copy0")).expect("the raw file reads");
    assert!(
        raw.starts_with("v1\n"),
        "the marker file carries the format version: {raw:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_zero_half_is_a_corrupt_marker_and_refuses() {
    let dir = gate_dir("zero-half");
    let mut gate = TmpGate::open(dir.clone()).expect("the gate opens");
    fs::write(dir.join("copy0"), "v1\n0\nStopped\n").expect("the corrupt file writes");
    let refusal = gate
        .read_copies()
        .expect_err("a zero counter half is no identity");
    assert!(
        refusal.to_string().contains("corrupt"),
        "the refusal names the corruption: {refusal}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_old_format_marker_refuses_as_incompatible() {
    let dir = gate_dir("old-format");
    let mut gate = TmpGate::open(dir.clone()).expect("the gate opens");
    // The pre-pair format: bare identity and marker, no version stamp.
    fs::write(dir.join("copy0"), "123\nStopped\n").expect("the old-format file writes");
    let refusal = gate
        .read_copies()
        .expect_err("an old-format marker is incompatible");
    assert!(
        refusal.to_string().contains("incompatible"),
        "the refusal names the incompatibility: {refusal}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_bump_at_the_last_life_refuses_instead_of_wrapping() {
    let (system, _) = mint_pair();
    let last = CrashCounter::new(u16::MAX).expect("65535 is lawful");
    let id = NodeId::new(system, last);
    assert_eq!(
        id.next_life(),
        None,
        "the sixteenth bit of lives is the last the packing holds"
    );
}

#[test]
fn a_crash_picks_up_its_actual_identity_and_never_self_resets() {
    // The full lifecycle at a minted identity: clean stop, clean restart
    // under the SAME pair, then two crashes, each bumping the counter by
    // exactly one. The system half never moves, the counter never
    // revisits, and nothing ever reads as zero or reverts to one.
    let (system, crash) = mint_pair();
    let life = packed(system, crash);
    let mut marker_set = copies(life, Marker::Joining);

    // The node halts cleanly and restarts: same identity, no bump.
    marker_set = marker_set.begin_stop();
    marker_set = marker_set.finish_stop();
    let (decision, written) = marker_set.restart().expect("the stopped quorum decides");
    assert_eq!(
        decision,
        RestartDecision::Continue { identity: life },
        "the clean start continues the minted identity"
    );
    assert_eq!(written.copies[0].identity, life);

    // First crash on the set the clean start wrote: the counter advances
    // by exactly one, the system half stands still.
    let bumped = life
        .next_life()
        .expect("the minted counter is below the bound");
    let (decision, written) = written.restart().expect("the crash decides");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: life,
            new: bumped
        },
        "the crash bumps the actual identity"
    );
    assert_eq!(written.copies[0].identity, bumped);
    assert_eq!(bumped.system_id(), Some(system));
    assert_eq!(
        bumped.crash_counter().expect("the counter decodes").get(),
        crash.get() + 1,
        "the counter advanced by exactly one"
    );
    assert_ne!(bumped.0, 0);
    assert_ne!(bumped.0, 1, "nothing reverts to one");

    // Second crash on the set the first bump wrote: the counter advances
    // again, never revisiting, never zeroing.
    let twice = bumped.next_life().expect("still below the bound");
    let (decision, _) = written.restart().expect("the second crash decides");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: bumped,
            new: twice
        },
        "each life is one counter advance past the last identity the disk saw"
    );
    assert_eq!(twice.system_id(), Some(system));
}

#[test]
fn both_halves_at_half_the_maximum_never_revert() {
    // The user's probe: both halves at u16::MAX / 2, the whole lifecycle
    // run, and the values must come back exactly, never reverted to a
    // memorised small pattern such as zero or one.
    let system = SystemId::new(u16::MAX / 2).expect("32767 is lawful");
    let crash = CrashCounter::new(u16::MAX / 2).expect("32767 is lawful");
    let life = NodeId::new(system, crash);
    let bumped = life.next_life().expect("32767 is below the bound");
    assert_eq!(bumped.system_id(), Some(system));
    assert_eq!(
        bumped.crash_counter(),
        Some(CrashCounter::new(crash.get() + 1).expect("32768 is lawful")),
        "the counter half advanced by exactly one at the half-maximum mint"
    );
    let (decision, _) = copies(life, Marker::Restarting)
        .restart()
        .expect("the crash decides at the half-maximum mint");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: life,
            new: bumped
        },
        "the bump at the half-maximum mint names the exact pair"
    );
}

#[test]
fn a_marker_set_of_blanks_reads_as_lost_never_as_a_zero_identity() {
    // The zero pattern forms no cohort: three blanks and one real
    // identity read as a torn set, and the machine refuses rather than
    // bumping a zero identity into circulation. The self-reset to zero
    // that the pre-pair world permitted is unrepresentable.
    let (system, crash) = mint_pair();
    let life = NodeId::new(system, crash);
    let mut torn = copies(NodeId(0), Marker::Stopping);
    torn.copies[0] = CopyState {
        identity: life,
        marker: Marker::Restarting,
    };
    assert_eq!(
        torn.restart(),
        Err(RestartRefusal::QuorumLost),
        "no identity cohort reaches the open threshold"
    );
    // All four blank: the same verdict, not a zero bump.
    assert_eq!(
        copies(NodeId(0), Marker::Stopping).restart(),
        Err(RestartRefusal::QuorumLost)
    );
}

#[test]
fn the_working_cohort_resolves_by_the_packed_order() {
    // Two cohorts reach the threshold: the higher packed identity wins,
    // whatever the mint drew, so a lone superseded copy cannot impose.
    let (high_system, high_crash) = mint_pair();
    let high = NodeId::new(high_system, high_crash);
    // The low cohort: the counter half drawn one lower, lawful.
    let low_crash =
        CrashCounter::new(high_crash.get() - 1).expect("the drawn counter is above one");
    let low = NodeId::new(high_system, low_crash);
    let mut set = copies(low, Marker::Stopped);
    set.copies[2] = CopyState {
        identity: high,
        marker: Marker::Restarting,
    };
    set.copies[3] = CopyState {
        identity: high,
        marker: Marker::Restarting,
    };
    let (class, identity) = set.classify().expect("two cohorts reach the threshold");
    assert_eq!(class, RestartClass::NotStopped);
    assert_eq!(
        identity, high,
        "the winner is the highest working cohort by the packed order"
    );
}
