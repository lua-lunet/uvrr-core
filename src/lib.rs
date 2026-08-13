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
//! # State of this crate
//!
//! The old alpha implementation was cut rather than patched: it encoded a
//! mutable `&mut self step() -> Vec<Output>` transition, an embedded advisory-lock
//! service, JSON in the datagram path, and a caller-supplied recovery nonce. All four
//! are contradicted by the contract above.
//!
//! The rewrite is landing layer by layer, `ids -> wire -> configuration -> {journal,
//! progress} -> quorum -> replica`. `ids` (identity newtypes, `ViewId`, the `Fault`
//! taxonomy), `wire` (the normative binary codec), `configuration` (era, membership,
//! weights, the reconfiguration fold), `journal` (the four logical journal
//! capabilities), `progress` (the `Progress` record), `observe` (seqlock observation),
//! `invariant` (the closed transition-legality checker), `quorum` (the strategy and the
//! closed intersection gate), `message` (the protocol bodies), `effects` (the host
//! effect vocabulary) and `replica` (the plan/publish/confirm pipeline and lifecycle)
//! are real modules with contract tests. `transfer` remains a documented stub.

// `deny`, not `forbid`: `observe` publishes a POD snapshot through a seqlock and the
// future `ffi` module crosses the C ABI. Each will carry one scoped
// `#[allow(unsafe_code)]` with a doc comment naming the invariant that makes it sound
// (decision B1). `forbid` would make those exceptions unexpressible and push the
// unsafety into a separate crate, where it is harder to audit rather than easier.
#![deny(unsafe_code)]
// Every public item states its intent and the property it must hold, so the intent
// outlives the code that currently implements it.
#![deny(missing_docs)]

pub mod configuration;
pub mod effects;
pub mod ids;
pub mod invariant;
pub mod journal;
pub mod message;
pub mod observe;
pub mod progress;
pub mod quorum;
pub mod replica;
pub mod transfer;
pub mod wire;
