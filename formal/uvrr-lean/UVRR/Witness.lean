/-! Witness acquisition: a non-member learns the committed era and the
committed log prefix before it participates. The find-the-cluster guard
adopts the maximum reported (committed) era; the leader replays the
contiguous journal suffix from the joiner's frontier; promotion then admits
a voter whose state equals ordinary state transfer. Scope: authentication,
the network and timeouts are environment; the implicit-promise telescoping
argument is prose in the addendum, not mechanised here. -/

namespace Witness

/-- H1: respondents report only eras the cluster has committed. Era
advances on commit application, so no honest response names a later era. -/
def Honest (reports : List Nat) (committed : Nat) : Prop :=
  ∀ r ∈ reports, r ≤ committed

/-- H2: the guard's adoption rule — the maximum reported era. -/
def adopt : List Nat → Nat
  | [] => 0
  | r :: rs => if adopt rs ≤ r then r else adopt rs

/-- W-era-bound: the adopted era never exceeds the committed era. -/
theorem adopt_le_of_honest {reports : List Nat} {committed : Nat}
    (h : Honest reports committed) : adopt reports ≤ committed := by
  induction reports with
  | nil => simp [adopt]
  | cons r rs ih =>
    have hr : r ≤ committed := h r List.mem_cons_self
    have hrs : ∀ x ∈ rs, x ≤ committed := fun x hx => h x (List.mem_cons_of_mem _ hx)
    have hrec := ih hrs
    simp only [adopt]
    by_cases hcase : adopt rs ≤ r
    · rw [if_pos hcase]; exact hr
    · rw [if_neg hcase]; exact hrec

/-- W-era-exact: once the leader's response — the committed era itself —
is among the reports, the adoption lands exactly on the committed era. -/
theorem adopt_eq_of_leader {reports : List Nat} {committed : Nat}
    (h : Honest reports committed) (hin : committed ∈ reports) :
    adopt reports = committed := by
  induction reports with
  | nil => cases hin
  | cons r rs ih =>
    have hr : r ≤ committed := h r List.mem_cons_self
    have hrs : ∀ x ∈ rs, x ≤ committed := fun x hx => h x (List.mem_cons_of_mem _ hx)
    rcases List.mem_cons.mp hin with heq | hin'
    · subst heq
      simp only [adopt]
      exact if_pos (adopt_le_of_honest hrs)
    · have hrec := ih hrs hin'
      simp only [adopt, hrec]
      by_cases hcase : committed ≤ r
      · rw [if_pos hcase]; exact Nat.le_antisymm hr hcase
      · rw [if_neg hcase]

/-- H3: the leader's catch-up replay from the joiner's frontier `k` up to
slot `n` — the contiguous suffix of the leader's log, delivered in order. -/
def replay {V : Type} (log : List V) (k n : Nat) : List V :=
  (log.drop k).take (n - k)

/-- W-replay: a journal holding exactly the frontier prefix, extended by the
in-order contiguous suffix, reconstructs the leader's prefix. The ordinary
accept path's slot discipline is what forces the fold to be this prefix:
each entry is admitted only as the next slot. -/
theorem replay_reconstructs {V : Type} (log : List V) {k n : Nat}
    (hkn : k ≤ n) : log.take k ++ replay log k n = log.take n := by
  rw [replay, show n = k + (n - k) by omega, List.take_add,
      show k + (n - k) - k = n - k by omega]

/-- A reincarnated node replays from blank: its frontier is slot 0. -/
theorem replay_from_blank {V : Type} (log : List V) (n : Nat) :
    replay log 0 n = log.take n := by
  simp [replay]

/-- W-promotion: the witness's committed prefix at any committed frontier
equals the leader's committed prefix — hence any member's, whose report is
a prefix of the primary by `NormalLog.reachable_inv`. -/
theorem promotion_committed_prefix {V : Type} (log : List V) {k n c : Nat}
    (hkn : k ≤ n) (hc : c ≤ n) :
    (log.take k ++ replay log k n).take c = log.take c := by
  rw [replay_reconstructs log hkn, List.take_take, Nat.min_eq_left hc]

/-- H4 made formal: quorum membership is decided by the configuration, and
the witness is not in it — no quorum certificate can contain the witness. -/
theorem witness_not_in_quorum {A : Type} (member : A → Prop)
    (quorum : (A → Prop) → Prop)
    (hq : ∀ q, quorum q → ∀ a, q a → member a)
    (w : A) (hw : ¬ member w) {q : A → Prop} (hqq : quorum q) : ¬ q w :=
  fun hqw => hw (hq q hqq w hqw)

/-- Fault control: a report above the committed era breaks the bound — H1 is
load-bearing. This is the failure that stranded the phantom-slot joiner;
the guard makes it unrepresentable at the engine. -/
theorem invented_era_breaks_bound : ¬ adopt [3, 7] ≤ 5 := by decide

/-- Fault control: a gapped replay cannot reconstruct the prefix — the slot
discipline of H3 is load-bearing. -/
theorem gapped_replay_fails :
    ([1, 2, 3, 4] : List Nat).take 2 ++ ([1, 2, 3, 4].drop 3).take (4 - 3)
      ≠ ([1, 2, 3, 4]).take 4 := by decide

end Witness
