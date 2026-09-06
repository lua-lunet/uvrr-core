# Solver survey: currently-maintained tooling for verifying Paxos/Raft/VSR/VRR-family protocols

Date of survey: 2026-09-05. Method: Tavily web search across the spec's
suggested angles, plus repo-level maintenance checks. Inclusion rule: tooling
one could realistically download/submodule and smoke-test TODAY; old results
with no maintained tooling are listed under "Excluded (stale)" for the record.
Prioritization: "can we submodule and run it today" — Tier 1 > Tier 2 > Tier 3.

For each candidate: name; what it proves; tool + language; maintenance status;
URL; smoke-test plausibility (timeboxed hello-world).

## Tier 1 — install and run today, actively maintained

### 1. Veil

- **What it proves**: safety (inductive invariants) of distributed protocols —
  16 case studies incl. two Paxos variants, Vertical Paxos, Stellar Consensus
  Protocol, Rabia, Suzuki-Kasami; `#check_invariants` push-button SMT
  verification with interactive Lean fallback; concrete/symbolic BMC. Liveness
  proofs are in active development (blog series "Liveness Proofs in Veil",
  June 2026).
- **Tool/language**: Lean 4 library (Lean-auto/Lean-SMT → cvc5/Z3).
- **Maintenance**: very active — VERSE lab; Veil 2.0 pre-release with
  TLC-style explicit-state model checker; usage-example template repo; blog
  posts Feb 2026 and Jun 2026; CADE/CAV'25 + Dafny'26 papers.
- **URL**: https://github.com/verse-lab/veil (template:
  https://github.com/verse-lab/veil-usage-example; online environment linked
  from https://lean-lang.org/use-cases/veil)
- **Smoke test**: trivially plausible — `lake` dependency on `verse-lab/veil`,
  run the Ring/FloodSet tutorials in the repo, then a 2-replica VR normal-case
  sketch. An hour, not a day.

### 2. Apalache

- **What it proves**: bounded symbolic model checking (all executions up to
  length k), inductive-invariant checking for unbounded executions, and
  liveness-to-safety (experimental) for TLA+ / Quint specs; applied to
  consensus: Tendermint light client, agreement/accountable safety of
  Tendermint, ChonkyBFT (BFT consensus), ZKsync governance.
- **Tool/language**: Scala; translates TLA+/Quint to SMT (Z3).
- **Maintenance**: very active — Informal Systems; v0.52.x with self-contained
  CAV 2026 artifact (chapter published 24 July 2026, "The TLA+ Model Checker
  Apalache"); Docker image `ghcr.io/apalache-mc/apalache:main`; Java 17/21.
- **URL**: https://github.com/apalache-mc/apalache (site: https://apalache-mc.org)
- **Smoke test**: very plausible — `docker pull` + run a bundled example;
  then point it at a small VRR view-change spec within a timebox.

### 3. Quint

- **What it proves**: executable, typed specification language with TLA
  semantics; simulator, static analysis, and Apalache-backed bounded
  model-checking/inductive checks; used in production protocol work (ChonkyBFT
  BFT consensus, Aztec governance, ZKsync, Starknet decentralization models).
- **Tool/language**: TypeScript/Node; `npm i @informalsystems/quint -g`.
- **Maintenance**: very active — dedicated core team at Informal Systems
  (Pintor, Moreira, Malicevic, Boukhari); live-coding launch April 2025;
  `quint-connect` Rust integration emerging.
- **URL**: https://github.com/quint-co/quint (site: https://quint.sh)
- **Smoke test**: the easiest of all — npm install, REPL a two-phase-commit
  example in minutes; a VRR normal-case + view-change model is a natural
  first spec.

### 4. TLA+ toolchain (TLC + TLAPS + SANY), with the Raft TLA+ specification

- **What it proves**: the reference ecosystem. TLC explicit-state model
  checking; TLAPS hierarchical deductive proof with SMT/ATP backends (Z3,
  cvc5, veriT, Zenon, Isabelle, LS4). Ongaro's Raft TLA+ spec (dissertation,
  hosted at raft.github.io) is the canonical Raft model; corpus-adjacent
  Multi-Paxos TLAPS proof runs on the same stack.
- **Tool/language**: Java (TLC/SANY), TLAPS proof manager; TLA+ language.
- **Maintenance**: very active — TLA+ Foundation; continuous releases,
  CommunityModules, awesome-tlaplus curated index.
- **URL**: https://github.com/tlaplus/tlaplus (Raft spec via
  https://raft.github.io; index: https://github.com/tlaplus/awesome-tlaplus)
- **Smoke test**: very plausible — TLA+ Toolbox or docker; model-check Ongaro's
  raft.tla at small N within an hour.

## Tier 2 — runnable today, directly reconfiguration-relevant artifacts

### 5. MongoRaftReconfig / logless-reconfig (TLA+ + TLAPS proof)

- **What it proves**: LeaderCompleteness and StateMachineSafety of
  MongoRaftReconfig, a *logless dynamic reconfiguration* protocol for a
  Raft-derived system — the first published TLAPS safety proof of a
  reconfiguration protocol for a Raft-based system (OPODIS 2021). Formally
  stated inductive invariant + TLAPS proof tree; TLC-checked finite instances.
  This is the closest published analogue to a VRR reconfiguration proof.
- **Tool/language**: TLA+ / TLAPS.
- **Maintenance**: the repo is a 2021 frozen artifact (not an evolving tool),
  but it re-checks with the currently-maintained TLAPS; Zenodo artifact
  (Dec 2021); the protocol itself ships in MongoDB, and specs also live in
  the mongodb/mongo repo.
- **URL**: https://github.com/will62794/logless-reconfig (artifact:
  https://zenodo.org/records/5525484; paper: arXiv:2102.11960)
- **Smoke test**: plausible — clone, install TLAPS, re-check the proofs in a
  timebox; also ideal reading material for how a reconfiguration inductive
  invariant is structured.

### 6. etcd-io/raft TLA+ spec + trace validation

- **What it proves**: a TLA+ spec aligned to the etcd raft *implementation*
  (which differs from original Raft, e.g., in reconfiguration), plus
  model-based trace validation of real etcd executions against the spec
  ("Runtime Protocol Refinement Checking for etcd-raft"; "Validating System
  Executions with the TLA+ Tools", TLA+ Conf 2024).
- **Tool/language**: TLA+ / TLC + trace-validation tooling.
- **Maintenance**: active-ish — etcd-io/raft issue #111 (Nov 2023) and PR
  thread, TLA+ Conf 2024 talks; part of a live CNCF project, so the spec is
  expected to track the implementation.
- **URL**: https://github.com/etcd-io/raft/issues/111 (spec PR linked from
  there; etcd tracking: https://github.com/etcd-io/etcd/issues/17004)
- **Smoke test**: plausible — model-check the spec with TLC today; trace
  validation needs more setup.

### 7. Verus

- **What it proves**: SMT-based functional verification of Rust systems code
  (safe-Rust dialect → Z3); no off-the-shelf consensus proof to clone, but
  the platform is used at Microsoft/Amazon, and Anvil (2024) layered a
  TLA-style embedding on Verus to prove *liveness* of Kubernetes controllers —
  a pattern that could apply to a Rust consensus core (vrr-core itself).
- **Tool/language**: Verus / Rust / Z3.
- **Maintenance**: very active (verus-lang org; industry users).
- **URL**: https://github.com/verus-lang/verus
- **Smoke test**: plausible for a hello-world (counter/stack examples in the
  repo); verifying a real protocol module is a project, not a smoke test.

## Tier 3 — active research lines, runnable with more effort

### 8. Ivy (ivy-verif line) + IvyBench

- **What it proves**: decidable-fragment safety verification of distributed
  protocols (the tool Veil's DSL ports); IvyBench collects 54 protocol safety
  problems (Paxos variants, chain replication, 2PC, reliable broadcast …).
- **Tool/language**: Ivy DSL / Python, Z3.
- **Maintenance**: mixed — microsoft/ivy is archived read-only (Jan 2021) and
  development moved to the ivy-verif line (per the archived README, Sep 2020);
  the surrounding research line is alive: IvyBench, Scimitar (2024),
  QSM-Cutoff (CAV 2025). Confirm current commit cadence before adopting.
- **URL**: https://github.com/microsoft/ivy (archived; follow ivy-verif);
  https://github.com/aman-goel/ivybench
- **Smoke test**: plausible via existing docker/deb install; usability is the
  known weak point.

### 9. Automatic invariant-inference tools: IC3PO / fol-ic3 / I4 / DistAI / DuoAI / SWISS

- **What it proves**: fully automatic inference of quantified inductive
  invariants for distributed protocols in Ivy/mypyvy format; IC3PO produced
  the first fully automatic proof of Lamport's Paxos; DuoAI (OSDI'22) solves
  more complex Paxos versions; QSM-Cutoff (CAV 2025, Luo/Goel/Sakallah)
  continues the line.
- **Tool/language**: Python/Z3, Ivy-format inputs.
- **Maintenance**: research tools with recent-line activity (CAV 2025 paper,
  ivybench updates); individual repos vary.
- **URL**: https://github.com/aman-goel (IC3PO, fol-ic3, ivybench);
  DuoAI: https://github.com/ysyakhao/DuoAI (per OSDI'22 artifact)
- **Smoke test**: plausible on IvyBench problems; feed a VRR view-change Ivy
  model and see what invariant it infers — genuinely attractive for the
  position paper's open question about VRR's inductive invariants.

### 10. mypyvy + Scimitar

- **What it proves**: safety of protocol abstractions via invariant inference;
  per the 2026 federated-verification survey, mypyvy/Scimitar (Padon et al.
  2024) verified AbstractRaft + AsyncRaft safety; mypyvy is the symbolic
  transition-system platform (inductive invariant checking + inference, BMC,
  model minimisation — the machinery Veil adopted).
- **Tool/language**: Python / Z3, mypyvy DSL.
- **Maintenance**: research platform (wilcoxjay/mypyvy); alive through the
  2024–2025 Scimitar/cutoff work, not a product.
- **URL**: https://github.com/wilcoxjay/mypyvy (converter:
  https://github.com/tchajed/ivy-to-mypyvy)
- **Smoke test**: plausible — bundled examples run; expect research-grade
  ergonomics.

### 11. Bythos

- **What it proves**: compositional mechanized safety AND liveness of
  composite BFT protocols (HotStuff-style), via an embedding of TLA in Coq
  (CoqTLA); LTL-style liveness specs; knowledge/trust proof reuse across
  protocol families (CCS 2024).
- **Tool/language**: Coq (Rocq).
- **Maintenance**: active research (2024, cited through 2025–2026 liveness
  work); build requires a Coq toolchain.
- **URL**: paper: https://ilyasergey.net/assets/pdf/papers/bythos-ccs24.pdf
  (artifact via the paper/VERSE-adjacent repos)
- **Smoke test**: moderately plausible — Coq build of the framework plus a
  bundled example is a day-box, not an hour-box.

## Watch-list (2025–2026, not yet submoduleable)

- **CCF "Smart Casual" (Azure Confidential Ledger, 2025)** — modified Raft
  TLA+ model + TLC + simulation + trace validation wired into CI; reportedly
  caught 6 bugs pre-production. Evidence that this workflow runs at
  production scale; specs live in the microsoft/CCF repo — mineable even if
  the CI harness is internal.
- **Specula / TraceLink (PGo)** — 2025 TLA+ community work: agentic
  derivation of TLA+ specs from code, code-spec conformance via trace
  validation (PGo compiler). Early-stage.
- **Antithesis Raft Bug Bash (2026)** — commercial deterministic testing found
  1 new etcd bug and 12 new RedisRaft bugs (blog, July 2026). Not open /
  not submoduleable, but relevant to VRR implementation testing.
- **"Federated Formal Verification" Raft campaign (arXiv 2606.02019, 2026)**
  — claims a full-algorithmic-scope Raft proof (joint consensus, leadership
  transfer, log compaction, linearizable reads, dynamic reconfiguration)
  dispatched across multiple verification backends. Artifact availability
  unverified; treat as a claim, not a tool, until a repo is confirmed.

## Excluded (stale — no maintained tooling)

- **Verdi / verdi-raft** (uwplse) — landmark first full Raft SMR-safety +
  linearizability proof in Coq (PLDI'15/CPP'16), but the repos are frozen;
  Coq-version drift makes a hello-world a porting exercise. Read the
  PROOF_ENGINEERING.md; don't plan to build on it.
- **Disel** (DistributedComponents/disel) — DSepL compositional protocol
  verification in Coq; frozen research.
- **PSync** (POPL'16) — round-based partially-synchronous DSL over Verdi;
  frozen.
- **IronFleet / IronKV** (Dafny) — frozen (2015–2017 era); methodology still
  the reference for layered refinement proofs.
- **Velisarios, Chapar, Cure, bft-consensus-agda (archived 2022)** — frozen
  Byzantine-era artifacts.
- **Ongaro's raft.tla TLAPS LogCompleteness proof (2014)** — partial/mechanical
  only against admitted invariants; superseded by the Tier 1/2 stack.

## Bottom line for item08.5

Ranked by "submodule and kick the tires today", against the position paper's
safety-first VRR-reconfiguration goal:

1. **Veil** — the position paper's own recommendation; the tool is at its most
   runnable point ever (template repo, Veil 2.0 preview, active liveness
   work), and a VRR normal-case/view-change model is a realistic first
   timebox.
2. **Quint + Apalache** — the fastest path to an executable, checkable VRR
   model (npm + docker, minutes to hello-world), with real consensus protocol
   usage behind it; the natural place to prototype VRR reconfiguration before
   committing to Lean.
3. **TLA+ (TLC/TLAPS) with the logless-reconfig artifact** — MongoRaftReconfig
   is the only published, mechanically checked *reconfiguration* safety proof
   in the Raft family; re-running it against current TLAPS is a cheap,
   directly relevant exercise, and Ongaro's raft.tla + the etcd-aligned spec
   give ready-made Raft-family baselines.
