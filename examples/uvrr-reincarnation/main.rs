//! TigerBeetle-style four-superblock durability substrate, driven through the
//! extracted TigerBeetle 0.17.9 superblock code (see README.md for the build).
//!
//! Director's delta demonstrated:
//!   * four superblock copies on disk, TB layout/checksums verbatim;
//!   * `flushed`/`unflushed` bit (in TB's reserved `flags` field);
//!   * restart reads all FOUR; ANY valid copy unflushed ⇒ dirty;
//!   * dirty ⇒ bump the incarnation (TB's `sequence`, hash-chained by
//!     `parent`), write the new identity + `flushed` to all four; higher
//!     identity wins on subsequent reads;
//!   * clean shutdown ⇒ flush writes, fsync, mark `flushed` in all four;
//!   * continuation commitment: the clean path continues the same identity.
//!
//! Dependency-free: std only. Run through `build.sh`, which compiles the Zig
//! extraction and links it.

use std::ffi::{CStr, CString, c_char, c_int};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct UvrrState {
    adopted_sequence: u64,
    flushed: bool,
    dirty: bool,
    valid_copies: u8,
    unflushed_copies: u8,
}

// Opaque handle: TigerBeetle's SuperBlockHeader by value.
#[repr(C)]
struct Handle {
    _header: [u8; 0],
    _align: u64,
}

#[link(name = "uvrr_sb", kind = "static")]
unsafe extern "C" {
    fn uvrr_sb_format(path: *const c_char, cluster: u64, out: *mut *mut Handle) -> c_int;
    fn uvrr_sb_open(path: *const c_char, out: *mut *mut Handle, state: *mut UvrrState) -> c_int;
    fn uvrr_sb_bump(handle: *mut Handle, path: *const c_char) -> c_int;
    fn uvrr_sb_set_flushed(handle: *mut Handle, path: *const c_char, flushed: bool) -> c_int;
    fn uvrr_sb_copy_size() -> u64;
    fn uvrr_sb_close(handle: *mut Handle);
}

fn check(rc: c_int, what: &str) {
    if rc != 0 {
        panic!("{what} failed with {rc}");
    }
}

struct Superblock {
    handle: *mut Handle,
    path: CString,
}

impl Superblock {
    fn format(path: &CStr, cluster: u64) -> Self {
        let mut handle: *mut Handle = std::ptr::null_mut();
        let rc = unsafe { uvrr_sb_format(path.as_ptr(), cluster, &mut handle) };
        check(rc, "format");
        Self {
            handle,
            path: path.to_owned(),
        }
    }

    fn open(path: &CStr) -> (Self, UvrrState) {
        let mut handle: *mut Handle = std::ptr::null_mut();
        let mut state = UvrrState::default();
        let rc = unsafe { uvrr_sb_open(path.as_ptr(), &mut handle, &mut state) };
        check(rc, "open");
        (
            Self {
                handle,
                path: path.to_owned(),
            },
            state,
        )
    }

    fn set_flushed(&mut self, flushed: bool) {
        let rc = unsafe { uvrr_sb_set_flushed(self.handle, self.path.as_ptr(), flushed) };
        check(rc, "set_flushed");
    }

    fn bump(&mut self) {
        let rc = unsafe { uvrr_sb_bump(self.handle, self.path.as_ptr()) };
        check(rc, "bump");
    }
}

impl Drop for Superblock {
    fn drop(&mut self) {
        unsafe { uvrr_sb_close(self.handle) }
    }
}

fn main() {
    let dir = std::env::temp_dir().join("uvrr-reincarnation-demo");
    std::fs::create_dir_all(&dir).expect("create demo dir");
    let path_buf = dir.join("superblock.bin");
    std::fs::remove_file(&path_buf).ok();
    let path = CString::new(path_buf.to_str().expect("utf8 path")).expect("nul-free");

    println!("== format: four TB superblock copies, sequence=1, flushed");
    drop(Superblock::format(&path, 0xBEEF));

    println!(
        "== CLEAN PATH: startup reads all 4 copies; all flushed => clean => continue same identity"
    );
    let (mut sb, state) = Superblock::open(&path);
    println!("   state: {state:?}");
    assert!(!state.dirty, "freshly formatted store must be clean");
    assert!(state.flushed, "freshly formatted store must be flushed");
    assert_eq!(state.valid_copies, 0b1111, "all four copies must verify");
    let identity = state.adopted_sequence;
    println!("   continuation commitment: identity {identity} continues unchanged");

    println!(
        "== clean shutdown path: writes go unflushed in flight, then flush + fsync + mark flushed"
    );
    sb.set_flushed(false);
    sb.set_flushed(true); // stop responding -> flush -> fsync -> mark flushed in all four
    drop(sb);
    let (sb, state) = Superblock::open(&path);
    println!("   state: {state:?}");
    assert!(!state.dirty, "clean shutdown must reopen clean");
    assert_eq!(
        state.adopted_sequence, identity,
        "clean path keeps identity"
    );
    drop(sb);

    println!("== DIRTY PATH: crash with in-flight (unflushed) state");
    let (mut sb, state) = Superblock::open(&path);
    assert!(!state.dirty, "precondition");
    sb.set_flushed(false); // simulates the crash: last write left unflushed
    drop(sb);

    let (sb, state) = Superblock::open(&path);
    println!("   state: {state:?}");
    assert!(state.dirty, "any valid copy unflushed => dirty");
    let old_identity = state.adopted_sequence;
    assert_eq!(old_identity, identity);
    drop(sb);

    println!("== dirty => bump incarnation, write new identity + flushed to ALL FOUR");
    let (mut sb, state) = Superblock::open(&path);
    assert!(state.dirty);
    sb.bump();
    drop(sb);

    let (sb, state) = Superblock::open(&path);
    println!("   state: {state:?}");
    assert!(!state.dirty, "post-bump store must reopen clean");
    assert!(state.flushed, "post-bump copies carry the flushed bit");
    assert_eq!(state.valid_copies, 0b1111, "bump wrote all four copies");
    let new_identity = state.adopted_sequence;
    assert_eq!(new_identity, old_identity + 1, "incarnation bumped by one");
    println!("   REINCARNATION: identity {old_identity} -> {new_identity} (higher identity wins)");
    drop(sb);

    println!("== higher-identity-wins: tearing one copy cannot regress the adopted identity");
    tear_one_copy(&path_buf, unsafe { uvrr_sb_copy_size() });
    let (sb, state) = Superblock::open(&path);
    println!("   state: {state:?}");
    assert_eq!(state.adopted_sequence, new_identity, "stale copy must lose");
    assert_eq!(state.valid_copies, 0b1110, "3 of 4 valid after tear");
    drop(sb);

    println!(
        "DEMO OK: clean path continued identity {identity}; dirty path reincarnated {old_identity} -> {new_identity}"
    );
}

/// Overwrite copy 0 with the previous generation's content (simulates a lost
/// write), so the store holds copies [old, new, new, new].
fn tear_one_copy(path: &std::path::Path, copy_size: u64) {
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open superblock zone");
    let mut stale = vec![0u8; copy_size as usize];
    file.seek(SeekFrom::Start(copy_size)).expect("seek copy 1");
    file.read_exact(&mut stale).expect("read copy 1");
    file.seek(SeekFrom::Start(0)).expect("seek copy 0");
    file.write_all(&stale).expect("write stale copy 0");
    file.sync_all().expect("fsync");
}
