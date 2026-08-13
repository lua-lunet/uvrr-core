//! Contract for `vrr_core::ids` and the `Fault` enum of `vrr_core::invariant`.
//!
//! Spec §1.2 (primary succession), §1.3 (slots and frontiers), §8.7.3 with Amendment
//! A1, and decisions W1, W2, S3, S4.
//!
//! This file is a gate on identity and arithmetic, not on protocol behaviour. Four
//! properties are asserted, and each of them is a property the rest of the crate is
//! entitled to assume without re-checking:
//!
//! 1. every identifier is layout-identical to its primitive, so the C ABI can
//!    pass them by value without a conversion layer that could disagree with itself;
//! 2. no successor wraps — every arithmetic edge is `None`, never a silently reused
//!    view or slot, because a reused view number is unrecoverable divergence;
//! 3. `ViewId::is_legal_successor` is the single point of truth for §8.7.3's surviving
//!    era/view relation, and its truth table is pinned exhaustively so a later change
//!    cannot loosen it by accident;
//! 4. the W1 ordering claim — that `(era, view)` and `(view, era)` lexicographic
//!    orders coincide on legal histories — is discharged by proptest here rather than
//!    asserted in a comment, because it is the whole justification for the derived
//!    `Ord` on `ViewId` being safe to use in the §10 higher-view rule.
//!
//! Groups 3 and 5 are exhaustive loops rather than samplers: the domains are tiny and
//! total coverage is strictly stronger than any number of random draws.

use std::cmp::Ordering;
use std::mem::{align_of, size_of};

use proptest::prelude::*;
use vrr::ids::{
    ClientId, Era, MessageId, NodeId, RequestNumber, Slot, Tick, View, ViewId, next_view_selecting,
};
use vrr::invariant::Fault;

// ---------------------------------------------------------------------------
// 1. Layout
// ---------------------------------------------------------------------------

/// Each newtype is `#[repr(transparent)]` over its primitive, so it crosses the C ABI
/// without marshalling. A later derive or added field that changed the layout
/// would be an ABI break invisible at the Rust call sites; this pins it.
#[test]
fn layout_matches_primitive() {
    assert_eq!(size_of::<NodeId>(), size_of::<u32>());
    assert_eq!(align_of::<NodeId>(), align_of::<u32>());

    assert_eq!(size_of::<Era>(), size_of::<u32>());
    assert_eq!(align_of::<Era>(), align_of::<u32>());

    assert_eq!(size_of::<View>(), size_of::<u32>());
    assert_eq!(align_of::<View>(), align_of::<u32>());

    assert_eq!(size_of::<Slot>(), size_of::<u64>());
    assert_eq!(align_of::<Slot>(), align_of::<u64>());

    assert_eq!(size_of::<Tick>(), size_of::<u64>());
    assert_eq!(align_of::<Tick>(), align_of::<u64>());

    assert_eq!(size_of::<RequestNumber>(), size_of::<u64>());
    assert_eq!(align_of::<RequestNumber>(), align_of::<u64>());

    assert_eq!(size_of::<MessageId>(), size_of::<[u8; 16]>());
    assert_eq!(align_of::<MessageId>(), align_of::<[u8; 16]>());

    assert_eq!(size_of::<ClientId>(), size_of::<u128>());
    assert_eq!(align_of::<ClientId>(), align_of::<u128>());

    // `ViewId` is two `u32` fields and nothing else. The 20-byte big-endian header of
    // W1 is `(tag, era, view, slot)`; the eight bytes contributed by `ViewId` must not
    // acquire padding.
    assert_eq!(size_of::<ViewId>(), 8);
}

// ---------------------------------------------------------------------------
// 2. No silent wrap
// ---------------------------------------------------------------------------

/// Every arithmetic edge yields `None`. A wrapping view number reuses a primary term,
/// and a wrapping slot reuses a log position; both are unrecoverable, so the type
/// system must not offer the operation that produces them.
#[test]
fn successors_do_not_wrap() {
    assert_eq!(View(u32::MAX).next(), None);
    assert_eq!(Era(u32::MAX).next(), None);
    assert_eq!(Slot(u64::MAX).next(), None);
    assert_eq!(Slot(0).prev(), None);

    // The non-edge cases, so a `None`-always implementation cannot pass this test.
    assert_eq!(View(0).next(), Some(View(1)));
    assert_eq!(Era(7).next(), Some(Era(8)));
    assert_eq!(Slot(0).next(), Some(Slot(1)));
    assert_eq!(Slot(1).prev(), Some(Slot(0)));

    assert_eq!(View::INITIAL, View(0));
    assert_eq!(Era::INITIAL, Era(0));
    assert_eq!(Slot::FIRST, Slot(0));
}

/// `ViewId` succession is checked in both components independently: W1's whole point is
/// that the two fields cannot alias, so an overflow in one must not be observable as a
/// change in the other.
#[test]
fn view_id_successors_do_not_wrap() {
    let at_view_max = ViewId {
        era: Era(3),
        view: View(u32::MAX),
    };
    assert_eq!(at_view_max.next_in_era(), None);
    assert_eq!(at_view_max.next_in_next_era(), None);

    let at_era_max = ViewId {
        era: Era(u32::MAX),
        view: View(9),
    };
    assert_eq!(
        at_era_max.next_in_era(),
        Some(ViewId {
            era: Era(u32::MAX),
            view: View(10)
        })
    );
    assert_eq!(at_era_max.next_in_next_era(), None);

    let ordinary = ViewId {
        era: Era(4),
        view: View(11),
    };
    assert_eq!(
        ordinary.next_in_era(),
        Some(ViewId {
            era: Era(4),
            view: View(12)
        })
    );
    assert_eq!(
        ordinary.next_in_next_era(),
        Some(ViewId {
            era: Era(5),
            view: View(12)
        })
    );
}

// ---------------------------------------------------------------------------
// 3. `is_legal_successor` truth table, exhaustive over a small domain
// ---------------------------------------------------------------------------

/// Exhaustive over era delta and view delta in `{-1, 0, +1, +2}`. Accept exactly
/// `view delta > 0` with `era delta` in `{0, +1}`: §8.7.3's surviving relation permits
/// era to stand still or advance by one across a view change, and §8.7.3 permits view
/// gaps, so `+2` in the view is legal while `+2` in the era is not.
#[test]
fn legal_successor_truth_table() {
    const DELTAS: [i64; 4] = [-1, 0, 1, 2];
    // Base chosen so every delta stays in range and neither field is zero.
    let base = ViewId {
        era: Era(10),
        view: View(20),
    };

    for era_delta in DELTAS {
        for view_delta in DELTAS {
            let next = ViewId {
                era: Era(apply(10, era_delta)),
                view: View(apply(20, view_delta)),
            };
            let expected = view_delta > 0 && (era_delta == 0 || era_delta == 1);
            assert_eq!(
                base.is_legal_successor(next),
                expected,
                "era delta {era_delta}, view delta {view_delta}"
            );
        }
    }
}

/// Signed delta applied to a small `u32` base without an `as` cast between widths.
fn apply(base: u32, delta: i64) -> u32 {
    let signed = i64::from(base) + delta;
    u32::try_from(signed).expect("test domain keeps every value in range")
}

/// A view change is not a self-loop, and it never goes backwards, whatever the era
/// does. Stated separately because the truth table above holds era and view deltas
/// independent and this is the diagonal that matters most in review.
#[test]
fn legal_successor_rejects_identity_and_regression() {
    let v = ViewId {
        era: Era(2),
        view: View(2),
    };
    assert!(!v.is_legal_successor(v));
    assert!(v.is_legal_successor(ViewId {
        era: Era(2),
        view: View(3)
    }));
    assert!(v.is_legal_successor(ViewId {
        era: Era(3),
        view: View(3)
    }));
    assert!(!v.is_legal_successor(ViewId {
        era: Era(1),
        view: View(3)
    }));
    assert!(!v.is_legal_successor(ViewId {
        era: Era(4),
        view: View(3)
    }));
}

/// The successor constructors must produce values the legality rule accepts. If these
/// two ever disagree, one of them is wrong and the crate has two notions of legality.
#[test]
fn constructors_agree_with_legality() {
    for era in 0u32..8 {
        for view in 0u32..8 {
            let v = ViewId {
                era: Era(era),
                view: View(view),
            };
            let in_era = v.next_in_era().expect("no overflow in this domain");
            let next_era = v.next_in_next_era().expect("no overflow in this domain");
            assert!(v.is_legal_successor(in_era));
            assert!(v.is_legal_successor(next_era));
        }
    }
}

/// The era-exhaustion branch of `is_legal_successor`: at `Era::MAX` the
/// era-`+1` successor is unrepresentable, so "era unchanged" is the only legal
/// successor left. The branch exists because §8.7.3 forbids wraparound — a reused
/// era would make `config(e)` ambiguous — and it had no direct test.
#[test]
fn legal_successor_at_era_exhaustion() {
    let base = ViewId {
        era: Era(u32::MAX),
        view: View(7),
    };
    // The one remaining legal successor: same era, higher view.
    assert!(base.is_legal_successor(ViewId {
        era: Era(u32::MAX),
        view: View(8)
    }));
    // Identity, regression, and era regression are still refused.
    assert!(!base.is_legal_successor(base));
    assert!(!base.is_legal_successor(ViewId {
        era: Era(u32::MAX),
        view: View(6)
    }));
    assert!(!base.is_legal_successor(ViewId {
        era: Era(u32::MAX - 1),
        view: View(8)
    }));
    // The constructors agree: the era is spent, the view is not.
    assert_eq!(base.next_in_next_era(), None);
    assert_eq!(
        base.next_in_era(),
        Some(ViewId {
            era: Era(u32::MAX),
            view: View(8)
        })
    );
}

// ---------------------------------------------------------------------------
// 4. Ordering agreement — the W1 claim
// ---------------------------------------------------------------------------

/// Generates a legal history: a strictly increasing view sequence with a non-decreasing
/// era, which is exactly the set of pairs `is_legal_successor` admits transitively.
fn legal_history() -> impl Strategy<Value = Vec<ViewId>> {
    // Each step advances the view by 1..=3 (gaps are legal, §8.7.3) and the era by
    // 0 or 1, matching the successor relation.
    prop::collection::vec((1u32..=3, 0u32..=1), 1..24).prop_map(|steps| {
        let mut era = 0u32;
        let mut view = 0u32;
        let mut out = vec![ViewId {
            era: Era(era),
            view: View(view),
        }];
        for (view_step, era_step) in steps {
            view += view_step;
            era += era_step;
            out.push(ViewId {
                era: Era(era),
                view: View(view),
            });
        }
        out
    })
}

proptest! {
    /// The load-bearing subtlety of W1: on any legal pair the orderings `(era, view)`
    /// and `(view, era)` coincide, because era advances only as views advance. The
    /// derived `Ord` on `ViewId { era, view }` is therefore usable for the §10
    /// higher-view rule without consulting configuration state. Pairs on which the two
    /// disagree are illegal and are rejected upstream, not silently ordered — which is
    /// why this is a test over legal histories and not over arbitrary pairs.
    #[test]
    fn orderings_agree_on_legal_pairs(history in legal_history()) {
        for a in &history {
            for b in &history {
                let by_era_then_view: Ordering = a.cmp(b);
                let by_view_then_era = (a.view, a.era).cmp(&(b.view, b.era));
                prop_assert_eq!(
                    by_era_then_view,
                    by_view_then_era,
                    "orderings disagree on {:?} vs {:?}",
                    a,
                    b
                );
            }
        }
    }

    /// Successive elements of a legal history satisfy the legality rule, so the
    /// generator in the property above really does generate legal pairs. Without this
    /// the ordering property could be vacuously true of a wrong generator.
    #[test]
    fn generated_history_is_legal(history in legal_history()) {
        for pair in history.windows(2) {
            prop_assert!(pair[0].is_legal_successor(pair[1]));
        }
    }
}

// ---------------------------------------------------------------------------
// 5. `next_view_selecting`, exhaustive over a small domain
// ---------------------------------------------------------------------------

/// For every small `(members, index, current)` the answer is the *least* view strictly
/// greater than `current` that selects `index` under `primary(v) = order[v mod N]`
/// (§1.2). Minimality is checked by scanning the open interval, so a correct-modulus
/// but non-minimal answer fails. Skipped views are legal (§8.7.3) and are the expected
/// output here, not a defect.
#[test]
fn next_view_selecting_is_least_and_correct() {
    for members in 1u32..=8 {
        for index in 0..members {
            for current in 0u32..64 {
                let got = next_view_selecting(View(current), index, members)
                    .expect("no overflow in this domain");
                assert!(got.0 > current, "{got:?} must exceed {current}");
                assert_eq!(got.0 % members, index, "{got:?} must select {index}");
                for skipped in (current + 1)..got.0 {
                    assert_ne!(
                        skipped % members,
                        index,
                        "{skipped} selects {index} and is earlier than {got:?}"
                    );
                }
            }
        }
    }
}

/// Degenerate membership is refused, not panicked on: this function is reachable from
/// a host-supplied configuration and the core does not abort a host process over an
/// argument it can reject.
#[test]
fn next_view_selecting_rejects_degenerate_arguments() {
    assert_eq!(next_view_selecting(View(0), 0, 0), None);
    assert_eq!(next_view_selecting(View(7), 3, 0), None);
    for members in 1u32..=8 {
        assert_eq!(next_view_selecting(View(5), members, members), None);
        assert_eq!(next_view_selecting(View(5), members + 1, members), None);
        assert_eq!(next_view_selecting(View(5), u32::MAX, members), None);
    }
}

/// Near `View(u32::MAX)` the answer may not exist. It is `None`, never a wrapped view.
#[test]
fn next_view_selecting_does_not_wrap() {
    // The last representable view is `u32::MAX`. With three members it selects
    // `u32::MAX % 3 == 0`, so index 0 is reachable from `u32::MAX - 3` and no index is
    // reachable from `u32::MAX` itself.
    assert_eq!(next_view_selecting(View(u32::MAX), 0, 3), None);
    assert_eq!(next_view_selecting(View(u32::MAX), 1, 3), None);
    assert_eq!(next_view_selecting(View(u32::MAX), 2, 3), None);

    assert_eq!(
        next_view_selecting(View(u32::MAX - 3), 0, 3),
        Some(View(u32::MAX))
    );
    // Index 1 would require `u32::MAX + 1`.
    assert_eq!(next_view_selecting(View(u32::MAX - 1), 1, 3), None);

    // Single-member configurations still terminate at the edge.
    assert_eq!(next_view_selecting(View(u32::MAX), 0, 1), None);
    assert_eq!(
        next_view_selecting(View(u32::MAX - 1), 0, 1),
        Some(View(u32::MAX))
    );
}

/// A membership at the top of the `u32` range. `members` arrives from a host-supplied
/// configuration, so it is an argument the function must survive even though it is not a
/// configuration size anyone would deploy. This pins the two answers at the extreme: the
/// representable one, and the one a full `members` stride past it that is not.
#[test]
fn next_view_selecting_survives_huge_membership() {
    let members = u32::MAX;
    let index = u32::MAX - 2;
    assert_eq!(
        next_view_selecting(View(2), index, members),
        Some(View(index))
    );
    // From the selecting view itself the next one would need a full `members` stride,
    // which is not representable.
    assert_eq!(next_view_selecting(View(index), index, members), None);

    // The other branch of the residue split, also at the extreme: `floor = 1` has
    // residue 1 under `members = u32::MAX`, so index 0 is `members - 1` views away and
    // lands exactly on the last representable view, whose residue is indeed 0.
    assert_eq!(
        next_view_selecting(View(0), 0, u32::MAX),
        Some(View(u32::MAX))
    );
    assert_eq!(u32::MAX % u32::MAX, 0);
    // Index 1 from there would need `u32::MAX + 1`.
    assert_eq!(next_view_selecting(View(u32::MAX), 0, u32::MAX), None);
}

// ---------------------------------------------------------------------------
// 6. `Slot::distance_to`
// ---------------------------------------------------------------------------

/// Suffix-length arithmetic (§13.1) must be exact or absent. `None` when the argument
/// is behind, because a negative suffix length silently coerced to zero would make a
/// truncated transfer look complete.
#[test]
fn slot_distance_is_exact_or_absent() {
    assert_eq!(Slot(4).distance_to(Slot(4)), Some(0));
    assert_eq!(Slot(4).distance_to(Slot(9)), Some(5));
    assert_eq!(Slot(4).distance_to(Slot(3)), None);
    assert_eq!(Slot(0).distance_to(Slot(u64::MAX)), Some(u64::MAX));
    assert_eq!(Slot(u64::MAX).distance_to(Slot(0)), None);
    assert_eq!(Slot(u64::MAX).distance_to(Slot(u64::MAX)), Some(0));

    for from in 0u64..16 {
        for to in 0u64..16 {
            let got = Slot(from).distance_to(Slot(to));
            if to >= from {
                assert_eq!(got, Some(to - from));
            } else {
                assert_eq!(got, None);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 7. `Fault` exhaustiveness
// ---------------------------------------------------------------------------

/// A wildcard-free `match` over every `Fault` variant. Adding a variant later is a
/// compile error here, which is the point: a new fault source must be classified
/// deliberately, and the sticky property must be restated for it rather than inherited
/// by default. Spec §5 invariant 5, §12, §15, decisions Q1 and S3.
#[test]
fn fault_variants_are_exhaustive_and_sticky() {
    const ALL: [Fault; 5] = [
        Fault::IllegalTransition,
        Fault::IndeterminatePersistence,
        Fault::QuorumObligation,
        Fault::ProgressJournalDivergence,
        Fault::HostDeclared,
    ];

    for fault in ALL {
        // No wildcard arm. This match is the gate.
        let named = match fault {
            Fault::IllegalTransition => "IllegalTransition",
            Fault::IndeterminatePersistence => "IndeterminatePersistence",
            Fault::QuorumObligation => "QuorumObligation",
            Fault::ProgressJournalDivergence => "ProgressJournalDivergence",
            Fault::HostDeclared => "HostDeclared",
        };
        assert_eq!(format!("{fault:?}"), named);
        assert!(fault.is_sticky(), "{named} must be sticky");
    }

    // The variants are distinct, so `ALL` above cannot be silently short.
    for (i, a) in ALL.iter().enumerate() {
        for (j, b) in ALL.iter().enumerate() {
            assert_eq!(a == b, i == j);
        }
    }
}
