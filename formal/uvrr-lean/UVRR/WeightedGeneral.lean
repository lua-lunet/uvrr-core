import UVRR.Synod

/-! Finite-support weighted-majority intersection, UPaxos Appendix A.
The node list enumerates the finite support. No universe-cardinality bound is
imposed. Duplicate entries are allowed mathematically (their weights add);
use a duplicate-free support list for the paper's ordinary set interpretation.
-/
namespace WeightedGeneral

def distance (x y : Nat) : Nat := (x - y) + (y - x)

def total {A : Type} (nodes : List A) (f : A → Nat) : Nat :=
  match nodes with
  | [] => 0
  | a :: rest => f a + total rest f

noncomputable def mass {A : Type} (nodes : List A) (w : A → Nat) (q : NSet A) : Nat := by
  classical
  exact total nodes (fun a => if q a then w a else 0)

def majority {A : Type} (nodes : List A) (w : A → Nat) : QSys A :=
  fun q => total nodes w < 2 * mass nodes w q

/-- A disjoint pair cannot collect more than the two total weights plus
the L1 discrepancy. The integral majority margins give the contradiction. -/
theorem disjoint_bound {A : Type} (nodes : List A) (w v : A → Nat)
    (q r : NSet A) (h : ∀ a ∈ nodes, ¬ (q a ∧ r a)) :
    2 * mass nodes w q + 2 * mass nodes v r ≤
    total nodes w + total nodes v + total nodes (fun a => distance (w a) (v a)) := by
  classical
  induction nodes with
  | nil => simp [mass, total]
  | cons a rest ih =>
    have ht : ∀ b ∈ rest, ¬ (q b ∧ r b) := fun b hb => h b (List.mem_cons_of_mem a hb)
    have hi := ih ht
    have ha := h a (List.mem_cons_self)
    have hp : 2 * (if q a then w a else 0) + 2 * (if r a then v a else 0) ≤
        w a + v a + distance (w a) (v a) := by
      by_cases hq : q a
      · by_cases hr : r a
        · exact False.elim (ha ⟨hq, hr⟩)
        · simp only [hq, hr, if_true, if_false, distance]; omega
      · by_cases hr : r a <;>
          simp only [hq, hr, if_true, if_false, distance] <;> omega
    simp only [mass, total] at hi ⊢
    omega

theorem total_scale {A : Type} (nodes : List A) (w : A → Nat) (k : Nat) :
    total nodes (fun a => k * w a) = k * total nodes w := by
  induction nodes with
  | nil => simp [total]
  | cons a rest ih => simp [total, ih, Nat.mul_add]

theorem mass_scale {A : Type} (nodes : List A) (w : A → Nat) (q : NSet A) (k : Nat) :
    mass nodes (fun a => k * w a) q = k * mass nodes w q := by
  classical
  unfold mass
  have he : (fun a => if q a then k * w a else 0) =
      (fun a => k * (if q a then w a else 0)) := by
    funext a; split <;> simp_all
  rw [he, total_scale]

theorem majority_scale {A : Type} (nodes : List A) (w : A → Nat)
    (q : NSet A) {k : Nat} (hk : 0 < k) (hq : majority nodes w q) :
    majority nodes (fun a => k * w a) q := by
  unfold majority at *
  rw [total_scale, mass_scale]
  have h := Nat.mul_lt_mul_of_pos_left hq hk
  simpa [Nat.mul_assoc, Nat.mul_left_comm] using h

/-- Lemma 2: positive integral rescalings with L1 distance at most one.
The proof also covers a zero total, whose majority family is empty. -/
theorem scaled_overlap {A : Type} (nodes : List A) (w v : A → Nat)
    (k l : Nat) (hk : 0 < k) (hl : 0 < l)
    (hd : total nodes (fun a => distance (k * w a) (l * v a)) ≤ 1) :
    Frown (majority nodes w) (majority nodes v) := by
  classical
  intro q r hq hr
  apply Classical.byContradiction
  intro hn
  have hdisjoint : ∀ a ∈ nodes, ¬ (q a ∧ r a) := fun a _ hp => hn ⟨a, hp⟩
  have hb := disjoint_bound nodes (fun a => k * w a) (fun a => l * v a) q r hdisjoint
  have hq' := majority_scale nodes w q hk hq
  have hr' := majority_scale nodes v r hl hr
  unfold majority at hq' hr'
  omega

theorem unit_change_overlap {A : Type} (nodes : List A) (w v : A → Nat)
    (hd : total nodes (fun a => distance (w a) (v a)) ≤ 1) :
    Frown (majority nodes w) (majority nodes v) := by
  apply scaled_overlap nodes w v 1 1 (by decide) (by decide)
  simpa using hd

theorem self_overlap {A : Type} (nodes : List A) (w : A → Nat) :
    Frown (majority nodes w) (majority nodes w) := by
  apply unit_change_overlap
  have hz : total nodes (fun a => distance (w a) (w a)) = 0 := by
    induction nodes with
    | nil => rfl
    | cons a rest ih =>
      change distance (w a) (w a) + total rest (fun a => distance (w a) (w a)) = 0
      rw [ih]
      simp [distance]
  omega

/-- Lemma 4: exact positive rescaling preserves intersection. -/
theorem scaled_equal_overlap {A : Type} (nodes : List A) (w v : A → Nat)
    (k l : Nat) (hk : 0 < k) (hl : 0 < l)
    (he : ∀ a ∈ nodes, k * w a = l * v a) :
    Frown (majority nodes w) (majority nodes v) := by
  apply scaled_overlap nodes w v k l hk hl
  have hz : total nodes (fun a => distance (k * w a) (l * v a)) = 0 := by
    induction nodes with
    | nil => rfl
    | cons a rest ih =>
      have ht := ih (fun b hb => he b (List.mem_cons_of_mem a hb))
      change distance (k * w a) (l * v a) + _ = 0
      rw [he a (List.mem_cons_self), ht]
      simp [distance]
  omega

def leftWeight (a : Bool) : Nat := if a then 0 else 1
def rightWeight (a : Bool) : Nat := if a then 1 else 0

/-- Sharpness of a uniform distance allowance: replacing ≤1 by ≤2 would
admit disjoint majorities. This is an existential sharpness statement. -/
theorem distance_two_counterexample :
    total [false, true] (fun a => distance (leftWeight a) (rightWeight a)) = 2 ∧
    ¬ Frown (majority [false, true] leftWeight) (majority [false, true] rightWeight) := by
  constructor
  · decide
  · intro h
    obtain ⟨a, ha, hb⟩ := h (fun a => a = false) (fun a => a = true)
      (by simp [majority, mass, total, leftWeight])
      (by simp [majority, mass, total, rightWeight])
    exact Bool.false_ne_true (ha.symm.trans hb)

end WeightedGeneral
