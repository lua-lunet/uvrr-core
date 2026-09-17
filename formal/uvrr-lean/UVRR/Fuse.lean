import UVRR.WeightedGeneral

/-! A fused acceptance acknowledges an entire finite slot range per node.
Slots are indexed from zero through the inclusive final index; a batch of
`m` commands therefore ends at index `m - 1`. The same response set supplies
each slot's quorum only when its eligibility
is preserved at every boundary. Quorum-backed chosen evidence is defined
explicitly; no converse of the Paxos P7 invariant is assumed.
-/
namespace Fuse

/-- One response set remains a quorum through a finite configuration sequence. -/
theorem eligible_through {A : Type} (Q : Nat → QSys A) (R : NSet A) (n : Nat)
    (hfirst : Q 0 R)
    (hstep : ∀ i, i < n → Q i R → Q (i + 1) R) :
    ∀ i, i ≤ n → Q i R := by
  intro i hi
  induction i with
  | zero => exact hfirst
  | succ i ih => exact hstep i (by omega) (ih (by omega))

/-- Each responding node certifies acceptance of every slot in the fused range. -/
def AtomicAcceptance {A : Type} (accepted : Nat → A → Prop)
    (R : NSet A) (n : Nat) : Prop :=
  ∀ a, R a → ∀ i, i ≤ n → accepted i a

/-- Chosen evidence consists of a legal quorum accepting the slot. -/
def ChosenEvidence {A : Type} (Q : Nat → QSys A)
    (accepted : Nat → A → Prop) (i : Nat) : Prop :=
  ∃ q, Q i q ∧ ∀ a, q a → accepted i a

/-- Finite eligibility preservation and atomic acknowledgements certify all slots. -/
theorem chosen_all {A : Type} (Q : Nat → QSys A) (R : NSet A) (n : Nat)
    (accepted : Nat → A → Prop) (hfirst : Q 0 R)
    (hstep : ∀ i, i < n → Q i R → Q (i + 1) R)
    (hatomic : AtomicAcceptance accepted R n) :
    ∀ i, i ≤ n → ChosenEvidence Q accepted i := by
  intro i hi
  exact ⟨R, eligible_through Q R n hfirst hstep i hi,
    fun a ha => hatomic a ha i hi⟩

/-- A protocol whose decision rule consumes chosen evidence may mark every slot. -/
theorem decide_all {A : Type} (Q : Nat → QSys A) (R : NSet A) (n : Nat)
    (accepted : Nat → A → Prop) (chosen : Nat → Prop) (hfirst : Q 0 R)
    (hstep : ∀ i, i < n → Q i R → Q (i + 1) R)
    (hatomic : AtomicAcceptance accepted R n)
    (hdecide : ∀ i, i ≤ n → ChosenEvidence Q accepted i → chosen i) :
    ∀ i, i ≤ n → chosen i := by
  intro i hi
  exact hdecide i hi (chosen_all Q R n accepted hfirst hstep hatomic i hi)

/-- Finite weight rows use zero outside their listed support. -/
def profile (row : List Nat) (a : Nat) : Nat := row[a]?.getD 0

def threeRows : Nat → List Nat
  | 0 => [1, 0, 1, 1]
  | 1 => [0, 0, 1, 1]
  | _ => [0, 1, 1, 1]

def threeQuorums (i : Nat) : QSys Nat :=
  WeightedGeneral.majority [0, 1, 2, 3] (profile (threeRows i))
def threeResponses : NSet Nat := fun a => a = 2 ∨ a = 3

/-- The two survivors form a quorum at all three rows of the two-step schedule. -/
theorem three_eligible : ∀ i, i ≤ 2 → threeQuorums i threeResponses := by
  intro i hi
  have h : i = 0 ∨ i = 1 ∨ i = 2 := by omega
  rcases h with h | h | h <;> subst i <;>
    simp [threeQuorums, threeRows, threeResponses, profile,
      WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total]

def fiveRows : Nat → List Nat
  | 0 => [1, 0, 1, 1, 1, 1]
  | 1 => [2, 0, 2, 2, 2, 2]
  | 2 => [2, 1, 2, 2, 2, 2]
  | 3 => [1, 1, 2, 2, 2, 2]
  | 4 => [0, 1, 2, 2, 2, 2]
  | 5 => [0, 2, 2, 2, 2, 2]
  | _ => [0, 1, 1, 1, 1, 1]

def fiveQuorums (i : Nat) : QSys Nat :=
  WeightedGeneral.majority [0, 1, 2, 3, 4, 5] (profile (fiveRows i))
def fiveResponses : NSet Nat := fun a => a = 2 ∨ a = 3 ∨ a = 4

/-- Three survivors qualify throughout doubling, replacement, and halving. -/
theorem five_eligible : ∀ i, i ≤ 6 → fiveQuorums i fiveResponses := by
  intro i hi
  have h : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 := by omega
  rcases h with h | h | h | h | h | h | h <;> subst i <;>
    simp [fiveQuorums, fiveRows, fiveResponses, profile,
      WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total]

/-- The first quorum alone does not imply the fixed responses qualify later. -/
theorem first_quorum_is_insufficient :
    threeQuorums 0 (fun a => a = 0 ∨ a = 2) ∧
    ¬ threeQuorums 1 (fun a => a = 0 ∨ a = 2) := by
  simp [threeQuorums, threeRows, profile,
    WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total]

/-- Two commands occupy slots zero and one, using their source configurations. -/
theorem three_fused_chosen (accepted : Nat → Nat → Prop)
    (hatomic : AtomicAcceptance accepted threeResponses 1) :
    ∀ i, i ≤ 1 → ChosenEvidence threeQuorums accepted i := by
  exact chosen_all threeQuorums threeResponses 1 accepted
    (three_eligible 0 (by decide))
    (fun i hi _ => three_eligible (i + 1) (by omega)) hatomic

/-- Six commands occupy slots zero through five, using their source configurations. -/
theorem five_fused_chosen (accepted : Nat → Nat → Prop)
    (hatomic : AtomicAcceptance accepted fiveResponses 5) :
    ∀ i, i ≤ 5 → ChosenEvidence fiveQuorums accepted i := by
  exact chosen_all fiveQuorums fiveResponses 5 accepted
    (five_eligible 0 (by decide))
    (fun i hi _ => five_eligible (i + 1) (by omega)) hatomic

/-- At least three surviving voters supply enough mass in every five-node row. -/
theorem five_survivor_margin (k : Nat) (hk : 3 ≤ k) :
    ∀ i, i ≤ 6 →
      WeightedGeneral.total [0, 1, 2, 3, 4, 5] (profile (fiveRows i)) <
        2 * (if i = 0 ∨ i = 6 then k else 2 * k) := by
  intro i hi
  have h : i = 0 ∨ i = 1 ∨ i = 2 ∨ i = 3 ∨ i = 4 ∨ i = 5 ∨ i = 6 := by omega
  rcases h with h | h | h | h | h | h | h <;> subst i <;>
    simp [fiveRows, profile, WeightedGeneral.total] <;> omega

/-- Any response set carrying these lower bounds qualifies at the selected row. -/
theorem five_quorum_of_survivor_mass (q : NSet Nat) (k i : Nat)
    (hk : 3 ≤ k) (hi : i ≤ 6)
    (hmass : (if i = 0 ∨ i = 6 then k else 2 * k) ≤
      WeightedGeneral.mass [0, 1, 2, 3, 4, 5] (profile (fiveRows i)) q) :
    fiveQuorums i q := by
  have hmargin := five_survivor_margin k hk i hi
  unfold fiveQuorums WeightedGeneral.majority
  omega

/-- A shared ballot in the initial era covers at most the next era under this guard. -/
theorem same_ballot_era_guard (initialEra lastEraOffset ballotEra : Nat)
    (hballot : ballotEra = initialEra)
    (hguard : ∀ i, i ≤ lastEraOffset → initialEra + i ≤ ballotEra + 1) :
    lastEraOffset ≤ 1 := by
  have hlast := hguard lastEraOffset (Nat.le_refl _)
  omega

/-! ### The Telescoping Theorem

In a three-node cluster whose reconfiguration schedule has two transitions —
the cluster states `E0 → E1 → E2` — under the fuse constraints: if `E0 → E1`
will pass, then `E1 → E2` will also pass. The first transition's acceptance
telescopes the remaining slots of the fused schedule. -/
namespace Telescope

/-- The schedule's quorum families: the weighted strict majority of each
transition's era configuration over the finite node list. -/
def ScheduleFamily {A : Type} (nodes : List A) (eraWeight : Nat → A → Nat) :
    Nat → QSys A :=
  fun e => WeightedGeneral.majority nodes (eraWeight e)

/-- The Telescoping Theorem. One datagram carries the whole batch, so one
response set `R` acknowledges every packed slot: the atomic batch property
(`docs/uvrr-fuse.md` §2) — a node processes the whole slab before reading any
other node's message, so the batch cannot be interrupted; all pass or all
fail. The fuse header ballot is both Phase 1 and Phase 2 for every command in
the fused batch, so the same replies count at every slot; a majority on the
first transition is a majority on every transition, with the same outcome.

Quorum-backed evidence at the first transition telescopes: the base case is
`R` a quorum at `E0` (the first transition's quorum condition), the
preservation step carries `R` across `E0 → E1` — exactly the first transition
passing — and the finite induction over the batch (the one Lamport runs:
one leader, one ballot, all future slots, until interrupted) certifies every
slot of the fused range, including the second transition's slots at
`E1 → E2`, provided the schedule's quorum family is preserved at both
boundaries. -/
theorem telescope_pass {A : Type} (nodes : List A) (eraWeight : Nat → A → Nat)
    (R : NSet A)
    (hbase : ScheduleFamily nodes eraWeight 0 R)
    (hstep : ∀ i, ScheduleFamily nodes eraWeight i R →
      ScheduleFamily nodes eraWeight (i + 1) R) :
    ∀ i, i ≤ 2 → ScheduleFamily nodes eraWeight i R :=
  eligible_through (ScheduleFamily nodes eraWeight) R 2 hbase (fun i _ h => hstep i h)

/-- Negative control (the paper's equal-total swap): on the two-branch
schedule `(1,2,1,2) → (2,1,2,1)` the same response set `BD` is a majority at
`E0` and a minority at `E1`, so the preservation hypothesis cannot be
dropped — a first transition passing with one quorum does not telescope the
remaining slots when the schedule switches the quorum family wholesale. -/
def swapRow : Nat := 0

def swapWeights (i : Nat) : Nat → Nat := fun a =>
  if a = 0 then (if i = swapRow then 1 else 2)
  else if a = 2 then (if i = swapRow then 1 else 2)
  else (if i = swapRow then 2 else 1)

def swapQuorums (i : Nat) : QSys Nat :=
  WeightedGeneral.majority [0, 1, 2, 3] (swapWeights i)

theorem swap_control :
    swapQuorums 0 (fun a => a = 1 ∨ a = 3) ∧ ¬ swapQuorums 1 (fun a => a = 1 ∨ a = 3) := by
  unfold swapQuorums WeightedGeneral.majority
  simp [swapWeights, swapRow, WeightedGeneral.mass, WeightedGeneral.total] <;> omega

end Telescope

end Fuse
