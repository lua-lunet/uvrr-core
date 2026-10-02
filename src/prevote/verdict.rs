//! What the collected replies authorise: the dual-quorum rule.
//!
//! A candidate prepares only when the offers carry **both** quorum families of the
//! configuration for the first unchosen instance — a phase-1 quorum
//! ([`Role::ViewChange`]) and a phase-2 quorum ([`Role::Commit`]). Either alone is
//! insufficient, and the two are insufficient for different reasons, which is why the
//! rule asks for both rather than for the stronger of them (`docs/prevoting.md`, the verdict section).
//!
//! The quorum questions are asked of the [`QuorumStrategy`] and never counted here, the
//! same rule the view change follows: the strategy is the single point of truth for what
//! a quorum is.

use std::collections::BTreeSet;

use crate::configuration::Configuration;
use crate::ids::{Ballot, NodeId};
use crate::quorum::{QuorumStrategy, Role as QuorumRole};

use super::alphabet::Exchange;

/// A reply, with the peer it came from: an offer names a peer and the quorum question is
/// about peers, so the two are never separable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reply {
    /// The peer that sent it.
    pub from: NodeId,
    /// What it said.
    pub exchange: Exchange,
}

impl Reply {
    /// A reply from a peer.
    #[must_use]
    pub fn new(from: NodeId, exchange: Exchange) -> Self {
        Reply { from, exchange }
    }
}

/// What the collected replies authorise.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Verdict {
    /// The premise was stale: a peer knows a later value, so learn from it rather than
    /// propose.
    CatchUp {
        /// The peer to learn from.
        from: NodeId,
    },
    /// Both families are present: run phase 1 at the ballot the offers cleared.
    Prepare {
        /// The ballot to prepare at, no less than any offered and no less than the
        /// candidate's own floor.
        ballot: Ballot,
    },
    /// The offers are not yet sufficient: keep collecting.
    Pending,
}

impl Verdict {
    /// Every verdict, in the enum's declared order. [`Verdict::CatchUp`] is
    /// instantiated at the first node so the array is total.
    pub const ALL: [Verdict; 3] = [
        Verdict::CatchUp { from: NodeId(0) },
        Verdict::Prepare {
            ballot: Ballot::INITIAL,
        },
        Verdict::Pending,
    ];

    /// The name, for every surface a human reads. The payload is not in the name: which
    /// peer, or which ballot, is the host's to read off the value.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Verdict::CatchUp { .. } => "catch-up",
            Verdict::Prepare { .. } => "prepare",
            Verdict::Pending => "pending",
        }
    }
}

/// What the replies authorise, given the candidate's own floor.
///
/// Three rules, in this order:
///
/// 1. Any catch-up offer ends the round: the candidate's premise was stale and it learns
///    instead of proposing. When several peers report something later, the lowest node id
///    is chosen so the verdict is a function of the *set* of replies and not of the order
///    the host happened to deliver them in.
/// 2. An offer the candidate may not act on does not count. A peer offering a ballot that
///    is not a legal successor of the candidate's own floor is offering something the
///    candidate has no right to prepare at, and [`Ballot::is_legal_successor`] is the one
///    place that rule is stated. Such an offer is dropped rather than counted, so a
///    round carrying one reports [`Verdict::Pending`] rather than preparing at a ballot it
///    cannot justify.
/// 3. Otherwise the offering peers must contain both a phase-1 and a phase-2 quorum of
///    `config`. Only then is the ballot to prepare at the greatest ballot offered, which
///    the offering quorum has by definition promised nothing above, and which is therefore
///    fresh for every member of that quorum.
///
/// # Arguments
///
/// - `floor` is the candidate's own promise floor, the lower bound on what it may
///   prepare at.
/// - `config` is the configuration corresponding to the first unchosen instance, which is
///   the configuration both families are counted in.
#[must_use]
pub fn verdict(
    strategy: &dyn QuorumStrategy,
    config: &Configuration,
    floor: Ballot,
    replies: &[Reply],
) -> Verdict {
    use Verdict::{CatchUp, Pending, Prepare};

    if let Some(from) = earliest_catch_up(replies) {
        return CatchUp { from };
    }

    // A set, not a list: a host that reports the same peer twice has still told us one
    // peer's floor, and a weight-based strategy refuses a set with duplicates outright,
    // which would turn a reporting quirk into a stalled round.
    let mut peers: BTreeSet<NodeId> = BTreeSet::new();
    let mut greatest = floor;
    for reply in replies {
        if let Exchange::OfferVote { promised } = reply.exchange
            && floor.is_legal_successor(promised)
        {
            peers.insert(reply.from);
            if promised > greatest {
                greatest = promised;
            }
        }
    }

    let offering: Vec<NodeId> = peers.into_iter().collect();
    let phase_one = strategy.is_quorum(QuorumRole::ViewChange, config, &offering);
    let phase_two = strategy.is_quorum(QuorumRole::Commit, config, &offering);
    if phase_one && phase_two {
        Prepare { ballot: greatest }
    } else {
        Pending
    }
}

/// The lowest-numbered peer reporting a later value, if any.
///
/// The lowest rather than the first so two candidates holding the same set of replies
/// reach the same verdict: a core that returns different answers for the same knowledge
/// in a different delivery order is not a core, it is a coin.
fn earliest_catch_up(replies: &[Reply]) -> Option<NodeId> {
    replies
        .iter()
        .filter_map(|reply| match reply.exchange {
            Exchange::OfferCatchUp { .. } => Some(reply.from),
            Exchange::Solicit { .. } | Exchange::OfferVote { .. } => None,
        })
        .min()
}
