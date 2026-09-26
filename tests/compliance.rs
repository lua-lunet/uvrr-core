//! The compliance suite's reference runner and exporter
//! (`docs/uvrr-compliance.md`). The runner replays every case in the
//! corpus through the abstract host interface and asserts the named
//! expectations; the exporter regenerates the corpus from the same
//! execution path, so the committed bytes and the codec cannot drift.

#![cfg(feature = "serde")]

#[path = "harness/mod.rs"]
mod harness;
#[path = "compliance/mod.rs"]
mod compliance;

use std::fs;
use std::path::Path;

use compliance::{Case, Op, assert_expectation};

/// Where the corpus lives.
const CORPUS: &str = "tests/compliance/corpus";

/// Reads every corpus file.
fn corpus() -> Vec<(String, Vec<Case>)> {
    let mut files: Vec<_> = fs::read_dir(CORPUS)
        .expect("the corpus directory is present")
        .map(|entry| entry.expect("the corpus directory reads"))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("the corpus file reads");
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .expect("a corpus file is named")
                .to_string();
            let cases: Vec<Case> =
                serde_json::from_str(&text).expect("the corpus file parses");
            (name, cases)
        })
        .collect()
}

#[test]
fn the_corpus_passes() {
    let corpus = corpus();
    assert!(!corpus.is_empty(), "the corpus has families");
    for (family, cases) in &corpus {
        assert!(!cases.is_empty(), "{family} has cases");
        for case in cases {
            let captured = compliance::Executor::run_case(case).unwrap_or_else(|e| {
                panic!("{}: {}: the case refused to run: {e}", case.id, case.clause)
            });
            if let Err(mismatch) = assert_expectation(case, &captured) {
                panic!(
                    "{}: {}: {mismatch}\ncaptured: {:#?}",
                    case.id, case.clause, captured
                );
            }
        }
    }
}

/// Regenerates the corpus from the reference behaviour. Ignored by
/// default: `cargo test --all-features --test compliance -- --ignored
/// export_corpus`. The regenerated files MUST equal the committed files.
#[test]
#[ignore = "the exporter writes the corpus; run it deliberately"]
fn export_corpus() {
    for (name, cases) in exporter_families() {
        let path = Path::new(CORPUS).join(format!("{name}.json"));
        let text = serde_json::to_string_pretty(&cases).expect("the corpus serializes");
        fs::write(&path, format!("{text}\n")).expect("the corpus file writes");
        eprintln!("exported {path} ({} cases)", cases.len());
    }
}

/// The families' case skeletons: the setup and input, with the clause
/// each case pins. The exporter executes each and captures the
/// expectation.
fn exporter_families() -> Vec<(String, Vec<Case>)> {
    vec![
        ("prepare", prepare_family()),
        ("host-fence", fence_family()),
        ("host-shutdown", shutdown_family()),
        ("host-crash", crash_family()),
        ("gossip-witness", gossip_family()),
    ]
}

// ----------------------------------------------------------------------
// The prepare family
// ----------------------------------------------------------------------

const TIMEOUT: u64 = 3;
const NODES: usize = 3;
const RECEIVER: usize = 2;

/// The relation of one numeric field against the receiver's own value.
#[derive(Clone, Copy)]
enum Rel {
    Less,
    Equal,
    Greater,
}

const REL_ALL: [Rel; 3] = [Rel::Less, Rel::Equal, Rel::Greater];

fn rel(r: Rel, base: u64) -> u64 {
    match r {
        Rel::Less => base - 1,
        Rel::Equal => base,
        Rel::Greater => base + 1,
    }
}

/// The receiver's assembled boot status.
#[derive(Clone, Copy)]
enum Boot {
    Normal,
    Restarting,
}

const BOOT_ALL: [Boot; 2] = [Boot::Normal, Boot::Restarting];

/// The setup that reaches the receiver at view 1, every node `Normal`.
fn view_one_setup() -> Vec<Op> {
    let mut setup = vec![
        Op::Provision {
            nodes: NODES,
            timeout: TIMEOUT,
        },
        Op::TickAll,
        Op::DeliverAll,
    ];
    for _ in 0..=TIMEOUT {
        setup.push(Op::Tick { node: 1 });
    }
    setup.push(Op::DeliverAll);
    setup
}

fn prepare_clause(v: &str, s: &str, p: &str) -> String {
    format!(
        "A backup MUST take exactly one route on a Prepare whose view is {v} \
         its own, whose offered slot is {s} the successor of its accepted \
         frontier, and whose piggybacked commit frontier is {p} its committed \
         frontier: a mismatch drops, a retransmit re-answers without moving \
         a frontier, an accept journals and answers, a piggyback claiming \
         the arriving slot is refused by name, a gap holds the frontier and \
         fetches, and a higher view fences the node"
    )
}

fn prepare_family() -> Vec<Case> {
    let mut seq = 1;
    let mut cases = Vec::new();
    for boot in BOOT_ALL {
        for v in REL_ALL {
            for s in REL_ALL {
                for p in REL_ALL {
                    cases.push(prepare_case(seq, boot, v, s, p));
                    seq += 1;
                }
            }
        }
    }
    cases
}

fn prepare_case(seq: u64, boot: Boot, v: Rel, s: Rel, p: Rel) -> Case {
    use compliance::{Executor, gossip_request, op_entry, prepare, view, wire_hex};
    use vrr::ids::Slot;

    // Build the receiver's state, reading its frontiers as the message's
    // dimensions anchor against them.
    let mut setup = view_one_setup();
    if let Boot::Restarting = boot {
        setup.push(Op::Halt { node: RECEIVER });
        setup.push(Op::Restart {
            node: RECEIVER,
            kind: "clean".into(),
        });
    }
    let mut probe = Executor::run_setup(&setup);
    let before = probe
        .harness()
        .snapshot(probe.id_of(RECEIVER))
        .expect("the receiver is live");
    let (accepted, committed, own_view) = (before.accepted, before.committed, before.view);
    let header_view = view(rel(v, own_view) as u32);
    let sender = rel(v, 1) as usize;
    let entry_slot = Slot(rel(s, accepted + 1));
    let piggyback = Slot(rel(p, committed));
    let entry = match s {
        Rel::Less => probe
            .harness()
            .journal_entry(probe.id_of(RECEIVER), entry_slot)
            .expect("the slot is held"),
        _ => op_entry(entry_slot, header_view.era, seq),
    };
    let input = Op::Deliver {
        to: RECEIVER,
        from: sender,
        wire: wire_hex(&prepare(header_view, entry_slot, entry, piggyback)),
    };
    let id = format!(
        "prepare-boot-{:?}-view-{:?}-slot-{:?}-piggyback-{:?}",
        match boot {
            Boot::Normal => "normal",
            Boot::Restarting => "restarting",
        },
        v,
        s,
        p
    );
    Case {
        id,
        family: "prepare".into(),
        clause: prepare_clause(
            match v {
                Rel::Less => "below",
                Rel::Equal => "equal to",
                Rel::Greater => "above",
            },
            match s {
                Rel::Less => "below",
                Rel::Equal => "equal to",
                Rel::Greater => "above",
            },
            match p {
                Rel::Less => "below",
                Rel::Equal => "equal to",
                Rel::Greater => "above",
            },
        ),
        keywords: vec!["MUST".into()],
        setup,
        input,
        expect: compliance::Expect::default(),
    }
}

// ----------------------------------------------------------------------
// The abstract host families
// ----------------------------------------------------------------------

fn fence_family() -> Vec<Case> {
    vec![
        Case {
            id: "fence-clean-restart-ticks-silently".into(),
            family: "host-fence".into(),
            clause: "A cleanly restarted node is boot-fenced: its tick MUST NOT \
                     emit any datagram while it is Restarting"
                .into(),
            keywords: vec!["MUST".into()],
            setup: vec![
                Op::Provision {
                    nodes: NODES,
                    timeout: TIMEOUT,
                },
                Op::Settle,
                Op::Halt { node: 1 },
            ],
            input: Op::Restart {
                node: 1,
                kind: "clean".into(),
            },
            expect: compliance::Expect::default(),
        },
        Case {
            id: "fence-bumped-node-ticks-silently".into(),
            family: "host-fence".into(),
            clause: "A bumped identity is not a member: its tick MUST NOT emit \
                     any datagram while it is Joining, and nothing resets to zero"
                .into(),
            keywords: vec!["MUST".into()],
            setup: vec![
                Op::Provision {
                    nodes: NODES,
                    timeout: TIMEOUT,
                },
                Op::Settle,
                Op::Crash { node: 1 },
            ],
            input: Op::Restart {
                node: 1,
                kind: "crashed".into(),
            },
            expect: compliance::Expect::default(),
        },
    ]
}

fn shutdown_family() -> Vec<Case> {
    vec![
        Case {
            id: "shutdown-writes-both-rounds-with-the-drain".into(),
            family: "host-shutdown".into(),
            clause: "The controlled shutdown MUST write Stopping to every marker \
                     copy, force the drain strictly between the rounds, and \
                     write Stopped to every copy"
                .into(),
            keywords: vec!["MUST".into()],
            setup: vec![
                Op::Provision {
                    nodes: NODES,
                    timeout: TIMEOUT,
                },
                Op::Settle,
            ],
            input: Op::Halt { node: 1 },
            expect: compliance::Expect::default(),
        },
        Case {
            id: "shutdown-clean-restart-continues-the-identity".into(),
            family: "host-shutdown".into(),
            clause: "The next boot over a stopped quorum MUST continue the same \
                     identity and reopen Restarting"
                .into(),
            keywords: vec!["MUST".into()],
            setup: vec![
                Op::Provision {
                    nodes: NODES,
                    timeout: TIMEOUT,
                },
                Op::Settle,
                Op::Halt { node: 1 },
            ],
            input: Op::Restart {
                node: 1,
                kind: "clean".into(),
            },
            expect: compliance::Expect::default(),
        },
    ]
}

fn crash_family() -> Vec<Case> {
    vec![Case {
        id: "crash-bumps-one-life-and-joins".into(),
        family: "host-crash".into(),
        clause: "A crash MUST leave the markers untouched, and the reopen over \
                 no stopped quorum MUST bump the identity exactly one life and \
                 write Joining"
            .into(),
        keywords: vec!["MUST".into()],
        setup: vec![
            Op::Provision {
                nodes: NODES,
                timeout: TIMEOUT,
            },
            Op::Settle,
            Op::Crash { node: 1 },
        ],
        input: Op::Restart {
            node: 1,
            kind: "crashed".into(),
        },
        expect: compliance::Expect::default(),
    }]
}

fn gossip_family() -> Vec<Case> {
    vec![Case {
        id: "gossip-adds-the-sender-to-every-witness-list".into(),
        family: "gossip-witness".into(),
        clause: "Every node that hears a join gossip MUST add the sender to its \
                 gossip-witness list, and the leader MUST stream to the witness"
            .into(),
        keywords: vec!["MUST".into()],
        setup: vec![
            Op::Provision {
                nodes: NODES,
                timeout: TIMEOUT,
            },
            Op::Settle,
        ],
        input: Op::Gossip {
            wire: String::new(), // filled by the exporter's probe below
        },
        expect: compliance::Expect::default(),
    }]
}
