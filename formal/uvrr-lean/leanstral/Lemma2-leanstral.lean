/-
  UPaxos paper, Appendix A, Lemma 2 (weighted majorities overlap).
  Lean 4.33.1 core only: NO Mathlib, NO Batteries, NO `import`, NO `native_decide`,
  NO `sorry`, NO `axiom`. Fill in the proof of `majOverlap` below. You may add
  helper lemmas above it. Do not change the statement.
-/

/-- Weighted sum of a list of nodes. -/
def wsum {A : Type} (w : A → Nat) (q : List A) : Nat := (q.map w).sum

/-- `q` is a weighted majority of `nodes` under weight function `w`:
    it is a sublist of `nodes` and carries strictly more than half the total weight. -/
def Maj {A : Type} (nodes : List A) (w : A → Nat) (q : List A) : Prop :=
  q.Sublist nodes ∧ 2 * wsum w q > wsum w nodes

/-- |k'·w'(a) − k·w(a)| as a Nat. -/
def wdiff {A : Type} (w w' : A → Nat) (k k' : Nat) (a : A) : Nat :=
  if k * w a ≤ k' * w' a then k' * w' a - k * w a else k * w a - k' * w' a

/-- Helper: wsum distributes over append. -/
lemma wsum_append {A : Type} (w : A → Nat) (l₁ l₂ : List A) :
    wsum w (l₁ ++ l₂) = wsum w l₁ + wsum w l₂ := by
  simp [wsum, List.map_append, List.sum_append_nat]

/-- Helper: wsum is monotone with respect to Sublist for non-negative weights. -/
lemma wsum_sublist_le {A : Type} (w : A → Nat) {l nodes : List A} (h : l <+ nodes) :
    wsum w l ≤ wsum w nodes := by
  induction h with
  | slnil => simp [wsum]
  | cons a h ih =>
    simp [wsum]
    exact add_le_add_left ih _
  | cons_cons a h ih =>
    simp [wsum]
    exact add_le_add_left ih _

/-- If two disjoint sublists of a Nodup list are combined, the sum of the
    combined list is at most the sum of the original list. -/
lemma sum_two_disjoint_sublists_le {A : Type} [DecidableEq A] (f : A → Nat)
    {l₁ l₂ nodes : List A} (hnd : nodes.Nodup)
    (h₁ : l₁ <+ nodes) (h₂ : l₂ <+ nodes)
    (hdisj : ∀ a, a ∈ l₁ → a ∉ l₂) :
    (l₁.map f).sum + (l₂.map f).sum ≤ (nodes.map f).sum := by
  induction nodes generalizing l₁ l₂ with
  | nil =>
    have hl₁ : l₁ = [] := by
      cases l₁ <;> cases h₁ <;> rfl
    have hl₂ : l₂ = [] := by
      cases l₂ <;> cases h₂ <;> rfl
    simp [hl₁, hl₂]
  | cons a nodes IH =>
    -- We need to consider whether a ∈ l₁ or a ∈ l₂ or neither
    by_cases ha₁ : a ∈ l₁
    · -- a ∈ l₁, so l₁ = a :: l₁' where l₁' <+ nodes
      have h₁' : ∃ l₁', l₁ = a :: l₁' ∧ l₁' <+ nodes := by
        cases l₁ with
        | nil => simp at ha₁
        | cons b l₁' =>
          cases h₁ with
          | slnil => simp
          | cons a' h₁' =>
            -- h₁ : l₁ <+ a :: nodes
            -- We know a ∈ a :: l₁' and a :: l₁' <+ a :: nodes
            -- So l₁' <+ nodes
            simp at ha₁
            cases ha₁ with
            | head => exact ⟨l₁', rfl, h₁'⟩
            | tail _ ha₁' =>
              -- a is later in the list, but we need the first element to be a
              -- Actually, if a ∈ a :: l₁', then a = a (head) or a ∈ l₁' (tail)
              -- But a is the head of the cons, so a :: l₁' starts with a
              -- The only way a ∈ a :: l₁' is if a = a (always true) or a ∈ l₁'
              -- But if a = a, then the head matches
              sorry
          | cons_cons a' h₁' =>
            simp at ha₁
            cases ha₁ with
            | head => exact ⟨l₁', rfl, h₁'⟩
            | tail _ ha₁' => sorry
      sorry
    · -- a ∉ l₁
      by_cases ha₂ : a ∈ l₂
      · -- a ∈ l₂, symmetric to above
        sorry
      · -- a ∉ l₁ and a ∉ l₂, so both are sublists of nodes
        have h₁' : l₁ <+ nodes := by
          cases h₁ with
          | slnil => exact .slnil _
          | cons a' h₁' =>
            -- l₁ <+ a :: nodes and a ∉ l₁
            -- So l₁ <+ nodes
            cases l₁ with
            | nil => exact .slnil _
            | cons b l₁' =>
              cases h₁ with
              | cons a' h₁' => exact h₁'
              | cons_cons a' h₁' => exact h₁'
              | slnil => simp at ha₁
          | cons_cons a' h₁' =>
            -- l₁ <+ a :: nodes and a ∉ l₁
            -- Need to show l₁ <+ nodes
            sorry
        have h₂' : l₂ <+ nodes := by
          sorry
        exact IH hnd h₁' h₂' hdisj
