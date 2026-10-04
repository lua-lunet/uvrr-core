//! `cluster_proof.rs` — a weighted-voting cluster that can only evolve legally.
//!
//! Model
//! -----
//! * The cluster is a sparse positional array; slot `i` holds `Some(w)` with
//!   `w ∈ {0, 1, 2}`, or `None` for a node that has left. `None` slots are
//!   immune to every operation.
//! * `increment(i)` — `w + 1`, may not exceed 2.
//! * `decrement(i)` — `w - 1`, may not go below 0.
//! * `join()` — a new node of weight 0 is appended (index = len).
//! * `leave(i)` — only a node of weight 0 may leave; its slot becomes `None`.
//! * `double()` — every weight doubles; rejected if any weight is 2, because
//!   it would have to exceed 2 (and the mass would no longer double exactly).
//! * `halve()` — every weight halves; rejected if any weight is odd, because
//!   it would become fractional.
//!
//! Mass rule, enforced between configurations (batch boundaries): a
//! transition is legal only if the total mass stays the same, changes by
//! exactly ±1, doubles exactly, or halves exactly.
//!
//! Proofs in this file:
//! 1. `every_operation_respects_bounds_and_the_mass_rule` — exhaustive over
//!    every state of length 1..=4 and every operation: success keeps the
//!    state valid and the mass transition legal, and matches an independent
//!    oracle; failure changes nothing and matches the oracle's rejection.
//! 2. `batch_mass_rule_is_enforced_between_configurations` — batches are
//!    atomic; a net change outside {same, ±1, x2, /2} is rejected.
//! 3. `bfs_every_state_of_length_3_and_4_is_reachable` — breadth-first
//!    search proves every valid 3- and 4-slot configuration is reachable
//!    from `[1,1,1]`, and that the node-replacement example needs exactly
//!    4 operations.
//! 4. Adjacent-state and series validation (built Red -> Green):
//!    `transition_between` classifies the mass change between two adjacent
//!    states (same / +1 / -1 / x2 / /2 / illegal) and `validate_series`
//!    proves a whole series of operations is a correct step between two
//!    states — refusing `halve()` while any node sits at weight 1, since
//!    a fractional weight is illegal.
//! 5. Viewstamped Replication (Revisited) leader rule (built Red -> Green):
//!    `Cluster::leader_array` reduce-lefts the sparse configuration into
//!    the dense array of voting-node indexes (weight 1 or 2; null and 0
//!    slots do not vote), and `validate_series` asserts that this array
//!    never has fewer than two entries — start, intermediates, and end.
//! 6. Eras, views, and leaders (built Red -> Green): a configuration state
//!    is an era. The immutable `Configuration` carries the sparse weights
//!    array, the dense voting-index array, N, the view, L = view % N, the
//!    leader's sparse index, and a parallel extrinsic details array
//!    (IPv6, port, DNS, rack, DC, cloud region) keyed by the sparse index
//!    — a unique node identity that is never recycled. `reconfigure`
//!    produces the next era through the full legality check.
//!
//! Build & run (no cargo needed):
//!   rustc --edition 2021 cluster_proof.rs -o demo && ./demo
//!   rustc --edition 2021 --test cluster_proof.rs -o tests && ./tests

/// The highest voting weight a node may carry.
pub const MAX_WEIGHT: u8 = 2;

/// A single cluster operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Add one to the voting weight of `index`. May not exceed `MAX_WEIGHT`.
    Increment(usize),
    /// Subtract one from the voting weight of `index`. May not go below 0.
    Decrement(usize),
    /// A new node joins with weight 0, appended at the next free index.
    Join,
    /// A node of weight 0 logically leaves; its slot becomes null.
    Leave(usize),
    /// Every weight doubles. Illegal if any weight would exceed 2.
    Double,
    /// Every weight halves. Illegal if any weight would become fractional.
    Halve,
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Increment(i) => write!(f, "increment({})", i),
            Op::Decrement(i) => write!(f, "decrement({})", i),
            Op::Join => write!(f, "join()"),
            Op::Leave(i) => write!(f, "leave({})", i),
            Op::Double => write!(f, "double()"),
            Op::Halve => write!(f, "halve()"),
        }
    }
}

/// Every way an operation can be refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClusterError {
    /// The index is past the end of the array.
    NoSuchNode(usize),
    /// The slot is null: that node already left and can never be touched.
    AlreadyLeft(usize),
    /// `increment` on a node already at weight 2.
    IncrementPastTwo(usize),
    /// `decrement` on a node already at weight 0.
    DecrementBelowZero(usize),
    /// `leave` on a node whose weight is not 0.
    LeaveWithWeight(usize),
    /// `double` with at least one node at weight 2 (would exceed 2).
    DoubleWouldExceedTwo(usize),
    /// `halve` with at least one odd weight (would become fractional).
    HalveWouldFraction(usize),
    /// A batch changed the total mass by anything other than 0, +1, -1, x2 or /2.
    IllegalMassTransition { before: u32, after: u32 },
}

/// The cluster: a sparse positional array of voting weights.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cluster {
    nodes: Vec<Option<u8>>,
}

/// Is `before -> after` one of the allowed mass transitions
/// (same, +1, -1, all-double, all-halve)?
pub fn allowed_transition(before: u32, after: u32) -> bool {
    after == before
        || after == before + 1
        || after + 1 == before
        || after == before * 2
        || (before % 2 == 0 && after == before / 2)
}

/// Name of the mass transition, for human-readable traces.
pub fn describe_transition(before: u32, after: u32) -> &'static str {
    if after == before {
        "same"
    } else if after == before + 1 {
        "+1"
    } else if after + 1 == before {
        "-1"
    } else if after == before * 2 {
        "x2"
    } else if before % 2 == 0 && after == before / 2 {
        "/2"
    } else {
        "ILLEGAL"
    }
}

// ---------------------------------------------------------------------------
// Adjacent-state validation (built Red -> Green; see the tests below).
// ---------------------------------------------------------------------------

/// How the total mass changed between two adjacent configurations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// Mass unchanged.
    Same,
    /// Mass increased by exactly one mass unit.
    PlusOne,
    /// Mass decreased by exactly one mass unit.
    MinusOne,
    /// Every weight doubled: mass exactly x2.
    Doubled,
    /// Every weight halved: mass exactly /2.
    Halved,
    /// Anything else: the two states cannot be adjacent configurations.
    Illegal,
}

impl Transition {
    /// Every variant except `Illegal` obeys the mass rule.
    pub fn is_legal(&self) -> bool {
        !matches!(self, Transition::Illegal)
    }
}

/// Why a series of operations between two states was rejected.
#[derive(Clone, Debug, PartialEq)]
pub enum SeriesError {
    /// One of the two states is not a legal configuration (weight > 2).
    InvalidState(Cluster),
    /// The operation at `step` was refused (fractional weight, weight
    /// out of bounds, op on a null slot, ...).
    Refused { step: usize, error: ClusterError },
    /// Every operation succeeded, but the series ended somewhere else.
    WrongEndState { expected: Cluster, actual: Cluster },
    /// The series succeeded and ended correctly, but the mass change
    /// between the two states is not same / +1 / -1 / x2 / /2.
    IllegalMassTransition { before: u32, after: u32 },
    /// VR-Revisited leader rule violated: this state has fewer than two
    /// voting nodes (weight 1 or 2). The leader array must never be
    /// shorter than two, at any point.
    TooFewVoters { state: Cluster, voters: usize },
}

/// Classify the mass change between two adjacent states. Only `Same`,
/// `PlusOne`, `MinusOne`, `Doubled` and `Halved` may sit next to each
/// other in the cluster's history.
pub fn transition_between(a: &Cluster, b: &Cluster) -> Transition {
    let (x, y) = (a.mass(), b.mass());
    if y == x {
        Transition::Same
    } else if y == x + 1 {
        Transition::PlusOne
    } else if y + 1 == x {
        Transition::MinusOne
    } else if y == x * 2 {
        Transition::Doubled
    } else if x % 2 == 0 && y == x / 2 {
        Transition::Halved
    } else {
        Transition::Illegal
    }
}

/// Replay a series of operations and check that it is a correct
/// reconfiguration step between two states: both states must be valid
/// configurations, every operation must succeed (so `halve()` is refused
/// while any node sits at weight 1 — a fractional weight is illegal),
/// the series must end exactly at `to`, the mass change between `from`
/// and `to` must be same / +1 / -1 / x2 / /2, and the Viewstamped
/// Replication (Revisited) leader rule must hold: at every point —
/// start, every intermediate state, and end — the leader array of
/// voting-node indexes (weight 1 or 2) must hold at least two entries.
pub fn validate_series(from: &Cluster, to: &Cluster, ops: &[Op]) -> Result<(), SeriesError> {
    if !from.is_valid() {
        return Err(SeriesError::InvalidState(from.clone()));
    }
    if !to.is_valid() {
        return Err(SeriesError::InvalidState(to.clone()));
    }
    // VR-Revisited leader rule: never fewer than two voting nodes.
    if let Some(e) = too_few_voters(from) {
        return Err(e);
    }
    if let Some(e) = too_few_voters(to) {
        return Err(e);
    }
    let mut c = from.clone();
    for (step, op) in ops.iter().enumerate() {
        c.apply(*op)
            .map_err(|error| SeriesError::Refused { step, error })?;
        if let Some(e) = too_few_voters(&c) {
            return Err(e);
        }
    }
    if &c != to {
        return Err(SeriesError::WrongEndState {
            expected: to.clone(),
            actual: c,
        });
    }
    if transition_between(from, to).is_legal() {
        Ok(())
    } else {
        Err(SeriesError::IllegalMassTransition {
            before: from.mass(),
            after: to.mass(),
        })
    }
}

/// The leader rule for a single state: the leader array of voting-node
/// indexes must never be shorter than two.
fn too_few_voters(c: &Cluster) -> Option<SeriesError> {
    let voters = c.leader_array().len();
    (voters < 2).then(|| SeriesError::TooFewVoters {
        state: c.clone(),
        voters,
    })
}

// ---------------------------------------------------------------------------
// Eras, views, and leaders (Viewstamped Replication, Revisited).
// A configuration state is an era. Built Red -> Green; see the tests below.
// ---------------------------------------------------------------------------

/// Extrinsic facts about a node — who it is on the network. Keyed by the
/// sparse-array index, a unique identity that is never recycled: a node
/// that has left keeps its slot (and its details) forever, and `join()`
/// always appends a fresh identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDetails {
    pub ipv6: String,
    pub port: u16,
    pub dns: String,
    pub rack: String,
    pub datacenter: String,
    pub cloud_region: String,
}

impl NodeDetails {
    pub fn new(
        ipv6: &str,
        port: u16,
        dns: &str,
        rack: &str,
        datacenter: &str,
        cloud_region: &str,
    ) -> Self {
        NodeDetails {
            ipv6: ipv6.to_owned(),
            port,
            dns: dns.to_owned(),
            rack: rack.to_owned(),
            datacenter: datacenter.to_owned(),
            cloud_region: cloud_region.to_owned(),
        }
    }
}

/// Why an era could not be built or advanced.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigurationError {
    /// A weight outside 0..=2 in the sparse array.
    InvalidWeights,
    /// Fewer than two voting nodes: no leader can be elected.
    TooFewVoters { voters: usize },
    /// The details array is not parallel to the sparse array.
    DetailsArity {
        sparse_len: usize,
        details_len: usize,
    },
    /// More `join()` operations in the series than details provided.
    MissingJoinedDetails { joins: usize, provided: usize },
    /// The operation series is not a legal reconfiguration step.
    Series(SeriesError),
}

/// One era: an immutable cluster configuration. Carries the sparse
/// weights array (index = unique, never-recycled node identity), the
/// dense voting-index array, N (its length, never below two), the view
/// number, L = view % N (an index into the dense array), the leader's
/// sparse index, and the parallel extrinsic details array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    weights: Vec<Option<u8>>,
    voters: Vec<usize>,
    view: u64,
    details: Vec<Option<NodeDetails>>,
}

impl Configuration {
    /// Build the era for a cluster state, a view number, and the details
    /// of every node identity. Refuses weights out of bounds, fewer than
    /// two voting nodes (no leader can be elected), and a details array
    /// that is not parallel to the sparse array.
    pub fn new(
        cluster: &Cluster,
        view: u64,
        details: Vec<Option<NodeDetails>>,
    ) -> Result<Self, ConfigurationError> {
        if !cluster.is_valid() {
            return Err(ConfigurationError::InvalidWeights);
        }
        let voters = cluster.leader_array();
        if voters.len() < 2 {
            return Err(ConfigurationError::TooFewVoters {
                voters: voters.len(),
            });
        }
        if details.len() != cluster.len() {
            return Err(ConfigurationError::DetailsArity {
                sparse_len: cluster.len(),
                details_len: details.len(),
            });
        }
        Ok(Configuration {
            weights: cluster.to_vec(),
            voters,
            view,
            details,
        })
    }

    /// Produce the next era by applying a series of operations. Runs the
    /// full legality check (valid states, per-step bounds, the leader
    /// rule, the end state, and the mass rule between the two eras), and
    /// carries details forward by identity, taking the details of every
    /// newly joined node from `joined` (extras are ignored). `next_view`
    /// is the caller's view policy for the new era (see `next_view`).
    pub fn reconfigure(
        &self,
        ops: &[Op],
        next_view: u64,
        joined: &[NodeDetails],
    ) -> Result<Configuration, ConfigurationError> {
        let joins = ops.iter().filter(|op| **op == Op::Join).count();
        if joined.len() < joins {
            return Err(ConfigurationError::MissingJoinedDetails {
                joins,
                provided: joined.len(),
            });
        }
        let from = self.cluster();
        let mut end = from.clone();
        for (step, op) in ops.iter().enumerate() {
            end.apply(*op).map_err(|error| {
                ConfigurationError::Series(SeriesError::Refused { step, error })
            })?;
        }
        validate_series(&from, &end, ops).map_err(ConfigurationError::Series)?;
        // Details travel with identities; join() appends a fresh identity.
        let mut details = self.details.clone();
        let mut next = 0;
        for op in ops {
            if *op == Op::Join {
                details.push(Some(joined[next].clone()));
                next += 1;
            }
        }
        Configuration::new(&end, next_view, details)
    }

    /// The underlying cluster state.
    pub fn cluster(&self) -> Cluster {
        Cluster::from_raw(self.weights.clone())
    }

    /// The sparse positional weights array; index = node identity.
    pub fn weights(&self) -> &[Option<u8>] {
        &self.weights
    }

    /// The dense array of voting-node indexes: no gaps, zero-indexed.
    pub fn voters(&self) -> &[usize] {
        &self.voters
    }

    /// N: the number of voting nodes; never below two in a built era.
    pub fn n(&self) -> usize {
        self.voters.len()
    }

    /// The view number of this era.
    pub fn view(&self) -> u64 {
        self.view
    }

    /// L = view % N: the leader's index into the dense array.
    pub fn l(&self) -> usize {
        (self.view % self.n() as u64) as usize
    }

    /// The leader: the sparse-array index of the voting node at dense
    /// index L.
    pub fn leader_index(&self) -> usize {
        self.voters[self.l()]
    }

    /// The extrinsic details of the node at sparse index `i`.
    pub fn details(&self, i: usize) -> Option<&NodeDetails> {
        self.details.get(i).and_then(|d| d.as_ref())
    }

    /// The extrinsic details of the current leader.
    pub fn leader_details(&self) -> Option<&NodeDetails> {
        self.details(self.leader_index())
    }

    /// The usual view policy: the next era's view is this view + 1.
    pub fn next_view(&self) -> u64 {
        self.view + 1
    }
}

impl std::fmt::Display for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, n) in self.weights.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match n {
                Some(w) => write!(f, "i{}={}", i, w)?,
                None => write!(f, "i{}=null", i)?,
            }
        }
        write!(
            f,
            "] | voters {:?} N={} view={} L={}",
            self.voters,
            self.n(),
            self.view,
            self.l()
        )?;
        if let Some(d) = self.leader_details() {
            write!(f, " leader=i{} ({} {})", self.leader_index(), d.ipv6, d.dns)?;
        } else {
            write!(f, " leader=i{}", self.leader_index())?;
        }
        Ok(())
    }
}

impl Cluster {
    /// A cluster of live nodes with the given weights.
    pub fn new(weights: &[u8]) -> Self {
        assert!(
            weights.iter().all(|w| *w <= MAX_WEIGHT),
            "weights must be in 0..=2"
        );
        Cluster {
            nodes: weights.iter().map(|w| Some(*w)).collect(),
        }
    }

    /// Build straight from raw slots (`None` = a node that has left).
    pub fn from_raw(nodes: Vec<Option<u8>>) -> Self {
        Cluster { nodes }
    }

    pub fn to_vec(&self) -> Vec<Option<u8>> {
        self.nodes.clone()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Total voting mass of all live nodes.
    pub fn mass(&self) -> u32 {
        self.nodes.iter().flatten().map(|w| u32::from(*w)).sum()
    }

    /// Viewstamped Replication (Revisited) leader array: the dense array
    /// of indexes of the voting nodes. A reduce-left over the sparse
    /// configuration that keeps the index of every node whose weight is
    /// 1 or 2 (a null slot has left, a 0 slot is leaving — neither votes).
    /// Only the index array is returned: callers need it for other purposes.
    pub fn leader_array(&self) -> Vec<usize> {
        self.nodes.iter().enumerate().fold(
            Vec::with_capacity(self.nodes.len()),
            |mut acc, (i, n)| {
                if matches!(n, Some(w) if *w == 1 || *w == 2) {
                    acc.push(i);
                }
                acc
            },
        )
    }

    /// Every slot holds a legal weight (0..=2) or is null.
    pub fn is_valid(&self) -> bool {
        self.nodes
            .iter()
            .all(|n| n.map_or(true, |w| w <= MAX_WEIGHT))
    }

    fn node(&self, i: usize) -> Result<u8, ClusterError> {
        match self.nodes.get(i) {
            None => Err(ClusterError::NoSuchNode(i)),
            Some(None) => Err(ClusterError::AlreadyLeft(i)),
            Some(Some(w)) => Ok(*w),
        }
    }

    /// Apply a single operation. On success every invariant holds;
    /// on failure the cluster is untouched.
    pub fn apply(&mut self, op: Op) -> Result<(), ClusterError> {
        match op {
            Op::Increment(i) => {
                let w = self.node(i)?;
                if w >= MAX_WEIGHT {
                    return Err(ClusterError::IncrementPastTwo(i));
                }
                self.nodes[i] = Some(w + 1);
            }
            Op::Decrement(i) => {
                let w = self.node(i)?;
                if w == 0 {
                    return Err(ClusterError::DecrementBelowZero(i));
                }
                self.nodes[i] = Some(w - 1);
            }
            Op::Join => self.nodes.push(Some(0)),
            Op::Leave(i) => {
                let w = self.node(i)?;
                if w != 0 {
                    return Err(ClusterError::LeaveWithWeight(i));
                }
                self.nodes[i] = None;
            }
            Op::Double => {
                if let Some(i) = self.nodes.iter().position(|n| *n == Some(MAX_WEIGHT)) {
                    return Err(ClusterError::DoubleWouldExceedTwo(i));
                }
                for n in self.nodes.iter_mut() {
                    if let Some(w) = n {
                        *w *= 2;
                    }
                }
            }
            Op::Halve => {
                if let Some(i) = self
                    .nodes
                    .iter()
                    .position(|n| matches!(n, Some(w) if w % 2 == 1))
                {
                    return Err(ClusterError::HalveWouldFraction(i));
                }
                for n in self.nodes.iter_mut() {
                    if let Some(w) = n {
                        *w /= 2;
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply a batch of operations "at the same time". The batch is atomic:
    /// the cluster only changes if every operation succeeds **and** the mass
    /// transition between the configuration before the batch and the one
    /// after it obeys the mass rule.
    pub fn apply_batch(&mut self, ops: &[Op]) -> Result<(), ClusterError> {
        let mut trial = self.clone();
        for op in ops {
            trial.apply(*op)?;
        }
        let (before, after) = (self.mass(), trial.mass());
        if allowed_transition(before, after) {
            *self = trial;
            Ok(())
        } else {
            Err(ClusterError::IllegalMassTransition { before, after })
        }
    }

    /// Every operation that could be attempted in this state (some may
    /// still be refused; callers filter by applying them).
    pub fn candidate_ops(&self, max_len: usize) -> Vec<Op> {
        let mut ops = vec![Op::Double, Op::Halve];
        if self.len() < max_len {
            ops.push(Op::Join);
        }
        for i in 0..self.len() {
            ops.push(Op::Increment(i));
            ops.push(Op::Decrement(i));
            ops.push(Op::Leave(i));
        }
        ops
    }
}

impl std::fmt::Display for Cluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, n) in self.nodes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match n {
                Some(w) => write!(f, "i{}={}", i, w)?,
                None => write!(f, "i{}=null", i)?,
            }
        }
        write!(f, "]")
    }
}

/// Every distinct configuration of a cluster of exactly `len` slots
/// (4 possibilities per slot: weight 0, 1, 2, or null).
pub fn all_states(len: usize) -> Vec<Cluster> {
    (0..4usize.pow(len as u32))
        .map(|mut d| {
            let mut nodes = Vec::with_capacity(len);
            for _ in 0..len {
                nodes.push(match d % 4 {
                    0 => Some(0),
                    1 => Some(1),
                    2 => Some(2),
                    _ => None,
                });
                d /= 4;
            }
            Cluster::from_raw(nodes)
        })
        .collect()
}

fn main() {
    println!("=== weighted-voting cluster: replacing node 1 of [1,1,1] ===");
    println!("mass rule: between configurations the mass may stay the same,");
    println!("change by exactly +/-1, all-double, or all-halve");
    println!();

    let mut c = Cluster::new(&[1, 1, 1]);
    println!("start   {}  mass = {}", c, c.mass());

    let before = c.mass();
    c.apply_batch(&[Op::Join, Op::Decrement(1)]).unwrap();
    println!();
    println!("batch 1 = [join(), decrement(1)]");
    println!(
        "interim {}  mass = {}   ({} -> {} is '{}')",
        c,
        c.mass(),
        before,
        c.mass(),
        describe_transition(before, c.mass())
    );

    let before = c.mass();
    c.apply_batch(&[Op::Increment(3), Op::Leave(1)]).unwrap();
    println!();
    println!("batch 2 = [increment(3), leave(1)]");
    println!(
        "final   {}  mass = {}   ({} -> {} is '{}')",
        c,
        c.mass(),
        before,
        c.mass(),
        describe_transition(before, c.mass())
    );
    println!();
    println!("node 1 is retired: a null slot no operation can touch.");
    println!("node 3 now carries its vote.");

    println!();
    println!("--- a batch whose net mass change is illegal is rejected atomically ---");
    let mut c2 = Cluster::new(&[1, 1, 1]);
    match c2.apply_batch(&[Op::Increment(0), Op::Increment(1)]) {
        Ok(()) => println!("BUG: illegal batch accepted"),
        Err(e) => println!(
            "rejected: {:?}\ncluster untouched: {}  mass = {}",
            e,
            c2,
            c2.mass()
        ),
    }

    println!();
    println!("--- series validation: is the operation series correct between two states? ---");
    let start = Cluster::new(&[1, 1, 1]);
    let interim = Cluster::from_raw(vec![Some(1), Some(0), Some(1), Some(0)]);
    let end = Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)]);
    println!(
        "batch 1  {} -> {}  transition = {:?}  validated = {:?}",
        start,
        interim,
        transition_between(&start, &interim),
        validate_series(&start, &interim, &[Op::Join, Op::Decrement(1)])
    );
    println!(
        "batch 2  {} -> {}  transition = {:?}  validated = {:?}",
        interim,
        end,
        transition_between(&interim, &end),
        validate_series(&interim, &end, &[Op::Increment(3), Op::Leave(1)])
    );
    println!(
        "net +2   {} -> {}  transition = {:?}  validated = {:?}",
        start,
        Cluster::new(&[2, 2, 1]),
        transition_between(&start, &Cluster::new(&[2, 2, 1])),
        validate_series(
            &start,
            &Cluster::new(&[2, 2, 1]),
            &[Op::Increment(0), Op::Increment(1)]
        )
    );
    let mut h = Cluster::new(&[1, 1, 1]);
    let halve_result = h.apply(Op::Halve);
    println!("halve()  {}  weight 1 present: {:?}", h, halve_result);

    println!();
    println!("--- Viewstamped Replication (Revisited) leader arrays ---");
    println!(
        "start   {}  leader array = {:?}",
        start,
        start.leader_array()
    );
    println!(
        "interim {}  leader array = {:?}",
        interim,
        interim.leader_array()
    );
    println!("end     {}  leader array = {:?}", end, end.leader_array());
    let two_voters = Cluster::new(&[1, 1]);
    let one_voter = Cluster::new(&[0, 1]);
    println!(
        "step to one voter  {} -> {}  validated = {:?}",
        two_voters,
        one_voter,
        validate_series(&two_voters, &one_voter, &[Op::Decrement(0)])
    );

    println!();
    println!("--- eras, views, and leaders (Viewstamped Replication, Revisited) ---");
    let d = |i: usize| {
        NodeDetails::new(
            &format!("2001:db8::{:x}", i + 1),
            4000,
            &format!("n{}.example", i),
            "rack-a",
            "dc-1",
            "eu-west",
        )
    };
    let era0 = Configuration::new(
        &Cluster::new(&[1, 1, 1]),
        6,
        vec![Some(d(0)), Some(d(1)), Some(d(2))],
    )
    .unwrap();
    println!("era 0  {}", era0);
    let era1 = era0
        .reconfigure(&[Op::Join, Op::Decrement(1)], era0.next_view(), &[d(3)])
        .unwrap();
    println!("era 1  {}", era1);
    let era2 = era1
        .reconfigure(&[Op::Increment(3), Op::Leave(1)], era1.next_view(), &[])
        .unwrap();
    println!("era 2  {}", era2);
    let drained = era0.reconfigure(&[Op::Decrement(0)], 6, &[]).unwrap();
    println!("the leader of era 0 drains its vote and vanishes from the dense array:");
    println!("       {}", drained);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    /// An independent restatement of the spec, written as a single in-place
    /// pass over the array, deliberately different in structure from
    /// `Cluster::apply`. The exhaustive test checks the two agree everywhere.
    fn oracle(state: &[Option<u8>], op: Op) -> Result<Vec<Option<u8>>, ClusterError> {
        use ClusterError::*;
        let mut s = state.to_vec();
        match op {
            Op::Join => {
                s.push(Some(0));
                Ok(s)
            }
            Op::Increment(i) => match s.get(i) {
                None => Err(NoSuchNode(i)),
                Some(None) => Err(AlreadyLeft(i)),
                Some(Some(2)) => Err(IncrementPastTwo(i)),
                Some(Some(w)) => {
                    s[i] = Some(*w + 1);
                    Ok(s)
                }
            },
            Op::Decrement(i) => match s.get(i) {
                None => Err(NoSuchNode(i)),
                Some(None) => Err(AlreadyLeft(i)),
                Some(Some(0)) => Err(DecrementBelowZero(i)),
                Some(Some(w)) => {
                    s[i] = Some(*w - 1);
                    Ok(s)
                }
            },
            Op::Leave(i) => match s.get(i) {
                None => Err(NoSuchNode(i)),
                Some(None) => Err(AlreadyLeft(i)),
                Some(Some(0)) => {
                    s[i] = None;
                    Ok(s)
                }
                Some(Some(_)) => Err(LeaveWithWeight(i)),
            },
            Op::Double => {
                for (i, n) in s.iter_mut().enumerate() {
                    match n {
                        Some(2) => return Err(DoubleWouldExceedTwo(i)),
                        Some(w) => *w *= 2,
                        None => {}
                    }
                }
                Ok(s)
            }
            Op::Halve => {
                for (i, n) in s.iter_mut().enumerate() {
                    match n {
                        Some(w) if *w % 2 == 1 => return Err(HalveWouldFraction(i)),
                        Some(w) => *w /= 2,
                        None => {}
                    }
                }
                Ok(s)
            }
        }
    }

    #[test]
    fn worked_example_replaces_node_1() {
        let mut c = Cluster::new(&[1, 1, 1]);
        assert_eq!(c.mass(), 3);

        // Batch 1: join a new 0-weight node and drain the node we want to remove.
        c.apply_batch(&[Op::Join, Op::Decrement(1)]).unwrap();
        assert_eq!(c.to_vec(), vec![Some(1), Some(0), Some(1), Some(0)]);
        assert_eq!(c.mass(), 2); // 3 -> 2 : legal (-1)

        // Batch 2: give the new node its vote, then retire the drained node.
        c.apply_batch(&[Op::Increment(3), Op::Leave(1)]).unwrap();
        assert_eq!(c.to_vec(), vec![Some(1), None, Some(1), Some(1)]);
        assert_eq!(c.mass(), 3); // 2 -> 3 : legal (+1)

        // The null slot is immune to everything.
        assert_eq!(c.apply(Op::Increment(1)), Err(ClusterError::AlreadyLeft(1)));
        assert_eq!(c.apply(Op::Decrement(1)), Err(ClusterError::AlreadyLeft(1)));
        assert_eq!(c.apply(Op::Leave(1)), Err(ClusterError::AlreadyLeft(1)));
        assert_eq!(c.mass(), 3);
    }

    #[test]
    fn bounds_are_enforced() {
        let mut c = Cluster::new(&[2, 0, 1]);
        assert_eq!(
            c.apply(Op::Increment(0)),
            Err(ClusterError::IncrementPastTwo(0))
        );
        assert_eq!(
            c.apply(Op::Decrement(1)),
            Err(ClusterError::DecrementBelowZero(1))
        );
        assert_eq!(
            c.apply(Op::Double),
            Err(ClusterError::DoubleWouldExceedTwo(0))
        );
        assert_eq!(c.apply(Op::Halve), Err(ClusterError::HalveWouldFraction(2)));
        assert_eq!(c.apply(Op::Leave(2)), Err(ClusterError::LeaveWithWeight(2)));
        assert_eq!(c.apply(Op::Increment(9)), Err(ClusterError::NoSuchNode(9)));
        assert!(c.is_valid() && c.mass() == 3); // all refusals changed nothing

        assert!(c.apply(Op::Leave(1)).is_ok()); // a weight-0 node may leave
        assert_eq!(c.to_vec(), vec![Some(2), None, Some(1)]);

        // double and halve stay exact: [2,null,1] -> refuse, but [0,1,1] halves.
        let mut d = Cluster::new(&[0, 1, 1]);
        d.apply(Op::Double).unwrap();
        assert_eq!(d.to_vec(), vec![Some(0), Some(2), Some(2)]); // mass 2 -> 4 = x2
        d.apply(Op::Halve).unwrap();
        assert_eq!(d.to_vec(), vec![Some(0), Some(1), Some(1)]); // mass 4 -> 2 = /2
    }

    #[test]
    fn batch_mass_rule_is_enforced_between_configurations() {
        let mut c = Cluster::new(&[1, 1, 1]);

        // Net +2 in one batch is not an allowed transition.
        let err = c
            .apply_batch(&[Op::Increment(0), Op::Increment(1)])
            .unwrap_err();
        assert_eq!(
            err,
            ClusterError::IllegalMassTransition {
                before: 3,
                after: 5
            }
        );
        assert_eq!(c.to_vec(), vec![Some(1), Some(1), Some(1)]); // untouched

        // An all-double batch is fine, and so is an all-halve batch.
        c.apply_batch(&[Op::Double]).unwrap();
        assert_eq!(c.to_vec(), vec![Some(2), Some(2), Some(2)]);
        c.apply_batch(&[Op::Halve]).unwrap();
        assert_eq!(c.to_vec(), vec![Some(1), Some(1), Some(1)]);

        // Mixing double with anything else breaks the exact-x2 rule.
        let err = c.apply_batch(&[Op::Double, Op::Decrement(0)]).unwrap_err();
        assert_eq!(
            err,
            ClusterError::IllegalMassTransition {
                before: 3,
                after: 5
            }
        );

        // The worked example's batches are fine: -1 then +1.
        c.apply_batch(&[Op::Join, Op::Decrement(1)]).unwrap(); // 3 -> 2
        c.apply_batch(&[Op::Increment(3), Op::Leave(1)]).unwrap(); // 2 -> 3
        assert_eq!(c.to_vec(), vec![Some(1), None, Some(1), Some(1)]);
    }

    /// The core proof: for every configuration of length 1..=4 and every
    /// operation, the implementation agrees with the independent oracle,
    /// success keeps the state valid and the mass transition legal, and
    /// failure changes nothing. 340 states x up to 15 ops, fully exhaustive.
    #[test]
    fn every_operation_respects_bounds_and_the_mass_rule() {
        let mut checked = 0usize;
        for len in 1..=4usize {
            for state in all_states(len) {
                // Every state with at least two voting nodes forms a legal
                // era, and the leader lookup stays consistent with the
                // dense array.
                if state.leader_array().len() >= 2 {
                    let era = Configuration::new(&state, 7, details_for(state.len())).unwrap();
                    assert_eq!(era.voters(), state.leader_array().as_slice());
                    assert_eq!(era.n(), era.voters().len());
                    assert!(era.l() < era.n());
                    assert_eq!(era.leader_index(), era.voters()[era.l()]);
                    assert!(matches!(era.weights()[era.leader_index()], Some(1 | 2)));
                }
                for op in state.candidate_ops(5) {
                    let expected = oracle(&state.to_vec(), op);

                    let mut c = state.clone();
                    let before_mass = c.mass();
                    let got = c.apply(op).map(|_| c.to_vec());

                    assert_eq!(got, expected, "state {} op {}", state, op);
                    checked += 1;

                    if let Ok(after) = &got {
                        // The resulting configuration is valid.
                        assert!(c.is_valid(), "{} then {} -> invalid {:?}", state, op, after);
                        // The mass transition obeys the rule.
                        assert!(
                            allowed_transition(before_mass, c.mass()),
                            "{} then {} : mass {} -> {}",
                            state,
                            op,
                            before_mass,
                            c.mass()
                        );
                        // The adjacent-state classifier agrees with the rule.
                        assert_eq!(
                            transition_between(&state, &c).is_legal(),
                            allowed_transition(before_mass, c.mass()),
                            "classifiers disagree for {} -> {}",
                            state,
                            c
                        );
                        // The leader array (VR-Revisited) matches an
                        // independent filter formulation.
                        let dense: Vec<usize> = c
                            .to_vec()
                            .iter()
                            .enumerate()
                            .filter(|(_, n)| **n == Some(1) || **n == Some(2))
                            .map(|(i, _)| i)
                            .collect();
                        assert_eq!(c.leader_array(), dense, "{} then {}", state, op);
                        if let Op::Join = op {
                            // join appends exactly one Some(0) at the end,
                            // leaving every earlier slot untouched.
                            assert_eq!(c.len(), state.len() + 1);
                            assert_eq!(after.last(), Some(&Some(0)));
                            assert_eq!(&after[..state.len()], &state.to_vec()[..]);
                        } else {
                            // no other operation ever changes the length.
                            assert_eq!(c.len(), state.len());
                        }
                    } else {
                        // A refused operation leaves the cluster untouched.
                        assert_eq!(c, state, "state {} op {}", state, op);
                    }
                }
            }
        }
        println!("exhaustive check: {} (state, op) pairs", checked);
    }

    /// Reachability proof: from [1,1,1], every valid configuration of
    /// length 3 or 4 can be reached by legal operations, and the
    /// node-replacement target needs exactly the 4 operations of the
    /// worked example.
    #[test]
    fn bfs_every_state_of_length_3_and_4_is_reachable() {
        let start = Cluster::new(&[1, 1, 1]);
        let max_len = 4usize;

        let mut dist: HashMap<Cluster, u32> = HashMap::new();
        let mut pred: HashMap<Cluster, (Cluster, Op)> = HashMap::new();
        let mut queue: VecDeque<Cluster> = VecDeque::new();
        dist.insert(start.clone(), 0);
        queue.push_back(start.clone());

        while let Some(c) = queue.pop_front() {
            for op in c.candidate_ops(max_len) {
                let mut next = c.clone();
                if next.apply(op).is_ok() && !dist.contains_key(&next) {
                    dist.insert(next.clone(), dist[&c] + 1);
                    pred.insert(next.clone(), (c.clone(), op));
                    queue.push_back(next);
                }
            }
        }

        // Every valid 3- and 4-slot configuration is reachable, and nothing else.
        let all: HashSet<Cluster> = all_states(3).into_iter().chain(all_states(4)).collect();
        for s in &all {
            assert!(dist.contains_key(s), "{} is unreachable", s);
        }
        assert_eq!(dist.len(), all.len(), "unexpected extra states reachable");

        // The node-replacement target is exactly 4 operations away:
        // join(), decrement(1), increment(3), leave(1) is optimal.
        let target = Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)]);
        assert_eq!(dist[&target], 4);

        let mut path = vec![];
        let mut cur = target.clone();
        while let Some((p, op)) = pred.get(&cur).cloned() {
            path.push(op);
            cur = p;
        }
        path.reverse();
        let shown: Vec<String> = path.iter().map(|op| op.to_string()).collect();
        println!("shortest reconfiguration: [{}]", shown.join(", "));

        // Replaying the found path really works, batch by batch.
        let mut c = start;
        for op in &path {
            c.apply_batch(&[*op]).unwrap();
        }
        assert_eq!(c, target);

        // And the two-batch plan from the problem statement does the same.
        let mut c = Cluster::new(&[1, 1, 1]);
        c.apply_batch(&[Op::Join, Op::Decrement(1)]).unwrap();
        c.apply_batch(&[Op::Increment(3), Op::Leave(1)]).unwrap();
        assert_eq!(c, target);
    }

    // -----------------------------------------------------------------
    // Red/Green TDD cycle 2: adjacent-state validation and series checking.
    // -----------------------------------------------------------------

    #[test]
    fn adjacent_states_are_classified_by_mass_change() {
        let s = |ws: &[u8]| Cluster::new(ws);
        // total mass changed by zero
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[1, 1, 1])),
            Transition::Same
        );
        // total mass changed by one, up or down
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[2, 1, 1])),
            Transition::PlusOne
        );
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[0, 1, 1])),
            Transition::MinusOne
        );
        // all-double / all-halve
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[2, 2, 2])),
            Transition::Doubled
        );
        assert_eq!(
            transition_between(&s(&[2, 2, 2]), &s(&[1, 1, 1])),
            Transition::Halved
        );
        // 1 -> 2 is both +1 and x2; the atomic step is +1
        assert_eq!(transition_between(&s(&[1]), &s(&[2])), Transition::PlusOne);
        // +2 or -2 between two adjacent states is illegal
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[2, 2, 1])),
            Transition::Illegal
        );
        assert_eq!(
            transition_between(&s(&[1, 1, 1]), &s(&[1, 0, 0])),
            Transition::Illegal
        );
        // null slots carry no mass, so leave() keeps the mass the same
        assert_eq!(
            transition_between(
                &Cluster::from_raw(vec![Some(1), Some(0), Some(1)]),
                &Cluster::from_raw(vec![Some(1), None, Some(1)]),
            ),
            Transition::Same
        );
    }

    #[test]
    fn halve_is_refused_when_any_node_is_at_weight_one() {
        // halving a weight-1 node would create a fractional weight
        let mut c = Cluster::new(&[1, 1, 1]);
        assert_eq!(c.apply(Op::Halve), Err(ClusterError::HalveWouldFraction(0)));
        // refused means untouched
        assert_eq!(c.to_vec(), vec![Some(1), Some(1), Some(1)]);
        assert_eq!(c.mass(), 3);

        // one weight-1 node anywhere blocks the whole halve
        let mut d = Cluster::new(&[2, 1, 0]);
        assert_eq!(d.apply(Op::Halve), Err(ClusterError::HalveWouldFraction(1)));

        // ... even next to a null slot
        let mut e = Cluster::from_raw(vec![Some(2), None, Some(1)]);
        assert_eq!(e.apply(Op::Halve), Err(ClusterError::HalveWouldFraction(2)));

        // all-even halve is fine
        let mut f = Cluster::new(&[2, 0, 2]);
        f.apply(Op::Halve).unwrap();
        assert_eq!(f.to_vec(), vec![Some(1), Some(0), Some(1)]);
    }

    #[test]
    fn a_correct_series_between_two_states_is_validated() {
        let from = Cluster::new(&[1, 1, 1]);
        let interim = Cluster::from_raw(vec![Some(1), Some(0), Some(1), Some(0)]);
        let end = Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)]);

        // the two batches of the worked example, one adjacent step each
        assert_eq!(transition_between(&from, &interim), Transition::MinusOne);
        assert_eq!(
            validate_series(&from, &interim, &[Op::Join, Op::Decrement(1)]),
            Ok(())
        );
        assert_eq!(transition_between(&interim, &end), Transition::PlusOne);
        assert_eq!(
            validate_series(&interim, &end, &[Op::Increment(3), Op::Leave(1)]),
            Ok(())
        );
    }

    #[test]
    fn a_series_ending_in_the_wrong_state_is_rejected() {
        let from = Cluster::new(&[1, 1, 1]);
        let wrong_end = Cluster::from_raw(vec![Some(1), None, Some(1), Some(2)]);
        let ops = [Op::Join, Op::Decrement(1), Op::Increment(3), Op::Leave(1)];
        match validate_series(&from, &wrong_end, &ops) {
            Err(SeriesError::WrongEndState { expected, actual }) => {
                assert_eq!(expected, wrong_end);
                assert_eq!(
                    actual,
                    Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)])
                );
            }
            other => panic!("expected WrongEndState, got {:?}", other),
        }
    }

    #[test]
    fn a_series_containing_an_illegal_halve_is_rejected_at_that_step() {
        let from = Cluster::new(&[1, 1, 1]);
        let to = Cluster::from_raw(vec![Some(0), Some(1), Some(1)]);
        let ops = [Op::Decrement(0), Op::Halve, Op::Increment(0)];
        // step 0 succeeds ([0,1,1]); step 1 is refused: node 1 has weight 1
        assert_eq!(
            validate_series(&from, &to, &ops),
            Err(SeriesError::Refused {
                step: 1,
                error: ClusterError::HalveWouldFraction(1)
            })
        );
    }

    #[test]
    fn a_series_with_an_illegal_net_mass_change_is_rejected() {
        // every operation is individually legal and the end state matches,
        // but +2 between two adjacent states breaks the mass rule
        let from = Cluster::new(&[1, 1, 1]);
        let to = Cluster::new(&[2, 2, 1]);
        let ops = [Op::Increment(0), Op::Increment(1)];
        assert_eq!(
            validate_series(&from, &to, &ops),
            Err(SeriesError::IllegalMassTransition {
                before: 3,
                after: 5
            })
        );
    }

    #[test]
    fn series_rejects_states_that_are_not_valid_configurations() {
        let bad = Cluster::from_raw(vec![Some(1), Some(3)]); // weight 3: illegal
        let ok = Cluster::new(&[1, 1]);
        assert_eq!(
            validate_series(&bad, &ok, &[]),
            Err(SeriesError::InvalidState(bad.clone()))
        );
        assert_eq!(
            validate_series(&ok, &bad, &[]),
            Err(SeriesError::InvalidState(bad.clone()))
        );
    }

    // -----------------------------------------------------------------
    // Red/Green TDD cycle 3: VR-Revisited leader arrays.
    // -----------------------------------------------------------------

    #[test]
    fn leader_array_collects_indexes_of_voting_nodes() {
        // every weight-1 node votes
        assert_eq!(Cluster::new(&[1, 1, 1]).leader_array(), vec![0, 1, 2]);
        // weight-2 nodes vote too; zeros do not
        assert_eq!(Cluster::new(&[2, 0, 2]).leader_array(), vec![0, 2]);
        // null slots do not vote and keep their holes
        assert_eq!(
            Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)]).leader_array(),
            vec![0, 2, 3]
        );
        assert_eq!(
            Cluster::from_raw(vec![None, Some(2), None, Some(1)]).leader_array(),
            vec![1, 3]
        );
        // the worked example's interim configuration
        assert_eq!(
            Cluster::from_raw(vec![Some(1), Some(0), Some(1), Some(0)]).leader_array(),
            vec![0, 2]
        );
        // nobody votes
        assert_eq!(Cluster::new(&[0, 0, 0]).leader_array(), Vec::<usize>::new());
    }

    #[test]
    fn a_series_may_never_drop_below_two_voting_nodes() {
        // end state with one voter: rejected even though -1 is a legal delta
        let from = Cluster::new(&[1, 1]);
        let to = Cluster::new(&[0, 1]);
        assert_eq!(
            validate_series(&from, &to, &[Op::Decrement(0)]),
            Err(SeriesError::TooFewVoters {
                state: to.clone(),
                voters: 1
            })
        );

        // start state with one voter: rejected before anything runs
        let lone = Cluster::new(&[1, 0]);
        let to = Cluster::new(&[1, 1]);
        assert_eq!(
            validate_series(&lone, &to, &[Op::Increment(1)]),
            Err(SeriesError::TooFewVoters {
                state: lone.clone(),
                voters: 1
            })
        );

        // legal delta, legal end state, but the series dips to one voter
        // in the middle: [1,1,1] -> [0,1,1] -> [0,0,1] -> [1,0,1]
        let from = Cluster::new(&[1, 1, 1]);
        let to = Cluster::from_raw(vec![Some(1), Some(0), Some(1)]);
        assert_eq!(
            validate_series(
                &from,
                &to,
                &[Op::Decrement(0), Op::Decrement(1), Op::Increment(0)]
            ),
            Err(SeriesError::TooFewVoters {
                state: Cluster::new(&[0, 0, 1]),
                voters: 1
            })
        );

        // the same reconfiguration without dipping is fine
        assert_eq!(validate_series(&from, &to, &[Op::Decrement(1)]), Ok(()));

        // and the worked example keeps >= 2 voters throughout
        let start = Cluster::new(&[1, 1, 1]);
        let interim = Cluster::from_raw(vec![Some(1), Some(0), Some(1), Some(0)]);
        let end = Cluster::from_raw(vec![Some(1), None, Some(1), Some(1)]);
        assert_eq!(
            validate_series(&start, &interim, &[Op::Join, Op::Decrement(1)]),
            Ok(())
        );
        assert_eq!(
            validate_series(&interim, &end, &[Op::Increment(3), Op::Leave(1)]),
            Ok(())
        );
    }

    // -----------------------------------------------------------------
    // Red/Green TDD cycle 4: eras, views, and leaders.
    // -----------------------------------------------------------------

    fn node_detail(i: usize) -> NodeDetails {
        NodeDetails::new(
            &format!("2001:db8::{:x}", i + 1),
            4000 + i as u16,
            &format!("n{}.example", i),
            "rack-a",
            "dc-1",
            "eu-west",
        )
    }

    fn details_for(len: usize) -> Vec<Option<NodeDetails>> {
        (0..len).map(|i| Some(node_detail(i))).collect()
    }

    #[test]
    fn an_era_holds_view_leader_and_parallel_details() {
        let c = Cluster::new(&[1, 1, 1]);
        let era = Configuration::new(&c, 6, details_for(3)).unwrap();
        assert_eq!(era.weights(), &[Some(1), Some(1), Some(1)]);
        assert_eq!(era.voters(), &[0, 1, 2]); // dense: no gaps
        assert_eq!(era.n(), 3);
        assert_eq!(era.view(), 6);
        assert_eq!(era.l(), 0); // 6 % 3
        assert_eq!(era.leader_index(), 0); // dense[0] = sparse 0
        assert_eq!(era.leader_details().unwrap().dns, "n0.example");
        assert_eq!(era.leader_details().unwrap().ipv6, "2001:db8::1");

        // advancing the view inside the same N rotates the leader
        let era = Configuration::new(&c, 7, details_for(3)).unwrap();
        assert_eq!((era.l(), era.leader_index()), (1, 1));
        assert_eq!(era.leader_details().unwrap().dns, "n1.example");
        let era = Configuration::new(&c, 8, details_for(3)).unwrap();
        assert_eq!((era.l(), era.leader_index()), (2, 2));
    }

    #[test]
    fn the_leader_is_found_through_the_dense_array_not_the_sparse_index() {
        // i1 = 0 (leaving) and i2 = null (left) do not vote
        let c = Cluster::from_raw(vec![Some(1), Some(0), None, Some(1)]);
        let era = Configuration::new(&c, 0, details_for(4)).unwrap();
        assert_eq!(era.voters(), &[0, 3]);
        assert_eq!(era.n(), 2);
        assert_eq!((era.l(), era.leader_index()), (0, 0));
        // view 1: L = 1 % 2 = 1, dense[1] = sparse 3
        let era = Configuration::new(&c, 1, details_for(4)).unwrap();
        assert_eq!((era.l(), era.leader_index()), (1, 3));
        assert_eq!(era.leader_details().unwrap().dns, "n3.example");
    }

    #[test]
    fn a_retired_leader_disappears_from_the_dense_array_forever() {
        let era = Configuration::new(&Cluster::new(&[1, 1, 1]), 6, details_for(3)).unwrap();
        assert_eq!(era.leader_index(), 0); // node0 leads

        // drain the leader's vote: it drops out of the dense array
        let drained = era.reconfigure(&[Op::Decrement(0)], 6, &[]).unwrap();
        assert_eq!(drained.weights(), &[Some(0), Some(1), Some(1)]);
        assert_eq!(drained.voters(), &[1, 2]);
        assert_eq!(
            (drained.n(), drained.l(), drained.leader_index()),
            (2, 0, 1)
        );

        // once it leaves, its slot is null and no operation can touch it
        let gone = drained.reconfigure(&[Op::Leave(0)], 7, &[]).unwrap();
        assert_eq!(gone.weights(), &[None, Some(1), Some(1)]);
        assert_eq!(gone.voters(), &[1, 2]);
        assert_eq!((gone.n(), gone.l(), gone.leader_index()), (2, 1, 2));
        // ... but its identity is never recycled and its details remain
        assert_eq!(gone.details(0).unwrap().dns, "n0.example");
        let mut dead = gone.cluster();
        assert_eq!(
            dead.apply(Op::Increment(0)),
            Err(ClusterError::AlreadyLeft(0))
        );
    }

    #[test]
    fn eras_track_view_and_leader_through_the_worked_example() {
        let era0 = Configuration::new(&Cluster::new(&[1, 1, 1]), 6, details_for(3)).unwrap();
        assert_eq!((era0.n(), era0.l(), era0.leader_index()), (3, 0, 0));

        // batch 1: join a new node, drain node 1; view 6 -> 7
        let era1 = era0
            .reconfigure(&[Op::Join, Op::Decrement(1)], 7, &[node_detail(3)])
            .unwrap();
        assert_eq!(era1.weights(), &[Some(1), Some(0), Some(1), Some(0)]);
        assert_eq!(era1.voters(), &[0, 2]);
        assert_eq!((era1.n(), era1.l(), era1.leader_index()), (2, 1, 2));

        // batch 2: give the new node its vote, retire node 1; view 7 -> 8
        let era2 = era1
            .reconfigure(&[Op::Increment(3), Op::Leave(1)], 8, &[])
            .unwrap();
        assert_eq!(era2.weights(), &[Some(1), None, Some(1), Some(1)]);
        assert_eq!(era2.voters(), &[0, 2, 3]);
        assert_eq!((era2.n(), era2.l(), era2.leader_index()), (3, 2, 3));
        // leadership rotated 0 -> 2 -> 3 across the eras

        // identities are stable: the left node keeps its details, the
        // joined node carries its own
        assert_eq!(era2.details(0).unwrap().dns, "n0.example");
        assert_eq!(era2.details(1).unwrap().dns, "n1.example");
        assert_eq!(era2.details(3).unwrap().dns, "n3.example");
        assert_eq!(era2.leader_details().unwrap().dns, "n3.example");
    }

    #[test]
    fn an_era_cannot_be_built_from_a_broken_state() {
        // fewer than two voting nodes: no leader can be elected
        let err = Configuration::new(&Cluster::new(&[0, 1]), 0, details_for(2)).unwrap_err();
        assert_eq!(err, ConfigurationError::TooFewVoters { voters: 1 });
        // details array not parallel to the sparse array
        let err = Configuration::new(&Cluster::new(&[1, 1]), 0, details_for(3)).unwrap_err();
        assert_eq!(
            err,
            ConfigurationError::DetailsArity {
                sparse_len: 2,
                details_len: 3
            }
        );
        // weight out of bounds
        let bad = Cluster::from_raw(vec![Some(3), Some(1)]);
        let err = Configuration::new(&bad, 0, details_for(2)).unwrap_err();
        assert_eq!(err, ConfigurationError::InvalidWeights);
    }

    #[test]
    fn reconfigure_only_accepts_legal_series() {
        let era = Configuration::new(&Cluster::new(&[1, 1, 1]), 6, details_for(3)).unwrap();

        // net +2 between adjacent states
        let err = era
            .reconfigure(&[Op::Increment(0), Op::Increment(1)], 7, &[])
            .unwrap_err();
        assert_eq!(
            err,
            ConfigurationError::Series(SeriesError::IllegalMassTransition {
                before: 3,
                after: 5
            })
        );
        // dipping below two voters mid-series
        let err = era
            .reconfigure(
                &[Op::Decrement(0), Op::Decrement(1), Op::Increment(0)],
                7,
                &[],
            )
            .unwrap_err();
        assert!(matches!(
            err,
            ConfigurationError::Series(SeriesError::TooFewVoters { voters: 1, .. })
        ));
        // halve with a weight-1 node at step 0
        let err = era.reconfigure(&[Op::Halve], 7, &[]).unwrap_err();
        assert_eq!(
            err,
            ConfigurationError::Series(SeriesError::Refused {
                step: 0,
                error: ClusterError::HalveWouldFraction(0)
            })
        );
        // join() without details for the new identity
        let err = era
            .reconfigure(&[Op::Join, Op::Decrement(1)], 7, &[])
            .unwrap_err();
        assert_eq!(
            err,
            ConfigurationError::MissingJoinedDetails {
                joins: 1,
                provided: 0
            }
        );
    }
}
