import UVRR.Synod

/-! Crash-vector reply filtering, following Michael, Ports, Sharma and Szekeres,
UW-CSE-17-08-01 (2017), Algorithm 1 and Definition 6. This module checks the
collection primitive. It does not assume that filtering alone proves durable
quorum knowledge: recovery must itself acquire and propagate its new incarnation,
and value/knowledge reconstruction and protocol integration remain obligations.
-/
namespace CrashVector

abbrev Vector (A : Type) := A → Nat

def join {A : Type} (v w : Vector A) : Vector A := fun a => max (v a) (w a)

structure Reply (A X : Type) where
  sender : A
  vector : Vector A
  value : X

structure State (A X : Type) where
  known : Vector A
  replies : List (Reply A X)

def initial {A X : Type} : State A X := ⟨fun _ => 0, []⟩

def prune {A X : Type} (known : Vector A) (replies : List (Reply A X)) : List (Reply A X) :=
  replies.filter (fun r => decide (known r.sender ≤ r.vector r.sender))

/-- Sender identity is a set predicate: duplicate replies never add quorum
identities. Payload selection and per-request freshness are separate layers. -/
def senders {A X : Type} (replies : List (Reply A X)) : NSet A :=
  fun a => ∃ r ∈ replies, r.sender = a

def receive {A X : Type} (s : State A X) (r : Reply A X) : State A X :=
  let known := join s.known r.vector
  ⟨known, prune known (r :: s.replies)⟩

def Bounded {A X : Type} (s : State A X) : Prop :=
  ∀ r ∈ s.replies, ∀ a, r.vector a ≤ s.known a

def Consistent {A X : Type} (replies : List (Reply A X)) : Prop :=
  ∀ r ∈ replies, ∀ q ∈ replies, r.vector q.sender ≤ q.vector q.sender

theorem prune_member {A X : Type} (known : Vector A) (replies : List (Reply A X))
    (r : Reply A X) : r ∈ prune known replies ↔ r ∈ replies ∧ known r.sender ≤ r.vector r.sender := by
  simp [prune]

theorem initial_bounded {A X : Type} : Bounded (initial (A := A) (X := X)) := by
  simp [Bounded, initial]

theorem receive_grows {A X : Type} (s : State A X) (r : Reply A X) (a : A) :
    s.known a ≤ (receive s r).known a := Nat.le_max_left _ _

theorem receive_bounded {A X : Type} {s : State A X} (h : Bounded s) (r : Reply A X) :
    Bounded (receive s r) := by
  intro q hq a
  have hm := ((prune_member _ _ q).mp hq).1
  rcases List.mem_cons.mp hm with he | hm
  · subst q; exact Nat.le_max_right _ _
  · exact Nat.le_trans (h q hm a) (Nat.le_max_left _ _)

theorem receive_consistent {A X : Type} {s : State A X} (h : Bounded s) (r : Reply A X) :
    Consistent (receive s r).replies := by
  intro q hq p hp
  have upper := receive_bounded h r q hq p.sender
  have lower := ((prune_member _ _ p).mp hp).2
  exact Nat.le_trans upper lower

theorem current_sender_generation {A X : Type} {s : State A X} (h : Bounded s)
    (r q : Reply A X) (hq : q ∈ (receive s r).replies) :
    q.vector q.sender = (receive s r).known q.sender := by
  apply Nat.le_antisymm (receive_bounded h r q hq q.sender)
  exact ((prune_member _ _ q).mp hq).2

inductive Reachable {A X : Type} : State A X → Prop
  | init : Reachable initial
  | receive {s} : Reachable s → ∀ r, Reachable (receive s r)

theorem reachable_bounded {A X : Type} {s : State A X} (h : Reachable s) : Bounded s := by
  induction h with
  | init => exact initial_bounded
  | receive _ r ih => exact receive_bounded ih r

theorem reachable_consistent {A X : Type} {s : State A X} (h : Reachable s) : Consistent s.replies := by
  cases h with
  | init => simp [Consistent, initial]
  | receive hr r => exact receive_consistent (reachable_bounded hr) r

/-- Once a later incarnation is known, an old reply cannot regain eligibility
while knowledge grows, even if that old reply is delivered repeatedly. -/
theorem old_reply_rejected {A X : Type} (known later : Vector A) (r : Reply A X)
    (old : r.vector r.sender < known r.sender) (grows : ∀ a, known a ≤ later a)
    (replies : List (Reply A X)) : r ∉ prune later replies := by
  intro hm
  have ht := ((prune_member _ _ r).mp hm).2
  have hg := grows r.sender
  omega

/-- The filter retains every reply whose sender incarnation remains current.
No global protocol safety predicate is consulted by the primitive. -/
theorem current_reply_retained {A X : Type} (known : Vector A) (replies : List (Reply A X))
    (r : Reply A X) (hr : r ∈ replies) (current : known r.sender = r.vector r.sender) :
    r ∈ prune known replies :=
  (prune_member known replies r).mpr ⟨hr, by omega⟩

namespace Example

def old : Reply Nat Unit := ⟨1, fun _ => 0, ()⟩
def observer : Reply Nat Unit := ⟨2, fun a => if a = 1 then 1 else 0, ()⟩
def replacement : Reply Nat Unit := ⟨1, observer.vector, ()⟩
def filtered : State Nat Unit := receive (receive initial old) observer

def twoOfThree (replies : List (Reply Nat Unit)) : Bool :=
  let ids := replies.map Reply.sender
  (ids.contains 0 && ids.contains 1) || (ids.contains 0 && ids.contains 2) ||
    (ids.contains 1 && ids.contains 2)

theorem filtered_reachable : Reachable filtered := .receive (.receive .init old) observer

/-- Distinct identities alone admit an incompatible pair of incarnations. -/
theorem unfiltered_quorum_inconsistent :
    twoOfThree [observer, old] = true ∧ ¬ Consistent [observer, old] := by
  refine ⟨rfl, ?_⟩
  intro h
  have bad := h observer List.mem_cons_self old (List.mem_cons_of_mem _ List.mem_cons_self)
  change 1 ≤ 0 at bad
  omega

/-- Filtering rejects the old reply, and a current reply restores a quorum.
This is progress of the collector under supplied replies, not a network
liveness theorem or proof that a recovering replica can obtain those replies. -/
theorem rejects_old_then_accepts_current :
    filtered.replies = [observer] ∧ twoOfThree filtered.replies = false ∧
    twoOfThree (receive filtered replacement).replies = true ∧
    Consistent (receive filtered replacement).replies :=
  ⟨rfl, rfl, rfl, receive_consistent (reachable_bounded filtered_reachable) replacement⟩

end Example
end CrashVector
