/-
  Rung 7 — Voting weights: the 3-zone hot-swap with weighted majorities.

  The paper (Appendix A) defines the weighted-majority configuration
  M(w) = { q | 2·Σ_{a∈q} w(a) > Σ_a w(a) } and proves (Lemma 2/3/4) that M(w) ⌢ M(w')
  whenever the scaled weights differ on at most one node by at most 1, or are
  integer multiples of each other. Here we CHECK the blog's concrete 7-step
  schedule (doubling all weights, then ±1 on one node at a time, then halving)
  by enumerating all 16 subsets of {W,X,Y,Z} — kernel-checked by `decide`.

  Negative control: bumping one node's weight by 2 in a single step breaks the
  frown, showing the "differ by ≤ 1" bound in Lemma 3 is tight.
-/
import UVRR.Structure

/-- All sublists (subsets) of a list. -/
def subs : List Nat → List (List Nat)
  | [] => [[]]
  | a :: l => let s := subs l; s ++ s.map (a :: ·)

/-- Weight of node a under weight vector w (node index = position). -/
def wt (w : List Nat) (a : Nat) : Nat := w.getD a 0

def wsum (w : List Nat) (q : List Nat) : Nat := (q.map (wt w)).sum

/-- Weighted majorities of nodes 0..3 under w, as an explicit quorum list. -/
def majW (w : List Nat) : List (List Nat) :=
  (subs [0,1,2,3]).filter fun q => decide (2 * wsum w q > wsum w [0,1,2,3])

/-- The blog's schedule (W,X,Y,Z), one row per era. -/
def rows : List (List Nat) :=
  [[1,1,1,0], [2,2,2,0], [2,2,2,1], [2,2,1,1], [2,2,0,1], [2,2,0,2], [1,1,0,1]]

def cfgW (w : List Nat) : CfgL := ⟨majW w, majW w⟩

/-- Boolean check of P1 along the whole schedule. -/
def scheduleOk : List (List Nat) → Bool
  | [] => true
  | [w] => frownB (majW w) (majW w)
  | w :: w' :: rest => p1B (cfgW w) (cfgW w') && scheduleOk (w' :: rest)

theorem rows_ok : scheduleOk rows = true := by decide

/-- Consequently every consecutive pair of rows satisfies the UPaxos frowns. -/
theorem rows_p1 :
    Frown (ofList (majW [1,1,1,0])) (ofList (majW [2,2,2,0])) ∧
    Frown (ofList (majW [2,2,2,0])) (ofList (majW [2,2,2,1])) ∧
    Frown (ofList (majW [2,2,2,1])) (ofList (majW [2,2,1,1])) ∧
    Frown (ofList (majW [2,2,1,1])) (ofList (majW [2,2,0,1])) ∧
    Frown (ofList (majW [2,2,0,1])) (ofList (majW [2,2,0,2])) ∧
    Frown (ofList (majW [2,2,0,2])) (ofList (majW [1,1,0,1])) := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> (rw [← frownB_iff]; decide)

/-- Negative control: [1,1,1,0] → [3,1,1,0] (one node +2) breaks the frown:
{X,Y} is a majority under the first, {W} alone under the second. -/
theorem plus_two_breaks : ¬ Frown (ofList (majW [1,1,1,0])) (ofList (majW [3,1,1,0])) := by
  rw [← frownB_iff]; decide
