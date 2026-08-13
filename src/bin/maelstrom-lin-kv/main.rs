//! Maelstrom node that runs the unmodified VRR replication core against the
//! `lin-kv` workload, so Knossos can check the protocol for linearizability
//! under Maelstrom's partition / kill / pause nemeses.
//!
//! The core is used as a Rust library (`vrr::vrr::Replica`), not through the C
//! ABI. That matters: `MAX_DATAGRAM` and the advisory-lock service both live in
//! `ffi.rs`, so linking the library directly needs **no change whatsoever** to
//! the code under test — `Replica` already treats client payloads as opaque
//! bytes. The FFI and Teal layers are a later phase of this harness.
//!
//! Architecture: two producer threads (stdin reader, ticker) feed one channel;
//! a single core thread owns all state and is the only writer to stdout, so
//! there is no lock around the replica.

mod kv;
mod proto;

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;
use vrr::vrr::{Input, Message, NodeId, Output, Replica, Status};

use crate::kv::Kv;
use crate::proto::{Incoming, KvRequest, KvResponse, Outgoing, error, from_hex, to_hex};

/// Scheduler granularity. Everything below is expressed in ticks.
const TICK: Duration = Duration::from_millis(100);
/// Leader heartbeat period, in ticks. Must be well under the election floor.
const HEARTBEAT_TICKS: u32 = 2;
/// Election timeout floor, in ticks, plus a per-node stagger so replicas do
/// not all time out on the same tick and trade views forever.
const ELECTION_FLOOR_TICKS: u32 = 12;
const ELECTION_STAGGER_TICKS: u32 = 5;
/// A recovery attempt that collects no quorum is retried with a fresh nonce.
/// The protocol requires the host to keep retrying; nothing re-broadcasts
/// `RECOVERY` on its own.
const RECOVERY_RETRY_TICKS: u32 = 25;

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
    /// A peer forwarded the op here because this node is the leader. The reply
    /// goes back the way it came, so every Maelstrom request is answered by the
    /// node the client actually addressed.
    Forwarded {
        via: String,
        client: String,
        client_msg_id: u64,
    },
}

#[derive(Default)]
struct NodeRunner {
    id: String,
    members: Vec<String>,
    replica: Option<Replica>,
    kv: Kv,
    next_msg_id: u64,
    /// `(client_id, request_num)` of an in-flight op -> where its answer goes.
    /// `Output::Reply` carries only bytes, so the correlation keys live inside
    /// the replicated payload.
    waiting: HashMap<(u64, u64), Waiter>,
    ticks_since_leader: u32,
    ticks_since_heartbeat: u32,
    ticks_recovering: u32,
}

impl NodeRunner {
    fn on_message(&mut self, message: Incoming) {
        match message.kind() {
            "init" => self.on_init(&message),
            "vrr" => self.on_peer(&message),
            "proxy" => self.on_proxy(&message),
            "proxy_reply" => self.on_proxy_reply(&message),
            "read" | "write" | "cas" => self.on_client(&message),
            other => eprintln!("ignoring unsupported message type {other}"),
        }
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
        // `Replica::new` requires a strictly sorted membership; the node's own
        // ID is located by position in that array.
        members.sort();

        match Replica::new(members.clone(), &node_id) {
            Ok(replica) => {
                self.id = node_id;
                self.members = members;
                self.replica = Some(replica);
            }
            Err(reason) => {
                eprintln!("cannot build replica: {reason}");
                if let Some(msg_id) = message.msg_id() {
                    self.reply(
                        &message.src,
                        msg_id,
                        serde_json::json!({
                            "type": "error",
                            "code": error::TEMPORARILY_UNAVAILABLE,
                            "text": format!("membership rejected: {reason}"),
                        }),
                    );
                }
                return;
            }
        }

        if let Some(msg_id) = message.msg_id() {
            self.reply(&message.src, msg_id, serde_json::json!({"type": "init_ok"}));
        }

        // A restarted process cannot tell itself apart from a first boot, and
        // the protocol requires a restarting replica to recover before it
        // participates. The durable marker is exactly the "durable monotonic
        // state" the recovery contract asks the host for. Without it, either
        // every node recovers at boot (nobody is Normal to answer, so the
        // cluster deadlocks) or a restarted amnesiac replica rejoins and
        // acknowledges entries it never held.
        let nonce = self.bump_persisted_nonce();
        if let Some(nonce) = nonce {
            eprintln!("restart detected; recovering with nonce {nonce}");
            self.drive(Input::Recover { nonce });
        }
    }

    /// Returns `Some(nonce)` when a marker already existed, meaning this is a
    /// restart. Always leaves a marker behind with a strictly greater counter.
    fn bump_persisted_nonce(&self) -> Option<u64> {
        let path = self.nonce_path();
        let previous = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok());
        let next = previous.map_or(1, |value| value.saturating_add(1));
        if let Err(error) = std::fs::write(&path, next.to_string()) {
            eprintln!(
                "cannot persist recovery nonce at {}: {error}",
                path.display()
            );
        }
        previous.map(|_| next)
    }

    fn nonce_path(&self) -> PathBuf {
        let dir = std::env::var_os("MAELSTROM_VRR_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        dir.join(format!("maelstrom-vrr-{}.nonce", self.id))
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
        let Some(decoded) = Message::decode(&wire) else {
            eprintln!("peer message failed the VRR codec");
            return;
        };

        // Any traffic from the current view's leader is evidence the leader
        // is alive, which is what suppresses this replica's election timeout.
        if let Some(replica) = &self.replica {
            if from == replica.leader_of(replica.view()) {
                self.ticks_since_leader = 0;
            }
        }
        self.drive(Input::Message {
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
        // diverge across replicas and stall a client after a leader change.
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

    /// Submits one client operation, forwarding to the leader when this node
    /// does not lead. Without forwarding, Maelstrom's client picks a node at
    /// random and roughly `1 - 1/K` of all operations are refused outright,
    /// which starves the log and leaves the checker almost nothing to verify.
    fn submit(&mut self, client_id: u64, request_num: u64, op: &Value, waiter: Waiter) {
        let kind = op.get("type").and_then(Value::as_str).unwrap_or("");
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

        let Some(replica) = &self.replica else {
            self.refuse(&waiter, "node not initialised");
            return;
        };

        if replica.status() != Status::Normal || !replica.is_leader() {
            // Forward once. A node that received a forward and still does not
            // lead refuses rather than bouncing it on, so a stale view cannot
            // start a forwarding loop.
            let leader_index = replica.leader_of(replica.view());
            let leader = self.members.get(leader_index as usize).cloned();
            match (&waiter, leader) {
                (Waiter::Client { node, msg_id }, Some(leader)) if leader != self.id => {
                    let body = serde_json::json!({
                        "type": "proxy",
                        "client": node,
                        "client_msg_id": msg_id,
                        "op": op,
                    });
                    // Deliberately no reply here: if the leader never answers,
                    // the operation is genuinely indeterminate, and claiming a
                    // definite failure would be a lie to the checker.
                    self.send(&leader, body);
                }
                _ => self.refuse(&waiter, "not the leader"),
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
        let outputs = self.step(Input::Request {
            client_id,
            request_num,
            // Correlation only; the core does not deduplicate on it.
            message_id: Uuid::from_u128(u128::from(client_id) << 64 | u128::from(request_num)),
            // The predicted execution value. This service never reads a clock,
            // so any deterministic value replicates correctly.
            execution_time: 0,
            payload,
        });
        if outputs.is_empty() {
            // The leader refused without appending: a stale request number, or
            // a duplicate still pending. Definitely not in the log.
            self.waiting.remove(&(client_id, request_num));
            self.refuse(&waiter, "request refused by leader");
            return;
        }
        self.route(outputs);
    }

    fn on_tick(&mut self) {
        let Some(replica) = &self.replica else { return };
        let status = replica.status();
        let leader = replica.is_leader();

        self.ticks_since_leader = self.ticks_since_leader.saturating_add(1);
        self.ticks_since_heartbeat = self.ticks_since_heartbeat.saturating_add(1);

        if status == Status::Recovering {
            self.ticks_recovering = self.ticks_recovering.saturating_add(1);
            if self.ticks_recovering >= RECOVERY_RETRY_TICKS {
                self.ticks_recovering = 0;
                if let Some(nonce) = self.bump_persisted_nonce() {
                    eprintln!("recovery stalled; retrying with nonce {nonce}");
                    self.drive(Input::Recover { nonce });
                }
            }
            return;
        }
        self.ticks_recovering = 0;

        // Replaying replicas are still isolated; leave them to finish.
        if status == Status::Replaying {
            return;
        }

        if leader && status == Status::Normal {
            if self.ticks_since_heartbeat >= HEARTBEAT_TICKS {
                self.ticks_since_heartbeat = 0;
                self.drive(Input::Idle);
            }
            self.ticks_since_leader = 0;
            return;
        }

        if self.ticks_since_leader >= self.election_timeout() {
            self.ticks_since_leader = 0;
            self.drive(Input::LeaderTimeout);
        }
    }

    fn election_timeout(&self) -> u32 {
        let index = self.index_of(&self.id).unwrap_or(0);
        ELECTION_FLOOR_TICKS + ELECTION_STAGGER_TICKS * index
    }

    /// Steps the replica and routes everything that falls out, including the
    /// service executions the outputs cascade into.
    fn drive(&mut self, input: Input) {
        let outputs = self.step(input);
        self.route(outputs);
    }

    fn step(&mut self, input: Input) -> Vec<Output> {
        match &mut self.replica {
            Some(replica) => replica.step(input),
            None => Vec::new(),
        }
    }

    fn route(&mut self, outputs: Vec<Output>) {
        let mut pending = std::collections::VecDeque::from(outputs);
        while let Some(output) = pending.pop_front() {
            match output {
                Output::Broadcast(message) => {
                    for peer in self.members.clone() {
                        if peer != self.id {
                            self.send_peer(&peer, &message);
                        }
                    }
                }
                Output::To(to, message) => {
                    if let Some(peer) = self.members.get(to as usize).cloned() {
                        self.send_peer(&peer, &message);
                    }
                }
                Output::Execute {
                    slot,
                    payload,
                    client_id,
                    request_num,
                    ..
                } => {
                    let result = self.execute(client_id, request_num, &payload);
                    for output in self
                        .step(Input::Complete { slot, result })
                        .into_iter()
                        .rev()
                    {
                        pending.push_front(output);
                    }
                }
                Output::Reply(bytes) => self.answer_client(&bytes),
            }
        }
    }

    /// Runs one committed entry against the replicated service. The envelope
    /// is cross-checked against the payload, mirroring what `locks::Service`
    /// does, so a mismatched replication envelope cannot execute silently.
    fn execute(&mut self, client_id: u64, request_num: u64, payload: &[u8]) -> Vec<u8> {
        let response = match serde_json::from_slice::<KvRequest>(payload) {
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
        };
        serde_json::to_vec(&response).unwrap_or_default()
    }

    fn answer_client(&mut self, bytes: &[u8]) {
        let Ok(response) = serde_json::from_slice::<KvResponse>(bytes) else {
            eprintln!("undecodable replicated result");
            return;
        };
        let Some(waiter) = self.waiting.remove(&response.ids()) else {
            // A reply for an op nobody here is waiting on — for example a
            // cached result replayed after a leader change.
            return;
        };
        self.answer(&waiter, response.body());
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

    /// The leader answered a forwarded op; relay it to the client that this
    /// node originally accepted the request from.
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
        match message.encode() {
            Ok(bytes) => {
                let body = serde_json::json!({"type": "vrr", "wire": to_hex(&bytes)});
                self.send(peer, body);
            }
            Err(error) => eprintln!("cannot encode VRR message: {error}"),
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
            .map(|index| index as NodeId)
    }
}

/// `c7` -> 7. Maelstrom client node IDs are `c` followed by an integer.
fn client_number(src: &str) -> Option<u64> {
    src.strip_prefix('c')?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_arbitrary_bytes() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        assert_eq!(from_hex(&to_hex(&bytes)), Some(bytes));
        assert_eq!(from_hex("abc"), None, "odd length must be rejected");
        assert_eq!(from_hex("zz"), None, "non-hex digits must be rejected");
    }

    #[test]
    fn client_ids_parse_from_maelstrom_node_names() {
        assert_eq!(client_number("c1"), Some(1));
        assert_eq!(client_number("c17"), Some(17));
        assert_eq!(client_number("n1"), None);
    }

    #[test]
    fn a_vrr_datagram_survives_the_hex_transport() {
        use vrr::vrr::Body;
        let message = Message {
            view: 0x0102_0304,
            slot: 0x0506_0708_090a_0b0c,
            body: Body::Recovery { nonce: 42 },
        };
        let wire = to_hex(&message.encode().expect("encodes"));
        let back = Message::decode(&from_hex(&wire).expect("decodes")).expect("valid");
        assert_eq!(back, message);
    }
}
