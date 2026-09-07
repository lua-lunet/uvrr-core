# Glossary of terms across the seven-source corpus

Sources (OCR'd papers in `.tmp/papers/ocr/`):

- `black-ltl-past-geatti-et-al-2021.md` — BLACK / LTL+Past
- `lean-auto-qian-et-al-2025.md` — Lean-auto
- `leanltl-vin-miller-fremont-2025.md` — LeanLTL
- `liskov-cowling-vr-revisited-2012.md` — Viewstamped Replication Revisited
- `multi-paxos-chand-liu-stoller-fm2016.md` — Multi-Paxos in TLA+/TLAPS
- `veil-lessons-auto-active-dafny2026.md` — Lessons from Veil (Dafny'26)
- `veil-pirlea-et-al-cav2025.md` — Veil (CAV'25)

Definitions are grounded in how the corpus uses each term; papers are cited by file name.

---

## 1. Lean 4 & ITP machinery

**Lean 4** — A dependently typed programming language and theorem prover whose logic is based on dependent type theory; the host proof assistant for Veil, LeanLTL, Lean-auto, Lean-SMT, and Duper, providing parsing, IDE integration, and metaprogramming "out of the box". `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`, `lean-auto-qian-et-al-2025.md`, `leanltl-vin-miller-fremont-2025.md`

**ITP (interactive theorem prover)** — A proof assistant in which proofs are developed interactively and machine-checked against well-accepted axioms (foundational verification); the corpus contrasts their expressivity and assurance with the months-to-years manual effort they require on large systems. `veil-pirlea-et-al-cav2025.md`, `lean-auto-qian-et-al-2025.md`

**Mathlib / Mathlib4** — Lean's mathematical library, described as the most prominent Lean project and the benchmark corpus on which Lean-auto was evaluated; LeanLTL also builds on Mathlib's `Set`. `lean-auto-qian-et-al-2025.md`, `leanltl-vin-miller-fremont-2025.md`

**Dependent type theory (λC, calculus of constructions) / PTS** — The logical foundation of Lean 4, Coq, and Agda: a Pure Type System (specified by sorts, axioms, and rules) in which types may depend on terms and other types; Lean-auto's preprocessing translates Lean 4 into this system. `lean-auto-qian-et-al-2025.md`

**λ→, λ*→, HOL / HOL\*** — Simply typed lambda calculus (λ→) is the term calculus of monomorphic higher-order logic (HOL), Lean-auto's translation target; λ*→ / HOL* is the variant with a countable number of universe levels, the intermediate system of Lean-auto's monomorphization, later erased by universe lifting via a bijective `GLift` construction. `lean-auto-qian-et-al-2025.md`

**Monomorphization vs. encoding-based translation** — The two approaches to translating a more expressive logic into a less expressive one: monomorphization finds instances of polymorphic symbols that behave monomorphically (sound, small outputs, incomplete — used by Lean-auto and Sledgehammer), while encoding-based translation (CoqHammer) encodes the richer system's constructs as predicates in the weaker one (almost complete but much larger, possibly unsound). `lean-auto-qian-et-al-2025.md`

**λ*→ abstraction, QMono, and Lean-auto's monomorphization stages (quantifier instantiation, universe lifting)** — The translation pipeline after preprocessing: quantifier instantiation is a saturation loop (`saturate`/`matchInst`) matching HOL* instances against hypotheses; λ*→ abstraction turns HOL* instances into HOL* variables on terms satisfying the QMono (quasi-monomorphic) predicate, targeting essentially higher-order problems (EHOPs); universe lifting erases universe levels via a bijective `GLift` construction. Along the way, typeclass instance arguments (`[self : HAdd α β γ]`, large expressions synthesized by typeclass inference) are absorbed into HOL* variables, and Lean's polymorphic, nested, and mutual inductive types are translated instance-by-instance when SMT solvers are the backend. `lean-auto-qian-et-al-2025.md`

**Definitional equality / equational theorems** — Lean 4's conversion relation under which syntactically different terms are equal; Lean-auto handles it via partial reduction, user `d[…]`/`u[…]` instructions, fingerprint-guarded `isDefEq` checks, and generated equational theorems between HOL* instances instead of costly full reduction. `lean-auto-qian-et-al-2025.md`

**Hammer / premise selection** — An ITP proof-automation tool with three components: premise selection (collecting the theorems needed — emulated in Lean-auto's evaluation by the theorems in each human proof), translation from ITP to ATP, and proof reconstruction; Sledgehammer and CoqHammer are prior hammers, and Lean-auto supplies the first translation for Lean 4. `lean-auto-qian-et-al-2025.md`

**Proof reconstruction / Duper** — Regenerating an ITP-side proof from an ATP's output so the Lean kernel checks it; supported for native provers — notably Duper, a proof-producing superposition prover in Lean 4 that works best as a Lean-auto backend — and, via Lean-SMT, for SMT solvers, removing untrusted solvers from the TCB at a 3–5x performance penalty. `lean-auto-qian-et-al-2025.md`, `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

**Lean-auto / Lean-SMT** — The two Lean-to-SMT translation libraries: Lean-auto is the interface between Lean 4 and automated theorem provers (Duper, Zipperposition, Z3, CVC5) implementing preprocessing + monomorphization, solving more Mathlib4 theorems than existing tools and serving as Veil's fast default translator; Lean-SMT produces small readable SMT-LIB queries (slower translation) and supports proof reconstruction. `lean-auto-qian-et-al-2025.md`, `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

**Lean tactics (simp and simp sets, aesop, grind, omega, nlinarith)** — Lean's automation: `simp` performs theorem-justified rewriting (Veil uses it for VC generation and translations — correct but slow, with large proof terms); curated simp sets such as LeanLTL's `push_ltl` push LTL satisfaction into first-order semantics so `linarith`/`omega`/`nlinarith` can finish; `aesop` is a best-first proof search; `grind` is the emerging, increasingly powerful proof-search tactic from which Veil users benefit transparently. `veil-lessons-auto-active-dafny2026.md`, `leanltl-vin-miller-fremont-2025.md`, `lean-auto-qian-et-al-2025.md`

**Lean metaprogramming (elaborators, environment extensions, MVC)** — Lean facilities for extending the language; the Veil lessons paper recommends keeping custom representations as environment extensions manipulated by one's own elaborators, structuring the verifier as model-view-controller, and controlling one's representations rather than reusing Lean's section variables. `veil-lessons-auto-active-dafny2026.md`

**Shallow vs. deep embedding** — Veil is shallow at the object level (its actions are ordinary Lean definitions, extending `do` notation with `require`, `assert`, `pick`) for full ecosystem compatibility, while keeping its own meta-level representations; a deep embedding instead encodes the DSL's syntax and semantics as data types. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`

**Trusted computing base (TCB)** — The components a user must trust: auto-active verifiers trust the language axiomatisation, VC generator, and SMT solvers, whereas Veil reduces its TCB to its frontend, the Lean compiler, and the Lean kernel — ideally just the kernel via transparent desugaring and proof reconstruction; a verifier whose VC generator is proven sound in the assistant (as Veil's is) is called foundational. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`

**LeanLTL** — A Lean 4 library unifying linear temporal logics over finite and infinite traces, allowing arbitrary Lean expressions inside formulas (via the `LLTL[...]` macro and strong/weak get notation compared to idiom brackets and monad arrows), with proved embeddings of LTL and LTLf into its semantics. `leanltl-vin-miller-fremont-2025.md`

**Trace / TraceSet / TraceFun** — LeanLTL's core types: a `Trace σ` is a nonempty, possibly finite sequence of states (option-valued with a defined prefix); a `TraceSet σ` is the semantic view of a formula as its set of satisfying traces (`t ⊨ p`); a `TraceFun σ α` maps traces to `Option α`, with `sget`/`wget` binding values as false/true on `none`. `leanltl-vin-miller-fremont-2025.md`

## 2. Temporal logic

**LTL (Linear Temporal Logic)** — The de-facto standard temporal specification language over infinite traces, with boolean connectives and future operators X (next), U (until), R (release), and shorthands F (finally) and G (globally); decidable, with practical satisfiability, model-checking, and synthesis tools. `leanltl-vin-miller-fremont-2025.md`, `black-ltl-past-geatti-et-al-2021.md`

**LTLf (LTL on finite traces)** — LTL whose semantics account for the end of a finite trace, splitting next into strong next Xˢ (false at the end) and weak next Xʷ (true at the end); motivated by runtime monitoring. `leanltl-vin-miller-fremont-2025.md`

**LTL+Past** — LTL extended with past operators: Y (yesterday), Z (weak yesterday), S (since), T (triggered), with shorthands O (once) and H (historically); it adds no expressive power over LTL but is exponentially more succinct, letting useful properties be stated more naturally. `black-ltl-past-geatti-et-al-2021.md`

**LTLMT / LTLfMT (LTL(f) Modulo Theories)** — Variants replacing propositions with atoms of an underlying theory (in the vein of SMT) such as linear integer arithmetic, bitvectors, or nonlinear real arithmetic; (semi-)decidable for some theories, keeping tools fully automatic but excluding undecidable theories. `leanltl-vin-miller-fremont-2025.md`

**Satisfiability checking** — Deciding whether a formula admits a model; the central problem of the BLACK paper, attacked historically by tableaux, reduction to model checking, temporal resolution, and automata-theoretic techniques. `black-ltl-past-geatti-et-al-2021.md`

**Tableau (one-pass, tree-shaped)** — A decision procedure in which a tree of nodes labeled with formula sets is expanded by rules (disjunction, conjunction, until, since, release, triggered) until branches are accepted or rejected; Reynolds' LTL system is one-pass and tree-shaped, and Geatti et al. extend it to past operators. `black-ltl-past-geatti-et-al-2021.md`

**Tableau rules: STEP, FORECAST, LOOP, PRUNE, X-eventuality** — The temporal machinery: STEP moves X-formulas into the next state's label; FORECAST nondeterministically guesses formulas needed to fulfill past requests; an X-eventuality `X(φ₁Uφ₂)` requests later fulfillment; LOOP accepts a looping branch with all eventualities fulfilled; PRUNE rejects a branch an impossible eventuality would unroll forever. `black-ltl-past-geatti-et-al-2021.md`

**NNF, closure, stepped normal form** — The formula-level notions of the encoding: negation normal form (negations on atoms only, enabled by keeping release/triggered/weak-yesterday primitive); the closure C(ψ) of subformulas and shifted temporal subformulas defining requests; and the stepped normal form snf(φ) unfolding each operator per its expansion rule, used in the k-unraveling. `black-ltl-past-geatti-et-al-2021.md`

**REMOVEPAST (equisatisfiable translation)** — A Tseitin-style translation replacing past subformulas with fresh proposition letters forced to replicate past semantics by axioms; the result is equisatisfiable (not equivalent) with the input, grows only linearly, and lets any future-only LTL tool handle the past. `black-ltl-past-geatti-et-al-2021.md`

**BLACK** — The Bounded LTL sAtisfiability Checker: iteratively SAT-encodes tableau branches up to depth k (via the k-unraveling `[φ]ᵏ`, base encoding `|φ|ᵏ`, and termination encoding `|φ|ᵀᵏ`), benchmarked against random formulas, a Kripke-structure-based `crscounter` family, and the nuXmv model checker (with MathSAT as its best backend); its direct past encoding outperforms the translation. `black-ltl-past-geatti-et-al-2021.md`

**nuXmv** — The symbolic model checker (with `sbmc` — simple bounded model checking — and `klive`/K-Liveness modalities) that, per the paper, is the only widely available tool directly supporting past operators; the experimental baseline. `black-ltl-past-geatti-et-al-2021.md`

**PTL / LS4** — Propositional Temporal Logic and its prover; the TLAPS backend invoked for temporal reasoning, e.g., concluding `Spec ⇒ □Inv` from `Spec ⇒ □(Inv ⇒ Safe)`-style steps. `multi-paxos-chand-liu-stoller-fm2016.md`

## 3. SMT/ATP automation

**SMT (Satisfiability Modulo Theories) / SMT-LIB** — Deciding satisfiability of first-order formulas modulo background theories; the automation backbone of auto-active verifiers (Dafny, Ivy, Veil) and of LTL modulo theories, with SMT-LIB as the query language Lean-SMT and Lean-auto translate to. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`, `leanltl-vin-miller-fremont-2025.md`, `lean-auto-qian-et-al-2025.md`

**Z3 / cvc5** — The SMT solvers used as backends by Veil (defaulting to cvc5 with a 5-second timeout, trying the other solver on failure) and as SMT backends of Lean-auto; Z3 is also among TLAPS's supported SMT backends. `veil-pirlea-et-al-cav2025.md`, `lean-auto-qian-et-al-2025.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**TPTP / Zipperposition** — The TPTP TH0 interchange format and a higher-order superposition prover; one of the three ATP kinds used to evaluate Lean-auto. `lean-auto-qian-et-al-2025.md`

**FOL (first-order logic)** — The logic of most ATPs and of tools like Ivy, mypyvy, and Veil's DSL; because Veil's VCG may emit higher-order Lean goals, custom tactics destruct higher-order structures, hoist quantifiers to the top level, and case-split to reduce goals to FOL for SMT. `veil-pirlea-et-al-cav2025.md`, `lean-auto-qian-et-al-2025.md`

**EPR (Effectively Propositional Logic) / decidable fragment** — A decidable fragment of first-order logic in which Ivy-style tools target push-button verification; some protocol encodings (Single-Decree and Vertical Paxos, Suzuki-Kasami with integer indices, Reliable Broadcast) fall outside EPR, motivating Veil's verification beyond EPR and Ivy's `complete=fo` flag. `veil-pirlea-et-al-cav2025.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**Verification condition (VC) / VCG / proof obligation** — The obligation a verifier generates relating a transition to its specification; Veil's VC generator is proven sound with respect to its semantics, and its VCG may emit higher-order goals; TLAPS likewise decomposes proofs into obligations `P ⇒ Q` sent to backend provers. `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**Weakest precondition (WP)** — A predicate transformer `WP σ ρ ≜ (ρ → σ → Prop) → (σ → Prop)` returning the weakest pre-state guaranteeing a postcondition; Veil encodes atomic commands this way because composing relational transitions would introduce higher-order quantification over state components that SMT cannot discharge. `veil-pirlea-et-al-cav2025.md`

**Bounded model checking (BMC)** — Symbolically searching bounded-length executions via SMT/SAT; Veil exposes `sat trace ... by bmc_sat` (finding a viable execution, checking non-vacuousness) and `unsat trace ... by bmc` (proving no such execution exists, including `any N actions`), and nuXmv's `sbmc` is the same idea. `veil-pirlea-et-al-cav2025.md`, `black-ltl-past-geatti-et-al-2021.md`

**Counterexample to induction (CTI)** — A concrete state showing a candidate invariant is not preserved; Veil's `#check_invariants` displays one so the user can add an invariant clause eliminating it, iterating toward an inductive invariant. `veil-lessons-auto-active-dafny2026.md`

**Model minimisation** — Reducing a solver counterexample's sort and relation cardinalities via incremental SMT queries (as in mypyvy) before display; in Veil's experience crucial for making protocol models understandable. `veil-pirlea-et-al-cav2025.md`

**Push-button verification / solver-guided interactive verification** — Fully automated invariant verification with zero manual proof effort (Veil's `#check_invariants`), and its hybrid style in which users write Lean proofs and intermittently invoke `solve_clause`, which reports success, a minimised counterexample, or unknown. `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

## 4. Distributed protocols & VR/VRR

**Viewstamped Replication (VR)** — Liskov and Cowling's replication protocol for an asynchronous network tolerating at most f crash failures among 2f+1 replicas: it runs a replicated state machine — replicas start in the same state and execute the same deterministic operation sequence — with a primary ordering client requests, backups monitoring it, and three sub-protocols (normal-case processing, view changes, recovery) keeping the service correct without disk writes; non-deterministic operations (e.g., reading local clocks) are tamed by having the primary compute a predicted value stored in the log and used at execution. `liskov-cowling-vr-revisited-2012.md`

**Environment and fault model (asynchronous network, crash vs. Byzantine)** — VR handles only crash failures (a machine is either fully correct or completely stopped), not Byzantine failures where nodes may fail arbitrarily; its network is asynchronous — messages may be lost, delayed, reordered, or duplicated, and non-arrival indicates nothing about the sender, though repeated sends eventually deliver — which is why each step needs f+1 of 2f+1 replicas and why quorums of f+1 are minimal. `liskov-cowling-vr-revisited-2012.md`

**Quorum / quorum intersection property** — The f+1-replica group processed at each protocol step; correctness depends on quorums of consecutive steps intersecting, so at least one participant of the next step knows what happened in the previous one; in Paxos terms, the quorum system Q must satisfy pairwise overlap (QuorumAssumption). `liskov-cowling-vr-revisited-2012.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**View change (VR view change), view-number, viewstamp** — The protocol replacing a failed primary: replicas exchange STARTVIEWCHANGE/DOVIEWCHANGE messages, the new primary selects the most recent log from f+1 replicas and announces STARTVIEW; its correctness condition is that every operation executed via an up-call (the call into the service code, performed only for committed operations) survives into the new view in the same order, relying on quorum intersection. The view-number monotonically identifies the period in which one replica is primary, filtering stale messages and letting clients track the primary; original VR instead ordered conflicting operations by a viewstamp ⟨view-number, op-number⟩, while the 2012 protocol takes the log from the latest previous active view. `liskov-cowling-vr-revisited-2012.md`

**Replica state (log, op-number, commit-number, client-table)** — The VR layer's state at a replica: the log of ordered requests, the op-number of the most recently received request, the commit-number of the most recently committed one, and a client-table recording each client's latest request number and result for exactly-once processing. `liskov-cowling-vr-revisited-2012.md`

**Recovery protocol / nonce** — VR's protocol for a crashed replica to rejoin: it sends RECOVERY with a nonce, collects f+1 RECOVERYRESPONSE messages (log data only from the primary), and resumes in status normal only with a state at least as recent as when it failed; the nonce prevents mixing responses across recoveries. `liskov-cowling-vr-revisited-2012.md`

**State transfer (with checkpoints and Merkle trees)** — The mechanism by which a lagging (not crashed) replica catches up via GETSTATE/NEWSTATE messages, truncating its log at its commit-number when it learns of a later view, and by which new replicas become up to date at the start of a reconfiguration epoch; checkpoints (application snapshots taken every O operations, enabling log garbage collection) and a Merkle tree over snapshot pages (so a recovering replica fetches only differing pages) make transfer and recovery efficient. `liskov-cowling-vr-revisited-2012.md`

**Reconfiguration / epoch-number** — VR's protocol for changing group membership and the failure threshold: a RECONFIGURATION client request commits through the old group, the system moves to a new epoch (starting at view 0), new replicas complete state transfer (STARTEPOCH/EPOCHSTARTED messages), and replaced replicas shut down only after f'+1 new replicas hold the state. `liskov-cowling-vr-revisited-2012.md`

**VR optimizations (witnesses, batching, leases, fast reads)** — Performance techniques adopted from Harp and PBFT: f witnesses store no service state and serve only view changes and recovery; batching runs the protocol once per batch of requests under load; leases let the primary answer reads unilaterally (with f leases and loosely synchronized clocks), while stale-tolerant reads can be served by backups supporting causality via last-request-numbers. `liskov-cowling-vr-revisited-2012.md`

**Paxos, Basic Paxos, Multi-Paxos** — Lamport's consensus algorithm: Basic Paxos agrees on a single value, Multi-Paxos on a sequence of values indexed by slots, with ballots shared across slots and previously failed slots detected and reused; the paper specifies Multi-Paxos by minimally extending Lamport et al.'s Basic Paxos TLA+ specification. `multi-paxos-chand-liu-stoller-fm2016.md`

**Proposers, acceptors, learners, ballots** — Paxos's roles and numbering: proposers propose values, acceptors vote by sending 2b messages, learners discover chosen values from a quorum's votes (roles may be co-located); a ballot b is a totally ordered proposal number, and acceptors promise not to vote in ballots below the highest they have seen. `multi-paxos-chand-liu-stoller-fm2016.md`

**Chosen / ChosenIn / VotedForIn / SafeAt / WontVoteIn** — The auxiliary safety predicates: value v is chosen for slot s iff some quorum of acceptors voted (b, s, v) (VotedForIn, realized by a 2b message); SafeAt(b,s,v) asserts no value but possibly v can be chosen for s below ballot b; WontVoteIn(a,b,s) says acceptor a has moved past b for s. Lemmas VotedInv and VotedOnce reduce Safe to these. `multi-paxos-chand-liu-stoller-fm2016.md`

**Safety / liveness** — The two standard property classes: Multi-Paxos's Safe (at most one value chosen per slot) is the property proven in TLAPS; VR's safety states committed operations survive view changes and reconfiguration in order, while liveness is that requests execute when enough replicas (f+1, or f+2 during recovery) can communicate. `multi-paxos-chand-liu-stoller-fm2016.md`, `liskov-cowling-vr-revisited-2012.md`

**TypeOK / AccInv / MsgInv** — The three kinds of invariants in the Multi-Paxos proof: type invariants on variable domains (TypeOK), process invariants over acceptor local data (AccInv, over pBal/aBal/aVoted), and message invariants over 1b/2a/2b (and preempt) messages (MsgInv); together Inv ≜ TypeOK ∧ AccInv ∧ MsgInv gives Spec ⇒ □Safe. `multi-paxos-chand-liu-stoller-fm2016.md`

**Inductive invariant, invariance lemma, increment** — An invariance lemma asserts a predicate continues to hold across one system step and is reused across many proof branches (5 lemmas used in 27 places); by induction over the disjuncts of Next it establishes Spec ⇒ □Inv, the standard shape of an inductive-invariant safety proof. An increment is a new element (e.g., the message a phase sends) added to a set in one step: the proof splits old elements — discharged by invariance lemmas — from the increment, proving each invariant conjunct for the increment from the phase's definition. `multi-paxos-chand-liu-stoller-fm2016.md`

**Preemption** — The optimization letting a proposer abandon a preempted ballot: acceptors receiving a too-low ballot reply with a preempt message carrying the highest ballot seen, and the Preempt action advances pBal to a higher unused ballot, avoiding wasteful low-ballot messages. `multi-paxos-chand-liu-stoller-fm2016.md`

**Veil** — The framework for automated and interactive verification of transition systems, embedded in Lean 4: specifications in an Ivy-inspired FOL DSL, a soundness-proved VC generator built on weakest-precondition encodings and Lean's do-notation (BigStep two-state relations `σ → ρ → σ → Prop`), SMT automation via Lean-auto/Lean-SMT with cvc5/Z3, BMC, and seamless fallback to interactive Lean proofs; evaluated on 16 case studies including Suzuki-Kasami, Paxos variants, Reliable Broadcast, the Stellar Consensus Protocol (via a typeclass-based abstraction), and Rabia. `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

**`#check_invariants` (with `?` and `!` variants)** — Veil's command checking every invariant clause is preserved by every transition via SMT; `?` prints theorem templates for interactive proof of every invariant, `!` only for those automation failed on; when an invariant is not preserved a CTI is displayed. `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

**Ivy / RML (Relational Modelling Language)** — The multi-modal verification tool for distributed algorithms whose DSL Veil's language almost verbatim ports; Ivy verifies decidable FOL properties via SMT and is over 10x faster than Veil, but is not foundational and its escape-hatches lack interactive-proof ergonomics. `veil-pirlea-et-al-cav2025.md`, `veil-lessons-auto-active-dafny2026.md`

## 5. Verification tools compared

**TLA⁺ / TLA (Temporal Logic of Actions)** — Lamport's specification language and its underlying logic: states assign values to variables, actions relate unprimed (current) and primed (next) values, and `Spec ≜ Init ∧ □[Next]_vars` defines behaviors that either execute an action or stutter; the stuttering-plus-nondeterminism model captures message loss, delay, reordering, duplication, and process crashes. `multi-paxos-chand-liu-stoller-fm2016.md`, `veil-lessons-auto-active-dafny2026.md`

**TLC** — The TLA⁺ concrete-state model checker used to quickly test protocol designs (the "fancy DSL for breadth-first search" usage pattern); Veil offers TLC-style concrete-state model checking by compiling its Lean definitions. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**Apalache** — The tool for symbolic model checking of TLA⁺; available in principle but "less user-friendly and outside the default path" of TLA⁺ practice. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`

**TLAPS (TLA⁺ Proof System) and its backends / hierarchical proofs** — The tool that mechanically checks hierarchical TLA⁺ proofs by generating obligations for backend provers — by default CVC3, Zenon (an extensible ATP producing checkable proofs), and Isabelle, with Z3, veriT, Yices, and the PTL prover LS4 also supported; used for the Multi-Paxos safety proof, though it relies on trusted translations to its backends (unlike Veil). Proofs follow Lamport's step-by-step hierarchical style (`⟨x⟩y. Assertion BY e₁,…,eₘ DEF d₁,…,dₙ`, with ASSUME/PROVE, CASE, USE DEF, and QED steps); the Multi-Paxos proof tree reaches 10 levels. `multi-paxos-chand-liu-stoller-fm2016.md`, `veil-pirlea-et-al-cav2025.md`

**Isabelle/HOL** — The polymorphic-HOL proof assistant: a TLAPS backend, the substrate of its Sledgehammer hammer, and the tool in which the Stellar Consensus Protocol abstraction soundness was manually proven before being ported to Veil. `veil-pirlea-et-al-cav2025.md`, `lean-auto-qian-et-al-2025.md`

**Dafny / F\* / Viper / Boogie (auto-active verifiers)** — Verifiers that provide substantial SMT automation while letting users guide proof search with additional assertions; their TCBs are large (language axiomatisation, VC generator, solvers), their assertion language is first-order, and Boogie/Viper are typical bespoke translation targets; Dafny also hosted IronFleet. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`, `multi-paxos-chand-liu-stoller-fm2016.md`

**Alloy** — A popular modelling tool for state-transition systems (with the TLA⁺ toolbox) that allows only bounded verification by assuming each sort is finite; one of the inspirations for Veil's DSL. `veil-pirlea-et-al-cav2025.md`

**Concrete-state vs. symbolic model checking** — The two model-checking modes the corpus contrasts: TLC-style enumeration of concrete states for testing designs vs. SMT/SAT-based symbolic exploration (Apalache, Veil's bmc, BLACK's encoding) that avoids explicit state explosion. `veil-lessons-auto-active-dafny2026.md`, `veil-pirlea-et-al-cav2025.md`, `black-ltl-past-geatti-et-al-2021.md`

**IronFleet** — The project that verified a state-machine-replication system with Multi-Paxos at its core, specified in Dafny; cited as proving both safety and liveness but at the cost of over 30,000 lines of proof. `multi-paxos-chand-liu-stoller-fm2016.md`

**mypyvy / UPVerifier** — Research tools that, like Ivy, verify distributed protocols using decidable FOL fragments and ATPs; mypyvy contributes the BMC capability and the model-minimisation approach Veil adopts. `veil-pirlea-et-al-cav2025.md`

**Verus / RefinedC / RefinedRust / Diaframe / Why3 / Jahob** — Neighboring verifier frameworks in Veil's related work: Why3 and Jahob as spiritual predecessors relaying VCs to third-party provers; RefinedC, RefinedRust, and Diaframe as foundational mostly-automated embeddings in the Rocq prover using tactic-based rather than FOL-solver automation. `veil-pirlea-et-al-cav2025.md`

**Rocq (formerly Coq)** — The proof assistant hosting the foundational verifier embeddings above and the Rabia formalization whose extra invariants (some requiring induction on phases) Veil reproduced interactively — also catching a discrepancy between the Ivy and Rocq invariant sets. `veil-pirlea-et-al-cav2025.md`

**TLPVS** — A PVS library for reasoning about LTL properties with theory support; per LeanLTL, the most similar prior work, though it does not consider finite traces. `leanltl-vin-miller-fremont-2025.md`

**IvyBench** — The benchmark collection of distributed-protocol verification problems from which 9 of Veil's 16 case studies were drawn. `veil-pirlea-et-al-cav2025.md`
