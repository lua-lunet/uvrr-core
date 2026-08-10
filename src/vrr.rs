use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use uuid::Uuid;

pub const MAX_DATAGRAM: usize = 65_507;

/// Largest log length a peer may claim for one chunked state transfer. The
/// claim is validated before anything is buffered, and storage grows only
/// with chunks actually received, so a claimed `total` cannot grow memory
/// by itself.
pub const MAX_CHUNK_TRANSFER_TOTAL: u64 = 10_000;

/// Largest buffered byte count for one in-flight reassembly. Bytes are
/// accounted per received chunk datagram (an upper bound on the decoded
/// content); a transfer that would cross the cap is abandoned and freed.
pub const MAX_CHUNK_BUFFER_BYTES: usize = 10_000_000;

/// Largest summed buffered byte count across every in-flight reassembly on
/// this replica.
pub const MAX_CHUNK_REASSEMBLY_BYTES: usize = 4 * MAX_CHUNK_BUFFER_BYTES;

/// Consecutive `Input::Idle` ticks without progress after which an in-flight
/// reassembly is dropped. A tick counter, not wall time: the host owns all
/// timing, and the retry path is a fresh transfer under a new uuid.
pub const CHUNK_TRANSFER_IDLE_LIMIT: u64 = 100;

pub type NodeId = u32;
pub type Epoch = u32;
pub type Slot = u64;

#[repr(u32)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Prepare = 0x10,
    PrepareOk = 0x11,
    Commit = 0x12,
    StartEpochChange = 0x20,
    DoEpochChange = 0x21,
    StartEpoch = 0x22,
    Recovery = 0x30,
    RecoveryResponse = 0x31,
    StateChunk = 0x40,
}

impl Tag {
    fn from_u32(value: u32) -> Option<Self> {
        Some(match value {
            0x10 => Self::Prepare,
            0x11 => Self::PrepareOk,
            0x12 => Self::Commit,
            0x20 => Self::StartEpochChange,
            0x21 => Self::DoEpochChange,
            0x22 => Self::StartEpoch,
            0x30 => Self::Recovery,
            0x31 => Self::RecoveryResponse,
            0x40 => Self::StateChunk,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub tag: Tag,
    pub epoch: Epoch,
    pub slot: Slot,
}

impl Header {
    pub fn encode(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..4].copy_from_slice(&(self.tag as u32).to_be_bytes());
        bytes[4..8].copy_from_slice(&self.epoch.to_be_bytes());
        bytes[8..].copy_from_slice(&self.slot.to_be_bytes());
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        Some(Self {
            tag: Tag::from_u32(u32::from_be_bytes(bytes[..4].try_into().ok()?))?,
            epoch: u32::from_be_bytes(bytes[4..8].try_into().ok()?),
            slot: u64::from_be_bytes(bytes[8..16].try_into().ok()?),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub slot: Slot,
    pub client_id: u64,
    pub request_num: u64,
    pub message_id: Uuid,
    pub execution_time: u64,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LogState {
    pub slot: Slot,
    pub commit: Slot,
    pub log: Vec<LogEntry>,
}

/// Which logical state-transfer message a [`Body::StateChunk`] reassembles
/// into. The kind payload (for example the recovery nonce) is fixed by the
/// first chunk of a transfer; a later chunk that disagrees abandons it.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    DoEpochChange { latest_normal: Epoch },
    StartEpoch,
    RecoveryResponse { nonce: u64 },
}

impl ChunkKind {
    fn discriminant(self) -> u8 {
        match self {
            Self::DoEpochChange { .. } => 0,
            Self::StartEpoch => 1,
            Self::RecoveryResponse { .. } => 2,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Prepare {
        commit: Slot,
        entry: LogEntry,
    },
    PrepareOk,
    Commit,
    StartEpochChange,
    DoEpochChange {
        latest_normal: Epoch,
        state: LogState,
    },
    StartEpoch {
        state: LogState,
    },
    Recovery {
        nonce: u64,
    },
    RecoveryResponse {
        nonce: u64,
        state: Option<LogState>,
    },
    StateChunk {
        transfer: Uuid,
        kind: ChunkKind,
        total: u64,
        first: u64,
        entries: Vec<LogEntry>,
        state_slot: Slot,
        state_commit: Slot,
    },
}

impl Body {
    fn tag(&self) -> Tag {
        match self {
            Self::Prepare { .. } => Tag::Prepare,
            Self::PrepareOk => Tag::PrepareOk,
            Self::Commit => Tag::Commit,
            Self::StartEpochChange => Tag::StartEpochChange,
            Self::DoEpochChange { .. } => Tag::DoEpochChange,
            Self::StartEpoch { .. } => Tag::StartEpoch,
            Self::Recovery { .. } => Tag::Recovery,
            Self::RecoveryResponse { .. } => Tag::RecoveryResponse,
            Self::StateChunk { .. } => Tag::StateChunk,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub epoch: Epoch,
    pub slot: Slot,
    pub body: Body,
}

impl Message {
    pub fn header(&self) -> Header {
        Header {
            tag: self.body.tag(),
            epoch: self.epoch,
            slot: self.slot,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = self.header().encode().to_vec();
        bytes.extend(serde_json::to_vec(&self.body)?);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let header = Header::decode(bytes)?;
        let body: Body = serde_json::from_slice(&bytes[16..]).ok()?;
        (header.tag == body.tag()).then_some(Self {
            epoch: header.epoch,
            slot: header.slot,
            body,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Request {
        client_id: u64,
        request_num: u64,
        message_id: Uuid,
        execution_time: u64,
        payload: Vec<u8>,
    },
    Message {
        from: NodeId,
        message: Message,
    },
    Idle,
    LeaderTimeout,
    Complete {
        slot: Slot,
        result: Vec<u8>,
    },
    Recover {
        nonce: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Broadcast(Message),
    To(NodeId, Message),
    Execute {
        slot: Slot,
        client_id: u64,
        request_num: u64,
        message_id: Uuid,
        execution_time: u64,
        payload: Vec<u8>,
    },
    Reply(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Normal,
    EpochChange,
    Recovering,
    Replaying,
}

#[derive(Clone)]
struct ClientEntry {
    request_num: u64,
    message_id: Uuid,
    result: Option<Vec<u8>>,
}

/// Diagnostic projection of one client-table entry.
///
/// Part of the [`Diagnostic`] observability boundary; not part of the
/// protocol surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDiagnostic {
    pub request_num: u64,
    pub message_id: Uuid,
    pub result: Option<Vec<u8>>,
}

/// Diagnostic projection of one in-flight chunked state transfer.
///
/// Part of the [`Diagnostic`] observability boundary; not part of the
/// protocol surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTransferDiagnostic {
    pub from: NodeId,
    pub kind: ChunkKind,
    pub transfer: Uuid,
    pub total: u64,
    pub received: u64,
    pub buffered_bytes: usize,
    pub idle_ticks: u64,
}

/// One in-flight reassembly: the chunks of a single logical state message
/// received so far, keyed by entry index. Storage grows only with received
/// chunks, never with the claimed total.
#[derive(Clone)]
struct ChunkTransfer {
    transfer: Uuid,
    kind: ChunkKind,
    epoch: Epoch,
    header_slot: Slot,
    total: u64,
    state_slot: Slot,
    state_commit: Slot,
    entries: BTreeMap<u64, LogEntry>,
    buffered_bytes: usize,
    idle_ticks: u64,
}

/// Reassembly buffers keyed by `(from, kind discriminant)`: at most one
/// transfer in flight per sender per logical message kind, so the buffer
/// count can never exceed three times the membership size.
#[derive(Clone, Default)]
struct ChunkReceiver {
    transfers: BTreeMap<(NodeId, u8), ChunkTransfer>,
}

impl ChunkReceiver {
    fn key(from: NodeId, kind: ChunkKind) -> (NodeId, u8) {
        (from, kind.discriminant())
    }

    fn clear(&mut self) {
        self.transfers.clear();
    }

    fn tick(&mut self) {
        self.transfers.retain(|_, transfer| {
            transfer.idle_ticks += 1;
            transfer.idle_ticks < CHUNK_TRANSFER_IDLE_LIMIT
        });
    }

    fn buffered_bytes(&self) -> usize {
        self.transfers
            .values()
            .map(|transfer| transfer.buffered_bytes)
            .sum()
    }

    fn diagnostics(&self) -> Vec<ChunkTransferDiagnostic> {
        self.transfers
            .iter()
            .map(|((from, _), transfer)| ChunkTransferDiagnostic {
                from: *from,
                kind: transfer.kind,
                transfer: transfer.transfer,
                total: transfer.total,
                received: transfer.entries.len() as u64,
                buffered_bytes: transfer.buffered_bytes,
                idle_ticks: transfer.idle_ticks,
            })
            .collect()
    }
}

/// Read-only projection of every protocol-relevant piece of internal replica
/// state: the public scalars and log plus the client table and result
/// history, the prepare-acknowledgement quorum accumulator, the epoch-change
/// evidence (`latest_normal`, start-change votes, the do-change reports and
/// the sent marker), the in-flight execution marker, and the recovery nonce
/// and evidence map.
///
/// This is a deliberate observability boundary for integration tests, not
/// part of the protocol surface: capturing it mutates nothing and no protocol
/// decision reads it. Tests must compare complete `Diagnostic` values rather
/// than pretending equality of the public getters alone is complete
/// immutability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub epoch: Epoch,
    pub status: Status,
    pub slot: Slot,
    pub commit: Slot,
    pub executed: Slot,
    pub executing: Option<Slot>,
    pub log: Vec<LogEntry>,
    pub clients: BTreeMap<u64, ClientDiagnostic>,
    pub results: BTreeMap<(u64, u64), Vec<u8>>,
    pub prepare_oks: BTreeMap<Slot, BTreeSet<NodeId>>,
    pub latest_normal: Epoch,
    pub start_changes: BTreeSet<NodeId>,
    pub sent_do_change: bool,
    pub do_changes: BTreeMap<NodeId, (Epoch, LogState)>,
    pub recovery_nonce: Option<u64>,
    pub recovery: BTreeMap<NodeId, (Epoch, Option<LogState>)>,
    pub transfers: Vec<ChunkTransferDiagnostic>,
}

#[derive(Clone)]
pub struct Replica {
    members: Vec<String>,
    node: NodeId,
    epoch: Epoch,
    status: Status,
    slot: Slot,
    commit_slot: Slot,
    executed_slot: Slot,
    executing: Option<Slot>,
    log: Vec<LogEntry>,
    clients: BTreeMap<u64, ClientEntry>,
    results: BTreeMap<(u64, u64), Vec<u8>>,
    prepare_oks: BTreeMap<Slot, BTreeSet<NodeId>>,
    latest_normal: Epoch,
    start_changes: BTreeSet<NodeId>,
    sent_do_change: bool,
    do_changes: BTreeMap<NodeId, (Epoch, LogState)>,
    recovery_nonce: Option<u64>,
    recovery: BTreeMap<NodeId, (Epoch, Option<LogState>)>,
    chunk_transfers: ChunkReceiver,
}

impl Replica {
    pub fn new(members: Vec<String>, own: &str) -> Result<Self, &'static str> {
        if members.len() < 3 || members.len() > u32::MAX as usize {
            return Err("membership must contain at least three representable nodes");
        }
        if members.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err("membership must be strictly sorted");
        }
        let node = members
            .iter()
            .position(|member| member == own)
            .ok_or("own address is not in membership")? as NodeId;
        Ok(Self {
            members,
            node,
            epoch: 0,
            status: Status::Normal,
            slot: 0,
            commit_slot: 0,
            executed_slot: 0,
            executing: None,
            log: Vec::new(),
            clients: BTreeMap::new(),
            results: BTreeMap::new(),
            prepare_oks: BTreeMap::new(),
            latest_normal: 0,
            start_changes: BTreeSet::new(),
            sent_do_change: false,
            do_changes: BTreeMap::new(),
            recovery_nonce: None,
            recovery: BTreeMap::new(),
            chunk_transfers: ChunkReceiver::default(),
        })
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn slot(&self) -> Slot {
        self.slot
    }
    pub fn commit(&self) -> Slot {
        self.commit_slot
    }
    pub fn executed(&self) -> Slot {
        self.executed_slot
    }
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }
    pub fn leader_of(&self, epoch: Epoch) -> NodeId {
        epoch % self.members.len() as u32
    }
    pub fn is_leader(&self) -> bool {
        self.leader_of(self.epoch) == self.node
    }

    /// Captures the complete diagnostic projection of the internal state.
    /// Pure observability: no mutation, no effect on any later step.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            epoch: self.epoch,
            status: self.status,
            slot: self.slot,
            commit: self.commit_slot,
            executed: self.executed_slot,
            executing: self.executing,
            log: self.log.clone(),
            clients: self
                .clients
                .iter()
                .map(|(client_id, client)| {
                    (
                        *client_id,
                        ClientDiagnostic {
                            request_num: client.request_num,
                            message_id: client.message_id,
                            result: client.result.clone(),
                        },
                    )
                })
                .collect(),
            results: self.results.clone(),
            prepare_oks: self.prepare_oks.clone(),
            latest_normal: self.latest_normal,
            start_changes: self.start_changes.clone(),
            sent_do_change: self.sent_do_change,
            do_changes: self.do_changes.clone(),
            recovery_nonce: self.recovery_nonce,
            recovery: self.recovery.clone(),
            transfers: self.chunk_transfers.diagnostics(),
        }
    }

    pub fn request_datagram_size(
        &self,
        client_id: u64,
        request_num: u64,
        message_id: Uuid,
        execution_time: u64,
        payload: &[u8],
    ) -> usize {
        if self.status != Status::Normal
            || !self.is_leader()
            || self
                .clients
                .get(&client_id)
                .is_some_and(|client| request_num <= client.request_num)
        {
            return 0;
        }
        let Some(slot) = self.slot.checked_add(1) else {
            return MAX_DATAGRAM + 1;
        };
        let entry = LogEntry {
            slot,
            client_id,
            request_num,
            message_id,
            execution_time,
            payload: payload.to_vec(),
        };
        let prepare = self.message(
            slot,
            Body::Prepare {
                commit: self.commit_slot,
                entry: entry.clone(),
            },
        );
        // Reserve the StateChunk framing as well as the Prepare framing: any
        // admitted request must be representable as a one-entry chunk, or the
        // log could hold an entry that no state transfer can ever move.
        // Worst-case integer widths keep the reserve valid as those fields
        // grow over the replica's lifetime.
        let chunk = self.message(
            slot,
            Body::StateChunk {
                transfer: Uuid::nil(),
                kind: ChunkKind::RecoveryResponse { nonce: u64::MAX },
                total: u64::MAX,
                first: u64::MAX - 1,
                entries: vec![entry],
                state_slot: u64::MAX,
                state_commit: u64::MAX,
            },
        );
        match (prepare.encode(), chunk.encode()) {
            (Ok(prepare), Ok(chunk)) => prepare.len().max(chunk.len()),
            _ => MAX_DATAGRAM + 1,
        }
    }

    fn quorum(&self) -> usize {
        self.members.len() / 2 + 1
    }
    fn member(&self, node: NodeId) -> bool {
        (node as usize) < self.members.len()
    }
    fn in_recovery(&self) -> bool {
        matches!(self.status, Status::Recovering | Status::Replaying)
    }
    fn message(&self, slot: Slot, body: Body) -> Message {
        Message {
            epoch: self.epoch,
            slot,
            body,
        }
    }
    fn state(&self) -> LogState {
        LogState {
            slot: self.slot,
            commit: self.commit_slot,
            log: self.log.clone(),
        }
    }

    fn state_body(kind: ChunkKind, state: LogState) -> Body {
        match kind {
            ChunkKind::DoEpochChange { latest_normal } => Body::DoEpochChange {
                latest_normal,
                state,
            },
            ChunkKind::StartEpoch => Body::StartEpoch { state },
            ChunkKind::RecoveryResponse { nonce } => Body::RecoveryResponse {
                nonce,
                state: Some(state),
            },
        }
    }

    /// The one logical state message, whole when it fits one datagram (the
    /// common path, wire-identical to before chunking existed) or as an
    /// ordered run of `StateChunk` datagrams greedily packed to
    /// `MAX_DATAGRAM`. Admission reserves the one-entry chunk framing, so a
    /// lone entry always fits; if an unrepresentable entry ever existed the
    /// oversize chunk is emitted anyway and the ffi.rs datagram cap stays
    /// the atomic backstop it was before.
    fn state_transfer(&self, slot: Slot, kind: ChunkKind, state: LogState) -> Vec<Message> {
        let whole = self.message(slot, Self::state_body(kind, state));
        if whole
            .encode()
            .is_ok_and(|bytes| bytes.len() <= MAX_DATAGRAM)
        {
            return vec![whole];
        }
        let state = match whole.body {
            Body::DoEpochChange { state, .. } | Body::StartEpoch { state } => state,
            Body::RecoveryResponse {
                state: Some(state), ..
            } => state,
            _ => unreachable!("state_body always carries a state"),
        };
        let total = state.log.len() as u64;
        let state_slot = state.slot;
        let state_commit = state.commit;
        let transfer = Uuid::new_v4();
        let chunk = |first: u64, entries: Vec<LogEntry>| {
            self.message(
                slot,
                Body::StateChunk {
                    transfer,
                    kind,
                    total,
                    first,
                    entries,
                    state_slot,
                    state_commit,
                },
            )
        };
        let mut messages = Vec::new();
        let mut first = 0;
        let mut entries: Vec<LogEntry> = Vec::new();
        for entry in state.log {
            entries.push(entry);
            let oversized = entries.len() > 1
                && chunk(first, entries.clone())
                    .encode()
                    .is_ok_and(|bytes| bytes.len() > MAX_DATAGRAM);
            if oversized {
                let tail = entries.pop().expect("more than one entry");
                let count = entries.len() as u64;
                messages.push(chunk(first, std::mem::take(&mut entries)));
                first += count;
                entries.push(tail);
            }
        }
        if !entries.is_empty() {
            messages.push(chunk(first, entries));
        }
        messages
    }

    pub fn step(&mut self, input: Input) -> Vec<Output> {
        if let Input::Idle = input {
            self.chunk_transfers.tick();
        }
        match input {
            Input::Request {
                client_id,
                request_num,
                message_id,
                execution_time,
                payload,
            } => self.request(client_id, request_num, message_id, execution_time, payload),
            Input::Message { from, message } => self.receive(from, message),
            Input::Idle if self.status == Status::Normal && self.is_leader() => {
                vec![Output::Broadcast(
                    self.message(self.commit_slot, Body::Commit),
                )]
            }
            Input::LeaderTimeout if !self.in_recovery() => self
                .epoch
                .checked_add(1)
                .map_or_else(Vec::new, |epoch| self.enter_change(epoch)),
            Input::Complete { slot, result } if self.status != Status::Recovering => {
                self.complete(slot, result)
            }
            Input::Recover { nonce } => {
                self.status = Status::Recovering;
                self.recovery_nonce = Some(nonce);
                self.recovery.clear();
                self.chunk_transfers.clear();
                vec![Output::Broadcast(
                    self.message(self.slot, Body::Recovery { nonce }),
                )]
            }
            _ => Vec::new(),
        }
    }

    fn request(
        &mut self,
        client_id: u64,
        request_num: u64,
        message_id: Uuid,
        execution_time: u64,
        payload: Vec<u8>,
    ) -> Vec<Output> {
        if self.status != Status::Normal || !self.is_leader() {
            return Vec::new();
        }
        if let Some(client) = self.clients.get(&client_id) {
            if request_num < client.request_num {
                return Vec::new();
            }
            if request_num == client.request_num {
                return client
                    .result
                    .clone()
                    .map(Output::Reply)
                    .into_iter()
                    .collect();
            }
        }
        let Some(slot) = self.slot.checked_add(1) else {
            return Vec::new();
        };
        if self.log.iter().any(|entry| entry.message_id == message_id) {
            return Vec::new();
        }
        self.slot = slot;
        let entry = LogEntry {
            slot: self.slot,
            client_id,
            request_num,
            message_id,
            execution_time,
            payload,
        };
        self.log.push(entry.clone());
        self.clients.insert(
            client_id,
            ClientEntry {
                request_num,
                message_id,
                result: None,
            },
        );
        vec![Output::Broadcast(self.message(
            self.slot,
            Body::Prepare {
                commit: self.commit_slot,
                entry,
            },
        ))]
    }

    fn receive(&mut self, from: NodeId, message: Message) -> Vec<Output> {
        if !self.member(from) || from == self.node {
            return Vec::new();
        }
        let epoch = message.epoch;
        let slot = message.slot;
        match message.body {
            Body::Prepare { commit, entry }
                if self.status == Status::Normal
                    && epoch == self.epoch
                    && from == self.leader_of(epoch)
                    && !self.is_leader() =>
            {
                if slot <= self.slot {
                    let installed = usize::try_from(slot)
                        .ok()
                        .and_then(|slot| slot.checked_sub(1))
                        .and_then(|index| self.log.get(index));
                    if installed != Some(&entry) || commit > self.slot {
                        return Vec::new();
                    }
                    self.commit_slot = self.commit_slot.max(commit);
                    let mut out = self.pending_execution();
                    out.push(Output::To(from, self.message(slot, Body::PrepareOk)));
                    return out;
                }
                if self.slot.checked_add(1) != Some(slot)
                    || entry.slot != slot
                    || commit > slot
                    || self
                        .log
                        .iter()
                        .any(|old| old.message_id == entry.message_id)
                    || self
                        .clients
                        .get(&entry.client_id)
                        .is_some_and(|client| entry.request_num <= client.request_num)
                {
                    return Vec::new();
                }
                self.slot = slot;
                self.log.push(entry.clone());
                self.clients.insert(
                    entry.client_id,
                    ClientEntry {
                        request_num: entry.request_num,
                        message_id: entry.message_id,
                        result: None,
                    },
                );
                self.commit_slot = self.commit_slot.max(commit);
                let mut out = self.pending_execution();
                out.push(Output::To(from, self.message(self.slot, Body::PrepareOk)));
                out
            }
            Body::PrepareOk
                if self.status == Status::Normal
                    && epoch == self.epoch
                    && self.is_leader()
                    && slot <= self.slot =>
            {
                self.prepare_oks.entry(slot).or_default().insert(from);
                if self.prepare_oks[&slot].len() < self.quorum() - 1 {
                    Vec::new()
                } else {
                    self.commit_slot = self.commit_slot.max(slot);
                    self.pending_execution()
                }
            }
            Body::Commit
                if self.status == Status::Normal
                    && epoch == self.epoch
                    && from == self.leader_of(epoch)
                    && slot <= self.slot =>
            {
                self.commit_slot = self.commit_slot.max(slot);
                self.pending_execution()
            }
            Body::StartEpochChange if !self.in_recovery() && epoch >= self.epoch => {
                let mut out = if epoch > self.epoch {
                    self.enter_change(epoch)
                } else {
                    Vec::new()
                };
                if self.status == Status::EpochChange {
                    self.start_changes.insert(from);
                    out.extend(self.qualify_change());
                }
                out
            }
            Body::DoEpochChange {
                latest_normal,
                state,
            } if !self.in_recovery()
                && epoch >= self.epoch
                && self.leader_of(epoch) == self.node =>
            {
                if !self.valid_state_message(slot, &state) {
                    return Vec::new();
                }
                let mut out = if epoch > self.epoch {
                    self.enter_change(epoch)
                } else {
                    Vec::new()
                };
                if self.status != Status::EpochChange {
                    return out;
                }
                self.do_changes.insert(from, (latest_normal, state));
                out.extend(self.finish_change());
                out
            }
            Body::StartEpoch { state }
                if !self.in_recovery() && epoch >= self.epoch && from == self.leader_of(epoch) =>
            {
                if (epoch == self.epoch && self.status != Status::EpochChange)
                    || !self.valid_state_message(slot, &state)
                {
                    return Vec::new();
                }
                self.epoch = epoch;
                let retained_suffix = state.slot > state.commit;
                self.adopt(state);
                self.activate_epoch(epoch);
                let mut out = if retained_suffix {
                    vec![Output::To(from, self.message(self.slot, Body::PrepareOk))]
                } else {
                    Vec::new()
                };
                out.extend(self.pending_execution());
                out
            }
            Body::Recovery { nonce } if self.status == Status::Normal => {
                if self.is_leader() {
                    let slot = self.slot;
                    self.state_transfer(slot, ChunkKind::RecoveryResponse { nonce }, self.state())
                        .into_iter()
                        .map(|message| Output::To(from, message))
                        .collect()
                } else {
                    vec![Output::To(
                        from,
                        self.message(self.slot, Body::RecoveryResponse { nonce, state: None }),
                    )]
                }
            }
            Body::RecoveryResponse { nonce, state }
                if self.status == Status::Recovering && self.recovery_nonce == Some(nonce) =>
            {
                if state.is_some() != (from == self.leader_of(epoch)) {
                    return Vec::new();
                }
                if state
                    .as_ref()
                    .is_some_and(|state| !self.valid_state_message(slot, state))
                {
                    return Vec::new();
                }
                let incoming = (epoch, state);
                if self
                    .recovery
                    .get(&from)
                    .is_some_and(|stored| !Self::stronger_recovery_response(stored, &incoming))
                {
                    Vec::new()
                } else {
                    self.recovery.insert(from, incoming);
                    self.finish_recovery()
                }
            }
            Body::StateChunk {
                transfer,
                kind,
                total,
                first,
                entries,
                state_slot,
                state_commit,
            } => self.receive_chunk(
                from,
                epoch,
                slot,
                transfer,
                kind,
                total,
                first,
                entries,
                state_slot,
                state_commit,
            ),
            _ => Vec::new(),
        }
    }

    /// Buffers one chunk of a chunked state transfer and, once the chunks
    /// contiguously cover `0..total`, materialises the logical message and
    /// re-enters the ordinary `receive` arm for it. No partial state is ever
    /// adopted: an incomplete transfer sits in the reassembly buffer and
    /// abandonment frees it entirely.
    #[allow(clippy::too_many_arguments)]
    fn receive_chunk(
        &mut self,
        from: NodeId,
        epoch: Epoch,
        slot: Slot,
        transfer: Uuid,
        kind: ChunkKind,
        total: u64,
        first: u64,
        entries: Vec<LogEntry>,
        state_slot: Slot,
        state_commit: Slot,
    ) -> Vec<Output> {
        // Buffer only what the re-entered logical message's own arm would
        // accept; every deeper check still happens on re-entry.
        let plausible = match kind {
            ChunkKind::DoEpochChange { .. } => {
                !self.in_recovery() && epoch >= self.epoch && self.leader_of(epoch) == self.node
            }
            ChunkKind::StartEpoch => {
                !self.in_recovery() && epoch >= self.epoch && from == self.leader_of(epoch)
            }
            ChunkKind::RecoveryResponse { nonce } => {
                self.status == Status::Recovering
                    && self.recovery_nonce == Some(nonce)
                    && from == self.leader_of(epoch)
            }
        };
        if !plausible {
            return Vec::new();
        }
        let count = entries.len() as u64;
        let well_formed = total != 0
            && total <= MAX_CHUNK_TRANSFER_TOTAL
            && total == state_slot
            && state_commit <= state_slot
            && first < total
            && !entries.is_empty()
            && first.checked_add(count).is_some_and(|end| end <= total)
            && entries
                .iter()
                .enumerate()
                .all(|(index, entry)| entry.slot == first + index as u64 + 1);
        if !well_formed {
            return Vec::new();
        }
        let chunk_bytes = self
            .message(
                slot,
                Body::StateChunk {
                    transfer,
                    kind,
                    total,
                    first,
                    entries: entries.clone(),
                    state_slot,
                    state_commit,
                },
            )
            .encode()
            .map_or(MAX_DATAGRAM + 1, |bytes| bytes.len());
        let key = ChunkReceiver::key(from, kind);
        // A fresh transfer uuid from the same sender and kind supersedes
        // whatever was buffered: the retry path restarts the whole transfer,
        // never a per-chunk retransmit.
        if self
            .chunk_transfers
            .transfers
            .get(&key)
            .is_some_and(|buffer| buffer.transfer != transfer)
        {
            self.chunk_transfers.transfers.remove(&key);
        }
        if !self.chunk_transfers.transfers.contains_key(&key) {
            if self.chunk_transfers.transfers.len() >= 3 * self.members.len() {
                return Vec::new();
            }
            self.chunk_transfers.transfers.insert(
                key,
                ChunkTransfer {
                    transfer,
                    kind,
                    epoch,
                    header_slot: slot,
                    total,
                    state_slot,
                    state_commit,
                    entries: BTreeMap::new(),
                    buffered_bytes: 0,
                    idle_ticks: 0,
                },
            );
        }
        // The first chunk fixes the invariant fields; a chunk of the same
        // transfer that disagrees abandons it.
        let consistent = self
            .chunk_transfers
            .transfers
            .get(&key)
            .is_some_and(|buffer| {
                buffer.epoch == epoch
                    && buffer.header_slot == slot
                    && buffer.kind == kind
                    && buffer.total == total
                    && buffer.state_slot == state_slot
                    && buffer.state_commit == state_commit
            });
        if !consistent {
            self.chunk_transfers.transfers.remove(&key);
            return Vec::new();
        }
        // Chunks land in any order: duplicate ranges are idempotent, a
        // conflicting overlap abandons the transfer.
        let mut fresh = false;
        let mut conflict = false;
        {
            let buffer = self
                .chunk_transfers
                .transfers
                .get_mut(&key)
                .expect("inserted");
            for (index, entry) in entries.into_iter().enumerate() {
                match buffer.entries.entry(first + index as u64) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(entry);
                        fresh = true;
                    }
                    Entry::Occupied(occupied) => {
                        if occupied.get() != &entry {
                            conflict = true;
                            break;
                        }
                    }
                }
            }
        }
        if conflict {
            self.chunk_transfers.transfers.remove(&key);
            return Vec::new();
        }
        if fresh {
            let buffered = self.chunk_transfers.buffered_bytes();
            let buffer = self
                .chunk_transfers
                .transfers
                .get_mut(&key)
                .expect("inserted");
            if buffer.buffered_bytes + chunk_bytes > MAX_CHUNK_BUFFER_BYTES
                || buffered + chunk_bytes > MAX_CHUNK_REASSEMBLY_BYTES
            {
                self.chunk_transfers.transfers.remove(&key);
                return Vec::new();
            }
            buffer.buffered_bytes += chunk_bytes;
            buffer.idle_ticks = 0;
        }
        let complete = self
            .chunk_transfers
            .transfers
            .get(&key)
            .is_some_and(|buffer| buffer.entries.len() as u64 == buffer.total);
        if !complete {
            return Vec::new();
        }
        let buffer = self
            .chunk_transfers
            .transfers
            .remove(&key)
            .expect("complete");
        let state = LogState {
            slot: buffer.state_slot,
            commit: buffer.state_commit,
            log: buffer.entries.into_values().collect(),
        };
        self.receive(
            from,
            Message {
                epoch: buffer.epoch,
                slot: buffer.header_slot,
                body: Self::state_body(buffer.kind, state),
            },
        )
    }

    fn pending_execution(&mut self) -> Vec<Output> {
        if self.executing.is_some() || self.executed_slot >= self.commit_slot {
            return Vec::new();
        }
        let entry = self.log[self.executed_slot as usize].clone();
        self.executing = Some(entry.slot);
        vec![Output::Execute {
            slot: entry.slot,
            client_id: entry.client_id,
            request_num: entry.request_num,
            message_id: entry.message_id,
            execution_time: entry.execution_time,
            payload: entry.payload.clone(),
        }]
    }

    fn complete(&mut self, slot: Slot, result: Vec<u8>) -> Vec<Output> {
        if self.executing != Some(slot)
            || self.executed_slot.checked_add(1) != Some(slot)
            || slot > self.commit_slot
        {
            return Vec::new();
        }
        let entry = &self.log[(slot - 1) as usize];
        let client_id = entry.client_id;
        let request_num = entry.request_num;
        let message_id = entry.message_id;
        let mut out = Vec::new();
        let mut matched = false;
        if let Some(client) = self.clients.get_mut(&client_id) {
            if client.request_num == request_num && client.message_id == message_id {
                client.result = Some(result.clone());
                matched = true;
            }
        }
        self.results
            .insert((client_id, request_num), result.clone());
        if matched && self.status == Status::Normal && self.is_leader() {
            out.push(Output::Reply(result));
        }
        self.executed_slot = slot;
        self.executing = None;
        if self.status == Status::Replaying && self.executed_slot == self.commit_slot {
            let epoch = self.epoch;
            self.activate_epoch(epoch);
        }
        out.extend(self.pending_execution());
        out
    }

    fn enter_change(&mut self, epoch: Epoch) -> Vec<Output> {
        self.epoch = epoch;
        self.status = Status::EpochChange;
        self.start_changes.clear();
        self.do_changes.clear();
        self.prepare_oks.clear();
        self.sent_do_change = false;
        self.chunk_transfers.clear();
        vec![Output::Broadcast(
            self.message(self.slot, Body::StartEpochChange),
        )]
    }

    fn qualify_change(&mut self) -> Vec<Output> {
        if self.sent_do_change || self.start_changes.len() < self.quorum() - 1 {
            return Vec::new();
        }
        self.sent_do_change = true;
        if self.is_leader() {
            self.do_changes
                .insert(self.node, (self.latest_normal, self.state()));
            self.finish_change()
        } else {
            let leader = self.leader_of(self.epoch);
            let slot = self.slot;
            let latest_normal = self.latest_normal;
            self.state_transfer(
                slot,
                ChunkKind::DoEpochChange { latest_normal },
                self.state(),
            )
            .into_iter()
            .map(|message| Output::To(leader, message))
            .collect()
        }
    }

    fn finish_change(&mut self) -> Vec<Output> {
        if !self.sent_do_change
            || !self.do_changes.contains_key(&self.node)
            || self.do_changes.len() < self.quorum()
        {
            return Vec::new();
        }
        let best = self
            .do_changes
            .values()
            .max_by_key(|(normal, state)| (*normal, state.slot))
            .expect("quorum")
            .1
            .clone();
        let commit_slot = self
            .do_changes
            .values()
            .map(|(_, state)| state.commit)
            .max()
            .expect("quorum");
        if best.slot < commit_slot {
            return Vec::new();
        }
        let epoch = self.epoch;
        self.adopt(LogState {
            slot: best.slot,
            commit: commit_slot,
            log: best.log,
        });
        self.activate_epoch(epoch);
        let mut out = self.pending_execution();
        let slot = self.slot;
        out.extend(
            self.state_transfer(slot, ChunkKind::StartEpoch, self.state())
                .into_iter()
                .map(Output::Broadcast),
        );
        out
    }

    fn stronger_recovery_response(
        stored: &(Epoch, Option<LogState>),
        incoming: &(Epoch, Option<LogState>),
    ) -> bool {
        let (stored_epoch, stored_state) = stored;
        let (incoming_epoch, incoming_state) = incoming;
        if incoming_epoch != stored_epoch {
            return incoming_epoch > stored_epoch;
        }
        match (stored_state, incoming_state) {
            (Some(stored), Some(incoming)) => {
                (incoming.slot, incoming.commit) > (stored.slot, stored.commit)
                    && incoming.slot >= stored.slot
                    && incoming.commit >= stored.commit
                    && incoming.log.len() >= stored.log.len()
                    && incoming.log[..stored.log.len()] == stored.log[..]
            }
            (None, Some(_)) => true,
            _ => false,
        }
    }

    fn finish_recovery(&mut self) -> Vec<Output> {
        if self.recovery.len() < self.quorum() {
            return Vec::new();
        }
        let epoch = self
            .recovery
            .values()
            .map(|(epoch, _)| *epoch)
            .max()
            .expect("quorum");
        let leader = self.leader_of(epoch);
        let Some((state_epoch, Some(state))) = self.recovery.get(&leader).cloned() else {
            return Vec::new();
        };
        if state_epoch != epoch {
            return Vec::new();
        }
        if !self.valid_state_message(state.slot, &state) {
            return Vec::new();
        }
        self.epoch = epoch;
        self.adopt(state);
        self.recovery_nonce = None;
        self.recovery.clear();
        self.chunk_transfers.clear();
        if self.executed_slot >= self.commit_slot {
            self.activate_epoch(epoch);
            Vec::new()
        } else {
            self.status = Status::Replaying;
            self.pending_execution()
        }
    }

    fn structurally_valid(state: &LogState) -> bool {
        if u64::try_from(state.log.len()).ok() != Some(state.slot) || state.commit > state.slot {
            return false;
        }
        let mut requests = BTreeMap::new();
        let mut messages = BTreeSet::new();
        state.log.iter().enumerate().all(|(index, entry)| {
            let slot = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_add(1));
            slot == Some(entry.slot)
                && messages.insert(entry.message_id)
                && requests
                    .insert(entry.client_id, entry.request_num)
                    .is_none_or(|previous| entry.request_num > previous)
        })
    }

    fn valid_state_message(&self, header_slot: Slot, state: &LogState) -> bool {
        header_slot == state.slot
            && state.commit >= self.executed_slot
            && Self::structurally_valid(state)
            && state.log.len() >= self.executed_slot as usize
            && state.log[..self.executed_slot as usize] == self.log[..self.executed_slot as usize]
    }

    fn adopt(&mut self, state: LogState) {
        let old_clients = std::mem::take(&mut self.clients);
        let old_results = std::mem::take(&mut self.results);
        self.log = state.log;
        self.slot = state.slot;
        self.commit_slot = state.commit;
        self.executing = None;
        self.prepare_oks.clear();
        for entry in &self.log {
            let key = (entry.client_id, entry.request_num);
            let result = old_results.get(&key).cloned().or_else(|| {
                old_clients
                    .get(&entry.client_id)
                    .filter(|old| {
                        old.request_num == entry.request_num && old.message_id == entry.message_id
                    })
                    .and_then(|old| old.result.clone())
            });
            if let Some(result) = &result {
                self.results.insert(key, result.clone());
            }
            self.clients.insert(
                entry.client_id,
                ClientEntry {
                    request_num: entry.request_num,
                    message_id: entry.message_id,
                    result,
                },
            );
        }
    }

    fn activate_epoch(&mut self, epoch: Epoch) {
        self.status = Status::Normal;
        self.latest_normal = epoch;
        self.start_changes.clear();
        self.do_changes.clear();
        self.sent_do_change = false;
        self.chunk_transfers.clear();
    }
}
