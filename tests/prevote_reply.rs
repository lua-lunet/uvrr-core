//! Contract for the pre-vote reply rule: which of the three answers a peer gives, and
//! which of them is silence.
//!
//! Spec `docs/prevoting.md`, the reply rule section.
//!
//! Silence is the answer that matters here. A node that still believes a leader is active
//! declines to encourage an election, and that is what stops a partitioned node rejoining
//! and displacing a healthy leader by acting on a stale local view. The rule is therefore
//! pinned on all four cells of the two facts that decide it — whether this node knows
//! something later than the solicitation advertised, and whether this node is a candidate —
//! because the pre-vote property is exactly the cell where a later knowledge does *not*
//! outrank the silence, and the catch-up case is the cell where it does.

use uvrr::ids::{Ballot, NodeId, Slot};
use uvrr::prevote::alphabet::{Exchange, reply};
use uvrr::prevote::{Ownership, PreVote, Recency};

/// A member that is not this node.
const PEER: NodeId = NodeId(7);

/// The promise floor this node reports in an offer.
const FLOOR: Ballot = Ballot {
    era: uvrr::ids::Era(0),
    view: uvrr::ids::View(4),
};

/// A state whose role is `Candidate`: no value known chosen recently.
fn candidate() -> PreVote {
    PreVote::new(Ownership::Mine, Recency::Stale, FLOOR)
}

/// A state whose role is `Follower`: another member owns the ballot of a recent value.
fn follower() -> PreVote {
    PreVote::new(Ownership::Theirs(PEER), Recency::Recent, FLOOR)
}

#[test]
fn a_peer_that_knows_something_later_offers_a_catch_up() {
    use Exchange::OfferCatchUp;
    // The candidate advertised slot 3; this node has committed slot 9.
    for me in [candidate(), follower()] {
        assert_eq!(
            reply(me, Slot(3), Slot(9)),
            Some(OfferCatchUp { frontier: Slot(9) }),
            "a later knowledge is said whatever this node believes about leadership"
        );
    }
}

#[test]
fn a_candidate_offers_its_promise_floor() {
    use Exchange::OfferVote;
    // Nothing later is known, and this node believes no leader is active, so it answers
    // with the floor the candidate must clear.
    assert_eq!(
        reply(candidate(), Slot(9), Slot(9)),
        Some(OfferVote { promised: FLOOR }),
        "equal frontiers are not later knowledge"
    );
}

#[test]
fn a_node_that_believes_a_leader_is_active_says_nothing() {
    // The pre-vote property. This node knows no more than the candidate does and is
    // perfectly willing to tell it so — it simply declines to encourage an election.
    assert_eq!(
        reply(follower(), Slot(3), Slot(3)),
        None,
        "a follower must not encourage an election"
    );
}

#[test]
fn the_four_cells_are_the_whole_domain() {
    // Both facts, crossed: the rule is total over them and there is no third answer.
    let cases = [
        (
            true,
            true,
            Some(Exchange::OfferCatchUp { frontier: Slot(9) }),
        ),
        (
            true,
            false,
            Some(Exchange::OfferCatchUp { frontier: Slot(9) }),
        ),
        (false, true, Some(Exchange::OfferVote { promised: FLOOR })),
        (false, false, None),
    ];
    assert_eq!(cases.len(), 4);
    for (later, is_candidate, claimed) in cases {
        let me = if is_candidate {
            candidate()
        } else {
            follower()
        };
        let committed = if later { Slot(9) } else { Slot(3) };
        assert_eq!(
            reply(me, Slot(3), committed),
            claimed,
            "later={later}, candidate={is_candidate}"
        );
    }
}

#[test]
fn every_role_but_the_candidate_holds_the_election() {
    use Ownership::Theirs;
    use uvrr::prevote::Role;
    // The silence is not the follower's alone: an unattributable ballot and a stale value
    // are candidates, and every role that is not a candidate withholds the vote. So the
    // rule tracks the role, not the ownership.
    for recency in uvrr::prevote::Recency::ALL {
        let mine = PreVote::new(Ownership::Mine, recency, FLOOR);
        let theirs = PreVote::new(Theirs(PEER), recency, FLOOR);
        for me in [mine, theirs] {
            let expected = if me.role() == Role::Candidate {
                Some(Exchange::OfferVote { promised: FLOOR })
            } else {
                None
            };
            assert_eq!(
                reply(me, Slot(3), Slot(3)),
                expected,
                "a {} must answer {}",
                me.role().name(),
                if expected.is_some() {
                    "with an offer"
                } else {
                    "silently"
                }
            );
        }
    }
}

#[test]
fn an_undisclosed_frontier_is_not_later_knowledge() {
    use Exchange::OfferCatchUp;
    // The comparison is strict: a peer that has committed exactly what was advertised has
    // nothing to add, and one slot is the whole difference between catching up and
    // answering with an offer.
    assert_eq!(
        reply(candidate(), Slot(7), Slot(8)),
        Some(OfferCatchUp { frontier: Slot(8) }),
        "one slot beyond the advertised frontier is later knowledge"
    );
    assert_eq!(
        reply(candidate(), Slot(8), Slot(8)),
        Some(Exchange::OfferVote { promised: FLOOR }),
        "exactly the advertised frontier is not later knowledge"
    );
    // A peer that has committed *less* than was advertised is behind, not ahead, and says
    // nothing about catching the sender up.
    assert_eq!(
        reply(candidate(), Slot(9), Slot(8)),
        Some(Exchange::OfferVote { promised: FLOOR }),
        "a frontier below the advertised one is not later knowledge"
    );
}
