# Rung 16: Shared multi-view committed-log compatibility

*2026-09-06T08:16:09Z by Showboat 0.6.1*
<!-- showboat-id: b43d5d1b-716e-44cc-aa0d-027107175ece -->

A shared transition model joins authentic prepare histories to local fencing, report emission and certified view activation. Its inductive invariant discharges same-view comparability and installed-base extension. Strong view induction proves committed-log compatibility and equal-length equality without a global agreement guard. Scope: one quorum family, arbitrary log length and view count, no crashes. Unique source activation is an explicit unused-view guard; concrete primary/wire refinement, recovery, membership changes, clients and progress remain open. A nonempty two-view execution and delayed wrong-view prepare control check the model interface.

```bash
cat UVRR/LogProvenance.lean
```

```output
import UVRR.NormalLog
import UVRR.ViewFence

/-! Shared multi-view message provenance. A view source has one immutable
installation base and an append-only primary history. Source activation
requires authentic quorum reports and the executable maximum-rank selector,
never a global agreement test. Local replicas use ViewFence transitions, receive
only issued next-slot prepares, and emit immutable reports. No crash operation
or first-round recovery knowledge is represented in this projection.
-/
namespace LogProvenance

abbrev Sources (V : Type) := Nat → Option (List V × List V)

def Origin {V : Type} (sources : Sources V) (v : Nat) (log : List V) : Prop :=
  ∃ base tail, sources v = some (base, tail) ∧ base <+: log ∧ log <+: tail

def Grows {V : Type} (s t : Sources V) : Prop :=
  ∀ v base tail, s v = some (base, tail) →
    ∃ tail', t v = some (base, tail') ∧ tail <+: tail'

theorem origin_grows {V : Type} {s t : Sources V} (h : Grows s t)
    {v : Nat} {log : List V} (ho : Origin s v log) : Origin t v log := by
  obtain ⟨base, tail, hs, hb, hl⟩ := ho
  obtain ⟨tail', ht, hp⟩ := h v base tail hs
  exact ⟨base, tail', ht, hb, hl.trans hp⟩

theorem grows_new {V : Type} (s : Sources V) (v : Nat) (base : List V)
    (fresh : s v = none) : Grows s (NormalLog.put s v (some (base, base))) := by
  intro b old tail hb
  by_cases he : b = v
  · subst b; rw [fresh] at hb; cases hb
  · exact ⟨tail, by simpa [NormalLog.put_other _ v b _ he] using hb, List.prefix_rfl⟩

theorem grows_append {V : Type} (s : Sources V) (v : Nat) (base tail : List V)
    (x : V) (hv : s v = some (base, tail)) :
    Grows s (NormalLog.put s v (some (base, tail ++ [x]))) := by
  intro b old previous hb
  by_cases he : b = v
  · subst b
    obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj (hv.symm.trans hb))
    exact ⟨tail ++ [x], NormalLog.put_same _ _ _, List.prefix_append _ _⟩
  · exact ⟨previous, by simpa [NormalLog.put_other _ v b _ he] using hb, List.prefix_rfl⟩

structure NodeOK {V : Type} (sources : Sources V) (s : ViewFence.State V) : Prop where
  reachable : ViewFence.Reachable s
  current : Origin sources s.retained s.log
  votes : ∀ v log, (v, log) ∈ s.votes → Origin sources v log
  replies : ∀ r ∈ s.replies, Origin sources r.retained r.log

theorem node_grows {V : Type} {s t : Sources V} (h : Grows s t)
    {node : ViewFence.State V} (hn : NodeOK s node) : NodeOK t node :=
  ⟨hn.reachable, origin_grows h hn.current,
    fun _ _ hv => origin_grows h (hn.votes _ _ hv),
    fun _ hr => origin_grows h (hn.replies _ hr)⟩

theorem node_install {V : Type} {sources : Sources V} {s : ViewFence.State V}
    (h : NodeOK sources s) (v : Nat) (log : List V) (hf : s.floor ≤ v)
    (hv : s.retained < v) (ho : Origin sources v log) :
    NodeOK sources (ViewFence.install s v log) := by
  constructor
  · exact .step h.reachable (.install s v log hf hv)
  · exact ho
  · intro b l hm
    rcases List.mem_cons.mp hm with he | hm
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj he; exact ho
    · exact h.votes b l hm
  · exact h.replies

theorem node_append {V : Type} {sources : Sources V} {s : ViewFence.State V}
    (h : NodeOK sources s) (x : V) (hn : s.floor = s.retained)
    (ho : Origin sources s.retained (s.log ++ [x])) :
    NodeOK sources (ViewFence.append s x) := by
  constructor
  · exact .step h.reachable (.append s x hn)
  · exact ho
  · intro b l hm
    rcases List.mem_cons.mp hm with he | hm
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj he; exact ho
    · exact h.votes b l hm
  · exact h.replies

theorem node_enter {V : Type} {sources : Sources V} {s : ViewFence.State V}
    (h : NodeOK sources s) (v : Nat) (hv : s.floor < v) :
    NodeOK sources (ViewFence.enter s v) :=
  ⟨.step h.reachable (.enter s v hv), h.current, h.votes, h.replies⟩

theorem node_reply {V : Type} {sources : Sources V} {s : ViewFence.State V}
    (h : NodeOK sources s) (hv : s.retained < s.floor) :
    NodeOK sources (ViewFence.reply s) := by
  constructor
  · exact .step h.reachable (.reply s hv)
  · exact h.current
  · exact h.votes
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r; exact h.current
    · exact h.replies r hr

structure Prepare (V : Type) where
  view : Nat
  before : List V
  entry : V

structure State (A V : Type) where
  sources : Sources V
  nodes : A → ViewFence.State V
  messages : List (Prepare V)

def initial {A V : Type} : State A V :=
  ⟨fun v => if v = 0 then some ([], []) else none, fun _ => ViewFence.initial, []⟩

/-- A view activation is justified by actual local reports from a quorum,
with the executable maximum-rank selector determining its base. -/
def Certified {A V : Type} (family : QSys A) (nodes : A → ViewFence.State V)
    (v : Nat) (base : List V) : Prop :=
  ∃ seed : ViewSelection.Report A V, ∃ rest : List (ViewSelection.Report A V),
    ∃ q : NSet A, family q ∧
    (∀ a, q a → ∃ r ∈ seed :: rest, r.sender = a) ∧
    (∀ r ∈ seed :: rest, (⟨v, r.retained, r.log⟩ : ViewFence.Reply V) ∈ (nodes r.sender).replies) ∧
    base = (ViewSelection.select seed rest).log

def RepliesGrow {A V : Type} (s t : A → ViewFence.State V) : Prop :=
  ∀ a r, r ∈ (s a).replies → r ∈ (t a).replies

theorem certified_grows {A V : Type} {family : QSys A} {s t : A → ViewFence.State V}
    (hg : RepliesGrow s t) {v : Nat} {base : List V} (hc : Certified family s v base) :
    Certified family t v base := by
  obtain ⟨seed, rest, q, hq, cov, auth, he⟩ := hc
  exact ⟨seed, rest, q, hq, cov, fun r hr => hg r.sender _ (auth r hr), he⟩

theorem local_replies_grow {V : Type} {s t : ViewFence.State V} (st : ViewFence.Step s t) :
    ∀ r ∈ s.replies, r ∈ t.replies := by
  cases st with
  | install v log hf hv => exact fun _ hr => hr
  | append x hn => exact fun _ hr => hr
  | enter v hv => exact fun _ hr => hr
  | reply hv => exact fun _ hr => List.mem_cons_of_mem _ hr

theorem put_replies_grow {A V : Type} (nodes : A → ViewFence.State V) (a : A)
    (node : ViewFence.State V) (st : ViewFence.Step (nodes a) node) :
    RepliesGrow nodes (NormalLog.put nodes a node) := by
  intro b r hr
  by_cases he : b = a
  · subst b; simpa using local_replies_grow st r hr
  · simpa [NormalLog.put_other _ a b _ he] using hr

inductive Step {A V : Type} (family : QSys A) : State A V → State A V → Prop
  | create (s : State A V) (v : Nat) (base : List V) (fresh : s.sources v = none) (certified : Certified family s.nodes v base) :
      Step family s {s with sources := NormalLog.put s.sources v (some (base, base))}
  | propose (s : State A V) (v : Nat) (base tail : List V) (x : V)
      (source : s.sources v = some (base, tail)) :
      Step family s {s with
        sources := NormalLog.put s.sources v (some (base, tail ++ [x]))
        messages := ⟨v, tail, x⟩ :: s.messages}
  | install (s : State A V) (a : A) (v : Nat) (base tail : List V)
      (source : s.sources v = some (base, tail))
      (floor : (s.nodes a).floor ≤ v) (higher : (s.nodes a).retained < v) :
      Step family s {s with nodes := NormalLog.put s.nodes a (ViewFence.install (s.nodes a) v base)}
  | receive (s : State A V) (a : A) (m : Prepare V) (sent : m ∈ s.messages)
      (normal : (s.nodes a).floor = (s.nodes a).retained)
      (view : m.view = (s.nodes a).retained)
      (next : m.before.length = (s.nodes a).log.length) :
      Step family s {s with nodes := NormalLog.put s.nodes a (ViewFence.append (s.nodes a) m.entry)}
  | enter (s : State A V) (a : A) (v : Nat) (higher : (s.nodes a).floor < v) :
      Step family s {s with nodes := NormalLog.put s.nodes a (ViewFence.enter (s.nodes a) v)}
  | report (s : State A V) (a : A) (changing : (s.nodes a).retained < (s.nodes a).floor) :
      Step family s {s with nodes := NormalLog.put s.nodes a (ViewFence.reply (s.nodes a))}

inductive Reachable {A V : Type} (family : QSys A) : State A V → Prop
  | init : Reachable family initial
  | step {s t} : Reachable family s → Step family s t → Reachable family t

structure Inv {A V : Type} (family : QSys A) (s : State A V) : Prop where
  well : ∀ v base tail, s.sources v = some (base, tail) → base <+: tail
  nodes : ∀ a, NodeOK s.sources (s.nodes a)
  messages : ∀ m ∈ s.messages, Origin s.sources m.view (m.before ++ [m.entry])
  certified : ∀ v base tail, 0 < v → s.sources v = some (base, tail) → Certified family s.nodes v base

theorem initial_inv {A V : Type} (family : QSys A) : Inv family (initial (A := A) (V := V)) := by
  have ho : Origin (initial (A := A) (V := V)).sources 0 [] :=
    ⟨[], [], by simp [initial], List.prefix_rfl, List.prefix_rfl⟩
  constructor
  · intro v base tail hs
    simp only [initial] at hs
    split at hs
    · obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hs); exact List.prefix_rfl
    · cases hs
  · intro a
    exact ⟨.init, ho, by simp [initial, ViewFence.initial], by simp [initial, ViewFence.initial]⟩
  · simp [initial]
  · intro v base tail hv hs
    simp [initial, Nat.ne_of_gt hv] at hs

theorem put_nodes {A V : Type} {sources : Sources V} {nodes : A → ViewFence.State V}
    (h : ∀ a, NodeOK sources (nodes a)) (a : A) (node : ViewFence.State V)
    (hn : NodeOK sources node) : ∀ b, NodeOK sources (NormalLog.put nodes a node b) := by
  intro b
  by_cases he : b = a
  · subst b; simpa using hn
  · simpa [NormalLog.put_other _ a b _ he] using h b

/-- Wire next-slot equality plus authenticated send history proves content
agreement; the handler does not inspect the ghost pre-send log's contents. -/
theorem receive_origin {A V : Type} {family : QSys A} {s : State A V} (h : Inv family s)
    (a : A) (m : Prepare V) (sent : m ∈ s.messages)
    (view : m.view = (s.nodes a).retained)
    (next : m.before.length = (s.nodes a).log.length) :
    Origin s.sources (s.nodes a).retained ((s.nodes a).log ++ [m.entry]) := by
  obtain ⟨base, tail, hs, hb, hl⟩ := (h.nodes a).current
  obtain ⟨base', tail', hs', _, hm⟩ := h.messages m sent
  rw [view, hs] at hs'
  obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hs')
  have hp : m.before <+: tail := (List.prefix_append _ _).trans hm
  have he : (s.nodes a).log = m.before :=
    (List.prefix_of_prefix_length_le hl hp (by omega)).eq_of_length_le (by omega)
  exact ⟨base, tail, hs, List.prefix_append_of_prefix hb, by simpa [he] using hm⟩

theorem step_inv {A V : Type} {family : QSys A} {s t : State A V}
    (h : Inv family s) (st : Step family s t) : Inv family t := by
  cases st with
  | create v base fresh cert =>
    have grows := grows_new s.sources v base fresh
    constructor
    · intro b old tail hs
      by_cases he : b = v
      · subst b; simp only [NormalLog.put_same, Option.some.injEq, Prod.mk.injEq] at hs
        obtain ⟨rfl, rfl⟩ := hs; exact List.prefix_rfl
      · simp only [NormalLog.put_other _ v b _ he] at hs
        exact h.well b old tail hs
    · intro a; exact node_grows grows (h.nodes a)
    · intro m hm; exact origin_grows grows (h.messages m hm)
    · intro b old tail hb hs
      by_cases he : b = v
      · subst b; simp only [NormalLog.put_same, Option.some.injEq, Prod.mk.injEq] at hs
        obtain ⟨rfl, rfl⟩ := hs; exact cert
      · simp only [NormalLog.put_other _ v b _ he] at hs
        exact h.certified b old tail hb hs
  | propose v base tail x source =>
    have grows := grows_append s.sources v base tail x source
    constructor
    · intro b old last hs
      by_cases he : b = v
      · subst b; simp only [NormalLog.put_same, Option.some.injEq, Prod.mk.injEq] at hs
        obtain ⟨rfl, rfl⟩ := hs
        exact List.prefix_append_of_prefix (h.well v base tail source)
      · simp only [NormalLog.put_other _ v b _ he] at hs
        exact h.well b old last hs
    · intro a; exact node_grows grows (h.nodes a)
    · intro m hm
      rcases List.mem_cons.mp hm with he | hm
      · subst m
        exact ⟨base, tail ++ [x], NormalLog.put_same _ _ _,
          List.prefix_append_of_prefix (h.well v base tail source), List.prefix_rfl⟩
      · exact origin_grows grows (h.messages m hm)
    · intro b old last hb hs
      by_cases he : b = v
      · subst b; simp only [NormalLog.put_same, Option.some.injEq, Prod.mk.injEq] at hs
        obtain ⟨rfl, rfl⟩ := hs; exact h.certified v base tail hb source
      · simp only [NormalLog.put_other _ v b _ he] at hs
        exact h.certified b old last hb hs
  | install a v base tail source hf hv =>
    exact ⟨h.well, put_nodes h.nodes a _ (node_install (h.nodes a) v base hf hv
      ⟨base, tail, source, List.prefix_rfl, h.well v base tail source⟩), h.messages,
      fun b old last hb hs => certified_grows (put_replies_grow s.nodes a _
        (.install _ v base hf hv)) (h.certified b old last hb hs)⟩
  | receive a m sent normal view next =>
    exact ⟨h.well, put_nodes h.nodes a _ (node_append (h.nodes a) m.entry normal
      (receive_origin h a m sent view next)), h.messages,
      fun b old last hb hs => certified_grows (put_replies_grow s.nodes a _
        (.append _ m.entry normal)) (h.certified b old last hb hs)⟩
  | enter a v higher =>
    exact ⟨h.well, put_nodes h.nodes a _ (node_enter (h.nodes a) v higher), h.messages,
      fun b old last hb hs => certified_grows (put_replies_grow s.nodes a _
        (.enter _ v higher)) (h.certified b old last hb hs)⟩
  | report a changing =>
    exact ⟨h.well, put_nodes h.nodes a _ (node_reply (h.nodes a) changing), h.messages,
      fun b old last hb hs => certified_grows (put_replies_grow s.nodes a _
        (.reply _ changing)) (h.certified b old last hb hs)⟩

theorem reachable_inv {A V : Type} {family : QSys A} {s : State A V}
    (h : Reachable family s) : Inv family s := by
  induction h with
  | init => exact initial_inv family
  | step _ st ih => exact step_inv ih st

theorem reports_comparable {A V : Type} {family : QSys A} {s : State A V}
    (h : Reachable family s)
    (a b : A) (r q : ViewFence.Reply V) (hr : r ∈ (s.nodes a).replies)
    (hq : q ∈ (s.nodes b).replies) (same : r.retained = q.retained) :
    r.log <+: q.log ∨ q.log <+: r.log := by
  obtain ⟨base, tail, hs, _, hp⟩ := ((reachable_inv h).nodes a).replies r hr
  obtain ⟨base', tail', hs', _, hp'⟩ := ((reachable_inv h).nodes b).replies q hq
  rw [← same, hs] at hs'
  obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj hs')
  exact List.prefix_or_prefix_of_prefix hp hp'

/-- Strong view induction with every provenance premise discharged by the
shared transition invariant. This fixes the configuration but bounds neither
view numbers nor log length nor the number of activations. -/
theorem later_base_preserves {A V : Type} {family : QSys A} {s : State A V}
    (run : Reachable family s) (overlap : Frown family family)
    (v : Nat) (committedPrefix : List V) (commitQ : NSet A) (hc : family commitQ)
    (votes : ∀ a, commitQ a → ∃ log, (v, log) ∈ (s.nodes a).votes ∧ committedPrefix <+: log) :
    ∀ b base tail, v < b → s.sources b = some (base, tail) → committedPrefix <+: base := by
  have inv := reachable_inv run
  intro b
  induction b using Nat.strongRecOn with
  | ind b ih =>
    intro base tail hvb source
    obtain ⟨seed, rest, q, hq, cov, auth, selected⟩ :=
      inv.certified b base tail (by omega) source
    rw [selected]
    apply ViewFence.selection_preserves s.nodes (fun a => (inv.nodes a).reachable)
      seed rest b v hvb committedPrefix family family overlap commitQ q hc hq cov votes auth
    · intro r hr p hp he
      exact reports_comparable run r.sender p.sender ⟨b, r.retained, r.log⟩
        ⟨b, p.retained, p.log⟩ (auth r hr) (auth p hp) he
    · intro r hr hvr
      have rb : r.retained < b :=
        ((ViewFence.reachable_inv (inv.nodes r.sender).reachable).replies _ (auth r hr)).2.1
      obtain ⟨oldBase, oldTail, hs, hb, _⟩ := (inv.nodes r.sender).replies _ (auth r hr)
      exact (ih r.retained rb oldBase oldTail hvr hs).trans hb

def Committed {A V : Type} (family : QSys A) (s : State A V) (v : Nat) (log : List V) : Prop :=
  ∃ q : NSet A, family q ∧ ∀ a, q a → ∃ voted, (v, voted) ∈ (s.nodes a).votes ∧ log <+: voted

/-- Committed whole logs are compatible in every reachable shared-model state.
Quorums, messages and replica identities are unbounded abstract domains;
nonemptiness follows from the assumed self-overlap of the quorum family. -/
theorem committed_comparable {A V : Type} {family : QSys A} {s : State A V}
    (run : Reachable family s) (overlap : Frown family family)
    (v w : Nat) (p q : List V) (hp : Committed family s v p) (hq : Committed family s w q) :
    p <+: q ∨ q <+: p := by
  obtain ⟨qp, hqp, vp⟩ := hp
  obtain ⟨qq, hqq, vq⟩ := hq
  obtain ⟨a, ha, _⟩ := overlap qp qp hqp hqp
  obtain ⟨b, hb, _⟩ := overlap qq qq hqq hqq
  obtain ⟨lp, hlp, hpp⟩ := vp a ha
  obtain ⟨lq, hlq, hqqp⟩ := vq b hb
  have inv := reachable_inv run
  obtain ⟨bp, tp, sp, hbp, htp⟩ := (inv.nodes a).votes v lp hlp
  obtain ⟨bq, tq, sq, hbq, htq⟩ := (inv.nodes b).votes w lq hlq
  rcases Nat.lt_trichotomy v w with hvw | he | hwv
  · have hbase := later_base_preserves run overlap v p qp hqp vp w bq tq hvw sq
    exact List.prefix_or_prefix_of_prefix (hbase.trans hbq) hqqp
  · subst w
    rw [sp] at sq
    obtain ⟨rfl, rfl⟩ := Prod.mk.inj (Option.some.inj sq)
    exact List.prefix_or_prefix_of_prefix (hpp.trans htp) (hqqp.trans htq)
  · have hbase := later_base_preserves run overlap w q qq hqq vq v bp tp hwv sp
    exact List.prefix_or_prefix_of_prefix hpp (hbase.trans hbp)

/-- Equal-length committed prefixes are equal, the log form of agreement. -/
theorem committed_equal {A V : Type} {family : QSys A} {s : State A V}
    (run : Reachable family s) (overlap : Frown family family)
    (v w : Nat) (p q : List V) (hp : Committed family s v p) (hq : Committed family s w q)
    (length : p.length = q.length) : p = q := by
  rcases committed_comparable run overlap v w p q hp hq with h | h
  · exact h.eq_of_length_le (by omega)
  · exact (h.eq_of_length_le (by omega)).symm

namespace Example

def family : QSys Unit := fun q => q ()
noncomputable def sent : State Unit Bool :=
  {initial with sources := NormalLog.put (initial (A := Unit)).sources 0 (some ([], [true])),
                messages := [⟨0, [], true⟩]}
noncomputable def voted : State Unit Bool :=
  {sent with nodes := NormalLog.put sent.nodes () (ViewFence.append (sent.nodes ()) true)}
noncomputable def delayed : State Unit Bool :=
  {voted with sources := NormalLog.put voted.sources 0 (some ([], [true, false])),
               messages := ⟨0, [true], false⟩ :: voted.messages}
noncomputable def entered : State Unit Bool :=
  {delayed with nodes := NormalLog.put delayed.nodes () (ViewFence.enter (delayed.nodes ()) 1)}
noncomputable def reported : State Unit Bool :=
  {entered with nodes := NormalLog.put entered.nodes () (ViewFence.reply (entered.nodes ()))}
noncomputable def activated : State Unit Bool :=
  {reported with sources := NormalLog.put reported.sources 1 (some ([true], [true]))}
noncomputable def installed : State Unit Bool :=
  {activated with nodes := NormalLog.put activated.nodes () (ViewFence.install (activated.nodes ()) 1 [true])}

def report : ViewSelection.Report Unit Bool := ⟨(), 0, [true], 0, by decide⟩

theorem certificate : Certified family reported.nodes 1 [true] := by
  refine ⟨report, [], fun _ => True, True.intro, ?_, ?_, rfl⟩
  · intro a _; cases a; exact ⟨report, List.mem_cons_self, rfl⟩
  · intro r hr
    simp only [List.mem_singleton] at hr; subst r
    simp [reported, entered, delayed, voted, sent, initial, report,
      ViewFence.reply, ViewFence.enter, ViewFence.append, ViewFence.initial]

theorem reachable : Reachable family installed := by
  have hsent : Reachable family sent := .step .init (.propose initial 0 [] [] true (by simp [initial]))
  have hvoted : Reachable family voted := .step hsent
    (.receive sent () ⟨0, [], true⟩ List.mem_cons_self rfl rfl rfl)
  have hdelayed : Reachable family delayed := .step hvoted
    (.propose voted 0 [] [true] false (by simp [voted, sent]))
  have hentered : Reachable family entered := .step hdelayed
    (.enter delayed () 1 (by simp [delayed, voted, sent, initial, ViewFence.append, ViewFence.initial]))
  have hreported : Reachable family reported := .step hentered
    (.report entered () (by simp [entered, delayed, voted, sent, initial,
      ViewFence.enter, ViewFence.append, ViewFence.initial]))
  have hactivated : Reachable family activated := .step hreported
    (.create reported 1 [true] (by simp [reported, entered, delayed, voted, sent, initial, NormalLog.put]) certificate)
  exact .step hactivated (.install activated () 1 [true] [true]
    (by simp [activated])
    (by simp [activated, reported, entered, ViewFence.reply, ViewFence.enter])
    (by simp [activated, reported, entered, delayed, voted, sent, initial,
      ViewFence.reply, ViewFence.enter, ViewFence.append, ViewFence.initial]))

/-- The shared model admits nonempty commits on both sides of a view change. -/
theorem commits_both_views : Committed family installed 0 [true] ∧
    Committed family installed 1 [true] := by
  constructor <;> refine ⟨fun _ => True, True.intro, ?_⟩ <;> intro a _ <;> cases a
  · exact ⟨[true], by simp [installed, activated, reported, entered, delayed, voted,
      sent, initial, ViewFence.install, ViewFence.reply, ViewFence.enter, ViewFence.append,
      ViewFence.initial], List.prefix_rfl⟩
  · exact ⟨[true], by simp [installed, ViewFence.install], List.prefix_rfl⟩

/-- Fault control: the delayed old-view slot-2 message has the right slot but
the wrong view. Bypassing only that guard permits an unsupported view-1 vote. -/
theorem wrong_view_breaks_origin :
    (⟨0, [true], false⟩ : Prepare Bool) ∈ installed.messages ∧
    (installed.nodes ()).floor = (installed.nodes ()).retained ∧
    [true].length = (installed.nodes ()).log.length ∧
    0 ≠ (installed.nodes ()).retained ∧
    ¬ Origin installed.sources 1 [true, false] := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · exact List.mem_cons_self
  · simp [installed, ViewFence.install]
  · simp [installed, ViewFence.install]
  · simp [installed, ViewFence.install]
  · intro ⟨base, tail, hs, _, hp⟩
    simp only [installed, activated, NormalLog.put_same, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, rfl⟩ := hs
    have hn := hp.length_le
    simp at hn

end Example
end LogProvenance
```

```bash
lake env lean UVRR/LogProvenance.lean
```

```output
```

```bash
printf 'import UVRR.LogProvenance\n#print axioms LogProvenance.reachable_inv\n#print axioms LogProvenance.later_base_preserves\n#print axioms LogProvenance.committed_comparable\n#print axioms LogProvenance.committed_equal\n#print axioms LogProvenance.Example.reachable\n#print axioms LogProvenance.Example.commits_both_views\n#print axioms LogProvenance.Example.wrong_view_breaks_origin\n' | lake env lean --stdin
```

```output
'LogProvenance.reachable_inv' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.later_base_preserves' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.committed_comparable' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.committed_equal' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.Example.reachable' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.Example.commits_both_views' depends on axioms: [propext, Classical.choice, Quot.sound]
'LogProvenance.Example.wrong_view_breaks_origin' depends on axioms: [propext, Classical.choice, Quot.sound]
```
