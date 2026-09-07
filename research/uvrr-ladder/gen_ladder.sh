#!/usr/bin/env bash
set -u
cd /Users/Shared/lua-lunet/vrr-core/formal/uvrr-lean || exit 1
export PATH=$HOME/.elan/bin:$HOME/.local/bin:$PATH
mkdir -p ladder; rm -f ladder/0[1-7]-*.md
fail=0
rung() { # file title module note-before axioms note-after
  local doc="ladder/$1.md"
  showboat init "$doc" "$2" >/dev/null || fail=1
  showboat note "$doc" "$4" >/dev/null || fail=1
  showboat exec "$doc" bash "cat UVRR/$3.lean" >/dev/null || fail=1
  showboat exec "$doc" bash "lake build UVRR.$3 2>&1 | tail -1" >/dev/null || fail=1
  local pr="import UVRR.$3\n"; for t in $5; do pr="$pr#print axioms $t\n"; done
  showboat exec "$doc" bash "printf '$pr' | lake env lean --stdin" >/dev/null || fail=1
  showboat note "$doc" "$6" >/dev/null || fail=1
}
rung 01-structure "Rung 1: Frown checker and the 3-zone hot-swap" Structure \
"**Claim.** Turner's frown operator Q1 ⌢ Q2 (every quorum of Q1 meets every quorum of Q2) is decidable for explicitly listed quorums, and the paper's/blog's server-replacement schedule (zones A,B,C hold W,X,Y; add Z beside W; remove Y) satisfies the UPaxos safety equation P1: QII_e ⌢ QI_e ⌢ QII_{e+1} at every era boundary, using plain majorities.

**Source.** UPaxos paper §IV-A (invariant P1) and the blog post 'Paxos Voting Weights' (first table). This corrects the earlier scratch baby-step, which had encoded QI_{e+1} ⌢ QII_e as the third overlap; the paper's third frown is the intra-era QII_{e+1} ⌢ QI_{e+1}, and Turner confirms the prepare-to-prepare overlap is not required.

**Negative control.** Replacing the whole cluster {0,1,2} by {3,4,5} in one step keeps both intra-era frowns but fails the cross-era frown, so P1 rejects it." \
"frownB_iff hotswap_p1 wholesale_cross_fails" \
"**Reading the output.** frownB_iff is the soundness/completeness bridge from the Bool checker to the Prop-level Frown; hotswap_p1 is closed by decide (kernel evaluation, no native_decide) for eras 0 and 1 and by rewriting for all later eras; wholesale_cross_fails shows the checker also refutes. Axioms listed are Lean's standard propext/Quot.sound only."
rung 02-lex-ballots "Rung 2: Era-tagged ballots are a well-founded total order" LexBallot \
"**Claim.** UPaxos encodes the era in the most significant bits of the ballot number, so ballots are (era, round) pairs under lexicographic order. This order is well-founded, transitive, irreflexive and total (the side conditions needed for the agreement proofs), and it respects eras: b2 ≺ b1 ⇒ e(b2) ≤ e(b1), which the paper's Lemma 9 relies on.

**Also.** theorem10_lex specialises the era-indexed agreement theorem (Rung 4) to these concrete ballots, discharging all order-theoretic obligations so only the protocol invariants remain as hypotheses." \
"bLt_wf bLt_total bLt_era_mono Paxos.theorem10_lex" \
"**Reading the output.** Well-foundedness comes from Lean core's Prod.lex on Nat; the remaining facts are settled by omega after unfolding the lexicographic characterisation bLt_iff. Classical.choice appears only through the decidability instances used by omega."
rung 03-synod-agreement "Rung 3: Synod consistency (Turner Appendix B, Theorem 8)" Synod \
"**Claim.** Given the six Synod invariants S1–S6 of the paper's Fig. 1 as hypotheses over an arbitrary ballot type with a well-founded strict total order, we prove Lemma 6, Lemma 7 and Theorem 8: if chosen(b1) and chosen(b2) then v(b1) = v(b2).

**Why this is the heart of it.** S1 is the weakened quorum requirement: only QI(b1) ⌢ QII(b2) is required when proposed(b1), chosen(b2) and b2 ≺ b1 — not equality of quorum systems. That weakening is exactly what lets configurations change between ballots. Lemma 7 is proved by well-founded induction on b1 exactly as in the paper.

**Encoding notes.** S4's 'v(b) = v(max P)' is stated as the existence of a maximal reported last-vote bm with v(b) = v(bm); this is what a leader actually computes from a finite set of responses. Theorem 8 additionally needs phase-II quorums to be nonempty (the paper's 'nonempty quorum')." \
"Synod.lemma6 Synod.lemma7 Synod.theorem8" \
"**Reading the output.** Theorem 8 depends on no axioms at all: the proof is fully constructive relative to the hypotheses. Every hypothesis is an explicit field of Synod.Inv, so nothing is hidden in typeclass instances."
rung 04-eras-theorem10 "Rung 4: Era-indexed Paxos, Lemma 9 and Theorem 10" Eras \
"**Claim.** With configurations ⟨QI_e, QII_e⟩ indexed by era, phase I of ballot b using QI_{e(b)} and phase II of instance i using QII_{e(i)}, the single assumption on configurations is P1: QII_e ⌢ QI_e ⌢ QII_{e+1} for every e. Lemma 9 shows that whenever proposed_i(b1), chosen_i(b2) and b2 ≺ b1, e(i) ∈ {e(b1), e(b1)+1}, hence QI_{e(b1)} ⌢ QII_{e(i)}. Theorem 10 (per-instance agreement across reconfigurations) then follows by instantiating Rung 3 per instance.

**Honest scope.** The paper derives proposed_i(b) ⇒ e(b) ≤ e(i) from its multi-promise machinery (P2–P5, i_max, e nondecreasing). We take that consequence as the hypothesis EraLe and do not model multi-promises. Liveness is out of scope by design." \
"Paxos.lemma9 Paxos.toSynod_inv Paxos.theorem10" \
"**Reading the output.** toSynod_inv is the refinement: the Paxos invariants imply the Synod invariants for every instance, with S1 supplied by Lemma 9. Theorem 10 is therefore Theorem 8 applied to the per-instance view; no new proof effort and no new axioms."
rung 05-counterexample "Rung 5: The cross-era frown is necessary (counterexample)" Counterexample \
"**Claim.** Drop only the cross-era half of P1 (QI_e ⌢ QII_{e+1}) while keeping both intra-era frowns and every other invariant, and agreement fails: two different values are chosen for the same instance.

**The history.** Two nodes. Era-0 quorums are {node 1} for both phases; era-1 quorums are {node 2}. Instance 0 is in era 1; ballots (0,0) and (0,1) are era-0 ballots, legal because e(b) ≤ e(i) ≤ e(b)+1. Node 1 grants both phase-I quorums but never accepts, so it reports no last vote and constrains nothing; node 2 grants both phase-II quorums but never promised, so it is free to accept both. Ballot (0,0) chooses false, ballot (0,1) chooses true.

**What this shows.** This is exactly the 'reconfigure without overlapping quorums' failure: the theorem of Rung 4 cannot be weakened to intra-era overlaps, so any uVRR reconfiguration that does not preserve the cross-era frown is unsafe, not merely unproven." \
"Counterexample.not_p1 Counterexample.two_chosen Counterexample.cross_frown_necessary" \
"**Reading the output.** cross_frown_necessary bundles: both intra-era frowns, EraMono, EraLe, P2/P3, P4, P5, P6, P7 all hold; P1 fails; and instance 0 has two chosen ballots with different values. All proved by direct case analysis."
rung 06-casting-vote "Rung 6: The leader's casting vote (Turner Section V)" CastingVote \
"**Claim.** The casting vote is a way of running phase I for the new-era ballot b' without stalling the old-era pipeline. With q ∈ QII_{e(b)}, q' ∈ QI_{e(b)+1} and q ∩ q' = {ℓ}, the leader sends prepare(b') only to q' minus itself and keeps streaming era-e proposals under b to q. We prove: (1) noninterference — no node of q other than ℓ can have promised b', so (2) guard_preserved — the acceptance guard for b at every such node is unchanged: the pipeline is not stalled; (3) completes_phase1 — the instant ℓ promises to itself, all of q' has promised, meeting P5's quorum obligation for b'.

**Safety needs nothing new.** Theorem 10 (Rung 4) already covers every history, including this schedule; the casting vote only chooses who is sent prepare(b') and when ℓ promises. It never relaxes P1–P7. What is NOT proved: liveness (that a casting vote exists, that messages arrive)." \
"CastingVote.noninterference CastingVote.guard_preserved CastingVote.completes_phase1 CastingVote.example3" \
"**Reading the output.** These are small set-theoretic lemmas by design: the paper's argument for the casting vote is itself a few lines, and the substance is that agreement is inherited unchanged from Theorem 10. example3 is the blog's 3-node picture (leader 0, followers 1 and 2)."
rung 07-weights "Rung 7: Voting weights along the 3-zone hot-swap" Weights \
"**Claim.** The blog's weighted schedule (W,X,Y,Z) = (1,1,1,0) → (2,2,2,0) → (2,2,2,1) → (2,2,1,1) → (2,2,0,1) → (2,2,0,2) → (1,1,0,1) satisfies the UPaxos frowns at every step, with quorums defined as weighted majorities M(w) = { q | 2·Σ_{a∈q} w(a) > Σ_a w(a) } (paper Appendix A). Checked by enumerating all 16 subsets of the four servers.

**Negative control.** Changing one server's weight by 2 in a single step, (1,1,1,0) → (3,1,1,0), breaks the frown ({X,Y} vs {W}), showing the paper's Lemma 3 bound (total change ≤ 1) is tight.

**General lemma.** The paper's Lemma 2 (arbitrary node sets and scale factors) was handed to Leanstral as a drafting experiment; see the Leanstral rung for the outcome. The concrete result here does not depend on it." \
"rows_ok rows_p1 plus_two_breaks" \
"**Reading the output.** rows_ok is one decide over the whole schedule; rows_p1 unpacks it into six Prop-level frowns via frownB_iff. Kernel-checked, no native_decide."
echo "fail=$fail"; ls ladder; wc -l ladder/*.md | tail -1
