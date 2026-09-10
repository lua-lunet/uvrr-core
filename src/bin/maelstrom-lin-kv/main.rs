//! Maelstrom node that runs the unmodified VRR replication core against the
//! `lin-kv` workload, so Knossos can check the protocol for linearizability
//! under Maelstrom's partition / kill / pause nemeses.
//!
//! The core is used as a Rust library (`vrr::replica::Replica`), planner
//! architecture: every event becomes a [`TimedInput`], [`Replica::plan`]
//! computes a transition against the published state and a journal view, and
//! this host publishes it with [`Replica::publish`] and executes the released
//! effects. The host owns time, transport, storage and the application —
//! exactly the split the core is written against.
//!
//! Architecture: two producer threads (stdin reader, ticker) feed one
//! channel; a single core thread owns all state and is the only writer to
//! stdout, so there is no lock around the replica.
//!
//! # Durability stance (§7)
//!
//! The node runs [`Stability::Volatile`] and persists nothing. That is safe
//! because of the genesis ruling (§1.3): every construction — first boot or
//! kill-nemesis restart alike — starts fenced (`Joining` on provision,
//! `Restarting` on reopen), and a node
//! becomes `Normal` only through the bootstrap adoption (§4). A voter whose
//! volatile state vanished while retaining authority is unrepresentable in
//! this host: a restarted node rejoins fenced, and the next view change
//! deposes any stale primary.
//!
//! # Time (S4)
//!
//! The core reads no clock; the host owns time. This host's clock is a
//! single `u64` counter, bumped once per driven input and carried as
//! [`TimedInput::at`].
//!
//! # Membership verbs (§8.7)
//!
//! Four Maelstrom message kinds drive cluster making through the core's
//! public [`Input::Reconfigure`] path — the §8.7.2 pre-proposal gates are
//! the core's, and every core refusal becomes a named Maelstrom `error`,
//! never a crash:
//!
//! - `join {node_id}` — admit an already-running bench node at weight 0
//!   ([`SystemOperation::Join`], stop-the-world). The bench roster is fixed
//!   by `node_ids`, so a join naming an id outside the roster is refused by
//!   the host (`malformed-request`, code 12) before the core is touched; a
//!   join of a member the configuration already holds is refused by the
//!   fold itself.
//! - `promote {node_id}` — one weight step toward voting
//!   ([`SystemOperation::Increment`]); takes the §8.7.6 pivot when one
//!   exists for this leader, and falls back to stop-the-world when the
//!   leader is non-pivotal — a latency outcome, not an error.
//! - `demote {node_id}` — one weight step down ([`SystemOperation::Decrement`],
//!   stop-the-world).
//! - `leave {node_id}` — remove a weight-0 member ([`SystemOperation::Leave`],
//!   stop-the-world); the canonical departure is `demote` to 0 then `leave`
//!   of the same node.
//!
//! A verb is a client RPC like any other: a node that does not lead
//! forwards it once, exactly as the lin-kv path does. The reply is not the
//! proposal's: it leaves when the establishing operation COMMITS and the
//! era folds on this node (§8.7.1), and every reply — `*_ok` or `error` —
//! echoes the node's current configuration view: the established era, the
//! current view, and the member order with weights. A refusal is definite
//! (the gates run before the proposal; the operation never entered the
//! log); a verb whose establishing operation dies uncommitted in a view
//! change gets no reply at all, which is the honest indeterminate.

mod kv;
mod proto;

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;
use vrr::configuration::SystemOperation;
use vrr::effects::{Effect, Stability};
use vrr::ids::{Era, NodeId, Operation, OperationId, Slot, Tick};
use vrr::journal::{Journal, SegmentedLog};
use vrr::message::Message;
use vrr::progress::Status;
use vrr::quorum::WeightedMajority;
use vrr::replica::{
    Input, PlanRefusal, PublishOutcome, Replica, TimedInput, ViewChangeKnobs, construct_pivot,
};
use vrr::wire::{Pack, Unpack};

use crate::kv::Kv;
use crate::proto::{Incoming, KvRequest, KvResponse, Outgoing, error, from_hex, to_hex};

/// Scheduler granularity. Everything below is expressed in ticks.
const TICK: Duration = Duration::from_millis(100);
/// Ticks of primary silence a `Normal` backup tolerates before fencing into
/// the next view (W5: the knob is host policy, uniform across the cluster).
/// With client traffic flowing, the primary's `Prepare`/`Commit` stream is
/// the activity evidence and suspicion never fires; the knob only decides
/// how quickly a genuinely dead primary is deposed.
const PRIMARY_TIMEOUT_TICKS: u64 = 25;

/// The replica this host runs: the default journal and the default quorum
/// strategy, exactly as the test harness provisions them.
type Node = Replica<SegmentedLog, WeightedMajority>;

enum Event {
    Message(Incoming),
    Tick,
    Eof,
}

fn main() {
    let (sender, receiver) = mpsc::channel();

    let reader = sender.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Incoming>(&line) {
                Ok(message) => {
                    if reader.send(Event::Message(message)).is_err() {
                        return;
                    }
                }
                Err(error) => eprintln!("undecodable maelstrom line: {error}: {line}"),
            }
        }
        let _ = reader.send(Event::Eof);
    });

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            if sender.send(Event::Tick).is_err() {
                return;
            }
        }
    });

    let mut node = NodeRunner::default();
    while let Ok(event) = receiver.recv() {
        match event {
            Event::Message(message) => node.on_message(message),
            Event::Tick => node.on_tick(),
            Event::Eof => return,
        }
    }
}

/// Where the answer to an in-flight operation has to go.
#[derive(Debug, Clone)]
enum Waiter {
    /// The client asked this node directly.
    Client { node: String, msg_id: u64 },
    /// A peer forwarded the op here because this node is the primary. The
    /// reply goes back the way it came, so every Maelstrom request is
    /// answered by the node the client actually addressed.
    Forwarded {
        via: String,
        client: String,
        client_msg_id: u64,
    },
}

/// A membership verb proposed and accepted, whose answer still waits: the
/// establishing operation commits only through a quorum, and the era folds
/// exactly there (§8.7.1). The request names the successor era it watches
/// and the slot its establishing operation accepted; the reply leaves when
/// that era's record names that slot.
struct PendingMembership {
    /// The era the establishing operation establishes once it commits.
    watch_era: Era,
    /// The slot the proposal accepted.
    slot: Slot,
    /// Where the answer goes.
    waiter: Waiter,
    /// The reply body's type: the verb's `*_ok`.
    reply_kind: String,
}

#[derive(Default)]
struct NodeRunner {
    id: String,
    members: Vec<String>,
    replica: Option<Node>,
    kv: Kv,
    next_msg_id: u64,
    /// The host clock (S4): bumped once per driven input, carried as
    /// `TimedInput::at`.
    tick: u64,
    /// `(client_id, request_num)` of an in-flight op -> where its answer
    /// goes. The operation identity the core carries opaque (§11.1) is
    /// exactly this pair, so an `Effect::Apply` correlates back to the
    /// Maelstrom client that is waiting.
    waiting: HashMap<(u64, u64), Waiter>,
    /// Membership verbs proposed but not yet committed: the establishing
    /// operation's slot and the era to watch for its fold.
    pending: Vec<PendingMembership>,
}

impl NodeRunner {
    fn on_message(&mut self, message: Incoming) {
        match message.kind() {
            "init" => self.on_init(&message),
            "vrr" => self.on_peer(&message),
            "proxy" => self.on_proxy(&message),
            "proxy_reply" => self.on_proxy_reply(&message),
            "read" | "write" | "cas" => self.on_client(&message),
            "join" | "promote" | "demote" | "leave" => self.on_membership(&message),
            other => eprintln!("ignoring unsupported message type {other}"),
        }
        self.flush_membership();
    }

    fn on_init(&mut self, message: &Incoming) {
        let node_id = message
            .field("node_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut members: Vec<String> = message
            .field("node_ids")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        // The genesis order is the sorted membership; the node's own
        // `NodeId` is its position in that array.
        members.sort();

        let knobs = ViewChangeKnobs {
            primary_timeout: PRIMARY_TIMEOUT_TICKS,
            // The §13.1 suffix budget: unbounded. The histories this harness
            // replicates are small, and the budget is a host policy knob (W5),
            // never a correctness input.
            view_change_budget: usize::MAX,
        };
        let built = members
            .iter()
            .position(|member| member == &node_id)
            .and_then(|index| {
                let own = NodeId(u32::try_from(index).ok()?);
                let genesis_order = (0..members.len())
                    .map(|i| u32::try_from(i).map(NodeId).ok())
                    .collect::<Option<Vec<_>>>()?;
                Some((own, genesis_order))
            })
            .and_then(|(own, genesis_order)| {
                // Always `provision`, never `reopen`: this host persists
                // nothing, and the fenced `Joining` start is the honest
                // statement of that (see the module docs).
                Node::provision(
                    own,
                    genesis_order,
                    WeightedMajority,
                    SegmentedLog::new(),
                    Stability::Volatile,
                    knobs,
                )
                .ok()
            });

        match built {
            Some(replica) => {
                self.id = node_id;
                self.members = members;
                self.replica = Some(replica);
            }
            None => {
                eprintln!("cannot provision replica for {node_id}");
                if let Some(msg_id) = message.msg_id() {
                    self.reply(
                        &message.src,
                        msg_id,
                        serde_json::json!({
                            "type": "error",
                            "code": error::TEMPORARILY_UNAVAILABLE,
                            "text": "membership rejected",
                        }),
                    );
                }
                return;
            }
        }

        if let Some(msg_id) = message.msg_id() {
            self.reply(&message.src, msg_id, serde_json::json!({"type": "init_ok"}));
        }
    }

    fn on_peer(&mut self, message: &Incoming) {
        let Some(from) = self.index_of(&message.src) else {
            eprintln!("peer message from unknown node {}", message.src);
            return;
        };
        let Some(wire) = message
            .field("wire")
            .and_then(Value::as_str)
            .and_then(from_hex)
        else {
            eprintln!("peer message without a decodable wire field");
            return;
        };
        let decoded = match Message::unpack_from(&wire) {
            Ok(decoded) => decoded,
            Err(error) => {
                eprintln!("peer message failed the VRR codec: {error:?}");
                return;
            }
        };
        self.drive(Input::Peer {
            from,
            message: decoded,
        });
    }

    fn on_client(&mut self, message: &Incoming) {
        let Some(msg_id) = message.msg_id() else {
            eprintln!("client op without msg_id");
            return;
        };
        let Some(client_id) = client_number(&message.src) else {
            eprintln!("unparseable client id {}", message.src);
            return;
        };
        let waiter = Waiter::Client {
            node: message.src.clone(),
            msg_id,
        };
        // The client's own `msg_id` is per-client monotonic and identical at
        // every node, so it is the request number. A per-node counter would
        // diverge across replicas and stall a client after a primary change.
        self.submit(client_id, msg_id, &message.body, waiter);
    }

    /// A peer forwarded a client op here because it believes this node leads.
    fn on_proxy(&mut self, message: &Incoming) {
        let Some(client) = message.field("client").and_then(Value::as_str) else {
            eprintln!("proxy without a client");
            return;
        };
        let Some(client_msg_id) = message.field("client_msg_id").and_then(Value::as_u64) else {
            eprintln!("proxy without a client_msg_id");
            return;
        };
        let Some(op) = message.field("op").cloned() else {
            eprintln!("proxy without an op");
            return;
        };
        let Some(client_id) = client_number(client) else {
            eprintln!("proxy for unparseable client {client}");
            return;
        };
        let waiter = Waiter::Forwarded {
            via: message.src.clone(),
            client: client.to_string(),
            client_msg_id,
        };
        self.submit(client_id, client_msg_id, &op, waiter);
    }

    /// Submits one client operation, forwarding to the primary when this
    /// node does not lead. Without forwarding, Maelstrom's client picks a
    /// node at random and roughly `1 - 1/K` of all operations are refused
    /// outright, which starves the log and leaves the checker almost
    /// nothing to verify.
    fn submit(&mut self, client_id: u64, request_num: u64, op: &Value, waiter: Waiter) {
        let kind = op.get("type").and_then(Value::as_str).unwrap_or("");
        if matches!(kind, "join" | "promote" | "demote" | "leave") {
            return self.submit_membership(kind, op, waiter);
        }
        let key = op.get("key").cloned().unwrap_or(Value::Null);
        let request = match kind {
            "read" => KvRequest::Read {
                client_id,
                request_num,
                key,
            },
            "write" => KvRequest::Write {
                client_id,
                request_num,
                key,
                value: op.get("value").cloned().unwrap_or(Value::Null),
            },
            "cas" => KvRequest::Cas {
                client_id,
                request_num,
                key,
                from: op.get("from").cloned().unwrap_or(Value::Null),
                to: op.get("to").cloned().unwrap_or(Value::Null),
            },
            other => {
                eprintln!("unsupported client op {other}");
                return;
            }
        };

        if !self.leads() {
            // Forward once. A node that received a forward and still does
            // not lead refuses rather than bouncing it on, so a stale view
            // cannot start a forwarding loop.
            let primary = self
                .primary()
                .and_then(|id| self.members.get(id.0 as usize).cloned());
            match (&waiter, primary) {
                (Waiter::Client { node, msg_id }, Some(primary)) if primary != self.id => {
                    let body = serde_json::json!({
                        "type": "proxy",
                        "client": node,
                        "client_msg_id": msg_id,
                        "op": op,
                    });
                    // Deliberately no reply here: if the primary never
                    // answers, the operation is genuinely indeterminate, and
                    // claiming a definite failure would be a lie to the
                    // checker.
                    self.send(&primary, body);
                }
                _ => self.refuse(&waiter, "not the primary"),
            }
            return;
        }

        let payload = match serde_json::to_vec(&request) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("cannot encode kv request: {error}");
                return;
            }
        };
        self.waiting
            .insert((client_id, request_num), waiter.clone());
        // The operation identity is the (client, request) pair itself:
        // assigned by the host, carried opaque by the core (§11.1, B2), and
        // handed back at `Effect::Apply`, where it finds the waiter.
        let stepped = self.step(Input::Propose {
            operation: Operation {
                id: OperationId {
                    msb: client_id,
                    lsb: request_num,
                },
                payload: payload.into(),
            },
        });
        match stepped {
            Ok(effects) => self.route(effects),
            Err(rejection) => {
                // `plan` refused before anything appended: the operation
                // provably never entered the log, so a definite failure is
                // the honest answer.
                self.waiting.remove(&(client_id, request_num));
                eprintln!("proposal refused: {rejection:?}");
                self.refuse(&waiter, "request refused by primary");
            }
        }
    }

    /// One membership verb, addressed by a client. The verb targets a node
    /// by `node_id`; the answering path is the KV path's: the addressed
    /// node submits, a backup forwards once to the primary.
    fn on_membership(&mut self, message: &Incoming) {
        let Some(msg_id) = message.msg_id() else {
            eprintln!("membership verb without msg_id");
            return;
        };
        let waiter = Waiter::Client {
            node: message.src.clone(),
            msg_id,
        };
        self.submit_membership(message.kind(), &message.body, waiter);
    }

    /// Submits one membership verb, direct or forwarded. The host's own
    /// pre-gates run before the core is touched: a verb without a
    /// `node_id`, or naming an id outside the bench roster (fixed by
    /// `node_ids`), is a definite malformed request. At the primary the
    /// verb becomes `Input::Reconfigure` — `Join`/`Decrement`/`Leave` stop
    /// the world (`pivot: None`), `Increment` takes the §8.7.6 pivot when
    /// one exists for this leader. The reply is not the proposal's: it
    /// leaves at the era fold (§8.7.1), so a primary death or a view
    /// change that drops the uncommitted establishing operation leaves
    /// the request unanswered — the honest indeterminate. Every core
    /// refusal is answered as a named Maelstrom error carrying the
    /// refusal diagnostic, never a crash.
    fn submit_membership(&mut self, kind: &str, request: &Value, waiter: Waiter) {
        let Some(target) = request.get("node_id").and_then(Value::as_str) else {
            self.membership_error(
                &waiter,
                error::MALFORMED_REQUEST,
                "membership verb without a node_id".to_string(),
            );
            return;
        };
        let Some(node) = self.index_of(target) else {
            self.membership_error(
                &waiter,
                error::MALFORMED_REQUEST,
                format!("membership verb names {target}, which is not a bench node"),
            );
            return;
        };
        if !self.leads() {
            // Forward once. The same ruling as the KV path: a node that
            // received a forward and still does not lead refuses rather
            // than bouncing it on.
            let primary = self
                .primary()
                .and_then(|id| self.members.get(id.0 as usize).cloned());
            match (&waiter, primary) {
                (Waiter::Client { node, msg_id }, Some(primary)) if primary != self.id => {
                    let body = serde_json::json!({
                        "type": "proxy",
                        "client": node,
                        "client_msg_id": msg_id,
                        "op": request,
                    });
                    self.send(&primary, body);
                }
                _ => self.refuse(&waiter, "not the primary"),
            }
            return;
        }
        let replica = self.replica.as_ref().expect("leads() held a live replica");
        // A join appends a learner at the end of the succession sequence:
        // position <= len() is the fold's R11 precondition, and len() of
        // the current configuration is the only position that appends.
        let position = replica.progress().config().current().config.len();
        let operation = match kind {
            "join" => SystemOperation::Join { node, position },
            "promote" => SystemOperation::Increment(node),
            "demote" => SystemOperation::Decrement(node),
            "leave" => SystemOperation::Leave(node),
            _ => unreachable!("the dispatch match admits four membership kinds"),
        };
        let pivot = match &operation {
            SystemOperation::Increment(_) => self.pivot_for(&operation),
            _ => None,
        };
        let stepped = self.step(Input::Reconfigure {
            op: operation.clone(),
            pivot: pivot.clone(),
        });
        let stepped = match (&pivot, stepped) {
            (Some(_), Err(PlanRefusal::ReconfigureViewExhausted { .. })) => {
                // The named `v'` is not representable (§8.7.3 forbids
                // wraparound); the stop-the-world path names no `v'` —
                // retry without the pivot.
                self.step(Input::Reconfigure {
                    op: operation,
                    pivot: None,
                })
            }
            (_, stepped) => stepped,
        };
        match stepped {
            Ok(effects) => {
                self.route(effects);
                let replica = self
                    .replica
                    .as_ref()
                    .expect("an accepted proposal implies a live replica");
                let progress = replica.progress();
                let watch_era = progress
                    .current()
                    .era
                    .next()
                    .expect("an accepted establishing operation has a successor era");
                self.pending.push(PendingMembership {
                    watch_era,
                    slot: progress.accepted(),
                    waiter,
                    reply_kind: format!("{kind}_ok"),
                });
            }
            Err(rejection) => {
                eprintln!("reconfiguration refused: {rejection:?}");
                self.membership_error(
                    &waiter,
                    error::TEMPORARILY_UNAVAILABLE,
                    format!("reconfiguration refused: {rejection:?}"),
                );
            }
        }
    }

    /// The concrete pivot for this leader under the fold of `operation`,
    /// when one exists (§8.7.6). A leader with no legal split gets `None`:
    /// the stop-the-world fallback is a latency outcome, not an error.
    fn pivot_for(&self, operation: &SystemOperation) -> Option<vrr::replica::Pivot> {
        let replica = self.replica.as_ref()?;
        let progress = replica.progress();
        let current = progress.config().current();
        let slot = progress.accepted().next()?;
        let next = progress.config().extend(operation, slot).ok()?;
        construct_pivot(
            &WeightedMajority,
            &current.config,
            &next.current().config,
            replica.own(),
        )
    }

    /// A membership refusal: a named Maelstrom error carrying the core's
    /// refusal diagnostic, echoing the node's current configuration view
    /// — the same shape the KV path's definite failures answer with.
    fn membership_error(&mut self, waiter: &Waiter, code: u32, text: String) {
        let body = serde_json::json!({
            "type": "error",
            "code": code,
            "text": text,
            "config": self.config_view(),
        });
        self.answer(waiter, body);
    }

    /// Answers every membership request whose establishing operation has
    /// committed and folded (§8.7.1): the watched era's record names the
    /// proposal's slot. Runs after every driven input, because the commit
    /// cascade rides ordinary peer traffic and ticks. An establishing
    /// operation a view change replaced never establishes anything here —
    /// whether it committed elsewhere is unknowable from this node, so
    /// the request gets no answer and the pending entry is dropped.
    fn flush_membership(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let Some(table) = self
            .replica
            .as_ref()
            .map(|replica| Arc::clone(replica.progress().config()))
        else {
            self.pending.clear();
            return;
        };
        let view = self.config_view();
        let mut answered: Vec<(Waiter, String, Value)> = Vec::new();
        let mut settled: Vec<usize> = Vec::new();
        for (index, pending) in self.pending.iter().enumerate() {
            match table.record(pending.watch_era) {
                Some(record) if record.established_by == pending.slot => {
                    answered.push((
                        pending.waiter.clone(),
                        pending.reply_kind.clone(),
                        view.clone(),
                    ));
                    settled.push(index);
                }
                Some(_) => {
                    eprintln!("membership verb lost: its establishing operation never committed");
                    settled.push(index);
                }
                None => {}
            }
        }
        for index in settled.iter().rev() {
            self.pending.remove(*index);
        }
        for (waiter, reply_kind, config) in answered {
            let body = serde_json::json!({
                "type": reply_kind,
                "config": config,
            });
            self.answer(&waiter, body);
        }
    }

    /// The node's current configuration view (§8.7.1), read through the
    /// same public surface the KV path drives: the published progress and
    /// its era table's newest record — the member order in succession
    /// sequence, each with its weight, with the current view and era.
    fn config_view(&self) -> Value {
        let Some(replica) = self.replica.as_ref() else {
            return serde_json::json!({ "era": 0, "view_era": 0, "view": 0, "members": [] });
        };
        let progress = replica.progress();
        let current = progress.current();
        let established = progress.config().current();
        let members = established
            .config
            .order()
            .iter()
            .map(|member| {
                serde_json::json!({
                    "node": self.members.get(member.node.0 as usize),
                    "weight": member.weight.0,
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "era": established.era.0,
            "view_era": current.era.0,
            "view": current.view.0,
            "members": members,
        })
    }

    fn on_tick(&mut self) {
        let Some(_replica) = &self.replica else {
            return;
        };

        // The tick drives the genesis-primary bootstrap and the view-change
        // suspicion timeout (S4).
        self.drive(Input::Tick);
    }

    /// Whether this node is the `Normal` primary of its current view — the
    /// only node a client op may be proposed to (§4).
    fn leads(&self) -> bool {
        match &self.replica {
            Some(replica) => {
                replica.progress().status() == Status::Normal
                    && self.primary() == Some(replica.own())
            }
            None => false,
        }
    }

    /// The primary of the node's current view under its own configuration
    /// history, when the configuration can name one.
    fn primary(&self) -> Option<NodeId> {
        let replica = self.replica.as_ref()?;
        let current = replica.progress().current();
        replica
            .progress()
            .config()
            .record(current.era)?
            .config
            .primary(current.view)
    }

    /// Steps the replica and routes everything that falls out. Plan
    /// rejections here are protocol weather (a fenced node refusing a peer
    /// message, a duplicate acknowledgement), each already named by the
    /// core; the drop diagnostics live on the observation handle.
    fn drive(&mut self, input: Input) {
        if let Ok(effects) = self.step(input) {
            self.route(effects);
        }
    }

    /// One plan/publish interval (§7, §12): plan against the published
    /// state and a journal view, publish the planned transition, hand the
    /// released effects back. `Volatile` stability always publishes; the
    /// parked outcome exists for the external-stability modes this host
    /// never selects.
    fn step(&mut self, input: Input) -> Result<Vec<Effect>, PlanRefusal> {
        let Some(replica) = &mut self.replica else {
            return Ok(Vec::new());
        };
        self.tick += 1;
        let timed = TimedInput {
            at: Tick(self.tick),
            event: input,
        };
        let planned = replica.plan(&timed, &replica.journal().view())?;
        match replica.publish(planned) {
            Ok(PublishOutcome::Published { effects, .. }) => Ok(effects),
            Ok(PublishOutcome::Parked { .. }) => {
                eprintln!("volatile stability never parks a transition");
                Ok(Vec::new())
            }
            Err(rejection) => {
                eprintln!("publish refused a planned transition: {rejection:?}");
                Ok(Vec::new())
            }
        }
    }

    /// Executes released effects, including the `Applied` feedback each
    /// `Apply` cascades into (§11.1): the acknowledgement carries no
    /// result — the boundary is one-way, and the core never answers a
    /// proposal (B2) — so the waiting client is answered here, at apply
    /// time, by the host.
    fn route(&mut self, effects: Vec<Effect>) {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            match effect {
                Effect::Send { to, message, .. } => {
                    // The effect's route era is a transport-visible fact
                    // (W1) that matters under overlap mode; this cluster
                    // runs a single era, so routing needs only the
                    // recipient.
                    if let Some(peer) = self.members.get(to.0 as usize).cloned() {
                        self.send_peer(&peer, &message);
                    }
                }
                Effect::Apply {
                    slot,
                    operation_id,
                    payload,
                } => {
                    // Every replica executes; only the node holding the
                    // waiter answers. Replay after a recovery restore is
                    // safe for this service (see kv.rs), and finds no
                    // waiter.
                    let response = self.execute(operation_id, &payload);
                    if let Some(waiter) = self.waiting.remove(&(operation_id.msb, operation_id.lsb))
                    {
                        self.answer(&waiter, response.body());
                    }
                    match self.step(Input::Applied { slot }) {
                        Ok(more) => {
                            for effect in more.into_iter().rev() {
                                pending.push_front(effect);
                            }
                        }
                        Err(rejection) => {
                            eprintln!("applied acknowledgement refused: {rejection:?}")
                        }
                    }
                }
                Effect::Persist(_) => {
                    eprintln!("volatile stability releases no persistence intents")
                }
                Effect::AdminResponse { .. } => {
                    eprintln!("the key-value node receives no admin submissions")
                }
            }
        }
    }

    /// Runs one committed entry against the replicated service. The
    /// envelope is cross-checked against the payload, so a mismatched
    /// replication envelope cannot execute silently.
    fn execute(&mut self, operation_id: OperationId, payload: &[u8]) -> KvResponse {
        let (client_id, request_num) = (operation_id.msb, operation_id.lsb);
        match serde_json::from_slice::<KvRequest>(payload) {
            Ok(request) if request.ids() == (client_id, request_num) => self.kv.execute(&request),
            Ok(_) => KvResponse::Failed {
                client_id,
                request_num,
                code: error::TEMPORARILY_UNAVAILABLE,
                text: "replication envelope does not match payload".into(),
            },
            Err(error) => KvResponse::Failed {
                client_id,
                request_num,
                code: error::TEMPORARILY_UNAVAILABLE,
                text: format!("undecodable payload: {error}"),
            },
        }
    }

    /// Routes one answer to whoever is waiting, hopping back through the
    /// forwarding node when the op arrived by proxy.
    fn answer(&mut self, waiter: &Waiter, body: Value) {
        match waiter {
            Waiter::Client { node, msg_id } => self.reply(&node.clone(), *msg_id, body),
            Waiter::Forwarded {
                via,
                client,
                client_msg_id,
            } => {
                let body = serde_json::json!({
                    "type": "proxy_reply",
                    "client": client,
                    "client_msg_id": client_msg_id,
                    "body": body,
                });
                self.send(&via.clone(), body);
            }
        }
    }

    /// The primary answered a forwarded op; relay it to the client that
    /// this node originally accepted the request from.
    fn on_proxy_reply(&mut self, message: &Incoming) {
        let Some(client) = message
            .field("client")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            eprintln!("proxy_reply without a client");
            return;
        };
        let Some(client_msg_id) = message.field("client_msg_id").and_then(Value::as_u64) else {
            eprintln!("proxy_reply without a client_msg_id");
            return;
        };
        let Some(body) = message.field("body").cloned() else {
            eprintln!("proxy_reply without a body");
            return;
        };
        self.reply(&client, client_msg_id, body);
    }

    /// A definite failure: the operation provably never entered the log.
    fn refuse(&mut self, waiter: &Waiter, text: &str) {
        let body = serde_json::json!({
            "type": "error",
            "code": error::TEMPORARILY_UNAVAILABLE,
            "text": text,
        });
        self.answer(waiter, body);
    }

    fn send_peer(&mut self, peer: &str, message: &Message) {
        let mut bytes = vec![0u8; message.packed_len()];
        match message.pack_into(&mut bytes) {
            Ok(_) => {
                let body = serde_json::json!({"type": "vrr", "wire": to_hex(&bytes)});
                self.send(peer, body);
            }
            Err(error) => eprintln!("cannot encode VRR message: {error:?}"),
        }
    }

    fn reply(&mut self, dest: &str, in_reply_to: u64, mut body: Value) {
        if let Value::Object(fields) = &mut body {
            fields.insert("in_reply_to".into(), Value::from(in_reply_to));
        }
        self.send(dest, body);
    }

    fn send(&mut self, dest: &str, mut body: Value) {
        self.next_msg_id += 1;
        if let Value::Object(fields) = &mut body {
            fields.insert("msg_id".into(), Value::from(self.next_msg_id));
        }
        let envelope = Outgoing {
            src: self.id.clone(),
            dest: dest.to_string(),
            body,
        };
        match serde_json::to_string(&envelope) {
            Ok(line) => {
                let mut stdout = std::io::stdout().lock();
                if writeln!(stdout, "{line}").is_err() {
                    return;
                }
                let _ = stdout.flush();
            }
            Err(error) => eprintln!("cannot encode outgoing envelope: {error}"),
        }
    }

    fn index_of(&self, node: &str) -> Option<NodeId> {
        self.members
            .iter()
            .position(|member| member == node)
            .and_then(|index| u32::try_from(index).ok())
            .map(NodeId)
    }
}

/// `c7` -> 7. Maelstrom client node IDs are `c` followed by an integer.
fn client_number(src: &str) -> Option<u64> {
    src.strip_prefix('c')?.parse().ok()
}
