//! The conformance host's gates: the transport of
//! `docs/uvrr-host-compliance.md` §8, proven rather than demonstrated.
//!
//! The host lives in the library behind the `conformance_host` feature and
//! ships as the `uvrr-conformance` binary; this target boots it in process
//! and holds it to three gates.
//!
//! # The three gates
//!
//! 1. **Equivalence.** Every case replayed over HTTP returns `pass`, and
//!    the capture served over the wire equals the capture the in-process
//!    runner derives, byte for byte. A divergence between the two runners
//!    is itself a finding: it would mean the transport, the codec, or one
//!    of the runners had grown an opinion of its own.
//! 2. **Drift.** The committed Hurl suite equals the suite generated from
//!    the committed corpus, the same gate the corpus itself carries, so
//!    the suite cannot replay a case which no longer exists.
//! 3. **The client.** The generated suite passes against a live host,
//!    driven by the `hurl` binary as an external process: the transport
//!    is exercised by a client that is not this crate.
//!
//! # Why the corpus is replay-only
//!
//! The host compares its own capture against the `expect` the request
//! carries and regenerates nothing. A codec divergence therefore surfaces
//! as a failed case with a named mismatch, never as a rewritten fixture.

// The corpus reader and the host's JSON codec are both behind the feature,
// exactly as the reference runner is: with the feature off this target
// compiles to nothing, no spawn paths and no guards.
#![cfg(feature = "conformance_host")]

#[path = "compliance_runner.rs"]
mod compliance_runner;

use compliance_runner as suite;

use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use uvrr::conformance::corpus::corpus;
use uvrr::conformance::{Case, Executor, Expect, Host};

use serde_json::json;

// ----------------------------------------------------------------------
// Booting the host
// ----------------------------------------------------------------------

/// Runs the host on this thread and `drive` on a worker thread against
/// it, returning whatever `drive` produced.
///
/// The host is not `Send`: the harness holds the boot gate's operation
/// log in an `Rc`, and a cluster of replicas behind it is not a thing to
/// move across a thread boundary. So the host stays here, the client
/// moves, and the worker wakes the listener with one connection when it
/// is done.
fn with_host<F, R>(drive: F) -> R
where
    F: FnOnce(&str) -> R + Send + 'static,
    R: Send + 'static,
{
    // Port zero: the kernel names the port, so two test targets running
    // in parallel never collide on a fixed one.
    let listener = TcpListener::bind("127.0.0.1:0").expect("the loopback listener binds");
    let address = listener
        .local_addr()
        .expect("the bound address is known")
        .to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_address = address.clone();
    let worker = std::thread::spawn(move || {
        // The stop flag is raised whether the client finishes or panics:
        // a client that dies mid-request must not leave the host blocked
        // in `accept` forever, which is how a failing gate would
        // otherwise present as a hung test rather than as its own message.
        let output =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drive(&worker_address)));
        worker_stop.store(true, Ordering::SeqCst);
        // The listener is blocked in `accept`; this connection wakes it so
        // the stop flag is observed without a poll or a sleep.
        let _ = TcpStream::connect(&worker_address);
        output
    });
    let mut host = Host::new();
    uvrr::conformance::serve(listener, move |request| host.route(request), stop);
    match worker.join().expect("the client thread finishes") {
        Ok(output) => output,
        // The panic is resumed here so it names the gate that failed.
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// The Hurl binary on `PATH`, or `None`. The absence of the client is not
/// a pass: the caller states it loudly and both the pre-push gate and CI
/// require the binary, so a push that never drove the suite over the wire
/// cannot happen.
fn hurl_binary() -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join("hurl"))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
}

/// POSTs one body to the host and returns the response's status and JSON.
fn method(address: &str, verb: &str, path: &str, body: &str) -> (u16, serde_json::Value) {
    let (status, text) = uvrr::conformance::server::request(address, verb, path, body)
        .unwrap_or_else(|error| panic!("the request to {path} completes: {error}"));
    let value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("the response to {path} is JSON: {error}\n{text}"));
    (status, value)
}

/// One family of the corpus, by name.
fn family(name: &str) -> Vec<Case> {
    corpus()
        .into_iter()
        .find(|(family, _)| family == name)
        .map(|(_, cases)| cases)
        .unwrap_or_else(|| panic!("the corpus holds the {name} family"))
}

// ----------------------------------------------------------------------
// The equivalence gate
// ----------------------------------------------------------------------

/// Every case over the transport, against the in-process runner: the
/// verdict is `pass` and the served capture is the in-process capture.
#[test]
fn the_http_runner_agrees_with_the_in_process_runner_on_every_case() {
    let corpus = corpus();
    let total = corpus.iter().map(|(_, cases)| cases.len()).sum::<usize>();
    let failures = with_host(move |address| {
        let mut failures = Vec::new();
        let mut agreed = 0usize;
        for (family, cases) in &corpus {
            assert_eq!(
                family,
                &cases
                    .first()
                    .map(|case| case.family.clone())
                    .unwrap_or_default(),
                "{family}: the case names its family file"
            );
            for case in cases {
                let in_process = Executor::run_case(case).unwrap_or_else(|error| {
                    panic!("{}: the case refused to run: {error}", case.id)
                });
                let body = serde_json::to_string(case).expect("a case serialises");
                let (status, response) = method(address, "POST", "/case", &body);
                if status != 200 {
                    failures.push(format!("{}: the host answered {status}", case.id));
                    continue;
                }
                if response["verdict"] != json!("pass") {
                    let mismatch = response["mismatch"]
                        .as_str()
                        .unwrap_or("no mismatch named")
                        .to_string();
                    failures.push(format!("{}: the verdict is not pass: {mismatch}", case.id));
                    continue;
                }
                // The byte-for-byte comparison: the transport adds
                // nothing to the capture and drops nothing from it.
                let captured: Expect = serde_json::from_value(response["expect"].clone())
                    .unwrap_or_else(|error| {
                        panic!("{}: the served capture is an Expect: {error}", case.id)
                    });
                if serde_json::to_string(&captured).unwrap_or_default()
                    != serde_json::to_string(&in_process).unwrap_or_default()
                {
                    failures.push(format!(
                        "{}: the served capture differs from the in-process capture",
                        case.id
                    ));
                    continue;
                }
                agreed += 1;
            }
        }
        (agreed, failures)
    });

    let (agreed, failures) = failures;
    assert!(
        failures.is_empty(),
        "every case must return pass over the transport with the served capture equal \
         to the in-process capture. Failing cases:\n{}",
        failures.join("\n")
    );
    assert_eq!(agreed, total, "every case in the corpus was served");
}

/// The session endpoints of §8.2 drive a case end to end: the setup
/// script, the input, and the settling window, one abstract host
/// operation at a time.
#[test]
fn the_session_endpoints_replay_a_case_through_the_host_interface() {
    let case = family("agreement")
        .into_iter()
        .next()
        .expect("the agreement family has a case");
    let id = case.id.clone();

    let outcome = with_host(move |address| {
        // A session opens with the provision operation's own arguments.
        let provision = json!({"op": "provision", "nodes": 3, "timeout": 3});
        let (status, opened) = method(address, "POST", "/session", &provision.to_string());
        assert_eq!(status, 200, "a session opens");
        let session = opened["session"]
            .as_str()
            .expect("the session answers its id")
            .to_string();

        // The setup script, every operation but the mandatory provision,
        // then the case's single input.
        for op in case
            .setup
            .iter()
            .skip(1)
            .chain(std::iter::once(&case.input))
        {
            let (status, applied) = method(
                address,
                "POST",
                &format!("/session/{session}/op"),
                &serde_json::to_string(op).expect("an operation serialises"),
            );
            assert_eq!(status, 200, "an operation is applied");
            assert_eq!(
                applied["ok"],
                json!(true),
                "the operation applied: {applied}"
            );
        }

        // The settling window, and the same capture the in-process
        // runner derives from the same case.
        let (status, captured) = method(address, "GET", &format!("/session/{session}/capture"), "");
        assert_eq!(status, 200, "the capture answers");
        let served: Expect = serde_json::from_value(captured["expect"].clone())
            .expect("the served capture is an Expect");
        let in_process = Executor::run_case(&case).expect("the case runs in process");
        (served, in_process)
    });

    let (served, in_process) = outcome;
    assert_eq!(
        serde_json::to_string(&served).unwrap_or_default(),
        serde_json::to_string(&in_process).unwrap_or_default(),
        "{id}: the session endpoints' capture equals the in-process capture"
    );
}

// ----------------------------------------------------------------------
// The drift gate
// ----------------------------------------------------------------------

/// The export and sync gate for the Hurl suite: the suite generated from
/// the committed corpus equals the committed suite, file by file, and the
/// suite directory holds exactly the generated families. Both failures
/// name the files and the regeneration command.
#[test]
fn the_committed_hurl_suite_equals_the_generated_suite() {
    let generated = suite::generated_suite();
    let committed = suite::committed_suite();
    let expected: Vec<&str> = generated.iter().map(|(name, _)| name.as_str()).collect();
    let declared: Vec<&str> = committed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        declared,
        expected,
        "{} does not hold exactly the generated families; a hand-written file carries no\n\
         `corpus-` prefix and the gate leaves it alone, and a generated family that is missing\n\
         fails here. Regenerate and commit the suite with\n    {REGEN}",
        suite::SUITE
    );
    for ((generated_name, generated_text), (name, text)) in generated.iter().zip(&committed) {
        assert_eq!(name, generated_name, "the families are in order");
        assert_eq!(
            text,
            generated_text,
            "{}/{} is stale: the suite generated from the committed corpus differs from the\n\
             committed bytes, so a corpus change was never re-exported. Regenerate and commit\n\
             it with\n    {REGEN}",
            suite::SUITE,
            name,
        );
    }
}

/// The command that regenerates the committed suite. Every message the
/// drift gate fails with names it, because a failure that says only what
/// differs leaves the reader to guess what to run.
const REGEN: &str = "cargo test --features conformance_host --test conformance_host -- --ignored export_hurl  \
     (make hurl-export)";

/// Regenerates the Hurl suite from the committed corpus. Ignored by
/// default: `cargo test --features conformance_host --test conformance_host
/// -- --ignored export_hurl`, which is what `make hurl-export` runs. The
/// export overwrites whatever the committed file holds, so a hand edit
/// belongs in the corpus.
#[test]
#[ignore = "the exporter writes the suite; run it deliberately"]
fn export_hurl() {
    suite::export_suite();
}

// ----------------------------------------------------------------------
// The client gate
// ----------------------------------------------------------------------

/// The generated suite, driven by the `hurl` binary against a live host:
/// the transport exercised by a client that is not this crate.
#[test]
fn the_hurl_suite_passes_against_the_host() {
    let Some(binary) = hurl_binary() else {
        // Not a pass and not a failure: the client is absent, so the lane
        // did not run. Both the pre-push gate and CI require the binary,
        // so a push or a CI lane without it cannot report green.
        eprintln!(
            "conformance host: THE HURL SUITE DID NOT RUN. `hurl` is not on PATH, so the \
             transport was not driven by an external client. Install it and re-run: \
             `cargo test --features conformance_host --test conformance_host -- --nocapture`"
        );
        return;
    };

    let report = with_host(move |address| {
        let output = Command::new(&binary)
            .args([
                "--test",
                "--variable",
                &format!("base_url=http://{address}"),
                suite::SUITE,
            ])
            .output()
            .unwrap_or_else(|error| panic!("hurl runs: {error}"));
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    });

    let (passed, stdout, stderr) = report;
    assert!(
        passed,
        "the Hurl suite must pass against the host.\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    // The suite ran, and it ran the corpus. A suite that replayed nothing
    // would pass just as loudly, so every generated family is named in
    // Hurl's own report and the executed request count is checked against
    // the case count the corpus holds.
    let mut replayed = 0usize;
    for (family, cases) in corpus() {
        let name = suite::file_of(&family);
        assert!(
            stderr.contains(&format!("Success {}/{name}", suite::SUITE)),
            "Hurl must report the {family} family as passing.\n--- stderr ---\n{stderr}"
        );
        replayed += cases.len();
    }
    let executed = stderr
        .lines()
        .find_map(|line| line.trim().strip_prefix("Executed requests: "))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|count| count.parse::<usize>().ok());
    assert!(
        executed.is_some_and(|executed| executed >= replayed),
        "Hurl must have executed at least the {replayed} requests the corpus names; \
         its summary reads:\n--- stderr ---\n{stderr}"
    );
}

/// The suite directory is where the generated files say they are, and the
/// corpus it replays is the committed corpus. The paths are stated in the
/// generated header, so a moved directory breaks the instruction rather
/// than the run.
#[test]
fn the_suite_and_the_corpus_are_where_they_are_stated() {
    assert!(
        std::path::Path::new(suite::SUITE).is_dir(),
        "the Hurl suite lives at {}",
        suite::SUITE
    );
    assert!(
        std::path::Path::new(suite::CORPUS).is_dir(),
        "the corpus the suite replays lives at {}",
        suite::CORPUS
    );
}
