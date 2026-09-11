# Rung 19: Temporal acquisition under explicit restart provenance

*2026-09-06T08:51:50Z by Showboat 0.6.1*
<!-- showboat-id: 5722d9e8-f5d9-496b-9546-5bc35cf820ec -->

Checks the secondary induction in Michael et al. (2017), Theorem 3. Forward recovery response ordering plus quorum intersection propagates a generic stable property from asynchronous witness events. Crash consistency yields that ordering conditional on retention of incarnation knowledge by recovery participants. This remaining outer-induction premise and the local transition provenance are explicit. No end-to-end recovery theorem or production repair is claimed. A countermodel satisfies the full origin contract except forward ordering and loses knowledge.

```bash
cat UVRR/AcquisitionOrder.lean
```

```output
import UVRR.CrashVector

/-! Temporal acquisition reduction for Michael et al. (2017), Theorem 3.
This checks the secondary induction, not the whole recovery protocol.
The remaining premise is retention of incarnation knowledge by participants
in earlier recovery certificates. It is stated explicitly in `no_backwards`;
no transition guard tests global value safety.
-/
namespace AcquisitionOrder

/-- The collector payload records a ghost send-event index. -/
abbrev Reply (A : Type) := CrashVector.Reply A Nat

abbrev Consistent {A : Type} (rs : List (Reply A)) : Prop := CrashVector.Consistent rs

/-- If a recovery's participants retain its incarnation knowledge at later
reply events, a consistent earlier-incarnation certificate forces every
intersection participant's recovery response to follow its certificate reply.
The retention premise must come from the outer recovery induction. -/
theorem no_backwards {A : Type} (rs : List (Reply A)) (consistent : Consistent rs)
    (q r : Reply A) (hq : q ∈ rs) (hr : r ∈ rs)
    (recoveryReply : Reply A)
    (newGeneration : Nat) (newer : q.vector q.sender < newGeneration)
    (retention : recoveryReply.value ≤ r.value →
      newGeneration ≤ r.vector q.sender) :
    r.value < recoveryReply.value := by
  by_cases order : recoveryReply.value ≤ r.value
  · have lower := retention order
    have upper := consistent r hr q hq
    omega
  · omega

/-- Knowledge is a generic stable property: this can be instantiated for a
value or an incarnation lower bound. Times denote events, not wall clocks. -/
structure History (A : Type) where
  online : Nat → A → Prop
  knows : Nat → A → Prop

/-- Local recovery provenance, stated for one historical acquisition.
An online participant after its witness either retains that witness's
knowledge, or inherits knowledge from an earlier quorum of response events.
Each response is strictly earlier than the observation, making induction
well founded. `afterWitness` is the ordering obligation discharged using
`no_backwards` once incarnation retention is proved.

These are theorem premises about a history, not executable refusal guards.
The full protocol must derive them from its transitions. -/
structure Provenance {A : Type} (h : History A) (family : QSys A)
    (support : NSet A) (witness : A → Nat) : Prop where
  atWitness : ∀ a, support a → h.knows (witness a) a
  origin : ∀ t a, support a → witness a ≤ t → h.online t a →
    (h.knows (witness a) a → h.knows t a) ∨
    ∃ (responders : NSet A) (sent : A → Nat), family responders ∧
      (∀ b, responders b → sent b < t ∧ h.online (sent b) b) ∧
      (∀ b, responders b → support b → witness b ≤ sent b) ∧
      (∀ b, responders b → h.knows (sent b) b → h.knows t a)

/-- Asynchronous acquisition does not need all participants simultaneously
online at the witness events. Quorum overlap and forward causal recovery
edges suffice for the secondary induction. This is conditional on Provenance;
it is not yet a theorem about the Rust implementation or crash-vector protocol. -/
theorem acquisition {A : Type} (h : History A) (family : QSys A)
    (support : NSet A) (witness : A → Nat)
    (overlap : Frown family family) (quorum : family support)
    (p : Provenance h family support witness) :
    ∀ t a, support a → witness a ≤ t → h.online t a → h.knows t a := by
  intro t
  induction t using Nat.strongRecOn with
  | ind t ih =>
    intro a ha started ready
    rcases p.origin t a ha started ready with retained | ⟨responders, sent, hq, past, ordered, transfer⟩
    · exact retained (p.atWitness a ha)
    · obtain ⟨b, hb, hr⟩ := overlap support responders quorum hq
      obtain ⟨earlier, online⟩ := past b hr
      exact transfer b hr (ih (sent b) earlier b hb (ordered b hr hb) online)

namespace Example

def family : QSys Unit := fun q => q ()
def support : NSet Unit := fun _ => True
def witness : Unit → Nat := fun _ => 1

def forgotten : History Unit where
  online := fun _ _ => True
  knows := fun t _ => t = 1

/-- All replies can be authentic, earlier, online, and from an intersecting
quorum while still carrying knowledge from before the witness. The ordering
premise rules out precisely this countermodel of the secondary induction. -/
theorem backward_recovery_loses_knowledge :
    family support ∧ forgotten.knows (witness ()) () ∧
    forgotten.online 2 () ∧ ¬ forgotten.knows 2 () ∧
    (0 < 2 ∧ forgotten.online 0 ()) ∧
    (forgotten.knows 0 () → forgotten.knows 2 ()) := by
  simp [family, support, forgotten, witness]

/-- The entire origin contract except forward ordering holds in the
countermodel, not merely one hand-picked recovery event. -/
theorem backward_origin : ∀ t a, support a → witness a ≤ t → forgotten.online t a →
    (forgotten.knows (witness a) a → forgotten.knows t a) ∨
    ∃ (responders : NSet Unit) (sent : Unit → Nat), family responders ∧
      (∀ b, responders b → sent b < t ∧ forgotten.online (sent b) b) ∧
      (∀ b, responders b → forgotten.knows (sent b) b → forgotten.knows t a) := by
  intro t a _ started _
  by_cases eq : t = 1
  · left; simp [forgotten, eq]
  · right
    refine ⟨support, fun _ => 0, trivial, ?_, ?_⟩
    · intro b _
      constructor
      · change 0 < t; change 1 ≤ t at started; omega
      · trivial
    · intro b _ impossible
      change 0 = 1 at impossible
      omega

end Example
end AcquisitionOrder
```

```bash
lake env lean UVRR/AcquisitionOrder.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.AcquisitionOrder
#print axioms AcquisitionOrder.no_backwards
#print axioms AcquisitionOrder.acquisition
#print axioms AcquisitionOrder.Example.backward_origin
#print axioms AcquisitionOrder.Example.backward_recovery_loses_knowledge
LEAN
```

```output
'AcquisitionOrder.no_backwards' depends on axioms: [propext, Quot.sound]
'AcquisitionOrder.acquisition' does not depend on any axioms
'AcquisitionOrder.Example.backward_origin' depends on axioms: [propext, Classical.choice, Quot.sound]
'AcquisitionOrder.Example.backward_recovery_loses_knowledge' depends on axioms: [propext]
```
