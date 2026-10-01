//! The compliance suite's executor, re-exported from the library: the
//! one copy lives at `src/conformance/corpus.rs` behind the `serde`
//! feature, and this module keeps the compliance runner and the
//! conformance host, which include it by path, compiling unchanged.

#[allow(unused_imports)]
pub use uvrr::conformance::corpus::*;
