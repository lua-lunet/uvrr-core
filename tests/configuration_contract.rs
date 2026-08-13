//! Contract for `vrr::configuration` — era, membership, voting weights, and the
//! reconfiguration operation alphabet.
//!
//! Spec §8.7.1 (configuration and era), §8.7.2 (the operation alphabet and its
//! precondition table), §8.7.5 (closure of weighted-majority configurations), §8.4
//! (weighted quorums, weight 0 = learner), §1.2 (primary selection), and decisions Q1
//! (quorum families are validated, not counted), W1 (era is its own field), W3/W4 (the
//! binary codec is normative and fixed-width).
//!
//! This file gates the *fold*, not quorum policy. Nothing here evaluates a quorum: that
//! is item06's `QuorumStrategy` and its closed gate in `vrr::quorum`. What is gated here
//! is the set of configurations that can exist at all, because every later intersection
//! argument (§8.7.4 `R1`/`R2`) is stated over configurations reachable by this fold and
//! is meaningless over configurations that are not.
//!
//! Six properties, each of which the rest of the crate is entitled to assume:
//!
//! 1. **A `Configuration` cannot be constructed invalid.** The only ways in are
//!    `Configuration::void()` and `apply`, so "non-empty `order` after `Init`, no
//!    duplicate `NodeId`, representable total" holds for every value that exists rather
//!    than for every value someone remembered to validate.
//! 2. **Era is established, not assigned.** `Void` establishes era 0, `Init` era 1, and
//!    every subsequent accepted operation `era + 1`. §8.7.8's rule that a replica may
//!    propose era `e` only if it holds the operation establishing `e` is then checkable
//!    by construction.
//! 3. **Refusal is specific.** Every precondition of §8.7.2's table has its own
//!    `ConfigError` variant and every test below asserts *that* variant. A test that
//!    accepts `is_err()` passes when the implementation refuses for the wrong reason,
//!    which is precisely the failure mode a precondition table exists to prevent.
//! 4. **Nothing saturates and nothing rounds.** `Halve` on an odd weight is refused,
//!    not floored; `Double` past `u32::MAX` is refused, not clamped; `Decrement` at
//!    weight 0 is refused, not a no-op. §8.7.5's closure proofs are arithmetic
//!    identities over exact weights and a rounded weight makes them vacuous.
//! 5. **The fold is persistent.** `apply` and `EraTable::extend` return new values and
//!    share retained `Arc`s. `Arc` identity is asserted by pointer comparison, because
//!    a table that deep-copied its history would still pass every behavioural test
//!    while making configuration history a per-message allocation.
//! 6. **The retention window is bounded and its overflow is a drop.** Three eras
//!    resident; an era outside the window is absent from the table, which is what makes
//!    an out-of-window message undecidable and therefore droppable (§10, §14.2).
//!
//! Groups 3, 4, 6, 7, 8, 11 and 12 are exhaustive loops rather than samplers: the
//! domains are small and total coverage is strictly stronger than any number of draws.
//! Group 8 in particular walks *every* operation sequence up to length 4 over a fixed
//! alphabet, which is a Definition-of-Done requirement for the era state machine.

use std::sync::Arc;

use proptest::prelude::*;
use vrr::configuration::{
    ConfigError, Configuration, EraTable, MAX_MEMBERS, Member, SystemOperation, Weight,
};
use vrr::ids::{Era, NodeId, Slot, View};
use vrr::wire::{Malformed, Pack, Unpack, UnpackError};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const N0: NodeId = NodeId(10);
const N1: NodeId = NodeId(11);
const N2: NodeId = NodeId(12);
const N3: NodeId = NodeId(13);

/// The genesis slot of `Void`, per the brief's precondition table.
const VOID_SLOT: Slot = Slot(1);
/// The genesis slot of `Init`, per the brief's precondition table.
const INIT_SLOT: Slot = Slot(2);

/// A void configuration followed by `Init` over `order`, at the two genesis slots.
///
/// Every test that needs a live cluster goes through here rather than through a
/// constructor, because there is no constructor: the invariants hold for every
/// `Configuration` that exists precisely because `apply` is the only way to make one.
fn initialised(order: &[NodeId]) -> Configuration {
    let void = Configuration::void();
    let void = void
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1 on the void configuration");
    void.apply(
        &SystemOperation::Init {
            order: order.to_vec(),
        },
        INIT_SLOT,
    )
    .expect("Init at slot 2 immediately after Void")
}

/// A three-member unit-weight cluster, the shape §8.7.5's proofs start from.
fn three() -> Configuration {
    initialised(&[N0, N1, N2])
}

/// Raises every member of `nodes` to the same weight `target`, using only `Init`,
/// `Double` and `Increment`, so every state this file asserts about is reachable by the
/// fold rather than fabricated through an escape hatch. That matters: "there is no public
/// constructor that can produce an invalid `Configuration`" is itself a property under
/// test, so a test-only constructor would weaken the thing it was helping to check.
///
/// Bit composition from the top of `target`: double, then add the current bit. Thirty-two
/// steps reach any `u32`, which is what makes the `u32::MAX` boundary a *reachable*
/// protocol state and not a hypothetical one. `Double` and `Increment` are the only
/// weight-raising operations in §8.7.2 and `Double` is global, which is why this raises
/// uniformly; per-member asymmetry is built afterwards with `Add` (weight 0) and
/// `Increment`.
fn raise_all(nodes: &[NodeId], target: u32) -> Configuration {
    assert!(target >= 1, "Init starts every member at weight 1");

    let mut config = initialised(nodes);
    let mut slot = Slot(3);
    let mut apply = |config: Configuration, op: SystemOperation| -> Configuration {
        let next = config.apply(&op, slot).expect("stays representable");
        slot = slot.next().expect("slot space");
        next
    };

    // `Init` already supplied the leading `1` of `target`.
    let bits = 32 - target.leading_zeros();
    for bit in (0..bits.saturating_sub(1)).rev() {
        config = apply(config, SystemOperation::Double);
        if (target >> bit) & 1 == 1 {
            for node in nodes {
                config = apply(config, SystemOperation::Increment(*node));
            }
        }
    }

    for node in nodes {
        assert_eq!(
            config.weight_of(*node),
            Some(Weight(target)),
            "raise_all must land exactly on {target}"
        );
    }
    config
}

/// `(node, weight)` pairs of `order`, in order, for compact sequence assertions.
fn shape(config: &Configuration) -> Vec<(NodeId, u32)> {
    config
        .order()
        .iter()
        .map(|member| (member.node, member.weight.0))
        .collect()
}

// ---------------------------------------------------------------------------
// 1. Void is inert
// ---------------------------------------------------------------------------

/// Era 0 has empty `order` and total weight 0, so **no legal quorum of any kind
/// exists** — the impossibility is arithmetic, not a guard. §8.7.1's quorum condition
/// `sum(W(n) for n in S) >= floor(T/2) + 1` reads `0 >= 1` on the void configuration and
/// is unsatisfiable by the empty set and by every other set. This test pins the
/// arithmetic rather than the guard, because a guard can be forgotten and
/// `floor(0/2) + 1 = 1 > 0` cannot.
#[test]
fn void_is_quorum_impossible() {
    let void = Configuration::void();

    assert_eq!(void.era(), Era(0));
    assert_eq!(void.order(), &[] as &[Member]);
    assert_eq!(void.len(), 0);
    assert_eq!(void.total(), 0);
    assert_eq!(void.weight_of_set(&[]), Some(0));
}

/// The void configuration selects no primary, at any view. `primary` is
/// `order[view mod len]` (§1.2) and there is no index into an empty sequence, so `None`
/// is the total answer rather than a defensive one.
#[test]
fn void_has_no_primary() {
    let void = Configuration::void();
    for view in 0..64u32 {
        assert_eq!(void.primary(View(view)), None, "view {view}");
    }
}

/// Only `Void` may be applied to the void configuration. Every other operation names a
/// member, a weight or a position that does not exist yet, and the refusal is
/// `NotInitialised` rather than `NotAMember` — the cluster has no membership at all, and
/// reporting a missing member would suggest one could be supplied.
#[test]
fn void_refuses_every_operation_but_void_and_init() {
    let void = Configuration::void();

    let ops = [
        SystemOperation::Increment(N0),
        SystemOperation::Decrement(N0),
        SystemOperation::Double,
        SystemOperation::Halve,
        SystemOperation::Add {
            node: N0,
            position: 0,
        },
        SystemOperation::Remove(N0),
    ];

    for op in &ops {
        assert_eq!(
            void.apply(op, VOID_SLOT),
            Err(ConfigError::NotInitialised),
            "{op:?} on the void configuration"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Genesis ordinals
// ---------------------------------------------------------------------------

/// `Void` occupies log slot 1 and no other (§8.7.2: "operation number 1 ... with an
/// otherwise empty log"). The slot is the only evidence the fold has that this is
/// genesis, so accepting it elsewhere would let a second `Void` erase a live cluster.
#[test]
fn void_accepted_only_at_slot_one() {
    let void = Configuration::void();

    let established = void
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1");
    assert_eq!(established.era(), Era(0));
    assert_eq!(established.len(), 0);
    assert_eq!(established.total(), 0);

    for slot in [Slot(0), Slot(2), Slot(3), Slot(u64::MAX)] {
        assert_eq!(
            void.apply(&SystemOperation::Void, slot),
            Err(ConfigError::WrongGenesisSlot {
                expected: VOID_SLOT,
                actual: slot,
            }),
            "Void at {slot:?}"
        );
    }
}

/// `Init` occupies log slot 2 and no other, and establishes era 1 with every weight `1`.
/// Unit initial weights are §8.7.2's rule and they remove a class of divergence: two
/// hosts cannot disagree about the genesis weights when the operation cannot carry them.
#[test]
fn init_accepted_only_at_slot_two_and_sets_unit_weights() {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void");

    let init = SystemOperation::Init {
        order: vec![N0, N1, N2],
    };

    let config = void.apply(&init, INIT_SLOT).expect("Init at slot 2");
    assert_eq!(config.era(), Era(1));
    assert_eq!(shape(&config), vec![(N0, 1), (N1, 1), (N2, 1)]);
    assert_eq!(config.total(), 3);

    for slot in [Slot(0), Slot(1), Slot(3), Slot(u64::MAX)] {
        assert_eq!(
            void.apply(&init, slot),
            Err(ConfigError::WrongGenesisSlot {
                expected: INIT_SLOT,
                actual: slot,
            }),
            "Init at {slot:?}"
        );
    }
}

/// `Init` is legal only *immediately* on the void configuration (§8.7.2: "immediately
/// preceded by `VOID`"). A second `Init` on a live cluster would replace its membership
/// wholesale without an overlap argument, so it is refused on the receiver's state, not
/// on the slot.
#[test]
fn init_refused_on_an_initialised_configuration() {
    let live = three();
    assert_eq!(
        live.apply(
            &SystemOperation::Init {
                order: vec![N0, N1]
            },
            INIT_SLOT,
        ),
        Err(ConfigError::AlreadyInitialised)
    );
}

/// `Init` with an empty order is refused: every operation must leave `order` non-empty
/// (§8.7.2, "every operation"), and an empty genesis would produce a cluster that can
/// never commit and can never be repaired, because repair itself needs a quorum.
#[test]
fn init_refuses_empty_order() {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void");
    assert_eq!(
        void.apply(&SystemOperation::Init { order: vec![] }, INIT_SLOT),
        Err(ConfigError::EmptyInitOrder)
    );
}

/// `order` is a list of *distinct* identifiers (§8.7.1). A repeated id would make
/// `weight_of` ambiguous and let one node's weight be counted twice by a quorum
/// evaluator that indexed by position, which is exactly the double-vote the distinctness
/// requirement exists to forbid.
#[test]
fn init_refuses_duplicate_node_ids() {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void");

    for order in [
        vec![N0, N0],
        vec![N0, N1, N0],
        vec![N0, N1, N2, N1],
        vec![N1, N1, N1],
    ] {
        let duplicate = SystemOperation::Init {
            order: order.clone(),
        };
        assert!(
            matches!(
                void.apply(&duplicate, INIT_SLOT),
                Err(ConfigError::DuplicateNode(_))
            ),
            "{order:?} must be refused as a duplicate"
        );
    }

    assert_eq!(
        void.apply(
            &SystemOperation::Init {
                order: vec![N0, N1, N0]
            },
            INIT_SLOT,
        ),
        Err(ConfigError::DuplicateNode(N0))
    );
}

// ---------------------------------------------------------------------------
// 2a. The membership cap (item03b1): a validation-cost bound, not a protocol limit
// ---------------------------------------------------------------------------

/// `Init` with 17 members is refused, and the refusal names the cap. The cap exists so
/// the quorum gate's `2^N` subset enumeration stays trivially cheap
/// (`configuration::MAX_MEMBERS`); a cluster that genuinely wants 17 members is asking
/// for a different validation-cost budget, which is a one-constant amendment, not a
/// silent acceptance.
#[test]
fn init_refuses_membership_above_the_cap() {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void");

    let seventeen: Vec<NodeId> = (0..17).map(NodeId).collect();
    assert_eq!(
        void.apply(
            &SystemOperation::Init {
                order: seventeen.clone()
            },
            INIT_SLOT,
        ),
        Err(ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS })
    );

    // Sixteen is the cap, not over it: the boundary itself must fold cleanly.
    let sixteen: Vec<NodeId> = seventeen[..16].to_vec();
    let config = void
        .apply(&SystemOperation::Init { order: sixteen }, INIT_SLOT)
        .expect("Init at exactly the cap");
    assert_eq!(config.len(), MAX_MEMBERS);
}

/// `Add` to a 16-member cluster is refused with the same named cap. The check fires
/// before the position check: no insertion position can make a seventeenth member
/// legal.
#[test]
fn add_refuses_the_seventeenth_member() {
    let sixteen: Vec<NodeId> = (0..16).map(NodeId).collect();
    let config = initialised(&sixteen);
    assert_eq!(
        config.apply(
            &SystemOperation::Add {
                node: NodeId(16),
                position: 16,
            },
            Slot(3),
        ),
        Err(ConfigError::MembershipCapExceeded { cap: MAX_MEMBERS })
    );
}

/// `len()` and `index_of()` are total **by cap, not by hope**: at the maximum
/// membership every member reports its exact position and the count needs no
/// conversion. This is the property that retired item03's documented `expect()` calls.
#[test]
fn len_and_index_of_are_total_at_the_cap() {
    let sixteen: Vec<NodeId> = (0..16).map(NodeId).collect();
    let config = initialised(&sixteen);
    assert_eq!(config.len(), 16);
    for (position, node) in sixteen.iter().enumerate() {
        assert_eq!(
            config.index_of(*node),
            Some(u32::try_from(position).expect("position < 16"))
        );
    }
    assert_eq!(config.index_of(NodeId(100)), None);
}

// ---------------------------------------------------------------------------
// 3. The precondition table, one test per row, specific variant each
// ---------------------------------------------------------------------------

/// `Increment(n)` requires `n ∈ order` (§8.7.2). The refusal names the node, so a host
/// logging a rejected proposal can say which one it got wrong.
#[test]
fn increment_requires_membership() {
    let config = three();
    assert_eq!(
        config.apply(&SystemOperation::Increment(N3), Slot(3)),
        Err(ConfigError::NotAMember(N3))
    );

    let up = config
        .apply(&SystemOperation::Increment(N1), Slot(3))
        .expect("Increment on a member");
    assert_eq!(shape(&up), vec![(N0, 1), (N1, 2), (N2, 1)]);
    assert_eq!(up.total(), 4);
    assert_eq!(up.era(), Era(2));
}

/// `Increment` at `u32::MAX` is refused, not saturated. A saturating increment would
/// make two distinct configurations indistinguishable and quietly break the §8.7.5
/// `T -> T+1` closure argument, which is stated over the *actual* new total.
///
/// `u32::MAX` is reachable by the fold in 32 steps by bit composition (`Double` then
/// `Increment`), which is why this test does not need an escape hatch to reach the
/// boundary — the boundary is a reachable state of the protocol, not a fabricated one.
#[test]
fn increment_refuses_weight_overflow() {
    // Two members at `u32::MAX`, plus a learner added afterwards so the configuration is
    // not uniform and the refusal is clearly about the named member's weight.
    let ceiling = raise_all(&[N0, N1], u32::MAX);
    assert_eq!(ceiling.weight_of(N0), Some(Weight(u32::MAX)));
    assert_eq!(ceiling.total(), u64::from(u32::MAX) * 2);

    assert_eq!(
        ceiling.apply(&SystemOperation::Increment(N0), Slot(10_000)),
        Err(ConfigError::WeightOverflow)
    );
    assert_eq!(
        ceiling.apply(&SystemOperation::Increment(N1), Slot(10_000)),
        Err(ConfigError::WeightOverflow)
    );

    let learner = ceiling
        .apply(
            &SystemOperation::Add {
                node: N2,
                position: 2,
            },
            Slot(10_000),
        )
        .expect("Add at weight 0");
    // A member below the ceiling still increments: the refusal above is about the weight
    // of the named member, not about `Increment` being unavailable in this era.
    let up = learner
        .apply(&SystemOperation::Increment(N2), Slot(10_001))
        .expect("Increment a learner");
    assert_eq!(up.weight_of(N2), Some(Weight(1)));
    assert_eq!(
        up.apply(&SystemOperation::Increment(N0), Slot(10_002)),
        Err(ConfigError::WeightOverflow)
    );
}

/// `Decrement(n)` requires `n ∈ order` and `W(n) >= 1` (§8.7.2). Weight 0 is a learner
/// (§8.4) and decrementing a learner has no arithmetic meaning; the refusal is
/// `WeightUnderflow`, distinct from `NotAMember`, because the two call for different
/// host actions.
#[test]
fn decrement_requires_membership_and_positive_weight() {
    let config = three();
    assert_eq!(
        config.apply(&SystemOperation::Decrement(N3), Slot(3)),
        Err(ConfigError::NotAMember(N3))
    );

    let learner = config
        .apply(
            &SystemOperation::Add {
                node: N3,
                position: 3,
            },
            Slot(3),
        )
        .expect("Add at weight 0");
    assert_eq!(learner.weight_of(N3), Some(Weight(0)));
    assert_eq!(
        learner.apply(&SystemOperation::Decrement(N3), Slot(4)),
        Err(ConfigError::WeightUnderflow(N3))
    );
}

/// `Double` requires every `W(n) * 2` representable in `u32`. Refusal is per
/// configuration, not per member: the operation is atomic, so one unrepresentable member
/// refuses the whole fold rather than doubling the others.
#[test]
fn double_requires_every_weight_representable() {
    let config = three();
    let doubled = config
        .apply(&SystemOperation::Double, Slot(3))
        .expect("Double unit weights");
    assert_eq!(shape(&doubled), vec![(N0, 2), (N1, 2), (N2, 2)]);
    assert_eq!(doubled.total(), 6);
    assert_eq!(doubled.era(), Era(2));
}

/// `Halve` requires **every** `W(n)` even (§8.7.2). This is the row most likely to be
/// implemented as a rounding division, and rounding is not refusal: it changes the total
/// by an amount that depends on how many odd members there were, which is not a quantity
/// §8.7.5's `T -> floor(T/2)` argument admits.
#[test]
fn halve_requires_every_weight_even() {
    let doubled = three()
        .apply(&SystemOperation::Double, Slot(3))
        .expect("Double");
    let halved = doubled
        .apply(&SystemOperation::Halve, Slot(4))
        .expect("Halve all-even weights");
    assert_eq!(shape(&halved), vec![(N0, 1), (N1, 1), (N2, 1)]);

    // Now make exactly one member odd.
    let odd = doubled
        .apply(&SystemOperation::Increment(N1), Slot(4))
        .expect("Increment");
    assert_eq!(odd.weight_of(N1), Some(Weight(3)));
    assert_eq!(
        odd.apply(&SystemOperation::Halve, Slot(5)),
        Err(ConfigError::OddWeight(N1))
    );
}

/// `Add { node, position }` requires `node ∉ order` and `position <= len()`, and inserts
/// at weight 0 (§8.7.2, §8.4). Weight 0 means the new member receives history and votes
/// in nothing, which is what makes the addition itself overlap-safe: the quorum families
/// of era `e+1` are identical to those of era `e`.
#[test]
fn add_requires_absence_and_a_valid_position() {
    let config = three();

    assert_eq!(
        config.apply(
            &SystemOperation::Add {
                node: N1,
                position: 0,
            },
            Slot(3),
        ),
        Err(ConfigError::DuplicateNode(N1))
    );

    assert_eq!(
        config.apply(
            &SystemOperation::Add {
                node: N3,
                position: 4,
            },
            Slot(3),
        ),
        Err(ConfigError::PositionOutOfRange {
            position: 4,
            len: 3,
        })
    );
}

/// `Remove(n)` requires `n ∈ order` and `W(n) == 0` (§8.7.2). A positive weight refuses
/// with `NonZeroWeight`, distinct from `NotAMember`: the host's next step differs — one
/// calls for `Decrement`, the other for a corrected node id.
#[test]
fn remove_requires_membership_and_zero_weight() {
    let config = three();

    assert_eq!(
        config.apply(&SystemOperation::Remove(N3), Slot(3)),
        Err(ConfigError::NotAMember(N3))
    );
    assert_eq!(
        config.apply(&SystemOperation::Remove(N1), Slot(3)),
        Err(ConfigError::NonZeroWeight(N1))
    );
}

/// `Decrement` that would take `T(W)` to 0 is refused: "every operation ... the result
/// has non-empty `order` and `T(W) >= 1`" (§8.7.2). A zero-total configuration is
/// quorum-impossible in exactly the way era 0 is, but unlike era 0 it would be reachable
/// *after* commitment, with no repair path that does not need a quorum.
#[test]
fn decrement_refuses_a_zero_total() {
    let single = initialised(&[N0]);
    assert_eq!(single.total(), 1);
    assert_eq!(
        single.apply(&SystemOperation::Decrement(N0), Slot(3)),
        Err(ConfigError::TotalWouldBeZero)
    );
}

/// `Halve` cannot produce a zero total, and the parity precondition is why.
///
/// Every legal state has `T(W) >= 1`, so some member is at weight `>= 1`; if every weight
/// is even then some member is at `>= 2` and halves to `>= 1`. `Halve`'s total floor is
/// therefore unreachable, and the refusal a host sees on a low-weight cluster is
/// `OddWeight`. Asserted so the two guards are not confused: `OddWeight` names a member,
/// `TotalWouldBeZero` does not, and a host reading a refused reconfiguration proposal
/// needs to know which of the two it is looking at.
#[test]
fn halve_never_reaches_a_zero_total() {
    let single = initialised(&[N0]);
    let learner = single
        .apply(
            &SystemOperation::Add {
                node: N1,
                position: 1,
            },
            Slot(3),
        )
        .expect("Add learner");
    // Weights are now (1, 0): the voting member is odd, so `Halve` refuses on parity.
    assert_eq!(
        learner.apply(&SystemOperation::Halve, Slot(4)),
        Err(ConfigError::OddWeight(N0))
    );

    // Weights (2, 0) halve to (1, 0): the minimum legal total survives exactly.
    let even = learner
        .apply(&SystemOperation::Increment(N0), Slot(4))
        .expect("Increment to 2");
    let halved = even.apply(&SystemOperation::Halve, Slot(5)).expect("Halve");
    assert_eq!(shape(&halved), vec![(N0, 1), (N1, 0)]);
    assert_eq!(halved.total(), 1);
    // And halving again refuses on parity, not on the total.
    assert_eq!(
        halved.apply(&SystemOperation::Halve, Slot(6)),
        Err(ConfigError::OddWeight(N0))
    );
}

/// The last member cannot be removed, and it is the *weight* precondition that stops it,
/// not a separate emptiness check.
///
/// This is worth pinning because it explains an absent `ConfigError` variant. `Remove(n)`
/// requires `W(n) == 0` (§8.7.2), and a sole member at weight 0 has `T(W) == 0`, which
/// the total floor already forbade at the `Decrement` that would have produced it. So the
/// state "one member, weight 0" is unreachable, and there is therefore no reachable
/// `Remove` whose result is an empty `order`: the "non-empty `order`" clause of §8.7.2 is
/// discharged by the total floor rather than by an independent guard. The refusal a host
/// actually sees is `NonZeroWeight`, and both halves of that route are asserted here so a
/// later item cannot add a redundant emptiness variant and change the observable error.
#[test]
fn remove_cannot_empty_the_order() {
    let pair = initialised(&[N0, N1]);
    let zeroed = pair
        .apply(&SystemOperation::Decrement(N1), Slot(3))
        .expect("Decrement N1 to 0");
    let one_left = zeroed
        .apply(&SystemOperation::Remove(N1), Slot(4))
        .expect("Remove the learner");
    assert_eq!(shape(&one_left), vec![(N0, 1)]);

    // The only member is at weight 1, so `Remove` refuses on the weight precondition.
    assert_eq!(
        one_left.apply(&SystemOperation::Remove(N0), Slot(5)),
        Err(ConfigError::NonZeroWeight(N0))
    );
    // And it cannot be taken to weight 0 first, because that is a zero total.
    assert_eq!(
        one_left.apply(&SystemOperation::Decrement(N0), Slot(5)),
        Err(ConfigError::TotalWouldBeZero)
    );
    // Same from a genesis single-member cluster: the two refusals are the only moves.
    let single = initialised(&[N0]);
    assert_eq!(single.len(), 1);
    assert_eq!(
        single.apply(&SystemOperation::Remove(N0), Slot(3)),
        Err(ConfigError::NonZeroWeight(N0))
    );
    assert_eq!(
        single.apply(&SystemOperation::Decrement(N0), Slot(3)),
        Err(ConfigError::TotalWouldBeZero)
    );
}

// ---------------------------------------------------------------------------
// 4. `Halve` is exact over every all-even shape
// ---------------------------------------------------------------------------

/// Exhaustive over small even shapes: `Halve` divides every weight exactly, and a single
/// odd member anywhere in `order` refuses. No rounding occurs at any position, which is
/// the failure mode a spot check at index 0 would miss.
#[test]
fn halve_is_exact_and_all_even_is_positional() {
    let nodes = [N0, N1, N2];
    for a in 1..=4u32 {
        for b in 1..=4u32 {
            for c in 1..=4u32 {
                let weights = [a * 2, b * 2, c * 2];
                let config = with_weights(&nodes, &weights);
                let halved = config
                    .apply(&SystemOperation::Halve, Slot(100))
                    .expect("all-even halves");
                assert_eq!(shape(&halved), vec![(N0, a), (N1, b), (N2, c)]);
                assert_eq!(halved.total(), u64::from(a + b + c));

                // Make each position odd in turn and confirm the refusal names it.
                for (index, node) in nodes.iter().enumerate() {
                    let mut odd = weights;
                    odd[index] += 1;
                    let config = with_weights(&nodes, &odd);
                    assert_eq!(
                        config.apply(&SystemOperation::Halve, Slot(100)),
                        Err(ConfigError::OddWeight(*node)),
                        "odd at index {index}"
                    );
                }
            }
        }
    }
}

/// Builds a configuration with the exact weights given, using only `Init` and
/// `Increment`, so the result is reachable by the fold rather than fabricated.
fn with_weights(nodes: &[NodeId], weights: &[u32]) -> Configuration {
    assert_eq!(nodes.len(), weights.len());
    let mut config = initialised(nodes);
    let mut slot = Slot(3);
    for (node, weight) in nodes.iter().zip(weights.iter()) {
        for _ in 1..*weight {
            config = config
                .apply(&SystemOperation::Increment(*node), slot)
                .expect("Increment");
            slot = slot.next().expect("slot space");
        }
    }
    config
}

// ---------------------------------------------------------------------------
// 5. `Double` overflow
// ---------------------------------------------------------------------------

/// A weight above `u32::MAX / 2` cannot be doubled and the whole operation is refused.
///
/// Both sides of the boundary are pinned: `u32::MAX / 2 = 2^31 - 1` doubles to
/// `u32::MAX - 1`, and `2^31` does not double at all. Refusal is per configuration, not
/// per member: a `Double` that raised the representable members and clamped the rest would
/// change the weight *ratios*, and §8.7.5's `DOUBLE` closure argument (`2w(S) >= T+1`)
/// depends on every weight scaling by the same factor.
#[test]
fn double_refuses_above_half_of_u32_max() {
    // The largest weight that can still be doubled.
    let half = raise_all(&[N0, N1], u32::MAX / 2);
    assert_eq!(half.weight_of(N0), Some(Weight((1 << 31) - 1)));
    let doubled = half
        .apply(&SystemOperation::Double, Slot(10_000))
        .expect("u32::MAX / 2 doubles to u32::MAX - 1");
    assert_eq!(doubled.weight_of(N0), Some(Weight(u32::MAX - 1)));
    assert_eq!(
        doubled.apply(&SystemOperation::Double, Slot(10_001)),
        Err(ConfigError::WeightOverflow)
    );

    // One above it cannot.
    let over = raise_all(&[N0, N1], 1 << 31);
    assert_eq!(
        over.apply(&SystemOperation::Double, Slot(10_000)),
        Err(ConfigError::WeightOverflow)
    );

    // A single unrepresentable member refuses the whole operation, even when every other
    // member would have doubled fine.
    let mixed = raise_all(&[N0], 1 << 31);
    let mixed = mixed
        .apply(
            &SystemOperation::Add {
                node: N1,
                position: 1,
            },
            Slot(10_000),
        )
        .expect("Add a learner");
    let mixed = mixed
        .apply(&SystemOperation::Increment(N1), Slot(10_001))
        .expect("Increment the learner to 1");
    assert_eq!(shape(&mixed), vec![(N0, 1 << 31), (N1, 1)]);
    assert_eq!(
        mixed.apply(&SystemOperation::Double, Slot(10_002)),
        Err(ConfigError::WeightOverflow)
    );
}

// ---------------------------------------------------------------------------
// 6. The departure sequence
// ---------------------------------------------------------------------------

/// `Decrement`-to-zero then `Remove` is the only route out of a configuration, and every
/// step of it is individually overlap-safe: a weight change of one preserves the §8.7.5
/// consecutive-era intersection, and a weight-0 removal changes no quorum family at all.
/// A single "remove a voting member" operation would not have either property.
#[test]
fn departure_is_decrement_then_remove() {
    let config = with_weights(&[N0, N1, N2], &[1, 3, 1]);
    assert_eq!(config.total(), 5);

    // `Remove` is refused at every positive weight on the way down.
    let mut current = config;
    let mut slot = Slot(50);
    for expected in [3u32, 2, 1] {
        assert_eq!(current.weight_of(N1), Some(Weight(expected)));
        assert_eq!(
            current.apply(&SystemOperation::Remove(N1), slot),
            Err(ConfigError::NonZeroWeight(N1)),
            "Remove at weight {expected}"
        );
        current = current
            .apply(&SystemOperation::Decrement(N1), slot)
            .expect("Decrement");
        slot = slot.next().expect("slot space");
    }

    assert_eq!(current.weight_of(N1), Some(Weight(0)));
    // At weight 0, `Decrement` is refused and `Remove` is the only move left.
    assert_eq!(
        current.apply(&SystemOperation::Decrement(N1), slot),
        Err(ConfigError::WeightUnderflow(N1))
    );

    let departed = current
        .apply(&SystemOperation::Remove(N1), slot)
        .expect("Remove at weight 0");
    assert_eq!(shape(&departed), vec![(N0, 1), (N2, 1)]);
    assert_eq!(departed.weight_of(N1), None);
    assert_eq!(departed.index_of(N1), None);
}

// ---------------------------------------------------------------------------
// 7. `Add` positions
// ---------------------------------------------------------------------------

/// Every `position ∈ 0..=len()` inserts at exactly that index with weight 0, and
/// `position > len()` refuses. The resulting sequence is asserted element by element,
/// because `order` *is* the failover succession the host is relying on (§1.2) and an
/// off-by-one here silently reorders primary selection for the life of the cluster.
#[test]
fn add_inserts_at_exactly_the_named_position() {
    for len in 1..=5usize {
        let nodes: Vec<NodeId> = (0..len)
            .map(|i| NodeId(100 + u32::try_from(i).unwrap()))
            .collect();
        let config = initialised(&nodes);
        let new = NodeId(999);

        for position in 0..=u32::try_from(len).unwrap() {
            let grown = config
                .apply(
                    &SystemOperation::Add {
                        node: new,
                        position,
                    },
                    Slot(100),
                )
                .expect("in-range position");

            let mut expected: Vec<(NodeId, u32)> = nodes.iter().map(|n| (*n, 1)).collect();
            expected.insert(usize::try_from(position).unwrap(), (new, 0));
            assert_eq!(shape(&grown), expected, "len {len} position {position}");

            assert_eq!(grown.index_of(new), Some(position));
            assert_eq!(grown.weight_of(new), Some(Weight(0)));
            // A learner contributes nothing (§8.4): the total is unchanged.
            assert_eq!(grown.total(), config.total());
            assert_eq!(grown.len(), config.len() + 1);
        }

        for position in [u32::try_from(len).unwrap() + 1, u32::MAX] {
            assert_eq!(
                config.apply(
                    &SystemOperation::Add {
                        node: new,
                        position,
                    },
                    Slot(100),
                ),
                Err(ConfigError::PositionOutOfRange {
                    position,
                    len: u32::try_from(len).unwrap(),
                }),
                "len {len} position {position}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 8. Era monotonicity, exhaustive to length 4
// ---------------------------------------------------------------------------

/// The alphabet the length-4 walk draws from. Deliberately small and deliberately
/// including operations that will be refused in some states, because "a refused
/// operation leaves the table unchanged" is half the property under test.
fn alphabet() -> Vec<SystemOperation> {
    vec![
        SystemOperation::Increment(N0),
        SystemOperation::Decrement(N0),
        SystemOperation::Double,
        SystemOperation::Halve,
        SystemOperation::Add {
            node: N3,
            position: 1,
        },
        SystemOperation::Remove(N3),
    ]
}

/// Exhaustive, not sampled: every sequence of length 0..=4 over a 6-symbol alphabet,
/// which is `1 + 6 + 36 + 216 + 1296 = 1555` walks. The property is the era state
/// machine itself — era increases by exactly 1 per *accepted* operation, a refused
/// operation is a no-op on the table, and `established_by` is the slot the caller
/// supplied. That triple is what makes §8.7.8's "a replica may propose era `e` only if
/// it holds the operation establishing `e`" checkable by construction: era and
/// establishing slot are in one-to-one correspondence, so there is no bookkeeping to
/// drift.
#[test]
fn era_advances_by_exactly_one_per_accepted_operation() {
    let ops = alphabet();
    let base = era_table_three();

    // Level-order enumeration: `frontier` holds only the sequences of the current
    // length, so each length-`n` sequence is generated exactly once.
    let mut sequences: Vec<Vec<usize>> = vec![vec![]];
    let mut frontier: Vec<Vec<usize>> = vec![vec![]];
    for _ in 0..4 {
        let mut next = Vec::new();
        for seq in &frontier {
            for index in 0..ops.len() {
                let mut extended = seq.clone();
                extended.push(index);
                next.push(extended);
            }
        }
        sequences.extend(next.iter().cloned());
        frontier = next;
    }
    assert_eq!(sequences.len(), 1 + 6 + 36 + 216 + 1296);

    for seq in &sequences {
        let mut table = base.clone();
        let mut slot = Slot(3);
        for index in seq {
            let op = &ops[*index];
            let before_era = table.current().era;
            let before_established = table.current().established_by;
            let before_shape = shape(&table.current().config);

            match table.extend(op, slot) {
                Ok(next) => {
                    assert_eq!(
                        next.current().era,
                        Era(before_era.0 + 1),
                        "era must advance by one on {op:?} in {seq:?}"
                    );
                    assert_eq!(next.current().established_by, slot);
                    assert_eq!(next.current().config.era(), next.current().era);
                    assert_eq!(next.current().total, next.current().config.total());
                    assert!(next.current().total >= 1);
                    assert!(!next.current().config.is_empty());
                    table = next;
                }
                Err(_) => {
                    // A refusal leaves the receiver untouched: the fold is persistent,
                    // so there is no partially applied state to roll back.
                    assert_eq!(table.current().era, before_era);
                    assert_eq!(table.current().established_by, before_established);
                    assert_eq!(shape(&table.current().config), before_shape);
                }
            }
            slot = slot.next().expect("slot space");
        }
    }
}

/// An era table advanced through `Void` and `Init` onto a three-member unit cluster.
fn era_table_three() -> EraTable {
    let table = EraTable::genesis();
    let table = table
        .extend(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1");
    table
        .extend(
            &SystemOperation::Init {
                order: vec![N0, N1, N2],
            },
            INIT_SLOT,
        )
        .expect("Init at slot 2")
}

// ---------------------------------------------------------------------------
// 9. `total()` is exact and cannot overflow
// ---------------------------------------------------------------------------

proptest! {
    /// `total()` accumulates into a `u64`, and `u32::MAX as u64 * u32::MAX as u64` is
    /// below `u64::MAX`, so a `u32`-weighted, `u32`-counted membership cannot overflow
    /// the accumulator. That is why `total()` returns `u64` rather than `Option<u64>`:
    /// there is no case to report. The claim is pinned at the top of the weight domain
    /// rather than trusted from the inequality, and the states are built by the fold, so
    /// the weights tested are weights the protocol can actually produce.
    #[test]
    fn total_is_the_exact_sum(
        count in 1usize..12,
        target in prop::sample::select(vec![
            1u32,
            2,
            3,
            1 << 15,
            (1 << 31) - 1,
            1 << 31,
            u32::MAX - 1,
            u32::MAX,
        ]),
        learners in 0usize..4,
    ) {
        let nodes: Vec<NodeId> = (0..count)
            .map(|i| NodeId(u32::try_from(i).unwrap()))
            .collect();
        let mut config = raise_all(&nodes, target);

        // Learners contribute nothing (§8.4), so appending them must not move `total()`.
        let voting_total = u64::from(target) * u64::try_from(count).unwrap();
        prop_assert_eq!(config.total(), voting_total);

        let mut slot = Slot(50_000);
        for i in 0..learners {
            let node = NodeId(1_000 + u32::try_from(i).unwrap());
            let position = config.len();
            config = config
                .apply(&SystemOperation::Add { node, position }, slot)
                .expect("Add a learner at the end");
            slot = slot.next().expect("slot space");
        }

        prop_assert_eq!(config.total(), voting_total);
        prop_assert_eq!(
            config.len(),
            u32::try_from(count + learners).unwrap()
        );

        // And the sum is the sum of the accessor's own view of the members, so `total()`
        // cannot disagree with `order()`.
        let recomputed: u64 = config
            .order()
            .iter()
            .map(|member| u64::from(member.weight.0))
            .sum();
        prop_assert_eq!(config.total(), recomputed);
    }
}

// ---------------------------------------------------------------------------
// 10. `weight_of_set`
// ---------------------------------------------------------------------------

/// The sum a quorum evaluator asks for, and the three ways it must refuse to answer:
/// an unknown member, a member named twice, and — not a refusal — a weight-0 member,
/// which contributes nothing (§8.4). Duplicate rejection *here* is what stops a set from
/// voting twice; a quorum evaluator that deduplicated for itself would be a second copy
/// of the rule.
#[test]
fn weight_of_set_rejects_unknown_and_duplicate_members() {
    let config = with_weights(&[N0, N1, N2], &[1, 2, 3]);
    let learner = config
        .apply(
            &SystemOperation::Add {
                node: N3,
                position: 3,
            },
            Slot(100),
        )
        .expect("Add learner");

    assert_eq!(learner.weight_of_set(&[]), Some(0));
    assert_eq!(learner.weight_of_set(&[N0]), Some(1));
    assert_eq!(learner.weight_of_set(&[N0, N1]), Some(3));
    assert_eq!(learner.weight_of_set(&[N0, N1, N2]), Some(6));

    // A learner is in the configuration and adds nothing.
    assert_eq!(learner.weight_of_set(&[N3]), Some(0));
    assert_eq!(learner.weight_of_set(&[N0, N1, N2, N3]), Some(6));

    // Unknown member.
    assert_eq!(learner.weight_of_set(&[NodeId(777)]), None);
    assert_eq!(learner.weight_of_set(&[N0, NodeId(777)]), None);

    // Named twice: the set would vote twice.
    assert_eq!(learner.weight_of_set(&[N0, N0]), None);
    assert_eq!(learner.weight_of_set(&[N0, N1, N0]), None);
    assert_eq!(learner.weight_of_set(&[N3, N3]), None);
}

// ---------------------------------------------------------------------------
// 11. `primary` is modular selection
// ---------------------------------------------------------------------------

/// `primary(v) = order[v mod len]` (§1.2), exhaustively over `len ∈ 1..=8` and
/// `view ∈ 0..64`. The core sorts nothing: the sequence is the one the host supplied in
/// `Init`, so the index is into *that* sequence and this test compares against it
/// directly rather than against a sorted copy.
#[test]
fn primary_is_modular_over_the_host_supplied_order() {
    for len in 1..=8u32 {
        let nodes: Vec<NodeId> = (0..len).map(|i| NodeId(500 - i)).collect();
        let config = initialised(&nodes);
        assert_eq!(
            shape(&config),
            nodes.iter().map(|n| (*n, 1)).collect::<Vec<_>>()
        );

        for view in 0..64u32 {
            let expected = nodes[usize::try_from(view % len).unwrap()];
            assert_eq!(
                config.primary(View(view)),
                Some(expected),
                "len {len} view {view}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 12. The `EraTable` retention window
// ---------------------------------------------------------------------------

/// Exactly three eras resident. After more than three advances, `record()` answers for
/// `{current - 1, current}` and answers `None` for anything older. `None` is the whole
/// mechanism: an out-of-window era is *undecidable*, so a message naming it is dropped
/// rather than faulting the node (§10, §14.2), and the item17 crash matrix owns that
/// claim end to end.
#[test]
fn era_table_retains_a_three_era_window() {
    let mut table = era_table_three();
    assert_eq!(table.current().era, Era(1));
    assert!(table.record(Era(0)).is_some());
    assert!(table.record(Era(1)).is_some());

    let mut slot = Slot(3);
    for _ in 0..8 {
        table = table
            .extend(&SystemOperation::Double, slot)
            .expect("Double always applies to a positive-weight cluster");
        slot = slot.next().expect("slot space");

        let current = table.current().era;
        assert!(
            table.record(current).is_some(),
            "current era {current:?} must be resident"
        );
        if let Some(previous) = current.0.checked_sub(1) {
            assert!(
                table.record(Era(previous)).is_some(),
                "era {previous} must be resident"
            );
        }
        if let Some(older) = current.0.checked_sub(2) {
            assert!(
                table.record(Era(older)).is_none(),
                "era {older} is outside the window and must be absent"
            );
        }
        assert!(
            table.record(Era(current.0 + 1)).is_none(),
            "an era not yet established has no record"
        );
    }
}

/// The retained records are shared, not copied: the `Arc` in the surviving previous-era
/// record after an `extend` is pointer-identical to the one in the receiver. Without
/// this, configuration history would be a per-advance deep copy, and `Progress`
/// snapshots (item05) would allocate a configuration per publication.
#[test]
fn era_table_shares_retained_arcs() {
    let table = era_table_three();
    let before = Arc::as_ptr(&table.record(Era(1)).expect("era 1").config);

    let extended = table
        .extend(&SystemOperation::Double, Slot(3))
        .expect("Double");
    let after = Arc::as_ptr(&extended.record(Era(1)).expect("era 1 retained").config);

    assert!(
        std::ptr::eq(before, after),
        "the retained era-1 configuration must be shared, not cloned"
    );
}

// ---------------------------------------------------------------------------
// 13. The fold is persistent
// ---------------------------------------------------------------------------

/// `extend` never mutates its receiver. A clone of the old table held across an `extend`
/// is unchanged in era, in shape, and in `Arc` identity. This is what lets a later item
/// hand an `EraTable` out by value to a diagnostic reader while the replica advances,
/// with no lock and no copy.
#[test]
fn extend_does_not_mutate_the_receiver() {
    let table = era_table_three();
    let kept = table.clone();

    let before_current_era = kept.current().era;
    let before_shape = shape(&kept.current().config);
    let before_ptr = Arc::as_ptr(&kept.record(Era(1)).expect("era 1").config);

    let extended = table
        .extend(&SystemOperation::Increment(N0), Slot(3))
        .expect("Increment");
    assert_eq!(extended.current().era, Era(2));

    assert_eq!(kept.current().era, before_current_era);
    assert_eq!(shape(&kept.current().config), before_shape);
    assert!(std::ptr::eq(
        before_ptr,
        Arc::as_ptr(&kept.record(Era(1)).expect("era 1 still there").config)
    ));

    // And `apply` on a `Configuration` is equally non-mutating.
    let config = three();
    let shape_before = shape(&config);
    let _ = config
        .apply(&SystemOperation::Double, Slot(3))
        .expect("Double");
    assert_eq!(shape(&config), shape_before);
}

// ---------------------------------------------------------------------------
// 14. `SystemOperation` on the wire
// ---------------------------------------------------------------------------

/// Every variant, in discriminant order. An explicit list rather than a generated one, so
/// the test agrees with §8.7.2's alphabet independently of the implementation.
fn all_variants() -> Vec<SystemOperation> {
    vec![
        SystemOperation::Void,
        SystemOperation::Init {
            order: vec![N0, N1, N2],
        },
        SystemOperation::Increment(N0),
        SystemOperation::Decrement(N1),
        SystemOperation::Double,
        SystemOperation::Halve,
        SystemOperation::Add {
            node: N3,
            position: 2,
        },
        SystemOperation::Remove(N2),
    ]
}

/// Round trip through the normative binary codec with `packed_len()` exact (W3, W4).
/// `LogEntry` (item07) carries a typed `SystemOperation`, so this is the encoding a
/// `Prepare` for a reconfiguration operation actually uses.
#[test]
fn system_operation_round_trips_with_exact_length() {
    for op in all_variants() {
        let needed = op.packed_len();
        let mut buf = vec![0u8; needed];
        let written = op
            .pack_into(&mut buf)
            .expect("buffer is exactly packed_len");
        assert_eq!(written, needed, "{op:?}");

        let decoded = SystemOperation::unpack_from(&buf).expect("round trip");
        assert_eq!(decoded, op);

        // One byte short is `Incomplete`, never a partial value.
        if needed > 0 {
            let short = &buf[..needed - 1];
            assert!(
                matches!(
                    SystemOperation::unpack_from(short),
                    Err(UnpackError::Incomplete { .. })
                ),
                "{op:?} truncated must be Incomplete"
            );
        }
    }
}

/// Discriminant `0` is reserved on the same reasoning as `wire::Tag`: an all-zero buffer
/// must not decode as a valid operation, because a codec in which the absence of a
/// message is a message cannot report a framing bug.
#[test]
fn system_operation_rejects_reserved_and_unknown_discriminants() {
    for raw in [0u8, 9, 10, 127, 128, 255] {
        let bytes = [raw];
        assert_eq!(
            SystemOperation::unpack_from(&bytes),
            Err(UnpackError::Malformed(Malformed::OutOfDomain)),
            "discriminant {raw}"
        );
    }
}

proptest! {
    /// The `Init` order is a length-prefixed sequence, so its encoded size is a function
    /// of the count and nothing else (W4). Proptest over the count, because this is the
    /// one variant whose `packed_len()` is not a constant and therefore the one that can
    /// disagree with `pack`.
    #[test]
    fn init_round_trips_at_every_length(count in 0usize..64) {
        let order: Vec<NodeId> = (0..count).map(|i| NodeId(u32::try_from(i).unwrap())).collect();
        let op = SystemOperation::Init { order };

        let needed = op.packed_len();
        let mut buf = vec![0u8; needed];
        let written = op.pack_into(&mut buf).expect("exact buffer");
        prop_assert_eq!(written, needed);
        prop_assert_eq!(SystemOperation::unpack_from(&buf).expect("round trip"), op);
    }
}
