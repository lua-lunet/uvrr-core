# Rung 13: Whole-log selection by last-normal view

*2026-09-11T00:42:06Z by Showboat 0.6.1*
<!-- showboat-id: 82af4ebb-b165-4d5b-880e-613ba4e6d656 -->

Claim: the nonempty-report selector returns a member of maximal (last-normal view, log length) rank. Quorum intersection and explicit voter-history, same-view comparability and strictly later-view preservation premises yield committed-prefix preservation. These temporal premises still need an operational proof. The selector checks only rank. The bounded scalar rank in the TLA model is proved equivalent; controls show a length-only selector loses a newer committed prefix and omission of the scalar length bound misranks reports.

```bash
cat UVRR/ViewSelection.lean
```

```output
import UVRR.Synod

/-! VRR-2012 Section 4.2: executable selection by last-normal view, then
log length. The preservation theorem is the view-induction step. It explicitly
requires same-view log comparability, quorum coverage, surviving voter history,
and preservation by strictly later normal views. Deriving those premises from
the complete message/restart transition system remains a separate obligation.
No global prefix-agreement check is used in the selection algorithm.
-/
namespace ViewSelection

structure Report (A V : Type) where
  sender : A
  retained : Nat
  log : List V
  committed : Nat
  bounded : committed ≤ log.length

abbrev RankLE {A V : Type} (r s : Report A V) : Prop :=
  r.retained < s.retained ∨
    (r.retained = s.retained ∧ r.log.length ≤ s.log.length)

def choose {A V : Type} (r s : Report A V) : Report A V :=
  if RankLE r s then s else r

/-- A seed report makes the collection nonempty by construction. -/
def select {A V : Type} (seed : Report A V) : List (Report A V) → Report A V
  | [] => seed
  | r :: rest => choose r (select seed rest)

theorem rank_refl {A V : Type} (r : Report A V) : RankLE r r := by
  exact Or.inr ⟨rfl, Nat.le_refl _⟩

theorem rank_trans {A V : Type} {r s t : Report A V}
    (h : RankLE r s) (h' : RankLE s t) : RankLE r t := by
  unfold RankLE at *; omega

theorem rank_total {A V : Type} (r s : Report A V) : RankLE r s ∨ RankLE s r := by
  unfold RankLE; omega

/-- The TLA model's bounded scalar encoding of ReportRank agrees exactly
with lexicographic ranking when both log lengths respect MaxLogLength. -/
theorem scalar_rank_equivalent {A V : Type} (r s : Report A V) (limit : Nat)
    (hr : r.log.length ≤ limit) (hs : s.log.length ≤ limit) :
    (r.retained * (limit + 1) + r.log.length ≤
      s.retained * (limit + 1) + s.log.length) ↔ RankLE r s := by
  constructor
  · intro h
    by_cases he : r.retained = s.retained
    · right; refine ⟨he, ?_⟩; rw [he] at h; omega
    · left
      apply Nat.lt_of_not_ge
      intro hge
      have hsr : s.retained + 1 ≤ r.retained := by omega
      have hm := Nat.mul_le_mul_right (limit + 1) hsr
      simp only [Nat.add_mul, Nat.one_mul] at hm
      omega
  · intro h
    rcases h with hlt | ⟨he, hlen⟩
    · have hm := Nat.mul_le_mul_right (limit + 1) (Nat.succ_le_of_lt hlt)
      simp only [Nat.succ_eq_add_one, Nat.add_mul, Nat.one_mul] at hm
      omega
    · rw [he]; omega

theorem choose_left {A V : Type} (r s : Report A V) : RankLE r (choose r s) := by
  unfold choose; split
  · assumption
  · exact rank_refl r

theorem choose_right {A V : Type} (r s : Report A V) : RankLE s (choose r s) := by
  unfold choose; split
  · exact rank_refl s
  · exact (rank_total r s).resolve_left (by assumption)

theorem choose_member {A V : Type} (r s : Report A V) : choose r s = r ∨ choose r s = s := by
  unfold choose; split
  · exact Or.inr rfl
  · exact Or.inl rfl

theorem select_member {A V : Type} (seed : Report A V) (rest : List (Report A V)) :
    select seed rest ∈ seed :: rest := by
  induction rest with
  | nil => exact List.mem_cons_self
  | cons r rest ih =>
    rcases choose_member r (select seed rest) with h | h
    · change choose r (select seed rest) ∈ _
      rw [h]; simp
    · change choose r (select seed rest) ∈ _
      rw [h]
      rcases List.mem_cons.mp ih with he | hm
      · rw [he]; exact List.mem_cons_self
      · exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hm)

theorem select_max {A V : Type} (seed : Report A V) (rest : List (Report A V)) :
    ∀ r ∈ seed :: rest, RankLE r (select seed rest) := by
  induction rest with
  | nil => intro r hr; simp only [List.mem_singleton] at hr; subst r; exact rank_refl _
  | cons s rest ih =>
    intro r hr
    change RankLE r (choose s (select seed rest))
    rcases List.mem_cons.mp hr with he | hm
    · subst r
      exact rank_trans (ih seed (List.mem_cons_self)) (choose_right _ _)
    · rcases List.mem_cons.mp hm with he | hm
      · subst r; exact choose_left _ _
      · exact rank_trans (ih r (List.mem_cons_of_mem _ hm)) (choose_right _ _)

/-- The actual view-induction step: the selector preserves a prefix certified
in view v. Later-view preservation is required only for strictly later views;
the equal-view case follows from comparability and maximal length. -/
theorem preserves_witness {A V : Type} (seed : Report A V) (rest : List (Report A V))
    (committedPrefix : List V) (v : Nat)
    (sameView : ∀ r ∈ seed :: rest, ∀ s ∈ seed :: rest,
      r.retained = s.retained → r.log <+: s.log ∨ s.log <+: r.log)
    (later : ∀ r ∈ seed :: rest, v < r.retained → committedPrefix <+: r.log)
    (witness : Report A V) (hw : witness ∈ seed :: rest)
    (hv : v ≤ witness.retained) (hp : committedPrefix <+: witness.log) :
    committedPrefix <+: (select seed rest).log := by
  have hs := select_member seed rest
  have hm := select_max seed rest witness hw
  by_cases hnew : v < (select seed rest).retained
  · exact later _ hs hnew
  · have he : witness.retained = (select seed rest).retained := by
      unfold RankLE at hm; omega
    have hlen : witness.log.length ≤ (select seed rest).log.length := by
      unfold RankLE at hm; omega
    rcases sameView witness hw _ hs he with h | h
    · exact hp.trans h
    · have heq := h.eq_of_length_le hlen
      rw [heq]
      exact hp

/-- Quorum intersection supplies the witness required by preserves_witness.
Coverage is by sender identity, so duplicated reports cannot manufacture a
quorum member. Voter-history and later-view premises are temporal obligations,
not assumptions inserted into the executable selector. -/
theorem quorum_preserves {A V : Type} (seed : Report A V) (rest : List (Report A V))
    (committedPrefix : List V) (v : Nat) (commitFamily viewFamily : QSys A)
    (overlap : Frown commitFamily viewFamily)
    (commitQ viewQ : NSet A) (hc : commitFamily commitQ) (hq : viewFamily viewQ)
    (coverage : ∀ a, viewQ a → ∃ r ∈ seed :: rest, r.sender = a)
    (voterHistory : ∀ r ∈ seed :: rest, commitQ r.sender →
      v ≤ r.retained ∧ (r.retained = v → committedPrefix <+: r.log))
    (sameView : ∀ r ∈ seed :: rest, ∀ s ∈ seed :: rest,
      r.retained = s.retained → r.log <+: s.log ∨ s.log <+: r.log)
    (later : ∀ r ∈ seed :: rest, v < r.retained → committedPrefix <+: r.log) :
    committedPrefix <+: (select seed rest).log := by
  obtain ⟨a, ha, hqa⟩ := overlap commitQ viewQ hc hq
  obtain ⟨r, hr, he⟩ := coverage a hqa
  obtain ⟨hv, hp⟩ := voterHistory r hr (he ▸ ha)
  apply preserves_witness seed rest committedPrefix v sameView later r hr hv
  rcases Nat.eq_or_lt_of_le hv with heq | hlt
  · exact hp heq.symm
  · exact later r hr hlt

/-- An older log can be longer while its uncommitted suffix disagrees. -/
def oldLong : Report Bool Bool := ⟨false, 0, [false, false], 0, by decide⟩
def newShort : Report Bool Bool := ⟨true, 1, [true], 1, by decide⟩

theorem newer_view_wins : select oldLong [newShort] = newShort := by simp [select, choose, RankLE, oldLong, newShort]

/-- Omitting the scalar encoding's length bound can reverse its ordering. -/
theorem scalar_without_bound_misranks :
    newShort.retained * (0 + 1) + newShort.log.length ≤
      oldLong.retained * (0 + 1) + oldLong.log.length ∧
    ¬ RankLE newShort oldLong := by decide

/-- Mutation: selecting only by length loses a later-view committed prefix.
Both messages individually satisfy their committed-length perimeter. -/
def lengthOnly {A V : Type} (r s : Report A V) : Report A V :=
  if r.log.length ≤ s.log.length then s else r

theorem length_only_loses_committed :
    lengthOnly oldLong newShort = oldLong ∧
    ¬ ([true] <+: (lengthOnly oldLong newShort).log) := by
  constructor
  · rfl
  · intro ⟨suffix, he⟩
    change [true] ++ suffix = [false, false] at he
    cases he

end ViewSelection
```

```bash
set -euo pipefail; lake build UVRR.ViewSelection >/dev/null; echo "UVRR.ViewSelection built"
```

```output
UVRR.ViewSelection built
```

```bash
printf 'import UVRR.ViewSelection\n#print axioms ViewSelection.select_member\n#print axioms ViewSelection.select_max\n#print axioms ViewSelection.scalar_rank_equivalent\n#print axioms ViewSelection.preserves_witness\n#print axioms ViewSelection.quorum_preserves\n#print axioms ViewSelection.newer_view_wins\n#print axioms ViewSelection.length_only_loses_committed\n#print axioms ViewSelection.scalar_without_bound_misranks\n' | lake env lean --stdin
```

```output
'ViewSelection.select_member' depends on axioms: [propext]
'ViewSelection.select_max' depends on axioms: [propext, Classical.choice, Quot.sound]
'ViewSelection.scalar_rank_equivalent' depends on axioms: [propext, Quot.sound]
'ViewSelection.preserves_witness' depends on axioms: [propext, Classical.choice, Quot.sound]
'ViewSelection.quorum_preserves' depends on axioms: [propext, Classical.choice, Quot.sound]
'ViewSelection.newer_view_wins' depends on axioms: [propext]
'ViewSelection.length_only_loses_committed' does not depend on any axioms
'ViewSelection.scalar_without_bound_misranks' does not depend on any axioms
```
