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
