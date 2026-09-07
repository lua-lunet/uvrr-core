import Solve.SetDuper

set_option auto.native true
set_option maxHeartbeats 1000000

/-- Right inverse in a group is also a left inverse. -/
theorem left_inv_of_right_inv {G : Type}
    (mul : G → G → G) (e : G) (inv : G → G)
    (assoc : ∀ a b c : G, mul (mul a b) c = mul a (mul b c))
    (mul_e : ∀ a : G, mul a e = a)
    (inv_r : ∀ a : G, mul a (inv a) = e)
    (a : G) : mul (inv a) a = e := by
  auto [assoc, mul_e, inv_r]
