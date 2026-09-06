import Lean

/-! ## Small arithmetic theorems -/

theorem one_plus_one : 1 + 1 = 2 := by
  rfl

theorem two_plus_two : 2 + 2 = 4 := by
  decide

theorem five_mul_three : 5 * 3 = 15 := by
  decide

theorem zero_add (n : Nat) : 0 + n = n := by
  omega

theorem succ_pred_eq_of_pos (n : Nat) (h : n > 0) : Nat.succ (Nat.pred n) = n := by
  cases n
  · exact (Nat.lt_irrefl 0 h).elim
  · rfl

