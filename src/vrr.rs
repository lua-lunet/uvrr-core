use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const MAX_DATAGRAM: usize = 65_507;

pub type NodeId = u32;
pub type View = u32;
pub type Slot = u64;

#[repr(u32)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Prepare = 0x10,
    PrepareOk = 0x11,
    Commit = 0x12,
    StartViewChange = 0x20,
    DoViewChange = 0x21,
    StartView = 0x22,
    Recovery = 0x30,
    RecoveryResponse = 0x31,
}

impl Tag {
    fn from_u32(value: u32) -> Option<Self> {
        Some(match value {
            0x10 => Self::Prepare,
            0x11 => Self::PrepareOk,
            0x12 => Self::Commit,
            0x20 => Self::StartViewChange,
            0x21 => Self::DoViewChange,
            0x22 => Self::StartView,
            0x30 => Self::Recovery,
            0x31 => Self::RecoveryResponse,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub tag: Tag,
    pub view: View,
    pub slot: Slot,
}

impl Header {
    pub fn encode(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..4].copy_from_slice(&(self.tag as u32).to_be_bytes());
        bytes[4..8].copy_from_slice(&self.view.to_be_bytes());
        bytes[8..].copy_from_slice(&self.slot.to_be_bytes());
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        Some(Self {
            tag: Tag::from_u32(u32::from_be_bytes(bytes[..4].try_into().ok()?))?,
            view: u32::from_be_bytes(bytes[4..8].try_into().ok()?),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Prepare {
        commit: Slot,
        entry: LogEntry,
    },
    PrepareOk,
    Commit,
    StartViewChange,
    DoViewChange {
        retained_view: View,
        state: LogState,
    },
    StartView {
        state: LogState,
    },
    Recovery {
        nonce: u64,
    },
    RecoveryResponse {
        nonce: u64,
        state: Option<LogState>,
    },
}

impl Body {
    fn tag(&self) -> Tag {
        match self {
            Self::Prepare { .. } => Tag::Prepare,
            Self::PrepareOk => Tag::PrepareOk,
            Self::Commit => Tag::Commit,
            Self::StartViewChange => Tag::StartViewChange,
            Self::DoViewChange { .. } => Tag::DoViewChange,
            Self::StartView { .. } => Tag::StartView,
            Self::Recovery { .. } => Tag::Recovery,
            Self::RecoveryResponse { .. } => Tag::RecoveryResponse,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub view: View,
    pub slot: Slot,
    pub body: Body,
}

impl Message {
    pub fn header(&self) -> Header {
        Header {
            tag: self.body.tag(),
            view: self.view,
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
            view: header.view,
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
    ViewChange,
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

/// Read-only projection of every protocol-relevant piece of internal replica
/// state: the public scalars and log plus the client table and result
/// history, the prepare-acknowledgement quorum accumulator, the view-change
/// evidence (`retained_view`, start-change votes, the do-change reports and
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
    pub view: View,
    pub status: Status,
    pub slot: Slot,
    pub commit: Slot,
    pub executed: Slot,
    pub executing: Option<Slot>,
    pub log: Vec<LogEntry>,
    pub clients: BTreeMap<u64, ClientDiagnostic>,
    pub results: BTreeMap<(u64, u64), Vec<u8>>,
    pub prepare_oks: BTreeMap<Slot, BTreeSet<NodeId>>,
    pub retained_view: View,
    pub start_view_changes: BTreeSet<NodeId>,
    pub sent_do_view_change: bool,
    pub do_view_changes: BTreeMap<NodeId, (View, LogState)>,
    pub recovery_nonce: Option<u64>,
    pub recovery: BTreeMap<NodeId, (View, Option<LogState>)>,
}

#[derive(Clone)]
pub struct Replica {
    members: Vec<String>,
    node: NodeId,
    view: View,
    status: Status,
    slot: Slot,
    commit_slot: Slot,
    executed_slot: Slot,
    executing: Option<Slot>,
    log: Vec<LogEntry>,
    clients: BTreeMap<u64, ClientEntry>,
    results: BTreeMap<(u64, u64), Vec<u8>>,
    prepare_oks: BTreeMap<Slot, BTreeSet<NodeId>>,
    retained_view: View,
    start_view_changes: BTreeSet<NodeId>,
    sent_do_view_change: bool,
    do_view_changes: BTreeMap<NodeId, (View, LogState)>,
    recovery_nonce: Option<u64>,
    recovery: BTreeMap<NodeId, (View, Option<LogState>)>,
    // FIXME bad code delete replace it
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
            view: 0,
            status: Status::Normal,
            slot: 0,
            commit_slot: 0,
            executed_slot: 0,
            executing: None,
            log: Vec::new(),
            clients: BTreeMap::new(),
            results: BTreeMap::new(),
            prepare_oks: BTreeMap::new(),
            retained_view: 0,
            start_view_changes: BTreeSet::new(),
            sent_do_view_change: false,
            do_view_changes: BTreeMap::new(),
            recovery_nonce: None,
            recovery: BTreeMap::new(),
        })
    }

    pub fn view(&self) -> View {
        self.view
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
    pub fn leader_of(&self, view: View) -> NodeId {
        view % self.members.len() as u32
    }
    pub fn is_leader(&self) -> bool {
        self.leader_of(self.view) == self.node
    }

    /// Captures the complete diagnostic projection of the internal state.
    /// Pure observability: no mutation, no effect on any later step.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            view: self.view,
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
            retained_view: self.retained_view,
            start_view_changes: self.start_view_changes.clone(),
            sent_do_view_change: self.sent_do_view_change,
            do_view_changes: self.do_view_changes.clone(),
            recovery_nonce: self.recovery_nonce,
            recovery: self.recovery.clone(),
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
        prepare
            .encode()
            .map_or(MAX_DATAGRAM + 1, |prepare| prepare.len())
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
            view: self.view,
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

    pub fn step(&mut self, input: Input) -> Vec<Output> {
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
                .view
                .checked_add(1)
                .map_or_else(Vec::new, |view| self.enter_view_change(view)),
            Input::Complete { slot, result } if self.status != Status::Recovering => {
                self.complete(slot, result)
            }
            Input::Recover { nonce } => {
                self.status = Status::Recovering;
                self.recovery_nonce = Some(nonce);
                self.recovery.clear();
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
        let view = message.view;
        let slot = message.slot;
        match message.body {
            Body::Prepare { commit, entry }
                if self.status == Status::Normal
                    && view == self.view
                    && from == self.leader_of(view)
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
                    && view == self.view
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
                    && view == self.view
                    && from == self.leader_of(view)
                    && slot <= self.slot =>
            {
                self.commit_slot = self.commit_slot.max(slot);
                self.pending_execution()
            }
            Body::StartViewChange if !self.in_recovery() && view >= self.view => {
                let mut out = if view > self.view {
                    self.enter_view_change(view)
                } else {
                    Vec::new()
                };
                if self.status == Status::ViewChange {
                    self.start_view_changes.insert(from);
                    out.extend(self.qualify_view_change());
                }
                out
            }
            Body::DoViewChange {
                retained_view,
                state,
            } if !self.in_recovery() && view >= self.view && self.leader_of(view) == self.node => {
                if !self.valid_state_message(slot, &state) {
                    return Vec::new();
                }
                let mut out = if view > self.view {
                    self.enter_view_change(view)
                } else {
                    Vec::new()
                };
                if self.status != Status::ViewChange {
                    return out;
                }
                self.do_view_changes.insert(from, (retained_view, state));
                out.extend(self.finish_view_change());
                out
            }
            Body::StartView { state }
                if !self.in_recovery() && view >= self.view && from == self.leader_of(view) =>
            {
                if (view == self.view && self.status != Status::ViewChange)
                    || !self.valid_state_message(slot, &state)
                {
                    return Vec::new();
                }
                self.view = view;
                let retained_suffix = state.slot > state.commit;
                self.adopt(state);
                self.activate_view(view);
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
                    vec![Output::To(
                        from,
                        self.message(
                            self.slot,
                            Body::RecoveryResponse {
                                nonce,
                                state: Some(self.state()),
                            },
                        ),
                    )]
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
                if state.is_some() != (from == self.leader_of(view)) {
                    return Vec::new();
                }
                if state
                    .as_ref()
                    .is_some_and(|state| !self.valid_state_message(slot, state))
                {
                    return Vec::new();
                }
                let incoming = (view, state);
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
            _ => Vec::new(),
        }
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
            let view = self.view;
            self.activate_view(view);
        }
        out.extend(self.pending_execution());
        out
    }

    fn enter_view_change(&mut self, view: View) -> Vec<Output> {
        self.view = view;
        self.status = Status::ViewChange;
        self.start_view_changes.clear();
        self.do_view_changes.clear();
        self.prepare_oks.clear();
        self.sent_do_view_change = false;
        vec![Output::Broadcast(
            self.message(self.slot, Body::StartViewChange),
        )]
    }

    fn qualify_view_change(&mut self) -> Vec<Output> {
        if self.sent_do_view_change || self.start_view_changes.len() < self.quorum() - 1 {
            return Vec::new();
        }
        self.sent_do_view_change = true;
        if self.is_leader() {
            self.do_view_changes
                .insert(self.node, (self.retained_view, self.state()));
            self.finish_view_change()
        } else {
            let leader = self.leader_of(self.view);
            let slot = self.slot;
            let retained_view = self.retained_view;
            vec![Output::To(
                leader,
                self.message(
                    slot,
                    Body::DoViewChange {
                        retained_view,
                        state: self.state(),
                    },
                ),
            )]
        }
    }

    fn finish_view_change(&mut self) -> Vec<Output> {
        if !self.sent_do_view_change
            || !self.do_view_changes.contains_key(&self.node)
            || self.do_view_changes.len() < self.quorum()
        {
            return Vec::new();
        }
        let best = self
            .do_view_changes
            .values()
            .max_by_key(|(normal, state)| (*normal, state.slot))
            .expect("quorum")
            .1
            .clone();
        let commit_slot = self
            .do_view_changes
            .values()
            .map(|(_, state)| state.commit)
            .max()
            .expect("quorum");
        if best.slot < commit_slot {
            return Vec::new();
        }
        let view = self.view;
        self.adopt(LogState {
            slot: best.slot,
            commit: commit_slot,
            log: best.log,
        });
        self.activate_view(view);
        let mut out = self.pending_execution();
        let slot = self.slot;
        out.push(Output::Broadcast(self.message(
            slot,
            Body::StartView {
                state: self.state(),
            },
        )));
        out
    }

    fn stronger_recovery_response(
        stored: &(View, Option<LogState>),
        incoming: &(View, Option<LogState>),
    ) -> bool {
        let (stored_view, stored_state) = stored;
        let (incoming_view, incoming_state) = incoming;
        if incoming_view != stored_view {
            return incoming_view > stored_view;
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
        let view = self
            .recovery
            .values()
            .map(|(view, _)| *view)
            .max()
            .expect("quorum");
        let leader = self.leader_of(view);
        let Some((state_view, Some(state))) = self.recovery.get(&leader).cloned() else {
            return Vec::new();
        };
        if state_view != view {
            return Vec::new();
        }
        if !self.valid_state_message(state.slot, &state) {
            return Vec::new();
        }
        self.view = view;
        self.adopt(state);
        self.recovery_nonce = None;
        self.recovery.clear();
        if self.executed_slot >= self.commit_slot {
            self.activate_view(view);
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

    fn activate_view(&mut self, view: View) {
        self.status = Status::Normal;
        self.retained_view = view;
        self.start_view_changes.clear();
        self.do_view_changes.clear();
        self.sent_do_view_change = false;
    }
}
