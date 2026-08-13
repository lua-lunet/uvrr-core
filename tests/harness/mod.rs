//! The scripted step-through harness: the deterministic host the Definition
//! of Done runs on.
//!
//! The harness is the host (docs/architecture.md: the host owns time,
//! transport, storage and packetization). It drives [`Replica`]s through the
//! plan/publish pipeline one explicit step at a time — a delivery, an
//! injection, a tick, a restart — and the *script* is the only source of
//! nondeterminism: no threads, no clock reads, no RNG. Same script, same
//! trace, every run, on every platform.
//!
//! # The legality gate
//!
//! After every step the harness scans every live node for a sticky fault the
//! script did not declare. The replica's `invariant::legal` (and S3's
//! indeterminate-persistence rule) does the forcing; the harness makes an
//! unexpected forcing a loud failure carrying the full step trace. A script
//! that intends a fault declares [`Harness::expect_fault`] beforehand; the
//! declaration is consumed when the fault is observed. The gate trips on
//! *transitions*: a fault restored from disk at `restart_with` is evidence
//! about a prior life, not a new event, and is seeded as known.
//!
//! # The safety checker
//!
//! [`check_cluster_safety`] re-derives the cluster-wide safety properties
//! from observations, journals and the harness's own apply records — an
//! independent second pair of eyes, redundant with the invariants
//! [`vrr::progress::Progress`] enforces internally. Redundancy here is the
//! point; do not optimise it away. The rules, checked in this order so a
//! violation surfaces as its own variant:
//!
//! 1. **Frontier sanity**: `checkpoint <= applied <= committed <= accepted`
//!    on every live node's observation.
//! 2. **Single primary per view**: no two live nodes are `Normal` in the
//!    same `(era, view)` while each is the primary under its own
//!    configuration history.
//! 3. **Committed-prefix agreement**: for any two live nodes, one committed
//!    history is a prefix of the other — entries identical at every shared
//!    slot, era and payload included.
//! 4. **Applied agreement**: applied slots are contiguous from the first
//!    client slot (`INIT_SLOT + 1`), and no two nodes applied different
//!    payloads at the same slot.
//!
//! # What is not here
//!
//! The protocol behaviour of the later paths — view change, recovery, state
//! transfer, checkpoints and reconfiguration inputs — is still refused with
//! the named
//! `PlanRejection::Unsupported`. Normal operation is live:
//! `Prepare`/`PrepareOk`/`Commit`, the client table, the Apply/Applied/Reply
//! boundary, and the bootstrap from the fenced `Recovering` genesis state.

// The harness is shared infrastructure compiled into every protocol test
// target; no single target drives the whole scripted surface — the suites
// do, collectively. Dead-code analysis runs per target, so the documented
// allowance lives here at the module root rather than scattered per method.
#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use vrr::configuration::{EraTable, INIT_SLOT, VOID_SLOT};
use vrr::effects::{Effect, Stability, StabilityResult};
use vrr::ids::{ClientId, Era, Fault, NodeId, RequestNumber, Slot, Tick, View, ViewId};
use vrr::journal::{Journal, JournalView, LogEntry, RangeOutcome, SegmentedLog};
use vrr::message::Message;
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::quorum::WeightedMajority;
use vrr::replica::{
    Input, LifecycleError, Observer, PersistedProgress, PlanRejection, PublishOutcome,
    PublishRejection, Replica, TimedInput, ViewChangeKnobs,
};
use vrr::wire::Tag;

/// The replica configuration every harness node runs: the default journal
/// and the default quorum strategy. Overridable per-node configuration is a
/// later milestone's concern; nothing in the current suites needs it.
pub type HarnessReplica = Replica<SegmentedLog, WeightedMajority>;

/// Step-trace ring capacity. A failure prints the whole buffer; a script
/// longer than this still shows the breaking step and its neighbourhood.
const TRACE_CAPACITY: usize = 512;

/// One live node: the replica, its observation handle, the effects released
/// but not yet executed by the harness, and the one outstanding persistence
/// intent an external-stability mode parks behind (§7, S2).
struct Node {
    replica: HarnessReplica,
    observer: Observer,
    /// `Apply` effects released but not yet performed by the harness.
    pending_effects: Vec<Effect>,
    /// The base revision of the parked persistence intent, if any.
    outstanding_intent: Option<u64>,
}

/// What the harness's "disk" recorded at crash time: the published progress
/// record, the journal, and the configuration history. The parked candidate
/// of an outstanding intent is volatile and dies with the node — correctly:
/// in an external-stability mode nothing unpersisted is durable.
struct Disk {
    persisted: PersistedProgress,
    journal: SegmentedLog,
    config: Arc<EraTable>,
}

/// One datagram in flight. `era` is the era authorising this copy, carried
/// separately exactly as `Effect::Send` carries it (W1), and surfaced in the
/// delivery trace: the era a copy was routed under is observable, so the
/// overlap-mode routing has something honest to decide over.
#[derive(Clone, Debug)]
struct Envelope {
    from: NodeId,
    to: NodeId,
    era: Era,
    message: Message,
}

/// The network: explicit queues and partition state. A partition holds
/// crossing datagrams — never drops them — so a script can inspect, deliver
/// or explicitly drop them later.
struct Network {
    queue: VecDeque<Envelope>,
    held: Vec<Envelope>,
    partition: Option<(Vec<NodeId>, Vec<NodeId>)>,
    /// Deliveries attempted to down nodes.
    undeliverable: u64,
    /// Held datagrams explicitly dropped by the script.
    dropped: u64,
}

impl Network {
    /// Whether `from`/`to` sit on opposite sides of the partition. A node in
    /// neither set is unreachable by the partition's terms and its traffic
    /// flows — the sets name the two sides, not the survivors.
    fn crosses(&self, from: NodeId, to: NodeId) -> bool {
        match &self.partition {
            None => false,
            Some((a, b)) => {
                (a.contains(&from) && b.contains(&to)) || (b.contains(&from) && a.contains(&to))
            }
        }
    }

    /// Enqueues a datagram, holding it if it crosses the partition. Returns
    /// whether it was held.
    fn route(&mut self, envelope: Envelope) -> bool {
        if self.crosses(envelope.from, envelope.to) {
            self.held.push(envelope);
            true
        } else {
            self.queue.push_back(envelope);
            false
        }
    }
}

/// What one step did to a node. Refusals are data, not failures: a script
/// asserts them, and every refusal is a named variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StepOutcome {
    /// The transition installed and released these effects (§7, volatile or
    /// confirmed).
    Published {
        /// The newly published revision.
        revision: u64,
        /// The released effects, in release order.
        effects: Vec<Effect>,
    },
    /// An external-stability mode parked the transition behind its
    /// persistence intent; the harness recorded the intent and waits for
    /// [`Harness::confirm`].
    Parked {
        /// The base revision the intent is named by.
        revision: u64,
    },
    /// `plan` refused the input.
    PlanRefused(PlanRejection),
    /// `publish` refused the planned transition.
    PublishRefused(PublishRejection),
    /// The node is down. The input was not delivered.
    NodeDown,
}

/// The result of delivering one queued datagram.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeliveryOutcome {
    /// The sender recorded on the envelope.
    pub from: NodeId,
    /// The addressee.
    pub to: NodeId,
    /// What the delivery did.
    pub outcome: StepOutcome,
}

/// One `Apply` effect performed by the harness: recorded into the apply
/// log, then fed back as `Input::Applied`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApplyOutcome {
    /// The applied slot.
    pub slot: Slot,
    /// The payload the application executed.
    pub payload: Box<[u8]>,
    /// What the feedback input did.
    pub outcome: StepOutcome,
}

/// One application-boundary event in harness step order: an `Apply` the
/// harness executed, or a `Reply` a node released. The record the
/// Reply-after-Apply property (§11.2) is asserted over structurally.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BoundaryEvent {
    /// The harness performed an `Apply` effect and fed `Applied` back.
    Applied {
        /// The node whose application ran.
        node: NodeId,
        /// The applied slot.
        slot: Slot,
    },
    /// A node released a `Reply`.
    Replied {
        /// The replying node.
        node: NodeId,
        /// The client addressed.
        client: ClientId,
        /// The request answered.
        request: RequestNumber,
    },
}

/// The evidence the safety checker rules on, per live node. Everything the
/// checker needs and nothing else: the published observation, the
/// configuration history that authorises it, the committed journal prefix,
/// and the harness's own apply record.
///
/// Constructed by [`Harness::check_safety`] from live replicas; also
/// constructible directly, which is how the contract test plants violations
/// to prove the checker is not vacuous.
#[derive(Clone, Debug)]
pub struct NodeEvidence {
    /// The node this evidence describes.
    pub id: NodeId,
    /// Its latest published observation (B1).
    pub snapshot: ProgressSnapshot,
    /// Its configuration history — primary beliefs are computed under the
    /// node's *own* table, so a divergent membership shows as two primaries.
    pub config: Arc<EraTable>,
    /// The committed journal prefix, from slot 1 through the committed
    /// frontier.
    pub committed: Vec<LogEntry>,
    /// The harness's apply record for this node.
    pub applied: Vec<(Slot, Box<[u8]>)>,
}

/// A violated cluster-wide safety rule. One variant per rule, checked in the
/// order the module documents, so a planted violation surfaces as its own
/// variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SafetyViolation {
    /// Rule 2: two live nodes each believe they are primary of the same
    /// `(era, view)`.
    DualPrimary {
        /// The contested view.
        view: ViewId,
        /// The first claimant.
        a: NodeId,
        /// The second claimant.
        b: NodeId,
    },
    /// Rule 3: two committed histories differ at a shared slot.
    CommittedDivergence {
        /// One node.
        a: NodeId,
        /// The other node.
        b: NodeId,
        /// The first slot at which their committed entries differ.
        slot: Slot,
    },
    /// Rule 4: two nodes applied different payloads at the same slot.
    AppliedConflict {
        /// The contested slot.
        slot: Slot,
        /// The node that applied first in evidence order.
        a: NodeId,
        /// The node whose payload disagrees.
        b: NodeId,
    },
    /// Rule 4: a node's apply record is not contiguous from the first
    /// client slot.
    AppliedGap {
        /// The node with the gap.
        node: NodeId,
        /// The slot the record required next.
        expected: Slot,
        /// The slot the record held instead.
        got: Slot,
    },
    /// Rule 1: an observation broke
    /// `checkpoint <= applied <= committed <= accepted`.
    FrontierChain {
        /// The node holding the broken observation.
        node: NodeId,
        /// The observed checkpoint frontier.
        checkpoint: u64,
        /// The observed applied frontier.
        applied: u64,
        /// The observed committed frontier.
        committed: u64,
        /// The observed accepted frontier.
        accepted: u64,
    },
}

/// The independent second pair of eyes (see the module docs for the rules
/// and their order). Redundant with `Progress`'s internal enforcement,
/// deliberately.
pub fn check_cluster_safety(nodes: &[NodeEvidence]) -> Result<(), SafetyViolation> {
    // Rule 1 — frontier sanity on every observation.
    for node in nodes {
        let s = &node.snapshot;
        if !(s.checkpoint <= s.applied && s.applied <= s.committed && s.committed <= s.accepted) {
            return Err(SafetyViolation::FrontierChain {
                node: node.id,
                checkpoint: s.checkpoint,
                applied: s.applied,
                committed: s.committed,
                accepted: s.accepted,
            });
        }
    }

    // Rule 2 — single primary per view. A node believes itself primary when
    // it is Normal and its own configuration history says `primary(view) ==
    // self`; two divergent memberships can then name two primaries for one
    // view, which is exactly the violation.
    let mut claims: Vec<(ViewId, NodeId)> = Vec::new();
    for node in nodes {
        if Status::from_word(node.snapshot.status) != Some(Status::Normal) {
            continue;
        }
        let view = ViewId {
            era: Era(node.snapshot.era),
            view: View(node.snapshot.view),
        };
        let primary = node
            .config
            .record(view.era)
            .and_then(|record| record.config.primary(view.view));
        if primary == Some(node.id) {
            for (claimed, other) in &claims {
                if *claimed == view {
                    return Err(SafetyViolation::DualPrimary {
                        view,
                        a: *other,
                        b: node.id,
                    });
                }
            }
            claims.push((view, node.id));
        }
    }

    // Rule 3 — committed-prefix agreement: identical entries at every shared
    // slot, so one history is a prefix of the other.
    for (position, a) in nodes.iter().enumerate() {
        for b in &nodes[position + 1..] {
            for (left, right) in a.committed.iter().zip(b.committed.iter()) {
                if left != right {
                    return Err(SafetyViolation::CommittedDivergence {
                        a: a.id,
                        b: b.id,
                        slot: left.slot,
                    });
                }
            }
        }
    }

    // Rule 4 — applied agreement and contiguity from the first client slot:
    // slots 1 and 2 are the genesis system operations (§8.7.2), which the
    // core folds and the application never sees.
    let mut seen: BTreeMap<Slot, (NodeId, &[u8])> = BTreeMap::new();
    for node in nodes {
        let mut expected = INIT_SLOT.next().expect("INIT_SLOT has a successor");
        for (slot, payload) in &node.applied {
            if *slot != expected {
                return Err(SafetyViolation::AppliedGap {
                    node: node.id,
                    expected,
                    got: *slot,
                });
            }
            match seen.get(slot) {
                Some((other, previous)) => {
                    if *previous != payload.as_ref() {
                        return Err(SafetyViolation::AppliedConflict {
                            slot: *slot,
                            a: *other,
                            b: node.id,
                        });
                    }
                }
                None => {
                    seen.insert(*slot, (node.id, payload.as_ref()));
                }
            }
            expected = slot.next().expect("an applied slot has a successor");
        }
    }

    Ok(())
}

/// The deterministic simulation host: a cluster of nodes, an explicit
/// network, a logical clock advanced only by the script, and the apply
/// records. Every mutating method is one step; every step is one trace line.
pub struct Harness {
    /// `None` is a crashed/down node; its volatile state died with it.
    nodes: Vec<Option<Node>>,
    network: Network,
    /// The harness's logical clock, advanced only by `tick`/`tick_all`.
    tick: Tick,
    /// The stability level every node runs (S3).
    stability: Stability,
    /// The view-change knobs every node runs (W5) — threaded
    /// through restarts, because a restarted node plays by the cluster's
    /// rules, not fresh ones.
    knobs: ViewChangeKnobs,
    /// Genesis order; also the node-index mapping (`NodeId(i)` is index `i`).
    genesis_order: Vec<NodeId>,
    /// Per-node record of the `Apply` effects the harness executed.
    applied: Vec<Vec<(Slot, Box<[u8]>)>>,
    /// Client replies released by nodes, in release order.
    replies: Vec<(NodeId, ClientId, RequestNumber, Box<[u8]>)>,
    /// The ordered Apply/Reply boundary record (§11.2 assertions).
    boundary: Vec<BoundaryEvent>,
    /// Per-node durable state recorded at crash time.
    disks: Vec<Option<Disk>>,
    /// Per-node pending fault declarations (`expect_fault`).
    declared_faults: Vec<bool>,
    /// Per-node faults already observed and accounted for.
    faulted_known: Vec<bool>,
    /// Next request number per client, for `client_request`.
    requests: BTreeMap<ClientId, u64>,
    /// The step trace ring.
    trace: VecDeque<String>,
    /// Steps taken, also the trace line number.
    step_seq: u64,
}

impl Harness {
    /// `n` provisioned nodes, volatile stability, `SegmentedLog`,
    /// `WeightedMajority`, genesis order `[NodeId(0)..NodeId(n))`.
    ///
    /// View-change knobs: suspicion disabled, unbounded suffix — the
    /// normal-operation suites never time out. View-change scripts use
    /// [`Harness::with_knobs`].
    #[must_use]
    pub fn provision(n: usize) -> Harness {
        Self::with_stability(n, Stability::Volatile)
    }

    /// [`Harness::provision`] with explicit view-change knobs: the
    /// timeout and the §13.1 suffix budget are host policy (W5), and a
    /// script that tests them sets them, never inherits them.
    #[must_use]
    pub fn with_knobs(n: usize, knobs: ViewChangeKnobs) -> Harness {
        Self::assemble(n, Stability::Volatile, knobs)
    }

    /// [`Harness::provision`] with an explicit stability level. In every
    /// non-volatile mode the harness records each `Persist` intent and the
    /// script must confirm it explicitly with [`Harness::confirm`] — the
    /// real host contract, mirrored.
    #[must_use]
    pub fn with_stability(n: usize, stability: Stability) -> Harness {
        Self::assemble(n, stability, Harness::no_view_change_knobs())
    }

    /// The knob setting that makes the view-change machinery inert: no
    /// suspicion ever fires, and suffixes are never truncated.
    fn no_view_change_knobs() -> ViewChangeKnobs {
        ViewChangeKnobs {
            primary_timeout: 0,
            view_change_budget: usize::MAX,
        }
    }

    fn assemble(n: usize, stability: Stability, knobs: ViewChangeKnobs) -> Harness {
        let genesis_order: Vec<NodeId> = (0..n)
            .map(|i| NodeId(u32::try_from(i).expect("cluster size fits u32")))
            .collect();
        let mut nodes = Vec::with_capacity(n);
        for &id in &genesis_order {
            let replica = Replica::provision(
                id,
                genesis_order.clone(),
                WeightedMajority,
                SegmentedLog::new(),
                stability,
                knobs,
            )
            .expect("the genesis order 0..n provisions");
            let observer = replica.observer();
            nodes.push(Some(Node {
                replica,
                observer,
                pending_effects: Vec::new(),
                outstanding_intent: None,
            }));
        }
        let mut harness = Harness {
            nodes,
            network: Network {
                queue: VecDeque::new(),
                held: Vec::new(),
                partition: None,
                undeliverable: 0,
                dropped: 0,
            },
            tick: Tick(0),
            stability,
            knobs,
            genesis_order,
            applied: (0..n).map(|_| Vec::new()).collect(),
            replies: Vec::new(),
            boundary: Vec::new(),
            disks: (0..n).map(|_| None).collect(),
            declared_faults: vec![false; n],
            faulted_known: vec![false; n],
            requests: BTreeMap::new(),
            trace: VecDeque::new(),
            step_seq: 0,
        };
        harness.record(format!("provision n={n} stability={stability:?}"));
        harness
    }

    // ------------------------------------------------------------------
    // Clock
    // ------------------------------------------------------------------

    /// The harness's logical clock. Never goes backwards; every
    /// [`TimedInput`] the harness emits carries the current value.
    #[must_use]
    pub fn now(&self) -> Tick {
        self.tick
    }

    fn advance_clock(&mut self) {
        self.tick = Tick(
            self.tick
                .0
                .checked_add(1)
                .expect("the tick space is not exhausted in a test"),
        );
    }

    /// Advances the clock and delivers `Input::Tick` to one node.
    pub fn tick(&mut self, id: NodeId) -> StepOutcome {
        self.advance_clock();
        self.drive(id, format!("n={} tick", id.0), Input::Tick)
    }

    /// Advances the clock once and delivers `Input::Tick` to every live
    /// node, in node order. Down nodes are skipped.
    pub fn tick_all(&mut self) -> Vec<(NodeId, StepOutcome)> {
        self.advance_clock();
        let mut results = Vec::new();
        for index in 0..self.nodes.len() {
            if self.nodes[index].is_some() {
                let id = self.genesis_order[index];
                let outcome = self.drive(id, format!("n={} tick", id.0), Input::Tick);
                results.push((id, outcome));
            }
        }
        results
    }

    // ------------------------------------------------------------------
    // Network
    // ------------------------------------------------------------------

    /// Enqueues a datagram as if from a peer — the way a script fabricates
    /// protocol traffic no node emitted. Partition crossing is decided here,
    /// at enqueue: crossing datagrams are held, the rest are queued.
    pub fn send(&mut self, from: NodeId, to: NodeId, message: Message) {
        let tag = message.header.tag;
        let era = message.header.view.era;
        let held = self.network.route(Envelope {
            from,
            to,
            era,
            message,
        });
        let whereabouts = if held { "held" } else { "queued" };
        self.record(format!(
            "net send n{}->n{} {tag:?} ({whereabouts})",
            from.0, to.0
        ));
    }

    /// Delivers the oldest queued datagram through plan/publish. One step.
    /// `None` when the queue is empty — no step, no trace line.
    pub fn deliver_next(&mut self) -> Option<DeliveryOutcome> {
        let envelope = self.network.queue.pop_front()?;
        Some(self.deliver_envelope(envelope))
    }

    /// Delivers the oldest queued datagram addressed to `id`. One step.
    /// `None` when the queue holds nothing for the node.
    pub fn deliver_to(&mut self, id: NodeId) -> Option<DeliveryOutcome> {
        let position = self
            .network
            .queue
            .iter()
            .position(|envelope| envelope.to == id)?;
        let envelope = self
            .network
            .queue
            .remove(position)
            .expect("the position was found above");
        Some(self.deliver_envelope(envelope))
    }

    /// Drains the queue, one step per datagram, in queue order.
    pub fn deliver_all(&mut self) -> Vec<DeliveryOutcome> {
        let mut outcomes = Vec::new();
        while let Some(outcome) = self.deliver_next() {
            outcomes.push(outcome);
        }
        outcomes
    }

    /// Delivers the oldest queued datagram addressed to `id` with the given
    /// tag and header slot — the way a script stages an out-of-order or
    /// retransmitted delivery. One step. `None` when no such datagram is
    /// queued.
    pub fn deliver_to_matching(
        &mut self,
        id: NodeId,
        tag: Tag,
        slot: Slot,
    ) -> Option<DeliveryOutcome> {
        let position = self.network.queue.iter().position(|envelope| {
            envelope.to == id
                && envelope.message.header.tag == tag
                && envelope.message.header.slot == slot
        })?;
        let envelope = self
            .network
            .queue
            .remove(position)
            .expect("the position was found above");
        Some(self.deliver_envelope(envelope))
    }

    fn deliver_envelope(&mut self, envelope: Envelope) -> DeliveryOutcome {
        let summary = format!(
            "net deliver n{}->n{} {:?} e{}",
            envelope.from.0, envelope.to.0, envelope.message.header.tag, envelope.era.0
        );
        let from = envelope.from;
        let to = envelope.to;
        let outcome = self.drive(
            to,
            summary,
            Input::Peer {
                from: envelope.from,
                message: envelope.message,
            },
        );
        if outcome == StepOutcome::NodeDown {
            self.network.undeliverable += 1;
        }
        DeliveryOutcome { from, to, outcome }
    }

    /// Force-feeds a raw message to a node (the DoD's "force feed messages
    /// to a simulated node"), bypassing the queues and the partition. Same
    /// plan/publish path as a delivery; one step. `from` is the
    /// transport-attributed sender the core's guards and quorum counting
    /// see — a fabrication names its claimed author.
    pub fn inject(&mut self, from: NodeId, id: NodeId, message: Message) -> StepOutcome {
        let summary = format!(
            "n={} inject n{} {:?} s{}",
            id.0, from.0, message.header.tag, message.header.slot.0
        );
        self.drive(id, summary, Input::Peer { from, message })
    }

    /// Holds crossing datagrams; traffic inside either side flows.
    pub fn partition(&mut self, a: Vec<NodeId>, b: Vec<NodeId>) {
        let show = |set: &[NodeId]| {
            set.iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let line = format!("net partition [{}] | [{}]", show(&a), show(&b));
        self.network.partition = Some((a, b));
        self.record(line);
    }

    /// Lifts the partition and requeues held datagrams, in the order they
    /// were held.
    pub fn heal(&mut self) {
        self.network.partition = None;
        let held = std::mem::take(&mut self.network.held);
        let count = held.len();
        for envelope in held {
            self.network.queue.push_back(envelope);
        }
        self.record(format!("net heal requeued={count}"));
    }

    /// Explicitly drops the held datagrams. Recorded in the trace — a drop
    /// is a script decision, never a harness default.
    pub fn drop_held(&mut self) -> usize {
        let held = std::mem::take(&mut self.network.held);
        let count = held.len();
        self.network.dropped += u64::try_from(count).expect("held count fits u64");
        let detail = held
            .iter()
            .map(|envelope| {
                format!(
                    "n{}->n{} {:?}",
                    envelope.from.0, envelope.to.0, envelope.message.header.tag
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.record(format!("net drop k={count} [{detail}]"));
        count
    }

    /// Explicitly drops the queued datagrams addressed to `id`. Recorded in
    /// the trace, like every drop: a script decision, never a default.
    /// The view-change suffix-budget choreography uses it to strand a node at an
    /// older frontier.
    pub fn drop_queued(&mut self, id: NodeId) -> usize {
        let mut kept = VecDeque::new();
        let mut dropped = Vec::new();
        for envelope in self.network.queue.drain(..) {
            if envelope.to == id {
                dropped.push(envelope);
            } else {
                kept.push_back(envelope);
            }
        }
        self.network.queue = kept;
        let count = dropped.len();
        self.network.dropped += u64::try_from(count).expect("dropped count fits u64");
        let detail = dropped
            .iter()
            .map(|envelope| {
                format!(
                    "n{}->n{} {:?}",
                    envelope.from.0, envelope.to.0, envelope.message.header.tag
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.record(format!("net drop-queued n{} k={count} [{detail}]", id.0));
        count
    }

    /// The first queued datagram addressed to `to` with the given tag,
    /// without delivering it — how a script inspects the evidence a node
    /// emitted (the view-change ranking and budget assertions).
    #[must_use]
    pub fn peek_queued(&self, to: NodeId, tag: Tag) -> Option<Message> {
        self.network
            .queue
            .iter()
            .find(|envelope| envelope.to == to && envelope.message.header.tag == tag)
            .map(|envelope| envelope.message.clone())
    }

    /// Delivers the first queued datagram addressed to `to` with the given
    /// tag, regardless of header slot. One step. `None` when no such
    /// datagram is queued.
    pub fn deliver_tag(&mut self, to: NodeId, tag: Tag) -> Option<DeliveryOutcome> {
        let position = self
            .network
            .queue
            .iter()
            .position(|envelope| envelope.to == to && envelope.message.header.tag == tag)?;
        let envelope = self
            .network
            .queue
            .remove(position)
            .expect("the position was found above");
        Some(self.deliver_envelope(envelope))
    }

    /// A whole-history copy of the node's journal: what a
    /// view-change install left behind, asserted directly — a divergent
    /// tail must be GONE from the journal, not merely from the frontier.
    /// The harness never reclaims, so the copy is always complete.
    #[must_use]
    pub fn journal_entries(&self, id: NodeId) -> Vec<LogEntry> {
        let index = self.index_of(id);
        let Some(node) = self.nodes[index].as_ref() else {
            return Vec::new();
        };
        let view = node.replica.journal().view();
        let mut entries = Vec::new();
        let Some(frontier) = view.accepted() else {
            return entries;
        };
        match view.copy_out(VOID_SLOT, frontier, &mut entries) {
            RangeOutcome::Complete => entries,
            RangeOutcome::Short { .. }
            | RangeOutcome::BelowRetention { .. }
            | RangeOutcome::Empty => {
                panic!("the harness never reclaims: its journals are whole")
            }
        }
    }

    /// The node's sticky fault, if declared — the identity, not just the
    /// `faulted` word the observation carries (the `expect_fault`
    /// scripts assert WHICH fault the breach declared).
    #[must_use]
    pub fn fault_of(&self, id: NodeId) -> Option<Fault> {
        self.nodes
            .get(usize::try_from(id.0).expect("node ids are small"))
            .and_then(Option::as_ref)
            .and_then(|node| node.replica.progress().fault())
    }

    /// Queued (deliverable) datagrams.
    #[must_use]
    pub fn queued_len(&self) -> usize {
        self.network.queue.len()
    }

    /// Datagrams held by the partition.
    #[must_use]
    pub fn held_len(&self) -> usize {
        self.network.held.len()
    }

    /// The held datagrams, as `(from, to, tag)` — inspectable, per the
    /// partition contract.
    #[must_use]
    pub fn held_summary(&self) -> Vec<(NodeId, NodeId, Tag)> {
        self.network
            .held
            .iter()
            .map(|envelope| (envelope.from, envelope.to, envelope.message.header.tag))
            .collect()
    }

    /// Deliveries attempted to down nodes.
    #[must_use]
    pub fn undeliverable_count(&self) -> u64 {
        self.network.undeliverable
    }

    /// Held datagrams explicitly dropped by the script.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.network.dropped
    }

    // ------------------------------------------------------------------
    // Client and stability inputs
    // ------------------------------------------------------------------

    /// Submits a client request to a specific node. Request numbers are
    /// per-client and monotonic, assigned by the harness from 1 — one less
    /// thing for a script to get wrong.
    pub fn client_request(&mut self, id: NodeId, client: ClientId, payload: &[u8]) -> StepOutcome {
        let request = self.requests.get(&client).copied().unwrap_or(1);
        self.requests.insert(
            client,
            request
                .checked_add(1)
                .expect("the request-number space is not exhausted in a test"),
        );
        self.client_request_numbered(id, client, request, payload)
    }

    /// Submits a client request with an explicit request number — the way a
    /// script drives the duplicate-suppression table's edge cases (§9.2):
    /// exact duplicates, in-flight duplicates, gaps, and replays.
    pub fn client_request_numbered(
        &mut self,
        id: NodeId,
        client: ClientId,
        request: u64,
        payload: &[u8],
    ) -> StepOutcome {
        let summary = format!("n={} client={} req={request}", id.0, client.0);
        self.drive(
            id,
            summary,
            Input::Client {
                client,
                request: RequestNumber(request),
                payload: payload.into(),
            },
        )
    }

    /// Confirms the node's one outstanding `Persist` intent — the external-
    /// stability host contract (S2/S3). Panics if no intent is outstanding:
    /// a confirmation of nothing is a script bug, not an event.
    pub fn confirm(&mut self, id: NodeId, result: StabilityResult) -> StepOutcome {
        let index = self.index_of(id);
        let revision = match self.nodes[index].as_ref() {
            Some(node) => node
                .outstanding_intent
                .unwrap_or_else(|| panic!("n={} has no outstanding Persist intent", id.0)),
            None => panic!("n={} is down; there is nothing to confirm", id.0),
        };
        self.confirm_raw(id, revision, result)
    }

    /// Confirms with an explicit revision, for scripts that exercise the
    /// replica's own refusal paths (`NoTransitionOutstanding`,
    /// `ConfirmationMismatch`).
    pub fn confirm_raw(
        &mut self,
        id: NodeId,
        revision: u64,
        result: StabilityResult,
    ) -> StepOutcome {
        let word = match &result {
            StabilityResult::Stable { .. } => "stable",
            StabilityResult::Failed { .. } => "failed",
            StabilityResult::Indeterminate { .. } => "indeterminate",
        };
        let outcome = self.drive(
            id,
            format!("n={} confirm r{revision} {word}", id.0),
            Input::StabilityConfirmation { revision, result },
        );
        if matches!(outcome, StepOutcome::Published { .. }) {
            let index = self.index_of(id);
            if let Some(node) = self.nodes[index].as_mut() {
                node.outstanding_intent = None;
            }
        }
        outcome
    }

    /// Performs the node's pending `Apply` effects in release order: records
    /// each into the apply log and feeds `Input::Applied` back with the
    /// payload echoed as the application's result. The feedback is the §11.2
    /// completion: for a client slot the replica advances `applied` and, on
    /// the primary, releases the Reply.
    ///
    /// The unknown-result re-drive (§9.2) is the host's to recognise
    /// (§11): an `Apply` at or below the slot this node's application
    /// already reached is a re-execution of a committed slot, not new work.
    /// The echo application is idempotent, so the re-drive is performed and
    /// its completion fed back — the core answers the outstanding retry —
    /// but the apply log records only the first execution, keeping the
    /// record the safety checker's contiguity rule rules on.
    pub fn execute_apply_effects(&mut self, id: NodeId) -> Vec<ApplyOutcome> {
        let index = self.index_of(id);
        let pending = match self.nodes[index].as_mut() {
            Some(node) => std::mem::take(&mut node.pending_effects),
            None => Vec::new(),
        };
        let mut outcomes = Vec::new();
        for effect in pending {
            match effect {
                apply @ Effect::Apply { .. } => {
                    let Effect::Apply { slot, payload } = apply else {
                        unreachable!("the pattern matched Apply")
                    };
                    let redrive = self.applied[index]
                        .last()
                        .is_some_and(|(greatest, _)| slot <= *greatest);
                    if !redrive {
                        self.applied[index].push((slot, payload.clone()));
                    }
                    self.boundary.push(BoundaryEvent::Applied {
                        node: self.genesis_order[index],
                        slot,
                    });
                    let outcome = self.drive(
                        id,
                        format!("n={} apply s{}", id.0, slot.0),
                        Input::Applied {
                            slot,
                            result: payload.clone(),
                        },
                    );
                    outcomes.push(ApplyOutcome {
                        slot,
                        payload,
                        outcome,
                    });
                }
                Effect::Send { .. } | Effect::Reply { .. } | Effect::Persist(_) => {
                    panic!("only Apply effects are routed to a node's pending list")
                }
            }
        }
        outcomes
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    /// Crashes a node: its volatile state dies with it, and the harness's
    /// "disk" records the published progress, the journal, and the
    /// configuration history. Deliveries to a down node are recorded
    /// undeliverable.
    pub fn crash(&mut self, id: NodeId) {
        let index = self.index_of(id);
        let node = self.nodes[index]
            .take()
            .unwrap_or_else(|| panic!("n={} is already down", id.0));
        self.disks[index] = Some(Disk {
            persisted: PersistedProgress::from(node.replica.progress()),
            journal: clone_journal(node.replica.journal()),
            config: Arc::clone(node.replica.progress().config()),
        });
        self.faulted_known[index] = false;
        self.declared_faults[index] = false;
        self.record(format!("n={} crash (disk recorded)", id.0));
    }

    /// A new life with no memory: `provision` semantics on a node that was a
    /// member. The amnesiac voter is §14.2's problem, not the harness's —
    /// the name says what this is.
    pub fn restart_amnesiac(&mut self, id: NodeId) -> Result<(), LifecycleError> {
        let index = self.index_of(id);
        assert!(
            self.nodes[index].is_none(),
            "n={} is up; crash it before restarting it",
            id.0
        );
        // The amnesiac life starts from genesis: its applied frontier is
        // wiped with everything else, so when it adopts a committed history
        // it RE-APPLIES entries the old life already executed — the spec's
        // at-least-once replay (§11), whose dedup is the host's problem.
        // The harness mirrors a host that lost its apply tracking with the
        // node: the record clears, and rule 4's contiguity claim restarts
        // with the new life.
        self.applied[index].clear();
        match Replica::provision(
            id,
            self.genesis_order.clone(),
            WeightedMajority,
            SegmentedLog::new(),
            self.stability,
            self.knobs,
        ) {
            Ok(replica) => {
                self.install(index, replica);
                self.record(format!("n={} restart(amnesiac)", id.0));
                Ok(())
            }
            Err(error) => {
                self.record(format!("n={} restart(amnesiac) refused: {error:?}", id.0));
                Err(error)
            }
        }
    }

    /// A later life: `reopen` with whatever the harness's disk recorded at
    /// crash time. A fault persisted in the record reopens faulted — that is
    /// evidence restored, not a new fault, so it does not trip the gate.
    pub fn restart_with(&mut self, id: NodeId) -> Result<(), LifecycleError> {
        let index = self.index_of(id);
        assert!(
            self.nodes[index].is_none(),
            "n={} is up; crash it before restarting it",
            id.0
        );
        let (journal, persisted, config) = match self.disks[index].as_ref() {
            Some(disk) => (
                clone_journal(&disk.journal),
                disk.persisted,
                Arc::clone(&disk.config),
            ),
            None => panic!("n={} has no recorded disk; crash it first", id.0),
        };
        match Replica::reopen(
            id,
            WeightedMajority,
            journal,
            persisted,
            config,
            self.stability,
            self.knobs,
        ) {
            Ok(replica) => {
                self.install(index, replica);
                self.record(format!("n={} restart(with disk)", id.0));
                Ok(())
            }
            Err(error) => {
                self.record(format!("n={} restart(with disk) refused: {error:?}", id.0));
                Err(error)
            }
        }
    }

    fn install(&mut self, index: usize, replica: HarnessReplica) {
        // A fault present at construction — a persisted fault at reopen —
        // is seeded as known: the gate trips on transitions, and this fault
        // happened in a prior life.
        self.faulted_known[index] = replica.progress().fault().is_some();
        self.declared_faults[index] = false;
        let observer = replica.observer();
        self.nodes[index] = Some(Node {
            replica,
            observer,
            pending_effects: Vec::new(),
            outstanding_intent: None,
        });
    }

    // ------------------------------------------------------------------
    // The legality gate and the safety checker
    // ------------------------------------------------------------------

    /// Declares that the script intends the next steps to fault this node.
    /// Consumed when the fault is observed; an *undeclared* fault fails the
    /// step with the full trace.
    pub fn expect_fault(&mut self, id: NodeId) {
        let index = self.index_of(id);
        self.declared_faults[index] = true;
        self.record(format!("n={} expect-fault declared", id.0));
    }

    /// Runs the safety checker over the live cluster and panics with the
    /// step trace on a violation. The script calls this after quiesce.
    pub fn assert_safety(&self) {
        if let Err(violation) = self.check_safety() {
            panic!(
                "safety violation: {violation:?}\nstep trace:\n{}",
                self.trace_dump()
            );
        }
    }

    /// Collects evidence from every live node and runs
    /// [`check_cluster_safety`].
    pub fn check_safety(&self) -> Result<(), SafetyViolation> {
        let mut evidence = Vec::new();
        for (index, slot) in self.nodes.iter().enumerate() {
            let Some(node) = slot else { continue };
            let snapshot = node.observer.read();
            let mut committed = Vec::new();
            let frontier = Slot(snapshot.committed);
            if frontier != Slot::FIRST {
                let outcome =
                    node.replica
                        .journal()
                        .view()
                        .copy_out(VOID_SLOT, frontier, &mut committed);
                match outcome {
                    RangeOutcome::Complete => {}
                    RangeOutcome::Short { .. }
                    | RangeOutcome::BelowRetention { .. }
                    | RangeOutcome::Empty => {
                        panic!(
                            "the harness journal must serve its own committed range: {outcome:?}"
                        )
                    }
                }
            }
            evidence.push(NodeEvidence {
                id: self.genesis_order[index],
                snapshot,
                config: Arc::clone(node.replica.progress().config()),
                committed,
                applied: self.applied[index].clone(),
            });
        }
        check_cluster_safety(&evidence)
    }

    // ------------------------------------------------------------------
    // Observation and the trace
    // ------------------------------------------------------------------

    /// The node's latest published observation, or `None` if it is down.
    #[must_use]
    pub fn snapshot(&self, id: NodeId) -> Option<ProgressSnapshot> {
        self.nodes
            .get(usize::try_from(id.0).expect("node ids are small"))
            .and_then(Option::as_ref)
            .map(|node| node.observer.read())
    }

    /// Whether the node is up.
    #[must_use]
    pub fn is_up(&self, id: NodeId) -> bool {
        self.nodes
            .get(usize::try_from(id.0).expect("node ids are small"))
            .is_some_and(Option::is_some)
    }

    /// The harness's apply record for the node.
    #[must_use]
    pub fn applied(&self, id: NodeId) -> &[(Slot, Box<[u8]>)] {
        &self.applied[usize::try_from(id.0).expect("node ids are small")]
    }

    /// Client replies released by nodes, in release order.
    #[must_use]
    pub fn replies(&self) -> &[(NodeId, ClientId, RequestNumber, Box<[u8]>)] {
        &self.replies
    }

    /// The node's latest published drop diagnostic: every refused
    /// peer guard has a named outcome, and this is where it is observed.
    /// `None` if the node is down.
    #[must_use]
    pub fn diagnostic(&self, id: NodeId) -> Option<Diagnostic> {
        self.nodes
            .get(usize::try_from(id.0).expect("node ids are small"))
            .and_then(Option::as_ref)
            .map(|node| node.observer.read_diagnostic())
    }

    /// The ordered Apply/Reply boundary record — what the Reply-after-Apply
    /// property (§11.2) is asserted over.
    #[must_use]
    pub fn boundary_events(&self) -> &[BoundaryEvent] {
        &self.boundary
    }

    /// The step trace, oldest first (bounded by the ring capacity).
    #[must_use]
    pub fn step_trace(&self) -> Vec<String> {
        self.trace.iter().cloned().collect()
    }

    /// The step trace as one string, as printed on failure.
    #[must_use]
    pub fn trace_dump(&self) -> String {
        self.trace.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    // ------------------------------------------------------------------
    // The step machinery
    // ------------------------------------------------------------------

    fn index_of(&self, id: NodeId) -> usize {
        let index = usize::try_from(id.0).expect("node ids are small");
        assert!(
            index < self.nodes.len(),
            "the script named n={id}, outside the cluster",
            id = id.0
        );
        index
    }

    /// One step: plan, publish, route the released effects, record the
    /// trace line, run the legality gate. Every input a node receives comes
    /// through here, so the gate stands after every step by construction.
    fn drive(&mut self, id: NodeId, summary: String, input: Input) -> StepOutcome {
        let index = self.index_of(id);
        let timed = TimedInput {
            at: self.tick,
            event: input,
        };
        let (outcome, released) = match self.nodes[index].as_mut() {
            None => (StepOutcome::NodeDown, Vec::new()),
            Some(node) => {
                let planned = node.replica.plan(&timed, &node.replica.journal().view());
                match planned {
                    Err(rejection) => (StepOutcome::PlanRefused(rejection), Vec::new()),
                    Ok(plan) => match node.replica.publish(plan) {
                        Err(rejection) => (StepOutcome::PublishRefused(rejection), Vec::new()),
                        Ok(PublishOutcome::Published { revision, effects }) => {
                            let released = effects.clone();
                            (StepOutcome::Published { revision, effects }, released)
                        }
                        Ok(PublishOutcome::Parked { revision, effects }) => {
                            (StepOutcome::Parked { revision }, effects)
                        }
                    },
                }
            }
        };
        for effect in released {
            self.route_effect(index, effect);
        }
        self.record(format!("{summary} -> {outcome:?}"));
        outcome
    }

    /// Routes one released effect to where the host would take it: `Send`
    /// to the network, `Apply` to the node's pending list, `Reply` to the
    /// reply log, `Persist` to the node's outstanding intent.
    fn route_effect(&mut self, from: usize, effect: Effect) {
        match effect {
            Effect::Send { to, era, message } => {
                self.network.route(Envelope {
                    from: self.genesis_order[from],
                    to,
                    era,
                    message,
                });
            }
            apply @ Effect::Apply { .. } => {
                if let Some(node) = self.nodes[from].as_mut() {
                    node.pending_effects.push(apply);
                }
            }
            Effect::Reply {
                client,
                request,
                result,
            } => {
                let node = self.genesis_order[from];
                self.replies.push((node, client, request, result));
                self.boundary.push(BoundaryEvent::Replied {
                    node,
                    client,
                    request,
                });
            }
            Effect::Persist(intent) => {
                if let Some(node) = self.nodes[from].as_mut() {
                    node.outstanding_intent = Some(intent.revision);
                }
            }
        }
    }

    /// Appends one trace line and runs the legality gate. Every harness
    /// action — delivery, injection, tick, lifecycle, network change — ends
    /// here, so the gate's "after every step" is structural, not remembered.
    fn record(&mut self, body: String) {
        self.step_seq += 1;
        self.push_trace(format!("#{} t={} {body}", self.step_seq, self.tick.0));
        self.scan_faults();
    }

    fn push_trace(&mut self, line: String) {
        if self.trace.len() == TRACE_CAPACITY {
            self.trace.pop_front();
        }
        self.trace.push_back(line);
    }

    /// The legality gate: any node that faulted since the last scan must
    /// have been declared. An undeclared fault fails with the full trace —
    /// "illegal transition forces the node crashed" made loud.
    fn scan_faults(&mut self) {
        let mut newly_faulted: Vec<(usize, Fault)> = Vec::new();
        for (index, slot) in self.nodes.iter().enumerate() {
            if let Some(node) = slot
                && !self.faulted_known[index]
                && let Some(fault) = node.replica.progress().fault()
            {
                newly_faulted.push((index, fault));
            }
        }
        for (index, fault) in newly_faulted {
            if self.declared_faults[index] {
                self.declared_faults[index] = false;
                self.faulted_known[index] = true;
                self.push_trace(format!("  n={index} faulted as declared: {fault:?}"));
            } else {
                let dump = self.trace_dump();
                panic!("undeclared fault on n={index}: {fault:?}\nstep trace:\n{dump}");
            }
        }
    }
}

/// A whole-history copy of a journal, for the harness's disk. The harness
/// never reclaims, so the copy is always complete.
fn clone_journal(journal: &SegmentedLog) -> SegmentedLog {
    let view = journal.view();
    let mut clone = SegmentedLog::new();
    if let Some(frontier) = view.accepted() {
        let mut entries = Vec::new();
        match view.copy_out(VOID_SLOT, frontier, &mut entries) {
            RangeOutcome::Complete => {}
            RangeOutcome::Short { .. }
            | RangeOutcome::BelowRetention { .. }
            | RangeOutcome::Empty => {
                panic!("the harness never reclaims: its disks are whole")
            }
        }
        clone
            .install_suffix(VOID_SLOT, &entries)
            .expect("a whole-history copy installs on an empty log");
    }
    clone
}
