# Proving Viewstamped Replication Revisited in Lean 4: A Position Paper

*Corpus: the seven OCR'd papers in `.tmp/papers/ocr/`. Every claim below is traceable to those files, cited by filename. Statements not supported by the corpus are explicitly flagged as speculation.*

## 1. Thesis

Viewstamped Replication Revisited (VRR) is an attractive target for machine-checked proof in Lean 4 because its safety argument is quorum-based, first-order in character, and concentrated at exactly the points where automation-assisted invariant checking shines: view changes, recovery, and — above all — reconfiguration. The corpus supports a precise claim: Lean 4, via Veil and Lean-auto, offers push-button SMT verification for decidable first-order invariants *and* a foundational interactive fallback when automation fails, in one tool, with a proven-sound verification-condition generator (`veil-pirlea-et-al-cav2025.md`). No published machine-checked proof of full VRR in Lean exists in this corpus; the defensible position is therefore not "Lean has verified VR" but "the Lean ecosystem is the strongest available fit for a safety-first verification of VRR's reconfiguration protocol, while liveness and temporal-property support remain future work everywhere in the corpus."

## 2. Evidence

### 2.1 Veil: automated and interactive verification in Lean (CAV'25)

Veil is "an open-source framework for automated and interactive verification of transition systems... implemented on top of the Lean proof assistant," which produces verification conditions in first-order logic discharged by SMT solvers, with an interactive mode when automation fails (`veil-pirlea-et-al-cav2025.md`, Abstract). Its evaluation is directly relevant to consensus-adjacent protocols:

- It verified all 16 distributed-protocol case studies automatically via `#check_invariants`, including two Paxos variants from Padon et al. and benchmarks from IvyBench, where "Ivy has failed to verify two benchmarks" (§4.1).
- On non-EPR encodings — Single-Decree Paxos, Vertical Paxos, Suzuki-Kasami with integer indices, Reliable Broadcast — "Veil manages to verify all of them," while Ivy times out on Paxos and Vertical Paxos (§4.2).
- Its VC generator is "proven sound with respect to the semantics of its specification language" (Abstract; §3.2), and it supports SMT-based bounded model checking (`sat trace`/`unsat trace`) to establish non-vacuousness and bounded safety (§2.2).
- When an invariant is not inductive, Veil minimises and displays a counterexample model (§3.4); when a query falls outside the decidable fragment, `#check_invariants?` emits a theorem template for interactive proof (§2.4).

For a reconfiguration protocol — where the hard obligations (epoch-aware message filtering, the "new epoch starts in view 0" rule, the f'+1 EPOCHSTARTED shutdown rule; see §2.6 below) are two-state invariants over unbounded replica state — this automated/interactive split is exactly the working style the VRR proofs would demand.

### 2.2 Veil's Dafny'26 lessons

The companion paper reports 18 months of building Veil entirely inside Lean (`veil-lessons-auto-active-dafny2026.md`). Key transferable findings:

- The embedding thesis: "All goals are Lean goals. All proofs are Lean proofs. All UI is Lean UI" (§1). Benefits listed include seamless fallback to interactive proofs, expressive higher-order logic reducible to FOL for SMT, and a reduced TCB — users trust "the Veil frontend... the Lean compiler... and the Lean kernel" (§2.2).
- Proof reconstruction via Lean-SMT removes SMT solvers from the TCB "at a 3-5x performance penalty" (§2.1).
- Performance is the persistent cost: Veil is "more than 10x slower than Ivy," with the largest case study at 160 seconds (§3.5). Verified translation through `simp` is sound by construction but slow (§3.5).
- Engineering lessons: metaprogramming is "a double-edged sword"; the rewrite to Veil 2.0 imposed an MVC architecture and custom representations (§3.1–3.2); the shallow object-level embedding may not suit languages "too different from Lean due to either being untyped (e.g., TLA⁺) or because their memory model... [is] too different" (§4).

### 2.3 Lean-auto: the automation bridge (CAV'25)

Lean-auto provides "general-purpose, ATP-based proof automation in Lean 4 for the first time," translating Lean 4's dependent type theory into (monomorphic) HOL by preprocessing plus monomorphization, with "soundness of the main translation procedure... guaranteed" and full proof reconstruction for one supported ATP class (`lean-auto-qian-et-al-2025.md`, Abstract, §1). Veil uses Lean-auto as its default SMT translator and Lean-SMT as the alternative (`veil-pirlea-et-al-cav2025.md`, §3.3). This is the mechanism that lets a VR specification written in Veil's Ivy-like DSL be discharged by cvc5 or Z3 without leaving Lean.

### 2.4 The TLA+ ecosystem, as the corpus characterizes it

The Veil papers describe TLA+ as "de-facto... the most popular tool... for prototyping and modelling" transition systems, with TLC for "concrete-state model checking of protocol designs," plus Apalache for symbolic checking and TLAPS for deductive proof — the latter two sitting outside TLA+'s default path (see §3 below). The Multi-Paxos FM'16 paper demonstrates both the ecosystem's maturity and its risks: it delivers "a complete proof written and automatically checked using TLAPS" of Multi-Paxos safety, with over 750 proof obligations checked in about 3 minutes (`multi-paxos-chand-liu-stoller-fm2016.md`, §1). But it is equally instructive that the authors discovered "an undocumented bug... in TLAPS" that had made an earlier claimed-complete proof incomplete, and had to add missing proof steps (`multi-paxos-chand-liu-stoller-fm2016.md`, §1, items 1–2). The proof is explicitly safety-only: "Because this work aims at proving safety, we do not specify any constraints on receiving messages" (§3.1 on Phase 1a). This is the corpus's best evidence of what a TLAPS-based VRR effort would look like — and of the trust and tooling caveats.

### 2.5 LTL and past-time tooling

LeanLTL is "a unifying framework for linear temporal logics in Lean 4," supporting finite and infinite traces, embedding arbitrary Lean expressions in LTL formulas, with proved embeddings of LTL and LTLf and simp-set-based automation (`leanltl-vin-miller-fremont-2025.md`, Abstract, §1, §3.3). Critically, its authors list as *future work* "support for other LTL variants, including past-time and bounded-time operators" (§5) and integration of LTL/LTLMT decision procedures (§3.3).

Outside Lean, BLACK shows what mature past-time support looks like: LTL+Past "does not add expressive power, but does increase the usability of the language," being "exponentially more succinct than LTL" (`black-ltl-past-geatti-et-al-2021.md`, Abstract, §1). BLACK implements both an equisatisfiable REMOVEPAST translation (Theorem 4) and a direct SAT encoding of past-aware tableau rules, with soundness and completeness proved (Theorem 10). Past-time operators are natural for reconfiguration reasoning — properties like "every epoch started only after the previous one committed" are historically-phrased — but no tool in the corpus connects this machinery to Lean today.

### 2.6 The protocol itself: VR Revisited

Liskov and Cowling's VRR (`liskov-cowling-vr-revisited-2012.md`) gives the proof target. The safety-critical skeleton:

- Groups of 2f+1 replicas; quorums of f+1; correctness "depends on the quorum intersection property" (§2.2).
- View change: the correctness condition is that "every operation that has been executed by means of an up-call... must survive into the new view in the same order" (§4.2); the new primary selects the log with the largest v' (latest normal view), ties broken by largest op-number; committed operations survive via quorum intersection (§4.2, §8.1). A subtlety the paper stresses: replicas must stop accepting PREPAREs from earlier views once view change starts, else two primaries can commit divergently (§8.1).
- Recovery: a recovering replica must rejoin "in a state at least as recent as when it failed," using a nonce and f+1 RECOVERYRESPONSEs including the latest known primary's (§4.3, §8.2); the two-exchange view change is necessary to prevent a recovered stale replica from corrupting the next view (§8.2).
- Reconfiguration (§7): triggered by a RECONFIGURATION client request processed through the *old* group's normal-case protocol; on commit the epoch-number increments; STARTEPOCH messages reach new replicas; new replicas state-transfer from old ones and only set status to normal when complete; replaced replicas shut down only after f'+1 EPOCHSTARTED messages; the new epoch starts "in view 0" — explicitly, because reusing the old view number could yield "two primaries in the new group" (§8.3); view change and recovery are modified for epoch-awareness (§7.2); correctness rests on the RECONFIGURATION request being the last committed request of the epoch and on old replicas not shutting down early (§8.3).

Section 8's correctness arguments are informal prose — precise, but not machine-checked. That gap is the opportunity.

## 3. Comparison: Lean vs TLA+, as the Veil authors actually frame it

The corpus contains a specific, often-misquoted passage. The Veil authors write (`veil-lessons-auto-active-dafny2026.md`, §2):

> "We developed it out of frustration with existing tools used for modelling and verifying distributed protocols. In particular, we were unsatisfied with both TLA+ [12] and Ivy [16, 20], the two most popular tools in this space. Both of these tools are great in some respects: TLA+ is amazing for modelling protocols and quickly testing them via concrete state model checking using TLC [30]. In principle, one can also symbolically model check TLA+ specifications using Apalache [10, 19] and semi-automatically prove properties using TLAPS [4], but the tooling for these tasks is less user-friendly and outside the default path, and thus, in practice, many people treat TLA+ as a fancy DSL for breadth-first search [9]. Ivy is extraordinarily powerful for proving decidable properties of distributed protocols using SMT. It also supports symbolic model checking and even tactic-based proofs [16], but again, these feel like bolt-ons rather than core features of the tool."

The framing is therefore *not* a claim that one tool "provides no automation" while the other "lacks a logical foundation." It is a recognition that TLA+ is excellent at what it does by default (modelling, TLC model checking), that its deductive and symbolic-checking capabilities exist but "sit outside the default path," and that Ivy's interactive capabilities feel like bolt-ons. Veil's wager is that embedding everything in Lean eliminates the seams: decidable-fragment automation, symbolic BMC, and full interactive proving become modes of one foundational tool, with the VC generator itself proven sound (`veil-pirlea-et-al-cav2025.md`, §1, §3.2). Against this, TLA+ with TLAPS has a demonstrated track record on a Paxos-family safety proof (Multi-Paxos FM'16), which no Lean tool in the corpus yet has for any VR-family protocol.

## 4. Gaps: what the corpus does not support

1. **No published full VR/VRR verification in Lean.** Veil's 16 case studies include Paxos variants, Rabia, SCP, and Suzuki-Kasami — not Viewstamped Replication (`veil-pirlea-et-al-cav2025.md`, §4.1). Nothing in the corpus contradicts feasibility, but nothing demonstrates it for VRR.
2. **LeanLTL's past-time (and bounded-time) operators are future work, not present** (`leanltl-vin-miller-fremont-2025.md`, §5). Until then, historical-phrased reconfiguration properties must be rewritten as present-tense invariants over explicit history variables — a manual, correctness-critical encoding step the corpus does not evaluate.
3. **Liveness is out of scope for every Lean tool in the corpus.** Veil's machinery is invariant/safety checking and BMC (`veil-pirlea-et-al-cav2025.md`, §2.2–2.4); its papers' future-work lists (invariant inference, protocol composition, embedded program verifiers) do not include liveness. Multi-Paxos FM'16 is likewise safety-only. VRR's liveness arguments (e.g., "the system is live because (1) the base protocol is live... (3) old replicas do not shut down until new replicas are ready"; `liskov-cowling-vr-revisited-2012.md`, §8.3) would need machinery the corpus does not supply in Lean.
4. **Cimatti et al. (FMCAD'04) is not in the corpus.** BLACK cites "Cimatti et al. [7]" for the Counter(N) Kripke structure used in its benchmarks (`black-ltl-past-geatti-et-al-2021.md`, §5), but that paper itself is not among the seven files and is paywalled; no claim here rests on its contents beyond what BLACK's text states.
5. **VRR's own correctness discussion is informal** (`liskov-cowling-vr-revisited-2012.md`, §8: "we provide an informal discussion of the correctness of the protocol"). The corpus contains no mechanized proof of VRR in any tool, Lean or otherwise.

*Speculation, flagged:* any estimate of how many inductive invariants a VRR-in-Veil development would need, or whether reconfiguration's cross-epoch state (old-configuration, epoch-number, transitioning status) stays within a decidable FOL fragment, is extrapolation from the Paxos-family results, not a corpus fact.

## 5. Recommendation and open questions

**Recommendation.** Model VRR in Veil's DSL with reconfiguration as a first-class action family, and pursue a *safety-first* strategy: (i) use `sat trace` BMC to establish non-vacuousness of normal-case, view-change, and reconfiguration traces; (ii) express the §8.1–8.3 safety conditions as invariants (committed operations survive view changes and epoch transitions in order; at most one active primary per view/epoch; recovering replicas rejoin no staler than they failed); (iii) let `#check_invariants` discharge what falls in the decidable fragment and port the residue to interactive Lean proofs via `#check_invariants?`; (iv) optionally enable Lean-SMT proof reconstruction when SMT solvers must leave the TCB, accepting the 3–5x penalty. Defer liveness and temporal properties until LeanLTL's operator set (or a successor) matures; until then, encode history explicitly. Treat TLAPS+Multi-Paxos as the reference point for effort estimation — hundreds of obligations even for a minimally-extended Basic Paxos — while noting that Veil's non-EPR Paxos results suggest the SMT path can carry further than Ivy-style tools.

**Open questions.**
- Can VRR's reconfiguration be stated in decidable FOL (Ivy-style) at all, or does epoch-indexed history force the interactive path from the start? (Unanswerable from the corpus.)
- Do Veil's performance costs (10x vs Ivy) become prohibitive at VRR's state-space complexity, given its largest case study ran 160 seconds?
- What is the right Lean-side encoding of past-time reconfiguration properties in the absence of past operators in LeanLTL — explicit history variables, or an event-ordering relation in the background theory?
- Can Veil's planned protocol-composition work (`veil-pirlea-et-al-cav2025.md`, §5) decompose VRR into normal-case, view-change, recovery, and reconfiguration modules with separately verified interfaces, rather than one monolithic invariant set?

## Appendix: User Notes

- Liveness proofs are not of interest here: vrr-core is a strong-consistency VRR implementation, so the focus is safety.
- The specific goal is to prove some additional capabilities around cluster reconfiguration.
- The aim is to find techniques directly applicable to VRR; in our case it may be provable with another method, so at this point this is deliberately a long-list approach to finding papers and tools that are applicable.
