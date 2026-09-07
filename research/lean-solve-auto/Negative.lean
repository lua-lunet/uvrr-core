import Solve.SetDuper

set_option auto.native true
set_option maxHeartbeats 1000000

/-- Negative control: needs induction on Nat; the superposition prover must fail. -/
theorem nat_add_comm_needs_induction (n m : Nat) : n + m = m + n := by
  auto
