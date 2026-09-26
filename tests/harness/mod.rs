//! The scripted step-through harness: the deterministic host the Definition
//! of Done runs on.
//!
//! The harness is the host (docs/architecture.md: the host owns time,
//! transport, storage and packetization). It drives [`Replica`]s through the
//! plan/publish pipeline one explicit step at a time, a delivery, an
//! injection, a tick, a restart, and the *script* is the only source of
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
//! from observations, journals and the harness's own apply records, an
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
//!    history is a prefix of the other, entries identical at every shared
//!    slot, era and payload included.
//! 4. **Applied agreement**: applied slots are contiguous from the first
//!    operation slot (`INIT_SLOT + 1`), and no two nodes applied different
//!    payloads at the same slot.
//!
//! # What is not here
//!
//! The non-stop-the-world reconfiguration pivot (§8.7.6–§8.7.7) is still
//! refused with the named `PlanRefusal::Unsupported`. Normal operation
//! is live:
//! `Prepare`/`PrepareOk`/`Commit`, the Propose/Apply/Applied boundary
//! (§11.1), the bootstrap from the fenced `Joining` genesis state, view
//! change, recovery (§6.1), state transfer (§10, §13.1 step 5), the
//! checkpoint frontier and the lazy reclamation it authorizes (§4, §11,
//! S1), and the host-forced view change (§14.2).
//!
//! # Reclamation
//!
//! The harness mirrors the host's retention policy (S1): after any step
//! that appended to a node's journal it offers the node the reclamation
//! opportunity, and the default journal drops whole slabs whose final slot
//! the PUBLISHED checkpoint frontier covers, the sole authorization (§4,
//! §11). No published checkpoint, no reclamation, however old the history.

// The harness is shared infrastructure compiled into every protocol test
// target; no single target drives the whole scripted surface, the suites
// do, collectively. Dead-code analysis runs per target, so the documented
// allowance lives here at the module root rather than scattered per method.
#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vrr::configuration::{EraTable, INIT_SLOT, SystemOperation, VOID_SLOT};
use vrr::effects::{Effect, PlanVerdict, Stability, StabilityResult};
use vrr::ids::{
    CrashCounter, Era, Fault, NodeId, Operation, OperationId, Slot, SystemId, Tick, View, ViewId,
};
use vrr::journal::{Journal, JournalView, LogEntry, Payload, RangeOutcome, SegmentedLog};
use vrr::lifecycle::{
    BootError, BootOutcome, CopyState, Crashed, LifecycleStore, Marker, Running, SuperblockCopies,
    boot,
};
use vrr::message::Message;
use vrr::observe::Diagnostic;
use vrr::plan::Plan;
use vrr::progress::{Progress, ProgressSnapshot, Status};
use vrr::quorum::WeightedMajority;
use vrr::reconfiguration::Abdication;
use vrr::replica::{
    Input, LifecycleRefusal, Observer, PersistedProgress, Pivot, PlanRefusal, PublishOutcome,
    PublishRefusal, Replica, TimedInput, ViewChangeKnobs,
};
use vrr::wire::Tag;

/// The boot-gate root sequence: one unique temporary root per harness, so
/// parallel test targets never share marker directories.
static GATE_ROOT_SEQ: AtomicU64 = AtomicU64::new(0);

/// A lawful random pair, minted strictly inside the identity space: both
/// halves in `1..u16::MAX` with two lives of bump headroom below the
/// bound, drawn from a xorshift seeded by the clock and the process id. No memorised value can satisfy the rules by accident:
/// a test mints its pair once at the start, runs its whole flow, and then
/// checks the mint was not violated, so a hardcoded identity hiding as a
/// fake satisfaction of the rules is impossible by construction.
///
/// # Panics
///
/// Never: the xorshift never yields zero and the range arithmetic cannot
/// overflow.
#[must_use]
pub fn mint_pair() -> (SystemId, CrashCounter) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0x9e37_79b9);
    let mut state = nanos ^ (u64::from(std::process::id()) << 32) | 1;
    let draw = |state: &mut u64| -> u16 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        // Strictly inside with two lives of headroom: 1..u16::MAX - 2, so
        // neither the zero edge nor the exhaustion edge is ever minted by
        // accident, and two lawful bumps always exist below the bound.
        u16::try_from((*state % (u16::MAX as u64 - 2)) + 1).unwrap_or(1)
    };
    let system = SystemId::new(draw(&mut state)).expect("the draw is non-zero");
    let crash = CrashCounter::new(draw(&mut state)).expect("the draw is non-zero");
    (system, crash)
}

/// A lawful random identity: the pair, packed.
#[must_use]
pub fn mint_id() -> NodeId {
    let (system, crash) = mint_pair();
    NodeId::new(system, crash)
}

/// The replica configuration every harness node runs: the default journal
/// and the default quorum strategy. Overridable per-node configuration is a
/// later milestone's concern; nothing in the current suites needs it.
pub type HarnessReplica = Replica<SegmentedLog, WeightedMajority>;

/// Step-trace ring capacity. A failure prints the whole buffer; a script
/// longer than this still shows the breaking step and its neighbourhood.
const TRACE_CAPACITY: usize = 512;

// ----------------------------------------------------------------------
// The boot gate's test store
// ----------------------------------------------------------------------

/// Blank marker content: the version stamp, identity 0, nothing vouched.
/// Identity 0 is never a live identity (a lawful pair is non-zero in both
/// halves), so a blank copy cannot win a working cohort and can never read
/// as `Stopped`. The version stamp is the format contract: a marker file
/// without it predates the durable identity pair and is refused as
/// incompatible, never silently migrated.
const GATE_BLANK: &str = "v1\n0\n-\n";

/// One marker file's name, by copy index.
fn gate_file(index: usize) -> String {
    format!("copy{index}")
}

/// The marker's on-disk name.
fn gate_marker_name(marker: Marker) -> &'static str {
    match marker {
        Marker::Stopping => "Stopping",
        Marker::Stopped => "Stopped",
        Marker::Restarting => "Restarting",
        Marker::Joining => "Joining",
    }
}

/// The harness's [`LifecycleStore`] (`docs/uvrr-boot-gate.md` §6): plain
/// marker files in the harness's temporary directory, four copies, one
/// file each, plus a WAL stub the drain forces. Every read, write, and
/// drain is recorded on the operation log in order, so a script can assert
/// the boot gate's fixed write schedules against it.
pub struct TmpGate {
    dir: PathBuf,
    ops: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl TmpGate {
    /// Opens the gate over `dir`, creating the four blank copies when the
    /// directory holds none, the first life has no durable identity yet.
    pub fn open(dir: PathBuf) -> std::io::Result<TmpGate> {
        fs::create_dir_all(&dir)?;
        for index in 0..4 {
            let path = dir.join(gate_file(index));
            if !path.exists() {
                fs::write(&path, GATE_BLANK)?;
            }
        }
        Ok(TmpGate {
            dir,
            ops: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        })
    }

    /// The shared operation log: the harness retains the handle, so the
    /// log survives the session that wrote it (a halt consumes its
    /// session; the schedule it wrote stays assertable).
    #[must_use]
    pub fn ops_handle(&self) -> std::rc::Rc<std::cell::RefCell<Vec<String>>> {
        std::rc::Rc::clone(&self.ops)
    }

    /// The operation log, in order: `read`, `commit:<marker>@<identity>`,
    /// `drain`. The fixed schedules of `docs/uvrr-boot-gate.md` §3 assert
    /// against this.
    #[must_use]
    pub fn ops(&self) -> Vec<String> {
        self.ops.borrow().clone()
    }

    /// The directory this gate owns, the physical superblock.
    #[must_use]
    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    fn read_one(&self, index: usize) -> std::io::Result<CopyState> {
        let text = fs::read_to_string(self.dir.join(gate_file(index)))?;
        let text = text.trim_end_matches('\n');
        let mut lines = text.split('\n');
        // The format stamp rules: a file without `v1` predates the durable
        // identity pair and is refused, never migrated in place.
        if lines.next() != Some("v1") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "incompatible marker format: the file predates the durable identity pair",
            ));
        }
        let identity_line = lines.next().unwrap_or("0");
        let marker_line = lines.next().unwrap_or("-");
        let identity = identity_line.parse::<u32>().unwrap_or(0);
        let marker = match marker_line {
            "Stopping" => Marker::Stopping,
            "Stopped" => Marker::Stopped,
            "Restarting" => Marker::Restarting,
            "Joining" => Marker::Joining,
            // Blank: nothing vouched, `Stopping` with identity 0 never
            // reads as a stop and never wins a working cohort.
            "-" => Marker::Stopping,
            _ => Marker::Stopping,
        };
        // A zero half under a real marker state is a corrupt marker, not
        // an identity: the pair is one-indexed in both halves
        // (`docs/uvrr-boot-gate.md` §5), so the read refuses rather than
        // classifies over a pattern no lawful life ever wrote.
        if identity == 0 && marker_line != "-" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "corrupt marker: a zero half is no identity",
            ));
        }
        Ok(CopyState {
            identity: NodeId(identity),
            marker,
        })
    }
}

impl LifecycleStore for TmpGate {
    type Error = std::io::Error;

    fn read_copies(&mut self) -> Result<Option<SuperblockCopies>, Self::Error> {
        self.ops.borrow_mut().push("read".to_string());
        let copies = [
            self.read_one(0)?,
            self.read_one(1)?,
            self.read_one(2)?,
            self.read_one(3)?,
        ];
        if copies.iter().all(|copy| copy.identity == NodeId(0)) {
            return Ok(None);
        }
        Ok(Some(SuperblockCopies { copies }))
    }

    fn commit(&mut self, copies: &SuperblockCopies) -> Result<(), Self::Error> {
        for (index, copy) in copies.copies.iter().enumerate() {
            let path = self.dir.join(gate_file(index));
            let mut file = fs::File::create(&path)?;
            file.write_all(
                format!(
                    "v1\n{}\n{}\n",
                    copy.identity.0,
                    gate_marker_name(copy.marker)
                )
                .as_bytes(),
            )?;
            file.sync_all()?;
        }
        self.ops.borrow_mut().push(format!(
            "commit:{}@{}",
            gate_marker_name(copies.copies[0].marker),
            copies.copies[0].identity.0
        ));
        Ok(())
    }

    fn drain(&mut self) -> Result<(), Self::Error> {
        let mut wal = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("wal"))?;
        wal.write_all(b"drain\n")?;
        wal.sync_all()?;
        self.ops.borrow_mut().push("drain".to_string());
        Ok(())
    }
}

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
    /// The admin verdicts released to the node's admin perimeter, in
    /// release order.
    admin_verdicts: Vec<PlanVerdict>,
}

/// What the harness's "disk" recorded at crash time: the published progress
/// record, the journal, and the configuration history. The parked candidate
/// of an outstanding intent is volatile and dies with the node, correctly:
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
/// crossing datagrams, never drops them, so a script can inspect, deliver
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
    /// flows, the sets name the two sides, not the survivors.
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
    PlanRefused(PlanRefusal),
    /// `publish` refused the planned transition.
    PublishRefused(PublishRefusal),
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
/// harness executed. The record the §11.1 boundary assertions (identity
/// carried through, slot order, no deduplication) are stated over.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BoundaryEvent {
    /// The harness performed an `Apply` effect and fed `Applied` back.
    Applied {
        /// The node whose application ran.
        node: NodeId,
        /// The applied slot.
        slot: Slot,
        /// The identity the proposing host assigned (§11.1), carried opaque.
        operation_id: OperationId,
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
    /// Its configuration history, primary beliefs are computed under the
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
    /// operation slot.
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
    // Rule 1, frontier sanity on every observation.
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

    // Rule 2, single primary per view. A node believes itself primary when
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

    // Rule 3, committed-prefix agreement: identical entries at every shared
    // slot, so one history is a prefix of the other. Slots align the
    // comparison: a node whose host retention policy let a committed prefix
    // go (S1) serves only its retained window.
    for (position, a) in nodes.iter().enumerate() {
        for b in &nodes[position + 1..] {
            for left in &a.committed {
                let Some(right) = b.committed.iter().find(|entry| entry.slot == left.slot) else {
                    continue;
                };
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

    // Rule 4, applied agreement and contiguity from the first operation
    // slot: slots 1 and 2 are the genesis system operations (§8.7.2), which
    // the core folds and the application never sees.
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
    /// The view-change knobs every node runs (W5), threaded
    /// through restarts, because a restarted node plays by the cluster's
    /// rules, not fresh ones.
    knobs: ViewChangeKnobs,
    /// The journal tail capacity every node runs, when a script pinned it:
    /// slab boundaries decide what reclamation can drop (whole slabs
    /// only), so a reclamation script must place them deterministically.
    /// `None` is the default journal's own capacity.
    tail_capacity: Option<usize>,
    /// Genesis order, as provisioned: the first life of system `i + 1`
    /// (crash counter 1, packed) at slot `i`.
    genesis_order: Vec<NodeId>,
    /// The identity-to-slot mapping: every identity the script ever names
    /// (genesis members, reincarnated lives, fresh joiners) occupies one
    /// slot, keyed on the packed identity itself rather than on arithmetic
    /// over its integer value.
    slots: BTreeMap<NodeId, usize>,
    /// The reverse mapping: the identity each slot belongs to, so the
    /// index-ordered walks (the tick sweep, the safety scan, the legality
    /// gate) can name the node they visit.
    slot_ids: Vec<NodeId>,
    /// Per-node record of the `Apply` effects the harness executed.
    applied: Vec<Vec<(Slot, Box<[u8]>)>>,
    /// The ordered Apply boundary record (§11.1 assertions).
    boundary: Vec<BoundaryEvent>,
    /// Per-node durable state recorded at crash time.
    disks: Vec<Option<Disk>>,
    /// The boot-gate root: one marker directory per node index, the
    /// physical superblocks.
    gate_root: PathBuf,
    /// The marker directory each index's current identity owns, the
    /// directory a restart boots over. Persists across crash and halt,
    /// exactly like the disk.
    gates: Vec<Option<PathBuf>>,
    /// The live node's gate session: the running process's marker store,
    /// consumed by [`Harness::halt`]. A node still awaiting its deferred
    /// latch (a reincarnation not yet seated) holds `None` here and its
    /// session in [`Harness::pending`] instead.
    sessions: Vec<Option<Running<TmpGate>>>,
    /// The retained operation log of each index's most recent gate,
    /// survives the session that wrote it, so a halt's schedule stays
    /// assertable after the session is consumed.
    gate_logs: Vec<Option<std::rc::Rc<std::cell::RefCell<Vec<String>>>>>,
    /// A reincarnation's deferred latch: the crashed session, held until
    /// the engine's seated observation mints the witness
    /// (`docs/uvrr-boot-gate.md` §3).
    pending: Vec<Option<Crashed<TmpGate>>>,
    /// Per-node pending fault declarations (`expect_fault`).
    declared_faults: Vec<bool>,
    /// Per-node faults already observed and accounted for.
    faulted_known: Vec<bool>,
    /// The step trace ring.
    trace: VecDeque<String>,
    /// Steps taken, also the trace line number.
    step_seq: u64,
}

impl Harness {
    /// `n` provisioned nodes, volatile stability, `SegmentedLog`,
    /// `WeightedMajority`, genesis order the first lives of systems 1
    /// through `n` (system `i + 1`, crash counter 1, packed).
    ///
    /// View-change knobs: suspicion disabled, unbounded suffix, the
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
        Self::assemble(n, Stability::Volatile, knobs, None, &[])
    }

    /// [`Harness::provision`] with an explicit stability level. In every
    /// non-volatile mode the harness records each `Persist` intent and the
    /// script must confirm it explicitly with [`Harness::confirm`], the
    /// real host contract, mirrored.
    #[must_use]
    pub fn with_stability(n: usize, stability: Stability) -> Harness {
        Self::assemble(n, stability, Harness::no_view_change_knobs(), None, &[])
    }

    /// [`Harness::provision`] with an explicit journal tail capacity. Slab
    /// boundaries decide what checkpoint-authorized reclamation can drop
    /// (whole slabs only, §4), so a reclamation script pins the capacity
    /// rather than inheriting the default journal's.
    #[must_use]
    pub fn with_journal_capacity(n: usize, tail_capacity: usize) -> Harness {
        Self::assemble(
            n,
            Stability::Volatile,
            Harness::no_view_change_knobs(),
            Some(tail_capacity),
            &[],
        )
    }

    /// [`Harness::with_journal_capacity`] with explicit view-change knobs:
    /// a reclamation script that also drives view changes needs both,
    /// slab boundaries decide what reclamation can drop, the timeout
    /// decides when suspicion fires.
    #[must_use]
    pub fn with_knobs_and_journal_capacity(
        n: usize,
        knobs: ViewChangeKnobs,
        tail_capacity: usize,
    ) -> Harness {
        Self::assemble(n, Stability::Volatile, knobs, Some(tail_capacity), &[])
    }

    /// [`Harness::with_stability`] with an explicit journal tail capacity:
    /// a shortfall script under an external-stability mode needs both,
    /// the stability level parks every transition behind its persistence
    /// intent (S2), and the pinned slab boundaries decide what
    /// checkpoint-authorized reclamation can drop (§4).
    #[must_use]
    pub fn with_stability_and_journal_capacity(
        n: usize,
        stability: Stability,
        tail_capacity: usize,
    ) -> Harness {
        Self::assemble(
            n,
            stability,
            Harness::no_view_change_knobs(),
            Some(tail_capacity),
            &[],
        )
    }

    /// [`Harness::with_knobs`] with a statically registered gossip-witness
    /// list (`docs/uvrr-rejoin-gossip-and-witnesses.md` §4): every
    /// provisioned node loads `witnesses` at startup. The witnesses
    /// themselves are outside the roster, they are not provisioned here;
    /// a script that wants one to process the stream boots it itself.
    #[must_use]
    pub fn with_static_witnesses(
        n: usize,
        knobs: ViewChangeKnobs,
        witnesses: &[NodeId],
    ) -> Harness {
        Self::assemble(n, Stability::Volatile, knobs, None, witnesses)
    }

    /// The knob setting that makes the view-change machinery inert: no
    /// suspicion ever fires, and suffixes are never truncated.
    fn no_view_change_knobs() -> ViewChangeKnobs {
        ViewChangeKnobs {
            primary_timeout: 0,
            view_change_budget: usize::MAX,
        }
    }

    fn assemble(
        n: usize,
        stability: Stability,
        knobs: ViewChangeKnobs,
        tail_capacity: Option<usize>,
        witnesses: &[NodeId],
    ) -> Harness {
        let genesis_order: Vec<NodeId> = (0..n)
            .map(|i| {
                NodeId::new(
                    SystemId::new(
                        u16::try_from(i + 1).expect("cluster size fits the system-id space"),
                    )
                    .expect("a one-indexed system id is non-zero"),
                    CrashCounter::new(1).expect("one is non-zero"),
                )
            })
            .collect();
        let mut nodes = Vec::with_capacity(n);
        let gate_root = std::env::temp_dir().join(format!(
            "uvrr-harness-{}-{}",
            std::process::id(),
            GATE_ROOT_SEQ.fetch_add(1, Ordering::SeqCst),
        ));
        let mut gates = Vec::with_capacity(n);
        let mut sessions = Vec::with_capacity(n);
        let mut gate_logs = Vec::with_capacity(n);
        let mut pending: Vec<Option<Crashed<TmpGate>>> = Vec::with_capacity(n);
        for (index, &id) in genesis_order.iter().enumerate() {
            let replica = Replica::provision(
                id,
                genesis_order.clone(),
                WeightedMajority,
                make_journal(tail_capacity),
                stability,
                knobs,
            )
            .map(|replica| replica.with_witnesses(witnesses))
            .expect("the genesis order 0..n provisions");
            let observer = replica.observer();
            // The first life latches its anchor: `(identity, Joining)` 4x
            // (`docs/uvrr-boot-gate.md` §3), so a later crash reads as a
            // crash, never as a clean stop.
            let dir = gate_root.join(format!("n{index}"));
            let gate = TmpGate::open(dir.clone()).expect("the gate directory opens");
            let mut session = match boot(gate) {
                Ok(BootOutcome::First(first)) => first.latch(id).expect("the first latch writes"),
                _ => panic!("a fresh gate directory classifies first"),
            };
            gate_logs.push(Some(session.store_mut().ops_handle()));
            gates.push(Some(dir));
            sessions.push(Some(session));
            pending.push(None);
            nodes.push(Some(Node {
                replica,
                observer,
                pending_effects: Vec::new(),
                outstanding_intent: None,
                admin_verdicts: Vec::new(),
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
            tail_capacity,
            slots: genesis_order
                .iter()
                .enumerate()
                .map(|(index, &id)| (id, index))
                .collect(),
            slot_ids: genesis_order.clone(),
            genesis_order,
            applied: (0..n).map(|_| Vec::new()).collect(),
            boundary: Vec::new(),
            disks: (0..n).map(|_| None).collect(),
            gate_root,
            gates,
            sessions,
            gate_logs,
            pending,
            declared_faults: vec![false; n],
            faulted_known: vec![false; n],
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

    /// The host-forced view change (§14.2): drives
    /// `Input::AdminForceView { target }` through the ordinary step
    /// machinery, the refusal or the fence is the script's to assert.
    pub fn force_view(&mut self, id: NodeId, target: ViewId) -> StepOutcome {
        self.drive(
            id,
            format!(
                "n={} admin force-view e{}v{}",
                id.0, target.era.0, target.view.0
            ),
            Input::AdminForceView { target },
        )
    }

    /// The administrator's abdication (rules §12 of
    /// `docs/uvrr-reconfiguration-rules.md`): `Input::Abdicate` through the
    /// ordinary step machinery, the named refusal or the standard
    /// view-change emission's publication is the script's to assert.
    pub fn abdicate(&mut self, id: NodeId, message: Abdication) -> StepOutcome {
        self.drive(
            id,
            format!(
                "n={} abdicate e{}v{}->v{}",
                id.0, message.current.era.0, message.current.view.0, message.target.0
            ),
            Input::Abdicate { message },
        )
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
                let id = self.slot_ids[index];
                let outcome = self.drive(id, format!("n={} tick", id.0), Input::Tick);
                results.push((id, outcome));
            }
        }
        results
    }

    // ------------------------------------------------------------------
    // Network
    // ------------------------------------------------------------------

    /// Enqueues a datagram as if from a peer, the way a script fabricates
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
    /// `None` when the queue is empty, no step, no trace line.
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
    /// tag and header slot, the way a script stages an out-of-order or
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
    /// see, a fabrication names its claimed author.
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

    /// Explicitly drops the held datagrams. Recorded in the trace, a drop
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
    /// without delivering it, how a script inspects the evidence a node
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
    /// view-change install left behind, asserted directly, a divergent
    /// tail must be GONE from the journal, not merely from the frontier.
    /// Only meaningful on a script that never published a checkpoint: the
    /// copy starts at slot 1, and a reclaimed prefix fails loudly.
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
                panic!("journal_entries requires an unreclaimed journal (no checkpoint published)")
            }
        }
    }

    /// The node's physical retention window, first and last slot the
    /// journal still holds (§4). This is what checkpoint-authorized
    /// reclamation moves; the logical accepted frontier never moves with
    /// it. `None` if the node is down.
    #[must_use]
    pub fn retained(&self, id: NodeId) -> Option<(Slot, Slot)> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| node.replica.journal().view().retained())
    }

    /// The journal entry at `slot`, or `None` when the slot is past the
    /// frontier or was reclaimed under a published checkpoint (§4).
    #[must_use]
    pub fn journal_entry(&self, id: NodeId, slot: Slot) -> Option<LogEntry> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .and_then(|node| node.replica.journal().view().get(slot).cloned())
    }

    /// The node's configuration history (§8.7.1), the record
    /// reconfiguration scripts assert over (the era, the establishing
    /// slot, the member weights). `None` if the node is down.
    #[must_use]
    pub fn era_table(&self, id: NodeId) -> Option<Arc<EraTable>> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| Arc::clone(node.replica.progress().config()))
    }

    /// The node's gossip-witness list (a copy, so the script can assert on
    /// it freely). Empty for a node that is down or absent, a down node
    /// carries no observable list.
    #[must_use]
    pub fn witnesses(&self, id: NodeId) -> Vec<NodeId> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| node.replica.witnesses().to_vec())
            .unwrap_or_default()
    }

    /// The node's sticky fault, if declared, the identity, not just the
    /// `faulted` word the observation carries (the `expect_fault`
    /// scripts assert WHICH fault the breach declared).
    #[must_use]
    pub fn fault_of(&self, id: NodeId) -> Option<Fault> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
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

    /// The held datagrams, as `(from, to, tag)`, inspectable, per the
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
    // Operation and stability inputs
    // ------------------------------------------------------------------

    /// Proposes an operation to a specific node (§6): the host assigns the
    /// operation's identity, and the core carries it opaque, there is no
    /// request numbering to manage, because the core never deduplicates
    /// (§11.1, B2).
    pub fn propose(
        &mut self,
        id: NodeId,
        operation_id: OperationId,
        payload: &[u8],
    ) -> StepOutcome {
        let summary = format!(
            "n={} op={:016x}:{:016x}",
            id.0, operation_id.msb, operation_id.lsb
        );
        self.drive(
            id,
            summary,
            Input::Propose {
                operation: Operation {
                    id: operation_id,
                    payload: payload.into(),
                },
            },
        )
    }

    /// A reconfiguration proposal (§8.7.2): `Input::Reconfigure` through
    /// the ordinary step machinery, the named refusal or the proposal's
    /// publication is the script's to assert. The pivot is `None` on the
    /// stop-the-world path (§8.7.4); a `Some` pivot runs the non-stop
    /// overlap transition (§8.7.6–§8.7.7).
    pub fn reconfigure(
        &mut self,
        id: NodeId,
        op: SystemOperation,
        pivot: Option<Pivot>,
    ) -> StepOutcome {
        self.drive(
            id,
            format!("n={} reconfigure {op:?}", id.0),
            Input::Reconfigure { op, pivot },
        )
    }

    /// Feeds an `Input::Applied` the harness's own apply execution did not
    /// generate, the way a script stages a duplicate, out-of-order or
    /// not-yet-committed acknowledgement (§11.1). The refusal is the
    /// script's to assert. One step.
    pub fn report_applied(&mut self, id: NodeId, slot: Slot) -> StepOutcome {
        self.drive(
            id,
            format!("n={} applied s{} (scripted)", id.0, slot.0),
            Input::Applied { slot },
        )
    }

    /// The host's checkpoint report (§5, §11): `Input::Checkpointed`
    /// through the ordinary step machinery, the refusal or the frontier
    /// advance is the script's to assert. The published frontier is the
    /// sole reclamation authorization (§4, S1); the drop itself is lazy,
    /// fired by a later append, never by this report alone.
    pub fn checkpoint(&mut self, id: NodeId, through: Slot) -> StepOutcome {
        self.drive(
            id,
            format!("n={} checkpoint s{}", id.0, through.0),
            Input::Checkpointed { through },
        )
    }

    /// Confirms the node's one outstanding `Persist` intent, the external-
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

    /// Executes the node's pending `Apply` effects in release order, feeding
    /// each completion back as `Input::Applied` (§11.1). The acknowledgement
    /// carries no result: the boundary is one-way, propose to apply, and the
    /// core never answers a proposal (B2). The echo application records
    /// every execution in slot order, the core does not deduplicate, so
    /// neither does the log the safety checker's contiguity rule rules on.
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
                    let Effect::Apply {
                        slot,
                        operation_id,
                        payload,
                    } = apply
                    else {
                        unreachable!("the pattern matched Apply")
                    };
                    self.applied[index].push((slot, payload.clone()));
                    self.boundary.push(BoundaryEvent::Applied {
                        node: id,
                        slot,
                        operation_id,
                    });
                    let outcome = self.drive(
                        id,
                        format!("n={} apply s{}", id.0, slot.0),
                        Input::Applied { slot },
                    );
                    outcomes.push(ApplyOutcome {
                        slot,
                        payload,
                        outcome,
                    });
                }
                Effect::Send { .. } | Effect::Persist(_) | Effect::AdminResponse { .. } => {
                    panic!("only Apply effects are routed to a node's pending list")
                }
            }
        }
        outcomes
    }

    /// Submits a reconfiguration plan over the node's admin ingress
    /// (`docs/weighted-reconfiguration-solver.md`): the verdict or the
    /// named refusal is the script's to assert. One step.
    pub fn submit_plan(&mut self, id: NodeId, plan: Plan) -> StepOutcome {
        self.drive(
            id,
            format!("n={} submit-plan ({} steps)", id.0, plan.steps.len()),
            Input::SubmitPlan { plan },
        )
    }

    /// The admin verdicts the node has released to its admin perimeter, in
    /// release order. Empty if the node is down or answered nothing.
    #[must_use]
    pub fn admin_verdicts(&self, id: NodeId) -> &[PlanVerdict] {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| node.admin_verdicts.as_slice())
            .unwrap_or(&[])
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    /// Crashes a node: its volatile state dies with it, and the harness's
    /// "disk" records the published progress, the journal, and the
    /// configuration history. The markers are left exactly as they were,
    /// a crash writes nothing (`docs/uvrr-boot-gate.md` §1). Deliveries to
    /// a down node are recorded undeliverable.
    pub fn crash(&mut self, id: NodeId) {
        let index = self.index_of(id);
        let node = self.nodes[index]
            .take()
            .unwrap_or_else(|| panic!("n={} is already down", id.0));
        self.sessions[index] = None;
        self.pending[index] = None;
        self.disks[index] = Some(Disk {
            persisted: PersistedProgress::from(node.replica.progress()),
            journal: clone_journal(node.replica.journal(), self.tail_capacity),
            config: Arc::clone(node.replica.progress().config()),
        });
        self.faulted_known[index] = false;
        self.declared_faults[index] = false;
        self.record(format!("n={} crash (disk recorded)", id.0));
    }

    /// A controlled halt (`docs/uvrr-boot-gate.md` §3): the two-round stop
    /// driven through the gate, `Stopping` 4x, the drain, `Stopped` 4x,
    /// then the disk records the drained state. This is the only stop
    /// whose restart resumes the same identity.
    pub fn halt(&mut self, id: NodeId) {
        let index = self.index_of(id);
        let node = self.nodes[index]
            .take()
            .unwrap_or_else(|| panic!("n={} is already down", id.0));
        let session = self.sessions[index]
            .take()
            .unwrap_or_else(|| panic!("n={} has not latched its identity; a node still awaiting its deferred latch cannot halt, it can only crash", id.0));
        let session = session
            .begin_stop()
            .expect("round one writes")
            .drain()
            .expect("the drain forces")
            .finish_stop()
            .expect("round two writes");
        self.disks[index] = Some(Disk {
            persisted: PersistedProgress::from(node.replica.progress()),
            journal: clone_journal(node.replica.journal(), self.tail_capacity),
            config: Arc::clone(node.replica.progress().config()),
        });
        self.faulted_known[index] = false;
        self.declared_faults[index] = false;
        drop(session);
        self.record(format!("n={} halt (drained, markers flushed)", id.0));
    }

    /// Crashes a node like [`Harness::crash`], but the recorded disk's
    /// journal physically retains only slots from `base` onward: a host
    /// retention policy (S1) let the earlier prefix go while the logical
    /// frontier stands. This is how a script stages a retained-base
    /// shortfall (§4).
    pub fn crash_with_retained_base(&mut self, id: NodeId, base: Slot) {
        let index = self.index_of(id);
        let node = self.nodes[index]
            .take()
            .unwrap_or_else(|| panic!("n={} is already down", id.0));
        self.sessions[index] = None;
        self.pending[index] = None;
        let view = node.replica.journal().view();
        let frontier = view
            .accepted()
            .expect("a node with no history retains nothing to truncate");
        assert!(
            base > VOID_SLOT && base <= frontier,
            "retained base {base:?} must sit inside the history ..={frontier:?}",
        );
        let mut entries = Vec::new();
        match view.copy_out(base, frontier, &mut entries) {
            RangeOutcome::Complete => {}
            other => panic!("the retained range is whole by construction: {other:?}"),
        }
        let mut journal = make_journal(self.tail_capacity);
        journal
            .install_suffix(base, &entries)
            .expect("a contiguous range installs on a pristine log, anchoring its window there");
        self.disks[index] = Some(Disk {
            persisted: PersistedProgress::from(node.replica.progress()),
            journal,
            config: Arc::clone(node.replica.progress().config()),
        });
        self.faulted_known[index] = false;
        self.declared_faults[index] = false;
        self.record(format!(
            "n={} crash (disk recorded, retained from s{})",
            id.0, base.0
        ));
    }

    /// A later life: the same identity resumes over whatever the harness's
    /// disk recorded at halt time. A fault persisted in the record reopens
    /// faulted, that is evidence restored, not a new fault, so it does
    /// not trip the gate.
    ///
    /// The restart routes through the boot gate: the markers must classify
    /// `Clean`, a stopped quorum from a [`Harness::halt`]. A crashed
    /// marker set refuses loudly (error-on-crashed is the contract,
    /// `docs/uvrr-boot-gate.md` §7): the same identity cannot resume a
    /// crash.
    pub fn restart_with(&mut self, id: NodeId) -> Result<(), LifecycleRefusal> {
        let index = self.index_of(id);
        assert!(
            self.nodes[index].is_none(),
            "n={} is up; halt or crash it before restarting it",
            id.0
        );
        let dir = self.gates[index]
            .clone()
            .unwrap_or_else(|| panic!("n={} has no gate directory", id.0));
        let gate = TmpGate::open(dir).expect("the gate directory opens");
        let (mut session, vouched) = match boot(gate) {
            Ok(BootOutcome::Clean(clean)) => clean.latch().expect("the clean latch writes"),
            Ok(BootOutcome::Crashed { .. }) => panic!(
                "n={} restart_with refused: the markers classify crashed, error-on-crashed is the contract; halt the node for the clean path or restart_as for reincarnation",
                id.0
            ),
            Ok(BootOutcome::First { .. }) => panic!(
                "n={} restart_with refused: the markers hold no life to resume",
                id.0
            ),
            Err((_, BootError::QuorumLost)) => panic!(
                "n={} restart_with refused: the marker set is torn beyond the quorum read",
                id.0
            ),
            Err((_, error)) => panic!("n={} restart_with refused: {error:?}", id.0),
        };
        let (journal, persisted, config) = match self.disks[index].as_ref() {
            Some(disk) => (
                clone_journal(&disk.journal, self.tail_capacity),
                disk.persisted,
                Arc::clone(&disk.config),
            ),
            None => panic!("n={} has no recorded disk; halt or crash it first", id.0),
        };
        self.gate_logs[index] = Some(session.store_mut().ops_handle());
        match Replica::resume(
            vouched,
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
                self.sessions[index] = Some(session);
                self.record(format!("n={} restart(with vouched disk)", id.0));
                Ok(())
            }
            Err(error) => {
                self.record(format!("n={} restart(with disk) refused: {error:?}", id.0));
                Err(error)
            }
        }
    }

    /// The reincarnation restart (§2 of `docs/uvrr-reincarnation.md`): the
    /// dirty path made concrete. The node that ran as `id` lost its
    /// volatile state and reopened under a NEW identity `new`, same disk,
    /// same journal, new `own`. The new identity is not a member until the
    /// forced sequence joins it; addressable from this moment on.
    ///
    /// The restart routes through the boot gate: the markers must classify
    /// `Crashed`. The replacement pair is decided by the gate, and the
    /// durable latch DEFERS to the engine's seated observation, the
    /// harness latches automatically on the first step after which
    /// [`Replica::rejoined`] mints its witness (`docs/uvrr-boot-gate.md`
    /// §3).
    ///
    /// Identity is not reused: the identity that crashed is never resumed
    /// (same-identity recovery after volatile-state loss is
    /// unrepresentable), so the script names a fresh `new`.
    pub fn restart_as(&mut self, id: NodeId, new: NodeId) -> Result<(), LifecycleRefusal> {
        let old_index = self.index_of(id);
        assert!(
            self.nodes[old_index].is_none(),
            "n={} is up; crash it before reincarnating it",
            id.0
        );
        assert_ne!(id, new, "reincarnation always changes the identity");
        assert_eq!(
            new,
            id.next_life().expect("the bump never exhausts in tests"),
            "the reincarnated identity is the same system's next crash counter"
        );
        let dir = self.gates[old_index]
            .clone()
            .unwrap_or_else(|| panic!("n={} has no gate directory", id.0));
        let gate = TmpGate::open(dir.clone()).expect("the gate directory opens");
        let crashed = match boot(gate) {
            Ok(BootOutcome::Crashed(crashed)) => crashed,
            Ok(BootOutcome::Clean { .. }) => panic!(
                "n={} restart_as refused: the markers classify a clean stop, restart_with is the path",
                id.0
            ),
            Ok(BootOutcome::First { .. }) => panic!(
                "n={} restart_as refused: the markers hold no life to reincarnate",
                id.0
            ),
            Err((_, BootError::QuorumLost)) => panic!(
                "n={} restart_as refused: the marker set is torn beyond the quorum read",
                id.0
            ),
            Err((_, error)) => panic!("n={} restart_as refused: {error:?}", id.0),
        };
        self.gate_logs[old_index] = Some(crashed.store().ops_handle());
        let pair = crashed
            .pair()
            .expect("the identity is nowhere near exhaustion");
        let (journal, persisted, config) = match self.disks[old_index].as_ref() {
            Some(disk) => (
                clone_journal(&disk.journal, self.tail_capacity),
                disk.persisted,
                Arc::clone(&disk.config),
            ),
            None => panic!("n={} has no recorded disk; crash it first", id.0),
        };
        match Replica::reincarnate(
            pair,
            new,
            WeightedMajority,
            journal,
            persisted,
            config,
            self.stability,
            self.knobs,
        ) {
            Ok(replica) => {
                let new_index = self.grow_to(new);
                self.gates[new_index] = Some(dir);
                self.gate_logs[new_index] = self.gate_logs[old_index].take();
                self.pending[new_index] = Some(crashed);
                self.install(new_index, replica);
                self.record(format!("n={:?} restart_as(old n={:?})", new, id));
                Ok(())
            }
            Err(error) => {
                self.record(format!(
                    "n={new:?} restart_as(old n={id:?}) refused: {error:?}"
                ));
                Err(error)
            }
        }
    }

    /// Boots a fresh later life over the deployment's genesis knowledge,
    /// the boot a joining member gets: the shared committed genesis
    /// (slots 1–2, byte-for-byte what [`Replica::provision`] installs)
    /// journaled, the same genesis fold as the era table, and no progress
    /// beyond it. The node is transport-addressable and fenced
    /// (`Status::Restarting`) until the primary's stream proves currency;
    /// it is not a member of any configuration it can name, so a fresh
    /// join converges only through the §10 learner acquisition. The
    /// identity must not be a genesis member: a genesis member
    /// provisions instead, and booting it over genesis knowledge again
    /// would fork no history but claim a second life it never lived.
    pub fn boot_as(&mut self, id: NodeId) -> Result<(), LifecycleRefusal> {
        assert!(
            !self.genesis_order.contains(&id),
            "n={id:?} is a genesis member; only a joining identity boots"
        );
        let mut journal = make_journal(self.tail_capacity);
        let table = EraTable::genesis()
            .extend(&SystemOperation::Void, VOID_SLOT)
            .and_then(|table| {
                table.extend(
                    &SystemOperation::Init {
                        order: self.genesis_order.clone(),
                    },
                    INIT_SLOT,
                )
            })
            .map_err(LifecycleRefusal::Configuration)?;
        let era = table.current().era;
        let genesis = [
            LogEntry {
                slot: VOID_SLOT,
                era: Era::INITIAL,
                payload: Payload::System(SystemOperation::Void),
            },
            LogEntry {
                slot: INIT_SLOT,
                era,
                payload: Payload::System(SystemOperation::Init {
                    order: self.genesis_order.clone(),
                }),
            },
        ];
        journal
            .install_suffix(VOID_SLOT, &genesis)
            .map_err(LifecycleRefusal::Journal)?;
        let view = ViewId {
            era,
            view: View::INITIAL,
        };
        let config = Arc::new(table);
        let entry = Progress::reconstitute(
            view,
            view,
            Status::Restarting,
            INIT_SLOT,
            INIT_SLOT,
            INIT_SLOT,
            Slot::NONE,
            0,
            Arc::clone(&config),
            None,
        )
        .map_err(LifecycleRefusal::Progress)?;
        // The fresh joiner is a first life: no durable identity yet, so the
        // gate latches its anchor and the replica enters through the
        // checked fresh-joiner constructor.
        let index = self.grow_to(id);
        let dir = self.gate_root.join(format!("n{index}"));
        let gate = TmpGate::open(dir.clone()).expect("the gate directory opens");
        let mut session = match boot(gate) {
            Ok(BootOutcome::First(first)) => first.latch(id).expect("the first latch writes"),
            _ => panic!("a fresh gate directory classifies first"),
        };
        match Replica::join(
            id,
            WeightedMajority,
            journal,
            PersistedProgress::from(&entry),
            Arc::clone(&config),
            self.stability,
            self.knobs,
        ) {
            Ok(replica) => {
                self.gates[index] = Some(dir);
                self.gate_logs[index] = Some(session.store_mut().ops_handle());
                self.sessions[index] = Some(session);
                self.install(index, replica);
                self.record(format!("n={id:?} boot_as (fresh joiner)"));
                Ok(())
            }
            Err(error) => {
                self.record(format!("n={id:?} boot_as refused: {error:?}"));
                Err(error)
            }
        }
    }

    /// The slot an identity occupies, assigning a fresh one past the end on
    /// first sight. Reincarnated identities live past the genesis order;
    /// slots key on the identity itself, genesis members and successors
    /// alike.
    fn grow_to(&mut self, id: NodeId) -> usize {
        if let Some(index) = self.slot_of(id) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push(None);
        self.applied.push(Vec::new());
        self.disks.push(None);
        self.gates.push(None);
        self.sessions.push(None);
        self.gate_logs.push(None);
        self.pending.push(None);
        self.declared_faults.push(false);
        self.faulted_known.push(false);
        self.slot_ids.push(id);
        self.slots.insert(id, index);
        index
    }

    /// The boot gate's operation log for a node, `read`, `commit:<marker>@<identity>`,
    /// `drain`, in order, from whichever holder rides the gate (the
    /// running session, the deferred crashed session, or nothing). The
    /// fixed write schedules of `docs/uvrr-boot-gate.md` §3 assert
    /// against this.
    pub fn gate_ops(&self, id: NodeId) -> Vec<String> {
        let index = self.index_of(id);
        self.gate_logs[index]
            .as_ref()
            .map(|log| log.borrow().clone())
            .unwrap_or_default()
    }

    /// Fires every deferred latch whose seated observation has arrived: a
    /// reincarnated node that has reached `Normal` at voting weight mints
    /// its witness, and the gate writes the deferred `(new, Joining)` 4x
    /// (`docs/uvrr-boot-gate.md` §3). Called at the end of every step, so
    /// no script can forget it.
    fn settle_pending(&mut self) {
        for index in 0..self.nodes.len() {
            if self.pending[index].is_none() {
                continue;
            }
            let Some(witness) = self.nodes[index]
                .as_ref()
                .and_then(|node| node.replica.rejoined())
            else {
                continue;
            };
            let crashed = self.pending[index].take().expect("checked above");
            match crashed.latch(witness) {
                Ok(session) => {
                    self.sessions[index] = Some(session);
                    self.record(format!(
                        "n={} latch (deferred, seated)",
                        self.slot_ids[index].0
                    ));
                }
                Err((_, error)) => panic!("the deferred latch refused: {error:?}"),
            }
        }
    }

    /// Feeds the node's `Input::Reincarnate { old }` (§4 of the doc): the
    /// bumped node announces its pair to the current configuration's
    /// members, the leader acts on it, the backups drop it by name.
    pub fn reincarnate(&mut self, id: NodeId, old: NodeId) -> StepOutcome {
        self.drive(
            id,
            format!("n={} reincarnate (old n={})", id.0, old.0),
            Input::Reincarnate { old },
        )
    }

    fn install(&mut self, index: usize, replica: HarnessReplica) {
        // A fault present at construction, a persisted fault at reopen,
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
            admin_verdicts: Vec::new(),
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
            if frontier != Slot::NONE {
                // A node whose host retention policy let the committed
                // prefix go (S1) serves its retained window; the checker
                // aligns by slot.
                let view = node.replica.journal().view();
                let (base, _) = view.retained();
                let outcome = view.copy_out(base, frontier, &mut committed);
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
                id: self.slot_ids[index],
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
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| node.observer.read())
    }

    /// Whether the node is up.
    #[must_use]
    pub fn is_up(&self, id: NodeId) -> bool {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .is_some_and(Option::is_some)
    }

    /// The harness's apply record for the node.
    #[must_use]
    pub fn applied(&self, id: NodeId) -> &[(Slot, Box<[u8]>)] {
        &self.applied[self.index_of(id)]
    }

    /// The node's latest published drop diagnostic: every refused
    /// peer guard has a named outcome, and this is where it is observed.
    /// `None` if the node is down.
    #[must_use]
    pub fn diagnostic(&self, id: NodeId) -> Option<Diagnostic> {
        self.slot_of(id)
            .and_then(|index| self.nodes.get(index))
            .and_then(Option::as_ref)
            .map(|node| node.observer.read_diagnostic())
    }

    /// The ordered Apply boundary record, what the §11.1 boundary
    /// assertions are stated over.
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

    /// The slot an identity occupies, when it has one: the per-node
    /// vectors key on the identity itself, never on its integer value.
    fn slot_of(&self, id: NodeId) -> Option<usize> {
        self.slots.get(&id).copied()
    }

    fn index_of(&self, id: NodeId) -> usize {
        self.slot_of(id)
            .unwrap_or_else(|| panic!("the script named n={}, outside the cluster", id.0))
    }

    /// One step: plan, publish, route the released effects, record the
    /// trace line, run the legality gate. Every input a node receives comes
    /// through here, so the gate stands after every step by construction.
    fn drive(&mut self, id: NodeId, summary: String, input: Input) -> StepOutcome {
        // An identity past the cluster is an unprovisioned node: the host
        // cannot deliver to it, the same verdict as a crashed one. (A
        // reconfiguration may add a member whose process was never
        // started, its stream is recorded undeliverable.)
        let Some(index) = self.slot_of(id) else {
            self.record(format!("{summary} -> NodeDown"));
            return StepOutcome::NodeDown;
        };
        let timed = TimedInput {
            at: self.tick,
            event: input,
        };
        let (outcome, released) = match self.nodes[index].as_mut() {
            None => (StepOutcome::NodeDown, Vec::new()),
            Some(node) => {
                let accepted_before = node.replica.progress().accepted();
                let planned = node.replica.plan(&timed, &node.replica.journal().view());
                match planned {
                    Err(rejection) => (StepOutcome::PlanRefused(rejection), Vec::new()),
                    Ok(plan) => match node.replica.publish(plan) {
                        Err(rejection) => (StepOutcome::PublishRefused(rejection), Vec::new()),
                        Ok(PublishOutcome::Published { revision, effects }) => {
                            // Opportunistic reclamation (§4, S1): an append
                            // is the lazy moment, and the published
                            // checkpoint is the sole authorization, with
                            // none, this is a no-op however old the history.
                            if node.replica.progress().accepted() > accepted_before {
                                node.replica.reclaim_journal();
                            }
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
        self.settle_pending();
        outcome
    }

    /// Routes one released effect to where the host would take it: `Send`
    /// to the network, `Apply` to the node's pending list, `Persist` to the
    /// node's outstanding intent.
    fn route_effect(&mut self, from: usize, effect: Effect) {
        match effect {
            Effect::Send { to, era, message } => {
                self.network.route(Envelope {
                    from: self.slot_ids[from],
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
            Effect::Persist(intent) => {
                if let Some(node) = self.nodes[from].as_mut() {
                    node.outstanding_intent = Some(intent.revision);
                }
            }
            Effect::AdminResponse { verdict } => {
                if let Some(node) = self.nodes[from].as_mut() {
                    node.admin_verdicts.push(verdict);
                }
            }
        }
    }

    /// Appends one trace line and runs the legality gate. Every harness
    /// action, delivery, injection, tick, lifecycle, network change, ends
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
    /// have been declared. An undeclared fault fails with the full trace,
    /// "illegal transition forces the node crashed" made loud.
    fn scan_faults(&mut self) {
        let mut newly_faulted: Vec<(usize, Fault)> = Vec::new();
        for (index, slot) in self.nodes.iter().enumerate() {
            if let Some(node) = slot {
                if !self.faulted_known[index] {
                    if let Some(fault) = node.replica.progress().fault() {
                        newly_faulted.push((index, fault));
                    }
                }
            }
        }
        for (index, fault) in newly_faulted {
            let id = self.slot_ids[index].0;
            if self.declared_faults[index] {
                self.declared_faults[index] = false;
                self.faulted_known[index] = true;
                self.push_trace(format!("  n={id} faulted as declared: {fault:?}"));
            } else {
                let dump = self.trace_dump();
                panic!("undeclared fault on n={id}: {fault:?}\nstep trace:\n{dump}");
            }
        }
    }
}

/// A whole-history copy of a journal, for the harness's disk. The harness
/// never reclaims below the published checkpoint, so the copy is always
/// the retained window; a disk recorded with a retained base (S1, see
/// `crash_with_retained_base`) copies that window.
fn clone_journal(journal: &SegmentedLog, tail_capacity: Option<usize>) -> SegmentedLog {
    let view = journal.view();
    let mut clone = make_journal(tail_capacity);
    if let Some(frontier) = view.accepted() {
        let (base, _) = view.retained();
        let mut entries = Vec::new();
        match view.copy_out(base, frontier, &mut entries) {
            RangeOutcome::Complete => {}
            RangeOutcome::Short { .. }
            | RangeOutcome::BelowRetention { .. }
            | RangeOutcome::Empty => {
                panic!("the retained window is whole by construction: {frontier:?}")
            }
        }
        clone
            .install_suffix(base, &entries)
            .expect("a whole-history copy installs on an empty log");
    }
    clone
}

/// The cluster's journal construction: the default log, or one with the
/// script-pinned tail capacity (see [`Harness::with_journal_capacity`]).
fn make_journal(tail_capacity: Option<usize>) -> SegmentedLog {
    match tail_capacity {
        Some(capacity) => SegmentedLog::with_tail_capacity(capacity),
        None => SegmentedLog::new(),
    }
}
