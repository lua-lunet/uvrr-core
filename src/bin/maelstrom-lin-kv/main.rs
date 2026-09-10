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
//! # Durability: two modes, one host
//!
//! The bench host has two durability modes, fixed by the environment at
//! init:
//!
//! - **Volatile — the default.** `MAELSTROM_VRR_STATE_DIR` unset or
//!   empty: no store at all — no file is opened, written or fsynced, and
//!   every construction takes the [`Stability::Volatile`] provision path:
//!   first boot or kill-nemesis restart alike provisions fenced
//!   `Recovering` (the genesis ruling, §1.3), and a node becomes `Normal`
//!   only through the bootstrap adoption (§4). A voter whose volatile
//!   state vanished while retaining authority is unrepresentable in this
//!   host: a restarted node rejoins fenced, and the next view change
//!   deposes any stale primary. A kill loses the process's state; the
//!   survivors inside the fault bound keep serving.
//! - **Persisted — opt-in.** `MAELSTROM_VRR_STATE_DIR` set: the state dir
//!   is the persistence home, and the write-through barrier and the §2
//!   restart decision below govern it. The state file carries exactly
//!   what the core considers durable: the §5 persisted progress record,
//!   the journal's retained history, and the §2 restart model. A kill
//!   loses at most the in-flight input; every effect the client or a peer
//!   ever observed rests on a file the host fsynced first.
//!
//! # The persisted mode's write-through (§7)
//!
//! In both modes the node runs [`Stability::Volatile`] — the core's
//! statement that effects release at `publish`. In the persisted mode the
//! host enforces the durability ordering itself, between publish and
//! observability: after every published transition the host rewrites the
//! node's state file (temp file, fsync, atomic rename, fsync of the
//! directory — the §7 `Forced` barrier shape) and only then routes any
//! released effect.
//!
//! The stability handshake stays `Volatile` because it is the only level
//! whose contract this host can discharge honestly: every other level parks
//! each transition behind an [`Effect::Persist`] intent, and the parked
//! transition — the would-be durable state the host would have to write —
//! is not observable through the core's public surface (the plan's candidate
//! and journal mutation are consumed by `publish`; `PublishOutcome::Parked`
//! releases only the intent's revision and ranges). A host outside the
//! crate cannot build the state it would be confirming, so the core's
//! parked protocol cannot be selected without confirming a barrier the host
//! cannot actually build. The honest equivalent is the host-side barrier
//! above: same ordering, enforced where the effects become observable.
//! (Reported core limitation, not papered over; see the repository report.)
//!
//! # The restart decision (§2 of `docs/uvrr-reincarnation.md`)
//!
//! In the persisted mode the state dir is the persistence home. On `init`:
//!
//! - **No state file** — the first life: `Node::provision` exactly as the
//!   genesis ruling prescribes, then the state file is written with the
//!   running sentinel (`unflushed`) before `init_ok` is answered.
//! - **A state file, all four superblock copies `flushed`** — a clean
//!   shutdown's checkpoint (§2's clean path): `Node::reopen` under the same
//!   identity. The core fences the node into `Recovering` (§5's boot rule)
//!   either way; the durable evidence is the honest statement, not an
//!   authority.
//! - **A state file, any copy `unflushed`** — the node was operating when
//!   it died (every Jepsen kill): the §2 dirty path. The host bumps the
//!   identity (§2: the bumped node's identity is one incarnation past the
//!   highest recorded, checked), reopens under the bumped identity, writes
//!   the bump before announcing it, and reports `Input::Reincarnate { old
//!   }` — from which point the core's reincarnation machinery owns the
//!   restart: the announcement (§4), the leader's forced weight sequence
//!   (§5, one era per batch), and the re-announce this host re-drives on
//!   every tick until the identity is a voting member again (§8).
//!
//! The bump is durable before it is announced: the state file is rewritten
//! with the new incarnation before the announcement is driven, so a crash
//! between the bump and the announcement replays the same bump — the
//! decision carries the pair (§2's continuation commitment).
//!
//! Torn or corrupt state files are crash artifacts, not inputs: the host
//! refuses the reopen by name (the core's [`LifecycleRefusal`] path),
//! answers `error`, and exits nonzero so Jepsen restarts the node. Never a
//! panic.
//!
//! # Identity mapping (host-side, transparent to Maelstrom)
//!
//! The core's `NodeId` space is the identity space the bump moves in; the
//! Maelstrom transport names a fixed roster. The host maps between them
//! ([`Identity`]): a genesis identity is its sorted-roster position, a
//! bumped identity is `incarnation * STRIDE + position`, and the low bits
//! always resolve a core identity back to the Maelstrom node string it
//! reincarnated under. A `Reincarnation(old, new)` announcement remaps the
//! sender's transport id onto the announced new identity (§6's attribution
//! rule, enforced host-side); before an announcement arrives, traffic from
//! a restarted node is attributed to its genesis identity — the boot fence
//! and the durable identity the node carries make that the same stance the
//! provision-fresh host already took, and the announcement closes it.
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
mod store;

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use serde_json::Value;
use vrr::configuration::{SystemOperation, VOID_SLOT};
use vrr::effects::{Effect, Stability};
use vrr::ids::{Era, NodeId, Operation, OperationId, Slot, Tick};
use vrr::journal::{Journal, LogEntry, Payload, SegmentedLog};
use vrr::message::{Body, Message};
use vrr::progress::Status;
use vrr::quorum::WeightedMajority;
use vrr::replica::{
    Input, PlanRefusal, PublishOutcome, Replica, SuperblockCopies, TimedInput, ViewChangeKnobs,
    construct_pivot,
};
use vrr::wire::{Pack, Unpack};

use crate::kv::Kv;
use crate::proto::{Incoming, KvRequest, KvResponse, Outgoing, error, from_hex, to_hex};
use crate::store::{NodeState, Store};

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
            // A clean shutdown (§2, persisted mode): the state file
            // becomes a flushed checkpoint — the next init under the same
            // state dir reopens cleanly, under the same identity. A kill
            // leaves the running sentinel in the file: the next init
            // reads dirty and bumps. The volatile mode has nothing to
            // flush; EOF is just exit.
            Event::Eof => {
                node.clean_shutdown();
                return;
            }
        }
    }
}

/// The stride between one node's consecutive identities. A genesis identity
/// is its sorted-roster position; a bumped identity is
/// `incarnation * STRIDE + position`, so identities never collide across
/// nodes and never wrap — the bump is checked, and an identity past the
/// `u32` space is a named refusal (§2's bump refuses rather than wraps).
const STRIDE: u32 = 1 << 24;

/// The host-side identity map: the core's `NodeId` space is the space the
/// identity bump moves in; the Maelstrom transport names a fixed roster.
/// Transparent to Maelstrom: a bumped internal identity resolves to the
/// same Maelstrom node string it reincarnated under, and traffic from a
/// node string resolves to the identity the host currently attributes to
/// it (the genesis position until a `Reincarnation` announcement remaps
/// it, §4, §6).
#[derive(Default)]
struct Identity {
    /// The sorted genesis roster, fixed by `node_ids` (§8.7.2's genesis
    /// order); the low bits of every core identity name a position in it.
    roster: Vec<String>,
    /// The transport attribution the host currently holds per Maelstrom
    /// id, learned at each observed announcement.
    current: HashMap<String, NodeId>,
}

impl Identity {
    /// The genesis identity of the node at roster position `index`.
    fn genesis(index: usize) -> NodeId {
        NodeId(u32::try_from(index).expect("a roster position fits the identity space"))
    }

    /// The identity the node at roster position `position` carries in
    /// incarnation `incarnation` (genesis is incarnation 0). Checked:
    /// the identity space is refused, never wrapped.
    fn of(incarnation: u64, position: u32) -> Option<NodeId> {
        let incarnations = u64::from(u32::MAX / STRIDE);
        if incarnation > incarnations {
            return None;
        }
        let base = u32::try_from(incarnation).ok()?.checked_mul(STRIDE)?;
        Some(NodeId(base.checked_add(position)?))
    }

    /// The Maelstrom node string a core identity resolves to: the roster
    /// position carried in the low bits. Total over every identity the
    /// core can name — genesis and bumped alike — and `None` for an
    /// identity outside the roster's reach, which the transport drops.
    fn string_of(&self, id: NodeId) -> Option<&str> {
        self.roster
            .get((id.0 % STRIDE) as usize)
            .map(String::as_str)
    }

    /// The identity the host's transport attributes an incoming datagram
    /// to: what it last learned for the sender, else the sender's genesis
    /// position.
    fn attributed(&self, src: &str) -> Option<NodeId> {
        self.current.get(src).copied().or_else(|| {
            self.roster
                .iter()
                .position(|member| member == src)
                .map(Identity::genesis)
        })
    }

    /// The transport remap a `Reincarnation(old, new)` announcement
    /// carries (§6): the restarted node announced the pair from its
    /// transport id, so the transport id now names `new`. Refused when
    /// the announcement's `old` does not resolve to the sender's own
    /// transport id — the host does not move another node's traffic on
    /// the announcer's word.
    fn remap(&mut self, src: &str, old: NodeId, new: NodeId) -> bool {
        let speaks_for = self.string_of(old).is_some_and(|string| string == src);
        if !speaks_for {
            return false;
        }
        self.current.insert(src.to_owned(), new);
        true
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
    identity: Identity,
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
    /// The persistence home, when the node opts in:
    /// `MAELSTROM_VRR_STATE_DIR` set at init. `None` is the volatile
    /// default — no store, no file I/O; the provision path is the
    /// historical one, and no write is ever issued.
    store: Option<Store>,
    /// The §2 four-superblock copies this node carries, written on every
    /// persist. `None` before the first init completes — and always in
    /// the volatile mode, which carries no restart model and writes
    /// nothing.
    copies: Option<SuperblockCopies>,
    /// The old identity of a dirty restart whose bump is announced (§4)
    /// and re-announced on every tick (§8) until the identity is a voting
    /// member again. `None` on a fresh provision or a clean reopen.
    announce: Option<NodeId>,
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

    /// The init handshake: parse the roster, take the durability mode the
    /// environment selects (volatile default, persisted opt-in), and — in
    /// the persisted mode — decide the restart (§2): provision a first
    /// life, or reopen the durable evidence — cleanly (flushed) under the
    /// same identity, or dirty (unflushed) under the bumped identity,
    /// whose restart the core's reincarnation machinery then owns. A
    /// state file the host cannot vouch for is the reopen refusal path:
    /// named, answered `error`, exit nonzero.
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
        self.id = node_id.clone();
        self.identity = Identity {
            roster: members.clone(),
            current: HashMap::new(),
        };
        let Some(index) = members.iter().position(|member| member == &node_id) else {
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
        };
        // The mode selection: a set, non-empty `MAELSTROM_VRR_STATE_DIR`
        // opts the node into the persisted mode; unset or empty is the
        // volatile default — no store, no file I/O, the provision path
        // below.
        self.store = match std::env::var("MAELSTROM_VRR_STATE_DIR") {
            Ok(dir) if !dir.is_empty() => match Store::open(std::path::Path::new(&dir), &node_id) {
                Ok(store) => Some(store),
                Err(reason) => self.refuse_node(
                    message,
                    format!("the state dir {dir} is unusable: {reason}"),
                ),
            },
            // Unset or empty: the volatile default. No store is opened and
            // no file is ever touched; the absent-evidence arm below takes
            // the historical provision path.
            _ => None,
        };
        let loaded = match &self.store {
            Some(store) => store.load(),
            None => Err(store::StoreError::Absent),
        };
        let genesis_order = (0..members.len())
            .map(Identity::genesis)
            .collect::<Vec<_>>();

        let knobs = ViewChangeKnobs {
            primary_timeout: PRIMARY_TIMEOUT_TICKS,
            // The §13.1 suffix budget: unbounded. The histories this harness
            // replicates are small, and the budget is a host policy knob (W5),
            // never a correctness input.
            view_change_budget: usize::MAX,
        };

        match loaded {
            Err(store::StoreError::Absent) => {
                // A first life — and the volatile default's every life:
                // the genesis ruling, exactly as the unpersisted host ran
                // it. In the persisted mode the state file is written with
                // the running sentinel before `init_ok` is answered.
                let own = Identity::genesis(index);
                match Node::provision(
                    own,
                    genesis_order,
                    WeightedMajority,
                    SegmentedLog::new(),
                    Stability::Volatile,
                    knobs,
                ) {
                    Ok(replica) => {
                        eprintln!(
                            "vrr-init: node {node_id} provisions a fresh identity (no persisted state)"
                        );
                        self.replica = Some(replica);
                        if self.store.is_some() {
                            self.copies = Some(fresh_copies().start_operating());
                        }
                    }
                    Err(reason) => {
                        self.refuse_node(message, format!("provision refused: {reason:?}"))
                    }
                }
            }
            Ok(state) => {
                // The durable evidence names a cluster: it must name THIS
                // one, or the reopen would reconstruct a configuration the
                // roster cannot answer.
                if state.roster != members {
                    self.refuse_node(
                        message,
                        format!(
                            "the persisted state names roster {:?}, not the init's {:?}",
                            state.roster, members
                        ),
                    );
                }
                self.reopen_node(message, state, index, knobs);
            }
            Err(store::StoreError::Corrupt(reason)) => {
                // A torn or corrupt file is a crash artifact: the core's
                // reopen refusal path — a named refusal, answered `error`,
                // nonzero exit so Jepsen restarts the node.
                self.refuse_node(
                    message,
                    format!("the persisted state is unreadable: {reason}"),
                );
            }
        }

        // Start of operating (§2, persisted mode): the running sentinel,
        // durable before the node answers anything. The volatile mode has
        // no file to write.
        if let Err(reason) = self.write_state() {
            self.refuse_node(message, format!("the state file is unwritable: {reason}"));
        }
        if let Some(msg_id) = message.msg_id() {
            self.reply(&message.src, msg_id, serde_json::json!({"type": "init_ok"}));
        }
    }

    /// The reopen decision on durable evidence (§2): rebuild the journal,
    /// the era table and the application from the file, classify the
    /// superblocks, and reopen — cleanly under the persisted identity, or
    /// dirty under the bumped identity, announcing. Every refusal is the
    /// reopen refusal path: named, answered `error`, exit nonzero.
    fn reopen_node(
        &mut self,
        message: &Incoming,
        state: NodeState,
        index: usize,
        knobs: ViewChangeKnobs,
    ) {
        let persisted_incarnation = state.copies.read_identity().0;
        let (decision, rewritten) = state.copies.restart().unwrap_or_else(|incarnation| {
            self.refuse_node(
                message,
                format!("the identity space is spent at incarnation {incarnation:?}"),
            )
        });
        let (own_incarnation, bumped) = match &decision {
            vrr::replica::RestartDecision::Continue { identity } => (identity.0, false),
            vrr::replica::RestartDecision::Bump { .. } => (rewritten.read_identity().0, true),
        };
        let crashed = match Identity::of(persisted_incarnation, u32::try_from(index).expect("a roster position fits")) {
            Some(crashed) => crashed,
            None => self.refuse_node(
                message,
                format!("the persisted incarnation {persisted_incarnation} is outside the identity space"),
            ),
        };
        let own = match Identity::of(own_incarnation, u32::try_from(index).expect("a roster position fits")) {
            Some(own) => own,
            None => self.refuse_node(
                message,
                format!("the identity space is spent: incarnation {own_incarnation} exceeds the u32 identity space"),
            ),
        };
        let mut journal = SegmentedLog::new();
        if let Some(first) = state.entries.first() {
            if first.slot != VOID_SLOT {
                self.refuse_node(
                    message,
                    format!(
                        "the persisted history begins at slot {}, not the genesis anchor {}",
                        first.slot.0, VOID_SLOT.0
                    ),
                );
            }
            if let Err(reason) = journal.install_suffix(first.slot, &state.entries) {
                self.refuse_node(
                    message,
                    format!("the persisted history is not contiguous: {reason:?}"),
                );
            }
        }
        let table = match fold_table(&state.entries, state.progress.committed) {
            Ok(table) => Arc::new(table),
            Err(reason) => self.refuse_node(message, reason),
        };
        // The application state the applied frontier names is rebuilt from
        // the persisted history (replay-safe: `kv.rs`); the core re-emits
        // the committed-but-unapplied upcalls itself.
        for entry in &state.entries {
            if entry.slot > state.progress.applied {
                break;
            }
            if let Payload::Operation { id, payload } = &entry.payload {
                let _ = self.execute(*id, payload);
            }
        }
        match Node::reopen(
            own,
            WeightedMajority,
            journal,
            state.progress,
            table,
            Stability::Volatile,
            knobs,
        ) {
            Ok(replica) => {
                self.replica = Some(replica);
                self.copies = Some(rewritten.start_operating());
                if bumped {
                    eprintln!(
                        "vrr-init: node {} reopens dirty: identity bumps {} -> {} (the core's reincarnation machinery owns the restart)",
                        self.id, crashed.0, own.0
                    );
                    // The §2 continuation commitment, durable before it is
                    // announced: the bump is written by the write-through
                    // below, before `init_ok` is answered.
                    self.announce = Some(crashed);
                    self.drive(Input::Reincarnate { old: crashed });
                } else {
                    eprintln!(
                        "vrr-init: node {} reopens cleanly (flushed state, identity {})",
                        self.id, own.0
                    );
                }
            }
            Err(reason) => self.refuse_node(message, format!("reopen refused: {reason:?}")),
        }
    }

    /// A restart this node cannot honestly enter: the named refusal path —
    /// the diagnostic on stderr, `error` to the init request, nonzero exit
    /// so Jepsen restarts the node. Never a panic.
    fn refuse_node(&mut self, message: &Incoming, why: String) -> ! {
        eprintln!("vrr-init: node {} refuses to start: {why}", self.id);
        if let Some(msg_id) = message.msg_id() {
            self.reply(
                &message.src,
                msg_id,
                serde_json::json!({
                    "type": "error",
                    "code": error::TEMPORARILY_UNAVAILABLE,
                    "text": format!("node refuses to start: {why}"),
                }),
            );
        }
        std::process::exit(1);
    }

    /// The write-through barrier: the file carries the state the core just
    /// published BEFORE any released effect is routed — the invariant that
    /// makes a kill lose at most the in-flight input. A failure is
    /// determinate: the node stops (exit nonzero, Jepsen restarts it from
    /// the last good file) rather than serving on a store it cannot vouch
    /// for.
    fn persist(&mut self) {
        if self.store.is_none() || self.replica.is_none() {
            return;
        }
        if let Err(reason) = self.write_state() {
            eprintln!("vrr-init: the state file failed write-through: {reason}");
            std::process::exit(1);
        }
    }

    /// Builds the snapshot and writes it.
    fn write_state(&self) -> std::io::Result<()> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let Some(replica) = &self.replica else {
            return Ok(());
        };
        let Some(copies) = self.copies else {
            return Ok(());
        };
        let state = NodeState::snapshot(
            copies,
            self.identity.roster.clone(),
            vrr::replica::PersistedProgress::from(replica.progress()),
            &replica.journal().view(),
        )
        .map_err(std::io::Error::other)?;
        store.write(&state)
    }

    /// A clean shutdown (§2): the flushed marker. The write's failure is
    /// logged, not fatal — the running sentinel it fails to replace is
    /// exactly the honest reading the next init will take (dirty, bump).
    fn clean_shutdown(&mut self) {
        if self.replica.is_none() {
            return;
        }
        if let Some(copies) = &self.copies {
            self.copies = Some(copies.clean_shutdown());
        }
        if let Err(reason) = self.write_state() {
            eprintln!("vrr-init: the clean shutdown did not flush: {reason}");
        }
    }

    fn on_peer(&mut self, message: &Incoming) {
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
            Err(reason) => {
                eprintln!("peer message failed the VRR codec: {reason:?}");
                return;
            }
        };
        // Transport attribution. A `Reincarnation(old, new)` announcement
        // (§4) is attributed to the identity it names: the announcement's
        // sender is the transport-attributed restarted node, and the pair
        // is the transport remap (§6) — from here on, the sender's traffic
        // is attributed to the new identity, and the superseded one's
        // replies can never regain eligibility.
        let from = match &decoded.body {
            Body::Reincarnation { old, new } if self.identity.remap(&message.src, *old, *new) => {
                *new
            }
            _ => match self.identity.attributed(&message.src) {
                Some(from) => from,
                None => {
                    eprintln!("peer message from unknown node {}", message.src);
                    return;
                }
            },
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
                .and_then(|id| self.identity.string_of(id).map(str::to_owned));
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
            Err(reason) => {
                eprintln!("cannot encode kv request: {reason}");
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
        let Some(node) = self.identity.attributed(target) else {
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
                .and_then(|id| self.identity.string_of(id).map(str::to_owned));
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
                    "node": self.identity.string_of(member.node),
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

        // The tick drives the bootstrap promotion path's decisions and the
        // view-change suspicion timeout (S4).
        self.drive(Input::Tick);
        // The §8 re-announce: a bumped identity that is not yet a voting
        // member re-announces on every tick until the forced sequence
        // promotes it (or a stable leader exists to finish it). The
        // announcement is idempotent at the leader (§4).
        if let Some(old) = self.announce {
            let voting = self.replica.as_ref().is_some_and(|replica| {
                replica
                    .progress()
                    .config()
                    .current()
                    .config
                    .weight_of(replica.own())
                    .is_some_and(|weight| weight.0 >= 1)
            });
            if !voting {
                self.drive(Input::Reincarnate { old });
            } else {
                self.announce = None;
            }
        }
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

    /// One plan/publish interval (§7, §12), closed by the host's own
    /// write-through barrier: the published state reaches the state file
    /// (fsynced) BEFORE the released effects are returned for routing, so
    /// no effect — a peer datagram, an applied answer to a client — is
    /// observable before what supports it is durable. `Volatile` always
    /// publishes; the parked outcome cannot arise at `Stability::Volatile`
    /// and is refused loudly rather than absorbed.
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
            Ok(PublishOutcome::Published { effects, .. }) => {
                self.persist();
                Ok(effects)
            }
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
                    // recipient. A core identity the roster's reach cannot
                    // resolve is a host defect: dropped, loudly.
                    let peer = self.identity.string_of(to).map(str::to_owned);
                    match peer {
                        Some(peer) => self.send_peer(&peer, &message),
                        None => eprintln!("effect names identity {} outside the roster", to.0),
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
            Err(reason) => KvResponse::Failed {
                client_id,
                request_num,
                code: error::TEMPORARILY_UNAVAILABLE,
                text: format!("undecodable payload: {reason}"),
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
            Err(reason) => eprintln!("cannot encode VRR message: {reason:?}"),
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
            Err(reason) => eprintln!("cannot encode outgoing envelope: {reason}"),
        }
    }
}

/// A fresh four-superblock set (§2): the genesis incarnation, flushed — a
/// provisioned node's state file is born a self-consistent checkpoint, and
/// the running sentinel below marks it dirty for the next restart.
fn fresh_copies() -> SuperblockCopies {
    SuperblockCopies {
        copies: std::array::from_fn(|_| vrr::replica::CopyState {
            identity: vrr::replica::Incarnation(0),
            marker: vrr::replica::Marker::Flushed,
        }),
    }
}

/// The era table a reopen needs, rebuilt from the persisted journal: the
/// core folds a committed system operation exactly when the commit frontier
/// covers it (§8.7.1), so replaying every system entry at or below the
/// persisted committed frontier, in slot order, reproduces the table — the
/// construction `Node::reopen`'s own documentation prescribes ("the host
/// reconstructs it from the journal it also persists"). A fold refusal is
/// persisted history the core would refuse: the reopen refusal path, named.
fn fold_table(
    entries: &[LogEntry],
    committed: Slot,
) -> Result<vrr::configuration::EraTable, String> {
    let mut table = vrr::configuration::EraTable::genesis();
    for entry in entries {
        if entry.slot > committed {
            break;
        }
        if let vrr::journal::Payload::System(operation) = &entry.payload {
            table = table.extend(operation, entry.slot).map_err(|reason| {
                format!(
                    "the persisted history does not fold at slot {}: {reason:?}",
                    entry.slot.0
                )
            })?;
        }
    }
    Ok(table)
}

/// `c7` -> 7. Maelstrom client node IDs are `c` followed by an integer.
fn client_number(src: &str) -> Option<u64> {
    src.strip_prefix('c')?.parse().ok()
}
