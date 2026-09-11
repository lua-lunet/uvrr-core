# Rung 20: Operational crash-vector acquisition certificates

*2026-09-06T08:58:58Z by Showboat 0.6.1*
<!-- showboat-id: 3bb19e31-ba24-49c2-9dfa-06cf7f4462b9 -->

Crash, start, emit, answer, collect and finish transitions derive authentic crash-consistent quorum certificates for exact request/incarnation identities. Historical replies originate at operational senders. Crash erases protocol knowledge; a concrete three-node trace completes recovery at incarnation 1. An authentic same-incarnation older-request reply violates freshness when replayed into a new acquisition. Echoed request identity is explicit; mapping it to the concrete nonce contract is open. This is acquisition mechanics, not persistence, a full recovery proof, liveness or a production repair.

```bash
cat UVRR/RestartAcquire.lean
```

```output
import UVRR.CrashVector
import UVRR.NormalLog

/-! Operational crash-vector acquisition, following Michael et al. (2017),
Algorithm 1. This is a safety projection with one active acquisition per node.
Request identity is an explicit echoed (incarnation, sequence) pair, in addition
to the response vector check. Mapping this to the wire/host nonce contract is
a refinement obligation; this is not a line-for-line encoding of the paper.
Messages are immutable authenticated history; emit can retransmit the current
request after pruning. Crash erases all protocol knowledge and starts a fresh
logical incarnation (the host freshness obligation, not a disk write).
Payload knowledge is ghost set membership, not a prescribed storage format.
ReadQuorum, liveness, reconstruction fidelity and log integration remain open.
-/
namespace RestartAcquire

structure Payload (A X : Type) where
  recipient : A
  generation : Nat
  sequence : Nat
  knowledge : NSet X

abbrev Reply (A X : Type) := CrashVector.Reply A (Payload A X)

structure Request (A X : Type) where
  sender : A
  vector : CrashVector.Vector A
  sequence : Nat
  value : Option X

structure Node (A X : Type) where
  online : Bool
  generation : Nat
  sequence : Nat
  active : Bool
  value : Option X
  collector : CrashVector.State A (Payload A X)
  knowledge : NSet X

structure Certificate (A X : Type) where
  owner : A
  generation : Nat
  sequence : Nat
  replies : List (Reply A X)

structure State (A X : Type) where
  nodes : A → Node A X
  requests : List (Request A X)
  responses : List (Reply A X)
  certificates : List (Certificate A X)

def initialNode {A X : Type} : Node A X :=
  ⟨true, 0, 0, false, none, CrashVector.initial, fun _ => False⟩

def initial {A X : Type} : State A X := ⟨fun _ => initialNode, [], [], []⟩

noncomputable def put {A X : Type} (s : State A X) (a : A) (n : Node A X) : State A X :=
  {s with nodes := NormalLog.put s.nodes a n}

noncomputable def crashed {A X : Type} (a : A) (n : Node A X) : Node A X :=
  {(initialNode (A := A) (X := X)) with
    online := false, generation := n.generation + 1,
    collector := ⟨NormalLog.put (fun _ => 0) a (n.generation + 1), []⟩}

def started {A X : Type} (n : Node A X) (value : Option X) : Node A X :=
  {n with
    sequence := n.sequence + 1, active := true, value := value,
    collector := ⟨n.collector.known, []⟩}

def request {A X : Type} (a : A) (n : Node A X) : Request A X :=
  ⟨a, n.collector.known, n.sequence, n.value⟩

def learned {X : Type} (known : NSet X) (value : Option X) : NSet X :=
  fun x => known x ∨ value = some x

def answered {A X : Type} (n : Node A X) (m : Request A X) : Node A X :=
  {n with
    collector := {n.collector with known := CrashVector.join n.collector.known m.vector},
    knowledge := learned n.knowledge m.value}

def response {A X : Type} (a : A) (n : Node A X) (m : Request A X) : Reply A X :=
  ⟨a, (answered n m).collector.known,
    ⟨m.sender, m.vector m.sender, m.sequence, (answered n m).knowledge⟩⟩

def collected {A X : Type} (n : Node A X) (r : Reply A X) : Node A X :=
  {n with collector := CrashVector.receive n.collector r}

def certificate {A X : Type} (a : A) (n : Node A X) : Certificate A X :=
  ⟨a, n.generation, n.sequence, n.collector.replies⟩

def finished {A X : Type} (n : Node A X) : Node A X :=
  {n with
    online := true, active := false,
    knowledge := fun x => n.knowledge x ∨ ∃ r ∈ n.collector.replies, r.value.knowledge x}

inductive Step {A X : Type} (family : QSys A) : State A X → State A X → Prop
  | crash (s) (a) : Step family s (put s a (crashed a (s.nodes a)))
  | start (s) (a) (value)
      (idle : (s.nodes a).active = false)
      (allowed : (s.nodes a).online = true ∨ value = none) :
      Step family s (put s a (started (s.nodes a) value))
  | emit (s) (a) (active : (s.nodes a).active = true) :
      Step family s {s with requests := request a (s.nodes a) :: s.requests}
  | answer (s) (a) (m) (authentic : m ∈ s.requests)
      (online : (s.nodes a).online = true) :
      Step family s {put s a (answered (s.nodes a) m) with
        responses := response a (s.nodes a) m :: s.responses}
  | collect (s) (a) (r) (authentic : r ∈ s.responses)
      (active : (s.nodes a).active = true)
      (recipient : r.value.recipient = a)
      (generation : r.value.generation = (s.nodes a).generation)
      (sequence : r.value.sequence = (s.nodes a).sequence)
      (vector : r.vector a = (s.nodes a).generation) :
      Step family s (put s a (collected (s.nodes a) r))
  | finish (s) (a) (active : (s.nodes a).active = true)
      (quorum : family (CrashVector.senders (s.nodes a).collector.replies)) :
      Step family s {put s a (finished (s.nodes a)) with
        certificates := certificate a (s.nodes a) :: s.certificates}

inductive Reachable {A X : Type} (family : QSys A) : State A X → Prop
  | init : Reachable family initial
  | step {s t} : Reachable family s → Step family s t → Reachable family t

def Fresh {A X : Type} (a : A) (generation sequence : Nat) (r : Reply A X) : Prop :=
  r.value.recipient = a ∧ r.value.generation = generation ∧
  r.value.sequence = sequence ∧ r.vector a = generation

def LocalOK {A X : Type} (responses : List (Reply A X)) (a : A) (n : Node A X) : Prop :=
  CrashVector.Bounded n.collector ∧ CrashVector.Consistent n.collector.replies ∧
  ∀ r ∈ n.collector.replies, r ∈ responses ∧ Fresh a n.generation n.sequence r

def Certified {A X : Type} (family : QSys A) (responses : List (Reply A X))
    (c : Certificate A X) : Prop :=
  family (CrashVector.senders c.replies) ∧ CrashVector.Consistent c.replies ∧
  ∀ r ∈ c.replies, r ∈ responses ∧ Fresh c.owner c.generation c.sequence r

structure Inv {A X : Type} (family : QSys A) (s : State A X) : Prop where
  nodes : ∀ a, LocalOK s.responses a (s.nodes a)
  certificates : ∀ c ∈ s.certificates, Certified family s.responses c

theorem local_mono {A X : Type} {rs more : List (Reply A X)}
    (included : ∀ r ∈ rs, r ∈ more) {a : A} {n : Node A X} (h : LocalOK rs a n) :
    LocalOK more a n :=
  ⟨h.1, h.2.1, fun r hr => ⟨included r (h.2.2 r hr).1, (h.2.2 r hr).2⟩⟩

theorem certified_mono {A X : Type} {family : QSys A} {rs more : List (Reply A X)}
    (included : ∀ r ∈ rs, r ∈ more) {c : Certificate A X} (h : Certified family rs c) :
    Certified family more c :=
  ⟨h.1, h.2.1, fun r hr => ⟨included r (h.2.2 r hr).1, (h.2.2 r hr).2⟩⟩

theorem put_inv {A X : Type} {family : QSys A} {s : State A X}
    (h : Inv family s) (a : A) (n : Node A X) (hn : LocalOK s.responses a n) :
    Inv family (put s a n) := by
  constructor
  · intro b
    by_cases eq : b = a
    · subst b; simpa [put] using hn
    · simpa [put, NormalLog.put_other _ a b _ eq] using h.nodes b
  · exact h.certificates

theorem initial_inv {A X : Type} (family : QSys A) : Inv family (initial (A := A) (X := X)) := by
  constructor
  · intro a
    exact ⟨CrashVector.initial_bounded, by simp [CrashVector.Consistent, initial, initialNode, CrashVector.initial],
      by simp [initial, initialNode, CrashVector.initial]⟩
  · simp [initial]

theorem crash_local {A X : Type} (rs : List (Reply A X)) (a : A) (n : Node A X) :
    LocalOK rs a (crashed a n) := by
  simp [LocalOK, CrashVector.Bounded, CrashVector.Consistent, crashed]

theorem start_local {A X : Type} (rs : List (Reply A X)) (a : A) (n : Node A X) (v : Option X) :
    LocalOK rs a (started n v) := by
  simp [LocalOK, CrashVector.Bounded, CrashVector.Consistent, started]

theorem answer_local {A X : Type} {rs : List (Reply A X)} {a : A} {n : Node A X}
    (h : LocalOK rs a n) (m : Request A X) : LocalOK rs a (answered n m) := by
  refine ⟨?_, h.2.1, h.2.2⟩
  intro r hr b
  exact Nat.le_trans (h.1 r hr b) (Nat.le_max_left _ _)

theorem collect_local {A X : Type} {rs : List (Reply A X)} {a : A} {n : Node A X}
    (h : LocalOK rs a n) (r : Reply A X) (authentic : r ∈ rs)
    (fresh : Fresh a n.generation n.sequence r) : LocalOK rs a (collected n r) := by
  refine ⟨CrashVector.receive_bounded h.1 r, CrashVector.receive_consistent h.1 r, ?_⟩
  intro q hq
  have member := ((CrashVector.prune_member _ _ q).mp hq).1
  rcases List.mem_cons.mp member with eq | old
  · subst q; exact ⟨authentic, fresh⟩
  · exact h.2.2 q old

theorem step_inv {A X : Type} {family : QSys A} {s t : State A X}
    (h : Inv family s) (step : Step family s t) : Inv family t := by
  cases step with
  | crash a => exact put_inv h a _ (crash_local _ _ _)
  | start a value idle allowed => exact put_inv h a _ (start_local _ _ _ _)
  | emit a active => exact ⟨h.nodes, h.certificates⟩
  | answer a m authentic online =>
    have hp := put_inv h a _ (answer_local (h.nodes a) m)
    constructor
    · intro b; exact local_mono (fun r hr => List.mem_cons_of_mem _ hr) (hp.nodes b)
    · intro c hc; exact certified_mono (fun r hr => List.mem_cons_of_mem _ hr) (hp.certificates c hc)
  | collect a r authentic active recipient generation sequence vector =>
    exact put_inv h a _ (collect_local (h.nodes a) r authentic ⟨recipient, generation, sequence, vector⟩)
  | finish a active quorum =>
    have hp := put_inv h a (finished (s.nodes a)) (h.nodes a)
    constructor
    · exact hp.nodes
    · intro c hc
      rcases List.mem_cons.mp hc with eq | old
      · subst c; exact ⟨quorum, (h.nodes a).2⟩
      · exact h.certificates c old

theorem reachable_inv {A X : Type} {family : QSys A} {s : State A X}
    (h : Reachable family s) : Inv family s := by
  induction h with
  | init => exact initial_inv _
  | step _ step ih => exact step_inv ih step

/-- Every completed acquisition, including recovery, has an authentic,
crash-consistent distinct-identity quorum for its exact request and incarnation.
The full persistence theorem is a further obligation, not a finish guard. -/
theorem completed_certificate {A X : Type} {family : QSys A} {s : State A X}
    (h : Reachable family s) (c : Certificate A X) (hc : c ∈ s.certificates) :
    Certified family s.responses c := (reachable_inv h).certificates c hc

/-- Historical responses originate at an operational sender. This is derived
from the answer transition, not assumed of the network history. -/
theorem response_origin {A X : Type} {family : QSys A} {s : State A X}
    (h : Reachable family s) : ∀ r ∈ s.responses,
    ∃ a n m, n.online = true ∧ r = response a n m := by
  induction h with
  | init => simp [initial]
  | step _ step ih =>
    cases step with
    | crash => exact ih
    | start => exact ih
    | emit => exact ih
    | collect => exact ih
    | finish => exact ih
    | answer a m authentic online =>
      intro r hr
      rcases List.mem_cons.mp hr with eq | old
      · exact ⟨a, _, m, online, eq⟩
      · exact ih r old

/-- Rebuilding transfers every retained response's stable knowledge. -/
theorem finish_transfers {A X : Type} (n : Node A X) (r : Reply A X)
    (hr : r ∈ n.collector.replies) (x : X) (known : r.value.knowledge x) :
    (finished n).knowledge x := Or.inr ⟨r, hr, known⟩

namespace Example

def family : QSys Nat := fun q => q 1 ∧ q 2
noncomputable def s0 : State Nat Unit := initial
noncomputable def s1 := put s0 0 (crashed 0 (s0.nodes 0))
noncomputable def s2 := put s1 0 (started (s1.nodes 0) none)
noncomputable def m := request 0 (s2.nodes 0)
noncomputable def s3 : State Nat Unit := {s2 with requests := [m]}
noncomputable def r1 := response 1 (s3.nodes 1) m
noncomputable def s4 : State Nat Unit := {put s3 1 (answered (s3.nodes 1) m) with responses := [r1]}
noncomputable def r2 := response 2 (s4.nodes 2) m
noncomputable def s5 : State Nat Unit := {put s4 2 (answered (s4.nodes 2) m) with responses := [r2, r1]}
noncomputable def s6 := put s5 0 (collected (s5.nodes 0) r1)
noncomputable def s7 := put s6 0 (collected (s6.nodes 0) r2)
noncomputable def cert := certificate 0 (s7.nodes 0)
noncomputable def s8 : State Nat Unit :=
  {put s7 0 (finished (s7.nodes 0)) with certificates := [cert]}

-- Reduction is confined to this concrete witness; the safety induction above
-- quantifies over arbitrary node/value types and unbounded finite executions.
theorem recovery_completes : Reachable family s8 ∧
    (s8.nodes 0).online = true ∧ cert.generation = 1 ∧
    Certified family s8.responses cert := by
  have h1 : Reachable family s1 := .step .init (.crash _ 0)
  have h2 : Reachable family s2 := .step h1 (.start _ 0 none (by simp [s1, put, crashed, initialNode]) (Or.inr rfl))
  have h3 : Reachable family s3 := .step h2 (.emit _ 0 (by simp [s2, put, started]))
  have h4 : Reachable family s4 := .step h3 (.answer _ 1 m List.mem_cons_self
    (by simp [s3, s2, s1, s0, put, initial, initialNode, NormalLog.put]))
  have h5 : Reachable family s5 := .step h4 (.answer _ 2 m List.mem_cons_self
    (by simp [s4, s3, s2, s1, s0, put, initial, initialNode, NormalLog.put]))
  have h6 : Reachable family s6 := by
    apply Reachable.step h5
    apply Step.collect s5 0 r1 (List.mem_cons_of_mem _ List.mem_cons_self)
    all_goals simp [s5, s4, s3, s2, s1, s0, put, CrashVector.join, r1, r2, response, answered, m, request, started, crashed, initial, initialNode, CrashVector.initial, NormalLog.put]
  have h7 : Reachable family s7 := by
    apply Reachable.step h6
    apply Step.collect s6 0 r2 List.mem_cons_self
    all_goals simp [s6, s5, s4, s3, s2, s1, s0, put, collected, CrashVector.receive, CrashVector.prune, CrashVector.join, r1, r2, response, answered, m, request, started, crashed, initial, initialNode, CrashVector.initial, NormalLog.put]
  have h8 : Reachable family s8 := .step h7 (.finish _ 0
    (by simp [s7, s6, collected, s5, s4, s3, s2, put, started, NormalLog.put])
    (by
      simp [family, CrashVector.senders, s7, s6, s5, s4, s3, s2, s1, s0,
        put, collected, CrashVector.receive, CrashVector.prune, CrashVector.join,
        r1, r2, response, answered, m, request, started, crashed, initial,
        initialNode, CrashVector.initial, NormalLog.put]))
  exact ⟨h8, by simp [s8, put, finished], by simp [cert, certificate, s7, s6, s5, s4, s3, s2, s1, s0, put, collected, CrashVector.receive, CrashVector.prune, CrashVector.join, r1, r2, response, answered, m, request, started, crashed, initial, initialNode, CrashVector.initial, NormalLog.put],
    completed_certificate h8 cert List.mem_cons_self⟩

/-- An authentic reply from the same incarnation can still belong to an
older acquisition. Reusing it after starting the next request violates Fresh.
This countermodel isolates sequence freshness from incarnation freshness. -/
theorem stale_request_countermodel :
    r1 ∈ s5.responses ∧
    r1.value.generation = (started (s5.nodes 0) none).generation ∧
    r1.value.sequence ≠ (started (s5.nodes 0) none).sequence ∧
    ¬ LocalOK s5.responses 0 (collected (started (s5.nodes 0) none) r1) := by
  refine ⟨List.mem_cons_of_mem _ List.mem_cons_self, ?_, ?_, ?_⟩
  · simp [r1, response, m, request, s5, s4, s3, s2, s1, s0, put, started,
      crashed, initial, initialNode, NormalLog.put]
  · simp [r1, response, m, request, s5, s4, s3, s2, s1, s0, put, started,
      crashed, initial, initialNode, NormalLog.put]
  · intro h
    have member : r1 ∈ (collected (started (s5.nodes 0) none) r1).collector.replies := by
      simp [collected, CrashVector.receive, CrashVector.prune, CrashVector.join,
        r1, response, answered, m, request, s5, s4, s3, s2, s1, s0, put,
        started, crashed, initial, initialNode, CrashVector.initial, NormalLog.put]
    have eq := (h.2.2 r1 member).2.2.2.1
    simp [collected, r1, response, m, request, s5, s4, s3, s2, s1, s0, put,
      started, crashed, initial, initialNode, NormalLog.put] at eq

end Example
end RestartAcquire
```

```bash
lake env lean UVRR/RestartAcquire.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.RestartAcquire
#print axioms RestartAcquire.completed_certificate
#print axioms RestartAcquire.response_origin
#print axioms RestartAcquire.finish_transfers
#print axioms RestartAcquire.Example.recovery_completes
#print axioms RestartAcquire.Example.stale_request_countermodel
LEAN
```

```output
'RestartAcquire.completed_certificate' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartAcquire.response_origin' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartAcquire.finish_transfers' does not depend on any axioms
'RestartAcquire.Example.recovery_completes' depends on axioms: [propext, Classical.choice, Quot.sound]
'RestartAcquire.Example.stale_request_countermodel' depends on axioms: [propext, Classical.choice, Quot.sound]
```
