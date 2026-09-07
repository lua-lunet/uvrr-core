import Solve.SetDuper

set_option auto.native true
set_option maxHeartbeats 8000000

/-- Commutativity of the group operation from the group axioms. -/
theorem mul_comm_of_axioms {G : Type}
    (mul : G → G → G) (e : G) (inv : G → G)
    (assoc : ∀ a b c : G, mul (mul a b) c = mul a (mul b c))
    (mul_e : ∀ a : G, mul a e = a)
    (e_mul : ∀ a : G, mul e a = a)
    (inv_r : ∀ a : G, mul a (inv a) = e)
    (inv_mul : ∀ a : G, mul (inv a) a = e)
    (a b : G) : mul a b = mul b a := by
  auto [assoc, mul_e, e_mul, inv_r, inv_mul]
