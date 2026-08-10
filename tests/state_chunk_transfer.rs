//! Core-owned multi-datagram state transfer: the `STATE_CHUNK` reassembly
//! machine and its end-to-end effect on epoch change and recovery.
//!
//! A logical state message (`DO_EPOCH_CHANGE` / `START_EPOCH` /
//! `RECOVERY_RESPONSE`) whose encoding exceeds one datagram is split at the
//! sender into bounded chunks and reassembled at the receiver, which then
//! re-enters the ordinary receive arm. These tests pin:
//!
//! - reassembly semantics: out-of-order delivery completes, duplicates are
//!   idempotent, conflicting overlaps and conflicting invariant fields
//!   abandon the transfer leaving zero residue, a claimed `total` above the
//!   configured maximum is rejected before any buffering, and a fresh
//!   transfer uuid supersedes a stalled one;
//! - bounded memory: buffered bytes are accounted per received chunk and a
//!   transfer that would cross the per-buffer cap is abandoned;
//! - expiry without a clock: a stalled transfer is dropped after
//!   `CHUNK_TRANSFER_IDLE_LIMIT` idle ticks, and progress resets the counter;
//! - observational equality: chunked delivery ends in a `Diagnostic`
//!   identical to single-datagram delivery, so no election, quorum, epoch or
//!   dedup invariant is weakened;
//! - end to end: a replica whose required transfer exceeds one datagram
//!   completes both epoch change and recovery and reaches `Status::Normal`;
//! - the FFI validation gate: an invalid service payload inside a chunk is
//!   rejected exactly as inside a whole state message.

use std::ffi::c_void;

use uuid::Uuid;
use vrr::locks::{Lease, Request};
use vrr::vrr::{
    Body, CHUNK_TRANSFER_IDLE_LIMIT, ChunkKind, Input, LogEntry, LogState,
    MAX_CHUNK_TRANSFER_TOTAL, MAX_DATAGRAM, Message, Output, Replica, Status, Tag,
};

mod support;
use support::{MEMBER_COUNT, ReplicaSnapshot, entry, id, message, node, receive, state};

/// A replica in `EpochChange` for epoch 1 that leads that epoch, so a
/// `DO_EPOCH_CHANGE` from node 0 is addressed to it.
fn leader_elect() -> Replica {
    let mut replica = node(MEMBER_COUNT, 1);
    replica.step(Input::LeaderTimeout);
    assert_eq!(replica.status(), Status::EpochChange);
    assert!(replica.is_leader());
    replica
}

/// A replica in `EpochChange` for epoch 1 that does not lead it, so a
/// `START_EPOCH` from node 1 is addressed to it.
fn epoch_change_backup() -> Replica {
    let mut replica = node(MEMBER_COUNT, 2);
    replica.step(Input::LeaderTimeout);
    assert_eq!(replica.status(), Status::EpochChange);
    replica
}

/// A replica in `Recovering` with `nonce`; the leader of epoch 0 is node 0.
fn recovering(nonce: u64) -> Replica {
    let mut replica = node(MEMBER_COUNT, 2);
    replica.step(Input::Recover { nonce });
    assert_eq!(replica.status(), Status::Recovering);
    replica
}

/// A structurally valid log of `n` entries (slots 1..=n, unique message ids,
/// request numbers increasing per client).
fn log(n: u64) -> Vec<LogEntry> {
    (1..=n).map(|slot| entry(slot, 23, 100 + slot)).collect()
}

#[allow(clippy::too_many_arguments)]
fn chunk(
    epoch: u32,
    slot: u64,
    transfer: Uuid,
    kind: ChunkKind,
    total: u64,
    first: u64,
    entries: Vec<LogEntry>,
    state_commit: u64,
) -> Message {
    message(
        epoch,
        slot,
        Body::StateChunk {
            transfer,
            kind,
            total,
            first,
            entries,
            state_slot: total,
            state_commit,
        },
    )
}

/// Slices `entries` into `width`-sized chunks of a notional transfer.
fn chunks(
    epoch: u32,
    slot: u64,
    transfer: Uuid,
    kind: ChunkKind,
    entries: &[LogEntry],
    width: usize,
    commit: u64,
) -> Vec<Message> {
    let total = entries.len() as u64;
    entries
        .chunks(width)
        .enumerate()
        .map(|(index, slice)| {
            chunk(
                epoch,
                slot,
                transfer,
                kind,
                total,
                (index * width) as u64,
                slice.to_vec(),
                commit,
            )
        })
        .collect()
}

fn transfers(replica: &Replica) -> Vec<vrr::vrr::ChunkTransferDiagnostic> {
    replica.diagnostic().transfers
}

#[test]
fn out_of_order_chunks_complete_into_the_exact_logical_message() {
    let entries = log(6);
    let state = state(entries.clone(), 0);
    let transfer = id(1);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };
    let parts = chunks(1, 6, transfer, kind, &entries, 2, 0);

    let mut chunked = leader_elect();
    // Deliver the middle, then the end: buffered, nothing adopted, no outputs.
    assert!(receive(&mut chunked, 0, parts[1].clone()).is_empty());
    assert!(receive(&mut chunked, 0, parts[2].clone()).is_empty());
    let in_flight = transfers(&chunked);
    assert_eq!(in_flight.len(), 1);
    assert_eq!(in_flight[0].transfer, transfer);
    assert_eq!(in_flight[0].total, 6);
    assert_eq!(in_flight[0].received, 4);
    assert!(in_flight[0].buffered_bytes > 0);
    assert_eq!(in_flight[0].idle_ticks, 0);
    assert!(chunked.diagnostic().do_changes.is_empty());

    // The head chunk completes contiguous coverage: the logical message
    // re-enters and lands in the do-change evidence.
    assert!(receive(&mut chunked, 0, parts[0].clone()).is_empty());
    assert!(
        transfers(&chunked).is_empty(),
        "completion frees the buffer"
    );

    let mut whole = leader_elect();
    let whole_outputs = receive(
        &mut whole,
        0,
        message(
            1,
            6,
            Body::DoEpochChange {
                latest_normal: 0,
                state,
            },
        ),
    );
    assert_eq!(
        chunked.diagnostic(),
        whole.diagnostic(),
        "chunked delivery is observationally identical to one datagram"
    );
    assert!(whole_outputs.is_empty());
}

#[test]
fn duplicate_chunks_are_idempotent() {
    let entries = log(4);
    let transfer = id(2);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };
    let parts = chunks(1, 4, transfer, kind, &entries, 2, 0);

    let mut replica = leader_elect();
    receive(&mut replica, 0, parts[0].clone());
    let after_first = ReplicaSnapshot::capture(&replica);
    // The same chunk again: no double-counting, no other mutation.
    assert!(receive(&mut replica, 0, parts[0].clone()).is_empty());
    let after_duplicate = ReplicaSnapshot::capture(&replica);
    assert_eq!(after_first, after_duplicate);
    assert_eq!(transfers(&replica)[0].received, 2);

    assert!(receive(&mut replica, 0, parts[1].clone()).is_empty());
    assert!(transfers(&replica).is_empty());
    assert_eq!(replica.diagnostic().do_changes.len(), 1);

    // A chunk for an already-completed transfer starts a fresh (stalled)
    // buffer rather than corrupting the adopted evidence; clean it up via
    // supersession by the same transfer restart path.
    assert!(receive(&mut replica, 0, parts[0].clone()).is_empty());
    assert_eq!(transfers(&replica).len(), 1);
}

#[test]
fn conflicting_overlap_abandons_the_transfer_without_residue() {
    let entries = log(4);
    let transfer = id(3);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };

    let mut replica = leader_elect();
    let baseline = ReplicaSnapshot::capture(&replica);

    let first = chunk(1, 4, transfer, kind, 4, 0, entries[..2].to_vec(), 0);
    assert!(receive(&mut replica, 0, first).is_empty());
    assert_eq!(transfers(&replica).len(), 1);

    // A chunk covering index 1 with a *different* entry conflicts.
    let mut altered = entries[1].clone();
    altered.payload = b"altered".to_vec();
    let conflicting = chunk(
        1,
        4,
        transfer,
        kind,
        4,
        1,
        vec![altered, entries[2].clone()],
        0,
    );
    assert!(receive(&mut replica, 0, conflicting).is_empty());

    assert!(
        transfers(&replica).is_empty(),
        "a conflicting overlap frees the buffer"
    );
    assert_eq!(
        ReplicaSnapshot::capture(&replica),
        baseline,
        "abandonment leaves zero residue"
    );
    assert!(replica.diagnostic().do_changes.is_empty());
}

#[test]
fn conflicting_invariant_fields_abandon_the_transfer() {
    let entries = log(4);
    let transfer = id(4);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };

    let mut replica = leader_elect();
    let baseline = ReplicaSnapshot::capture(&replica);

    let first = chunk(1, 4, transfer, kind, 4, 0, entries[..2].to_vec(), 0);
    receive(&mut replica, 0, first);
    assert_eq!(transfers(&replica).len(), 1);

    // Same transfer uuid, but the repeated `state_commit` disagrees.
    let drifted = chunk(1, 4, transfer, kind, 4, 2, entries[2..].to_vec(), 1);
    assert!(receive(&mut replica, 0, drifted).is_empty());
    assert!(transfers(&replica).is_empty());
    assert_eq!(ReplicaSnapshot::capture(&replica), baseline);
}

#[test]
fn claimed_total_above_the_configured_maximum_is_rejected() {
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };
    let mut replica = leader_elect();
    let baseline = ReplicaSnapshot::capture(&replica);

    let total = MAX_CHUNK_TRANSFER_TOTAL + 1;
    let too_big = chunk(1, total, id(5), kind, total, 0, log(1), 0);
    assert!(receive(&mut replica, 0, too_big).is_empty());
    assert!(
        transfers(&replica).is_empty(),
        "an oversize claim never allocates a buffer"
    );
    assert_eq!(ReplicaSnapshot::capture(&replica), baseline);

    // A claim inconsistent with the repeated state slot is rejected too.
    let entries = log(2);
    let inconsistent = message(
        1,
        2,
        Body::StateChunk {
            transfer: id(6),
            kind,
            total: 2,
            first: 0,
            entries,
            state_slot: 3,
            state_commit: 0,
        },
    );
    assert!(receive(&mut replica, 0, inconsistent).is_empty());
    assert!(transfers(&replica).is_empty());
    assert_eq!(ReplicaSnapshot::capture(&replica), baseline);
}

#[test]
fn a_fresh_transfer_uuid_supersedes_the_stalled_transfer() {
    let entries = log(4);
    let state = state(entries.clone(), 0);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };

    let mut replica = leader_elect();
    let stalled = chunks(1, 4, id(7), kind, &entries, 2, 0);
    receive(&mut replica, 0, stalled[0].clone());
    assert_eq!(transfers(&replica)[0].transfer, id(7));

    // The recovering peer's retry restarts the transfer under a new uuid:
    // the old buffer is freed, the new one takes over, and it completes.
    let retry = chunks(1, 4, id(8), kind, &entries, 2, 0);
    receive(&mut replica, 0, retry[0].clone());
    let in_flight = transfers(&replica);
    assert_eq!(in_flight.len(), 1, "supersession frees the old buffer");
    assert_eq!(in_flight[0].transfer, id(8));
    assert_eq!(in_flight[0].received, 2);

    assert!(receive(&mut replica, 0, retry[1].clone()).is_empty());
    assert!(transfers(&replica).is_empty());
    assert_eq!(
        replica.diagnostic().do_changes[&0],
        (0, state),
        "the superseding transfer completes"
    );
}

#[test]
fn idle_ticks_expire_a_stalled_transfer_and_progress_resets() {
    let entries = log(6);
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };
    let parts = chunks(1, 6, id(9), kind, &entries, 2, 0);

    let mut replica = leader_elect();
    receive(&mut replica, 0, parts[0].clone());
    for tick in 1..4 {
        replica.step(Input::Idle);
        assert_eq!(transfers(&replica)[0].idle_ticks, tick);
    }
    // A fresh chunk is progress: the idle counter resets.
    receive(&mut replica, 0, parts[1].clone());
    assert_eq!(transfers(&replica)[0].idle_ticks, 0);
    assert_eq!(transfers(&replica)[0].received, 4);

    // A genuinely stalled transfer is dropped exactly at the limit.
    for _ in 1..CHUNK_TRANSFER_IDLE_LIMIT {
        replica.step(Input::Idle);
        assert_eq!(transfers(&replica).len(), 1, "dropped only at the limit");
    }
    replica.step(Input::Idle);
    assert!(
        transfers(&replica).is_empty(),
        "stalled transfer expires without a clock"
    );
    assert!(replica.diagnostic().do_changes.is_empty());
}

#[test]
fn buffered_bytes_are_capped_and_abandonment_frees_everything() {
    let kind = ChunkKind::DoEpochChange { latest_normal: 0 };
    let mut replica = leader_elect();
    let baseline = ReplicaSnapshot::capture(&replica);

    // One ~10 KB payload per chunk encodes to ~40 KB of JSON per datagram;
    // the per-buffer cap must trip within a few hundred chunks even though
    // the claimed total is legitimate.
    let total = MAX_CHUNK_TRANSFER_TOTAL;
    let mut delivered = 0_u64;
    loop {
        let first = delivered;
        let mut fat = entry(first + 1, 23, 100 + first + 1);
        fat.payload = vec![b'x'; 10_000];
        let part = chunk(1, total, id(10), kind, total, first, vec![fat], 0);
        assert!(receive(&mut replica, 0, part).is_empty());
        delivered += 1;
        if transfers(&replica).is_empty() {
            break;
        }
        assert!(
            delivered < total,
            "the per-buffer byte cap must trip before the claimed total completes"
        );
    }
    assert!(
        delivered > 1,
        "the buffer grew with received chunks before the cap tripped"
    );
    assert_eq!(
        ReplicaSnapshot::capture(&replica),
        baseline,
        "crossing the cap abandons the transfer and frees its memory"
    );
}

#[test]
fn start_epoch_chunked_delivery_matches_single_datagram_delivery() {
    let entries = log(6);
    let state = state(entries.clone(), 0);
    let transfer = id(11);
    let parts = chunks(1, 6, transfer, ChunkKind::StartEpoch, &entries, 2, 0);

    let mut chunked = epoch_change_backup();
    assert!(receive(&mut chunked, 1, parts[2].clone()).is_empty());
    assert!(receive(&mut chunked, 1, parts[0].clone()).is_empty());
    assert_eq!(chunked.status(), Status::EpochChange);
    let chunked_outputs = receive(&mut chunked, 1, parts[1].clone());

    let mut whole = epoch_change_backup();
    let whole_outputs = receive(&mut whole, 1, message(1, 6, Body::StartEpoch { state }));

    assert_eq!(chunked.status(), Status::Normal);
    assert_eq!(chunked.diagnostic(), whole.diagnostic());
    assert_eq!(
        chunked_outputs, whole_outputs,
        "completion emits exactly the whole-message outputs"
    );
}

#[test]
fn recovery_response_chunked_delivery_matches_single_datagram_delivery() {
    let entries = log(6);
    let state = state(entries.clone(), 0);
    let nonce = 9;
    let transfer = id(12);
    let kind = ChunkKind::RecoveryResponse { nonce };
    let parts = chunks(0, 6, transfer, kind, &entries, 2, 0);

    let mut chunked = recovering(nonce);
    assert!(receive(&mut chunked, 0, parts[1].clone()).is_empty());
    assert!(receive(&mut chunked, 0, parts[0].clone()).is_empty());
    assert!(receive(&mut chunked, 0, parts[2].clone()).is_empty());

    let mut whole = recovering(nonce);
    receive(
        &mut whole,
        0,
        message(
            0,
            6,
            Body::RecoveryResponse {
                nonce,
                state: Some(state),
            },
        ),
    );

    assert_eq!(
        chunked.diagnostic().recovery,
        whole.diagnostic().recovery,
        "reassembled evidence equals whole-message evidence"
    );
    assert_eq!(chunked.diagnostic(), whole.diagnostic());
}

/// A state transfer larger than one datagram, built through the public
/// request path of a real leader replica.
fn oversize_leader() -> (Replica, LogState) {
    let mut leader = node(3, 0);
    for slot in 1..=60_u64 {
        leader.step(Input::Request {
            client_id: 7,
            request_num: slot,
            message_id: id(slot as u8),
            execution_time: 100,
            payload: vec![b'x'; 500],
        });
    }
    let state = LogState {
        slot: 60,
        commit: 0,
        log: leader.log().to_vec(),
    };
    let whole = Message {
        epoch: 0,
        slot: 60,
        body: Body::RecoveryResponse {
            nonce: 9,
            state: Some(state.clone()),
        },
    };
    assert!(
        whole.encode().unwrap().len() > MAX_DATAGRAM,
        "the drive must exceed one datagram"
    );
    (leader, state)
}

fn assert_all_chunks_fit(outputs: &[Output]) -> Vec<Message> {
    let mut messages = Vec::new();
    for output in outputs {
        let (Output::To(_, message) | Output::Broadcast(message)) = output else {
            continue;
        };
        assert!(
            message.encode().unwrap().len() <= MAX_DATAGRAM,
            "every emitted datagram fits one datagram"
        );
        if matches!(message.body, Body::StateChunk { .. }) {
            messages.push(message.clone());
        }
    }
    assert!(
        messages.len() > 1,
        "the oversize transfer must actually chunk"
    );
    messages
}

#[test]
fn recovery_past_the_datagram_ceiling_reaches_normal() {
    let (mut leader, state) = oversize_leader();
    let nonce = 9;

    let mut backup = node(3, 1);
    let mut recovered = node(3, 2);
    let recovery = match recovered.step(Input::Recover { nonce })[0].clone() {
        Output::Broadcast(message) => message,
        other => panic!("recovery starts with a broadcast; got {other:?}"),
    };

    // The leader answers with a chunked transfer; the backup answers whole
    // with no state.
    let leader_outputs = receive(&mut leader, 2, recovery.clone());
    let parts = assert_all_chunks_fit(&leader_outputs);
    let backup_outputs = receive(&mut backup, 2, recovery);
    let [Output::To(2, backup_response)] = backup_outputs.as_slice() else {
        panic!("backup answers with one whole response; got {backup_outputs:?}")
    };
    assert!(matches!(
        backup_response.body,
        Body::RecoveryResponse { state: None, .. }
    ));

    // Quorum evidence: the backup's whole response, then the leader's chunks
    // delivered out of order.
    receive(&mut recovered, 1, backup_response.clone());
    assert_eq!(recovered.status(), Status::Recovering);
    for part in parts.iter().rev() {
        receive(&mut recovered, 0, part.clone());
    }

    assert_eq!(
        recovered.status(),
        Status::Normal,
        "a replica past the one-datagram ceiling completes recovery"
    );
    assert_eq!(recovered.log(), state.log.as_slice());
    assert!(transfers(&recovered).is_empty());

    // The same drive delivered as one datagram ends in the identical state.
    let mut twin = node(3, 2);
    twin.step(Input::Recover { nonce });
    receive(&mut twin, 1, backup_response.clone());
    receive(
        &mut twin,
        0,
        message(
            0,
            state.slot,
            Body::RecoveryResponse {
                nonce,
                state: Some(state),
            },
        ),
    );
    assert_eq!(
        recovered.diagnostic(),
        twin.diagnostic(),
        "chunked recovery is observationally identical to one datagram"
    );
}

#[test]
fn epoch_change_past_the_datagram_ceiling_reaches_normal() {
    let (mut node0, state) = oversize_leader();
    let mut node1 = node(3, 1);
    let mut node2 = node(3, 2);

    // Everyone times out into epoch 1, led by node 1.
    let timeout0 = node0.step(Input::LeaderTimeout);
    let [Output::Broadcast(sec0)] = timeout0.as_slice() else {
        panic!("leader timeout broadcasts one start-epoch-change")
    };
    let timeout1 = node1.step(Input::LeaderTimeout);
    let [Output::Broadcast(sec1)] = timeout1.as_slice() else {
        panic!("leader timeout broadcasts one start-epoch-change")
    };
    node2.step(Input::LeaderTimeout);

    // Node 0 qualifies and ships its oversize log to the leader-elect as
    // chunks; node 1 qualifies on node 0's vote.
    let do_change_parts = assert_all_chunks_fit(&receive(&mut node0, 1, sec1.clone()));
    assert!(receive(&mut node1, 0, sec0.clone()).is_empty());

    // Out-of-order delivery completes the do-change; the leader-elect adopts
    // the best log and broadcasts the start-epoch as chunks.
    for part in do_change_parts.iter().skip(1).rev() {
        assert!(receive(&mut node1, 0, part.clone()).is_empty());
    }
    let start_epoch_parts =
        assert_all_chunks_fit(&receive(&mut node1, 0, do_change_parts[0].clone()));
    assert_eq!(node1.status(), Status::Normal);
    assert_eq!(node1.epoch(), 1);
    assert_eq!(node1.log(), state.log.as_slice());

    // A backup reassembles the start-epoch and activates with the same log.
    for part in start_epoch_parts
        .iter()
        .skip(1)
        .chain(start_epoch_parts.iter().take(1))
    {
        receive(&mut node2, 1, part.clone());
    }
    assert_eq!(node2.status(), Status::Normal);
    assert_eq!(node2.epoch(), 1);
    assert_eq!(node2.log(), state.log.as_slice());
    assert!(transfers(&node2).is_empty());
}

#[test]
fn a_small_log_transfer_stays_a_single_message() {
    let mut leader = node(3, 0);
    leader.step(Input::Request {
        client_id: 7,
        request_num: 1,
        message_id: id(1),
        execution_time: 100,
        payload: b"small".to_vec(),
    });
    let outputs = receive(&mut leader, 2, message(0, 0, Body::Recovery { nonce: 9 }));
    let [Output::To(2, whole)] = outputs.as_slice() else {
        panic!("a small state answers whole; got {outputs:?}")
    };
    assert!(
        matches!(whole.body, Body::RecoveryResponse { state: Some(_), .. }),
        "no chunk framing on the common path"
    );
}

const OK: i32 = 0;
const VRR_MESSAGE: i32 = -5;

unsafe extern "C" {
    fn vrr_node_new(
        members_len: usize,
        members_data: *const u8,
        own_len: usize,
        own_data: *const u8,
        out: *mut *mut c_void,
    ) -> i32;
    fn vrr_node_free(node: *mut c_void);
    fn vrr_node_receive(node: *mut c_void, from: u32, len: usize, data: *const u8) -> i32;
}

fn ffi_node() -> *mut c_void {
    let members = b"0:1\0"
        .iter()
        .chain(b"1:1\0")
        .chain(b"2:1")
        .copied()
        .collect::<Vec<_>>();
    let own = b"1:1";
    let mut node = std::ptr::null_mut();
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
    node
}

fn service_json(message: u8) -> Vec<u8> {
    serde_json::to_vec(&Request::Set {
        message_id: Uuid::from_bytes([message; 16]),
        client_id: 7,
        request_num: 9,
        lock_id: 11,
        lease: Lease {
            lease_id: 13,
            holder: Uuid::from_bytes([17; 16]),
            expiry: 1_000,
        },
    })
    .unwrap()
}

fn chunk_wire(payload: Vec<u8>) -> Vec<u8> {
    let entry = LogEntry {
        slot: 1,
        client_id: 7,
        request_num: 9,
        message_id: Uuid::from_bytes([3; 16]),
        execution_time: 100,
        payload,
    };
    Message {
        epoch: 0,
        slot: 2,
        body: Body::StateChunk {
            transfer: id(13),
            kind: ChunkKind::StartEpoch,
            total: 2,
            first: 0,
            entries: vec![entry],
            state_slot: 2,
            state_commit: 0,
        },
    }
    .encode()
    .unwrap()
}

#[test]
fn an_invalid_payload_inside_a_chunk_is_rejected_like_a_whole_message() {
    let node = ffi_node();

    let valid = chunk_wire(service_json(3));
    assert_eq!(
        unsafe { vrr_node_receive(node, 0, valid.len(), valid.as_ptr()) },
        OK,
        "a valid chunk passes the inbound validation gate"
    );

    let invalid = chunk_wire(br#"{"op":"get","message_id":"not-a-uuid"}"#.to_vec());
    assert_eq!(
        unsafe { vrr_node_receive(node, 0, invalid.len(), invalid.as_ptr()) },
        VRR_MESSAGE,
        "an invalid payload inside a chunk is refused exactly as inside a whole state message"
    );

    unsafe { vrr_node_free(node) };
}

#[test]
fn state_chunk_round_trips_the_wire_codec() {
    let entries = log(2);
    let original = chunk(7, 2, id(14), ChunkKind::StartEpoch, 2, 0, entries, 1);
    let bytes = original.encode().unwrap();
    assert_eq!(
        u32::from_be_bytes(bytes[..4].try_into().unwrap()),
        Tag::StateChunk as u32
    );
    assert_eq!(Message::decode(&bytes), Some(original));
}
