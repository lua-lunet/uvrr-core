//! Pre-voting: the handshake a candidate runs before it is entitled to a phase 1.
//!
//! A candidate does not propose because a wait expired. It proposes because it asked, the
//! peers answered, and the answers carried both quorum families. This module is the
//! asking and the answering: the four roles, the two waits, and the total function from
//! an event to the action it obliges. The specification is `docs/prevoting.md`, and this
//! code is judged against that document.
//!
//! # What is here and what is not
//!
//! Here: the role machine, the message alphabet, the reply rule, and the verdict the
//! collected replies authorise. All four are pure functions over closed domains, and the
//! role function has no default arm, so a new variant cannot enter either enum without
//! the table being revisited.
//!
//! Not here, and deliberately: the wire. No [`crate::wire::Tag`], no
//! [`crate::message::Body`], no codec. Nothing in [`crate::replica`] consults this
//! module and no [`crate::effects::Effect`] it describes is ever released. It is a
//! specification that compiles and is tested, not a code path.
//!
//! # Nothing here is persisted
//!
//! Every field is volatile and none survives a restart in either direction. A clean
//! shutdown loses it and the node re-derives its role from what it learns; a crash
//! restart loses it and the reincarnated node rejoins as a weight-0 standby caught up by
//! ordinary state transfer. This is the point rather than an omission: the state is a
//! belief about failure, and a durable belief about failure is what forces a design to
//! reconcile a record of suspicion with a record of progress, a reconciliation this
//! protocol has no crash-recover class to hold (`docs/prevoting.md`, the ephemeral-state section).
//!
//! # The two waits
//!
//! A pre-voting node runs two waits and the protocol states only their order. **No
//! variant in this module carries a duration**, for the reason `crate::timeout` states of
//! its own flavours: a duration is host policy, may be dynamic, and is never the
//! protocol's to state. What crosses this boundary is which wait expired, not how long
//! it took.

use crate::ids::{Ballot, NodeId};

pub mod alphabet;
pub mod verdict;

/// Who owns the ballot in which the value this node last saw chosen was chosen.
///
/// Every ballot has a well-defined owner — [`crate::configuration::Configuration::primary`]
/// names the member it selects — so ownership is a fact about the ballot, not an election
/// result.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ownership {
    /// This node owns the ballot.
    Mine,
    /// That named member owns the ballot.
    Theirs(NodeId),
    /// No chosen value is known, or one is known whose ballot cannot be attributed to a
    /// member of the configuration this node holds. The second case is what a
    /// reconfiguration produces: a node may hold a chosen value whose ballot belongs to a
    /// generation it has not adopted. Claiming a seat on an unattributable ballot would be
    /// claiming one it cannot prove, so this answers [`Role::Candidate`].
    Unknown,
}

impl Ownership {
    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Ownership::Mine => "mine",
            Ownership::Theirs(_) => "theirs",
            Ownership::Unknown => "unknown",
        }
    }
}

/// How recently the value this node last saw chosen was chosen, as the protocol sees it.
///
/// This is the *rank* of a choice against the two waits, never a duration: the shorter
/// wait separates [`Recency::VeryRecent`] from [`Recency::Recent`], the longer separates
/// [`Recency::Recent`] from [`Recency::Stale`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Recency {
    /// Within the leadership wait: fresh enough to lead.
    VeryRecent,
    /// Within the recency wait but not the leadership wait: fresh enough to be an
    /// incumbent, not fresh enough to be the leader.
    Recent,
    /// Older than the recency wait: nothing to be incumbent over.
    Stale,
    /// No value is known to have been chosen.
    Never,
}

impl Recency {
    /// Every recency, in the enum's declared order: a test enumerates the domain from
    /// it, so a new variant cannot enter the enum without that test naming it.
    pub const ALL: [Recency; 4] = [
        Recency::VeryRecent,
        Recency::Recent,
        Recency::Stale,
        Recency::Never,
    ];

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Recency::VeryRecent => "very-recent",
            Recency::Recent => "recent",
            Recency::Stale => "stale",
            Recency::Never => "never",
        }
    }
}

/// The node's own answer to whether it is leading, and whether anyone is.
///
/// The role is a total function of [`Ownership`] and [`Recency`] alone: who owns the
/// ballot of the value this node last saw chosen, and how recently that value was chosen.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    /// This node owns the ballot of a value chosen within the leadership wait.
    Leader,
    /// This node owns the ballot of a value chosen within the recency wait but not the
    /// leadership wait.
    Incumbent,
    /// Another member owns the ballot of a value chosen recently.
    Follower,
    /// No value is known to have been chosen recently.
    Candidate,
}

impl Role {
    /// Every role, in the enum's declared order.
    pub const ALL: [Role; 4] = [
        Role::Leader,
        Role::Incumbent,
        Role::Follower,
        Role::Candidate,
    ];

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Role::Leader => "leader",
            Role::Incumbent => "incumbent",
            Role::Follower => "follower",
            Role::Candidate => "candidate",
        }
    }
}

/// The role for an ownership and a recency: total, closed, and with no default arm.
///
/// The rule it exhausts: a node that owns the ballot of a very recent value is the
/// [`Role::Leader`], one that owns the ballot of a merely recent value is the
/// [`Role::Incumbent`], one that knows another member owns the ballot of a recent value is
/// a [`Role::Follower`], and everything else is a [`Role::Candidate`].
///
/// The two rows that carry the argument are in the doc comment on [`Ownership::Unknown`]
/// (an unattributable ballot proves no seat, so the row answers Candidate) and here: a
/// node whose own value has gone [`Recency::Stale`] is a Candidate and not an Incumbent,
/// because an incumbent is defined by owning a *recent* choice and once the recency wait
/// has expired there is nothing to be incumbent over.
#[must_use]
pub fn role(ownership: Ownership, recency: Recency) -> Role {
    use Ownership::{Mine, Theirs, Unknown};
    use Recency::{Never, Recent, Stale, VeryRecent};
    use Role::{Candidate, Follower, Incumbent, Leader};
    match (ownership, recency) {
        (Mine, VeryRecent) => Leader,
        (Mine, Recent) => Incumbent,
        (Mine, Stale) | (Mine, Never) => Candidate,
        (Theirs(_), VeryRecent) | (Theirs(_), Recent) => Follower,
        (Theirs(_), Stale) | (Theirs(_), Never) => Candidate,
        // The whole unattributable row, named rather than folded into the arms above:
        // claiming leadership on a ballot this node cannot place would be claiming a seat
        // it cannot prove, and the solicitation path is how the uncertainty resolves.
        (Unknown, VeryRecent) | (Unknown, Recent) | (Unknown, Stale) | (Unknown, Never) => {
            Candidate
        }
    }
}

/// Which of the node's two waits expired.
///
/// The waits are ordered and the order is the whole of the distinction: the shorter
/// decides whether the node may call itself leader, the longer whether it still counts as
/// recently chosen (`docs/prevoting.md`, the two-waits section).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Wait {
    /// The shorter wait expired: the value this node owns is no longer very recent, so an
    /// incumbent rather than the leader.
    Leadership,
    /// The longer wait expired: the value is no longer recent, so a candidate.
    Recency,
}

impl Wait {
    /// Every wait, in the enum's declared order.
    pub const ALL: [Wait; 2] = [Wait::Leadership, Wait::Recency];

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Wait::Leadership => "leadership",
            Wait::Recency => "recency",
        }
    }
}

/// The recency one wait-expiry leaves behind: a rank moved down by the wait that expired,
/// and never moved up.
///
/// The three arms carry the ordering of the two waits. A [`Wait::Leadership`] expiry
/// spends the shorter wait, so it can only demote [`Recency::VeryRecent`] to
/// [`Recency::Recent`]; demoting a value that is not very recent would mean the shorter
/// wait was still outstanding, which the ordering forbids. A [`Wait::Recency`] expiry
/// demotes either freshness to [`Recency::Stale`], because by the time the longer wait
/// expires the shorter one is long gone. [`Recency::Never`] cannot demote, because nothing
/// was chosen and so no wait was ever outstanding — a clock event on it is not protocol.
#[must_use]
fn demote(recency: Recency, wait: Wait) -> Recency {
    use Recency::{Never, Recent, Stale, VeryRecent};
    use Wait::{Leadership, Recency as RecencyWait};
    match (recency, wait) {
        (VeryRecent, Leadership) => Recent,
        (VeryRecent, RecencyWait) | (Recent, RecencyWait) => Stale,
        (Never, Leadership) | (Never, RecencyWait) => Never,
        // Already spent: the shorter wait cannot demote a value it no longer covers, and
        // the longer cannot demote one that is already stale. Each name is stated so the
        // eight cells are visibly the whole domain.
        (Recent | Stale, Leadership) | (Stale, RecencyWait) => recency,
    }
}

/// What happened to the node.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Event {
    /// The node learned that a value was chosen, and the host resolved the ballot's owner
    /// and the rank of the choice against its own deadlines. The two are supplied rather
    /// than derived because attribution needs the configuration and recency needs the
    /// host's clock, neither of which this module reads.
    Chosen {
        /// Who owns the ballot the value was chosen in.
        owner: Ownership,
        /// How recently it was chosen.
        recency: Recency,
    },
    /// One of the node's two waits expired.
    WaitExpired(Wait),
}

impl Event {
    // No `ALL` here, unlike the payload-free enums beside it: this one's domain is the
    // product of an ownership and a recency, so a test enumerates the two separately and
    // crosses them, which is the idiom `tests/exhaustive_prepare.rs` uses for the same
    // reason.

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Event::Chosen { .. } => "chosen",
            Event::WaitExpired(_) => "wait-expired",
        }
    }
}

/// What an event obliges the node to do.
///
/// An action is a duty, not a message: the transport, the ballot to use and the payload
/// are the host's or the verifier's. What this module states is which of the four
/// obligations a given event creates, if any.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Nothing follows: the event named no role that owes traffic.
    Nothing,
    /// Propose an empty entry in the ballot this node already owns, which reinstates it as
    /// the leader without asking anyone. Cheaper than a candidate's solicitation, and the
    /// reason the incumbent exists as a state distinct from the candidate.
    ProposeNoOp,
    /// Ask the peers, before any phase 1, what they have promised.
    Solicit,
}

impl Action {
    /// Every action, in the enum's declared order.
    pub const ALL: [Action; 3] = [Action::Nothing, Action::ProposeNoOp, Action::Solicit];

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Action::Nothing => "nothing",
            Action::ProposeNoOp => "propose-no-op",
            Action::Solicit => "solicit",
        }
    }
}

/// The node's volatile pre-vote state, and nothing else.
///
/// Two facts and a floor: who owns the ballot of the value last seen chosen, how recently
/// it was chosen, and the greatest ballot this node has promised. The role is derived from
/// the first two and is never stored, so the two cannot disagree. The floor is not part of
/// the role — it is what this node reports in an offer — and the host is the only thing
/// that raises it, when it sends a promise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PreVote {
    owner: Ownership,
    recency: Recency,
    floor: Ballot,
}

impl PreVote {
    /// The state for an ownership, a recency and a promise floor.
    #[must_use]
    pub fn new(owner: Ownership, recency: Recency, floor: Ballot) -> Self {
        PreVote {
            owner,
            recency,
            floor,
        }
    }

    /// Who owns the ballot of the value last seen chosen.
    #[must_use]
    pub fn owner(self) -> Ownership {
        self.owner
    }

    /// How recently that value was chosen.
    #[must_use]
    pub fn recency(self) -> Recency {
        self.recency
    }

    /// The greatest ballot this node has promised, which is what it offers a candidate.
    #[must_use]
    pub fn floor(self) -> Ballot {
        self.floor
    }

    /// This node's role, derived rather than stored.
    #[must_use]
    pub fn role(self) -> Role {
        role(self.owner, self.recency)
    }

    /// The answer to an event: the next state and the action the event obliges, with no
    /// refusal arm because no event is unanswerable. Every pair of role and event names
    /// one action, so a new variant in either enum is a compile error here.
    #[must_use]
    pub fn step(self, event: Event) -> Step {
        match event {
            Event::Chosen { owner, recency } => {
                let next = PreVote {
                    owner,
                    recency,
                    floor: self.floor,
                };
                Step {
                    action: on_learning(next.role()),
                    next,
                }
            }
            Event::WaitExpired(wait) => {
                let next = PreVote {
                    owner: self.owner,
                    recency: demote(self.recency, wait),
                    floor: self.floor,
                };
                Step {
                    action: on_expiry(self.role(), wait),
                    next,
                }
            }
        }
    }
}

/// The answer to an event: the next volatile state, and what the node now owes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// What the node must now do.
    pub action: Action,
    /// The state it holds after the event.
    pub next: PreVote,
}

/// What a learned choice obliges, given the role the choice names.
///
/// Only the incumbent owes traffic, and it reinstates itself by proposing an empty entry
/// in the ballot it already owns. The leader owes nothing because the value it just saw
/// chosen is one it proposed itself; the follower owes nothing because it believes a
/// leader is active; the candidate owes nothing yet because it asks when its wait expires,
/// not when it learns.
#[must_use]
fn on_learning(role: Role) -> Action {
    use Action::{Nothing, ProposeNoOp};
    use Role::{Candidate, Follower, Incumbent, Leader};
    match role {
        Leader => Nothing,
        Incumbent => ProposeNoOp,
        Follower => Nothing,
        Candidate => Nothing,
    }
}

/// What an expired wait obliges, given the role the expiry leaves behind.
///
/// The rule: a leader whose shorter wait expires becomes an incumbent and reinstates
/// itself; anything that is no longer recent becomes a candidate and asks. A wait that
/// expires on a role it cannot move is not protocol for that role — a follower has no
/// leadership wait outstanding, and a candidate has nothing to reinstate — and the answer
/// is [`Action::Nothing`].
#[must_use]
fn on_expiry(before: Role, wait: Wait) -> Action {
    use Action::{Nothing, ProposeNoOp, Solicit};
    use Role::{Candidate, Follower, Incumbent, Leader};
    use Wait::{Leadership, Recency as RecencyWait};
    match (before, wait) {
        (Leader, Leadership) => ProposeNoOp,
        (Incumbent, Leadership) => Nothing,
        (Follower, Leadership) => Nothing,
        (Candidate, Leadership) => Nothing,
        // The longer wait: whatever the role was, the value is no longer recent, so the
        // node is a candidate and must ask. A leader reaching this arm without its
        // shorter wait having fired is a host that never armed it; asking is the safe
        // answer, and the recency it leaves behind is stale either way.
        (Leader | Incumbent | Follower | Candidate, RecencyWait) => Solicit,
    }
}
