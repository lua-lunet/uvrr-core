import Solve.SetDuper

set_option auto.native true
set_option maxHeartbeats 1000000

/-- Symmetry + transitivity of a relation entail the reversed composed edge. -/
theorem sym_trans_composed {α : Type} (r : α → α → Prop)
    (sym : ∀ x y : α, r x y → r y x)
    (trans : ∀ x y z : α, r x y → r y z → r x z)
    (a b c : α) (h1 : r a b) (h2 : r b c) : r c a := by
  auto [sym, trans, h1, h2]
