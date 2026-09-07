import Aesop

theorem aesop_t1 (p q : Prop) : p ∨ q → q ∨ p := by aesop

theorem aesop_t2 (p q : Prop) : (¬ p ∨ ¬ q) → ¬ (p ∧ q) := by aesop

theorem aesop_t3 (a b c : Prop) : (a → b) → (b → c) → a → c := by aesop

theorem aesop_t4 (p q r : Prop) : (p → q → r) → (p ∧ q) → r := by aesop

-- NEGATIVE CONTROL: Diophantine-style equality; aesop should NOT solve it.
theorem aesop_neg (n : Nat) : n ^ 2 + n + 41 = 43 * n → n = 2 := by aesop
