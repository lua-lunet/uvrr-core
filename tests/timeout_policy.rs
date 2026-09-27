//! The flavoured-timeout model pinned: the state enum and the timeout enum
//! are closed, the matcher is exhaustive over every pair with no default
//! arm (the compiler proves the 64 pairs are all named), and the policy tool
//! prints exactly the opinion the matcher returns, Sorry runbook included.
//!
//! The 64-pair table below is not hand-picked examples: the domain is small
//! and closed, so the pin is the whole domain, and a new variant cannot
//! enter either enum without this table failing to compile or to pass.

use uvrr::timeout::{Opinion, State, Timeout, UNFLUSHED_RUNBOOK, matcher};

/// The matcher over the full domain, pinned pair by pair.
#[test]
fn the_matcher_pins_every_pair_of_the_closed_domain() {
    use Opinion::{DoNothing, Heartbeat, Retransmit, Sorry, StartViewChange};
    use State::{
        Booted, Crashed, InTheCluster, Steady, Stopping, StoppingNotFlushed, Unknown, Witness,
    };
    use Timeout::{
        Booted as BootedWait, Cluster, Crashed as CrashedWait, Steady as SteadyWait,
        Stopping as StoppingWait, StoppingNotFlushed as UnflushedWait, Unknown as UnknownWait,
        Witness as WitnessWait,
    };
    let sorry = || Sorry {
        runbook: UNFLUSHED_RUNBOOK,
    };
    // The rule, stated as the full table: the unflushed stop answers Sorry
    // whatever fired on it; the non-agents do nothing whatever fired on them;
    // the agents answer the flavour they met, and a foreign non-agent
    // flavour on an agent does nothing.
    let expected: [((State, Timeout), Opinion); 64] = [
        ((InTheCluster, Cluster), Retransmit),
        ((InTheCluster, WitnessWait), DoNothing),
        ((InTheCluster, UnknownWait), DoNothing),
        ((InTheCluster, BootedWait), DoNothing),
        ((InTheCluster, CrashedWait), DoNothing),
        ((InTheCluster, SteadyWait), Heartbeat),
        ((InTheCluster, StoppingWait), DoNothing),
        ((InTheCluster, UnflushedWait), DoNothing),
        ((Witness, Cluster), DoNothing),
        ((Witness, WitnessWait), DoNothing),
        ((Witness, UnknownWait), DoNothing),
        ((Witness, BootedWait), DoNothing),
        ((Witness, CrashedWait), DoNothing),
        ((Witness, SteadyWait), DoNothing),
        ((Witness, StoppingWait), DoNothing),
        ((Witness, UnflushedWait), DoNothing),
        ((Unknown, Cluster), DoNothing),
        ((Unknown, WitnessWait), DoNothing),
        ((Unknown, UnknownWait), DoNothing),
        ((Unknown, BootedWait), DoNothing),
        ((Unknown, CrashedWait), DoNothing),
        ((Unknown, SteadyWait), DoNothing),
        ((Unknown, StoppingWait), DoNothing),
        ((Unknown, UnflushedWait), DoNothing),
        ((Booted, Cluster), DoNothing),
        ((Booted, WitnessWait), DoNothing),
        ((Booted, UnknownWait), DoNothing),
        ((Booted, BootedWait), DoNothing),
        ((Booted, CrashedWait), DoNothing),
        ((Booted, SteadyWait), DoNothing),
        ((Booted, StoppingWait), DoNothing),
        ((Booted, UnflushedWait), DoNothing),
        ((Crashed, Cluster), DoNothing),
        ((Crashed, WitnessWait), DoNothing),
        ((Crashed, UnknownWait), DoNothing),
        ((Crashed, BootedWait), DoNothing),
        ((Crashed, CrashedWait), DoNothing),
        ((Crashed, SteadyWait), DoNothing),
        ((Crashed, StoppingWait), DoNothing),
        ((Crashed, UnflushedWait), DoNothing),
        ((Steady, Cluster), Retransmit),
        ((Steady, WitnessWait), DoNothing),
        ((Steady, UnknownWait), DoNothing),
        ((Steady, BootedWait), DoNothing),
        ((Steady, CrashedWait), DoNothing),
        ((Steady, SteadyWait), StartViewChange),
        ((Steady, StoppingWait), DoNothing),
        ((Steady, UnflushedWait), DoNothing),
        ((Stopping, Cluster), DoNothing),
        ((Stopping, WitnessWait), DoNothing),
        ((Stopping, UnknownWait), DoNothing),
        ((Stopping, BootedWait), DoNothing),
        ((Stopping, CrashedWait), DoNothing),
        ((Stopping, SteadyWait), DoNothing),
        ((Stopping, StoppingWait), DoNothing),
        ((Stopping, UnflushedWait), DoNothing),
        ((StoppingNotFlushed, Cluster), sorry()),
        ((StoppingNotFlushed, WitnessWait), sorry()),
        ((StoppingNotFlushed, UnknownWait), sorry()),
        ((StoppingNotFlushed, BootedWait), sorry()),
        ((StoppingNotFlushed, CrashedWait), sorry()),
        ((StoppingNotFlushed, SteadyWait), sorry()),
        ((StoppingNotFlushed, StoppingWait), sorry()),
        ((StoppingNotFlushed, UnflushedWait), sorry()),
    ];
    for ((state, timeout), opinion) in expected {
        assert_eq!(
            matcher(state, timeout),
            opinion,
            "the opinion for {state:?} at {timeout:?}"
        );
    }
    // The domain is closed and the table is the whole of it.
    assert_eq!(State::ALL.len() * Timeout::ALL.len(), expected.len());
}

/// The Sorry runbook is the recorded statement, not a guess: the exact text
/// a node stopping but not flushed carries, stated once beside the matcher.
#[test]
fn the_sorry_runbook_is_the_recorded_statement() {
    let Opinion::Sorry { runbook } =
        matcher(State::StoppingNotFlushed, Timeout::StoppingNotFlushed)
    else {
        panic!("the unflushed stop answers Sorry");
    };
    assert_eq!(runbook, UNFLUSHED_RUNBOOK);
    assert!(runbook.contains("freeing disk space and retrying the flush"));
    assert!(runbook.contains("a policy we do not comprehend and will never decide"));
}

/// The policy tool prints the matcher's opinion, and the Sorry opinion
/// prints the runbook: the tool is living documentation, and what it prints
/// is what the library returns.
#[test]
fn the_tool_prints_the_matchers_opinions() {
    let tool = env!("CARGO_BIN_EXE_timeout-policy");
    let run = |state: &str, timeout: &str| {
        let out = std::process::Command::new(tool)
            .args([state, timeout])
            .output()
            .expect("the tool runs");
        assert!(out.status.success(), "{state} {timeout}: {out:?}");
        String::from_utf8(out.stdout).expect("the opinion is UTF-8")
    };
    assert_eq!(run("in-the-cluster", "cluster").trim(), "retransmit");
    assert_eq!(run("in-the-cluster", "steady").trim(), "heartbeat");
    assert_eq!(run("steady", "steady").trim(), "start-view-change");
    assert_eq!(run("witness", "cluster").trim(), "do-nothing");
    let sorry = run("stopping-not-flushed", "cluster");
    let mut lines = sorry.lines();
    assert_eq!(lines.next(), Some("sorry"));
    assert_eq!(lines.next(), Some(UNFLUSHED_RUNBOOK));
}

/// The tool's usage enumerates the domain and refuses the unknown: a new
/// variant cannot enter the enums without the tool naming it, and a
/// nonsense argument never reaches the matcher.
#[test]
fn the_tools_usage_enumerates_the_domain_and_refuses_the_unknown() {
    let tool = env!("CARGO_BIN_EXE_timeout-policy");
    let out = std::process::Command::new(tool)
        .arg("nonsense")
        .output()
        .expect("the tool runs");
    assert!(!out.status.success());
    let usage = String::from_utf8(out.stderr).expect("the usage is UTF-8");
    for state in State::ALL {
        assert!(
            usage.contains(state.name()),
            "the usage names the state {}: {usage}",
            state.name()
        );
    }
    for timeout in Timeout::ALL {
        assert!(
            usage.contains(timeout.name()),
            "the usage names the timeout {}: {usage}",
            timeout.name()
        );
    }
}
