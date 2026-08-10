//! Regression for the whole-log datagram-poisoning boundary defect (review triage T0,
//! consolidated finding F01, P0), re-pinned for core-owned chunked transfer.
//!
//! A public-path drive grows the log of an FFI node until the whole-log
//! `DO_EPOCH_CHANGE` generated during epoch change encodes just beyond
//! `MAX_DATAGRAM`. Originally `Node::peer` rejected the datagram only after
//! `Replica::step` had applied the core mutations, and `Node::step` then
//! poisoned the node permanently; the F01 fix made that refusal atomic.
//!
//! The ceiling is now removed rather than refused: the core splits an
//! oversize logical transfer into an ordered run of `STATE_CHUNK` datagrams,
//! each at most `MAX_DATAGRAM`, so the qualifying `START_EPOCH_CHANGE` receive
//! returns `OK` and the transfer commits through the public output queue.
//! This test pins that contract end-to-end through the FFI: every queued
//! datagram fits one datagram, the chunks reassemble to the exact whole-log
//! `DO_EPOCH_CHANGE`, and the node is never poisoned.
//!
//! `START_EPOCH` and `RECOVERY_RESPONSE` share the same chunking path
//! (`Replica::state_transfer`); driving one path pins the shared mechanism.

use std::ffi::c_void;

use uuid::Uuid;
use vrr::locks::{Lease, Request};
use vrr::vrr::{Body, ChunkKind, LogEntry, LogState, MAX_DATAGRAM, Message};

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
    fn vrr_node_recover(node: *mut c_void, nonce_len: usize, nonce: *const u8) -> i32;
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
    fn next(&self) -> Option<(u32, u32, Vec<u8>)> {
        let mut kind = 0;
        let mut to = 0;
        let mut tag = 0;
        let mut epoch = 0;
        let mut slot_hi = 0;
        let mut slot_lo = 0;
        let mut len = 0;
        let mut bytes = vec![0; MAX_DATAGRAM];
        let status = unsafe {
            vrr_node_next(
                self.0,
                &mut kind,
                &mut to,
                &mut tag,
                &mut epoch,
                &mut slot_hi,
                &mut slot_lo,
                bytes.len(),
                &mut len,
                bytes.as_mut_ptr(),
            )
        };
        match status {
            0 => None,
            1 => {
                bytes.truncate(len);
                Some((kind, to, bytes))
            }
            other => panic!("vrr_node_next failed with {other}"),
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        unsafe { vrr_node_free(self.0) };
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

/// Encoded size of the `DO_EPOCH_CHANGE` this node generates in epoch 1 with a
/// log of `entries` requests: header slot = log length, `latest_normal` = 0,
/// `commit` = 0 (no PREPARE_OK quorum is ever delivered in this drive).
fn do_epoch_change_datagram(entries: u64) -> usize {
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
    .encode()
    .unwrap()
    .len()
}

#[test]
fn oversize_whole_log_transfer_is_chunked_without_poisoning_the_node() {
    // Pin the boundary deterministically: the first log length whose generated
    // DO_EPOCH_CHANGE exceeds MAX_DATAGRAM, with the previous length still fitting.
    let oversize = (1u64..)
        .find(|&entries| do_epoch_change_datagram(entries) > MAX_DATAGRAM)
        .expect("a finite whole-log datagram ceiling exists");
    assert!(
        do_epoch_change_datagram(oversize - 1) <= MAX_DATAGRAM,
        "boundary is not exact: {} entries must still fit",
        oversize - 1
    );
    eprintln!(
        "whole-log DO_EPOCH_CHANGE boundary: {oversize} entries encode to {} bytes \
         (> {MAX_DATAGRAM}); {} entries encode to {} bytes",
        do_epoch_change_datagram(oversize),
        oversize - 1,
        do_epoch_change_datagram(oversize - 1),
    );

    // Node 0 of three members leads epoch 0; peers are 1 and 2.
    let members = b"0:1\0"
        .iter()
        .chain(b"1:1\0")
        .chain(b"2:1")
        .copied()
        .collect::<Vec<_>>();
    let own = b"0:1";
    let mut raw = std::ptr::null_mut();
    assert_eq!(
        unsafe {
            vrr_node_new(
                members.len(),
                members.as_ptr(),
                own.len(),
                own.as_ptr(),
                &mut raw,
            )
        },
        OK
    );
    assert!(!raw.is_null());
    let node = Node(raw);

    // Grow the log to the first oversize length through the public request path.
    for slot in 1..=oversize {
        let json = json(slot);
        assert_eq!(
            unsafe { vrr_node_request(node.0, 3, b"100".as_ptr(), json.len(), json.as_ptr()) },
            OK,
            "request {slot}"
        );
    }

    // Leader timeout moves the node into epoch 1, where node 1 leads and this
    // node is a backup. One peer START_EPOCH_CHANGE qualifies the whole-log
    // DO_EPOCH_CHANGE, which the core now chunks instead of refusing.
    assert_eq!(unsafe { vrr_node_leader_timeout(node.0) }, OK);
    let start_epoch_change = Message {
        epoch: 1,
        slot: 0,
        body: Body::StartEpochChange,
    }
    .encode()
    .unwrap();
    assert!(start_epoch_change.len() <= MAX_DATAGRAM);
    assert_eq!(
        unsafe {
            vrr_node_receive(
                node.0,
                1,
                start_epoch_change.len(),
                start_epoch_change.as_ptr(),
            )
        },
        OK,
        "oversize whole-log DO_EPOCH_CHANGE must chunk, not refuse"
    );

    // Drain the queue: the ordinary outputs (one PREPARE broadcast per
    // request, one START_EPOCH_CHANGE broadcast) then the chunked transfer —
    // STATE_CHUNK datagrams to the epoch-1 leader, each within one datagram,
    // sharing one transfer uuid and reassembling to the exact whole log.
    let mut transfer = None;
    let mut reassembled: Vec<Option<LogEntry>> = (0..oversize).map(|_| None).collect();
    let mut chunks = 0_u64;
    let mut ordinary = 0_u64;
    while let Some((kind, to, bytes)) = node.next() {
        assert!(
            bytes.len() <= MAX_DATAGRAM,
            "every queued datagram fits one datagram"
        );
        let message = Message::decode(&bytes).expect("queued datagram decodes");
        if let Body::StateChunk {
            transfer: uuid,
            kind: chunk_kind,
            total,
            first,
            entries,
            state_slot,
            state_commit,
        } = message.body
        {
            assert_eq!(kind, 2, "STATE_CHUNK goes to the leader only");
            assert_eq!(to, 1, "node 1 leads epoch 1");
            assert_eq!(message.epoch, 1);
            assert_eq!(message.slot, oversize, "header carries the logical slot");
            assert_eq!(chunk_kind, ChunkKind::DoEpochChange { latest_normal: 0 });
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
                    "no overlapping chunks"
                );
            }
            chunks += 1;
        } else {
            ordinary += 1;
        }
    }
    assert_eq!(
        ordinary,
        oversize + 1,
        "prepares plus the start-epoch-change"
    );
    assert!(chunks > 1, "the oversize transfer must actually chunk");
    let transfer = transfer.expect("a chunked transfer was emitted");
    assert_ne!(transfer, Uuid::nil(), "transfer identity is a fresh uuid");
    let log: Vec<LogEntry> = reassembled.into_iter().map(Option::unwrap).collect();
    assert_eq!(
        log,
        (1..=oversize).map(entry).collect::<Vec<_>>(),
        "chunks reassemble to the exact whole log"
    );

    // The node was never poisoned: public entry points keep serving.
    assert_eq!(
        unsafe { vrr_node_idle(node.0) },
        OK,
        "node must keep serving after a chunked transfer"
    );
    assert_eq!(
        unsafe { vrr_node_recover(node.0, 1, b"9".as_ptr()) },
        OK,
        "node must accept recovery after a chunked transfer"
    );
}
