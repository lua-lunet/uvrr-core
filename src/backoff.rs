//! The recommended randomized-timeout schedule, as pure arithmetic (S4).
//!
//! The core owns no clock, no randomness and no timer phase: the host draws every
//! random value and delivers every tick, and the `primary_timeout` field of
//! [`crate::replica::ViewChangeKnobs`] counts what the host delivers. What the
//! core CAN own is the arithmetic of the recommendation, so every host computes
//! the same schedule from the same inputs. This module is that arithmetic and
//! nothing else: no clock reads, no random draws, no I/O, no state.
//!
//! # The schedule
//!
//! ```text
//! unit   = 2·rtt                            (unit_from_rtt)
//! window = min(unit · 2^attempt, CAP)        (per failed or interrupted
//!                                             election attempt, capped)
//! fixed  = window / 2                       (floor)
//! span   = window / 2                       (floor)
//! ```
//!
//! All values are whole milliseconds. `attempt` is the host's count of failed or
//! interrupted election attempts since the last commit or adopt — host state; the
//! core holds none of this. The host draws `j` uniformly from `[0, span)`, arms
//! `fixed + j` as that attempt's suspicion deadline, and translates it into host
//! ticks for `primary_timeout`.
//!
//! # Why each rule holds
//!
//! - **Unit = 2·rtt.** The first window is one unit, split evenly into a fixed and
//!   a uniform random half, so the first suspicion deadline falls in
//!   `[unit/2, unit)`: the earliest it can fire is `unit/2 = rtt` — a full round
//!   trip. An answer in flight from a live primary lands before the earliest
//!   suspicion, and the random half adds up to one more round trip of margin. A
//!   unit of 20 ms presumes an RTT of ~10 ms — ~5 ms one-way between servers in
//!   two DCs.
//! - **Doubling.** Each failed or interrupted election attempt doubles the window —
//!   unit, `2·unit`, `4·unit`, … — because a duel that repeats under a flat window
//!   repeats forever.
//! - **The fixed half grows** so a duel survivor gets real work done inside its
//!   window: a node that has just won an election must fit a `Prepare`/`Commit`
//!   round before its own next suspicion fires.
//! - **The random half widens** so dueling hosts' timers spread: the next pair of
//!   deadlines decorrelate, and one node completes its election while the other
//!   waits. The host draws the value; the spread is the whole point, so the span —
//!   not a constant — is what must grow with the window.
//! - **The cap** is a backstop against a partition that keeps attempts failing for
//!   a long time: past it, the schedule stops growing and keeps firing at
//!   [`CAP_MILLIS`]. It binds the whole schedule, including attempt 0: a unit
//!   above the cap is mis-sized (a round trip above 2.5 s), and the schedule will
//!   not pretend otherwise by waiting longer than the cap.
//!
//! # Totality
//!
//! Both functions are total: multiplication saturates rather than wrapping, the
//! doubling exponent clamps at 63 because `2^63` already exceeds the cap for every
//! unit above zero, and attempt counts past the one that reaches the cap saturate
//! there forever. A zero unit yields zero windows — a host that has disabled
//! tick-driven suspicion has no schedule to compute.

/// The window cap, in milliseconds: the backstop past which the doubling stops.
/// Every window is at most this, however large the unit or the attempt count.
pub const CAP_MILLIS: u64 = 5_000;

/// The recommended timeout unit for a measured round trip: twice the RTT, in
/// milliseconds.
///
/// A unit of `2·rtt` makes the earliest possible suspicion ([`window`]'s fixed
/// half, at attempt 0) one full round trip after the last observed activity,
/// so an answer in flight from a live primary lands before it. Saturates
/// rather than wraps at the `u64` edge.
pub fn unit_from_rtt(rtt_millis: u64) -> u64 {
    rtt_millis.saturating_mul(2)
}

/// The suspicion window for `attempt` under `unit_millis`, split into its fixed
/// and uniform-random halves, in milliseconds.
///
/// The window is `min(unit·2^attempt, [`CAP_MILLIS`])`, and the returned pair is
/// `(fixed_millis, jitter_millis)` with `fixed = jitter = window/2` (floor: an
/// odd window's halves sum to the window minus one). Attempt 0 is the unit
/// itself. Doubling saturates at the cap: attempts past the one that reaches
/// it return `(CAP_MILLIS/2, CAP_MILLIS/2)` forever, and the `2^attempt` factor
/// clamps at `2^63`, which exceeds the cap for every unit above zero.
pub fn window(unit_millis: u64, attempt: u64) -> (u64, u64) {
    let scale = 1u64 << attempt.min(63);
    let whole = unit_millis.saturating_mul(scale).min(CAP_MILLIS);
    (whole / 2, whole / 2)
}
