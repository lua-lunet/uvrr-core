//! TigerBeetle direct-IO durability substrate, driven through the vendored
//! TigerBeetle 0.17.9 IO + superblock stack (zig/ — see zig/PATCH_MANIFEST.md
//! and README.md for the build).
//!
//! The marker transition machine (docs/vrr-durability-model.md §5.1; the
//! Rust twin src/replica/reincarnation.rs), demonstrated over TB's actual
//! code paths:
//!   * the data file is opened through TB's per-OS direct block-IO layer
//!     (darwin: O_DSYNC + F_NOCACHE + flock + F_FULLFSYNC flush; linux would
//!     use O_DIRECT — see the fact-check notes in zig/PATCH_MANIFEST.md);
//!   * four superblock copies, TB checksum scheme and 2-of-4 open quorum
//!     over the sequence hash-chain;
//!   * T1: `uvrr_begin_stop` writes `stopping` 4x, the host drain sits
//!     strictly between, `uvrr_finish_stop` writes `stopped` 4x — the
//!     marker order is the drain's proof, so a `stopped` copy vouches for
//!     the WAL under it (this demo's WAL and checkpoint writes are already
//!     durable-on-write through the direct-IO layer, so the drain has
//!     nothing left to flush);
//!   * T2: a boot reading 2-of-4 `stopped` is the clean stop — continue
//!     under the same identity, write `restarting` 4x;
//!   * T3: a boot reading no stopped quorum — a crash (the running state's
//!     `restarting` markers), death mid-join (`joining` markers) — bumps
//!     the identity and writes `joining` 4x: the reincarnation;
//!   * no `started` state is written: no safety logic looks for it;
//!   * membership ops (add_one/remove_one/double/halve) through the ops WAL,
//!     checkpointed into one 4 KiB block, replayed on reopen.
//!
//! Dependency-free: std only. Run through `build.sh`, which compiles the
//! vendored Zig tree with TB's pinned Zig 0.14.1 and links it.

use std::ffi::{CString, c_char, c_int};

#[link(name = "uvrr_sb", kind = "static")]
unsafe extern "C" {
    fn uvrr_format(
        dir_path: *const c_char,
        file_name: *const c_char,
        cluster_lo: u64,
        cluster_hi: u64,
        incarnation: u64,
    ) -> c_int;
    fn uvrr_open(
        dir_path: *const c_char,
        file_name: *const c_char,
        cluster_lo: u64,
        cluster_hi: u64,
    ) -> c_int;
    fn uvrr_begin_stop() -> c_int;
    fn uvrr_finish_stop() -> c_int;
    fn uvrr_op(op: c_int, identity_lo: u64, identity_hi: u64, weight: u16, learner: c_int)
    -> c_int;
    fn uvrr_checkpoint() -> c_int;
    fn uvrr_sequence() -> u64;
    fn uvrr_incarnation() -> u64;
    fn uvrr_marker() -> c_int;
    fn uvrr_member_count() -> u32;
    fn uvrr_member(
        index: u32,
        identity_lo: *mut u64,
        identity_hi: *mut u64,
        weight: *mut u16,
        learner: *mut c_int,
    ) -> c_int;
    fn uvrr_close();
}

fn check(rc: c_int, what: &str) {
    if rc != 0 {
        panic!("{what} failed with {rc}");
    }
}

const OP_ADD_ONE: c_int = 0;
const OP_REMOVE_ONE: c_int = 1;
const OP_DOUBLE: c_int = 2;
const OP_HALVE: c_int = 3;

const MARKER_STOPPING: c_int = 0;
const MARKER_STOPPED: c_int = 1;
const MARKER_RESTARTING: c_int = 2;
const MARKER_JOINING: c_int = 3;

const CLUSTER: u128 = 0xBEEF;

fn marker() -> c_int {
    unsafe { uvrr_marker() }
}

fn incarnation() -> u64 {
    unsafe { uvrr_incarnation() }
}

fn main() {
    let dir = std::env::temp_dir().join("uvrr-reincarnation-demo");
    std::fs::create_dir_all(&dir).expect("create demo dir");
    let path_buf = dir.join("uvrr-data.bin");
    std::fs::remove_file(&path_buf).ok();
    let dir_c = CString::new(dir.to_str().expect("utf8 dir")).expect("nul-free");
    let file_c = CString::new("uvrr-data.bin").expect("nul-free");
    let open = |dir_c: &CString, file_c: &CString| {
        check(
            unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
            "open",
        );
    };

    println!("== format: 4 superblock copies + ops WAL + one 4KiB checkpoint block (TB direct IO)");
    check(
        unsafe { uvrr_format(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0, 1) },
        "format",
    );
    unsafe { uvrr_close() };

    println!(
        "== T2 boot: the pristine stopped copyset reads clean at 2-of-4 => continue identity 1"
    );
    open(&dir_c, &file_c);
    assert_eq!(incarnation(), 1, "the clean stop continues the identity");
    assert_eq!(marker(), MARKER_RESTARTING, "the boot writes restarting 4x");
    println!("   continuation commitment: identity 1 continues unchanged");

    println!("== membership: add_one/remove_one/double/halve through the ops WAL");
    member_add(1, 1);
    member_add(2, 1);
    member_add(3, 2);
    assert_eq!(unsafe { uvrr_member_count() }, 3, "three members");
    member_double(1);
    member_halve(3);
    member_remove(2);
    assert_eq!(unsafe { uvrr_member_count() }, 2, "two members left");
    member_add(4, 1); // learner
    member_add(5, 4);
    member_add(6, 2);
    println!("   members now: {}", unsafe { uvrr_member_count() });
    check(unsafe { uvrr_checkpoint() }, "checkpoint");

    println!("== T1 stop: begin_stop writes stopping 4x; the drain; finish_stop writes stopped 4x");
    let seq_before = unsafe { uvrr_sequence() };
    check(unsafe { uvrr_begin_stop() }, "begin_stop");
    assert_eq!(marker(), MARKER_STOPPING, "the stop command is written 4x");
    assert_eq!(
        unsafe { uvrr_sequence() },
        seq_before + 1,
        "each marker write is a new parent-chained copyset"
    );
    // The drain: flush the WALs and the grids. This demo's writes are
    // durable-on-write through TB's direct-IO layer, so nothing is left to
    // flush — the marker order still IS the proof discipline.
    check(unsafe { uvrr_finish_stop() }, "finish_stop");
    assert_eq!(marker(), MARKER_STOPPED, "the drain's proof is written 4x");
    assert_eq!(unsafe { uvrr_sequence() }, seq_before + 2);
    unsafe { uvrr_close() };

    println!("== T2 boot again: the stopped copyset is the clean stop => identity 1 continues");
    open(&dir_c, &file_c);
    assert_eq!(incarnation(), 1);
    assert_eq!(marker(), MARKER_RESTARTING);
    assert_eq!(unsafe { uvrr_member_count() }, 5, "membership replayed");
    drop_members();
    unsafe { uvrr_close() };
    // The crash while running: the node's markers hold `restarting` from
    // the boot above and it died without stopping. The next open IS the
    // boot that reads them.

    println!("== T3 boot: the crash left the restarting markers => the identity is dead, bump");
    open(&dir_c, &file_c);
    assert_eq!(incarnation(), 2, "the crash bumps the identity");
    assert_eq!(marker(), MARKER_JOINING, "the bump writes joining 4x");
    println!("   REINCARNATION: identity 1 -> 2 (not a member until the forced sequence seats it)");
    unsafe { uvrr_close() };

    println!("== T3 boot again: death mid-join (joining markers) reincarnates again");
    open(&dir_c, &file_c);
    assert_eq!(incarnation(), 3, "the identity only moves forward");
    assert_eq!(marker(), MARKER_JOINING);

    println!("== the reincarnated node stops cleanly: T1 then T2 continues identity 3");
    check(unsafe { uvrr_begin_stop() }, "begin_stop");
    check(unsafe { uvrr_finish_stop() }, "finish_stop");
    unsafe { uvrr_close() };
    open(&dir_c, &file_c);
    assert_eq!(incarnation(), 3, "the bumped identity continues");
    assert_eq!(marker(), MARKER_RESTARTING);
    assert_eq!(unsafe { uvrr_member_count() }, 5, "membership intact");
    drop_members();
    unsafe { uvrr_close() };

    println!(
        "DEMO OK: T1 proved the drain (stopped 4x), T2 continued identities 1 and 3, T3 reincarnated 1 -> 2 -> 3; membership replayed through the WAL"
    );
}

fn member_add(identity: u64, weight: u16) {
    check(
        unsafe { uvrr_op(OP_ADD_ONE, identity, 0, weight, 0) },
        "op add_one",
    );
}

fn member_remove(identity: u64) {
    check(
        unsafe { uvrr_op(OP_REMOVE_ONE, identity, 0, 0, 0) },
        "op remove_one",
    );
}

fn member_double(identity: u64) {
    check(
        unsafe { uvrr_op(OP_DOUBLE, identity, 0, 0, 0) },
        "op double",
    );
}

fn member_halve(identity: u64) {
    check(unsafe { uvrr_op(OP_HALVE, identity, 0, 0, 0) }, "op halve");
}

fn drop_members() {
    let count = unsafe { uvrr_member_count() };
    for i in 0..count {
        let mut lo = 0u64;
        let mut hi = 0u64;
        let mut weight = 0u16;
        let mut learner: c_int = 0;
        check(
            unsafe { uvrr_member(i, &mut lo, &mut hi, &mut weight, &mut learner) },
            "member read",
        );
        println!(
            "   member[{i}]: identity {:#x} weight {weight} learner {learner}",
            ((hi as u128) << 64) | lo as u128
        );
    }
}
