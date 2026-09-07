import Mathlib.Tactic

theorem three_mul_four : (3 : Nat) * 4 = 12 := by
  decide

theorem add_comm_small (a b : Nat) : a + b = b + a := by
  omega

theorem seven_lt_nine : (7 : Nat) < 9 := by
  decide
