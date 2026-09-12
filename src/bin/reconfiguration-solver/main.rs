//! Command-line interface to the weighted-majority operator solver.
use vrr::configuration::{Member, Snapshot, Weight};
use vrr::ids::{Era, NodeId};
use vrr::solver::{solve, solve_replacement};

fn configuration(s: &str) -> Result<vrr::configuration::Configuration, String> {
    let order = s
        .split(',')
        .map(|field| {
            let (id, weight) = field.split_once(':').ok_or("expected identity:weight")?;
            Ok(Member {
                node: NodeId(id.parse::<u32>().map_err(|e| e.to_string())?),
                weight: Weight(weight.parse::<u32>().map_err(|e| e.to_string())?),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Snapshot { era: Era(1), order }
        .inflate()
        .map_err(|e| format!("{e:?}"))
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 && args.len() != 4 {
        return Err("usage: reconfiguration-solver CURRENT TARGET AVAILABLE\n   or: reconfiguration-solver CURRENT --replace OLD:NEW AVAILABLE\nconfigs: 0:1,1:2,2:1; available: 0,1,2; identities are u32".into());
    }
    let current = configuration(&args[0])?;
    let live = args
        .last()
        .unwrap()
        .split(',')
        .map(|n| n.parse::<u32>().map(NodeId).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let steps = if args.len() == 4 && args[1] == "--replace" {
        let (old, new) = args[2].split_once(':').ok_or("expected OLD:NEW")?;
        solve_replacement(
            &current,
            NodeId(old.parse::<u32>().map_err(|e| e.to_string())?),
            NodeId(new.parse::<u32>().map_err(|e| e.to_string())?),
            &live,
        )
    } else if args.len() == 3 {
        solve(&current, &configuration(&args[1])?, &live)
    } else {
        return Err("expected --replace".into());
    }
    .map_err(|e| e.to_string())?;
    println!("Strict weighted-majority plan; commit each batch in sequence.");
    println!(
        "Acquire state before promotion; use a quorum-backed view change to select a live positive-weight leader."
    );
    for step in steps {
        let weights = step
            .config
            .order()
            .iter()
            .map(|m| format!("{}:{}", m.node.0, m.weight.0))
            .collect::<Vec<_>>()
            .join(",");
        let mass: u64 = step
            .config
            .order()
            .iter()
            .filter(|m| live.contains(&m.node))
            .map(|m| u64::from(m.weight.0))
            .sum();
        println!(
            "era {}: {:?} => {weights}; available {mass}/{}",
            step.config.era().0,
            step.ops,
            step.config.total()
        );
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
