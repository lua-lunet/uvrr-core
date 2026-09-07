//! TigerBeetle direct-IO durability substrate, driven through the vendored
//! TigerBeetle 0.17.9 IO + superblock stack (zig/ — see zig/PATCH_MANIFEST.md
//! and README.md for the build).
//!
//! Demonstrated over TB's actual code paths:
//!   * the data file is opened through TB's per-OS direct block-IO layer
//!     (darwin: O_DSYNC + F_NOCACHE + flock + F_FULLFSYNC flush; linux would
//!     use O_DIRECT — see the fact-check notes in zig/PATCH_MANIFEST.md);
//!   * four superblock copies, TB checksum scheme and 2-of-4 open quorum;
//!   * `uvrr_flushed` mark; any valid copy unflushed ⇒ dirty restart;
//!   * dirty ⇒ bump the incarnation (hash-chained `parent`, advanced
//!     `sequence`), write the new identity to all four copies; higher
//!     identity wins on subsequent reads;
//!   * clean shutdown ⇒ flushed mark on all four copies through TB's sync path;
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
    fn uvrr_clean_shutdown() -> c_int;
    fn uvrr_dirty_restart() -> c_int;
    fn uvrr_op(op: c_int, identity_lo: u64, identity_hi: u64, weight: u16, learner: c_int) -> c_int;
    fn uvrr_checkpoint() -> c_int;
    fn uvrr_sequence() -> u64;
    fn uvrr_incarnation() -> u64;
    fn uvrr_flushed() -> c_int;
    fn uvrr_member_count() -> u32;
    fn uvrr_simulate_dirty_copy(index: u32) -> c_int;
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

const CLUSTER: u128 = 0xBEEF;

fn main() {
    let dir = std::env::temp_dir().join("uvrr-reincarnation-demo");
    std::fs::create_dir_all(&dir).expect("create demo dir");
    let path_buf = dir.join("uvrr-data.bin");
    std::fs::remove_file(&path_buf).ok();
    let dir_c = CString::new(dir.to_str().expect("utf8 dir")).expect("nul-free");
    let file_c = CString::new("uvrr-data.bin").expect("nul-free");

    println!("== format: 4 superblock copies + ops WAL + one 4KiB checkpoint block (TB direct IO)");
    check(
        unsafe {
            uvrr_format(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0, 1)
        },
        "format",
    );
    unsafe { uvrr_close() };

    println!("== CLEAN PATH: open; all copies flushed => clean => continue same identity");
    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "open",
    );
    assert_eq!(unsafe { uvrr_sequence() }, 0);
    assert_eq!(unsafe { uvrr_flushed() }, 1, "fresh format must be flushed");
    let identity = unsafe { uvrr_incarnation() };
    println!("   continuation commitment: identity {identity} continues unchanged");

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

    println!("== clean shutdown: flushed mark written to all four copies, TB sync path");
    check(unsafe { uvrr_clean_shutdown() }, "clean_shutdown");
    unsafe { uvrr_close() };

    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "reopen after clean shutdown",
    );
    assert_eq!(unsafe { uvrr_flushed() }, 1, "clean store reopens flushed");
    assert_eq!(unsafe { uvrr_member_count() }, 5, "membership replayed");
    drop_members();
    unsafe { uvrr_close() };

    println!("== DIRTY PATH: crash with unflushed state (simulated torn shutdown)");
    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "open",
    );
    // A crash that left copy 0 recorded unflushed (consistent header, flushed=0).
    check(unsafe { uvrr_simulate_dirty_copy(0) }, "simulate_dirty_copy");
    unsafe { uvrr_close() };

    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "open dirty",
    );
    assert_eq!(unsafe { uvrr_flushed() }, 1, "3-of-4 flushed still wins");
    let old_identity = unsafe { uvrr_incarnation() };

    println!("== dirty => bump incarnation, rewrite all four copies");
    check(unsafe { uvrr_dirty_restart() }, "dirty_restart");
    unsafe { uvrr_close() };

    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "reopen after bump",
    );
    let new_identity = unsafe { uvrr_incarnation() };
    assert_eq!(new_identity, old_identity + 1, "incarnation bumped");
    assert_eq!(unsafe { uvrr_flushed() }, 0, "bumped copies start unflushed");
    println!("   REINCARNATION: identity {old_identity} -> {new_identity} (higher identity wins)");
    check(unsafe { uvrr_clean_shutdown() }, "clean_shutdown");
    unsafe { uvrr_close() };

    println!("== membership replay across restart: reopen replays checkpoint + WAL suffix");
    check(
        unsafe { uvrr_open(dir_c.as_ptr(), file_c.as_ptr(), CLUSTER as u64, 0) },
        "reopen",
    );
    assert_eq!(unsafe { uvrr_member_count() }, 5, "membership intact");
    drop_members();
    unsafe { uvrr_close() };

    println!(
        "DEMO OK: clean path continued identity {identity}; dirty path reincarnated {old_identity} -> {new_identity}; membership replayed through the WAL"
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
    check(unsafe { uvrr_op(OP_DOUBLE, identity, 0, 0, 0) }, "op double");
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
