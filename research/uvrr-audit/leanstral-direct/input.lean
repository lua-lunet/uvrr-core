/- Core Lean 4.33.1 only. Single-file proof task. -/
namespace WeightedContribution

def distance (x y : Nat) : Nat := (x - y) + (y - x)

def total {A : Type} (nodes : List A) (f : A → Nat) : Nat :=
  match nodes with
  | [] => 0
  | a :: rest => f a + total rest f

/-- Disjoint weighted selections obey an L1 bound. -/
theorem disjoint_bound {A : Type} (nodes : List A) (w v : A → Nat)
    (q r : A → Bool) (h : ∀ a ∈ nodes, q a = true → r a = false) :
    2 * total nodes (fun a => if q a then w a else 0) +
      2 * total nodes (fun a => if r a then v a else 0) ≤
    total nodes w + total nodes v + total nodes (fun a => distance (w a) (v a)) := by
  sorry

end WeightedContribution
