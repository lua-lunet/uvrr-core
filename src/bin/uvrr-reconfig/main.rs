//! The reconfiguration operator CLI: compute a plan, or submit one to a
//! leader's admin port. `docs/weighted-reconfiguration-solver.md` records the
//! interface; the JSONL schema it speaks is defined by `vrr::plan`.
use std::io::Read;
use std::net::UdpSocket;
use std::time::Duration;

use clap::{Parser, Subcommand};
use vrr::configuration::{Configuration, Member, Snapshot, Weight};
use vrr::ids::{Era, NodeId};
use vrr::plan::Plan;
use vrr::solver::{solve, solve_replacement};

/// The plan payload budget: one UDP datagram must carry the whole submission,
/// header line included, and the leader's receive buffer is provisioned for
/// 64 KiB.
const MAX_PLAN_BYTES: usize = 60_000;

/// The leader response wait: one retry window of the protocol's own timeouts,
/// after which the submission is unanswered and the operator must look at the
/// leader before resubmitting.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Parser)]
#[command(
    name = "uvrr-reconfig",
    about = "Compute or submit a weighted reconfiguration plan"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compute a reconfiguration plan and print it as plan JSONL.
    Plan {
        /// The current committed configuration, JSONL, one {"id":N,"weight":W}
        /// per line; `-` reads standard input.
        #[arg(long)]
        current: String,
        /// The target configuration, in the same JSONL form; conflicts with
        /// `--replace`.
        #[arg(long, group = "goal")]
        target: Option<String>,
        /// Replace one incarnation: OLD:NEW, retaining weight and succession
        /// position; conflicts with `--target`.
        #[arg(long, group = "goal")]
        replace: Option<String>,
        /// The available identities, comma-separated.
        #[arg(long)]
        available: String,
        /// Write the plan JSONL here instead of standard output.
        #[arg(long)]
        out: Option<String>,
    },
    /// Submit a plan to a leader's admin port as one UDP datagram.
    Apply {
        /// The plan JSONL; `-` reads standard input.
        #[arg(long)]
        plan: String,
        /// The leader's admin endpoint, host:port.
        #[arg(long)]
        leader: String,
    },
}

fn read_source(source: &str) -> Result<String, String> {
    if source == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("cannot read standard input: {error}"))?;
        Ok(text)
    } else {
        std::fs::read_to_string(source).map_err(|error| format!("cannot read {source}: {error}"))
    }
}

fn write_sink(sink: Option<&String>, text: &str) -> Result<(), String> {
    match sink {
        None => print!("{text}"),
        Some(path) => {
            std::fs::write(path, text).map_err(|error| format!("cannot write {path}: {error}"))?
        }
    }
    Ok(())
}

/// The current-configuration file: one `{"id":N,"weight":W}` per line, parsed
/// once at this perimeter into the checked core configuration.
fn configuration(text: &str) -> Result<Configuration, String> {
    #[derive(serde::Deserialize)]
    struct ConfigMember {
        id: u32,
        weight: u32,
    }
    let mut order = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let member: ConfigMember = serde_json::from_str(line).map_err(|error| {
            format!(
                "line {}: expected {{\"id\":N,\"weight\":W}}: {error}",
                number + 1
            )
        })?;
        order.push(Member {
            node: NodeId(member.id),
            weight: Weight(member.weight),
        });
    }
    Snapshot { era: Era(1), order }
        .inflate()
        .map_err(|error| format!("the configuration is not legal: {error:?}"))
}

fn available_ids(text: &str) -> Result<Vec<NodeId>, String> {
    text.split(',')
        .map(|field| {
            field
                .trim()
                .parse::<u32>()
                .map(NodeId)
                .map_err(|error| format!("available identities are comma-separated u32s: {error}"))
        })
        .collect()
}

fn run_plan(
    current: String,
    target: Option<String>,
    replace: Option<String>,
    available: String,
    out: Option<String>,
) -> Result<(), String> {
    let current = configuration(&read_source(&current)?)?;
    let live = available_ids(&available)?;
    let steps = match (target, replace) {
        (Some(target), None) => {
            let target = configuration(&read_source(&target)?)?;
            solve(&current, &target, &live)
        }
        (None, Some(replace)) => {
            let (old, new) = replace
                .split_once(':')
                .ok_or("expected --replace OLD:NEW")?;
            let (old, new) = match (old.trim().parse::<u32>(), new.trim().parse::<u32>()) {
                (Ok(old), Ok(new)) => (NodeId(old), NodeId(new)),
                _ => return Err("expected --replace OLD:NEW with u32 identities".to_string()),
            };
            solve_replacement(&current, old, new, &live)
        }
        _ => return Err("exactly one of --target or --replace is required".to_string()),
    }
    .map_err(|error| error.to_string())?;
    let plan = Plan {
        initial: current.order().to_vec(),
        steps: steps.into_iter().map(|step| step.ops).collect(),
    };
    let jsonl = plan
        .to_jsonl()
        .map_err(|error| format!("the plan cannot be serialised: {error}"))?;
    write_sink(out.as_ref(), &jsonl)
}

fn run_apply(plan: String, leader: String) -> Result<(), String> {
    let text = read_source(&plan)?;
    Plan::from_jsonl(&text).map_err(|error| format!("the plan is refused: {error}"))?;
    let header = "{\"kind\":\"plan_submit\",\"version\":1}";
    let mut payload = String::with_capacity(text.len() + header.len() + 2);
    payload.push_str(header);
    payload.push('\n');
    payload.push_str(text.trim_end());
    payload.push('\n');
    if payload.len() > MAX_PLAN_BYTES {
        return Err(format!(
            "the serialised plan is {} bytes; the datagram budget is {MAX_PLAN_BYTES}",
            payload.len()
        ));
    }
    let socket =
        UdpSocket::bind("0.0.0.0:0").map_err(|error| format!("cannot open a socket: {error}"))?;
    socket
        .set_read_timeout(Some(RESPONSE_TIMEOUT))
        .map_err(|error| format!("cannot arm the response timeout: {error}"))?;
    socket
        .send_to(payload.as_bytes(), &leader)
        .map_err(|error| format!("cannot send to {leader}: {error}"))?;
    let mut buffer = vec![0u8; 65_536];
    let (length, _) = socket
        .recv_from(&mut buffer)
        .map_err(|error| format!("no leader response: {error}"))?;
    let response: serde_json::Value = serde_json::from_slice(&buffer[..length])
        .map_err(|error| format!("the leader response is not JSON: {error}"))?;
    match response.get("verdict").and_then(|verdict| verdict.as_str()) {
        Some("accepted") => {
            println!("accepted");
            Ok(())
        }
        Some("rejected") => {
            let reason = response
                .get("reason")
                .and_then(|reason| reason.as_str())
                .unwrap_or("unspecified");
            eprintln!("rejected: {reason}");
            std::process::exit(1);
        }
        _ => Err(format!("the leader response is not a verdict: {response}")),
    }
}

fn main() {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Plan {
            current,
            target,
            replace,
            available,
            out,
        } => run_plan(current, target, replace, available, out),
        Command::Apply { plan, leader } => run_apply(plan, leader),
    };
    if let Err(error) = outcome {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
