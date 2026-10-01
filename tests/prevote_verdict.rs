//! Contract for the pre-vote verdict: what the collected replies authorise.
//!
//! Spec `docs/prevoting.md`, the dual-quorum rule and its *What is proved* negative controls,
//! keep both of its conjuncts load-bearing.
//!
//! The rule asks for a phase-1 quorum **and** a phase-2 quorum of the first unchosen
//! instance's configuration. Two tests here are the point of the file:
//!
//! 1. **Both conjuncts are load-bearing.** A strategy in which the two families differ —
//!    which is the case the rule exists for — admits an offering set that satisfies the
//!    phase-1 family alone and an offering set that satisfies the phase-2 family alone, and
//!    both are refused. Delete either conjunct from [`verdict`] and one of these fails.
//! 2. **Under the shipped strategy the two families coincide**, so the second conjunct is
//!    latent rather than redundant: on five unit-weight members [`WeightedMajority`] draws
//!    the same families for both roles, and any set that is a quorum for one is a quorum
//!    for the other. That is worth stating, because a reader who tested the rule only
//!    there would conclude the second conjunct is dead code. It is not: it is the conjunct
//!    that binds when weights differ, and the refusal cases above are where it earns its
//!    place.
//!
//! The quorum questions are asked of the strategy, so these tests supply strategies. That
//! is the house shape (`tests/quorum_contract.rs`'s threshold strategy), and it is the only
//! honest way to observe the rule: the shipped strategy cannot separate the families.

use uvrr::configuration::{Configuration, INIT_SLOT, SystemOperation, VOID_SLOT};
use uvrr::ids::{Ballot, Era, NodeId, Slot, View};
use uvrr::prevote::alphabet::Exchange;
use uvrr::prevote::verdict::{Reply, Verdict, verdict};
use uvrr::quorum::{QuorumStrategy, Role, WeightedMajority};

const N0: NodeId = NodeId(0);
const N1: NodeId = NodeId(1);
const N2: NodeId = NodeId(2);
const N3: NodeId = NodeId(3);
const N4: NodeId = NodeId(4);

/// Five unit-weight members, the smallest odd cluster with three distinct majorities.
const FIVE: [NodeId; 5] = [N0, N1, N2, N3, N4];

/// The candidate's own promise floor. Every offer a test offers legally is above it, and
/// every offer it offers illegally is not a legal successor of it.
const FLOOR: Ballot = Ballot {
    era: Era(0),
    view: View(1),
};

/// A ballot one view above the floor: always a legal successor.
fn above(view: u32) -> Ballot {
    Ballot {
        era: Era(0),
        view: View(view),
    }
}

/// A configuration of five unit-weight members, built the only way a configuration can
/// come to exist.
fn five() -> Configuration {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1 on the void configuration");
    void.apply(
        &SystemOperation::Init {
            order: FIVE.to_vec(),
        },
        INIT_SLOT,
    )
    .expect("Init at slot 2 immediately after Void")
}

/// A threshold strategy whose two families differ. This is the shape of every deliberate
/// violation in the crate's quorum tests, and here it is not a violation at all: it is
/// the legitimate case of a configuration whose phase-1 and phase-2 quorums are not the
/// same set, which is what the dual-quorum rule is written for.
struct Split {
    commit: u64,
    view_change: u64,
}

impl QuorumStrategy for Split {
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool {
        match config.weight_of_set(members) {
            Some(weight) => match role {
                Role::Commit => weight >= self.commit,
                Role::ViewChange => weight >= self.view_change,
                Role::Restart | Role::Fence => false,
            },
            None => false,
        }
    }

    fn threshold(&self, role: Role, _config: &Configuration) -> Option<u64> {
        match role {
            Role::Commit => Some(self.commit),
            Role::ViewChange => Some(self.view_change),
            Role::Restart | Role::Fence => None,
        }
    }
}

/// Offers of `promised` from each of `from`.
fn offers(from: &[NodeId], promised: Ballot) -> Vec<Reply> {
    from.iter()
        .map(|node| Reply::new(*node, Exchange::OfferVote { promised }))
        .collect()
}

#[test]
fn a_catch_up_offer_ends_the_round_whatever_else_arrived() {
    let config = five();
    let strategy = WeightedMajority;
    let replies = vec![
        Reply::new(N3, Exchange::OfferVote { promised: above(2) }),
        Reply::new(N1, Exchange::OfferCatchUp { frontier: Slot(9) }),
        Reply::new(N4, Exchange::OfferVote { promised: above(3) }),
    ];
    // The premise was stale, so the candidate learns instead of proposing — even though
    // the offers alongside would have satisfied both families.
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &replies),
        Verdict::CatchUp { from: N1 },
        "a catch-up outranks a completed offer set"
    );
}

#[test]
fn the_verdict_is_a_function_of_the_set_and_not_of_the_delivery_order() {
    let config = five();
    let strategy = WeightedMajority;
    let replies = vec![
        Reply::new(N4, Exchange::OfferCatchUp { frontier: Slot(9) }),
        Reply::new(N2, Exchange::OfferCatchUp { frontier: Slot(7) }),
    ];
    let mut reversed = replies.clone();
    reversed.reverse();
    // A core that answered differently for the same knowledge in a different order would
    // be a coin, not a core.
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &reversed),
        verdict(&strategy, &config, FLOOR, &replies),
        "the verdict must not depend on the order replies arrived in"
    );
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &replies),
        Verdict::CatchUp { from: N2 },
        "the lowest-numbered peer reporting a later value is chosen"
    );
}

#[test]
fn both_families_authorise_a_prepare_at_the_greatest_offer() {
    let config = five();
    let strategy = WeightedMajority;
    let replies = offers(&[N0, N1, N2, N4], above(5));
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &replies),
        Verdict::Prepare { ballot: above(5) },
        "three of five unit members is a quorum in both families"
    );
}

#[test]
fn the_prepare_ballot_is_the_greatest_offer_and_never_below_the_candidate() {
    let config = five();
    let strategy = WeightedMajority;
    let replies = offers(&[N0, N1, N2], above(9));
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &replies),
        Verdict::Prepare { ballot: above(9) },
        "the greatest offer is the ballot that clears every offering peer's floor"
    );
    // A candidate that has promised higher than anyone offers still prepares at its own
    // floor rather than below it.
    let high = above(20);
    assert_eq!(
        verdict(&strategy, &config, high, &offers(&[N0, N1, N2], above(9))),
        Verdict::Pending,
        "offers below the candidate's own floor are not legal successors of it"
    );
}

#[test]
fn an_illegal_offer_does_not_count_towards_the_quorum() {
    let config = five();
    let strategy = WeightedMajority;
    // An offer two eras on is not a legal successor of the candidate's floor, so the
    // candidate has no right to prepare at it. Two legal offers and two such offers: were
    // the illegal pair counted the set would be four of five and sufficient, so this case
    // distinguishes counting them from dropping them.
    let illegal = Exchange::OfferVote {
        promised: Ballot {
            era: Era(2),
            view: View(4),
        },
    };
    assert_eq!(
        verdict(
            &strategy,
            &config,
            FLOOR,
            &[
                Reply::new(N0, Exchange::OfferVote { promised: above(4) }),
                Reply::new(N1, Exchange::OfferVote { promised: above(4) }),
                Reply::new(N2, illegal),
                Reply::new(N3, illegal),
            ]
        ),
        Verdict::Pending,
        "two legal offers are not a quorum, and two unusable ones cannot make them one"
    );

    // The ballot is the second observation: an illegal offer is the greatest ballot in
    // the order, so if it were counted at all it would be the ballot chosen. It is not.
    assert_eq!(
        verdict(
            &strategy,
            &config,
            FLOOR,
            &[
                Reply::new(N0, Exchange::OfferVote { promised: above(4) }),
                Reply::new(N1, Exchange::OfferVote { promised: above(4) }),
                Reply::new(N2, Exchange::OfferVote { promised: above(4) }),
                Reply::new(N3, illegal),
            ]
        ),
        Verdict::Prepare { ballot: above(4) },
        "the ballot must be the greatest legal offer, never the greatest offered"
    );
}

#[test]
fn one_family_alone_is_refused_and_that_is_the_negative_control() {
    let config = five();
    // The phase-1 family is the looser of the two: three members clear it, two do not.
    let phase_one_only = Split {
        commit: 4,
        view_change: 3,
    };
    // Three offers clear the phase-1 family and not the phase-2 family. A candidate that
    // started a phase 1 on the strength of them would gather promises and then be unable
    // to finish what it started.
    assert_eq!(
        verdict(
            &phase_one_only,
            &config,
            FLOOR,
            &offers(&[N0, N1, N2], above(4))
        ),
        Verdict::Pending,
        "a phase-1 quorum alone cannot decide, and must be refused"
    );

    // The mirror: three offers clear the phase-2 family and not the phase-1 family. The
    // candidate would find a set that could accept but not one that had all promised, so
    // its ballot would not be demonstrably fresh for anyone.
    let phase_two_only = Split {
        commit: 3,
        view_change: 4,
    };
    assert_eq!(
        verdict(
            &phase_two_only,
            &config,
            FLOOR,
            &offers(&[N0, N1, N2], above(4))
        ),
        Verdict::Pending,
        "a phase-2 quorum alone cannot complete a phase 1, and must be refused"
    );

    // Four offers clear both in either arrangement, which is what shows the refusals above
    // are about the missing conjunct and not about the size of the set.
    assert!(matches!(
        verdict(
            &phase_one_only,
            &config,
            FLOOR,
            &offers(&[N0, N1, N2, N3], above(4))
        ),
        Verdict::Prepare { .. }
    ));
    assert!(matches!(
        verdict(
            &phase_two_only,
            &config,
            FLOOR,
            &offers(&[N0, N1, N2, N3], above(4))
        ),
        Verdict::Prepare { .. }
    ));
}

#[test]
fn the_shipped_strategy_draws_the_two_families_identically() {
    let config = five();
    let strategy = WeightedMajority;
    // On five unit-weight members the two families coincide, so the second conjunct of the
    // rule is latent here and every quorum of one is a quorum of the other. Stated as a
    // test because it is the fact a reader needs in order not to delete the conjunct.
    for size in 0..=5usize {
        let set: Vec<NodeId> = FIVE.iter().take(size).copied().collect();
        assert_eq!(
            strategy.is_quorum(Role::ViewChange, &config, &set),
            strategy.is_quorum(Role::Commit, &config, &set),
            "the families coincide on {set:?} and must be observed to"
        );
    }
    // Under a strategy that separates them they do not, which is the case the rule is for.
    let split = Split {
        commit: 4,
        view_change: 3,
    };
    let three = [N0, N1, N2];
    assert!(split.is_quorum(Role::ViewChange, &config, &three));
    assert!(!split.is_quorum(Role::Commit, &config, &three));
}

#[test]
fn one_peer_twice_is_one_peer() {
    let config = five();
    let strategy = WeightedMajority;
    // A host that reports the same peer twice has told us one peer's floor. A weight-based
    // strategy refuses a set with duplicates outright, so without the de-duplication this
    // would be a reporting quirk that stalls a round.
    let replies = vec![
        Reply::new(N0, Exchange::OfferVote { promised: above(4) }),
        Reply::new(N0, Exchange::OfferVote { promised: above(4) }),
        Reply::new(N1, Exchange::OfferVote { promised: above(4) }),
        Reply::new(N1, Exchange::OfferVote { promised: above(4) }),
    ];
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &replies),
        Verdict::Pending,
        "two peers reported twice are two peers, not a quorum of five"
    );
}

#[test]
fn nothing_offered_means_nothing_authorised() {
    let config = five();
    let strategy = WeightedMajority;
    assert_eq!(
        verdict(&strategy, &config, FLOOR, &[]),
        Verdict::Pending,
        "an empty reply set authorises nothing"
    );
}
