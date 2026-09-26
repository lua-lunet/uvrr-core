//! The compliance suite's executor: the abstract host interface of
//! `docs/uvrr-compliance.md`, as data over the reference harness. A case
//! is a setup script, one input, and the expected post-state; the
//! executor replays the script, applies the input, drains the network,
//! and captures the delivery sequence and the post records, so the
//! exporter and the runner share one execution path.

use serde::{Deserialize, Serialize};

use super::harness::{Harness, StepOutcome};
use vrr::ids::{CrashCounter, Era, NodeId, Operation, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Header, Message};
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Pack, Tag, Unpack};

/// The settle bound: the bootstrap converges long before this, and a
/// script that cannot settle inside it is a corpus defect, not a slow
/// machine, so the executor panics rather than spinning.
const SETTLE_BOUND: usize = 1_000;

/// The drain bound: the post-input cascade is finite and deterministic.
const DRAIN_BOUND: usize = 1_000;

/// The vector operations' fixed identity half: part of the fixture
/// (`docs/uvrr-compliance.md` §4), never zero, never privileged.
const OP_MSB: u64 = 0x7665_6374;

// ----------------------------------------------------------------------
// The case record
// ----------------------------------------------------------------------

/// One abstract host operation, the sans-I/O boundary mirrored as data.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Build a genesis cluster; MUST be the setup's first operation.
    Provision {
        /// The roster size.
        nodes: usize,
        /// The primary-timeout knob, in host ticks.
        timeout: u64,
    },
    /// The deterministic bootstrap: tick and deliver until quiet and
    /// every live node is `Normal`.
    Settle,
    /// Submit one operation for ordering.
    Propose {
        /// The proposing node's roster index.
        node: usize,
        /// The opaque payload bytes, as UTF-8 text.
        payload: String,
    },
    /// One host timer event.
    Tick { node: usize },
    /// One timer event to every live node.
    TickAll,
    /// Deliver every queued datagram once.
    DeliverAll,
    /// Kill a node: volatile state lost, markers as they are.
    Crash { node: usize },
    /// Reopen over the recorded disk.
    Restart {
        node: usize,
        /// `clean` continues the identity; `crashed` bumps one life.
        kind: String,
    },
    /// The controlled shutdown: both marker rounds with the drain.
    Halt { node: usize },
    /// An outside identity boots over the deployment's genesis
    /// knowledge, at the roster index one past the end.
    Boot { node: usize },
    /// Deliver one wire datagram.
    Deliver {
        to: usize,
        from: usize,
        /// The datagram, hex-encoded.
        wire: String,
    },
    /// Deliver one wire datagram to every live node from the outside
    /// gossip sender (the roster index one past the end).
    Gossip { wire: String },
}

/// One expected delivery in the post-input drain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedDelivery {
    pub from: usize,
    pub to: usize,
    pub wire: String,
}

/// One node's post-state. Absent fields are unconstrained.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostNode {
    pub node: usize,
    pub status: Option<String>,
    pub accepted: Option<u64>,
    pub committed: Option<u64>,
    pub applied: Option<u64>,
    pub journal: Option<Vec<String>>,
    pub gate_ops: Option<Vec<String>>,
    pub witnesses: Option<Vec<u64>>,
}

/// The expectation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expect {
    /// The exact delivery sequence of the post-input drain.
    #[serde(default)]
    pub deliveries: Vec<ExpectedDelivery>,
    #[serde(default)]
    pub post: Vec<PostNode>,
}

/// One compliance case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub family: String,
    pub clause: String,
    pub keywords: Vec<String>,
    pub setup: Vec<Op>,
    pub input: Op,
    pub expect: Expect,
}

// ----------------------------------------------------------------------
// Hex and wire
// ----------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize]);
        out.push(HEX[(byte & 0xf) as usize]);
    }
    out
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err("odd hex length".into());
    }
    let value = |b: u8| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err("not hex".into()),
    };
    (0..bytes.len() / 2)
        .map(|i| Ok((value(bytes[i * 2])? << 4) | value(bytes[i * 2 + 1])?))
        .collect()
}

/// Packs a message to its wire hex.
pub fn wire_hex(message: &Message) -> String {
    let mut buf = vec![0u8; message.packed_len()];
    let len = message.pack_into(&mut buf).expect("the codec packs");
    buf.truncate(len);
    hex_encode(&buf)
}

/// Unpacks a message from its wire hex.
pub fn from_wire_hex(text: &str) -> Result<Message, String> {
    let bytes = hex_decode(text)?;
    Message::unpack_from(&bytes).map_err(|e| format!("the wire bytes do not decode: {e:?}"))
}

// ----------------------------------------------------------------------
// The executor
// ----------------------------------------------------------------------

/// The abstract host interface over one harness.
pub struct Executor {
    h: Harness,
    /// The roster size; the index one past the end names the outside
    /// gossip sender.
    nodes: usize,
    /// The vector operation sequence, one past the last proposal.
    op_seq: u64,
}

impl Executor {
    /// `provision`, the setup's mandatory first operation.
    pub fn provision(nodes: usize, timeout: u64) -> Executor {
        let knobs = ViewChangeKnobs {
            primary_timeout: timeout,
            view_change_budget: usize::MAX,
        };
        Executor {
            h: Harness::with_knobs(nodes, knobs),
            nodes,
            op_seq: 1,
        }
    }

    /// The harness, for the exporter's state inspection.
    pub fn harness(&mut self) -> &mut Harness {
        &mut self.h
    }

    /// The identity a roster index names: position + 1 as the system
    /// half, one as the counter half; the index one past the end is the
    /// outside gossip sender.
    #[must_use]
    pub fn id_of(&self, index: usize) -> NodeId {
        let system =
            SystemId::new(u16::try_from(index + 1).expect("the index fits the system space"))
                .expect("a one-indexed system id is non-zero");
        NodeId::new(system, CrashCounter::new(1).expect("one is non-zero"))
    }

    /// The roster index an identity names, by its system half.
    #[must_use]
    fn index_of(&self, id: NodeId) -> usize {
        usize::from(id.system_id().expect("a live identity is lawful").get()) - 1
    }

    /// Applies one operation.
    pub fn apply(&mut self, op: &Op) -> Result<(), String> {
        match op {
            Op::Provision { nodes, timeout } => {
                let _ = (nodes, timeout);
                Err("provision is the setup's first operation, built by the executor".into())
            }
            Op::Settle => {
                for _ in 0..SETTLE_BOUND {
                    if self.quiet_and_normal() {
                        return Ok(());
                    }
                    self.h.tick_all();
                    self.h.deliver_all();
                }
                Err("settle did not converge inside the bound".into())
            }
            Op::Propose { node, payload } => {
                let id = OperationId {
                    msb: OP_MSB,
                    lsb: self.op_seq,
                };
                self.op_seq += 1;
                self.h
                    .propose(self.id_of(*node), id, payload.as_bytes());
                Ok(())
            }
            Op::Tick { node } => {
                self.h.tick(self.id_of(*node));
                Ok(())
            }
            Op::TickAll => {
                self.h.tick_all();
                Ok(())
            }
            Op::DeliverAll => {
                self.h.deliver_all();
                Ok(())
            }
            Op::Crash { node } => {
                self.h.crash(self.id_of(*node));
                Ok(())
            }
            Op::Restart { node, kind } => {
                let id = self.id_of(*node);
                match kind.as_str() {
                    "clean" => self.h.restart_with(id).map_err(|e| format!("{e:?}")),
                    "crashed" => {
                        let next = id
                            .next_life()
                            .ok_or("the identity space is spent")?;
                        self.h.restart_as(id, next).map_err(|e| format!("{e:?}"))
                    }
                    other => Err(format!("no such restart kind: {other}")),
                }
            }
            Op::Halt { node } => {
                self.h.halt(self.id_of(*node));
                Ok(())
            }
            Op::Boot { node } => {
                self.h
                    .boot_as(self.id_of(*node))
                    .map_err(|e| format!("{e:?}"))
            }
            Op::Deliver { to, from, wire } => {
                let message = from_wire_hex(wire)?;
                let outcome = self.h.inject(self.id_of(*from), self.id_of(*to), message);
                if matches!(outcome, StepOutcome::NodeDown) {
                    return Err("the receiver is down".into());
                }
                Ok(())
            }
            Op::Gossip { wire } => {
                let message = from_wire_hex(wire)?;
                let sender = self.id_of(self.nodes);
                for index in 0..self.nodes {
                    let id = self.id_of(index);
                    if self.h.is_up(id) {
                        self.h.inject(sender, id, message.clone());
                    }
                }
                Ok(())
            }
        }
    }

    fn quiet_and_normal(&self) -> bool {
        if self.h.queued_len() != 0 {
            return false;
        }
        (0..self.nodes)
            .filter(|&i| self.h.is_up(self.id_of(i)))
            .all(|i| {
                self.h
                    .snapshot(self.id_of(i))
                    .is_some_and(|s| s.status == vrr::progress::Status::Normal.to_word())
            })
    }

    /// The post-input drain: delivers until quiet, recording the exact
    /// delivery sequence. No timer event fires in the drain.
    pub fn drain(&mut self) -> Vec<ExpectedDelivery> {
        let mut out = Vec::new();
        for _ in 0..DRAIN_BOUND {
            let queue = self.h.queued_envelopes();
            let Some((from, to, message)) = queue.first().cloned() else {
                return out;
            };
            out.push(ExpectedDelivery {
                from: self.index_of(from),
                to: self.index_of(to),
                wire: wire_hex(&message),
            });
            self.h.deliver_next();
        }
        panic!("the drain did not quiet inside the bound");
    }

    /// One node's full post record, every field captured.
    #[must_use]
    pub fn post_of(&self, index: usize) -> PostNode {
        let id = self.id_of(index);
        let snapshot = self.h.snapshot(id);
        let status = snapshot.map(|s| {
            vrr::progress::Status::from_word(s.status)
                .expect("the word decodes")
                .name()
                .to_string()
        });
        PostNode {
            node: index,
            status,
            accepted: snapshot.map(|s| s.accepted),
            committed: snapshot.map(|s| s.committed),
            applied: snapshot.map(|s| s.applied),
            journal: Some(
                self.h
                    .journal_entries(id)
                    .into_iter()
                    .map(|entry| render_payload(&entry))
                    .collect(),
            ),
            gate_ops: Some(self.h.gate_ops(id)),
            witnesses: Some(self.h.witnesses(id).into_iter().map(|w| w.0).collect()),
        }
    }

    /// Runs a setup script and leaves the executor open for inspection:
    /// the exporter probes the assembled state to build the input.
    pub fn run_setup(setup: &[Op]) -> Result<Executor, String> {
        let Some(Op::Provision { nodes, timeout }) = setup.first() else {
            return Err("the setup must open with provision".into());
        };
        let mut ex = Executor::provision(*nodes, *timeout);
        for op in &setup[1..] {
            ex.apply(op)
                .map_err(|e| format!("the setup's {:?} refused: {e}", op))?;
        }
        Ok(ex)
    }

    /// Runs a case's setup and input, then drains and captures the full
    /// expectation. The exporter's capture path; the runner replays the
    /// same code.
    pub fn run_case(case: &Case) -> Result<Expect, String> {
        let Some(Op::Provision { nodes, timeout }) = case.setup.first() else {
            return Err("the setup must open with provision".into());
        };
        let mut ex = Executor::provision(*nodes, *timeout);
        for op in &case.setup[1..] {
            ex.apply(op)
                .map_err(|e| format!("the setup's {:?} refused: {e}", op))?;
        }
        ex.apply(&case.input)
            .map_err(|e| format!("the input {:?} refused: {e}", case.input))?;
        let deliveries = ex.drain();
        let post = (0..*nodes).map(|i| ex.post_of(i)).collect();
        Ok(Expect { deliveries, post })
    }
}

/// Renders a journal entry's payload as corpus text.
fn render_payload(entry: &LogEntry) -> String {
    match &entry.payload {
        Payload::Operation { payload, .. } => {
            String::from_utf8_lossy(payload).into_owned()
        }
        Payload::System(op) => format!("{op:?}"),
    }
}

// ----------------------------------------------------------------------
// The assertion
// ----------------------------------------------------------------------

/// Asserts a captured expectation against a case's named expectation:
/// the delivery sequence exactly, the named post fields exactly.
pub fn assert_expectation(case: &Case, captured: &Expect) -> Result<(), String> {
    if captured.deliveries != case.expect.deliveries {
        return Err(format!(
            "the delivery sequence differs: expected {:?}, captured {:?}",
            case.expect.deliveries, captured.deliveries
        ));
    }
    for named in &case.expect.post {
        let Some(captured_post) = captured.post.get(named.node) else {
            return Err(format!("no post record for node {}", named.node));
        };
        let field = |a: &Option<String>, b: &Option<String>, name: &str| -> Option<String> {
            a.as_ref().zip(b.as_ref()).filter(|(x, y)| x != y).map(
               |(x, y)| format!("{name}: expected {y:?}, captured {x:?}"),
            )
        };
        if let Some(mismatch) = field(&captured_post.status, &named.status, "status")
            .or_else(|| field(&captured_post.accepted, &named.accepted, "accepted"))
            .or_else(|| field(&captured_post.committed, &named.committed, "committed"))
            .or_else(|| field(&captured_post.applied, &named.applied, "applied"))
            .or_else(|| field(&captured_post.journal, &named.journal, "journal"))
            .or_else(|| field(&captured_post.gate_ops, &named.gate_ops, "gate_ops"))
            .or_else(|| field(&captured_post.witnesses, &named.witnesses, "witnesses"))
        {
            return Err(mismatch);
        }
    }
    Ok(())
}

/// The builder's re-export for the exporter families.
pub fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// A `Prepare` message as the exporter builds it.
pub fn prepare(view: ViewId, slot: Slot, entry: LogEntry, committed: Slot) -> Message {
    Message {
        header: Header {
            tag: Tag::Prepare,
            view,
            slot,
        },
        body: Body::Prepare { entry, committed },
    }
}

/// A `GossipRequest` as the exporter builds it, at the era named.
pub fn gossip_request(era: Era, prepared: Slot, committed: Slot) -> Message {
    Message {
        header: Header {
            tag: Tag::GossipRequest,
            view: ViewId {
                era,
                view: View(0),
            },
            slot: Slot(0),
        },
        body: Body::GossipRequest { prepared, committed },
    }
}

/// The operation entry the exporter journals.
pub fn op_entry(slot: Slot, era: Era, seq: u64) -> LogEntry {
    LogEntry {
        slot,
        era,
        payload: Payload::Operation {
            id: OperationId {
                msb: OP_MSB,
                lsb: seq,
            },
            payload: b"proposed".as_slice().into(),
        },
    }
}

/// The operation payload the exporter proposes.
pub const PROPOSED: &str = "proposed";

/// A proposal operation for the exporter.
pub fn operation(seq: u64) -> Operation {
    Operation {
        id: OperationId {
            msb: OP_MSB,
            lsb: seq,
        },
        payload: PROPOSED.as_bytes().into(),
    }
}
