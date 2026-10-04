//! uvrr-solve: the offline reconfiguration solver.
//!
//! Reads two configuration files (current, target) and prints the legal
//! operation path between them as a per-slot plan: the era and the view each
//! slot establishes, and the operations the slot carries, with the `Nominate`
//! rider minted where the leader must be held stable. Unreachable targets
//! print `unreachable` and exit non-zero: `None` is a proof, not an error.
//!
//! File format, one record per line:
//!   era <u32> view <u32>      the deployment's running era and view
//!   node <u32> <u32>          a member and its weight, in succession order
//!
//! The solver is the library's own: this binary is the administrator's
//! offline handle on exactly the code the cluster runs.

use std::process::ExitCode;
use uvrr::configuration::{Configuration, INIT_SLOT, SystemOperation};
use uvrr::ids::{Era, NodeId, Slot, View};
use uvrr::solve;

fn parse(path: &str) -> Result<(Configuration, Era, View), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut era = None;
    let mut view = None;
    let mut members = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            [] | ["#", ..] => {}
            ["era", e, "view", v] => {
                era = Some(e.parse().map_err(|_| format!("line {}: bad era", n + 1))?);
                view = Some(v.parse().map_err(|_| format!("line {}: bad view", n + 1))?);
            }
            ["era", e] => era = Some(e.parse().map_err(|_| format!("line {}: bad era", n + 1))?),
            ["view", v] => view = Some(v.parse().map_err(|_| format!("line {}: bad view", n + 1))?),
            ["node", id, w] => members.push((
                NodeId(
                    id.parse()
                        .map_err(|_| format!("line {}: bad node", n + 1))?,
                ),
                w.parse::<u32>()
                    .map_err(|_| format!("line {}: bad weight", n + 1))?,
            )),
            _ => return Err(format!("line {}: unrecognised record", n + 1)),
        }
    }
    if members.is_empty() {
        return Err(format!("{path}: no members"));
    }
    let order: Vec<NodeId> = members.iter().map(|(node, _)| *node).collect();
    let mut config = Configuration::void()
        .apply(&SystemOperation::Init { order }, INIT_SLOT)
        .map_err(|e| format!("{path}: init refused: {e:?}"))?;
    for (node, weight) in &members {
        loop {
            let have = config.weight_of(*node).expect("the member exists").0;
            if have == *weight {
                break;
            }
            let op = if have < *weight {
                SystemOperation::Increment(*node)
            } else {
                SystemOperation::Decrement(*node)
            };
            config = config
                .apply(&op, Slot(0))
                .map_err(|e| format!("{path}: shaping refused: {e:?}"))?;
        }
    }
    Ok((
        config,
        Era(era.ok_or("the era line is missing")?),
        View(view.ok_or("the view line is missing")?),
    ))
}

fn render(op: &SystemOperation) -> String {
    match op {
        SystemOperation::Void => "void".to_string(),
        SystemOperation::Init { .. } => "init".to_string(),
        SystemOperation::Increment(node) => format!("increment {}", node.0),
        SystemOperation::Decrement(node) => format!("decrement {}", node.0),
        SystemOperation::Double => "double".to_string(),
        SystemOperation::Halve => "halve".to_string(),
        SystemOperation::Join { node, position } => format!("join {} at {}", node.0, position),
        SystemOperation::Leave(node) => format!("leave {}", node.0),
        SystemOperation::Nominate { from, offset } => format!("nominate {} +{}", from.0, offset),
        SystemOperation::Batch(ops) => format!("batch[{}]", ops.len()),
    }
}

fn run(args: &[String]) -> ExitCode {
    let [_, current_path, target_path, slot_text] = args else {
        eprintln!("usage: uvrr-solve <current-file> <target-file> <first-slot>");
        return ExitCode::FAILURE;
    };
    let ((current, era, view), target) = match (parse(current_path), parse(target_path)) {
        (Ok(current), Ok((target, _, _))) => (current, target),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let slot = match slot_text.parse() {
        Ok(slot) => Slot(slot),
        Err(_) => {
            eprintln!(
                "usage: uvrr-solve <current-file> <target-file> <first-slot>: the slot must be a u64"
            );
            return ExitCode::FAILURE;
        }
    };
    match solve::solve(&current, &target)
        .and_then(|path| solve::fold(&path, &current, (era, view, slot)))
    {
        Some(steps) => {
            for step in steps {
                let ops: Vec<String> = step.ops.iter().map(render).collect();
                println!(
                    "slot {} era {} view {} ops {}",
                    step.slot.0,
                    step.era.0,
                    step.view.0,
                    ops.join(",")
                );
            }
            ExitCode::SUCCESS
        }
        None => {
            println!("unreachable");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    run(&args)
}
