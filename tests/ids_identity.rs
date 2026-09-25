//! The identity types carry the law (the boot gate's host requirements): a
//! node identity is the durable pair of the sysadmin-assigned system
//! identifier and the crash bump counter, one-indexed in both halves, never
//! read as zero, packed into the u32 `NodeId` that is the wire, disk, and
//! C-ABI form. These tests pin the construction: zero is unrepresentable,
//! the packing is injective inside the u16 halves, the print form names the
//! host identity left-padded to full width, and the types cost nothing.

use vrr::ids::{CrashCounter, NodeId, SystemId};

fn sys(v: u16) -> SystemId {
    SystemId::new(v).expect("a non-zero system identifier is lawful")
}

fn crash(v: u16) -> CrashCounter {
    CrashCounter::new(v).expect("a non-zero crash counter is lawful")
}

#[test]
fn zero_is_unrepresentable() {
    assert!(SystemId::new(0).is_none());
    assert!(CrashCounter::new(0).is_none());
}

#[test]
fn the_pair_packs_msb_system_lsb_counter() {
    let id = NodeId::new(sys(21), crash(112));
    assert_eq!(u32::from(id), (21u32 << 16) | 112u32);
}

#[test]
fn the_pair_unpacks_what_it_packed() {
    let id = NodeId::new(sys(7), crash(65535));
    assert_eq!(id.system_id(), Some(sys(7)));
    assert_eq!(id.crash_counter(), Some(crash(65535)));
    assert_eq!(u32::from(id), (7u32 << 16) | 65535u32);
}

#[test]
fn a_zero_half_is_no_identity() {
    // Raw bit patterns whose system half is zero never name a system.
    assert_eq!(NodeId::from(3u32).system_id(), None);
    assert_eq!(NodeId::from(0u32).system_id(), None);
    assert_eq!(NodeId::from(0u32).crash_counter(), None);
    // The lawful constructor's output always decodes.
    let id = NodeId::new(sys(1), crash(1));
    assert!(id.system_id().is_some() && id.crash_counter().is_some());
    assert!(id.is_lawful());
    assert!(!NodeId::from(3u32).is_lawful());
}

#[test]
fn the_counter_advances_one_life_at_a_time() {
    let first = NodeId::new(sys(4), crash(1));
    let second = first.next_life().expect("a fresh counter is available");
    assert_eq!(second.system_id(), first.system_id());
    assert_eq!(second.crash_counter(), Some(crash(2)));
    assert!(second > first);
    // The counter space is u16: the 65535th life is the last the packing holds.
    let last = NodeId::new(sys(4), crash(65535));
    assert!(last.next_life().is_none());
}

#[test]
fn the_print_form_names_the_pair_left_padded_to_full_width() {
    assert_eq!(NodeId::new(sys(21), crash(112)).to_string(), "0002100112");
    assert_eq!(NodeId::new(sys(1), crash(1)).to_string(), "0000100001");
    assert_eq!(
        NodeId::new(sys(65535), crash(65535)).to_string(),
        "6553565535"
    );
    // A raw pattern with a zero half prints its bits honestly.
    assert_eq!(NodeId::from(3u32).to_string(), "0000000003");
}

#[test]
fn the_types_cost_nothing() {
    use core::mem::{align_of, size_of};
    assert_eq!(size_of::<SystemId>(), size_of::<u16>());
    assert_eq!(size_of::<CrashCounter>(), size_of::<u16>());
    assert_eq!(align_of::<SystemId>(), align_of::<u16>());
    assert_eq!(align_of::<CrashCounter>(), align_of::<u16>());
    // The pair is never wider than the wire form on the boundary.
    assert_eq!(size_of::<NodeId>(), size_of::<u32>());
}

#[test]
fn the_hostid_tool_initialises_prints_and_bumps() {
    let tool = env!("CARGO_BIN_EXE_uvrr-hostid");
    let out = std::process::Command::new(tool)
        .args(["init", "21", "112"])
        .output()
        .expect("the tool runs");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("0002100112"),
        "init prints the padded pair: {text}"
    );
    let packed = ((21u32 << 16) | 112u32).to_string();
    assert!(text.contains(&packed), "init prints the packed u32: {text}");

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_uvrr-hostid"))
        .args(["print", &packed])
        .output()
        .expect("the tool runs");
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("0002100112")
    );

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_uvrr-hostid"))
        .args(["bump", &packed])
        .output()
        .expect("the tool runs");
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("0002100113")
    );

    // Zero is refused, not defaulted.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_uvrr-hostid"))
        .args(["init", "0", "1"])
        .output()
        .expect("the tool runs");
    assert!(!out.status.success());
}
