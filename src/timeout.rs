//! The flavoured-timeout model: given a state, a clock, and an admin policy
//! that an unknown waiting time is exceeded, what may the node do.
//!
//! A node's timeout is a local clock event on a single-threaded host timer.
//! Its duration is never prescribed, may be dynamic, and is host policy, not
//! protocol: the core reads no clock (S4, `docs/uvrr-durability-model.md`
//! §13.2), and a duration belongs to the host the way
//! [`crate::replica::ViewChangeKnobs::primary_timeout`] does. What the node may DO when the
//! waiting time is exceeded is protocol, and this module is that answer as a
//! total function: the state enum and the timeout enum are closed, the
//! matcher is exhaustive over every pair with no default arm, and a new
//! variant cannot enter either enum without the matcher being revisited.
//!
//! The matcher returns opinions, not prose: the legal family is retransmit,
//! do nothing, heartbeat, or start a view change. A delay never panics: a
//! stall that may clear is the operator's business, and for the states
//! outside the protocol's proof the matcher returns [`Opinion::Sorry`]
//! carrying the runbook statement rather than faulting the node over time it
//! does not own.

/// A node's protocol condition. The states are finite and the enum is
/// exhaustive; a new state cannot enter without the matcher being revisited.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum State {
    /// Seated in the cluster: a member whose votes count in a quorum.
    InTheCluster,
    /// A witness to the stream: it never votes, and a clock event on it is
    /// not protocol.
    Witness,
    /// Unknown to any other node: no quorum counts it, no message addresses
    /// it, and its clock is its own.
    Unknown,
    /// Booted, walking the boot machine's own path: the machine owns its
    /// progress, not a timeout.
    Booted,
    /// Crashed: the process is gone, and the timeout belongs to whatever
    /// the host does about corpses.
    Crashed,
    /// In a steady state: a seated member in a settled view, the state whose
    /// waits carry the suspicion and the heartbeat opinions.
    Steady,
    /// Stopping, walking the controlled halt's drain: the drain owns the
    /// stop, and a timeout adds nothing.
    Stopping,
    /// Stopping but not flushed: the stall this presents is outside the
    /// protocol's proof, and the matcher answers with [`Opinion::Sorry`].
    StoppingNotFlushed,
}

impl State {
    /// Every state, in the enum's declared order: the policy tool's usage
    /// enumerates the domain from it, so a new variant cannot enter the
    /// enum without the tool naming it.
    pub const ALL: [State; 8] = [
        State::InTheCluster,
        State::Witness,
        State::Unknown,
        State::Booted,
        State::Crashed,
        State::Steady,
        State::Stopping,
        State::StoppingNotFlushed,
    ];

    /// The name, for every surface a human reads: the policy tool takes it
    /// as an argument, and the name is stated beside the variant it names
    /// so the two cannot drift apart.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            State::InTheCluster => "in-the-cluster",
            State::Witness => "witness",
            State::Unknown => "unknown",
            State::Booted => "booted",
            State::Crashed => "crashed",
            State::Steady => "steady",
            State::Stopping => "stopping",
            State::StoppingNotFlushed => "stopping-not-flushed",
        }
    }
}

/// A timeout flavour, one per state. The types make the durations explicit
/// and unprescribed: no variant carries a number, because a duration is host
/// policy, may be dynamic, and is never the protocol's to state.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Timeout {
    /// The wait of a member seated in the cluster: the quorum response the
    /// working member awaits.
    Cluster,
    /// A clock event on a witness.
    Witness,
    /// A clock event on a node unknown to the others.
    Unknown,
    /// A clock event while booted.
    Booted,
    /// A clock event on a crashed node.
    Crashed,
    /// The wait of a steady member: the suspicion a backup reads, and the
    /// quiet a healthy leader answers with its last commit.
    Steady,
    /// A clock event while stopping.
    Stopping,
    /// A clock event while stopping but not flushed.
    StoppingNotFlushed,
}

impl Timeout {
    /// Every timeout flavour, in the enum's declared order: the policy
    /// tool's usage enumerates the domain from it, so a new variant cannot
    /// enter the enum without the tool naming it.
    pub const ALL: [Timeout; 8] = [
        Timeout::Cluster,
        Timeout::Witness,
        Timeout::Unknown,
        Timeout::Booted,
        Timeout::Crashed,
        Timeout::Steady,
        Timeout::Stopping,
        Timeout::StoppingNotFlushed,
    ];

    /// The name, for every surface a human reads: the policy tool takes it
    /// as an argument, and the name is stated beside the variant it names
    /// so the two cannot drift apart.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Timeout::Cluster => "cluster",
            Timeout::Witness => "witness",
            Timeout::Unknown => "unknown",
            Timeout::Booted => "booted",
            Timeout::Crashed => "crashed",
            Timeout::Steady => "steady",
            Timeout::Stopping => "stopping",
            Timeout::StoppingNotFlushed => "stopping-not-flushed",
        }
    }
}

/// What the node may do when the waiting time is exceeded: an opinion, not
/// prose, and never a panic over time the node does not own.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Opinion {
    /// Resend what has not been received a response on: the un-acknowledged
    /// traffic, a relay of what the core already released
    /// (`docs/uvrr-durability-model.md` §13.2, the transport replay).
    Retransmit,
    /// The clock event is not protocol for this condition: nothing follows
    /// from it.
    DoNothing,
    /// A leader with no outstanding matters and matching frontiers may send
    /// its last commit as a heartbeat: the frontier announcement and the
    /// proof of life in one, the host's option of suggestion grade.
    Heartbeat,
    /// Start a view change: the steady member's suspicion of its primary,
    /// resolved through the fence exchange and the quorum.
    StartViewChange,
    /// The state is outside the protocol's proof, and the runbook statement
    /// carried here is the whole of what the matcher will say about it: a
    /// node stopping but not flushed presents a stall the operator may
    /// resolve by freeing disk space and retrying the flush, or by a hard
    /// kill, and which of those is wanted is a policy we do not comprehend
    /// and will never decide.
    Sorry {
        /// The runbook statement, carried in the type, never a guess.
        runbook: &'static str,
    },
}

impl Opinion {
    /// The name, for every surface a human reads: the policy tool prints it,
    /// and the name is stated beside the variant it names so the two cannot
    /// drift apart.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Opinion::Retransmit => "retransmit",
            Opinion::DoNothing => "do-nothing",
            Opinion::Heartbeat => "heartbeat",
            Opinion::StartViewChange => "start-view-change",
            Opinion::Sorry { .. } => "sorry",
        }
    }
}

/// The runbook statement a node stopping but not flushed carries: stated
/// once, beside the matcher that returns it, so the type and the text are
/// what the tool prints and nothing else.
pub const UNFLUSHED_RUNBOOK: &str = "a node stopping but not flushed presents a stall the operator may resolve by freeing disk space and retrying the flush, or by a hard kill, and which of those is wanted is a policy we do not comprehend and will never decide";

/// The Sorry policy matcher: the exhaustive opinion for a state and a
/// timeout, every pair named, with no default arm anywhere.
///
/// The rule it exhausts: a clock event on a non-agent (a witness, a node
/// unknown to the others, a booted node, a crashed node, a stopping node)
/// is not protocol, and the opinion is [`Opinion::DoNothing`]; a node
/// stopping but not flushed is outside the proof, and every timeout on it
/// answers [`Opinion::Sorry`] with the runbook statement; an agent (a member
/// seated in the cluster, a steady member) answers the flavour it met: the
/// cluster wait resends the un-acknowledged traffic, the steady wait of a
/// seated member with nothing outstanding is the heartbeat option, and the
/// steady wait of a steady member is the suspicion that starts the view
/// change. A foreign non-agent flavour on an agent is not protocol for it
/// either: the opinion is [`Opinion::DoNothing`].
#[must_use]
pub fn matcher(state: State, timeout: Timeout) -> Opinion {
    use Opinion::{DoNothing, Heartbeat, Retransmit, Sorry, StartViewChange};
    use State::{
        Booted, Crashed, InTheCluster, Steady, Stopping, StoppingNotFlushed, Unknown, Witness,
    };
    use Timeout::{Cluster, Steady as SteadyWait};
    match (state, timeout) {
        // The unflushed stop outranks every flavour it can meet: the stall
        // is outside the proof, whatever clock fired on it.
        (StoppingNotFlushed, Timeout::Cluster)
        | (StoppingNotFlushed, Timeout::Witness)
        | (StoppingNotFlushed, Timeout::Unknown)
        | (StoppingNotFlushed, Timeout::Booted)
        | (StoppingNotFlushed, Timeout::Crashed)
        | (StoppingNotFlushed, SteadyWait)
        | (StoppingNotFlushed, Timeout::Stopping)
        | (StoppingNotFlushed, Timeout::StoppingNotFlushed) => Sorry {
            runbook: UNFLUSHED_RUNBOOK,
        },
        // The non-agents: a clock event on them is not protocol.
        (Witness, Timeout::Cluster)
        | (Witness, Timeout::Witness)
        | (Witness, Timeout::Unknown)
        | (Witness, Timeout::Booted)
        | (Witness, Timeout::Crashed)
        | (Witness, SteadyWait)
        | (Witness, Timeout::Stopping)
        | (Witness, Timeout::StoppingNotFlushed)
        | (Unknown, Timeout::Cluster)
        | (Unknown, Timeout::Witness)
        | (Unknown, Timeout::Unknown)
        | (Unknown, Timeout::Booted)
        | (Unknown, Timeout::Crashed)
        | (Unknown, SteadyWait)
        | (Unknown, Timeout::Stopping)
        | (Unknown, Timeout::StoppingNotFlushed)
        | (Booted, Timeout::Cluster)
        | (Booted, Timeout::Witness)
        | (Booted, Timeout::Unknown)
        | (Booted, Timeout::Booted)
        | (Booted, Timeout::Crashed)
        | (Booted, SteadyWait)
        | (Booted, Timeout::Stopping)
        | (Booted, Timeout::StoppingNotFlushed)
        | (Crashed, Timeout::Cluster)
        | (Crashed, Timeout::Witness)
        | (Crashed, Timeout::Unknown)
        | (Crashed, Timeout::Booted)
        | (Crashed, Timeout::Crashed)
        | (Crashed, SteadyWait)
        | (Crashed, Timeout::Stopping)
        | (Crashed, Timeout::StoppingNotFlushed)
        | (Stopping, Timeout::Cluster)
        | (Stopping, Timeout::Witness)
        | (Stopping, Timeout::Unknown)
        | (Stopping, Timeout::Booted)
        | (Stopping, Timeout::Crashed)
        | (Stopping, SteadyWait)
        | (Stopping, Timeout::Stopping)
        | (Stopping, Timeout::StoppingNotFlushed) => DoNothing,
        // The cluster wait: resend what has not been answered.
        (InTheCluster, Cluster) | (Steady, Cluster) => Retransmit,
        // A seated member with nothing outstanding at the steady quiet: the
        // heartbeat option, the host's to take or leave.
        (InTheCluster, SteadyWait) => Heartbeat,
        // A steady member at its own wait: the suspicion, and the view change.
        (Steady, SteadyWait) => StartViewChange,
        // An agent meeting a non-agent's flavour: the event is not protocol
        // for the agent either.
        (InTheCluster, Timeout::Witness)
        | (InTheCluster, Timeout::Unknown)
        | (InTheCluster, Timeout::Booted)
        | (InTheCluster, Timeout::Crashed)
        | (InTheCluster, Timeout::Stopping)
        | (InTheCluster, Timeout::StoppingNotFlushed)
        | (Steady, Timeout::Witness)
        | (Steady, Timeout::Unknown)
        | (Steady, Timeout::Booted)
        | (Steady, Timeout::Crashed)
        | (Steady, Timeout::Stopping)
        | (Steady, Timeout::StoppingNotFlushed) => DoNothing,
    }
}
