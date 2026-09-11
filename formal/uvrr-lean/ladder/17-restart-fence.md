# Rung 17: Replicated fences through fresh recovery episodes

*2026-09-06T08:23:38Z by Showboat 0.6.1*
<!-- showboat-id: 0130e2d4-5baf-48c6-9339-ed94588cd3c3 -->

Starting from a checkpoint where a support quorum is online above a known fence, arbitrary finite crash/restart sequences preserve that bound for online members of the support. Crashes erase local bounds. Recovering replicas cannot reply. Recovery counts distinct identities from current-episode historical replies, even if a responder has since crashed. Generation is an explicit environment freshness abstraction, not a durable protocol field. This proves a fence projection, not restored logs, first-round view-change composition or liveness. A concrete fresh recovery succeeds; two authentic stale replies would form a quorum and restore zero if episode checking were removed.

```bash
cat UVRR/RestartFence.lean
```

```output
import UVRR.NormalLog

/-! Replicated fencing knowledge under volatile-state loss. online=None means
recovering and forbids response generation. A crash erases the local bound.
Generation is ghost freshness metadata supplied by the environment: it models
non-reuse of recovery episode nonces, not a protocol counter surviving on disk.
Responses can be delayed and responders can crash after sending. Restarting uses
historical authenticated replies of the current episode, counted by identity.
This projection establishes a fence lower bound, not log recovery or liveness.
-/
namespace RestartFence

structure Response (A : Type) where
  sender : A
  recipient : A
  generation : Nat
  bound : Nat

structure State (A : Type) where
  online : A → Option Nat
  generation : A → Nat
  responses : List (Response A)

def initial {A : Type} (bounds : A → Nat) : State A :=
  ⟨fun a => some (bounds a), fun _ => 0, []⟩

noncomputable def crash {A : Type} (s : State A) (a : A) : State A :=
  {s with online := NormalLog.put s.online a none,
          generation := NormalLog.put s.generation a (s.generation a + 1)}

noncomputable def onlineAt {A : Type} (s : State A) (a : A) (bound : Nat) : State A :=
  {s with online := NormalLog.put s.online a (some bound)}

def respond {A : Type} (s : State A) (r : Response A) : State A :=
  {s with responses := r :: s.responses}

inductive Step {A : Type} (family : QSys A) : State A → State A → Prop
  | crash (s : State A) (a : A) : Step family s (crash s a)
  | advance (s : State A) (a : A) (old next : Nat)
      (ready : s.online a = some old) (monotone : old ≤ next) : Step family s (onlineAt s a next)
  | respond (s : State A) (r : Response A)
      (ready : s.online r.sender = some r.bound) (other : r.sender ≠ r.recipient)
      (causal : r.generation ≤ s.generation r.recipient) : Step family s (respond s r)
  | recover (s : State A) (a : A) (evidence : List (Response A)) (chosen : Response A)
      (recovering : s.online a = none) (member : chosen ∈ evidence)
      (quorum : family (fun b => ∃ r ∈ evidence, r.sender = b))
      (valid : ∀ r ∈ evidence, r ∈ s.responses ∧ r.recipient = a ∧ r.generation = s.generation a)
      (maximum : ∀ r ∈ evidence, r.bound ≤ chosen.bound) : Step family s (onlineAt s a chosen.bound)

inductive Run {A : Type} (family : QSys A) : State A → State A → Prop
  | refl (s) : Run family s s
  | step {s t u} : Run family s t → Step family t u → Run family s u

/-- Every member of a fixed support quorum either retains the fence or is
excluded while recovering. Replies eligible for a recovering support member
preserve that fence when their sender also belongs to the support quorum. -/
structure Inv {A : Type} (support : NSet A) (bound : Nat) (s : State A) : Prop where
  retained : ∀ a, support a → ∀ known, s.online a = some known → bound ≤ known
  causal : ∀ r ∈ s.responses, r.generation ≤ s.generation r.recipient
  evidence : ∀ r ∈ s.responses, support r.sender → support r.recipient →
    s.online r.recipient = none → r.generation = s.generation r.recipient → bound ≤ r.bound

theorem initial_inv {A : Type} (support : NSet A) (bound : Nat) (bounds : A → Nat)
    (h : ∀ a, support a → bound ≤ bounds a) : Inv support bound (initial bounds) := by
  constructor
  · intro a ha known hk
    have he : bounds a = known := Option.some.inj hk
    exact he ▸ h a ha
  · simp [initial]
  · simp [initial]

/-- An arbitrary checkpoint can contain delayed earlier responses. If the
support quorum is currently online above the fence, none is eligible for an
offline support recipient until that recipient starts a fresh episode. -/
theorem checkpoint_inv {A : Type} (support : NSet A) (bound : Nat) (s : State A)
    (ready : ∀ a, support a → ∃ known, s.online a = some known ∧ bound ≤ known)
    (causal : ∀ r ∈ s.responses, r.generation ≤ s.generation r.recipient) : Inv support bound s := by
  constructor
  · intro a ha known hk
    obtain ⟨old, ho, hb⟩ := ready a ha
    have he := Option.some.inj (ho.symm.trans hk)
    exact he ▸ hb
  · exact causal
  · intro r hr hs ht hoff fresh
    obtain ⟨known, hk, _⟩ := ready r.recipient ht
    rw [hk] at hoff; cases hoff

theorem crash_inv {A : Type} {support : NSet A} {bound : Nat} {s : State A}
    (h : Inv support bound s) (a : A) : Inv support bound (crash s a) := by
  constructor
  · intro b hb known hk
    by_cases he : b = a
    · subst b; simp [crash] at hk
    · exact h.retained b hb known (by simpa [crash, NormalLog.put_other _ a b _ he] using hk)
  · intro r hr
    have hc := h.causal r hr
    by_cases he : r.recipient = a
    · rw [he] at hc
      simp only [crash, he, NormalLog.put_same]; omega
    · simpa [crash, NormalLog.put_other _ a r.recipient _ he] using hc
  · intro r hr hs ht hoff fresh
    by_cases he : r.recipient = a
    · have hc := h.causal r hr
      simp only [crash, he, NormalLog.put_same] at fresh
      rw [he] at hc; omega
    · exact h.evidence r hr hs ht
        (by simpa [crash, NormalLog.put_other _ a r.recipient _ he] using hoff)
        (by simpa [crash, NormalLog.put_other _ a r.recipient _ he] using fresh)

theorem online_inv {A : Type} {support : NSet A} {bound : Nat} {s : State A}
    (h : Inv support bound s) (a : A) (known : Nat) (safe : support a → bound ≤ known) :
    Inv support bound (onlineAt s a known) := by
  constructor
  · intro b hb value hv
    by_cases he : b = a
    · subst b
      have eq : known = value := by simpa [onlineAt] using hv
      exact eq ▸ safe hb
    · exact h.retained b hb value (by simpa [onlineAt, NormalLog.put_other _ a b _ he] using hv)
  · exact h.causal
  · intro r hr hs ht hoff fresh
    by_cases he : r.recipient = a
    · simp [onlineAt, he] at hoff
    · exact h.evidence r hr hs ht
        (by simpa [onlineAt, NormalLog.put_other _ a r.recipient _ he] using hoff) fresh

theorem respond_inv {A : Type} {support : NSet A} {bound : Nat} {s : State A}
    (h : Inv support bound s) (r : Response A) (ready : s.online r.sender = some r.bound)
    (causal : r.generation ≤ s.generation r.recipient) : Inv support bound (respond s r) := by
  constructor
  · exact h.retained
  · intro m hm
    rcases List.mem_cons.mp hm with he | hm
    · subst m; exact causal
    · exact h.causal m hm
  · intro m hm hs ht hoff fresh
    rcases List.mem_cons.mp hm with he | hm
    · subst m; exact h.retained r.sender hs r.bound ready
    · exact h.evidence m hm hs ht hoff fresh

/-- Historical replies count even if their sender has since crashed. Fresh
recipient episodes, rather than current sender status, exclude stale evidence. -/
theorem recovery_bound {A : Type} {family : QSys A} {support : NSet A} {bound : Nat} {s : State A}
    (h : Inv support bound s) (overlap : Frown family family) (supported : family support)
    (a : A) (evidence : List (Response A)) (chosen : Response A)
    (recovering : s.online a = none)
    (quorum : family (fun b => ∃ r ∈ evidence, r.sender = b))
    (valid : ∀ r ∈ evidence, r ∈ s.responses ∧ r.recipient = a ∧ r.generation = s.generation a)
    (maximum : ∀ r ∈ evidence, r.bound ≤ chosen.bound) (ha : support a) : bound ≤ chosen.bound := by
  obtain ⟨b, hb, r, hr, he⟩ := overlap support _ supported quorum
  obtain ⟨hm, ht, hf⟩ := valid r hr
  have hp := h.evidence r hm (he ▸ hb) (ht ▸ ha) (ht ▸ recovering) (ht ▸ hf)
  exact Nat.le_trans hp (maximum r hr)

theorem step_inv {A : Type} {family : QSys A} {support : NSet A} {bound : Nat} {s t : State A}
    (h : Inv support bound s) (overlap : Frown family family) (supported : family support)
    (st : Step family s t) : Inv support bound t := by
  cases st with
  | crash a => exact crash_inv h a
  | advance a old next ready monotone =>
    exact online_inv h a next (fun ha => Nat.le_trans (h.retained a ha old ready) monotone)
  | respond r ready other causal => exact respond_inv h r ready causal
  | recover a evidence chosen recovering member quorum valid maximum =>
    exact online_inv h a chosen.bound
      (recovery_bound h overlap supported a evidence chosen recovering quorum valid maximum)

theorem run_inv {A : Type} {family : QSys A} {support : NSet A} {bound : Nat} {s t : State A}
    (h : Inv support bound s) (overlap : Frown family family) (supported : family support)
    (run : Run family s t) : Inv support bound t := by
  induction run with
  | refl => exact h
  | step _ st ih => exact step_inv ih overlap supported st

/-- Replicated knowledge persists through any finite number of crashes and
recoveries; the bound is erased locally on every crash, never assumed durable. -/
theorem replicated_fence {A : Type} {family : QSys A} {support : NSet A} {bound : Nat} {s t : State A}
    (overlap : Frown family family) (supported : family support)
    (ready : ∀ a, support a → ∃ known, s.online a = some known ∧ bound ≤ known)
    (causal : ∀ r ∈ s.responses, r.generation ≤ s.generation r.recipient)
    (run : Run family s t) :
    ∀ a, support a → ∀ known, t.online a = some known → bound ≤ known :=
  (run_inv (checkpoint_inv support bound s ready causal) overlap supported run).retained

namespace Example

def family : QSys Nat := fun q => (q 0 ∧ q 1) ∨ (q 0 ∧ q 2) ∨ (q 1 ∧ q 2)
def support : NSet Nat := fun a => a = 0 ∨ a = 1

theorem overlap : Frown family family := by
  intro q r hq hr
  rcases hq with hq | hq | hq <;> rcases hr with hr | hr | hr
  · exact ⟨0, hq.1, hr.1⟩
  · exact ⟨0, hq.1, hr.1⟩
  · exact ⟨1, hq.2, hr.1⟩
  · exact ⟨0, hq.1, hr.1⟩
  · exact ⟨0, hq.1, hr.1⟩
  · exact ⟨2, hq.2, hr.2⟩
  · exact ⟨1, hq.1, hr.2⟩
  · exact ⟨2, hq.2, hr.2⟩
  · exact ⟨1, hq.1, hr.1⟩

theorem supported : family support := Or.inl ⟨Or.inl rfl, Or.inr rfl⟩

def oldOne : Response Nat := ⟨1, 0, 0, 0⟩
def oldTwo : Response Nat := ⟨2, 0, 0, 0⟩
def freshOne : Response Nat := ⟨1, 0, 1, 1⟩
def freshTwo : Response Nat := ⟨2, 0, 1, 0⟩
def earlier : State Nat := respond (respond (initial (fun _ => 0)) oldOne) oldTwo
noncomputable def checkpoint : State Nat := onlineAt (onlineAt earlier 0 1) 1 1
noncomputable def crashed : State Nat := crash checkpoint 0
noncomputable def answered : State Nat := respond (respond crashed freshOne) freshTwo
noncomputable def recovered : State Nat := onlineAt answered 0 1

theorem checkpoint_history : Run family (initial (fun _ => 0)) checkpoint := by
  have h1 := Run.step (Run.refl (initial (fun (_ : Nat) => 0)))
    (Step.respond (family := family) _ oldOne rfl (by decide) (by decide))
  have h2 := Run.step h1 (Step.respond _ oldTwo rfl (by decide) (by decide))
  have h3 := Run.step h2 (Step.advance _ 0 0 1 rfl (by decide))
  exact Run.step h3 (Step.advance _ 1 0 1 (by simp [onlineAt, respond, initial, NormalLog.put]) (by decide))

theorem checkpoint_safe : Inv support 1 checkpoint := by
  apply checkpoint_inv
  · intro a ha
    rcases ha with rfl | rfl <;> exact ⟨1, by simp [checkpoint, onlineAt, NormalLog.put], Nat.le_refl _⟩
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r; exact Nat.le_refl _
    · have he := List.mem_singleton.mp hr; subst r; exact Nat.le_refl _

theorem fresh_quorum : family (fun b => ∃ r ∈ [freshOne, freshTwo], r.sender = b) :=
  Or.inr (Or.inr ⟨⟨freshOne, List.mem_cons_self, rfl⟩,
    ⟨freshTwo, List.mem_cons_of_mem _ List.mem_cons_self, rfl⟩⟩)

theorem recovery_run : Run family checkpoint recovered := by
  have h0 : Run family checkpoint crashed := .step (.refl _) (.crash _ 0)
  have h1 := Run.step h0 (Step.respond _ freshOne
    (by simp [crashed, crash, checkpoint, onlineAt, freshOne, NormalLog.put])
    (by decide) (by simp [crashed, crash, freshOne]))
  have h2 : Run family checkpoint answered := Run.step h1 (Step.respond _ freshTwo
    (by simp [respond, crashed, crash, checkpoint, onlineAt, earlier, initial,
      freshTwo, NormalLog.put])
    (by decide) (by simp [respond, crashed, crash, freshTwo]))
  apply Run.step h2 (Step.recover answered 0 [freshOne, freshTwo] freshOne
    (by simp [answered, respond, crashed, crash]) List.mem_cons_self fresh_quorum ?_ ?_)
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r
      exact ⟨List.mem_cons_of_mem _ List.mem_cons_self, rfl, by simp [answered, respond, crashed, crash, freshOne, checkpoint, onlineAt, earlier, initial]⟩
    · have he := List.mem_singleton.mp hr; subst r
      exact ⟨List.mem_cons_self, rfl, by simp [answered, respond, crashed, crash, freshTwo, checkpoint, onlineAt, earlier, initial]⟩
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r; exact Nat.le_refl _
    · have he := List.mem_singleton.mp hr; subst r; decide

/-- A real recovery restores the erased bound using fresh historical replies. -/
theorem recovery_preserves_fence : Inv support 1 recovered ∧ recovered.online 0 = some 1 :=
  ⟨run_inv checkpoint_safe overlap supported recovery_run, by simp [recovered, onlineAt]⟩

/-- Reusing the earlier episode's two authentic replies would restore zero.
They have distinct senders and the right recipient; only freshness rejects them. -/
theorem stale_quorum_breaks_fence :
    Inv support 1 crashed ∧
    family (fun b => ∃ r ∈ [oldOne, oldTwo], r.sender = b) ∧
    (∀ r ∈ [oldOne, oldTwo], r ∈ crashed.responses ∧ r.recipient = 0 ∧ r.bound = 0) ∧
    oldOne.generation ≠ crashed.generation 0 ∧
    ¬ Inv support 1 (onlineAt crashed 0 0) := by
  refine ⟨crash_inv checkpoint_safe 0, ?_, ?_, ?_, ?_⟩
  · exact Or.inr (Or.inr ⟨⟨oldOne, List.mem_cons_self, rfl⟩,
      ⟨oldTwo, List.mem_cons_of_mem _ List.mem_cons_self, rfl⟩⟩)
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r; exact ⟨List.mem_cons_of_mem _ List.mem_cons_self, rfl, rfl⟩
    · have he := List.mem_singleton.mp hr; subst r; exact ⟨List.mem_cons_self, rfl, rfl⟩
  · simp [oldOne, crashed, crash, checkpoint, onlineAt, earlier, respond, initial]
  · intro h
    have hb := h.retained 0 (Or.inl rfl) 0 (by simp [onlineAt])
    omega

end Example
end RestartFence
```

```bash
lake env lean UVRR/RestartFence.lean
```

```output
```

```bash
printf 'import UVRR.RestartFence\n#print axioms RestartFence.run_inv\n#print axioms RestartFence.replicated_fence\n#print axioms RestartFence.Example.checkpoint_history\n#print axioms RestartFence.Example.recovery_run\n#print axioms RestartFence.Example.recovery_preserves_fence\n#print axioms RestartFence.Example.stale_quorum_breaks_fence\n' | lake env lean --stdin
```

```output
'RestartFence.run_inv' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartFence.replicated_fence' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartFence.Example.checkpoint_history' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartFence.Example.recovery_run' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartFence.Example.recovery_preserves_fence' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartFence.Example.stale_quorum_breaks_fence' depends on axioms: [propext, Classical.choice, Quot.sound]
```
