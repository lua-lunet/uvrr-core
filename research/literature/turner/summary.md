# Unbounded Pipelining in Dynamically Reconfigurable Paxos Clusters

David C. Turner, revision 1A9DBA37, 14 August 2017.
Operations and Planning Systems division, Tracsis plc, Leeds, United Kingdom.
Source PDF: `paxos-reconf-latest.pdf` (pinned by the revision marker and the sha256 in
`MANIFEST.md`). Extracted verbatim text: `text.txt`. This summary is a reference
account of the paper's content; per-section prose below follows the paper's own
notation: ≺ and ≻ order ballots, ⌢ relates quorum sets (Q₁ ⌢ Q₂ iff every
q₁ ∈ Q₁ and q₂ ∈ Q₂ intersect), ℓ is the distinguished leader, and e(·) is the era.

## Overview

The paper extends Paxos so that the pipeline of concurrently-running consensus
instances need never be bounded, not even while a cluster reconfiguration is in
progress. Classical Paxos bounds the pipeline by an α > 0 so that a configuration
change chosen at instance i takes effect only from instance i + α, guaranteeing that
no undecided configuration change can invalidate an in-flight proposal. Raft instead
leaves the pipeline unbounded and restricts which reconfigurations may be performed.
Turner's algorithm subsumes both: reconfigurations satisfying a per-era quorum-intersection
condition (P1: QIIe ⌢ QIe ⌢ QIIe+1) proceed with a fully concurrent pipeline, the
configuration and its eras being held inside the replicated state machine (RSM)
itself, so no external oracle is required. If the conditions for delay-free progress
are not met, a temporary pipeline limit restores liveness, recovering the bounded
regimes of Dynamic and Stoppable Paxos as special cases. Reworked liveness and
consistency proofs are given; the appendices are informal versions of formal Isabelle/HOL
proof efforts.

## Section-by-section claims

### I. Introduction

- An RSM replicates a deterministic state machine across 2f + 1 nodes; consensus on a
  sequence of transition-describing values keeps the replicas in lockstep, tolerating f
  faults with quorums of at least f + 1 nodes. The collection of quorums in use is the
  configuration.
- The configuration must itself be reconfigurable at runtime, which is achieved by
  holding the configuration in the RSM and choosing special reconfiguration commands by
  consensus.
- Paxos runs conceptually one two-phase Synod instance per chosen value; pipelining
  lets new instances start before older ones complete, but a configuration change in
  the pipeline can invalidate quorums. Paxos implementations bound the pipeline by
  α so a change chosen at i takes effect at i + α at the earliest; Raft keeps the
  pipeline unbounded and restricts admissible reconfigurations instead.
- Contribution: the pipeline may remain unbounded even during reconfiguration,
  provided the reconfiguration satisfies the conditions of section IV-A; otherwise a
  temporary limit suffices. The algorithm is restated in full (its modifications
  invalidate the original proofs), with reworked proofs in section IV-C and
  appendices B and C.

### II. Related work

- Lamport's bounded-pipeline reconfiguration (α = 3, later clarified to arbitrary α);
  the Static/Dynamic Paxos distinction; Cheap Paxos (self-reconfiguring, heterogeneous
  nodes); Stoppable Paxos (stop, reconfigure, restart; per-reconfiguration limits);
  Vertical Paxos and Egalitarian Paxos (external oracle for configuration — which
  merely moves the reconfiguration problem to the oracle, ad infinitum).
- Chubby's reconfiguration is unstated but probably bounded-pipeline; Birman, Malkhi
  and van Renesse note α > 1 can be unacceptably complex, so real systems disable
  pipelining (α = 1) or reconfiguration. Bortnikov et al. pass RSM responsibility
  along Stoppable-Paxos lines but start execution speculatively.
- Raft: unbounded pipeline, restricted reconfigurations, formally proved only without
  reconfiguration; simple majorities as quorums. Viewstamped Replication (Liskov and
  Cowling) reconfigures in Vertical-Paxos fashion; Zab gained a limited-pipeline
  reconfiguration (Shraer, Reed, Malkhi and Junqueira). Jehl and Meling reconfigure
  using eventual consistency rather than consensus, remaining available when no leader
  can be elected.

### III. The Synod algorithm

- Setting: asynchronous, non-Byzantine; messages may be delayed, reordered, duplicated
  or dropped, nodes may run slowly or stop. B is a set of ballot identifiers under a
  wellfounded total order ≺, A a set of node identifiers, V the set of choosable values.
- Five message kinds in two phases: prepare(b); free promise promised(a, b); forced
  promise promised(a, b; b′); proposal proposed(b); acceptance accepted(a, b); success
  chosen(b). The same symbols double as predicates. A value function v : B → V assigns
  each ballot its value.
- Invariants S1–S6 (fig. 1) yield consistency: if chosen(b) and chosen(b′) then
  v(b) = v(b′) (theorem 8). Phase completion is judged per ballot by quorum receipt,
  where the phase-I and phase-II quorum sets QI(b), QII(b) need only satisfy
  QI(b₁) ⌢ QII(b₂) for the relevant ballot pairs (per-phase quorums, III-B), not the
  classical all-majorities-intersect property; Howard, Malkhi and Spiegelman
  discovered this weakening independently.
- III-A (implementing the value function): carrying values inside every message
  duplicates them (2f messages per value in a unicast 2f + 1 network — twice what is
  necessary); Observation O4 of Cheap Paxos already allows hashes, and Turner elides
  values from the consensus messages altogether. The consensus messages stay small and
  fast while large values move by a separate, slower mechanism; any implementation of
  v is acceptable that agrees with v on proposed ballots, so the owner of an
  unproposed ballot may change its value freely. The insert-only set
  {⟨b, v(b)⟩ | proposed(b)} is a convergent replicated data type, implementable without
  consensus and resilient independently of it (the original presentation replicates it
  across all 2f + 1 nodes; Cheap Paxos across the f + 1 primaries with hashes on the
  f auxiliaries; Vertical Paxos's careful state transfer becomes unnecessary once v is
  separated out).
- III-B (per-phase quorums): QI(b₁) ⌢ QII(b₂) is only required when proposed(b₁),
  chosen(b₂) and b₁ ≻ b₂ (invariant S1); this weaker invariant is the key enabling
  general reconfiguration (section IV-A).

### IV. The Paxos algorithm

- Paxos is a sequence of Synod instances run in parallel; messages gain an instance
  index i: promisedᵢ(a, b), promisedᵢ(a, b; b′), proposedᵢ(b), acceptedᵢ(a, b),
  chosenᵢ(b), plus the multi-promise promised≥i(a, b) standing for the infinite set
  {promisedⱼ(a, b) | j ≥ i}; prepare(b) applies to all instances and is unindexed.
  Each instance i has its own value function vᵢ : B → V; theorem 10 gives
  consistency per instance.
- Invariants P1–P7 (fig. 2) add to the Synod-shaped invariants the configuration and
  era discipline below. P5's promise-set and P7's choice conditions mirror S4 and S6;
  the era constraints in P2–P4 and P7 replace separate acceptance invariants.
- IV-A (configuration changes): a fixed configuration is a pair ⟨QI, QII⟩ with
  QI ⌢ QII. Changes are a sequence ⟨QI₀, QII₀⟩, ⟨QI₁, QII₁⟩, … with
  QIIe ⌢ QIe ⌢ QIIe+1 for all e; e is the era. During a change from era e to e + 1,
  instances may use the interim configuration ⟨QIe, QIIe+1⟩. Two nondecreasing era
  functions e(i) (instances) and e(b) (ballots) police the use: proposedᵢ(b) may be
  emitted only on promises from a quorum in QIe(b) (implying e(b) ≤ e(i)), and
  chosenᵢ(b) only if e(i) ≤ e(b) + 1 on acceptances from a quorum in QIIe(i); hence
  chosenᵢ(b) forces e(b) ≤ e(i) ≤ e(b) + 1 and therefore QIe(b) ⌢ QIIe(i) (lemma 9).
- IV-B (dynamic configuration changes): the configuration sequence and the era
  sequence e(0), e(1), …, e(imax) are themselves chosen by consensus and held in the
  RSM as finite sequences that may only be appended to; ballot eras e(b) are fixed in
  advance instead. The sequences are long enough for progress: emax ≥ e(imax) and if
  every j < i is chosen then imax ≥ i. A node may promise at instance i only if
  e(b) ≤ e(min(i, imax)), so promises can be made even when e(i) is not yet known
  (i > imax); P7 requires i ≤ imax (hence e(i) known) before chosenᵢ(b).
- IV-C (liveness): liveness is provable only relative to a distinguished nonfaulty
  node ℓ that is eventually the only emitter of prepares, with sufficiently many
  nonfaulty nodes and eventually at least one value per instance (FLP caveat [21]).
  Unlike the original proof, suitable ballots have an upper bound in era (a proposal
  needs e(b) ≤ e(i)), so ℓ must be able to pick a ballot that is large enough yet in
  the right era for every node: for each b, e ≥ e(b) and a there must exist
  b′ ≻ b with e(b′) = e and owner(b′) = a. Simple integers cannot do this; an
  implementation takes B = N × N × A lexicographic, with e(⟨e, n, a⟩) = e and
  owner(⟨e, n, a⟩) = a, as in Egalitarian Paxos (whose "epoch" terminology is
  deliberately avoided, being leader-election vocabulary). With that, the induction
  of theorem 1 goes through: ℓ picks bi with e(bi) ∈ {e(i) − 1, e(i)} and
  owner(bi) = ℓ, completes phase I on promises from a quorum in QIe(bi), sets
  vᵢ(bi), broadcasts proposedᵢ(bi) and finishes on acceptances from a quorum in
  QIIe(i).
- IV-D (fully concurrent configuration changes): in normal running there is an
  instance i₀ and ballot b with e(b) = e(i₀) = e(imax) = emax, ℓ = owner(b) holds
  promises from a quorum qI ∈ QIe(b) for b at every instance i ≥ i₀, and ℓ (the
  leader) may then emit proposedᵢ(b) for any i ≥ i₀, normally accepted without
  delay. To reconfigure to ⟨QInew, QIInew⟩ with QIe(b) ⌢ QIInew ⌢ QInew, the operator
  appends ⟨QInew, QIInew⟩ as era e(b) + 1 (emax = e(b) + 1), picks the change instance
  ic = imax + 1, and sets e(i) = e(i₀) for i₀ ≤ i < ic and e(ic) = e(i₀) + 1: since
  e(ic) ≤ e(b) + 1, instances at and beyond ic can still be proposed and chosen on
  ballot b even though phase I has not run in the new era, so no concurrency limit is
  needed. The system is then out of normal running (e(b) = e(imax) − 1): raising
  e(imax) further would create instances with e(i) > e(b) + 1, unchoosable on b. The
  leader restores normal running by choosing b′ with e(b′) = e(b) + 1 and
  owner(b′) = ℓ and running phase I for b′ without blocking era e(b): possible when
  ℓ has a casting vote, i.e. there are quorums q ∈ QIIe(b) and q′ ∈ QIe(b)+1 with
  q ∩ q′ = {ℓ}; ℓ sends prepare(b′) only to q′ \ {ℓ}, the nodes of q continue
  accepting in era e(b), and on the last promise ℓ sends itself promised≥i′(ℓ, b′)
  for suitable i′, completing phase I instantly (self-messages have no network delay).
  If another node ℓ′ has the casting vote, ℓ abdicates to it first; if no node does,
  prepare(b′) must be broadcast and progress may pause until phase I completes. Node
  failures may force retries or a new leader; liveness and consistency persist, only
  performance suffers. Fig. 3 (see below) walks through the whole flow concretely.

### V. Examples

- With QIe = QIIe =: Qe for each e, weighted majority quorums work: a weight function
  w : A → N with finitely many nonzero values defines
  M(w) = {q | 2 Σ_{a∈q} w(a) > Σ_{a∈A} w(a)}; M(w) ⌢ M(w) (corollary 5), and
  M(w) = M(w′) when w′ is a constant multiple of w.
- Raft's quorums are simple majorities, i.e. weights in {0, 1}; its reconfigurations
  amount to changing one node's weight by ±1. The weight-1...n family shows why:
  M(w1...3) ⌢ M(w1...4) and M(w1...4) ⌢ M(w1...5) but M(w1...3) ⌢̸ M(w1...5), since
  {a1, a2} and {a3, a4, a5} are disjoint majorities — hence no adding/removing more
  than one node at a time under {0,1} weights.
- Lemma 3 generalizes to any integer weight functions with Σ|w′(a) − w(a)| ≤ 1
  (the "amoeba" analogy of [8]). This matters for nodes sharing infrastructure
  (power, network), where correlated failures make extra independence expensive: a
  naive aold → anew swap in a three-node cluster (weights (aold, anew, a1, a2) of
  (1,0,1,1), (1,1,1,1), (0,1,1,1) across eras e, e+1, e+2) needs all four
  infrastructures independent — a correlated failure of any two nodes stops progress —
  and leaves era e + 1 without any casting vote, whereas the seven-era schedule
  (1,0,1,1), (2,0,2,2), (2,1,2,2), (1,1,2,2), (0,1,2,2), (0,2,2,2), (0,1,1,1)
  achieves the same swap letting aold and anew share infrastructure, with both a1 and
  a2 holding casting votes throughout. At publication time only one AWS region (us-east-1) and one GCP region
  (us-central1) offered four independent zones.
- Early Raft-style joint configurations handle arbitrary changes from Qe to an
  unrelated Q′: set Qe+2 = Q′ and Qe+1 = {q ∪ q′ | q ∈ Qe, q′ ∈ Q′}, satisfying
  Qe ⌢ Qe+1 ⌢ Qe+1 ⌢ Q′.
- Weightless w (all weights zero) gives M(w) = ∅; Q ⌢ ∅ always holds, so changing to
  or from a weightless configuration is always permitted, though an instance with
  Qe(i) = ∅ can never be chosen.
- Consecutive instance eras need not differ by at most one: an era may be skipped
  (e(i+1) = e(i) + 2), recovering Stoppable-Paxos-style single-step arbitrary
  reconfiguration (pick Qe+1 = ∅ or another bridging configuration). Because
  delayed phase I only prevents stalling when the era increases by 1, an increase by 2
  forces a new phase I before any new phase II; to keep the pipeline alive the
  operator chooses α > 0 and sets e(i + α) = e(i) + 2 with e(j) = e(i) for
  i < j ≤ i + α, hoping phase I completes within α instances — the communication
  pattern of fig. 5 with α = 3.

### VI. Conclusion

- The algorithm runs an unlimited pipeline while no change is in progress (a variable
  improvement over Dynamic Paxos's fixed α) and, unlike Stoppable Paxos, needs no
  concurrency limit at all when the reconfiguration satisfies section IV-A's
  conditions; both bounded variants suffer when their limit is mispredicted
  (too small: lost parallelism; too large: expensive changes; either way a pause is
  still possible on a delayed phase-I message or a request burst).
- A chosen reconfiguration completes after a single round-trip to a quorum, with all
  clients served throughout, however long the round-trip takes. Raft-style
  reconfigurations are used, but a change takes effect only once chosen, avoiding
  leader-failure back-tracking; simple majorities are generalized to
  integer-weighted majorities that cope better with correlated failures during
  maintenance. Vertical Paxos's goals are met without a separate configuration
  oracle. Eliding values (not even hashes) from messages may make Cheap Paxos cheaper
  still and simplify state transfer to newly commissioned nodes.

## Lemma and theorem list

| Number | Statement |
|--------|-----------|
| Theorem 1 | Liveness: given an eventually unique nonfaulty prepare-emitter ℓ, sufficiently many nonfaulty nodes, and eventually at least one value per instance, a value is eventually chosen for every instance. |
| Lemma 2 | If w, w′ : A → N are weight functions and k, k′ positive integers with Σ_{a∈A} \|k′w′(a) − kw(a)\| ≤ 1, then M(w) ⌢ M(w′). |
| Lemma 3 | If w, w′ : A → N are weight functions with Σ_{a∈A} \|w′(a) − w(a)\| ≤ 1, then M(w) ⌢ M(w′). (Lemma 2 with k = k′ = 1.) |
| Lemma 4 | If k′w′(a) = kw(a) for all a (weight functions differing by a constant factor), then M(w) ⌢ M(w′). |
| Corollary 5 | M(w) ⌢ M(w) for any weight function w. (Lemma 4 with w′ = w.) |
| Lemma 6 | If accepted(a, b₂), promised(a, b₁; b₃) and b₂ ≺ b₁ then b₂ ⪯ b₃: b₃ is the greatest accepted ballot below b₁, and b₂ is another such, so b₂ cannot exceed it. |
| Lemma 7 | If chosen(b₂), proposed(b₁) and b₂ ≺ b₁ then v(b₁) = v(b₂), by minimal-counterexample induction on b₁ using S1–S6. |
| Theorem 8 | Synod consistency: if chosen(b₁) and chosen(b₂) then v(b₁) = v(b₂). |
| Lemma 9 | For b₁ ≻ b₂ ∈ B, if proposedᵢ(b₁) and chosenᵢ(b₂) then QIe(b₁) ⌢ QIIe(i): the promise/era discipline pins e(i) ∈ {e(b₁), e(b₁) + 1}, so P1 applies. |
| Theorem 10 | Paxos consistency: if chosenᵢ(b₁) and chosenᵢ(b₂) then vᵢ(b₁) = vᵢ(b₂), by deriving the Synod invariants per instance (QI(b) := QIe(b), QII(b) := QIIe(i), promised(a, b) := promisedᵢ(a, b) ∨ ∃i′ ≤ i. promised≥i′(a, b), etc.) and applying theorem 8. |

Invariant sets: S1–S6 (fig. 1) for Synod; P1–P7 (fig. 2) for Paxos, where P2–P4
constrain promises (free, forced and multi-) by era and by previously accepted ballots,
P5 requires a phase-I quorum in QIe(b) backing any proposal (with the forced-promise
ballots P determining vᵢ(b) = vᵢ(max(P)) when nonempty), P6 ties acceptances to
proposals, and P7 ties choices to a phase-II quorum in QIIe(i) with i ≤ imax and
e(i) ≤ e(b) + 1.

## What each figure depicts (as stated or implied by the text)

- Fig. 1 (page 3): the boxed invariant list S1–S6 preserved by the Synod algorithm;
  the quorum-intersection requirement of S1, promise discipline S2–S4, and the
  acceptance/choice conditions S5–S6.
- Fig. 2 (page 5): the boxed invariant list P1–P7 preserved by the Paxos algorithm —
  the same shape as fig. 1 plus the era and configuration constraints P1–P4, P7.
- Fig. 3 (page 6): message flow during a fully concurrent reconfiguration with nodes
  a1, a2 and leader ℓ. Era e has {ℓ, a1} ∈ QIe and {ℓ, a2} ∈ QIIe; era e + 1 the same
  with e + 1 subscripts, so the leader needs only a1's response for phase I and only
  a2's for phase II (a casting vote). Initially imax = i + 3 and
  e(i) = e(i+1) = e(i+2) = e(i+3) = e = emax, all instances before i already chosen.
  The leader completes phase I at a large ballot b (e(b) = e) and proposes for i,
  i+1, i+2 on client requests. A configuration change proposed at i+3, when chosen,
  appends ⟨QIe+1, QIIe+1⟩, sets e(i+4) = e(i+5) = … = e + 1 and raises emax and imax,
  taking the system out of normal running (e(imax) = e + 1 ≠ e(b)); to return, the
  leader must complete phase I at a ballot b′ with e(b′) = emax, sending prepare(b′)
  to a1. Meanwhile it keeps servicing requests with ballot b — proposedi+4(b) through
  proposedi+11(b) — deducing choseni+4(b) etc. safely because e(i+4) = e + 1 ≤ e(b) + 1
  and {ℓ, a2} ∈ QIIe+1 (note: the leader now uses QIIe+1, not QIIe); the dashed
  horizontal line marks the leader's move from era e to era e + 1, and there is no
  limit on how many requests can be handled this way while the prepare(b′) response
  is arbitrarily delayed. The response from a1 arrives just after proposedi+11(b) and
  before any proposal for i+12; the leader sends itself promised≥i+12(ℓ, b′),
  completing phase I at b′ (since {ℓ, a1} ∈ QIe+1) and restoring normal running, with
  proposals proposedj(b′) for all j ≥ i+12 — without ever having had to predict the
  phase-I completion time or the request count. Messages from ℓ are labelled; the
  successful responses (promises from a1, acceptances from a2) are unlabelled for
  clarity.
- Fig. 4 (page 7): the equivalent reconfiguration in Dynamic Paxos with pipeline
  length α = 2 — at most two proposals run concurrently, so the flow (prepare(b),
  proposals on b, prepare(b′), proposals on b′) shows the pause between instances
  i+5 and i+6 caused by the delay completing phase I at b′; a larger α would have
  hidden it but made changes expensive, and no fixed α avoids pauses under bursts or
  delayed phase-I messages.
- Fig. 5 (page 8): the equivalent reconfiguration in Stoppable Paxos, which allows
  unlimited parallel proposals within each configuration and out-of-order execution —
  the stopping command is proposed at instance i+3 before the two preceding instances
  i+1 and i+2 are proposed. By choosing i+3 the operator limits the system to at most
  two more client requests before the reconfiguration completes; the remaining free
  instances are used up before phase I completes at b′, temporarily suspending client
  request processing (the α = 3 pattern referenced in section V).

## License and attribution

© 2016-7 Tracsis plc; Creative Commons Attribution-ShareAlike 4.0 International
(CC BY-SA 4.0). The material in this directory is extracted and summarised for
reference and comparison with attribution; the paper's text is Turner's.
