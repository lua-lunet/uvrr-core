//! `uvrr-hostid`: the sysadmin's identity tool for the identity law. Three
//! verbs, all std-only and all offline:
//!
//! - `uvrr-hostid init <system> <crash>` validates the pair (both non-zero,
//!   both u16) and prints the padded form and the packed u32 to burn into the
//!   boot marker before first boot.
//! - `uvrr-hostid print <node-id>` decodes a packed u32 to the padded pair.
//! - `uvrr-hostid bump <node-id>` prints the next life: the counter
//!   incremented, the value the host must flush before the first emission.
//!
//! Refusals exit non-zero with the reason on stderr; the tool never writes
//! any file, because the marker write is the host's own boot-gate act.

use std::process::ExitCode;

use vrr::ids::{CrashCounter, NodeId, SystemId};

const USAGE: &str = "usage: uvrr-hostid init <system> <crash> | print <node-id> | bump <node-id>";

fn parse_u16(word: Option<&String>, what: &str) -> Result<u16, String> {
    let word = word.ok_or_else(|| format!("missing {what}\n{USAGE}"))?;
    let value: u16 = word
        .parse()
        .map_err(|_| format!("{what} is not a u16: {word}\n{USAGE}"))?;
    if value == 0 {
        return Err(format!("{what} is one-indexed and never zero\n{USAGE}"));
    }
    Ok(value)
}

fn parse_node(word: Option<&String>) -> Result<NodeId, String> {
    let word = word.ok_or_else(|| format!("missing node-id\n{USAGE}"))?;
    let value: u32 = word
        .parse()
        .map_err(|_| format!("node-id is not a u32: {word}\n{USAGE}"))?;
    Ok(NodeId::from(value))
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("init") => {
            let system =
                SystemId::new(parse_u16(args.get(1), "system")?).expect("non-zero was checked");
            let crash =
                CrashCounter::new(parse_u16(args.get(2), "crash")?).expect("non-zero was checked");
            let id = NodeId::new(system, crash);
            println!("{id} {}", u32::from(id));
            Ok(())
        }
        Some("print") => {
            let id = parse_node(args.get(1))?;
            println!("{id}");
            if !id.is_lawful() {
                eprintln!("warning: a zero half is no lawful identity");
            }
            Ok(())
        }
        Some("bump") => {
            let id = parse_node(args.get(1))?;
            let next = id.next_life().ok_or_else(|| {
                "no next life: not a lawful identity, or the counter space is exhausted".to_string()
            })?;
            println!("{next} {}", u32::from(next));
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(2)
        }
    }
}
