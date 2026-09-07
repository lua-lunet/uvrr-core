# Rung 6: The leader's casting vote (Turner Section V)

*2026-09-05T23:31:05Z by Showboat 0.6.1*
<!-- showboat-id: 77b58ad8-4760-48b0-bd28-79bf289cebb6 -->

**Claim.** The casting vote is a way of running phase I for the new-era ballot b' without stalling the old-era pipeline. With q ∈ QII_{e(b)}, q' ∈ QI_{e(b)+1} and q ∩ q' = {ℓ}, the leader sends prepare(b') only to q' minus itself and keeps streaming era-e proposals under b to q. We prove: (1) noninterference — no node of q other than ℓ can have promised b', so (2) guard_preserved — the acceptance guard for b at every such node is unchanged: the pipeline is not stalled; (3) completes_phase1 — the instant ℓ promises to itself, all of q' has promised, meeting P5's quorum obligation for b'.

**Safety needs nothing new.** Theorem 10 (Rung 4) already covers every history, including this schedule; the casting vote only chooses who is sent prepare(b') and when ℓ promises. It never relaxes P1–P7. What is NOT proved: liveness (that a casting vote exists, that messages arrive).

```bash
cat UVRR/CastingVote.lean
```

```output
/-
  Rung 6 — The leader's casting vote (UPaxos paper, Section V).

  Setting: the cluster is in era e under leader ℓ with ballot b. The next
  configuration is chosen; ℓ must move to a ballot b' with e(b') = e(b)+1.
  ℓ has a CASTING VOTE if there are quorums q ∈ QII_{e(b)} and q' ∈ QI_{e(b)+1}
  with q ∩ q' = {ℓ}. ℓ then sends prepare(b') ONLY to q' \ {ℓ}, keeps streaming
  era-e proposals under b to q, and finally promises b' to itself, which
  completes phase I of b' at a moment of ℓ's own choosing (no network delay).

  What we prove, and what we do not:
  * `noninterference` — while ℓ withholds its own promise, no node of q other
    than ℓ has promised b', so the acceptance guard for b at every node of
    q \ {ℓ} is unchanged: the era-e pipeline is not stalled by phase I of b'.
    (This is the "non-stop" claim, stated as a safety property of the guard.)
  * `completes_phase1` — the moment ℓ adds its own promise, the promisers
    cover q' ∈ QI_{e(b)+1}, i.e. the quorum obligation of P5 for b' is met.
  * Agreement needs NO new assumption: Theorem 10 already covers every history
    including this schedule, because the casting-vote mode only chooses WHO is
    sent prepare(b') and WHEN ℓ promises; it never relaxes P1–P7.
  We do NOT prove liveness (that a casting vote always exists, or that
  messages arrive).
-/
import UVRR.Eras

universe u

namespace CastingVote
variable {A : Type u}

/-- ℓ is the unique common member of q and q'. -/
def HasCastingVote (q q' : NSet A) (ℓ : A) : Prop := ∀ a, (q a ∧ q' a) ↔ a = ℓ

/-- Acceptance guard at node a for ballot b: no promise above b. -/
def Acceptable {B : Type} (lt : B → B → Prop) (promised : A → B → Prop) (a : A) (b : B) : Prop :=
  ∀ b'', promised a b'' → ¬ lt b b''

/-- Non-interference: if prepare(b') was sent only to q' \ {ℓ} (so only those
nodes can have promised b'), then no node of q other than ℓ promised b'. -/
theorem noninterference {q q' : NSet A} {ℓ : A} (hcv : HasCastingVote q q' ℓ)
    (promisedNew : A → Prop) (hsent : ∀ a, promisedNew a → q' a ∧ a ≠ ℓ) :
    ∀ a, q a → a ≠ ℓ → ¬ promisedNew a := by
  intro a hq hne hp
  obtain ⟨hq', _⟩ := hsent a hp
  exact hne ((hcv a).1 ⟨hq, hq'⟩)

/-- Hence the acceptance guard for the old ballot b is preserved at every node
of q \ {ℓ}: the new promises (all for b', all outside q \ {ℓ}) add no constraint. -/
theorem guard_preserved {B : Type} (lt : B → B → Prop) {q q' : NSet A} {ℓ : A}
    (hcv : HasCastingVote q q' ℓ) (promisedOld : A → B → Prop) (b' : B)
    (promisedNew : A → Prop) (hsent : ∀ a, promisedNew a → q' a ∧ a ≠ ℓ)
    (b : B) (a : A) (hq : q a) (hne : a ≠ ℓ)
    (hold : Acceptable lt promisedOld a b) :
    Acceptable lt (fun x c => promisedOld x c ∨ (promisedNew x ∧ c = b')) a b := by
  intro b'' h
  rcases h with h | ⟨hn, _⟩
  · exact hold b'' h
  · exact absurd hn (noninterference hcv promisedNew hsent a hq hne)

/-- Completion: once every node of q' \ {ℓ} has promised and ℓ promises itself,
all of q' has promised — the phase-I quorum obligation for b' is met. -/
theorem completes_phase1 {q' : NSet A} {ℓ : A} (promised : A → Prop)
    (hothers : ∀ a, q' a → a ≠ ℓ → promised a) (hself : promised ℓ) :
    ∀ a, q' a → promised a := by
  intro a ha
  by_cases h : a = ℓ
  · subst h; exact hself
  · exact hothers a ha h

/-- Casting vote in the 3-node example (ℓ=0, a1=1, a2=2): q = {0,1}, q' = {0,2}. -/
theorem example3 : HasCastingVote (fun a : Nat => a = 0 ∨ a = 1) (fun a => a = 0 ∨ a = 2) 0 := by
  intro a; constructor
  · intro ⟨h1, h2⟩; omega
  · intro h; subst h; exact ⟨Or.inl rfl, Or.inl rfl⟩

end CastingVote
```

```bash
lake build UVRR.CastingVote 2>&1 | tail -1
```

```output
Build completed successfully (4 jobs).
```

```bash
printf 'import UVRR.CastingVote\n#print axioms CastingVote.noninterference\n#print axioms CastingVote.guard_preserved\n#print axioms CastingVote.completes_phase1\n#print axioms CastingVote.example3\n' | lake env lean --stdin
```

```output
'CastingVote.noninterference' does not depend on any axioms
'CastingVote.guard_preserved' does not depend on any axioms
'CastingVote.completes_phase1' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVote.example3' depends on axioms: [propext, Quot.sound]
```

**Reading the output.** These are small set-theoretic lemmas by design: the paper's argument for the casting vote is itself a few lines, and the substance is that agreement is inherited unchanged from Theorem 10. example3 is the blog's 3-node picture (leader 0, followers 1 and 2).
