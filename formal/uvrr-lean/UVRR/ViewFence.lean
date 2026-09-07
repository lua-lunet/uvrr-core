import UVRR.ViewSelection

/-! Multi-view local fencing, without crashes. Installing a higher normal view
may replace an uncommitted suffix; appends within that view preserve prefixes.
Entering view change raises the floor before any reply is emitted. Votes and
replies are immutable ghost histories. Installation takes an arbitrary log:
its cross-view safety must come from the global selection theorem, not a guard
that already assumes agreement. The first view-change round and recovery are
not yet modeled here; this is the local history projection they must preserve.
-/
namespace ViewFence

structure Reply (V : Type) where
  target : Nat
  retained : Nat
  log : List V

structure State (V : Type) where
  floor : Nat
  retained : Nat
  log : List V
  votes : List (Nat × List V)
  replies : List (Reply V)

def initial {V : Type} : State V := ⟨0, 0, [], [], []⟩

def install {V : Type} (s : State V) (v : Nat) (log : List V) : State V :=
  {s with floor := v, retained := v, log := log, votes := (v, log) :: s.votes}

def append {V : Type} (s : State V) (x : V) : State V :=
  {s with log := s.log ++ [x], votes := (s.retained, s.log ++ [x]) :: s.votes}

def enter {V : Type} (s : State V) (target : Nat) : State V :=
  {s with floor := target}

def reply {V : Type} (s : State V) : State V :=
  {s with replies := ⟨s.floor, s.retained, s.log⟩ :: s.replies}

inductive Step {V : Type} : State V → State V → Prop
  | install (s : State V) (v : Nat) (log : List V)
      (floor : s.floor ≤ v) (higher : s.retained < v) : Step s (install s v log)
  | append (s : State V) (x : V) (normal : s.floor = s.retained) : Step s (append s x)
  | enter (s : State V) (target : Nat) (higher : s.floor < target) : Step s (enter s target)
  | reply (s : State V) (changing : s.retained < s.floor) : Step s (reply s)

inductive Reachable {V : Type} : State V → Prop
  | init : Reachable initial
  | step {s t} : Reachable s → Step s t → Reachable t

def Covers {V : Type} (retained : Nat) (log : List V) (vote : Nat × List V) : Prop :=
  vote.1 ≤ retained ∧ (vote.1 = retained → vote.2 <+: log)

def ReplySafe {V : Type} (s : State V) (r : Reply V) : Prop :=
  r.target ≤ s.floor ∧ r.retained < r.target ∧
    ∀ vote ∈ s.votes, vote.1 < r.target → Covers r.retained r.log vote

structure Inv {V : Type} (s : State V) : Prop where
  floor : s.retained ≤ s.floor
  current : ∀ vote ∈ s.votes, Covers s.retained s.log vote
  replies : ∀ r ∈ s.replies, ReplySafe s r

theorem initial_inv {V : Type} : Inv (initial (V := V)) := by
  constructor
  · exact Nat.le_refl _
  · simp [initial]
  · simp [initial]

theorem install_inv {V : Type} {s : State V} (h : Inv s) (v : Nat) (log : List V)
    (hf : s.floor ≤ v) (hv : s.retained < v) : Inv (install s v log) := by
  constructor
  · exact Nat.le_refl _
  · intro vote hm
    rcases List.mem_cons.mp hm with he | hm
    · subst vote; exact ⟨Nat.le_refl _, fun _ => List.prefix_rfl⟩
    · have hc := (h.current vote hm).1
      refine ⟨by change vote.1 ≤ v; omega, ?_⟩
      intro he; change vote.1 = v at he; omega
  · intro r hr
    obtain ⟨hrf, hrt, hist⟩ := h.replies r hr
    refine ⟨Nat.le_trans hrf hf, hrt, ?_⟩
    intro vote hm hlt
    rcases List.mem_cons.mp hm with he | hm
    · subst vote; change v < r.target at hlt; omega
    · exact hist vote hm hlt

theorem append_inv {V : Type} {s : State V} (h : Inv s) (x : V)
    (hn : s.floor = s.retained) : Inv (append s x) := by
  constructor
  · exact h.floor
  · intro vote hm
    rcases List.mem_cons.mp hm with he | hm
    · subst vote; exact ⟨Nat.le_refl _, fun _ => List.prefix_rfl⟩
    · obtain ⟨hv, hp⟩ := h.current vote hm
      exact ⟨hv, fun he => List.prefix_append_of_prefix (hp he)⟩
  · intro r hr
    obtain ⟨hrf, hrt, hist⟩ := h.replies r hr
    refine ⟨hrf, hrt, ?_⟩
    intro vote hm hlt
    rcases List.mem_cons.mp hm with he | hm
    · subst vote; change s.retained < r.target at hlt; omega
    · exact hist vote hm hlt

theorem enter_inv {V : Type} {s : State V} (h : Inv s) (target : Nat)
    (ht : s.floor < target) : Inv (enter s target) := by
  constructor
  · exact Nat.le_trans h.floor (Nat.le_of_lt ht)
  · exact h.current
  · intro r hr
    obtain ⟨hrf, hrt, hist⟩ := h.replies r hr
    exact ⟨Nat.le_trans hrf (Nat.le_of_lt ht), hrt, hist⟩

theorem reply_inv {V : Type} {s : State V} (h : Inv s)
    (hc : s.retained < s.floor) : Inv (reply s) := by
  constructor
  · exact h.floor
  · exact h.current
  · intro r hr
    rcases List.mem_cons.mp hr with he | hr
    · subst r
      exact ⟨Nat.le_refl _, hc, fun vote hv _ => h.current vote hv⟩
    · exact h.replies r hr

theorem reachable_inv {V : Type} {s : State V} (hr : Reachable s) : Inv s := by
  induction hr with
  | init => exact initial_inv
  | step _ st ih =>
    cases st with
    | install v log hf hv => exact install_inv ih v log hf hv
    | append x hn => exact append_inv ih x hn
    | enter target ht => exact enter_inv ih target ht
    | reply hc => exact reply_inv ih hc

/-- A historical report bounds every vote below its target, including votes
recorded after the report. Equal retained views preserve the voted prefix. -/
theorem report_covers_vote {V : Type} {s : State V} (hr : Reachable s)
    (r : Reply V) (hm : r ∈ s.replies) (v : Nat) (log : List V)
    (hv : (v, log) ∈ s.votes) (hlt : v < r.target) :
    v ≤ r.retained ∧ (v = r.retained → log <+: r.log) :=
  (reachable_inv hr).replies r hm |>.2.2 (v, log) hv hlt

/-- Operationally derived voter-history premise for the existing selector.
All reports target the same new view. A commit-quorum member need only have
voted a log extending the committed prefix, not exactly that prefix. -/
theorem voter_history {A V : Type} (states : A → State V)
    (reachable : ∀ a, Reachable (states a)) (reports : List (ViewSelection.Report A V))
    (target v : Nat) (before : v < target) (commitQ : NSet A) (committedPrefix : List V)
    (votes : ∀ a, commitQ a → ∃ log, (v, log) ∈ (states a).votes ∧ committedPrefix <+: log)
    (authentic : ∀ r ∈ reports, (⟨target, r.retained, r.log⟩ : Reply V) ∈ (states r.sender).replies) :
    ∀ r ∈ reports, commitQ r.sender →
      v ≤ r.retained ∧ (r.retained = v → committedPrefix <+: r.log) := by
  intro r hr hq
  obtain ⟨log, hv, hp⟩ := votes r.sender hq
  obtain ⟨hle, he⟩ := report_covers_vote (reachable r.sender) _ (authentic r hr) v log hv before
  exact ⟨hle, fun eq => hp.trans (he eq.symm)⟩

/-- Whole-log selection now consumes local executions instead of an assumed
voter-history axiom. Same-view message provenance and the strictly later-view
induction hypothesis remain explicit global composition obligations. -/
theorem selection_preserves {A V : Type} (states : A → State V)
    (reachable : ∀ a, Reachable (states a))
    (seed : ViewSelection.Report A V) (rest : List (ViewSelection.Report A V))
    (target v : Nat) (before : v < target) (committedPrefix : List V)
    (commitFamily viewFamily : QSys A) (overlap : Frown commitFamily viewFamily)
    (commitQ viewQ : NSet A) (hc : commitFamily commitQ) (hq : viewFamily viewQ)
    (coverage : ∀ a, viewQ a → ∃ r ∈ seed :: rest, r.sender = a)
    (votes : ∀ a, commitQ a → ∃ log, (v, log) ∈ (states a).votes ∧ committedPrefix <+: log)
    (authentic : ∀ r ∈ seed :: rest,
      (⟨target, r.retained, r.log⟩ : Reply V) ∈ (states r.sender).replies)
    (sameView : ∀ r ∈ seed :: rest, ∀ q ∈ seed :: rest,
      r.retained = q.retained → r.log <+: q.log ∨ q.log <+: r.log)
    (later : ∀ r ∈ seed :: rest, v < r.retained → committedPrefix <+: r.log) :
    committedPrefix <+: (ViewSelection.select seed rest).log :=
  ViewSelection.quorum_preserves seed rest committedPrefix v commitFamily viewFamily
    overlap commitQ viewQ hc hq coverage
    (voter_history states reachable _ target v before commitQ committedPrefix votes authentic)
    sameView later

/-- Close the strictly-later-view induction over all natural view numbers.
The remaining history interface requires each report log to extend the base
installed in its retained view and same-view provenance. No later-view safety
hypothesis is supplied by the caller. Only actually activated views require
quorum evidence; absent views have no fabricated reports. View zero is the initial view and need
not have been installed through a view-change quorum. -/
theorem all_later_views_preserve {A V : Type} (states : A → State V)
    (reachable : ∀ a, Reachable (states a))
    (active : Nat → Prop) (base : Nat → List V) (seed : Nat → ViewSelection.Report A V)
    (rest : Nat → List (ViewSelection.Report A V))
    (family : QSys A) (overlap : Frown family family)
    (viewQ : Nat → NSet A) (viewQuorum : ∀ b, active b → family (viewQ b))
    (coverage : ∀ b, active b → ∀ a, viewQ b a → ∃ r ∈ seed b :: rest b, r.sender = a)
    (selected : ∀ b, active b → base b = (ViewSelection.select (seed b) (rest b)).log)
    (authentic : ∀ b, active b → ∀ r ∈ seed b :: rest b,
      (⟨b, r.retained, r.log⟩ : Reply V) ∈ (states r.sender).replies)
    (sameView : ∀ b, active b → ∀ r ∈ seed b :: rest b, ∀ q ∈ seed b :: rest b,
      r.retained = q.retained → r.log <+: q.log ∨ q.log <+: r.log)
    (extendsBase : ∀ b, active b → ∀ r ∈ seed b :: rest b, base r.retained <+: r.log ∧ (0 < r.retained → active r.retained))
    (v : Nat) (committedPrefix : List V) (commitQ : NSet A) (hc : family commitQ)
    (votes : ∀ a, commitQ a → ∃ log, (v, log) ∈ (states a).votes ∧ committedPrefix <+: log) :
    ∀ b, active b → v < b → committedPrefix <+: base b := by
  intro b
  induction b using Nat.strongRecOn with
  | ind b ih =>
    intro hb hvb
    rw [selected b hb]
    apply selection_preserves states reachable (seed b) (rest b) b v hvb committedPrefix
      family family overlap commitQ (viewQ b) hc (viewQuorum b hb)
      (coverage b hb) votes (authentic b hb) (sameView b hb)
    intro r hr hvr
    have hrb : r.retained < b :=
      ((reachable_inv (reachable r.sender)).replies _ (authentic b hb r hr)).2.1
    obtain ⟨hp, ha⟩ := extendsBase b hb r hr
    exact (ih r.retained hrb (ha (by omega)) hvr).trans hp

/-- A nonempty two-view execution instantiates the induction interface. -/
def demoState : State Bool :=
  reply (enter (append (install (reply (enter initial 1)) 1 []) true) 2)

theorem demo_reachable : Reachable demoState :=
  .step (.step (.step (.step (.step (.step .init
    (.enter initial 1 (by decide))) (.reply _ (by decide)))
    (.install _ 1 [] (by decide) (by decide))) (.append _ true (by decide)))
    (.enter _ 2 (by decide))) (.reply _ (by decide))

def demoBase (b : Nat) : List Bool := if b = 2 then [true] else []
def demoReport (b : Nat) : ViewSelection.Report Unit Bool :=
  if b = 2 then ⟨(), 1, [true], 0, by decide⟩ else ⟨(), 0, [], 0, by decide⟩

theorem two_view_application : [true] <+: demoBase 2 := by
  apply all_later_views_preserve (fun (_ : Unit) => demoState) (fun _ => demo_reachable)
    (fun b => b = 1 ∨ b = 2) demoBase demoReport (fun _ => [])
    (fun q => q ()) (fun _ _ hq hr => ⟨(), hq, hr⟩)
    (fun _ _ => True) (fun _ _ => True.intro)
    (v := 1) (commitQ := fun _ => True) (hc := True.intro)
  · intro b hb a _; cases a; exact ⟨demoReport b, List.mem_cons_self, by cases hb with
      | inl h => subst b; rfl
      | inr h => subst b; rfl⟩
  · intro b hb; rcases hb with rfl | rfl <;> rfl
  · intro b hb r hr
    simp only [List.mem_singleton] at hr; subst r
    rcases hb with rfl | rfl <;> simp [demoState, demoReport, reply, enter, append, install, initial]
  · intro b hb r hr q hq he
    simp only [List.mem_singleton] at hr hq; subst r; subst q
    exact Or.inl List.prefix_rfl
  · intro b hb r hr
    simp only [List.mem_singleton] at hr; subst r
    rcases hb with rfl | rfl <;> simp [demoBase, demoReport]
  · intro a _
    exact ⟨[true], by simp [demoState, reply, enter, append, install], List.prefix_rfl⟩
  · exact Or.inr rfl
  · decide

/-- Fault control: continuing normal appends after emitting a fenced report
makes the immutable reply omit a vote below its target. -/
def fenced : State Bool := reply (enter initial 1)

theorem fenced_reachable : Reachable fenced :=
  .step (.step .init (.enter initial 1 (by decide))) (.reply _ (by decide))

theorem append_after_fence_breaks_history :
    Reachable fenced ∧ fenced.floor ≠ fenced.retained ∧
    (⟨1, 0, []⟩ : Reply Bool) ∈ (append fenced true).replies ∧
    (0, [true]) ∈ (append fenced true).votes ∧
    ¬ Covers 0 [] (0, [true]) ∧ ¬ Inv (append fenced true) := by
  have bad : ¬ Covers 0 [] (0, [true]) := by
    intro h
    have hp := h.2 rfl
    obtain ⟨tail, he⟩ := hp
    cases he
  refine ⟨fenced_reachable, by decide, List.mem_cons_self, by decide, bad, ?_⟩
  intro h
  have hs := h.replies ⟨1, 0, []⟩ List.mem_cons_self
  exact bad (hs.2.2 (0, [true]) (by decide) (by decide))

end ViewFence
