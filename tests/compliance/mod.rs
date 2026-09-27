//! The compliance suite's executor: the abstract host interface of
//! `docs/uvrr-compliance.md`, as data over the reference harness. A case
//! is a setup script, one input, and the expected post-state; the
//! executor replays the script, applies the input, drains the network,
//! and captures the delivery sequence and the post records, so the
//! exporter and the runner share one execution path.
//!
//! Nodes are addressed by the explicit pair `system:counter`
//! (`docs/uvrr-boot-gate.md` §5), one-indexed in both halves: the
//! roster members of a provisioned cluster are `1:1` through `nodes:1`,
//! a bumped life is `system:2` and onward, and the gossip sender is
//! `nodes+1:1`.

use serde::{Deserialize, Serialize};

use super::harness::{Harness, StepOutcome};
use uvrr::configuration::SystemOperation;
use uvrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use uvrr::journal::{LogEntry, Payload};
use uvrr::message::{Body, Message};
use uvrr::progress::Status;
use uvrr::replica::ViewChangeKnobs;
use uvrr::wire::{Header, Pack, Tag, Unpack};

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

/// One typed cluster operation, the `reconfigure` operation's argument,
/// in the corpus's abstract operation names (`docs/uvrr-compliance.md`
/// §3). Genesis operations (`void`, `init`) establish eras 0 and 1 and
/// are never reconfiguration arguments; they appear in journal content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SystemOp {
    Increment { node: String },
    Decrement { node: String },
    Double,
    Halve,
    Join { node: String, position: u32 },
    Leave { node: String },
    Nominate { from: u32, offset: u32 },
    Batch { ops: Vec<SystemOp> },
}

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
        /// The proposing node, as `system:counter`.
        node: String,
        /// The opaque payload bytes, as UTF-8 text.
        payload: String,
    },
    /// Submit one typed cluster operation for ordering, over the
    /// ordinary consensus pipeline.
    Reconfigure {
        /// The proposing node, as `system:counter`.
        node: String,
        /// The typed operation, in abstract operation names.
        system: SystemOp,
    },
    /// One host timer event.
    Tick { node: String },
    /// One timer event to every live node.
    TickAll,
    /// Deliver every queued datagram once.
    DeliverAll,
    /// Kill a node: volatile state lost, markers as they are.
    Crash { node: String },
    /// Reopen over the recorded disk.
    Restart {
        node: String,
        /// `clean` continues the identity; `crashed` bumps one life.
        kind: String,
    },
    /// The controlled shutdown: both marker rounds with the drain.
    Halt { node: String },
    /// An outside identity boots over the deployment's genesis
    /// knowledge, as a joining member.
    Boot { node: String },
    /// The reincarnated node announces its replacement pair.
    Announce { node: String, old: String },
    /// Deliver one wire datagram.
    Deliver {
        to: String,
        from: String,
        /// The datagram, hex-encoded.
        wire: String,
    },
    /// Deliver one wire datagram to every live node from the outside
    /// gossip sender, the roster's one-past-the-end identity.
    Gossip { wire: String },
}

/// One expected delivery in the post-input drain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedDelivery {
    pub from: String,
    pub to: String,
    pub wire: String,
}

/// One node's post-state. Absent fields are unconstrained.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostNode {
    /// The node's identity, as `system:counter`.
    pub node: String,
    pub status: Option<String>,
    pub era: Option<u32>,
    pub view: Option<u32>,
    pub accepted: Option<u64>,
    pub committed: Option<u64>,
    pub applied: Option<u64>,
    pub journal: Option<Vec<String>>,
    pub members: Option<Vec<String>>,
    pub weights: Option<Vec<u64>>,
    pub markers: Option<Vec<String>>,
    pub witnesses: Option<Vec<String>>,
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
    /// The Lean theorem the case pins; the corpus and the
    /// formalization share one vocabulary.
    pub theorem: String,
    pub clause: String,
    pub keywords: Vec<String>,
    pub setup: Vec<Op>,
    pub input: Op,
    pub expect: Expect,
}

// ----------------------------------------------------------------------
// Identities, hex, and wire
// ----------------------------------------------------------------------

/// The identity half's parse failure: the pair is one-indexed in both
/// halves, so a zero half is no identity (`docs/uvrr-boot-gate.md` §5).
fn half(text: &str) -> Result<u16, String> {
    text.parse::<u16>()
        .map_err(|e| format!("the identity half {text:?} is not a u16: {e}"))
        .and_then(|value| {
            if value == 0 {
                Err(format!("the identity half {text:?} is zero: no identity"))
            } else {
                Ok(value)
            }
        })
}

/// The identity an explicit pair names.
pub fn identity(pair: &str) -> Result<NodeId, String> {
    let (system, counter) = pair
        .split_once(':')
        .ok_or_else(|| format!("the identity {pair:?} is not the pair system:counter"))?;
    let system = uvrr::ids::SystemId::new(half(system)?)
        .ok_or_else(|| format!("the system half {system:?} is refused"))?;
    let counter = uvrr::ids::CrashCounter::new(half(counter)?)
        .ok_or_else(|| format!("the counter half {counter:?} is refused"))?;
    Ok(NodeId::new(system, counter))
}

/// The explicit pair an identity names.
pub fn pair_of(id: NodeId) -> String {
    let system = id.system_id().expect("a live identity is lawful").get();
    let counter = id.crash_counter().expect("a live identity is lawful").get();
    format!("{system}:{counter}")
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err("odd hex length".into());
    }
    let value = |b: u8| -> Result<u8, String> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err("not hex".into()),
        }
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
// Abstract names
// ----------------------------------------------------------------------

/// The corpus's status vocabulary, stated once: the protocol's own
/// status names, never a Rust debug rendering.
fn status_name(status: Status) -> &'static str {
    match status {
        Status::Normal => "Normal",
        Status::ViewChange => "ViewChange",
        Status::Restarting => "Restarting",
        Status::Replaying => "Replaying",
        Status::Joining => "Joining",
    }
}

/// The abstract operation name of a typed cluster operation
/// (`docs/uvrr-compliance.md` §3): the corpus's vocabulary, never Rust
/// debug formatting.
fn system_op_name(op: &SystemOperation) -> String {
    match op {
        SystemOperation::Void => "void".into(),
        SystemOperation::Init { order } => format!(
            "init order={}",
            order
                .iter()
                .map(|id| pair_of(*id))
                .collect::<Vec<_>>()
                .join(",")
        ),
        SystemOperation::Increment(node) => format!("increment {}", pair_of(*node)),
        SystemOperation::Decrement(node) => format!("decrement {}", pair_of(*node)),
        SystemOperation::Double => "double".into(),
        SystemOperation::Halve => "halve".into(),
        SystemOperation::Join { node, position } => {
            format!("join {} at {position}", pair_of(*node))
        }
        SystemOperation::Leave(node) => format!("leave {}", pair_of(*node)),
        SystemOperation::Nominate { from, offset } => {
            format!("nominate from={} offset={offset}", from.0)
        }
        SystemOperation::Batch(ops) => format!(
            "batch [{}]",
            ops.iter()
                .map(system_op_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The typed operation an abstract name names, with the argument
/// identities resolved.
fn system_op_of(op: &SystemOp) -> Result<SystemOperation, String> {
    Ok(match op {
        SystemOp::Increment { node } => SystemOperation::Increment(identity(node)?),
        SystemOp::Decrement { node } => SystemOperation::Decrement(identity(node)?),
        SystemOp::Double => SystemOperation::Double,
        SystemOp::Halve => SystemOperation::Halve,
        SystemOp::Join { node, position } => SystemOperation::Join {
            node: identity(node)?,
            position: *position,
        },
        SystemOp::Leave { node } => SystemOperation::Leave(identity(node)?),
        SystemOp::Nominate { from, offset } => SystemOperation::Nominate {
            from: View(*from),
            offset: *offset,
        },
        SystemOp::Batch { ops } => {
            SystemOperation::Batch(ops.iter().map(system_op_of).collect::<Result<_, _>>()?)
        }
    })
}

/// Renders a journal entry's payload as corpus text.
fn render_payload(entry: &LogEntry) -> String {
    match &entry.payload {
        Payload::Operation { payload, .. } => String::from_utf8_lossy(payload).into_owned(),
        Payload::System(op) => system_op_name(op),
    }
}

// ----------------------------------------------------------------------
// The executor
// ----------------------------------------------------------------------

/// The abstract host interface over one harness.
pub struct Executor {
    h: Harness,
    /// The roster size; the one-past-the-end identity names the outside
    /// gossip sender.
    nodes: usize,
    /// Every identity the cluster ever knew, in first-seen order: the
    /// provision roster, then every bumped and booted life.
    known: Vec<NodeId>,
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
        let known = (0..nodes)
            .map(|index| {
                identity(&format!("{}:1", index + 1)).expect("the provision identity is lawful")
            })
            .collect();
        Executor {
            h: Harness::with_knobs(nodes, knobs),
            nodes,
            known,
            op_seq: 1,
        }
    }

    /// The harness, for the exporter's state inspection.
    pub fn harness(&mut self) -> &mut Harness {
        &mut self.h
    }

    /// The identity a corpus pair names; the one-past-the-end roster
    /// position names the outside gossip sender.
    pub fn id_of(&self, pair: &str) -> Result<NodeId, String> {
        identity(pair)
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
                let id = identity(node)?;
                let op = OperationId {
                    msb: OP_MSB,
                    lsb: self.op_seq,
                };
                self.op_seq += 1;
                self.h.propose(id, op, payload.as_bytes());
                Ok(())
            }
            Op::Reconfigure { node, system } => {
                let id = identity(node)?;
                let op = system_op_of(system)?;
                self.h.reconfigure(id, op, None);
                Ok(())
            }
            Op::Tick { node } => {
                self.h.tick(identity(node)?);
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
                self.h.crash(identity(node)?);
                Ok(())
            }
            Op::Restart { node, kind } => {
                let id = identity(node)?;
                match kind.as_str() {
                    "clean" => self.h.restart_with(id).map_err(|e| format!("{e:?}"))?,
                    "crashed" => {
                        let next = id.next_life().ok_or("the identity space is spent")?;
                        self.h.restart_as(id, next).map_err(|e| format!("{e:?}"))?;
                        self.known.push(next);
                    }
                    other => return Err(format!("no such restart kind: {other}")),
                }
                Ok(())
            }
            Op::Halt { node } => {
                self.h.halt(identity(node)?);
                Ok(())
            }
            Op::Boot { node } => {
                let id = identity(node)?;
                self.h.boot_as(id).map_err(|e| format!("{e:?}"))?;
                self.known.push(id);
                Ok(())
            }
            Op::Announce { node, old } => {
                let id = identity(node)?;
                let previous = identity(old)?;
                self.h.reincarnate(id, previous);
                Ok(())
            }
            Op::Deliver { to, from, wire } => {
                let message = from_wire_hex(wire)?;
                let outcome = self.h.inject(identity(from)?, identity(to)?, message);
                if matches!(outcome, StepOutcome::NodeDown) {
                    return Err("the receiver is down".into());
                }
                Ok(())
            }
            Op::Gossip { wire } => {
                let message = from_wire_hex(wire)?;
                let sender = identity(&format!("{}:1", self.nodes + 1))?;
                let known = self.known.clone();
                for id in &known {
                    if self.h.is_up(*id) {
                        self.h.inject(sender, *id, message.clone());
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
        self.known.iter().filter(|id| self.h.is_up(**id)).all(|id| {
            self.h
                .snapshot(*id)
                .is_some_and(|s| s.status == Status::Normal.to_word())
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
                from: pair_of(from),
                to: pair_of(to),
                wire: wire_hex(&message),
            });
            self.h.deliver_next();
        }
        panic!("the drain did not quiet inside the bound");
    }

    /// One node's full post record, every field captured.
    #[must_use]
    pub fn post_of(&self, id: NodeId) -> PostNode {
        let snapshot = self.h.snapshot(id);
        let status = snapshot
            .map(|s| status_name(Status::from_word(s.status).expect("the word decodes")))
            .map(str::to_string);
        let frontiers = snapshot.map(|s| (s.era, s.view, s.accepted, s.committed, s.applied));
        let (era, view, accepted, committed, applied) = frontiers
            .map(|(era, view, accepted, committed, applied)| {
                (
                    Some(era),
                    Some(view),
                    Some(accepted),
                    Some(committed),
                    Some(applied),
                )
            })
            .unwrap_or((None, None, None, None, None));
        // The configuration through the public snapshot: the ordered
        // membership with its weights, learners included.
        let config = self.h.era_table(id).map(|table| {
            let record = table.current();
            let snapshot = record.config.to_snapshot();
            (
                snapshot.order.iter().map(|m| pair_of(m.node)).collect(),
                snapshot
                    .order
                    .iter()
                    .map(|m| u64::from(m.weight.0))
                    .collect(),
            )
        });
        let (members, weights) = config
            .map(|(members, weights)| (Some(members), Some(weights)))
            .unwrap_or((None, None));
        PostNode {
            node: pair_of(id),
            status,
            era,
            view,
            accepted,
            committed,
            applied,
            journal: Some(
                self.h
                    .journal_entries(id)
                    .into_iter()
                    .map(|entry| render_payload(&entry))
                    .collect(),
            ),
            members,
            weights,
            markers: Some(self.h.marker_states(id)),
            witnesses: Some(self.h.witnesses(id).into_iter().map(pair_of).collect()),
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
        let post = ex.known.iter().map(|id| ex.post_of(*id)).collect();
        Ok(Expect { deliveries, post })
    }
}

// ----------------------------------------------------------------------
// The assertion
// ----------------------------------------------------------------------

/// One named field's mismatch, or `None` when the field passes or is
/// unnamed.
fn field<T: PartialEq + std::fmt::Debug>(
    captured: &Option<T>,
    named: &Option<T>,
    name: &str,
) -> Option<String> {
    captured
        .as_ref()
        .zip(named.as_ref())
        .filter(|(x, y)| x != y)
        .map(|(x, y)| format!("{name}: expected {y:?}, captured {x:?}"))
}

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
        let Some(captured_post) = captured.post.iter().find(|p| p.node == named.node) else {
            return Err(format!("no post record for node {}", named.node));
        };
        if let Some(mismatch) = field(&captured_post.status, &named.status, "status")
            .or_else(|| field(&captured_post.era, &named.era, "era"))
            .or_else(|| field(&captured_post.view, &named.view, "view"))
            .or_else(|| field(&captured_post.accepted, &named.accepted, "accepted"))
            .or_else(|| field(&captured_post.committed, &named.committed, "committed"))
            .or_else(|| field(&captured_post.applied, &named.applied, "applied"))
            .or_else(|| field(&captured_post.journal, &named.journal, "journal"))
            .or_else(|| field(&captured_post.members, &named.members, "members"))
            .or_else(|| field(&captured_post.weights, &named.weights, "weights"))
            .or_else(|| field(&captured_post.markers, &named.markers, "markers"))
            .or_else(|| field(&captured_post.witnesses, &named.witnesses, "witnesses"))
        {
            return Err(mismatch);
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------
// The exporter's message builders
// ----------------------------------------------------------------------

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

/// A `PrepareOk` message as the exporter builds it: a fabricated vote.
pub fn prepare_ok(view: ViewId, slot: Slot) -> Message {
    Message {
        header: Header {
            tag: Tag::PrepareOk,
            view,
            slot,
        },
        body: Body::PrepareOk {},
    }
}

/// A `GossipRequest` as the exporter builds it, at the view named.
pub fn gossip_request(view: ViewId, prepared: Slot, committed: Slot) -> Message {
    Message {
        header: Header {
            tag: Tag::GossipRequest,
            view,
            slot: Slot(0),
        },
        body: Body::GossipRequest {
            prepared,
            committed,
        },
    }
}

/// A `Fuse` message as the exporter builds it, at the ballot named.
pub fn fuse(view: ViewId, first_slot: Slot, ops: Vec<SystemOperation>) -> Message {
    Message {
        header: Header {
            tag: Tag::Fuse,
            view,
            slot: first_slot,
        },
        body: Body::Fuse { ops },
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
