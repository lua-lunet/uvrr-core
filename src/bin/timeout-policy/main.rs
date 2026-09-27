//! The flavoured-timeout policy tool: living documentation for the
//! matcher's model. Given a state and a timeout flavour on the command
//! line, it prints the opinion the matcher returns, and for the Sorry
//! opinions it prints the runbook statement the type carries. Given
//! anything else, it prints the two enums it accepts and exits nonzero:
//! the tool's own usage is the enumeration of its domain, so a new variant
//! cannot enter the library without the tool naming it here.
//!
//! Std only: the policy model is the library's own, the tool needs nothing
//! else, and a default-features consumer never builds it unless asked.

use uvrr::timeout::{Opinion, State, Timeout, matcher};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(state_name), Some(timeout_name), None) = (args.next(), args.next(), args.next())
    else {
        usage();
        std::process::exit(2);
    };
    let Some(state) = State::ALL.iter().find(|s| s.name() == state_name) else {
        usage();
        std::process::exit(2);
    };
    let Some(timeout) = Timeout::ALL.iter().find(|t| t.name() == timeout_name) else {
        usage();
        std::process::exit(2);
    };
    match matcher(*state, *timeout) {
        Opinion::Sorry { runbook } => {
            println!("sorry");
            println!("{runbook}");
        }
        opinion => println!("{}", opinion.name()),
    }
}

/// The tool's usage: the two enums it accepts, named, one per line, so the
/// usage is the enumeration of the domain and nothing else.
fn usage() {
    eprintln!("usage: timeout-policy <state> <timeout>");
    eprintln!("states:");
    for state in State::ALL {
        eprintln!("  {}", state.name());
    }
    eprintln!("timeouts:");
    for timeout in Timeout::ALL {
        eprintln!("  {}", timeout.name());
    }
}
