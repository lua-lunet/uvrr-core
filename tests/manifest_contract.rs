//! Manifest contract: the published dependency surface of `vrr-core`.
//!
//! This is a gate, not a smoke test. It asserts properties of `Cargo.toml` and
//! `src/lib.rs` that a later change cannot silently regress by adding a convenient
//! dependency or relaxing a lint. It reads both files with `include_str!` and
//! parses with std string operations only, deliberately: pulling in a TOML crate
//! to police the dependency set would be self-defeating, and a `[dev-dependencies]`
//! entry is still a build-time dependency for anyone who runs `cargo test` on this
//! crate. The parser below is therefore crude on purpose. It only needs to be
//! correct for a manifest this crate controls.
//!
//! Desired property, stated once so it survives any rewrite of the assertions: a
//! consumer who writes `vrr-core = "0.1"` with `default-features = false` pulls in
//! zero transitive crates, so that `vrr-core` can be vendored into a build with a
//! frozen or audited dependency set (decision W2, W3).

/// The manifest under test, embedded at compile time so the assertions cannot
/// drift from the manifest that produced this test binary.
const MANIFEST: &str = include_str!("../Cargo.toml");

/// The crate root under test, embedded for the same reason.
const LIB_RS: &str = include_str!("../src/lib.rs");

/// Returns the body of the `[name]` table: the text after the header line up to
/// the next line beginning with `[`. A sub-table (`[name.sub]`) terminates the
/// section, which is the conservative reading here — it can only cause a false
/// failure, never a false pass, and this crate's manifest has no sub-tables under
/// the tables interrogated below.
fn table<'a>(manifest: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("\n[{name}]\n");
    let start = manifest.find(&header)? + header.len();
    let body = &manifest[start..];
    match body.find("\n[") {
        Some(end) => Some(&body[..end]),
        None => Some(body),
    }
}

/// A manifest line that declares a dependency, i.e. `key = ...` at column zero,
/// ignoring blank lines and comments.
fn is_dependency_line(line: &str) -> bool {
    match line.chars().next() {
        None => false,
        Some('#') => false,
        Some(c) if c.is_whitespace() => false,
        Some(_) => line.contains('='),
    }
}

#[test]
fn non_optional_dependency_set_is_empty() {
    let deps = table(MANIFEST, "dependencies")
        .expect("[dependencies] table must exist, even if every entry is optional");

    let offenders: Vec<&str> = deps
        .lines()
        .filter(|line| is_dependency_line(line))
        .filter(|line| !line.contains("optional = true"))
        .collect();

    assert!(
        offenders.is_empty(),
        "every [dependencies] entry must carry `optional = true`; \
         a library consumer's non-optional dependency set is empty. Offending lines: {offenders:?}"
    );
}

/// The complete set of permitted `[dev-dependencies]`, in sorted order.
///
/// `proptest` settled the project's position on dev-dependencies before `serde_json`
/// arrived: build-time tooling for `cargo test` is not a consumer's supply chain, and the
/// `[dependencies]` gate above is what protects the consumer. This list keeps that
/// concession bounded rather than open, so the argument has to be made again for a third
/// entry (decision P3).
const DEV_DEPENDENCY_ALLOWLIST: [&str; 2] = ["proptest", "serde_json"];

/// The name a dependency line declares: the text left of the first `=`, trimmed.
fn dependency_name(line: &str) -> &str {
    line.split('=').next().unwrap_or(line).trim()
}

#[test]
fn dev_dependency_set_is_exactly_the_allowlist() {
    let deps = table(MANIFEST, "dev-dependencies")
        .expect("[dev-dependencies] table must exist; proptest and serde_json live there");

    let mut declared: Vec<&str> = deps
        .lines()
        .filter(|line| is_dependency_line(line))
        .map(dependency_name)
        .collect();
    declared.sort_unstable();

    let unexpected: Vec<&&str> = declared
        .iter()
        .filter(|name| !DEV_DEPENDENCY_ALLOWLIST.contains(name))
        .collect();
    let missing: Vec<&&str> = DEV_DEPENDENCY_ALLOWLIST
        .iter()
        .filter(|name| !declared.contains(name))
        .collect();

    assert!(
        unexpected.is_empty(),
        "[dev-dependencies] declares {unexpected:?}, which is outside the allowlist \
         {DEV_DEPENDENCY_ALLOWLIST:?}. A new dev-dependency is a decision, not a \
         convenience: amend P3 in docs/architecture.md with the reason the test cannot be \
         written without it, then add it to DEV_DEPENDENCY_ALLOWLIST here. Do not add it \
         quietly."
    );
    assert!(
        missing.is_empty(),
        "[dev-dependencies] is missing {missing:?}, which the allowlist requires. \
         Removing an entry is also a decision: if the tests no longer need it, drop it \
         from DEV_DEPENDENCY_ALLOWLIST and record why in P3, so the allowlist does not \
         outlive its rationale."
    );
    assert_eq!(
        declared, DEV_DEPENDENCY_ALLOWLIST,
        "[dev-dependencies] must name exactly the allowlist, no more and no fewer"
    );
}

#[test]
fn license_is_mit() {
    assert!(
        MANIFEST.contains("license = \"MIT\""),
        "license must be MIT (decision P2); LICENSE and README already say MIT"
    );
    assert!(
        !MANIFEST.contains("Apache"),
        "no residual Apache-2.0 declaration may remain in the manifest"
    );
}

#[test]
fn uuid_appears_nowhere() {
    assert!(
        !MANIFEST.contains("uuid"),
        "`uuid` must not appear anywhere in the manifest; \
         OperationId is host-supplied (decision W2)"
    );
}

#[test]
fn msrv_is_declared() {
    assert!(
        MANIFEST.contains("rust-version = \"1.85\""),
        "rust-version = \"1.85\" must be declared; edition 2024 sets the floor \
         and clippy's msrv lints enforce it"
    );
}

#[test]
fn feature_table_declares_serde_and_maelstrom() {
    let features = table(MANIFEST, "features").expect("[features] table must exist");

    for required in ["default", "serde", "maelstrom"] {
        let declared = features
            .lines()
            .map(str::trim)
            .any(|line| line.starts_with(required) && line.contains('='));
        assert!(
            declared,
            "[features] must declare `{required}`; found table body:\n{features}"
        );
    }
}

#[test]
fn crate_root_denies_unsafe_code() {
    assert!(
        LIB_RS.contains("#![deny(unsafe_code)]"),
        "src/lib.rs must carry #![deny(unsafe_code)]; `deny` not `forbid` so that \
         `observe` and the future `ffi` module may each carry a scoped, documented \
         #[allow(unsafe_code)] (decision B1)"
    );
    assert!(
        LIB_RS.contains("#![deny(missing_docs)]"),
        "src/lib.rs must carry #![deny(missing_docs)]; every public item states its \
         intent so the intent outlives the code"
    );
}
