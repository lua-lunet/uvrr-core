# Rung 14: Normal-log message provenance

*2026-09-11T00:42:06Z by Showboat 0.6.1*
<!-- showboat-id: e7f53053-46db-4002-a19a-e0ebd647253c -->

Fixed-view operational induction derives comparable report logs and retention of replica prefixes. Prepare wire data is a slot and entry; the pre-send log is ghost evidence. Receivers enforce the next slot without a global agreement test. Delayed and duplicate messages are allowed. An authentic out-of-order delivery witnesses failure when the slot guard is bypassed. Unique primary activation and simulation across view changes and crashes remain open composition obligations.

```bash
cat UVRR/NormalLog.lean
```

```output
import UVRR.ViewSelection

/-! Fixed-view VRR message projection. The primary starts from an arbitrary
installed log and appends. Prepare.before is ghost send-history evidence;
the wire projection is (length + 1, entry). Receivers check only the slot.
StartView installs the base once. Messages persist, allowing arbitrary delay
and duplication. Crash/restart and unique primary activation remain explicit
composition obligations; no transition tests global prefix agreement.
-/
namespace NormalLog

structure Prepare (V : Type) where
  before : List V
  entry : V

def Prepare.slot {V : Type} (m : Prepare V) : Nat := m.before.length + 1

structure State (A V : Type) where
  primary : List V
  replicas : A → Option (List V)
  messages : List (Prepare V)
  reports : List (A × List V)

def initial {A V : Type} (base : List V) : State A V :=
  ⟨base, fun _ => none, [], []⟩

noncomputable def put {A X : Type} (f : A → X) (a : A) (x : X) : A → X := by
  classical
  exact fun b => if b = a then x else f b

@[simp]
theorem put_same {A X : Type} (f : A → X) (a : A) (x : X) : put f a x a = x := by
  simp [put]

@[simp]
theorem put_other {A X : Type} (f : A → X) (a b : A) (x : X)
    (h : b ≠ a) : put f a x b = f b := by simp [put, h]

inductive Step {A V : Type} (base : List V) : State A V → State A V → Prop
  | propose (s : State A V) (x : V) : Step base s
      {s with primary := s.primary ++ [x],
              messages := ⟨s.primary, x⟩ :: s.messages}
  | start (s : State A V) (a : A) (notNormal : s.replicas a = none) :
      Step base s {s with replicas := put s.replicas a (some base)}
  | receive (s : State A V) (a : A) (log : List V) (m : Prepare V)
      (normal : s.replicas a = some log) (sent : m ∈ s.messages)
      (next : m.slot = log.length + 1) :
      Step base s {s with replicas := put s.replicas a (some (log ++ [m.entry]))}
  | report (s : State A V) (a : A) (log : List V)
      (normal : s.replicas a = some log) :
      Step base s {s with reports := (a, log) :: s.reports}

inductive Reachable {A V : Type} (base : List V) : State A V → Prop
  | init : Reachable base (initial base)
  | step {s t} : Reachable base s → Step base s t → Reachable base t

structure Inv {A V : Type} (base : List V) (s : State A V) : Prop where
  base_prefix : base <+: s.primary
  replica_prefix : ∀ a log, s.replicas a = some log → log <+: s.primary
  message_prefix : ∀ m ∈ s.messages, m.before ++ [m.entry] <+: s.primary
  report_prefix : ∀ r ∈ s.reports, r.2 <+: s.primary

theorem initial_inv {A V : Type} (base : List V) : Inv base (initial (A := A) base) := by
  constructor
  · exact List.prefix_rfl
  · intro a log h; cases h
  · simp [initial]
  · simp [initial]

/-- Equal-length prefixes of the send history coincide. The receiver checks
the next slot, not the contents of the ghost pre-send log. -/
theorem receive_prefix {A V : Type} {base : List V} {s : State A V}
    (h : Inv base s) (a : A) (log : List V) (m : Prepare V)
    (normal : s.replicas a = some log) (sent : m ∈ s.messages)
    (next : m.slot = log.length + 1) : log ++ [m.entry] <+: s.primary := by
  have hm := h.message_prefix m sent
  have hb : m.before <+: s.primary := (List.prefix_append _ _).trans hm
  have hl := h.replica_prefix a log normal
  have he : log.length = m.before.length := by simp [Prepare.slot] at next; omega
  have eq : log = m.before :=
    (List.prefix_of_prefix_length_le hl hb (by omega)).eq_of_length_le (by omega)
  rw [eq]; exact hm

theorem step_inv {A V : Type} {base : List V} {s t : State A V}
    (h : Inv base s) (st : Step base s t) : Inv base t := by
  cases st with
  | propose x =>
    constructor
    · exact List.prefix_append_of_prefix h.base_prefix
    · intro a log ha; exact List.prefix_append_of_prefix (h.replica_prefix a log ha)
    · intro m hm
      rcases List.mem_cons.mp hm with he | hm
      · subst m; exact List.prefix_rfl
      · exact List.prefix_append_of_prefix (h.message_prefix m hm)
    · intro r hr; exact List.prefix_append_of_prefix (h.report_prefix r hr)
  | start a hn =>
    constructor
    · exact h.base_prefix
    · intro b log hb
      by_cases he : b = a
      · subst b; simp only [put_same, Option.some.injEq] at hb
        subst log; exact h.base_prefix
      · simp only [put_other _ a b _ he] at hb
        exact h.replica_prefix b log hb
    · exact h.message_prefix
    · exact h.report_prefix
  | receive a log m hn hm hnext =>
    constructor
    · exact h.base_prefix
    · intro b log' hb
      by_cases he : b = a
      · subst b; simp only [put_same, Option.some.injEq] at hb
        subst log'; exact receive_prefix h a log m hn hm hnext
      · simp only [put_other _ a b _ he] at hb
        exact h.replica_prefix b log' hb
    · exact h.message_prefix
    · exact h.report_prefix
  | report a log hn =>
    constructor
    · exact h.base_prefix
    · exact h.replica_prefix
    · exact h.message_prefix
    · intro r hr
      rcases List.mem_cons.mp hr with he | hr
      · subst r; exact h.replica_prefix a log hn
      · exact h.report_prefix r hr

theorem reachable_inv {A V : Type} {base : List V} {s : State A V}
    (h : Reachable base s) : Inv base s := by
  induction h with
  | init => exact initial_inv _
  | step _ st ih => exact step_inv ih st

/-- Same-view report comparability follows from the message transitions. -/
theorem reports_comparable {A V : Type} {base : List V} {s : State A V}
    (h : Reachable base s) (r q : A × List V)
    (hr : r ∈ s.reports) (hq : q ∈ s.reports) : r.2 <+: q.2 ∨ q.2 <+: r.2 :=
  List.prefix_or_prefix_of_prefix ((reachable_inv h).report_prefix r hr)
    ((reachable_inv h).report_prefix q hq)

/-- Interface to the view selector for reports from this fixed view. -/
theorem selection_same_view {A V : Type} {base : List V} {s : State A V}
    (h : Reachable base s) (r q : ViewSelection.Report A V)
    (hr : (r.sender, r.log) ∈ s.reports) (hq : (q.sender, q.log) ∈ s.reports) :
    r.log <+: q.log ∨ q.log <+: r.log := reports_comparable h _ _ hr hq

/-- A normal replica cannot lose its accepted prefix within this view. -/
theorem step_replica_monotone {A V : Type} {base : List V} {s t : State A V}
    (st : Step base s t) (a : A) (log : List V)
    (hn : s.replicas a = some log) :
    ∃ nextLog, t.replicas a = some nextLog ∧ log <+: nextLog := by
  cases st with
  | propose x => exact ⟨log, hn, List.prefix_rfl⟩
  | report b other hb => exact ⟨log, hn, List.prefix_rfl⟩
  | start b hb =>
    by_cases he : a = b
    · subst a; rw [hn] at hb; cases hb
    · exact ⟨log, by simpa [put_other _ b a _ he] using hn, List.prefix_rfl⟩
  | receive b other m hb hm hnext =>
    by_cases he : a = b
    · subst a
      have heq : log = other := Option.some.inj (hn.symm.trans hb)
      subst other
      exact ⟨log ++ [m.entry], put_same _ _ _, List.prefix_append _ _⟩
    · exact ⟨log, by simpa [put_other _ b a _ he] using hn, List.prefix_rfl⟩

inductive Later {A V : Type} (base : List V) : State A V → State A V → Prop
  | refl (s) : Later base s s
  | step {s t u} : Later base s t → Step base t u → Later base s u

/-- Historical vote prefixes survive any finite fixed-view continuation. -/
theorem replica_history {A V : Type} {base : List V} {s t : State A V}
    (run : Later base s t) (a : A) (log : List V)
    (hn : s.replicas a = some log) :
    ∃ nextLog, t.replicas a = some nextLog ∧ log <+: nextLog := by
  induction run with
  | refl => exact ⟨log, hn, List.prefix_rfl⟩
  | step run st ih =>
    obtain ⟨middle, hm, hp⟩ := ih
    obtain ⟨last, hl, hpl⟩ := step_replica_monotone st a middle hm
    exact ⟨last, hl, hp.trans hpl⟩

/-- A concrete reachable send history: slot 2 can arrive before slot 1. -/
noncomputable def beforeOutOfOrder : State Unit Bool :=
  let s0 := initial (A := Unit) ([] : List Bool)
  let s1 := {s0 with primary := [false], messages := [⟨[], false⟩]}
  let s2 := {s1 with primary := [false, true], messages := ⟨[false], true⟩ :: s1.messages}
  {s2 with replicas := put s2.replicas () (some [])}

theorem beforeOutOfOrder_reachable : Reachable [] beforeOutOfOrder := by
  exact .step (.step (.step .init (.propose _ false)) (.propose _ true))
    (.start _ () rfl)

/-- Fault control: accepting an authentic slot-2 message at empty slot 1
violates the invariant. The normal receive guard rejects this exact input. -/
theorem missing_slot_guard_breaks_prefix :
    Reachable [] beforeOutOfOrder ∧
    beforeOutOfOrder.replicas () = some [] ∧
    (⟨[false], true⟩ : Prepare Bool) ∈ beforeOutOfOrder.messages ∧
    (⟨[false], true⟩ : Prepare Bool).slot ≠ ([] : List Bool).length + 1 ∧
    ¬ ([true] <+: beforeOutOfOrder.primary) := by
  refine ⟨beforeOutOfOrder_reachable, ?_, ?_, by decide, ?_⟩
  · simp [beforeOutOfOrder]
  · simp [beforeOutOfOrder]
  · intro ⟨suffix, he⟩
    change [true] ++ suffix = [false, true] at he
    cases he

end NormalLog
```

```bash
lake env lean UVRR/NormalLog.lean
```

```output
```

```bash
printf 'import UVRR.NormalLog\n#print axioms NormalLog.reachable_inv\n#print axioms NormalLog.reports_comparable\n#print axioms NormalLog.replica_history\n#print axioms NormalLog.missing_slot_guard_breaks_prefix\n' | lake env lean --stdin
```

```output
'NormalLog.reachable_inv' depends on axioms: [propext, Classical.choice, Quot.sound]
'NormalLog.reports_comparable' depends on axioms: [propext, Classical.choice, Quot.sound]
'NormalLog.replica_history' depends on axioms: [propext, Classical.choice, Quot.sound]
'NormalLog.missing_slot_guard_breaks_prefix' depends on axioms: [propext, Classical.choice, Quot.sound]
```
