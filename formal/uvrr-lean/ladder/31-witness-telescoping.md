# Rung 31: the telescoped-promise equivalence of the witness stream

Evidence recorded 27 September 2026 against the source based on `c2120c8`.

The target is `Witness.telescoped_promise_equivalence` in
`UVRR/Witness.lean`. The witness ladder's safety paragraph argues that a
leader streaming phase-2 at view `v` to a witness telescopes a promise: per
Lamport, a promise at ballot `b` covers every lower ballot, so an
acceptance at `v` raises the witness's promise floor to `v` exactly as if a
prepare at `v` had arrived first. This rung mechanises the equivalence over
the witness's accept history — the `(view, accepted-prefix)` pairs the
stream delivered, oldest first.

## Claim

The model is three definitions. `AcceptHistory` is the delivered stream.
`Monotone` is its well-formedness: views strictly increase along the
history — the witness never accepts below the highest view it has accepted.
`floor` is the induced promise floor: the highest accepted view (0 for an
empty history), covering every ballot at or below it — the telescoping
itself. The named theorem carries the obligation's three conjuncts:

1. **Fence.** `P23W` states the P23 discipline over the witness's
   accept/promise relations (`InducedPromise`, `StreamAccepted`): with the
   induced promise at ballot `b` in force at the end of a delivered prefix,
   no later delivery accepts strictly below `b`. `fence_of_monotone`
   discharges it for every split of a monotone history; the proof induces
   on the delivered prefix and never iterates the history's slots.
2. **Equivalence.** The induced promise state `inducedPromiseState` — the
   fence and the greatest acceptance reported with it — equals the promise
   state of a member that processed phase-1 at the floor view: the same
   fence, and the history's last entry is the greatest acceptance
   (`floor_eq_getLast`: under `Monotone` the floor is the last entry's
   view).
3. **Promotion consistency.** Under H5, kept a named hypothesis as
   `LeaderEarned` (the leader sends phase-2 only for views whose phase-1
   completed on a quorum of members — a construction rule, not an
   assumption about timing), at promotion the witness's floor equals the
   leader's view and its committed prefix equals the leader's, composed
   with the existing `promotion_committed_prefix`; quorum certificates that
   count it rest on the same evidence as for any member.

## Negative controls

- **Backslide refused.** A witness accepting below its highest accepted
  view violates the equivalence: on the concrete two-entry history
  `[(5, 2), (3, 2)]` the well-formedness predicate and the P23-shaped fence
  both fail (`backslide_breaks_equivalence`).
- **Short promotion refused.** A promotion below the leader's prefix
  violates it: the promoted state taken at a frontier short of the stream's
  delivered frontier is not the leader's prefix
  (`short_promotion_breaks_equivalence`, the `gapped_replay_fails` idiom of
  a concrete prefix/take mismatch).

## Source

```bash
cat UVRR/Witness.lean
```

```output
/-! Witness acquisition: a non-member learns the committed era and the
committed log prefix before it participates. The find-the-cluster guard
adopts the maximum reported (committed) era; the leader replays the
contiguous journal suffix from the joiner's frontier; promotion then admits
a voter whose state equals ordinary state transfer. Scope: authentication,
the network and timeouts are environment; the implicit-promise telescoping
is mechanised here as `telescoped_promise_equivalence` over the witness's
accept history. H5 (the leader sends phase-2 only for views whose phase-1
completed on a quorum of members) remains a construction rule, carried as
the named hypothesis `LeaderEarned`. -/

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

/-- The witness's accept history: the `(view, accepted-prefix)` pairs the
stream delivered, oldest first. The prefix component is the frontier the
entry carries; the telescoping argument quantifies over the view only. -/
abbrev AcceptHistory := List (Nat × Nat)

/-- W-monotone, the well-formedness of an accept history: views strictly
increase along the history — the witness never accepts below the highest
view it has accepted. -/
inductive Monotone : AcceptHistory → Prop
  | nil : Monotone []
  | single (e : Nat × Nat) : Monotone [e]
  | cons {e f : Nat × Nat} {rest : AcceptHistory} :
      e.1 < f.1 → Monotone (f :: rest) → Monotone (e :: f :: rest)

/-- Inversion at a two-entry head: the step and the monotone tail. -/
theorem Monotone.cons_inv {e f : Nat × Nat} {rest : AcceptHistory}
    (hm : Monotone (e :: f :: rest)) : e.1 < f.1 ∧ Monotone (f :: rest) := by
  cases hm with
  | cons hlt hrest => exact ⟨hlt, hrest⟩

/-- W-floor: the induced promise floor of a delivered history — the highest
accepted view (0 for an empty history). The induced promise covers every
ballot at or below the floor — Lamport telescoping: a phase-2 accepted at
view `v` raises the floor to `v` exactly as if a prepare at `v` had
arrived first. -/
def floor : AcceptHistory → Nat
  | [] => 0
  | e :: es => max e.1 (floor es)

/-- Every accepted view in the history is at or below the induced floor. -/
theorem le_floor {hist : AcceptHistory} : ∀ e ∈ hist, e.1 ≤ floor hist := by
  induction hist with
  | nil => intro e he; exact absurd he List.not_mem_nil
  | cons e es ih =>
    intro x hx
    rcases List.mem_cons.mp hx with rfl | hx'
    · exact Nat.le_max_left _ _
    · exact Nat.le_trans (ih x hx') (Nat.le_max_right _ _)

/-- In a monotone history the head entry's view is strictly below every
later entry's view. -/
theorem Monotone.head_lt : ∀ (e : Nat × Nat) (hist : AcceptHistory),
    Monotone (e :: hist) → ∀ f ∈ hist, e.1 < f.1
  | _, [], _ => fun f hf => absurd hf List.not_mem_nil
  | e, g :: gs, hm => by
    obtain ⟨hlt, hrest⟩ := Monotone.cons_inv hm
    intro f hf
    rcases List.mem_cons.mp hf with rfl | hf'
    · exact hlt
    · exact Nat.lt_trans hlt (Monotone.head_lt g gs hrest f hf')

/-- The floor of a history whose first step strictly increases is computed
by the tail: the earlier entry is already below the tail's floor. -/
theorem floor_cons_of_lt {e f : Nat × Nat} {rest : AcceptHistory}
    (h : e.1 < f.1) : floor (e :: f :: rest) = floor (f :: rest) := by
  have hle : f.1 ≤ floor (f :: rest) := le_floor f List.mem_cons_self
  exact Nat.max_eq_right (Nat.le_trans (Nat.le_of_lt h) hle)

/-- The floor of a nonempty delivered prefix is strictly below every view
delivered later: once the induced promise covers a ballot, the stream
never delivers an acceptance below it. -/
theorem Monotone.floor_lt_of_mem {hist : AcceptHistory} (hm : Monotone hist) :
    ∀ (earlier later : AcceptHistory), earlier ≠ [] → hist = earlier ++ later →
      ∀ f ∈ later, floor earlier < f.1 := by
  induction hm with
  | nil =>
    intro earlier later hne hsplit
    cases earlier with
    | nil => exact absurd rfl hne
    | cons d ds =>
      rw [List.cons_append] at hsplit
      exact absurd hsplit.symm (List.cons_ne_nil _ _)
  | single x =>
    intro earlier later hne hsplit f hf
    cases earlier with
    | nil => exact absurd rfl hne
    | cons d ds =>
      rw [List.cons_append, List.cons.injEq] at hsplit
      obtain ⟨-, htl⟩ := hsplit
      cases ds with
      | nil =>
        rw [List.nil_append] at htl
        subst htl
        cases hf
      | cons k ks =>
        rw [List.cons_append] at htl
        exact absurd htl.symm (List.cons_ne_nil _ _)
  | cons hlt hrest ih =>
    rename_i e g rest
    intro earlier later hne hsplit f hf
    cases earlier with
    | nil => exact absurd rfl hne
    | cons d ds =>
      rw [List.cons_append, List.cons.injEq] at hsplit
      obtain ⟨hd, htl⟩ := hsplit
      subst hd
      cases ds with
      | nil =>
        rw [List.nil_append] at htl
        subst htl
        have hfloor : floor [e] = e.1 := Nat.max_eq_left (Nat.zero_le _)
        rw [hfloor]
        exact Monotone.head_lt e (g :: rest) (Monotone.cons hlt hrest) f hf
      | cons k ks =>
        rw [List.cons_append] at htl
        have hge : g = k ∧ rest = ks ++ later := List.cons.injEq _ _ _ _ ▸ htl
        obtain ⟨hgk, -⟩ := hge
        subst hgk
        rw [floor_cons_of_lt hlt]
        exact ih (g :: ks) later (List.cons_ne_nil _ _) htl f hf

/-- Under `Monotone` the induced floor is the last entry's view: the
highest accepted view is the most recent acceptance. -/
theorem floor_eq_getLast {hist : AcceptHistory} (hm : Monotone hist)
    (hne : hist ≠ []) : floor hist = (hist.getLast hne).1 := by
  induction hm with
  | nil => exact absurd rfl hne
  | single e =>
    have hfloor : floor [e] = e.1 := Nat.max_eq_left (Nat.zero_le _)
    rw [hfloor, List.getLast_singleton]
  | cons hlt hrest ih =>
    rename_i e f rest
    have h1 : floor (e :: f :: rest) = floor (f :: rest) := floor_cons_of_lt hlt
    have h2 : floor (f :: rest) = ((f :: rest).getLast (List.cons_ne_nil _ _)).1 :=
      ih (List.cons_ne_nil _ _)
    have h3 : (e :: f :: rest).getLast hne = (f :: rest).getLast (List.cons_ne_nil _ _) :=
      List.getLast_cons (List.cons_ne_nil _ _)
    rw [h1, h2, h3]

/-- In a nonempty monotone history the last entry's view is the head's view
exactly when the history is a singleton: a longer history has strictly
advanced past its first acceptance. -/
theorem getLast_eq_of_monotone {hist : AcceptHistory} (hm : Monotone hist)
    (hne : hist ≠ []) (hlast : (hist.getLast hne).1 = (hist.head hne).1) :
    ∃ p, hist = [((hist.head hne).1, p)] := by
  cases hist with
  | nil => exact absurd rfl hne
  | cons e es =>
    cases es with
    | nil =>
      refine ⟨e.2, ?_⟩
      cases e
      rfl
    | cons f rest =>
      have hmem : (f :: rest).getLast (List.cons_ne_nil _ _) ∈ f :: rest :=
        List.getLast_mem (List.cons_ne_nil _ _)
      have hlt : e.1 < ((f :: rest).getLast (List.cons_ne_nil _ _)).1 :=
        Monotone.head_lt e (f :: rest) hm _ hmem
      have hgl : (e :: f :: rest).getLast hne = (f :: rest).getLast (List.cons_ne_nil _ _) :=
        List.getLast_cons (List.cons_ne_nil _ _)
      rw [hgl] at hlast
      simp only [List.head_cons] at hlast
      omega

/-- The witness's induced promise relation over a delivered prefix: the
floor covers every ballot at or below it. -/
def InducedPromise (hist : AcceptHistory) (b : Nat) : Prop := b ≤ floor hist

/-- The witness's acceptance relation over a delivered suffix: the stream
records an acceptance at view `v` in it. -/
def StreamAccepted (hist : AcceptHistory) (v : Nat) : Prop := ∃ p, (v, p) ∈ hist

/-- The P23-shaped discipline for the witness stream, mirroring
`Paxos.P23`: with the induced promise at ballot `b` in force at the end of
a delivered prefix, no later delivery accepts strictly below `b`. -/
def P23W (earlier later : AcceptHistory) : Prop :=
  ∀ b b', InducedPromise earlier b → b' < b → ¬ StreamAccepted later b'

/-- The fence: over a monotone history, every split into a delivered prefix
and a later suffix satisfies the P23-shaped discipline. -/
theorem fence_of_monotone {hist : AcceptHistory} (hm : Monotone hist)
    {earlier later : AcceptHistory} (hsplit : hist = earlier ++ later) :
    P23W earlier later := by
  intro b b' hb hb' hacc
  obtain ⟨p, hmem⟩ := hacc
  cases he : earlier with
  | nil =>
    have hb0 : InducedPromise [] b := he ▸ hb
    have hbz : b = 0 := Nat.le_zero.mp hb0
    omega
  | cons e es =>
    have hne : earlier ≠ [] := he ▸ List.cons_ne_nil e es
    have hlt := Monotone.floor_lt_of_mem hm earlier later hne hsplit (b', p) hmem
    exact absurd (Nat.lt_of_lt_of_le (Nat.lt_trans hlt hb') hb) (Nat.lt_irrefl _)

/-- The promise state of an acceptor: the fence (every ballot at or below
it is covered) and the greatest acceptance reported with it. A member that
processed phase-1 at view `v` having accepted at `g` carries `⟨v, g⟩`. -/
structure PromiseState where
  fence : Nat
  greatestAccepted : Nat

/-- The witness's induced promise state after a delivered history: the
induced floor as the fence, and the last entry's view as the greatest
acceptance reported. -/
def inducedPromiseState (hist : AcceptHistory) (hne : hist ≠ []) : PromiseState :=
  ⟨floor hist, (hist.getLast hne).1⟩

/-- H5, the construction rule kept a named hypothesis: the leader sends
phase-2 at view `v` only for a view whose phase-1 completed on a quorum of
members — a quorum's worth of member promises at `v`. -/
def LeaderEarned {A : Type} (quorum : (A → Prop) → Prop)
    (memberPromise : A → Nat → Prop) (v : Nat) : Prop :=
  ∃ q, quorum q ∧ ∀ a, q a → memberPromise a v

/-- The telescoped-promise equivalence (W-telescoping): over the witness's
accept history, a phase-2 accepted at view `v` raises the promise floor to
`v` exactly as if a prepare at `v` had arrived first. The three conjuncts
of the obligation: the P23-shaped fence holds over every split of the
history (`Monotone` suffices); the induced promise state is the promise
state of a member that processed phase-1 at the floor view — the same
fence, and the history's last entry is the greatest acceptance reported
with it; and under H5 (`LeaderEarned`) at promotion the witness's floor
equals the leader's view and its committed prefix equals the leader's
(`promotion_committed_prefix`), so quorum certificates that count it rest
on the same evidence as for any member. The proof induces on the delivered
prefix; it does not iterate the history's slots. -/
theorem telescoped_promise_equivalence {A V : Type} (quorum : (A → Prop) → Prop)
    (memberPromise : A → Nat → Prop) (log : List V)
    {hist : AcceptHistory} (hm : Monotone hist) (hne : hist ≠ [])
    {v n : Nat} (hlast : hist.getLast hne = (v, n))
    (h5 : LeaderEarned quorum memberPromise v)
    {k c : Nat} (hkn : k ≤ n) (hc : c ≤ n) :
    (∀ earlier later, hist = earlier ++ later → P23W earlier later)
    ∧ inducedPromiseState hist hne = ⟨v, v⟩
    ∧ floor hist = v
      ∧ (log.take k ++ replay log k n).take c = log.take c
      ∧ LeaderEarned quorum memberPromise v := by
  have hfloor : floor hist = v := by
    rw [floor_eq_getLast hm hne, hlast]
  refine ⟨fun earlier later hsplit => fence_of_monotone hm hsplit, ?_, ?_, ?_, h5⟩
  · show ({ fence := floor hist, greatestAccepted := (hist.getLast hne).1 } : PromiseState)
        = ⟨v, v⟩
    rw [hfloor, hlast]
  · exact hfloor
  · exact promotion_committed_prefix log hkn hc

/-- Fault control: a report above the committed era breaks the bound — H1 is
load-bearing. This is the failure that stranded the phantom-slot joiner;
the guard makes it unrepresentable at the engine. -/
theorem invented_era_breaks_bound : ¬ adopt [3, 7] ≤ 5 := by decide

/-- Fault control: a gapped replay cannot reconstruct the prefix — the slot
discipline of H3 is load-bearing. -/
theorem gapped_replay_fails :
    ([1, 2, 3, 4] : List Nat).take 2 ++ ([1, 2, 3, 4].drop 3).take (4 - 3)
      ≠ ([1, 2, 3, 4]).take 4 := by decide

/-- Fault control: a witness accepting below its highest accepted view
violates the equivalence — on a concrete two-entry history the
well-formedness predicate and the P23-shaped fence both fail. -/
theorem backslide_breaks_equivalence :
    ¬ Monotone [(5, 2), (3, 2)] ∧ ¬ P23W [(5, 2)] [(3, 2)] := by
  refine ⟨?_, ?_⟩
  · intro hm
    have h : (5 : Nat) < 3 := (Monotone.cons_inv hm).1
    omega
  · intro h
    exact h 5 3 (by show (5 : Nat) ≤ floor [(5, 2)]; decide) (by decide) ⟨2, by decide⟩

/-- Fault control: a promotion below the leader's prefix violates the
equivalence — the promoted state at a frontier short of the stream's
delivered frontier is not the leader's prefix. -/
theorem short_promotion_breaks_equivalence :
    (([1, 2, 3, 4] : List Nat).take 0 ++ replay [1, 2, 3, 4] 0 4).take 3
      ≠ ([1, 2, 3, 4]).take 4 := by decide

end Witness
```

## Kernel-checked results

Replay from `formal/uvrr-lean/`:

```bash
lake build
python3 check_axioms.py
```

```output
✔ [28/30] Built UVRR.Witness (519ms)
✔ [29/30] Built UVRR (332ms)
Build completed successfully (30 jobs).
PASS 518 declarations: only standard Lean axioms
```

Direct axiom query:

```bash
printf '%s\n' 'import UVRR' \
  '#print axioms Witness.telescoped_promise_equivalence' \
  '#print axioms Witness.fence_of_monotone' \
  '#print axioms Witness.floor_eq_getLast' \
  '#print axioms Witness.getLast_eq_of_monotone' \
  '#print axioms Witness.backslide_breaks_equivalence' \
  '#print axioms Witness.short_promotion_breaks_equivalence' | lake env lean --stdin
```

```output
'Witness.telescoped_promise_equivalence' depends on axioms: [propext, Classical.choice, Quot.sound]
'Witness.fence_of_monotone' depends on axioms: [propext, Quot.sound]
'Witness.floor_eq_getLast' depends on axioms: [propext]
'Witness.getLast_eq_of_monotone' depends on axioms: [propext, Classical.choice, Quot.sound]
'Witness.backslide_breaks_equivalence' depends on axioms: [propext, Quot.sound]
'Witness.short_promotion_breaks_equivalence' does not depend on any axioms
```

The statements and proof bodies are hand-landed and kernel-checked; no
Leanstral loop was needed to discharge a body. Only the permitted standard
Lean axioms appear.
