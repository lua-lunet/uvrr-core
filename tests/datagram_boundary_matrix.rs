//! Boundary matrix around the one-datagram ceiling for generated peer output
//! (item02 verification of the F01 transactional FFI fix, re-pinned for
//! core-owned chunked transfer).
//!
//! This matrix covers the two arms around the exact boundary end-to-end
//! through the public FFI output queue:
//!
//! - just-fits (largest log whose whole-log `DO_EPOCH_CHANGE` still fits): the
//!   step commits exactly once — the queued datagram is at most `MAX_DATAGRAM`,
//!   decodes to the exact expected message, and a repeat trigger produces no
//!   duplicate output. Small-log clusters keep this single-message wire
//!   behaviour.
//! - just-over (first oversize log): the transfer commits chunked — the queue
//!   gains an ordered run of `STATE_CHUNK` datagrams, each at most
//!   `MAX_DATAGRAM`, sharing one transfer uuid and reassembling to the exact
//!   whole-log `DO_EPOCH_CHANGE`. The commit is still exactly-once: a repeat
//!   trigger produces no duplicate transfer.
//!
//! The one-datagram ceiling used to be a liveness limit; it is now only the
//! chunk size. The `ffi.rs` cap remains as an unreachable backstop for state
//! transfer.

use std::ffi::c_void;
use std::ptr;

use uuid::Uuid;
use vrr::locks::{Lease, Request};
use vrr::vrr::{Body, ChunkKind, LogEntry, LogState, MAX_DATAGRAM, Message, Tag};

const OK: i32 = 0;

unsafe extern "C" {
    fn vrr_node_new(
        members_len: usize,
        members_data: *const u8,
        own_len: usize,
        own_data: *const u8,
        out: *mut *mut c_void,
    ) -> i32;
    fn vrr_node_free(node: *mut c_void);
    fn vrr_node_request(
        node: *mut c_void,
        execution_time_len: usize,
        execution_time: *const u8,
        json_len: usize,
        json: *const u8,
    ) -> i32;
    fn vrr_node_receive(node: *mut c_void, from: u32, len: usize, data: *const u8) -> i32;
    fn vrr_node_idle(node: *mut c_void) -> i32;
    fn vrr_node_leader_timeout(node: *mut c_void) -> i32;
    fn vrr_node_next(
        node: *mut c_void,
        out_kind: *mut u32,
        out_to: *mut u32,
        out_tag: *mut u32,
        out_epoch: *mut u32,
        out_slot_hi: *mut u32,
        out_slot_lo: *mut u32,
        capacity: usize,
        out_len: *mut usize,
        out_data: *mut u8,
    ) -> i32;
}

struct Node(*mut c_void);

impl Node {
    fn new() -> Self {
        let members = b"0:1\0"
            .iter()
            .chain(b"1:1\0")
            .chain(b"2:1")
            .copied()
            .collect::<Vec<_>>();
        let own = b"0:1";
        let mut node = ptr::null_mut();
        // Every pointer passed here is derived from a live Rust slice; `Node`
        // frees the successful handle on drop.
        let status = unsafe {
            vrr_node_new(
                members.len(),
                members.as_ptr(),
                own.len(),
                own.as_ptr(),
                &mut node,
            )
        };
        assert_eq!(status, OK);
        assert!(!node.is_null());
        Self(node)
    }

    fn request(&self, json: &[u8]) -> i32 {
        unsafe { vrr_node_request(self.0, 3, b"100".as_ptr(), json.len(), json.as_ptr()) }
    }

    fn receive(&self, from: u32, bytes: &[u8]) -> i32 {
        unsafe { vrr_node_receive(self.0, from, bytes.len(), bytes.as_ptr()) }
    }

    fn next(&self) -> Option<Output> {
        let mut output = Output::default();
        let mut bytes = vec![0; MAX_DATAGRAM];
        let status = unsafe {
            vrr_node_next(
                self.0,
                &mut output.kind,
                &mut output.to,
                &mut output.tag,
                &mut output.epoch,
                &mut output.slot_hi,
                &mut output.slot_lo,
                bytes.len(),
                &mut output.len,
                bytes.as_mut_ptr(),
            )
        };
        match status {
            0 => None,
            1 => {
                bytes.truncate(output.len);
                output.bytes = bytes;
                Some(output)
            }
            other => panic!("vrr_node_next failed with {other}"),
        }
    }

    fn drain(&self) -> Vec<Output> {
        std::iter::from_fn(|| self.next()).collect()
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        unsafe { vrr_node_free(self.0) };
    }
}

#[derive(Default)]
struct Output {
    kind: u32,
    to: u32,
    tag: u32,
    epoch: u32,
    slot_hi: u32,
    slot_lo: u32,
    len: usize,
    bytes: Vec<u8>,
}

impl Output {
    fn slot(&self) -> u64 {
        ((self.slot_hi as u64) << 32) | self.slot_lo as u64
    }
}

/// The exact client payload the core stores for request `slot`.
fn json(slot: u64) -> Vec<u8> {
    serde_json::to_vec(&Request::Set {
        message_id: Uuid::from_u128(slot as u128),
        client_id: 7,
        request_num: slot,
        lock_id: 11,
        lease: Lease {
            lease_id: 13,
            holder: Uuid::from_u128(17),
            expiry: 1000,
        },
    })
    .unwrap()
}

/// The exact log entry the core appends for request `slot`.
fn entry(slot: u64) -> LogEntry {
    LogEntry {
        slot,
        client_id: 7,
        request_num: slot,
        message_id: Uuid::from_u128(slot as u128),
        execution_time: 100,
        payload: json(slot),
    }
}

/// The exact whole-log `DO_EPOCH_CHANGE` this node generates in epoch 1 with a
/// log of `entries` requests: header slot = log length, `latest_normal` = 0,
/// `commit` = 0 (no PREPARE_OK quorum is ever delivered in this drive).
fn do_epoch_change(entries: u64) -> Message {
    Message {
        epoch: 1,
        slot: entries,
        body: Body::DoEpochChange {
            latest_normal: 0,
            state: LogState {
                slot: entries,
                commit: 0,
                log: (1..=entries).map(entry).collect(),
            },
        },
    }
}

/// The first log length whose generated `DO_EPOCH_CHANGE` exceeds one
/// datagram, asserting the boundary is exact: the previous length still fits.
fn first_oversize_log() -> u64 {
    let oversize = (1u64..)
        .find(|&entries| do_epoch_change(entries).encode().unwrap().len() > MAX_DATAGRAM)
        .expect("a finite whole-log datagram ceiling exists");
    assert!(
        do_epoch_change(oversize - 1).encode().unwrap().len() <= MAX_DATAGRAM,
        "boundary is not exact: {} entries must still fit",
        oversize - 1
    );
    oversize
}

fn start_epoch_change() -> Vec<u8> {
    let bytes = Message {
        epoch: 1,
        slot: 0,
        body: Body::StartEpochChange,
    }
    .encode()
    .unwrap();
    assert!(bytes.len() <= MAX_DATAGRAM);
    bytes
}

/// Node 0 leads epoch 0; `entries` requests queue that many PREPARE broadcasts,
/// then a leader timeout enters epoch 1 (node 1 leads) and queues one
/// START_EPOCH_CHANGE broadcast.
fn grow(node: &Node, entries: u64) {
    for slot in 1..=entries {
        assert_eq!(node.request(&json(slot)), OK, "request {slot}");
    }
    assert_eq!(unsafe { vrr_node_leader_timeout(node.0) }, OK);
}

/// Asserts the `expected`-th drained output is the queued START_EPOCH_CHANGE
/// broadcast for epoch 1 carrying the current log length in its header slot.
fn assert_start_epoch_change(output: &Output, slot: u64) {
    assert_eq!(output.kind, 1, "START_EPOCH_CHANGE is a broadcast");
    assert_eq!(output.tag, Tag::StartEpochChange as u32);
    assert_eq!(output.epoch, 1);
    assert_eq!(output.slot(), slot);
}

#[test]
fn just_fits_transfer_commits_exactly_once_within_one_datagram() {
    let fits = first_oversize_log() - 1;
    let node = Node::new();
    grow(&node, fits);

    assert_eq!(node.receive(1, &start_epoch_change()), OK);

    // The queue holds the ordinary outputs first, then the committed transfer:
    // `fits` PREPARE broadcasts, one START_EPOCH_CHANGE broadcast, one
    // DO_EPOCH_CHANGE to the epoch-1 leader.
    let drained = node.drain();
    assert_eq!(
        drained.len(),
        usize::try_from(fits).unwrap() + 2,
        "queue must hold every ordinary output plus the single transfer"
    );
    for (index, output) in drained[..drained.len() - 2].iter().enumerate() {
        let slot = u64::try_from(index).unwrap() + 1;
        assert_eq!(output.kind, 1, "PREPARE {slot} is a broadcast");
        assert_eq!(output.tag, Tag::Prepare as u32, "PREPARE {slot}");
        assert_eq!(output.epoch, 0, "PREPARE {slot}");
        assert_eq!(output.slot(), slot, "PREPARE order is preserved");
    }
    assert_start_epoch_change(&drained[drained.len() - 2], fits);

    let transfer = &drained[drained.len() - 1];
    assert_eq!(transfer.kind, 2, "DO_EPOCH_CHANGE goes to the leader only");
    assert_eq!(transfer.to, 1, "node 1 leads epoch 1");
    assert_eq!(transfer.tag, Tag::DoEpochChange as u32);
    assert_eq!(transfer.epoch, 1);
    assert_eq!(transfer.slot(), fits);
    assert!(
        transfer.bytes.len() <= MAX_DATAGRAM,
        "just-fits datagram is {} bytes (> {MAX_DATAGRAM})",
        transfer.bytes.len()
    );
    assert_eq!(
        Message::decode(&transfer.bytes),
        Some(do_epoch_change(fits)),
        "queued datagram must decode to the exact whole-log DO_EPOCH_CHANGE"
    );

    // The success committed exactly once: a second qualifying trigger adds no
    // duplicate transfer, and the node keeps serving.
    assert_eq!(node.receive(2, &start_epoch_change()), OK);
    assert!(
        node.next().is_none(),
        "a committed sent_do_change must not retransmit"
    );
    assert_eq!(unsafe { vrr_node_idle(node.0) }, OK);
}

#[test]
fn just_over_transfer_chunks_commit_exactly_once_within_datagrams() {
    let oversize = first_oversize_log();
    let node = Node::new();
    grow(&node, oversize);

    assert_eq!(
        node.receive(1, &start_epoch_change()),
        OK,
        "oversize whole-log DO_EPOCH_CHANGE must chunk, not refuse"
    );

    // The queue holds the ordinary outputs first, then the chunked transfer:
    // `oversize` PREPARE broadcasts, one START_EPOCH_CHANGE broadcast, then
    // the ordered run of STATE_CHUNK datagrams to the epoch-1 leader.
    let drained = node.drain();
    let ordinary = usize::try_from(oversize).unwrap() + 1;
    assert!(
        drained.len() > ordinary,
        "queue must hold the ordinary outputs plus the chunked transfer"
    );
    for (index, output) in drained[..ordinary - 1].iter().enumerate() {
        let slot = u64::try_from(index).unwrap() + 1;
        assert_eq!(output.kind, 1, "PREPARE {slot} is a broadcast");
        assert_eq!(output.tag, Tag::Prepare as u32, "PREPARE {slot}");
        assert_eq!(output.epoch, 0, "PREPARE {slot}");
        assert_eq!(output.slot(), slot, "PREPARE order is preserved");
    }
    assert_start_epoch_change(&drained[ordinary - 1], oversize);

    let chunks = &drained[ordinary..];
    assert!(
        chunks.len() > 1,
        "the oversize transfer must actually chunk; got {} chunk(s)",
        chunks.len()
    );
    let mut transfer = None;
    let mut reassembled: Vec<Option<LogEntry>> = (0..oversize).map(|_| None).collect();
    for chunk in chunks {
        assert_eq!(chunk.kind, 2, "STATE_CHUNK goes to the leader only");
        assert_eq!(chunk.to, 1, "node 1 leads epoch 1");
        assert_eq!(chunk.tag, Tag::StateChunk as u32);
        assert_eq!(chunk.epoch, 1);
        assert_eq!(
            chunk.slot(),
            oversize,
            "header carries the logical message slot"
        );
        assert!(
            chunk.bytes.len() <= MAX_DATAGRAM,
            "chunk datagram is {} bytes (> {MAX_DATAGRAM})",
            chunk.bytes.len()
        );
        let message = Message::decode(&chunk.bytes).expect("chunk datagram decodes");
        let Body::StateChunk {
            transfer: uuid,
            kind,
            total,
            first,
            entries,
            state_slot,
            state_commit,
        } = message.body
        else {
            panic!("transfer datagrams are STATE_CHUNK; got {:?}", message.body)
        };
        assert_eq!(kind, ChunkKind::DoEpochChange { latest_normal: 0 });
        assert_eq!(total, oversize);
        assert_eq!(state_slot, oversize);
        assert_eq!(state_commit, 0);
        match transfer {
            None => transfer = Some(uuid),
            Some(ongoing) => assert_eq!(ongoing, uuid, "one transfer uuid per logical message"),
        }
        for (index, entry) in entries.into_iter().enumerate() {
            let at = usize::try_from(first + index as u64).unwrap();
            assert!(
                reassembled[at].replace(entry).is_none(),
                "chunks must not overlap"
            );
        }
    }
    let log: Vec<LogEntry> = reassembled
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .expect("chunks contiguously cover the whole log");
    let Body::DoEpochChange { state, .. } = do_epoch_change(oversize).body else {
        unreachable!()
    };
    assert_eq!(
        log, state.log,
        "chunks reassemble to the exact whole-log DO_EPOCH_CHANGE log"
    );

    // The success committed exactly once: a second qualifying trigger adds no
    // duplicate transfer, and the node keeps serving.
    assert_eq!(node.receive(2, &start_epoch_change()), OK);
    assert!(
        node.next().is_none(),
        "a committed sent_do_change must not retransmit"
    );
    assert_eq!(unsafe { vrr_node_idle(node.0) }, OK);
}
