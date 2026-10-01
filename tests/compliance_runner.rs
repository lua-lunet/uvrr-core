#![cfg(feature = "serde")]

//! The Hurl suite: the committed corpus replayed over the transport
//! (`docs/uvrr-host-compliance.md` §8).
//!
//! The suite is generated, and gated the way the corpus is: the exporter
//! writes the files from the committed corpus bytes, and the drift gate
//! rebuilds them in memory and compares, so a corpus change that nobody
//! re-exported fails the gate instead of leaving a suite that replays a
//! case which no longer exists. The generator reads the corpus from disk
//! and never from the in-memory families, because the suite is
//! replay-only: it may replay the committed bytes, it may not regenerate
//! them.
//!
//! One file per family, named `corpus-<family>.hurl` beside the
//! hand-written `contract.hurl`, which drives the session endpoints of
//! §8.2. The prefix keeps the two apart: the drift gate owns the files
//! it generates and says nothing about the one a hand writes.

use uvrr::conformance::corpus::Case;

use std::fs;
use std::path::{Path, PathBuf};

/// Where the generated suite lives, relative to the crate root the tests
/// run in.
pub const SUITE: &str = "tests/hurl";

/// The corpus the suite replays.
pub const CORPUS: &str = "tests/compliance/corpus";

/// The prefix the generated families carry. A hand-written file in the
/// suite directory has no prefix, and the drift gate leaves it alone.
const PREFIX: &str = "corpus-";

/// Every corpus family, with its cases, as the committed bytes parse
/// them. The same reading the in-process runner does.
pub fn corpus() -> Vec<(String, Vec<Case>)> {
    let mut files: Vec<PathBuf> = fs::read_dir(CORPUS)
        .expect("the corpus directory is present")
        .map(|entry| entry.expect("the corpus directory reads"))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("the corpus file reads");
            let family = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a corpus file is named")
                .to_string();
            let cases: Vec<Case> = serde_json::from_str(&text).expect("the corpus file parses");
            (family, cases)
        })
        .collect()
}

/// The name a family's generated file carries.
pub fn file_of(family: &str) -> String {
    format!("{PREFIX}{family}.hurl")
}

/// The header a generated file opens with: what the file is, where it
/// came from, and how to regenerate it. The generator's text is a pure
/// function of the committed corpus, so this header is part of what the
/// drift gate compares.
fn header(family: &str, cases: usize) -> String {
    format!(
        "# The `{family}` family of the uVRR compliance corpus, replayed over the\n\
         # transport (`docs/uvrr-host-compliance.md` §8): {cases} cases, each POSTed\n\
         # to the host's case endpoint with its expectation in the body, so the host\n\
         # compares its own capture against the committed bytes.\n\
         #\n\
         # Generated from `tests/compliance/corpus/{family}.json` by\n\
         # `cargo test --features conformance_host --test conformance_host -- --ignored\n\
         # export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`\n\
         # rebuilds this file in memory and compares: a corpus change that was not\n\
         # re-exported fails the gate. Edit the corpus, then re-export.\n"
    )
}

/// One hurl entry: the case as the request body, then the verdict and the
/// settling window asserted.
///
/// The `count` on the deliveries is load-bearing rather than decorative:
/// `expect.deliveries` is the exact multiset, so a count of zero asserts
/// silence and a count of six asserts that six and no more were served.
/// The verdict alone would not prove the served body carries the set.
fn entry(case: &Case, deliveries: usize) -> String {
    let body = serde_json::to_string_pretty(case).expect("a case serialises");
    format!(
        "# {}\n\
         # {}\n\
         POST {{{{base_url}}}}/case\n\
         Content-Type: application/json\n\
         {body}\n\
         HTTP 200\n\
         [Asserts]\n\
         jsonpath \"$.id\" == \"{id}\"\n\
         jsonpath \"$.family\" == \"{family}\"\n\
         jsonpath \"$.verdict\" == \"pass\"\n\
         jsonpath \"$.mismatch\" not exists\n\
         jsonpath \"$.expect.deliveries\" count == {deliveries}\n",
        case.id,
        case.clause,
        id = case.id,
        family = case.family,
    )
}

/// The generated suite: one file per family, as `(file name, text)`. The
/// text is a pure function of the committed corpus bytes.
pub fn generated_suite() -> Vec<(String, String)> {
    corpus()
        .into_iter()
        .map(|(family, cases)| {
            let count = cases.len();
            let mut text = header(&family, count);
            for case in &cases {
                text.push('\n');
                text.push_str(&entry(case, case.expect.deliveries.len()));
            }
            (file_of(&family), text)
        })
        .collect()
}

/// The generated files as committed: `(file name, text)`, read from the
/// suite directory.
pub fn committed_suite() -> Vec<(String, String)> {
    let mut files: Vec<PathBuf> = fs::read_dir(SUITE)
        .expect("the suite directory is present")
        .map(|entry| entry.expect("the suite directory reads"))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "hurl")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.starts_with(PREFIX))
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a suite file is named")
                .to_string();
            let text = fs::read_to_string(&path).expect("the suite file reads");
            (format!("{name}.hurl"), text)
        })
        .collect()
}

/// Writes the generated suite. Ignored by default: the exporter writes
/// files, so it runs deliberately and the gate asserts the result without
/// writing.
pub fn export_suite() {
    fs::create_dir_all(SUITE).expect("the suite directory creates");
    for (name, text) in generated_suite() {
        let path = Path::new(SUITE).join(&name);
        fs::write(&path, &text).expect("the suite file writes");
        eprintln!("exported {path:?} ({} bytes)", text.len());
    }
}
