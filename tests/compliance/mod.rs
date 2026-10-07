//! The compliance suite's executor, re-exported from the library: the
//! one copy lives at `src/conformance/corpus.rs` behind the `serde`
//! feature, and this module keeps the compliance runner, which includes
//! it by path, compiling unchanged.
//!
//! # The committed compliance artefacts are generated
//!
//! Two committed file sets are generated from the reference behaviour,
//! and two gates say so by rebuilding them in memory and comparing:
//!
//! - the corpus, `tests/compliance/corpus/*.json`, gated by
//!   `the_regenerated_corpus_equals_the_committed_corpus` in
//!   `tests/compliance.rs`;
//! - the Hurl suite, `tests/hurl/corpus-*.hurl`, gated by
//!   `the_committed_hurl_suite_equals_the_generated_suite` in
//!   `tests/conformance_host.rs`.
//!
//! Regenerate both and commit both:
//!
//! ```text
//! make corpus-export   # cargo test --all-features --test compliance -- --ignored export_corpus
//! make hurl-export     # cargo test --features conformance_host --test conformance_host -- --ignored export_hurl
//! ```
//!
//! The order matters: the suite is generated from the corpus bytes, so a
//! corpus change is exported first and the suite second. CI carries the
//! same discipline as a named step of its own on every pull request, `the
//! generated compliance artefacts are current`, which runs both exporters
//! and fails the pull request when the committed tree differs from the
//! regenerated one. The pre-push hook's all-features lane runs the sync
//! gate, so a push with a stale artefact set is a push the hook refuses.
//!
//! The exporters overwrite without guard. A hand edit inside
//! `tests/compliance/corpus/`, or inside a `tests/hurl/corpus-*.hurl`
//! file, is destroyed by the next export with nothing said about it: the
//! bytes are a function of the reference behaviour, so the reference is
//! what an edit belongs in.

#[allow(unused_imports)]
pub use uvrr::conformance::corpus::*;
