//! Contract for `vrr::backoff` — the recommended randomized-timeout schedule,
//! as pure arithmetic.
//!
//! Decision S4 externalises the clock: the host owns the timers and the
//! randomness, and the schedule is published so every host computes the same
//! windows from the same inputs. What is pinned here, so the recommendation
//! (stated on `ViewChangeKnobs`) and the code cannot drift:
//!
//! 1. **Unit sizing is 2×rtt**, exhaustively over a closed rtt domain and at
//!    the `u64` edge, where doubling saturates instead of wrapping.
//! 2. **Windows double from the unit and split evenly.** `window(u, a)` is
//!    `min(u·2^a, 5000)` with `fixed = jitter = window/2`, checked
//!    exhaustively over units × attempts against an independent iterative
//!    reference — a second derivation of the same schedule that must agree
//!    with the module's closed form.
//! 3. **The recommendation's worked example is pinned exactly**: a 10 ms RTT
//!    sizes a 20 ms unit, whose windows run (10,10), (20,20), (40,40), … until
//!    the cap binds at attempt 8 (20·2^8 = 5120 > 5000), after which every
//!    attempt count — an absurd one included — saturates at (2500, 2500).

use vrr::backoff::{CAP_MILLIS, unit_from_rtt, window};

/// The window of `attempt` under `unit`, by iterated doubling with an early
/// exit at the cap — the schedule stated as a loop rather than the module's
/// closed form, so agreement between the two is evidence and not tautology.
fn reference_window(unit: u64, attempt: u64) -> u64 {
    let mut whole = unit;
    for _ in 0..attempt {
        if whole >= CAP_MILLIS {
            break;
        }
        whole = whole.saturating_mul(2);
    }
    whole.min(CAP_MILLIS)
}

/// Asserts the split of `window(unit, attempt)` against the reference: the
/// halves are even, and each is the floor half of the reference window.
fn assert_matches_reference(unit: u64, attempt: u64) {
    let (fixed, jitter) = window(unit, attempt);
    let whole = reference_window(unit, attempt);
    assert_eq!(fixed, jitter, "unit {unit} attempt {attempt}");
    assert_eq!(fixed, whole / 2, "unit {unit} attempt {attempt}");
}

#[test]
fn unit_is_twice_the_round_trip() {
    for rtt in 0..=2048u64 {
        assert_eq!(unit_from_rtt(rtt), 2 * rtt, "rtt {rtt} ms");
    }
    assert_eq!(unit_from_rtt(u64::MAX / 2 + 1), u64::MAX);
    assert_eq!(unit_from_rtt(u64::MAX), u64::MAX);
}

#[test]
fn windows_double_from_the_unit_and_split_evenly() {
    for unit in 0..=64u64 {
        for attempt in 0..=16u64 {
            assert_matches_reference(unit, attempt);
        }
    }
    for unit in [
        65u64,
        100,
        312,
        625,
        1000,
        2499,
        2500,
        2501,
        3125,
        4999,
        5000,
        5001,
        6000,
        10_000,
        u64::MAX / 2,
        u64::MAX,
    ] {
        for attempt in [0u64, 1, 8, 13, 63, 64, 65, 1000, u64::MAX] {
            assert_matches_reference(unit, attempt);
        }
    }
}

#[test]
fn schedule_is_pinned_for_a_twenty_millisecond_unit() {
    assert_eq!(CAP_MILLIS, 5000);
    let unit = unit_from_rtt(10);
    assert_eq!(unit, 20);
    let doubling = [
        (10, 10),
        (20, 20),
        (40, 40),
        (80, 80),
        (160, 160),
        (320, 320),
        (640, 640),
        (1280, 1280),
    ];
    for (attempt, expected) in doubling.iter().enumerate() {
        assert_eq!(window(unit, attempt as u64), *expected, "attempt {attempt}");
    }
    for attempt in [8u64, 9, 10, 100, u64::MAX] {
        assert_eq!(window(unit, attempt), (2500, 2500), "attempt {attempt}");
    }
}
