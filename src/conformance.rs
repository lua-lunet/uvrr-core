//! The reference conformance host, in the library
//! (`docs/uvrr-host-compliance.md` §8).
//!
//! The module is the crate's own host tooling: the sans-I/O core proper
//! reads no clock and opens no socket, and the two modules that do are
//! stated here so the boundary is explicit. `harness` is the reference
//! host the Definition of Done runs on; `corpus` is the case record and
//! executor behind the `serde` feature, the same opt-in the reference
//! runner takes; `server` and `host` are the loopback transport the
//! corpus is replayed over, behind the `conformance_host` feature.
//!
//! The zero-dependency property a consumer buys is untouched: every
//! Cargo dependency stays optional and no feature name enters the
//! default set.

#[cfg(feature = "serde")]
pub mod corpus;
pub mod harness;
#[cfg(feature = "conformance_host")]
pub mod host;
#[cfg(feature = "conformance_host")]
pub mod server;

#[cfg(feature = "serde")]
pub use corpus::{
    Case, Executor, Expect, Op, SystemOp, assert_expectation, identity, pair_of, wire_hex,
};
pub use harness::{Harness, StepOutcome, mint_id, mint_pair};
#[cfg(feature = "conformance_host")]
pub use host::Host;
#[cfg(feature = "conformance_host")]
pub use server::{Request, Response, request, serve};
