//! Sans-I/O Viewstamped Replication Revisited (VRR-2012).
//!
//! `vrr-core` is a library with a C ABI, not a framework. It never tells the host what
//! to do. The whole of it is one total function,
//!
//! ```text
//! tick + message + state -> state + list(messages)
//! ```
//!
//! and nothing in this crate reads a clock, opens a socket, touches a file, or spawns a
//! thread. The host owns time, transport, storage, packetization, threading and naming.
//! `docs/architecture.md` records what is closed for modification, what is open for
//! extension, and the rulings each module below is judged against. The normative
//! specification is `docs/vrr-durability-model.md`, and every
//! section reference in this crate is to that document.
//!
//! # The modules
//!
//! `ids` (identity newtypes, the durable pair, `ViewId`, the `Fault`
//! taxonomy), `wire` (the normative binary codec), `configuration` (era, membership,
//! weights, the reconfiguration fold), `journal` (the four logical journal
//! capabilities), `progress` (the `Progress` record), `observe` (seqlock observation),
//! `invariant` (the closed transition-legality checker), `quorum` (the strategy and the
//! closed intersection gate), `message` (the protocol bodies), `effects` (the host
//! effect vocabulary), `backoff` (the recommended randomized-timeout schedule, as
//! pure arithmetic for the host), `lifecycle` (the boot gate and the typestate
//! marker driver) and `replica` (the plan/publish/confirm pipeline and the
//! lifecycle) carry contract tests. `replica` is split into `normal` (§4
//! `Prepare`/`PrepareOk`/`Commit`), `view_change` (§9),
//! `transfer` (state transfer, §13.1 step 5), `reincarnation` (§10, §6.1) and
//! `reconfiguration` (§8.7.1–§8.7.8).

// `deny`, not `forbid`: `observe` publishes a POD snapshot through a seqlock and the
// future `ffi` module crosses the C ABI. Each will carry one scoped
// `#[allow(unsafe_code)]` with a doc comment naming the invariant that makes it sound
// (decision B1). `forbid` would make those exceptions unexpressible and push the
// unsafety into a separate crate, where it is harder to audit rather than easier.
#![deny(unsafe_code)]
// Every public item states its intent and the property it must hold, so the intent
// outlives the code that currently implements it.
#![deny(missing_docs)]

// The README's Rust code blocks are doctests: `cargo test --doc` compiles and
// runs them. `#[cfg(doctest)]` keeps this module out of every non-doctest build
// (including normal `cargo test`), and the inner `#![doc = include_str!]` makes
// rustdoc extract the fenced blocks as tests without bloating the crate's
// published API docs. This is the only place README code is exercised, so the
// README stays illustrative and a single source of truth stays compiling.
#[cfg(doctest)]
mod readme_doctests {
    #![doc = include_str!("../README.md")]
}

pub mod backoff;
pub mod configuration;
pub mod effects;
pub mod ids;
pub mod invariant;
pub mod journal;
pub mod lifecycle;
pub mod message;
pub mod observe;
pub mod plan;
pub mod progress;
pub mod quorum;
pub mod reconfiguration;
pub mod replica;
pub mod solver;
pub mod wire;

/// Compile-time trace logging of the protocol's internal state at the top and
/// the bottom of processing (the `trace` feature). When the feature is off
/// the macro expands to nothing: the statements it guards are removed from
/// the build entirely, zero production overhead. This is the observability
/// affordance for a downstream host tracing its own problems against the
/// core: every step boundary, planner branch, fold step, and refusal is
/// named at the point it happens.
///
/// Run the suite with tracing on:
///
/// ```text
/// cargo test --features trace -- --nocapture
/// ```
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "trace")]
        eprintln!($($arg)*);
    };
}
