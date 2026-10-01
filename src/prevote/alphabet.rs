//! The pre-vote alphabet and the reply rule.
//!
//! Three messages, and the rule that decides which of them a peer sends. The rule is the
//! mechanism: silence is an answer, and it is the answer of a cluster that still believes
//! its leader is alive, which is what stops a partitioned node rejoining and displacing a
//! healthy leader by acting on a stale local view (`docs/prevoting.md`, the reply rule section).

use crate::ids::{Ballot, Slot};

use super::{PreVote, Role};

/// A message of the pre-vote handshake.
///
/// Not on the wire: there is no [`crate::wire::Tag`] and no [`crate::message::Body`] for
/// any of these, and binding them is a separate change with its own obligations.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exchange {
    /// A candidate's request, broadcast to the members it knows. It carries the sender's
    /// committed frontier so a peer can tell whether it knows something later, and the
    /// sender's own ballot so a peer can tell whether the candidate is behind.
    ///
    /// The ballot is not decoration. A member that receives a solicitation carrying a
    /// later ballot than its own must perform another phase 1 at a later ballot still,
    /// rather than ignore it; without the ballot in the message a minority that has run
    /// ahead sits in a later view, can neither accept the leader's proposals nor gather
    /// enough offers to elect anyone, and is stuck being caught up while visibly
    /// signalling candidacy. The system stays correct and the operator's signal stops
    /// meaning what it should (`docs/prevoting.md`, the alphabet section).
    Solicit {
        /// The sender's committed frontier, the premise a peer checks its own against.
        frontier: Slot,
        /// The sender's own ballot.
        ballot: Ballot,
    },
    /// A peer's answer naming the greatest ballot for which it has already promised: the
    /// floor a candidate must clear.
    OfferVote {
        /// The greatest ballot this peer has promised.
        promised: Ballot,
    },
    /// A peer's answer naming a later chosen value it knows of. The candidate learns from
    /// that peer instead of proposing.
    OfferCatchUp {
        /// The later frontier this peer knows chosen.
        frontier: Slot,
    },
}

impl Exchange {
    /// Every message, in the enum's declared order: a test enumerates the domain from it,
    /// so a new variant cannot enter the alphabet without that test naming it.
    pub const ALL: [Exchange; 3] = [
        Exchange::Solicit {
            frontier: Slot(0),
            ballot: Ballot::INITIAL,
        },
        Exchange::OfferVote {
            promised: Ballot::INITIAL,
        },
        Exchange::OfferCatchUp { frontier: Slot(0) },
    ];

    /// The name, for every surface a human reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Exchange::Solicit { .. } => "solicit",
            Exchange::OfferVote { .. } => "offer-vote",
            Exchange::OfferCatchUp { .. } => "offer-catch-up",
        }
    }
}

/// The answer this node gives a solicitation, or [`None`] for silence.
///
/// The rule it exhausts, over the two facts that decide it: a peer that knows a value
/// chosen later than the one advertised says so; otherwise a peer that still believes a
/// leader is active says nothing at all; otherwise it offers its promise floor.
///
/// The first case deliberately outranks the second. A peer with something later to say
/// says it even when it also believes a leader is active, because the premise of the
/// solicitation is stale and the answer to a stale premise is the truth, not a vote.
///
/// # Arguments
///
/// - `advertised` is the frontier the solicitation carried.
/// - `committed` is this node's own committed frontier.
///
/// A silence is a refusal to encourage an election, not an absence of knowledge: this
/// node may know perfectly well what the candidate wants to hear and withhold it because
/// it does not believe a new ballot is warranted.
#[must_use]
pub fn reply(me: PreVote, advertised: Slot, committed: Slot) -> Option<Exchange> {
    use Exchange::{OfferCatchUp, OfferVote};
    match (committed > advertised, me.role() == Role::Candidate) {
        (true, _) => Some(OfferCatchUp {
            frontier: committed,
        }),
        (false, true) => Some(OfferVote {
            promised: me.floor(),
        }),
        (false, false) => None,
    }
}
