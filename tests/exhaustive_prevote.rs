//! Exhaustive coverage of the pre-vote step function: every ownership, every recency and
//! every wait, with the action and the resulting recency pinned pair by pair.
//!
//! Spec `docs/prevoting.md`, the two waits and the actions they oblige.
//!
//! The expectations below are transcribed from the document, not from the step function.
//! The step function's own `demote` and `on_expiry` are private, so a test could not
//! restate them by calling in; it has to say what the table says. That is the property
//! that makes this file worth having: a matcher that answered `Nothing` everywhere would
//! pass a test written against the matcher and fail this one.

use uvrr::ids::{Ballot, NodeId};
use uvrr::prevote::{Action, Ownership, PreVote, Recency, Role, Wait};

/// A member that is not this node.
const PEER: NodeId = NodeId(7);

/// The promise floor every case starts from. The floor is inert here: a wait expiry
/// never raises it, only the host raises it when it sends a promise.
const FLOOR: Ballot = Ballot::INITIAL;

#[derive(Clone, Copy)]
#[rustfmt::skip]
enum Owner { Mine, Theirs, Unknown }

#[derive(Clone, Copy)]
#[rustfmt::skip]
enum Rec { VeryRecent, Recent, Stale, Never }

#[derive(Clone, Copy)]
#[rustfmt::skip]
enum Wait_ { Leadership, Recency }

#[derive(Clone, Copy)]
#[rustfmt::skip]
enum Act { Nothing, ProposeNoOp, Solicit }

const OWNER_ALL: [Owner; 3] = [Owner::Mine, Owner::Theirs, Owner::Unknown];
const REC_ALL: [Rec; 4] = [Rec::VeryRecent, Rec::Recent, Rec::Stale, Rec::Never];
const WAIT_ALL: [Wait_; 2] = [Wait_::Leadership, Wait_::Recency];

fn owner_of(owner: Owner) -> Ownership {
    match owner {
        Owner::Mine => Ownership::Mine,
        Owner::Theirs => Ownership::Theirs(PEER),
        Owner::Unknown => Ownership::Unknown,
    }
}

fn recency_of(rec: Rec) -> Recency {
    match rec {
        Rec::VeryRecent => Recency::VeryRecent,
        Rec::Recent => Recency::Recent,
        Rec::Stale => Recency::Stale,
        Rec::Never => Recency::Never,
    }
}

fn wait_of(wait: Wait_) -> Wait {
    match wait {
        Wait_::Leadership => Wait::Leadership,
        Wait_::Recency => Wait::Recency,
    }
}

/// The role the document's table names, restated independently of [`role`].
fn expected_role(owner: Owner, rec: Rec) -> Role {
    use Owner::{Mine, Theirs, Unknown};
    use Rec::{Never, Recent, Stale, VeryRecent};
    use Role::{Candidate, Follower, Incumbent, Leader};
    match (owner, rec) {
        (Mine, VeryRecent) => Leader,
        (Mine, Recent) => Incumbent,
        (Mine, Stale) | (Mine, Never) => Candidate,
        (Theirs, VeryRecent) | (Theirs, Recent) => Follower,
        (Theirs, Stale) | (Theirs, Never) => Candidate,
        (Unknown, VeryRecent) | (Unknown, Recent) | (Unknown, Stale) | (Unknown, Never) => {
            Candidate
        }
    }
}

/// The recency one wait-expiry leaves behind, restated from `docs/prevoting.md` §2.
///
/// The shorter wait spends only freshness; the longer demotes either freshness to stale
/// and leaves a stale value and a never-chosen one alone.
fn expected_recency(rec: Rec, wait: Wait_) -> Rec {
    match (rec, wait) {
        (Rec::VeryRecent, Wait_::Leadership) => Rec::Recent,
        (Rec::VeryRecent, Wait_::Recency) => Rec::Stale,
        (Rec::Recent, Wait_::Recency) => Rec::Stale,
        (Rec::Stale, Wait_::Recency) => Rec::Stale,
        (Rec::Never, Wait_::Leadership) => Rec::Never,
        (Rec::Never, Wait_::Recency) => Rec::Never,
        (Rec::Recent, Wait_::Leadership) => Rec::Recent,
        (Rec::Stale, Wait_::Leadership) => Rec::Stale,
    }
}

/// The action a wait-expiry obliges, restated from the role table and from
/// *The cost of being wrong* of `docs/prevoting.md`.
///
/// A leader whose shorter wait expires reinstates itself with an empty entry. Anything
/// whose value is no longer recent is a candidate and asks. A shorter wait expiring on a
/// role it cannot move is not protocol for that role: a follower has no leadership wait
/// outstanding and a candidate has nothing to reinstate.
fn expected_action(before: Role, wait: Wait_) -> Act {
    use Act::{Nothing, ProposeNoOp, Solicit};
    use Role::Leader;
    use Wait_::{Leadership, Recency};
    match (before, wait) {
        (Leader, Leadership) => ProposeNoOp,
        (_, Recency) => Solicit,
        (_, Leadership) => Nothing,
    }
}

fn act_of(act: Act) -> Action {
    match act {
        Act::Nothing => Action::Nothing,
        Act::ProposeNoOp => Action::ProposeNoOp,
        Act::Solicit => Action::Solicit,
    }
}

fn run_case(seq: usize, owner: Owner, rec: Rec, wait: Wait_) {
    let before = expected_role(owner, rec);
    let state = PreVote::new(owner_of(owner), recency_of(rec), FLOOR);
    assert_eq!(
        state.role(),
        before,
        "case {seq}: the stored role must be the table's role for {} at {}",
        match owner {
            Owner::Mine => "mine",
            Owner::Theirs => "theirs",
            Owner::Unknown => "unknown",
        },
        match rec {
            Rec::VeryRecent => "very-recent",
            Rec::Recent => "recent",
            Rec::Stale => "stale",
            Rec::Never => "never",
        }
    );

    let step = state.step(uvrr::prevote::Event::WaitExpired(wait_of(wait)));

    assert_eq!(
        step.action,
        act_of(expected_action(before, wait)),
        "case {seq}: a {} wait expiring on {}",
        match wait {
            Wait_::Leadership => "leadership",
            Wait_::Recency => "recency",
        },
        before.name()
    );
    assert_eq!(
        step.next.recency(),
        recency_of(expected_recency(rec, wait)),
        "case {seq}: the recency a {} wait leaves behind",
        match wait {
            Wait_::Leadership => "leadership",
            Wait_::Recency => "recency",
        }
    );
    // The next role is the table's role for the next knowledge, not a stored opinion:
    // ownership never changes on a wait expiry, so the next role is `expected_role` at
    // the demoted recency.
    assert_eq!(
        step.next.role(),
        expected_role(owner, expected_recency(rec, wait)),
        "case {seq}: the role the demoted recency names"
    );
    // The floor is inert: only the host raises it, by promising.
    assert_eq!(
        step.next.floor(),
        FLOOR,
        "case {seq}: the floor is unchanged"
    );
    assert_eq!(
        step.next.owner(),
        owner_of(owner),
        "case {seq}: a wait expiry learns nothing about ownership"
    );
}

#[test]
fn exhaustive_wait_expiry_over_every_ownership_recency_and_wait() {
    let mut seq = 1;
    for owner in OWNER_ALL {
        for rec in REC_ALL {
            for wait in WAIT_ALL {
                run_case(seq, owner, rec, wait);
                seq += 1;
            }
        }
    }
    assert_eq!(seq - 1, 3 * 4 * 2, "the domain is 3 x 4 x 2");
}
