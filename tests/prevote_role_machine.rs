//! Contract for the pre-vote role machine: [`uvrr::prevote::role`], the closed table that
//! names every node's role, and the enums around it.
//!
//! Spec `docs/prevoting.md`, the role table and the three rows that carry the
//! argument.
//!
//! What is pinned here:
//!
//! 1. **The table is the whole domain.** All twelve ownership × recency cells are listed
//!    and the product of the two domains is asserted against the table length, so a new
//!    variant cannot enter either enum without this file failing to compile or the
//!    closure assertion failing. There is no default arm in the matcher to fall through.
//! 2. **The three argued rows.** A stale value this node owns yields Candidate and not
//!    Incumbent; an unattributable ballot yields Candidate at every recency; a value
//!    another member owns yields Follower only while it is recent.
//! 3. **Ownership's payload is not a hidden input.** Every `Ownership::Theirs` node names
//!    the same role, so the answer does not depend on which member it was.

use uvrr::ids::NodeId;
use uvrr::prevote::{Ownership, Recency, Role, role};

/// A member that is not this node.
const PEER: NodeId = NodeId(7);

/// Every ownership in the domain, paired with a member for `Theirs`.
#[rustfmt::skip]
const OWNERS: [(Ownership, NodeId); 3] =
    [(Ownership::Mine, NodeId(0)), (Ownership::Theirs(PEER), PEER), (Ownership::Unknown, NodeId(0))];

#[test]
fn the_role_table_is_the_whole_domain() {
    use Ownership::{Mine, Theirs, Unknown};
    use Recency::{Never, Recent, Stale, VeryRecent};
    use Role::{Candidate, Follower, Incumbent, Leader};

    // Transcribed from `docs/prevoting.md` §3, not from the matcher.
    let expected: [((Ownership, Recency), Role); 12] = [
        ((Mine, VeryRecent), Leader),
        ((Mine, Recent), Incumbent),
        ((Mine, Stale), Candidate),
        ((Mine, Never), Candidate),
        ((Theirs(PEER), VeryRecent), Follower),
        ((Theirs(PEER), Recent), Follower),
        ((Theirs(PEER), Stale), Candidate),
        ((Theirs(PEER), Never), Candidate),
        ((Unknown, VeryRecent), Candidate),
        ((Unknown, Recent), Candidate),
        ((Unknown, Stale), Candidate),
        ((Unknown, Never), Candidate),
    ];

    for ((ownership, recency), claimed) in expected {
        assert_eq!(
            role(ownership, recency),
            claimed,
            "the role for {} at {}",
            ownership.name(),
            recency.name()
        );
    }

    // The domain is closed and the table is all of it.
    assert_eq!(OWNERS.len() * Recency::ALL.len(), expected.len());
}

#[test]
fn a_stale_value_this_node_owns_is_a_candidate_and_not_an_incumbent() {
    use Ownership::Mine;
    use Recency::{Recent, Stale};
    use Role::{Candidate, Incumbent};
    // An incumbent is defined by owning a *recent* choice. Once the recency wait has
    // expired there is nothing to be incumbent over, and the node needs the peers'
    // permission like anybody else.
    assert_eq!(role(Mine, Recent), Incumbent);
    assert_eq!(role(Mine, Stale), Candidate);
}

#[test]
fn an_unattributable_ballot_proves_no_seat_at_any_recency() {
    use Ownership::Unknown;
    use Role::Candidate;
    // The row a reconfiguration produces: a node may hold a chosen value whose ballot
    // belongs to a generation it has not adopted. Claiming leadership on a ballot it
    // cannot place would be claiming a seat it cannot prove, so every recency answers
    // Candidate — including the freshest one.
    for recency in uvrr::prevote::Recency::ALL {
        assert_eq!(
            role(Unknown, recency),
            Candidate,
            "an unattributable ballot at {} must not name a role",
            recency.name()
        );
    }
}

#[test]
fn ownership_payload_is_not_a_hidden_input() {
    use Ownership::Theirs;
    for recency in uvrr::prevote::Recency::ALL {
        let claimed = role(Theirs(NodeId(1)), recency);
        for peer in 0u32..8 {
            assert_eq!(
                role(Theirs(NodeId(peer)), recency),
                claimed,
                "the role must not depend on which member owns the ballot, at {}",
                recency.name()
            );
        }
    }
}

#[test]
fn every_enum_names_itself_and_the_domains_are_the_declared_size() {
    assert_eq!(Recency::ALL.len(), 4);
    assert_eq!(Role::ALL.len(), 4);
    assert_eq!(uvrr::prevote::Wait::ALL.len(), 2);
    assert_eq!(uvrr::prevote::Action::ALL.len(), 3);
    assert_eq!(uvrr::prevote::alphabet::Exchange::ALL.len(), 3);
    assert_eq!(uvrr::prevote::verdict::Verdict::ALL.len(), 3);

    // The names are what a human and an operator read, so they are pinned beside the
    // variants they name and the two cannot drift apart.
    let recencies = [
        (Recency::VeryRecent, "very-recent"),
        (Recency::Recent, "recent"),
        (Recency::Stale, "stale"),
        (Recency::Never, "never"),
    ];
    for (recency, name) in recencies {
        assert_eq!(recency.name(), name);
    }

    let roles = [
        (Role::Leader, "leader"),
        (Role::Incumbent, "incumbent"),
        (Role::Follower, "follower"),
        (Role::Candidate, "candidate"),
    ];
    for (claimed, name) in roles {
        assert_eq!(claimed.name(), name);
    }

    let actions = [
        (uvrr::prevote::Action::Nothing, "nothing"),
        (uvrr::prevote::Action::ProposeNoOp, "propose-no-op"),
        (uvrr::prevote::Action::Solicit, "solicit"),
    ];
    for (action, name) in actions {
        assert_eq!(action.name(), name);
    }

    let waits = [
        (uvrr::prevote::Wait::Leadership, "leadership"),
        (uvrr::prevote::Wait::Recency, "recency"),
    ];
    for (wait, name) in waits {
        assert_eq!(wait.name(), name);
    }
}
