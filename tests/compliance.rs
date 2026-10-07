//! The compliance suite's reference runner and exporter
//! (`docs/uvrr-host-compliance.md`). The runner replays every case in the
//! corpus through the abstract host interface and asserts the named
//! expectations; the exporter regenerates the corpus from the same
//! execution path, so the committed bytes and the codec cannot drift.
//!
//! The committed corpus is generated, and the export and sync gate below
//! is what says so: it rebuilds the corpus in memory and compares it
//! against the committed bytes, without writing. Regenerate and commit
//! with `make corpus-export`.

#[cfg(feature = "serde")]
#[path = "compliance/mod.rs"]
mod compliance;
#[cfg(feature = "serde")]
#[path = "harness/mod.rs"]
mod harness;

/// The corpus proof did not run; it is not a corpus proof.
///
/// The corpus reader lives in the library behind the `serde` feature, so
/// everything the gate needs is behind it too, and a run without the
/// feature carries no corpus statement of any kind. That run must not
/// read as a green corpus: this test is the always-compiled half of the
/// target, its name says what happened, and it prints the lanes that
/// carry the real thing. The pre-push hook and CI both run the
/// all-features lane, where the corpus is replayed for real.
#[cfg(not(feature = "serde"))]
#[test]
fn the_corpus_gate_did_not_run_without_the_serde_feature() {
    eprintln!(
        "\n\
         THE COMPLIANCE CORPUS DID NOT RUN. This target's corpus runner, its export\n\
         and sync gate, and its exporter are all behind the `serde` feature, so this run\n\
         carries no corpus proof whatever: a green line here says nothing about the corpus.\n\
         Run the two lanes that carry it:\n    \
         cargo test --all-features --test compliance\n    \
         cargo test --all-features --test compliance -- --ignored export_corpus\n"
    );
}

/// Everything the corpus needs is behind the `serde` feature, so
/// everything the corpus needs lives here: the runner, the exporters,
/// the families, and the sync gate that refuses a drifted corpus. The
/// notice above is what a run without the feature sees instead.
#[cfg(feature = "serde")]
mod serde_gated {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::compliance::{
        Case, Executor, Expect, Op, SystemOp, assert_expectation, commit, fuse, gossip_request,
        identity, op_entry, pair_of, prepare, prepare_ok, view, wire_hex,
    };
    use uvrr::configuration::SystemOperation;
    use uvrr::ids::{Ballot, Era, Slot, View};
    use uvrr::journal::{LogEntry, Payload};

    /// Where the corpus lives.
    const CORPUS: &str = "tests/compliance/corpus";

    /// The command that regenerates the committed corpus. Every message a
    /// gate here fails with names it, because a failure that says only
    /// what differs leaves the reader to guess what to run.
    const REGEN: &str = "cargo test --all-features --test compliance -- --ignored export_corpus  (make corpus-export)";

    /// The primary-timeout knob of every corpus cluster, in host ticks.
    const TIMEOUT: u64 = 3;

    // ----------------------------------------------------------------------
    // The corpus files
    // ----------------------------------------------------------------------

    /// Reads every corpus file, with its family name.
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
                let cases: Vec<Case> = serde_json::from_str(&text).expect("the corpus file parses");
                (name, cases)
            })
            .collect()
    }

    /// The corpus passes: every case in every family, replayed from the
    /// committed data through the abstract host interface.
    #[test]
    fn the_corpus_passes() {
        let corpus = corpus();
        assert!(!corpus.is_empty(), "the corpus has families");
        for (family, cases) in &corpus {
            assert!(!cases.is_empty(), "{family} has cases");
            for case in cases {
                assert_eq!(
                    case.family, *family,
                    "{}: the case names its family file",
                    case.id
                );
                let captured = Executor::run_case(case).unwrap_or_else(|e| {
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

    /// The export and sync gate: the corpus regenerated from the reference
    /// behaviour equals the committed corpus, family by family, and the
    /// corpus directory holds exactly the families. Both failures name the
    /// files and the regeneration command.
    #[test]
    fn the_regenerated_corpus_equals_the_committed_corpus() {
        let families = exporter_families();
        let mut expected: Vec<PathBuf> = families
            .iter()
            .map(|(name, _)| Path::new(CORPUS).join(format!("{name}.json")))
            .collect();
        expected.sort();
        let mut committed: Vec<PathBuf> = fs::read_dir(CORPUS)
            .expect("the corpus directory is present")
            .map(|entry| entry.expect("the corpus directory reads"))
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "json"))
            .collect();
        committed.sort();
        assert_eq!(
            committed, expected,
            "{CORPUS} does not hold exactly the families the exporter writes: a family here the\n\
             exporter does not name, or a family the exporter names that is missing, fails here.\n\
             Regenerate and commit the whole directory with\n    {REGEN}"
        );
        for (name, cases) in &families {
            let path = Path::new(CORPUS).join(format!("{name}.json"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("the corpus file {path:?} reads: {e}"));
            let regenerated = serde_json::to_string_pretty(cases).expect("the corpus serializes");
            assert_eq!(
                text,
                format!("{regenerated}\n"),
                "{path:?} is stale: the corpus regenerated from the reference behaviour differs\n\
                 from the committed bytes, so the reference and the committed corpus disagree on\n\
                 this family. Regenerate and commit it with\n    {REGEN}"
            );
        }
    }

    /// Regenerates the corpus from the reference behaviour. Ignored by
    /// default: `cargo test --all-features --test compliance -- --ignored
    /// export_corpus`, which is what `make corpus-export` runs. The
    /// regenerated files MUST equal the committed files; the sync gate
    /// asserts it without writing. The export overwrites whatever the
    /// committed file holds, so a hand edit belongs in the reference.
    #[test]
    #[ignore = "the exporter writes the corpus; run it deliberately"]
    fn export_corpus() {
        fs::create_dir_all(CORPUS).expect("the corpus directory creates");
        for (name, cases) in exporter_families() {
            let path = Path::new(CORPUS).join(format!("{name}.json"));
            let text = serde_json::to_string_pretty(&cases).expect("the corpus serializes");
            fs::write(&path, format!("{text}\n")).expect("the corpus file writes");
            eprintln!("exported {path:?} ({} cases)", cases.len());
        }
    }

    // ----------------------------------------------------------------------
    // The case builder
    // ----------------------------------------------------------------------

    /// Builds one case by capturing the reference behaviour: the setup and
    /// input are data, the expectation is what the executor captures.
    fn case(
        id: &str,
        family: &str,
        theorem: &str,
        clause: String,
        setup: Vec<Op>,
        input: Op,
    ) -> Case {
        let mut built = Case {
            id: id.into(),
            family: family.into(),
            theorem: theorem.into(),
            clause,
            keywords: vec!["MUST".into()],
            setup,
            input,
            expect: Expect::default(),
        };
        built.expect = Executor::run_case(&built)
            .unwrap_or_else(|e| panic!("{}: the case refused to run: {e}", built.id));
        built
    }

    /// A settled three-node cluster's setup.
    fn settled_setup() -> Vec<Op> {
        vec![
            Op::Provision {
                nodes: 3,
                timeout: TIMEOUT,
            },
            Op::Settle,
        ]
    }

    /// The view and the primary of a setup's end state, through the public
    /// interface: the snapshot's era and view, and the configuration's
    /// primary of that view.
    fn probe_primary(setup: &[Op], node: &str) -> (Ballot, String) {
        let mut probe = Executor::run_setup(setup).expect("the probe setup runs");
        let id = probe.id_of(node).expect("the corpus identity is lawful");
        let snapshot = probe
            .harness()
            .snapshot(id)
            .expect("the probed node is live");
        let view = Ballot {
            era: Era(snapshot.era),
            view: View(snapshot.view),
        };
        let primary = probe
            .harness()
            .era_table(id)
            .expect("the probed node is live")
            .record(view.era)
            .expect("the probed era is retained")
            .config
            .primary(view.view)
            .expect("a non-void configuration always answers");
        (view, pair_of(primary))
    }

    /// The view and the accepted and committed frontiers of a setup's end
    /// state, through the public interface.
    fn probe_state(setup: &[Op], node: &str) -> (Ballot, u64, u64) {
        let mut probe = Executor::run_setup(setup).expect("the probe setup runs");
        let id = probe.id_of(node).expect("the corpus identity is lawful");
        let snapshot = probe
            .harness()
            .snapshot(id)
            .expect("the probed node is live");
        (
            Ballot {
                era: Era(snapshot.era),
                view: View(snapshot.view),
            },
            snapshot.accepted,
            snapshot.committed,
        )
    }

    /// The wire hex of the announcement a freshly bumped node emits, from a
    /// throwaway probe: the corpus's input data packs the reference codec's
    /// own bytes.
    fn probe_announcement(setup: &[Op], node: &str, old: &str) -> String {
        let mut probe = Executor::run_setup(setup).expect("the probe setup runs");
        let id = probe.id_of(node).expect("the corpus identity is lawful");
        let previous = probe.id_of(old).expect("the corpus identity is lawful");
        probe.harness().reincarnate(id, previous);
        let queue = probe.harness().queued_envelopes();
        let (_, _, message) = queue
            .iter()
            .find(|(from, _, _)| *from == id)
            .expect("the announcement is queued");
        wire_hex(message)
    }

    /// The typed operations of the fused schedule, with the argument
    /// identities resolved from the corpus's explicit pairs.
    fn fuse_ops() -> Vec<SystemOperation> {
        let joined = identity("4:1").expect("the corpus identity is lawful");
        vec![
            SystemOperation::Join {
                node: joined,
                position: 3,
            },
            SystemOperation::Increment(joined),
        ]
    }

    // ----------------------------------------------------------------------
    // The families
    // ----------------------------------------------------------------------

    fn exporter_families() -> Vec<(&'static str, Vec<Case>)> {
        vec![
            ("prepare-accept", prepare_accept_family()),
            ("view-selection", view_selection_family()),
            ("agreement", agreement_family()),
            ("casting-vote", casting_vote_family()),
            ("reconfiguration", reconfiguration_family()),
            ("reincarnation-safety", reincarnation_safety_family()),
            ("fuse", fuse_family()),
            ("identity", identity_family()),
            ("witness", witness_family()),
            ("boot-gate", boot_gate_family()),
        ]
    }

    // ----------------------------------------------------------------------
    // The prepare-accept family
    // ----------------------------------------------------------------------

    /// The relation of one numeric field against the receiver's own value.
    #[derive(Clone, Copy, PartialEq, Eq)]
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

    fn rel_word(r: Rel) -> &'static str {
        match r {
            Rel::Less => "below",
            Rel::Equal => "equal to",
            Rel::Greater => "above",
        }
    }

    /// The same relation as an id slug: the clause reads prose, the id
    /// reads a slug, and the two never mix.
    fn rel_slug(r: Rel) -> &'static str {
        match r {
            Rel::Less => "less",
            Rel::Equal => "equal",
            Rel::Greater => "greater",
        }
    }

    fn boot_word(b: Boot) -> &'static str {
        match b {
            Boot::Normal => "normal",
            Boot::Restarting => "restarting",
        }
    }

    /// The setup that reaches the receiver at view 1, every node [`uvrr::progress::Status::Normal`].
    fn view_one_setup() -> Vec<Op> {
        let mut setup = vec![
            Op::Provision {
                nodes: 3,
                timeout: TIMEOUT,
            },
            Op::TickAll,
            Op::DeliverAll,
        ];
        for _ in 0..=TIMEOUT {
            setup.push(Op::Tick { node: "2:1".into() });
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
             a frontier, an accept journals and answers, a gap holds the frontier \
             and fetches, and a higher view fences the node"
        )
    }

    /// The rung theorem the route pins, named by an exhaustive match: the
    /// family is the Acceptor rung's, and each route exercises its own
    /// theorem.
    fn prepare_theorem(boot: Boot, v: Rel, s: Rel) -> &'static str {
        match (v, s) {
            (Rel::Less, _) => "Acceptor.free_forbids",
            (Rel::Greater, _) => match boot {
                Boot::Normal => "Acceptor.promise_preserves",
                Boot::Restarting => "Acceptor.free_forbids",
            },
            (Rel::Equal, Rel::Less) => "Acceptor.report_last",
            (Rel::Equal, Rel::Equal) => "Acceptor.accept_preserves",
            (Rel::Equal, Rel::Greater) => "Acceptor.reachable_inv",
        }
    }

    /// The prepare-accept family: the [`uvrr::wire::Tag::Prepare`] a backup receives, over the
    /// cross product of its dimensions, minus the unreachable input
    /// (`docs/uvrr-host-compliance.md` §7): the piggyback claiming the arriving
    /// slot itself, which no conforming primary emits, whose refusal is
    /// recorded by the mint-based exhaustive suite. The chain-fold pair
    /// closes over the system entry's own dimension, the fold the arriving
    /// entry is judged against.
    fn prepare_accept_family() -> Vec<Case> {
        let mut seq = 1;
        let mut cases = Vec::new();
        for boot in BOOT_ALL {
            for v in REL_ALL {
                for s in REL_ALL {
                    for p in REL_ALL {
                        if v == Rel::Equal && s == Rel::Equal && p == Rel::Greater {
                            continue;
                        }
                        cases.push(prepare_case(seq, boot, v, s, p));
                        seq += 1;
                    }
                }
            }
        }
        cases.extend(chain_fold_cases());
        cases
    }

    /// An establishing operation at its own slot, stamped with the era the
    /// case names: the corpus's own rendering of a [`Payload::System`]
    /// entry, the shape a fused slab's packed slots journal.
    fn system_entry(slot: Slot, era: Era, op: SystemOperation) -> LogEntry {
        LogEntry {
            slot,
            era,
            payload: Payload::System(op),
        }
    }

    /// The accept-perimeter chain fold (`docs/uvrr-fuse.md` §6): an arriving
    /// system entry's guards are evaluated against the configuration the
    /// fold of the accepted-but-uncommitted predecessors established, not
    /// against the committed configuration alone, exactly as though the
    /// predecessors had arrived as separate datagrams.
    ///
    /// The pair discriminates the rule, because the two cases carry the same
    /// operation at the same era and differ in one thing only: the join the
    /// first accepts at the slot before, uncommitted. Judged against the
    /// committed configuration alone the promotion names a member no
    /// committed fold has introduced and is refused; judged against the
    /// chain fold it is the promotion the join's own fold made legal.
    fn chain_fold_cases() -> Vec<Case> {
        let learner = identity("4:1").expect("the corpus identity is lawful");
        let join = SystemOperation::Join {
            node: learner,
            position: 3,
        };
        let promote = SystemOperation::Increment(learner);
        let bare = settled_setup();
        let (view, accepted, committed) = probe_state(&bare, "2:1");
        let slot = Slot(accepted + 1);
        let piggyback = Slot(committed);
        // The chain: the join accepted at the receiver's next slot, its own
        // commit frontier untouched.
        let mut chained = bare.clone();
        chained.push(Op::Deliver {
            to: "2:1".into(),
            from: "1:1".into(),
            wire: wire_hex(&prepare(
                view,
                slot,
                system_entry(slot, view.era, join),
                piggyback,
            )),
        });
        let tail = Slot(slot.0 + 1);
        vec![
            case(
                "prepare-accept-the-chain-fold-judges-the-promotion-inside-it",
                "prepare-accept",
                "Acceptor.accept_preserves",
                "A system entry MUST be judged against the configuration the \
                 fold of its accepted but uncommitted predecessors \
                 established: the member an uncommitted join introduced is \
                 promoted, the entry journals at its own slot, and the \
                 committed frontier stays where the chain left it"
                    .into(),
                chained,
                Op::Deliver {
                    to: "2:1".into(),
                    from: "1:1".into(),
                    wire: wire_hex(&prepare(
                        view,
                        tail,
                        system_entry(tail, view.era, promote.clone()),
                        piggyback,
                    )),
                },
            ),
            case(
                "prepare-accept-without-the-accepted-predecessor-the-promotion-is-refused",
                "prepare-accept",
                "Acceptor.reachable_inv",
                "The same promotion with nothing accepted before it MUST be \
                 refused: the committed configuration holds no such member, \
                 the receiver keeps its own state, and nothing journals or \
                 answers"
                    .into(),
                bare,
                Op::Deliver {
                    to: "2:1".into(),
                    from: "1:1".into(),
                    wire: wire_hex(&prepare(
                        view,
                        slot,
                        system_entry(slot, view.era, promote),
                        piggyback,
                    )),
                },
            ),
        ]
    }

    fn prepare_case(seq: u64, boot: Boot, v: Rel, s: Rel, p: Rel) -> Case {
        // Build the receiver's state, reading its frontiers as the message's
        // dimensions anchor against them.
        let mut setup = view_one_setup();
        if let Boot::Restarting = boot {
            setup.push(Op::Halt { node: "3:1".into() });
            setup.push(Op::Restart {
                node: "3:1".into(),
                kind: "clean".into(),
            });
        }
        let mut probe = Executor::run_setup(&setup).expect("the probe setup runs");
        let receiver = probe.id_of("3:1").expect("the corpus identity is lawful");
        let before = probe
            .harness()
            .snapshot(receiver)
            .expect("the receiver is live");
        let (accepted, committed, own_view) = (before.accepted, before.committed, before.view);
        let header_view = view(rel(v, u64::from(own_view)) as u32);
        let sender = match v {
            // Below the receiver's view: a member that is not the primary.
            Rel::Less => "1:1",
            // The primary of the receiver's view.
            Rel::Equal => "2:1",
            // Above the receiver's view: a member that is not the primary.
            Rel::Greater => "3:1",
        };
        let entry_slot = Slot(rel(s, accepted + 1));
        let piggyback = Slot(rel(p, committed));
        let entry = match s {
            Rel::Less => probe
                .harness()
                .journal_entry(receiver, entry_slot)
                .expect("the slot is held"),
            _ => op_entry(entry_slot, header_view.era, seq),
        };
        let input = Op::Deliver {
            to: "3:1".into(),
            from: sender.into(),
            wire: wire_hex(&prepare(header_view, entry_slot, entry, piggyback)),
        };
        let id = format!(
            "prepare-accept-boot-{}-view-{}-slot-{}-piggyback-{}",
            boot_word(boot),
            rel_slug(v),
            rel_slug(s),
            rel_slug(p)
        );
        case(
            &id,
            "prepare-accept",
            prepare_theorem(boot, v, s),
            prepare_clause(rel_word(v), rel_word(s), rel_word(p)),
            setup,
            input,
        )
    }

    // ----------------------------------------------------------------------
    // The view-selection family
    // ----------------------------------------------------------------------

    /// The failover's suspicion setup: the primary is crashed, one backup
    /// has suspected past the timeout, its fence datagrams are queued.
    fn failover_setup() -> Vec<Op> {
        let mut setup = settled_setup();
        setup.push(Op::Crash { node: "1:1".into() });
        for _ in 0..=TIMEOUT {
            setup.push(Op::Tick { node: "2:1".into() });
        }
        setup
    }

    fn view_selection_family() -> Vec<Case> {
        // The second survivor's suspicion completes the election: its fence
        // datagram arms the quorum, the new primary collects the surviving
        // evidence, selects the highest-ranked report, and installs.
        let elected = {
            let mut setup = failover_setup();
            setup.push(Op::Tick { node: "3:1".into() });
            setup.push(Op::DeliverAll);
            setup.push(Op::Settle);
            setup
        };
        let serving = elected.clone();
        let (_, primary) = probe_primary(&serving, "2:1");
        vec![
            case(
                "view-selection-the-failover-collects-elects-and-installs",
                "view-selection",
                "ViewSelection.newer_view_wins",
                "The survivors of a silent primary MUST complete the failover: \
                 the surviving evidence reaches the new primary, the \
                 highest-ranked report is selected, and the installed view \
                 is announced to every survivor"
                    .into(),
                failover_setup(),
                Op::Tick { node: "3:1".into() },
            ),
            case(
                "view-selection-the-committed-prefix-survives-and-serves",
                "view-selection",
                "ViewSelection.quorum_preserves",
                "The selected history MUST preserve the committed prefix, and \
                 the cluster MUST serve under the installed view: a proposal \
                 commits and every survivor journals the identical history"
                    .into(),
                serving,
                Op::Propose {
                    node: primary,
                    payload: "re-elected".into(),
                },
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The agreement family
    // ----------------------------------------------------------------------

    fn agreement_family() -> Vec<Case> {
        let setup = settled_setup();
        let (_, primary) = probe_primary(&setup, "1:1");
        vec![case(
            "agreement-a-proposal-commits-identically-everywhere",
            "agreement",
            "Synod.theorem8",
            "A proposal from the primary MUST commit through the quorum: \
             every node journals the identical entry at the identical slot \
             and advances the same committed frontier"
                .into(),
            setup,
            Op::Propose {
                node: primary,
                payload: "agreed".into(),
            },
        )]
    }

    // ----------------------------------------------------------------------
    // The casting-vote family
    // ----------------------------------------------------------------------

    /// The heavy cluster's setup: one increment seats the heavy member
    /// inside every majority — weights {1, 1, 2}, floor(4/2)+1 = 3, and no
    /// majority excludes it.
    fn heavy_setup() -> Vec<Op> {
        let mut heavy = settled_setup();
        let (_, primary) = probe_primary(&heavy, "1:1");
        heavy.push(Op::Reconfigure {
            node: primary,
            system: SystemOp::Increment { node: "3:1".into() },
        });
        heavy.push(Op::DeliverAll);
        heavy.push(Op::Settle);
        heavy
    }

    fn casting_vote_family() -> Vec<Case> {
        let heavy = heavy_setup();
        let (_, serving_primary) = probe_primary(&heavy, "1:1");
        let mut without = heavy;
        without.push(Op::Crash { node: "3:1".into() });
        vec![
            case(
                "casting-vote-phase-one-completes-through-the-heavy-member",
                "casting-vote",
                "CastingVote.completes_phase1",
                "A member whose weight seats it inside every majority MUST be \
                 the member through which phase one completes: the commit \
                 carries its acceptance"
                    .into(),
                heavy_setup(),
                Op::Propose {
                    node: serving_primary.clone(),
                    payload: "through-the-heavy-member".into(),
                },
            ),
            case(
                "casting-vote-without-the-heavy-member-no-quorum-completes",
                "casting-vote",
                "CastingVote.guard_preserved",
                "Without the member whose weight seats it inside every \
                 majority, no phase-one quorum MUST form: nothing commits and \
                 no frontier of commitment moves"
                    .into(),
                without,
                Op::Propose {
                    node: serving_primary,
                    payload: "without-the-heavy-member".into(),
                },
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The reconfiguration family
    // ----------------------------------------------------------------------

    fn reconfiguration_family() -> Vec<Case> {
        let setup = settled_setup();
        let (_, primary) = probe_primary(&setup, "1:1");
        let mut cases = vec![
            case(
                "reconfiguration-the-establishing-commit-opens-the-next-era",
                "reconfiguration",
                "Eras.theorem10",
                "A committed reconfiguration operation MUST establish the \
                 next era, and the cluster MUST serve under it with the \
                 weights it names"
                    .into(),
                setup.clone(),
                Op::Reconfigure {
                    node: primary.clone(),
                    system: SystemOp::Increment { node: "3:1".into() },
                },
            ),
            case(
                "reconfiguration-a-join-inserts-a-weight-zero-learner",
                "reconfiguration",
                "IdentityLaw.weight0_majority_irrelevant",
                "A join MUST insert the new member at weight zero: the \
                 learner holds a succession position and is invisible to \
                 every majority"
                    .into(),
                setup,
                Op::Reconfigure {
                    node: primary,
                    system: SystemOp::Join {
                        node: "4:1".into(),
                        position: 3,
                    },
                },
            ),
        ];
        cases.extend(view_jump_cases());
        cases
    }

    /// The commit-time view advance over a slab's own eras: the grounded
    /// nomination walk of the per-slot era law
    /// (`docs/uvrr-fuse.md` §4 step 4, the NOMINATE chapter of
    /// `docs/uvrr-protocols.md`) and the per-slot view-jump law that
    /// governs it.
    ///
    /// The slab is the join-and-promote schedule with the nomination rider
    /// appended, so its fold stamps two establishing packed slots one era
    /// apart and the rider rides the last of them. The two cases differ in
    /// the rider alone, and that is the whole of the rule: a rider naming
    /// the running view redeems the second era, so the serving view crosses
    /// both eras of the slab in the one published advance, each spanned era
    /// established by a slot the committed frontier covers; a rider naming
    /// no running view redeems nothing, the fold stops before the run the
    /// view cannot carry, and the committed frontier lands on the head slot
    /// with the serving view where it stood.
    ///
    /// The quotient the walk clause enforces, the total era jump divided by
    /// the slot count the advance commits, is in `(0, 1]` exactly where the
    /// walk lands: one era per establishing packed slot, and a walk that
    /// advances. The transition-legality refusals themselves are gate-level
    /// and no abstract host operation reaches them, so the corpus states the
    /// law as the two advances the library itself performs.
    fn view_jump_cases() -> Vec<Case> {
        let settled = settled_setup();
        let (view, accepted, _) = probe_state(&settled, "2:1");
        let head_slot = Slot(accepted + 1);
        // The slab, with the rider's `from` naming the running view or none
        // of it: the pair differs in that one field.
        let schedule = |from: View| {
            let mut ops = fuse_ops();
            ops.push(SystemOperation::Nominate { from, offset: 1 });
            ops
        };
        let walked = schedule(view.view);
        let idle = schedule(View(view.view.0 + 1));
        let tail_slot = Slot(head_slot.0 + walked.len() as u64 - 1);
        let folded = |ops: Vec<SystemOperation>| {
            let mut setup = settled.clone();
            setup.push(Op::Deliver {
                to: "2:1".into(),
                from: "1:1".into(),
                wire: wire_hex(&fuse(view, head_slot, ops)),
            });
            setup
        };
        let announce = || Op::Deliver {
            to: "2:1".into(),
            from: "1:1".into(),
            wire: wire_hex(&commit(view, tail_slot)),
        };
        vec![
            case(
                "reconfiguration-the-rider-walks-the-view-across-the-slabs-eras",
                "reconfiguration",
                "ViewJump.view_jump_safe",
                "A commit-time view advance MUST cross more than one era \
                 exactly when the pair's own evidence grounds it: the rider \
                 names the running view, every era the jump spans is \
                 established by a slot the committed frontier covers, and the \
                 whole advance lands in one transition with the serving view \
                 at the slab's own final era"
                    .into(),
                folded(walked),
                announce(),
            ),
            case(
                "reconfiguration-without-the-matching-rider-the-tail-defers",
                "reconfiguration",
                "ViewJump.successor_not_required",
                "The same slab with no rider naming the running view MUST NOT \
                 walk: the fold stops before the run the serving view cannot \
                 carry, the committed frontier lands on the head slot, and \
                 the view stands where the pair left it"
                    .into(),
                folded(idle),
                announce(),
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The reincarnation-safety family
    // ----------------------------------------------------------------------

    /// The forced sequence's choreography (`docs/uvrr-protocols.md`, the reincarnation chapter):
    /// the missed proposal, the crash, the bumped reopen, the announcement,
    /// the first forced batch, the ordinary view change into the era it
    /// established, and the idempotent re-announce that carries the final
    /// batch and seats the bumped life.
    fn reincarnation_safety_family() -> Vec<Case> {
        let mut setup = settled_setup();
        let (_, primary) = probe_primary(&setup, "1:1");
        setup.push(Op::Propose {
            node: primary,
            payload: "missed".into(),
        });
        setup.push(Op::DeliverAll);
        setup.push(Op::Crash { node: "3:1".into() });
        setup.push(Op::Restart {
            node: "3:1".into(),
            kind: "crashed".into(),
        });
        setup.push(Op::Announce {
            node: "3:2".into(),
            old: "3:1".into(),
        });
        setup.push(Op::DeliverAll);
        for _ in 0..=TIMEOUT {
            setup.push(Op::Tick { node: "1:1".into() });
        }
        setup.push(Op::DeliverAll);
        vec![case(
            "reincarnation-safety-the-forced-sequence-seats-the-bumped-life",
            "reincarnation-safety",
            "ReincarnationGeneral.forced_sequence_era_safe",
            "A crashed member MUST return through a bumped identity and the \
             forced sequence: the old identity is evicted, the bumped life is \
             seated at weight one, every journal agrees, the witness lists \
             drop the promoted node, and the deferred marker round lands at \
             the seated witness"
                .into(),
            setup,
            Op::Announce {
                node: "3:2".into(),
                old: "3:1".into(),
            },
        )]
    }

    // ----------------------------------------------------------------------
    // The fuse family
    // ----------------------------------------------------------------------

    fn fuse_family() -> Vec<Case> {
        // A valid envelope: the ballot is the receiver's current view, the
        // batch begins at the successor of its accepted frontier.
        let valid = settled_setup();
        let (view, accepted, _) = probe_state(&valid, "2:1");
        let first_slot = Slot(accepted + 1);
        // A stale ballot: the cluster has moved one view past the message's.
        let mut stale = settled_setup();
        for _ in 0..=TIMEOUT {
            stale.push(Op::Tick { node: "2:1".into() });
        }
        stale.push(Op::DeliverAll);
        let (current, stale_accepted, _) = probe_state(&stale, "2:1");
        let stale_slot = Slot(stale_accepted + 1);
        let stale_view = Ballot {
            era: current.era,
            view: View(current.view.0 - 1),
        };
        let mut cases = vec![
            case(
                "fuse-the-valid-batch-is-accepted-as-a-whole",
                "fuse",
                "Fuse.decide_all",
                "A Fuse datagram MUST be accepted as a whole: every packed \
                 operation journals at its own slot in batch order, the \
                 frontiers advance per packed operation, and the accept \
                 answers once"
                    .into(),
                valid,
                Op::Deliver {
                    to: "2:1".into(),
                    from: "1:1".into(),
                    wire: wire_hex(&fuse(view, first_slot, fuse_ops())),
                },
            ),
            case(
                "fuse-the-stale-ballot-is-refused-as-a-whole",
                "fuse",
                "Fuse.same_ballot_era_guard",
                "A Fuse at a ballot the receiver has passed MUST be refused \
                 as a whole: nothing journals, nothing installs, and no \
                 partial accept is visible"
                    .into(),
                stale,
                Op::Deliver {
                    to: "2:1".into(),
                    from: "1:1".into(),
                    wire: wire_hex(&fuse(stale_view, stale_slot, fuse_ops())),
                },
            ),
        ];
        cases.extend(slab_ballot_cases());
        cases
    }

    /// The slab's per-slot ballots, as the cross product of the packed slot a
    /// retransmitted [`uvrr::wire::Tag::Prepare`] names (the head or the tail)
    /// against the era that entry carries (the head ballot's era or the
    /// tail's).
    ///
    /// The slab is the join-and-promote schedule of `fuse_ops`, one
    /// establishing operation per packed slot, so its fold stamps the head
    /// slot with the head ballot's era and the tail slot with the era one
    /// establishing operation on (`docs/uvrr-fuse.md` §1). The acceptor's
    /// journal holds those stamped entries, and the era authorisation admits
    /// the header's era and its successor at either slot, so all four routes
    /// reach the retransmission rule and are discriminated by the entry the
    /// fold installed alone: a packed slot re-answers at the ballot its own
    /// fold stamped and is refused at any other's.
    ///
    /// The two tail-slot cases state the promise-to-tail obligation
    /// (`docs/uvrr-fuse.md` §3): the promise the accept incurred is the
    /// slab's last ballot, so the tail slot answers at the tail's own era and
    /// nothing below it is ever folded into that slot. The two head-slot cases
    /// state the per-slot era law: the head slot carries the head's era and
    /// no other, one era per establishing operation.
    fn slab_ballot_cases() -> Vec<Case> {
        let settled = settled_setup();
        let (view, accepted, committed) = probe_state(&settled, "2:1");
        let ops = fuse_ops();
        let head_slot = Slot(accepted + 1);
        let tail_slot = Slot(head_slot.0 + ops.len() as u64 - 1);
        let head_era = view.era;
        let tail_era = head_era.next().expect("the era space is not spent");
        // The piggyback rides the sender's own commit frontier, as it does
        // on the wire: the two cases of a slot differ in the era alone.
        let piggyback = Slot(committed);
        // The folded slab: the crossing envelope the receiver takes whole.
        let mut slab = settled.clone();
        slab.push(Op::Deliver {
            to: "2:1".into(),
            from: "1:1".into(),
            wire: wire_hex(&fuse(view, head_slot, ops.clone())),
        });
        let route = |slot: Slot, era: Era, op: SystemOperation| Op::Deliver {
            to: "2:1".into(),
            from: "1:1".into(),
            wire: wire_hex(&prepare(view, slot, system_entry(slot, era, op), piggyback)),
        };
        let head_op = ops[0].clone();
        let tail_op = ops[1].clone();
        vec![
            case(
                "fuse-the-head-slot-re-answers-at-the-head-ballot",
                "fuse",
                "Fuse.decide_all",
                "Every packed slot MUST carry its own ballot: the head slot \
                 re-answers a retransmission stamped with the head ballot's \
                 era, the journal holds that entry, and no frontier moves"
                    .into(),
                slab.clone(),
                route(head_slot, head_era, head_op.clone()),
            ),
            case(
                "fuse-the-head-slot-at-the-tail-ballot-is-refused",
                "fuse",
                "Fuse.same_ballot_era_guard",
                "A packed slot MUST NOT hold another packed slot's ballot: the \
                 head slot stamped with the tail ballot's era is not the \
                 entry the fold installed, so it is refused and nothing \
                 journals or answers"
                    .into(),
                slab.clone(),
                route(head_slot, tail_era, head_op),
            ),
            case(
                "fuse-the-tail-slot-at-the-head-ballot-is-refused",
                "fuse",
                "Fuse.Telescope.telescope_pass",
                "The promise a folded slab ends at MUST be the tail ballot: the \
                 tail slot stamped with the head ballot's era is refused, \
                 because the accept incurred the tail's own ballot and nothing \
                 below it may be folded into that slot"
                    .into(),
                slab.clone(),
                route(tail_slot, head_era, tail_op.clone()),
            ),
            case(
                "fuse-the-tail-slot-re-answers-at-the-tail-ballot",
                "fuse",
                "Fuse.Telescope.telescope_pass",
                "The slab's promise MUST be discharged at the tail ballot: the \
                 tail slot re-answers a retransmission stamped with the era \
                 the fold of the head slot established, and no frontier moves"
                    .into(),
                slab,
                route(tail_slot, tail_era, tail_op),
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The identity family
    // ----------------------------------------------------------------------

    fn identity_family() -> Vec<Case> {
        // The bumped reopen: the announcement a freshly bumped node emits.
        let mut bumped = settled_setup();
        bumped.push(Op::Crash { node: "3:1".into() });
        bumped.push(Op::Restart {
            node: "3:1".into(),
            kind: "crashed".into(),
        });
        let announcement = probe_announcement(&bumped, "3:2", "3:1");
        // The iterated bump: the bumped life crashes in its turn.
        let mut twice = bumped.clone();
        twice.push(Op::Crash { node: "3:2".into() });
        vec![
            case(
                "identity-the-announcement-packs-the-durable-pair",
                "identity",
                "IdentityLaw.pack_wire",
                "The reincarnation announcement MUST pack the durable pair on \
                 the wire, the old identity and its bump, and a hearing \
                 backup MUST drop it by name and list the announcer"
                    .into(),
                bumped.clone(),
                Op::Deliver {
                    to: "2:1".into(),
                    from: "3:2".into(),
                    wire: announcement,
                },
            ),
            case(
                "identity-each-life-is-one-counter-past-and-never-revisited",
                "identity",
                "IdentityLaw.lawful_life_separates",
                "Each bump MUST advance the crash counter exactly one past \
                 the last identity the disk saw: a value is burnt once and \
                 serves one life, and nothing self-resets to zero"
                    .into(),
                twice,
                Op::Restart {
                    node: "3:2".into(),
                    kind: "crashed".into(),
                },
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The witness family
    // ----------------------------------------------------------------------

    fn witness_family() -> Vec<Case> {
        let setup = settled_setup();
        let (view, primary) = probe_primary(&setup, "1:1");
        let (_, _, committed) = probe_state(&setup, "1:1");
        // The outside node's join gossip: from blank frontiers.
        let join_gossip = wire_hex(&gossip_request(view, Slot(0), Slot(0)));
        // A cluster member's gap gossip: one below the committed frontier.
        let member_gossip = wire_hex(&gossip_request(
            view,
            Slot(committed - 1),
            Slot(committed - 1),
        ));
        let mut listed = setup.clone();
        listed.push(Op::Gossip {
            wire: join_gossip.clone(),
        });
        vec![
            case(
                "witness-the-foreign-join-gossip-is-listed-by-every-hearer",
                "witness",
                "Witness.witness_not_in_quorum",
                "Every node that hears a join gossip MUST add the sender to \
                 its gossip-witness list, and the list MUST never name a \
                 voter: the leader answers, the backups record"
                    .into(),
                setup.clone(),
                Op::Gossip { wire: join_gossip },
            ),
            case(
                "witness-the-leader-streams-phase-two-to-the-listed-witness",
                "witness",
                "Witness.telescoped_promise_equivalence",
                "The leader MUST stream every phase-two to the listed \
                 witness: the commit announcements reach the witness as \
                 they reach the members"
                    .into(),
                listed,
                Op::Propose {
                    node: primary.clone(),
                    payload: "streamed".into(),
                },
            ),
            case(
                "witness-a-members-gossip-is-pushed-and-never-listed",
                "witness",
                "Witness.witness_not_in_quorum",
                "A cluster member's gossip MUST be answered with the push of \
                 the missing range and MUST NOT list the sender: the list \
                 never names a voter"
                    .into(),
                setup,
                Op::Deliver {
                    to: primary,
                    from: "3:1".into(),
                    wire: member_gossip,
                },
            ),
        ]
    }

    // ----------------------------------------------------------------------
    // The boot-gate family
    // ----------------------------------------------------------------------

    fn boot_gate_family() -> Vec<Case> {
        // The fabricated vote: a PrepareOk from the roster's outside
        // identity, for the primary's next slot.
        let settled = settled_setup();
        let (view, primary) = probe_primary(&settled, "1:1");
        let (_, accepted, _) = probe_state(&settled, "1:1");
        let fabricated = wire_hex(&prepare_ok(view, Slot(accepted + 1)));
        // The controlled halt, then the clean restart over it: the corpus's
        // fence post is the bumped standby's, not the resumed member's — a
        // cleanly restarted member ticks the full protocol and suspects a
        // silent primary like any member, so its tick is no fence.
        let mut halted = settled_setup();
        halted.push(Op::Halt { node: "1:1".into() });
        // The crashed reopen, then the bumped life's fenced tick.
        let mut crashed = settled_setup();
        crashed.push(Op::Crash { node: "3:1".into() });
        let mut bumped = crashed.clone();
        bumped.push(Op::Restart {
            node: "3:1".into(),
            kind: "crashed".into(),
        });
        // The Commit path's fence post, and its complement, both read from
        // one announcement: the settled view's own primary, at that view's
        // committed frontier, delivered at that view's ballot. The pair is
        // what discriminates the rule, because the two receivers differ in
        // one thing only, the weight they hold in the view's era's
        // configuration: the fenced member is the cleanly restarted one,
        // reopened at the retained ballot, and the outsider is the bumped
        // life, which no configuration it can name counts.
        let mut reopened = settled_setup();
        reopened.push(Op::Halt { node: "3:1".into() });
        reopened.push(Op::Restart {
            node: "3:1".into(),
            kind: "clean".into(),
        });
        let (fenced_view, fenced_primary) = probe_primary(&reopened, "1:1");
        let (_, _, fenced_committed) = probe_state(&reopened, "1:1");
        let announcement = wire_hex(&commit(fenced_view, Slot(fenced_committed)));
        let mut outside = settled_setup();
        outside.push(Op::Crash { node: "3:1".into() });
        outside.push(Op::Restart {
            node: "3:1".into(),
            kind: "crashed".into(),
        });
        vec![
            case(
                "boot-gate-the-controlled-halt-is-two-rounds-with-the-drain-between",
                "boot-gate",
                "ReincarnationGeneral.forced_run_terminal_flushed",
                "The controlled shutdown MUST write Stopping to every marker \
                 copy, force the drain strictly between the rounds, and \
                 write Stopped to every copy"
                    .into(),
                settled_setup(),
                Op::Halt { node: "1:1".into() },
            ),
            case(
                "boot-gate-the-clean-restart-continues-the-same-identity",
                "boot-gate",
                "ReincarnationGeneral.forced_run_committed",
                "The next boot over a stopped quorum MUST continue the same \
                 identity and reopen Restarting, one marker round at the \
                 continuing pair"
                    .into(),
                halted,
                Op::Restart {
                    node: "1:1".into(),
                    kind: "clean".into(),
                },
            ),
            case(
                "boot-gate-the-crashed-restart-bumps-and-enters-the-fence",
                "boot-gate",
                "IdentityLaw.wire_requires_bump",
                "A crashed reopen MUST read no stopped quorum, bump the \
                 identity exactly one life, and reopen fenced: the pair is \
                 decided before the wire phase, and the durable Joining \
                 round at the bumped pair defers to the seated witness"
                    .into(),
                crashed,
                Op::Restart {
                    node: "3:1".into(),
                    kind: "crashed".into(),
                },
            ),
            case(
                "boot-gate-the-bumped-fence-ticks-silently",
                "boot-gate",
                "ReincarnationGeneral.evicted_never_voting",
                "A bumped identity is outside every configuration it can \
                 name: its tick MUST NOT vote, MUST NOT drive a view change, \
                 and MUST NOT emit any datagram, and nothing resets to zero"
                    .into(),
                bumped,
                Op::Tick { node: "3:2".into() },
            ),
            case(
                "boot-gate-a-fabricated-vote-from-a-non-member-is-discarded",
                "boot-gate",
                "ReincarnationGeneral.evicted_never_voting",
                "A fabricated vote from a non-member MUST be discarded by \
                 name: no frontier moves and no datagram is emitted"
                    .into(),
                settled,
                Op::Deliver {
                    to: primary,
                    from: "4:1".into(),
                    wire: fabricated,
                },
            ),
            case(
                "boot-gate-the-first-life-anchors-before-it-answers",
                "boot-gate",
                "IdentityLaw.lawful_one_indexed",
                "A first life MUST latch its anchor, one Joining round at \
                 the one-indexed pair, before it answers anything"
                    .into(),
                settled_setup(),
                Op::Boot { node: "4:1".into() },
            ),
            case(
                "boot-gate-a-fenced-member-adopts-the-commit-bootstrap",
                "boot-gate",
                "Acceptor.accept_preserves",
                "A fenced member of the view's own configuration MUST leave \
                 the fence on a Commit announcement from that view's \
                 primary: the receiver reopens Normal and no frontier of \
                 commitment moves"
                    .into(),
                reopened,
                Op::Deliver {
                    to: "3:1".into(),
                    from: fenced_primary.clone(),
                    wire: announcement.clone(),
                },
            ),
            case(
                "boot-gate-an-outside-identity-stays-fenced-on-the-commit",
                "boot-gate",
                "ReincarnationGeneral.evicted_never_voting",
                "A fenced identity holding no weight in the view's era MUST \
                 stay fenced on the same announcement: it adopts no view, no \
                 frontier of commitment moves, and it answers nothing"
                    .into(),
                outside,
                Op::Deliver {
                    to: "3:2".into(),
                    from: fenced_primary,
                    wire: announcement,
                },
            ),
        ]
    }
}
