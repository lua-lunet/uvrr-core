import UVRR.WeightedGeneral
import UVRR.CastingVote

/-! Finite weighted reconfiguration reachability with a fixed availability set.
The distinguished leader is available. Lists enumerate the remaining available
and unavailable identities, including zero-weight identities. Every path uses
single-identity unit changes and retains an available strict majority and a
positive leader. This is an availability and membership reachability theorem;
it does not assert a singleton intersection witness at each edge.
-/
namespace WeightedReachability

/-- One identity gains one unit; list positions are persistent identities. -/
inductive OneUp : List Nat → List Nat → Prop where
  | head (n : Nat) (xs : List Nat) : OneUp (n :: xs) ((n + 1) :: xs)
  | tail (n : Nat) {xs ys : List Nat} : OneUp xs ys → OneUp (n :: xs) (n :: ys)

theorem OneUp.sum {xs ys : List Nat} (h : OneUp xs ys) : ys.sum = xs.sum + 1 := by
  induction h with
  | head n xs => simp; omega
  | tail n h ih => simp; omega

/-- A finite sequence of coordinate increments. -/
inductive Raises : List Nat → List Nat → Prop where
  | refl (xs : List Nat) : Raises xs xs
  | step {xs ys zs : List Nat} : OneUp xs ys → Raises ys zs → Raises xs zs

theorem Raises.trans {xs ys zs : List Nat} (h : Raises xs ys) (g : Raises ys zs) :
    Raises xs zs := by
  induction h with
  | refl => exact g
  | step hu _ ih => exact .step hu (ih g)

theorem Raises.cons (n : Nat) {xs ys : List Nat} (h : Raises xs ys) :
    Raises (n :: xs) (n :: ys) := by
  induction h with
  | refl => exact .refl _
  | step hu _ ih => exact .step (.tail n hu) ih

theorem Raises.sum_le {xs ys : List Nat} (h : Raises xs ys) : xs.sum ≤ ys.sum := by
  induction h with
  | refl => exact Nat.le_refl _
  | step hu _ ih => have := hu.sum; omega

theorem raise_head (n : Nat) (xs : List Nat) : Raises (0 :: xs) (n :: xs) := by
  induction n with
  | zero => exact .refl _
  | succ n ih => exact ih.trans (.step (.head n xs) (.refl _))

theorem raise_zero (xs : List Nat) : Raises (List.replicate xs.length 0) xs := by
  induction xs with
  | nil => exact .refl _
  | cons n xs ih =>
    change Raises (0 :: List.replicate xs.length 0) (n :: xs)
    exact (ih.cons 0).trans (raise_head n xs)

structure Profile where
  leader : Nat
  available : List Nat
  unavailable : List Nat
  deriving DecidableEq

/-- All available identities together form a strict weighted majority. -/
def Available (s : Profile) : Prop :=
  0 < s.leader ∧ s.unavailable.sum < s.leader + s.available.sum

/-- A legal elementary edit changes one persistent identity's weight by one. -/
inductive UnitStep : Profile → Profile → Prop where
  | leader (l : Nat) (a d : List Nat) : UnitStep ⟨l, a, d⟩ ⟨l + 1, a, d⟩
  | available (l : Nat) (d : List Nat) {a b : List Nat} :
      OneUp a b → UnitStep ⟨l, a, d⟩ ⟨l, b, d⟩
  | unavailable (l : Nat) (a : List Nat) {d e : List Nat} :
      OneUp d e → UnitStep ⟨l, a, d⟩ ⟨l, a, e⟩
  | reverse {s t : Profile} : UnitStep s t → UnitStep t s

/-- Every vertex, including both endpoints, satisfies availability. -/
inductive Reaches : Profile → Profile → Prop where
  | refl {s : Profile} : Available s → Reaches s s
  | step {s t u : Profile} : Available s → UnitStep s t → Reaches t u → Reaches s u

theorem Reaches.start {s t : Profile} (h : Reaches s t) : Available s := by
  cases h with
  | refl h => exact h
  | step h _ _ => exact h

theorem Reaches.finish {s t : Profile} (h : Reaches s t) : Available t := by
  induction h with
  | refl h => exact h
  | step _ _ _ ih => exact ih

theorem Reaches.trans {s t u : Profile} (h : Reaches s t) (g : Reaches t u) :
    Reaches s u := by
  induction h with
  | refl => exact g
  | step ha hu _ ih => exact .step ha hu (ih g)

theorem Reaches.reverse {s t : Profile} (h : Reaches s t) : Reaches t s := by
  induction h with
  | refl ha => exact .refl ha
  | step ha hu hp ih =>
    exact ih.trans (.step hp.start (.reverse hu) (.refl ha))

/-- Decreasing unavailable weights never removes an available majority. -/
theorem lower_unavailable (l : Nat) (a : List Nat) {d e : List Nat}
    (h : Raises d e) (ha : Available ⟨l, a, e⟩) :
    Reaches ⟨l, a, e⟩ ⟨l, a, d⟩ := by
  induction h with
  | refl => exact .refl ha
  | @step d m e hu hp ih =>
    have ham : Available ⟨l, a, m⟩ := by
      have := hp.sum_le
      simp only [Available] at ha ⊢
      constructor <;> omega
    have had : Available ⟨l, a, d⟩ := by
      have := hu.sum
      simp only [Available] at ham ⊢
      constructor <;> omega
    exact (ih ha).trans (.step ham (.reverse (.unavailable l a hu)) (.refl had))

/-- Once unavailable weights are zero, a positive available leader suffices
while all other available coordinates are reduced. -/
theorem lower_available (l : Nat) (d : List Nat) (hl : 0 < l) (hd : d.sum = 0)
    {a b : List Nat} (h : Raises a b) : Reaches ⟨l, b, d⟩ ⟨l, a, d⟩ := by
  have all : ∀ xs : List Nat, Available ⟨l, xs, d⟩ := by
    intro xs
    simp only [Available]
    constructor <;> omega
  induction h with
  | refl => exact .refl (all _)
  | step hu _ ih =>
    exact ih.trans (.step (all _) (.reverse (.available l d hu)) (.refl (all _)))

theorem lower_leader (l : Nat) (a d : List Nat) (hl : 0 < l)
    (ha : a.sum = 0) (hd : d.sum = 0) : Reaches ⟨l, a, d⟩ ⟨1, a, d⟩ := by
  induction l with
  | zero => omega
  | succ n ih =>
    cases n with
    | zero => exact .refl (by simp [Available, hd, ha])
    | succ n =>
      have hs : Available ⟨n + 1 + 1, a, d⟩ := by simp [Available, hd, ha]
      exact .step hs (.reverse (.leader (n + 1) a d)) (ih (by omega))

/-- A single positive leader is a common hub for fixed identity lists. -/
def hub (a d : Nat) : Profile := ⟨1, List.replicate a 0, List.replicate d 0⟩

/-- Constructive reduction for any finite number of identities and Nat weights:
remove unavailable weight, remove other available weight, normalise the leader. -/
theorem reaches_hub (s : Profile) (h : Available s) :
    Reaches s (hub s.available.length s.unavailable.length) := by
  cases s with
  | mk l a d =>
    have p := lower_unavailable l a (raise_zero d) h
    have q := lower_available l (List.replicate d.length 0) h.1
      (by simp) (raise_zero a)
    have r := lower_leader l (List.replicate a.length 0) (List.replicate d.length 0)
      h.1 (by simp) (by simp)
    exact p.trans (q.trans r)

/-- All profiles on the same available/unavailable identity lists are connected
when both endpoints have an available strict majority and positive live leader.
Zero-weight phantom identities are ordinary list positions; no message from an
unavailable identity is a hypothesis of this arithmetic reachability theorem. -/
theorem connected (s t : Profile) (hs : Available s) (ht : Available t)
    (ha : s.available.length = t.available.length)
    (hd : s.unavailable.length = t.unavailable.length) : Reaches s t := by
  have p := reaches_hub s hs
  have q := (reaches_hub t ht).reverse
  rw [ha, hd] at p
  exact p.trans q

/-- The exact phantom weight that moves total weight to twice the available
weight minus one. Phantom identities may split this number into unit weights. -/
def phantomWeight (available unavailable : Nat) : Nat :=
  available - unavailable - 1

/-- Every available strict majority and positive live leader admits phantom
padding with two abstract majority witnesses. The preparing witness is all
available identities; the old witness is every unavailable identity plus the
leader. No cardinality bound occurs, and no unavailable promise is required. -/
theorem phantom_casting_arithmetic (a d l : Nat) (hmajority : d < a)
    (hleader : 0 < l) :
    let p := phantomWeight a d
    d + p < a ∧
    a + d + p + 1 = 2 * a ∧
    a + d + p < 2 * a ∧
    a + d + p < 2 * (d + p + l) := by
  unfold phantomWeight
  omega

/-- Every prefix of phantom padding keeps the original available identities a
strict weighted majority, so padding can be introduced one unit at a time. -/
theorem phantom_prefix_available (a d k : Nat) (hmajority : d < a)
    (hk : k ≤ phantomWeight a d) : d + k < a := by
  unfold phantomWeight at hk
  omega

/-- Abstract casting witnesses: the preparing quorum consists entirely of live
identities, and its intersection with unavailable identities plus leader is
exactly the leader. `phantom_casting_arithmetic` supplies both majority tests. -/
theorem phantom_casting_intersection {A : Type} (live : A → Prop) (leader : A)
    (hleader : live leader) :
    CastingVote.HasCastingVote
      (fun a => ¬ live a ∨ a = leader) live leader := by
  intro a
  constructor
  · intro ⟨h, ha⟩
    rcases h with h | h
    · exact False.elim (h ha)
    · exact h
  · intro h
    subst a
    exact ⟨Or.inr rfl, hleader⟩

/-- The two abstract witness masses count the whole support once and the
leader's mass twice. This includes zero-weight identities without exception. -/
theorem casting_mass_partition {A : Type} (nodes : List A) (w : A → Nat)
    (live : A → Prop) (leader : A) (hleader : live leader) :
    WeightedGeneral.mass nodes w (fun a => ¬ live a ∨ a = leader) +
      WeightedGeneral.mass nodes w live =
    WeightedGeneral.total nodes w +
      WeightedGeneral.mass nodes w (fun a => a = leader) := by
  classical
  induction nodes with
  | nil => simp [WeightedGeneral.mass, WeightedGeneral.total]
  | cons a nodes ih =>
    by_cases hl : live a
    · by_cases he : a = leader
      · subst a
        simp only [WeightedGeneral.mass, WeightedGeneral.total] at ih ⊢
        simp only [hleader, not_true_eq_false, false_or, if_true]
        omega
      · simp only [WeightedGeneral.mass, WeightedGeneral.total] at ih ⊢
        simp only [hl, he, not_true_eq_false, or_self, if_false, if_true]
        omega
    · have he : a ≠ leader := by intro h; subst a; exact hl hleader
      simp only [WeightedGeneral.mass, WeightedGeneral.total] at ih ⊢
      simp only [hl, he, not_false_eq_true, true_or, if_false, if_true]
      omega

/-- The padded odd-total equation implies actual weighted-majority membership
of the abstract witnesses in the existing weighted quorum model. The preparing
quorum contains only available identities. -/
theorem weighted_casting_at_odd_total {A : Type} (nodes : List A) (w : A → Nat)
    (live : A → Prop) (leader : A) (hleader : live leader)
    (hweight : 0 < WeightedGeneral.mass nodes w (fun a => a = leader))
    (htotal : WeightedGeneral.total nodes w + 1 =
      2 * WeightedGeneral.mass nodes w live) :
    WeightedGeneral.majority nodes w (fun a => ¬ live a ∨ a = leader) ∧
    WeightedGeneral.majority nodes w live ∧
    CastingVote.HasCastingVote (fun a => ¬ live a ∨ a = leader) live leader := by
  have hm := casting_mass_partition nodes w live leader hleader
  refine ⟨?_, ?_, phantom_casting_intersection live leader hleader⟩
  · unfold WeightedGeneral.majority
    omega
  · unfold WeightedGeneral.majority
    omega

/-- Least phantom weight for a chosen live preparing quorum. Natural-number
subtraction implements the maximum with zero. -/
def minimumPhantomWeight (total preparing leader : Nat) : Nat :=
  2 * preparing + 1 - (total + 2 * leader)

/-- Optimal padding for any chosen live majority containing a positive leader.
The preparing quorum stays a majority, and its complement plus leader becomes
a majority. The final conjunct proves minimality of the phantom weight. -/
theorem minimum_phantom_arithmetic (t p l : Nat) (hl : 0 < l)
    (hp : p ≤ t) (hq : t < 2 * p) :
    let d := minimumPhantomWeight t p l
    t + d < 2 * p ∧
    t + d < 2 * (t + d - p + l) ∧
    ∀ k : Nat, t + k < 2 * (t + k - p + l) → d ≤ k := by
  dsimp [minimumPhantomWeight]
  constructor
  · omega
  constructor
  · omega
  intro k hk
  omega

/-- With weights at most two, an inclusion-minimal live majority has weight
at most floor(total/2)+2. Under that numerical bound at most three phantom
weight units suffice, hence at most two new identities with weights 0,1,2. -/
theorem minimum_phantom_at_most_three (t p l : Nat) (hl : 0 < l)
    (hp : p ≤ t / 2 + 2) : minimumPhantomWeight t p l ≤ 3 := by
  unfold minimumPhantomWeight
  omega

/-- The optimal cap-two padding fits into two phantom identity slots. -/
theorem two_phantom_identities_suffice (t p l : Nat) (hl : 0 < l)
    (hp : p ≤ t / 2 + 2) :
    ∃ x y : Nat, x ≤ 2 ∧ y ≤ 2 ∧ x + y = minimumPhantomWeight t p l := by
  have h := minimum_phantom_at_most_three t p l hl hp
  let d := minimumPhantomWeight t p l
  refine ⟨min d 2, d - min d 2, ?_, ?_, ?_⟩ <;> dsimp [d] at * <;> omega

/-- Sharp numerical instance: total six, preparing mass five, leader mass one. -/
theorem minimum_phantom_three_sharp : minimumPhantomWeight 6 5 1 = 3 := by decide

/-- Every identity weight is within the palette 0,...,cap. -/
def ListBounded (cap : Nat) : List Nat → Prop
  | [] => True
  | x :: xs => x ≤ cap ∧ ListBounded cap xs

def Capped (cap : Nat) (s : Profile) : Prop :=
  s.leader ≤ cap ∧ ListBounded cap s.available ∧ ListBounded cap s.unavailable

theorem OneUp.lower_bounded {cap : Nat} {xs ys : List Nat} (h : OneUp xs ys)
    (hb : ListBounded cap ys) : ListBounded cap xs := by
  induction h with
  | head n xs =>
    change n + 1 ≤ cap ∧ ListBounded cap xs at hb
    exact ⟨by omega, hb.2⟩
  | tail n h ih => exact ⟨hb.1, ih hb.2⟩

theorem Raises.lower_bounded {cap : Nat} {xs ys : List Nat} (h : Raises xs ys)
    (hb : ListBounded cap ys) : ListBounded cap xs := by
  induction h with
  | refl => exact hb
  | step hu _ ih => exact hu.lower_bounded (ih hb)

theorem zero_bounded (cap n : Nat) : ListBounded cap (List.replicate n 0) := by
  induction n with
  | zero => trivial
  | succ n ih => exact ⟨Nat.zero_le _, ih⟩

/-- A path whose every state has an available majority, a positive live leader,
and all coordinates in the specified bounded palette. -/
inductive CappedReaches (cap : Nat) : Profile → Profile → Prop where
  | refl {s : Profile} : Available s → Capped cap s → CappedReaches cap s s
  | step {s t u : Profile} : Available s → Capped cap s → UnitStep s t →
      CappedReaches cap t u → CappedReaches cap s u

theorem CappedReaches.start {cap : Nat} {s t : Profile}
    (h : CappedReaches cap s t) : Available s ∧ Capped cap s := by
  cases h with
  | refl ha hb => exact ⟨ha, hb⟩
  | step ha hb _ _ => exact ⟨ha, hb⟩

theorem CappedReaches.trans {cap : Nat} {s t u : Profile}
    (h : CappedReaches cap s t) (g : CappedReaches cap t u) :
    CappedReaches cap s u := by
  induction h with
  | refl => exact g
  | step ha hb hu _ ih => exact .step ha hb hu (ih g)

theorem CappedReaches.reverse {cap : Nat} {s t : Profile}
    (h : CappedReaches cap s t) : CappedReaches cap t s := by
  induction h with
  | refl ha hb => exact .refl ha hb
  | step ha hb hu hp ih =>
    exact ih.trans (.step hp.start.1 hp.start.2 (.reverse hu) (.refl ha hb))

theorem CappedReaches.reaches {cap : Nat} {s t : Profile}
    (h : CappedReaches cap s t) : Reaches s t := by
  induction h with
  | refl ha _ => exact .refl ha
  | step ha _ hu _ ih => exact .step ha hu ih

theorem capped_lower_unavailable (cap l : Nat) (a : List Nat) {d e : List Nat}
    (h : Raises d e) (ha : Available ⟨l, a, e⟩) (hb : Capped cap ⟨l, a, e⟩) :
    CappedReaches cap ⟨l, a, e⟩ ⟨l, a, d⟩ := by
  induction h with
  | refl => exact .refl ha hb
  | @step d m e hu hp ih =>
    have ham : Available ⟨l, a, m⟩ := by
      have := hp.sum_le
      simp only [Available] at ha ⊢
      constructor <;> omega
    have had : Available ⟨l, a, d⟩ := by
      have := hu.sum
      simp only [Available] at ham ⊢
      constructor <;> omega
    have hbm : Capped cap ⟨l, a, m⟩ :=
      ⟨hb.1, hb.2.1, hp.lower_bounded hb.2.2⟩
    have hbd : Capped cap ⟨l, a, d⟩ :=
      ⟨hb.1, hb.2.1, hu.lower_bounded hbm.2.2⟩
    exact (ih ha hb).trans
      (.step ham hbm (.reverse (.unavailable l a hu)) (.refl had hbd))

theorem capped_lower_available (cap l : Nat) (d : List Nat) (hl : 0 < l)
    (hd : d.sum = 0) {a b : List Nat} (h : Raises a b)
    (hb : Capped cap ⟨l, b, d⟩) :
    CappedReaches cap ⟨l, b, d⟩ ⟨l, a, d⟩ := by
  have all : ∀ xs : List Nat, Available ⟨l, xs, d⟩ := by
    intro xs
    simp only [Available]
    constructor <;> omega
  induction h with
  | refl => exact .refl (all _) hb
  | @step a m b hu hp ih =>
    have hbm : Capped cap ⟨l, m, d⟩ :=
      ⟨hb.1, hp.lower_bounded hb.2.1, hb.2.2⟩
    have hba : Capped cap ⟨l, a, d⟩ :=
      ⟨hb.1, hu.lower_bounded hbm.2.1, hb.2.2⟩
    exact (ih hb).trans
      (.step (all _) hbm (.reverse (.available l d hu)) (.refl (all _) hba))

theorem capped_lower_leader (cap l : Nat) (a d : List Nat) (hl : 0 < l)
    (ha : a.sum = 0) (hd : d.sum = 0) (hb : Capped cap ⟨l, a, d⟩) :
    CappedReaches cap ⟨l, a, d⟩ ⟨1, a, d⟩ := by
  induction l with
  | zero => omega
  | succ n ih =>
    cases n with
    | zero => exact .refl (by simp [Available, hd, ha]) hb
    | succ n =>
      have hs : Available ⟨n + 1 + 1, a, d⟩ := by simp [Available, hd, ha]
      have hm : Capped cap ⟨n + 1, a, d⟩ := ⟨by have hh : n + 1 + 1 ≤ cap := hb.1; change n + 1 ≤ cap; omega, hb.2⟩
      exact .step hs hb (.reverse (.leader (n + 1) a d)) (ih (by omega) hm)

/-- The reduction never increases a coordinate, so it respects any endpoint
palette cap, including the voting weights 0,1,2. -/
theorem capped_reaches_hub (cap : Nat) (s : Profile) (h : Available s)
    (hb : Capped cap s) :
    CappedReaches cap s (hub s.available.length s.unavailable.length) := by
  cases s with
  | mk l a d =>
    have p := capped_lower_unavailable cap l a (raise_zero d) h hb
    have q := capped_lower_available cap l (List.replicate d.length 0) h.1
      (by simp) (raise_zero a) ⟨hb.1, hb.2.1, zero_bounded _ _⟩
    have r := capped_lower_leader cap l (List.replicate a.length 0)
      (List.replicate d.length 0) h.1 (by simp) (by simp)
      ⟨hb.1, zero_bounded _ _, zero_bounded _ _⟩
    exact p.trans (q.trans r)

/-- Complete finite-size connectivity inside any bounded weight palette:
every intermediate configuration retains the same cap and available majority. -/
theorem capped_connected (cap : Nat) (s t : Profile)
    (hs : Available s) (ht : Available t) (hbs : Capped cap s) (hbt : Capped cap t)
    (ha : s.available.length = t.available.length)
    (hd : s.unavailable.length = t.unavailable.length) : CappedReaches cap s t := by
  have p := capped_reaches_hub cap s hs hbs
  have q := (capped_reaches_hub cap t ht hbt).reverse
  rw [ha, hd] at p
  exact p.trans q

theorem zero_one_two_connected (s t : Profile)
    (hs : Available s) (ht : Available t) (hbs : Capped 2 s) (hbt : Capped 2 t)
    (ha : s.available.length = t.available.length)
    (hd : s.unavailable.length = t.unavailable.length) : CappedReaches 2 s t :=
  capped_connected 2 s t hs ht hbs hbt ha hd

/-- A bounded-weight scan reaches any attainable threshold without overshooting
by more than one identity's maximum weight. The witness is an actual prefix. -/
theorem bounded_prefix_crossing (cap threshold initial : Nat) (xs : List Nat)
    (hb : ListBounded cap xs) (hi : initial ≤ threshold + cap)
    (ht : threshold < initial + xs.sum) :
    ∃ part suffix : List Nat, xs = part ++ suffix ∧
      threshold < initial + part.sum ∧ initial + part.sum ≤ threshold + cap := by
  induction xs generalizing initial with
  | nil => exact ⟨[], [], rfl, ht, hi⟩
  | cons x xs ih =>
    by_cases hnow : threshold < initial
    · exact ⟨[], x :: xs, rfl, hnow, hi⟩
    · have hx : x ≤ cap := hb.1
      have hinext : initial + x ≤ threshold + cap := by omega
      have htnext : threshold < (initial + x) + xs.sum := by
        simp only [List.sum_cons] at ht
        omega
      obtain ⟨p, s, hxs, hlo, hhi⟩ := ih (initial + x) hb.2 hinext htnext
      refine ⟨x :: p, s, ?_, ?_, ?_⟩
      · simp only [List.cons_append, hxs]
      · simp only [List.sum_cons]
        omega
      · simp only [List.sum_cons]
        omega

def totalWeight (s : Profile) : Nat :=
  s.leader + s.available.sum + s.unavailable.sum

/-- Any cap-two available majority supplies a live preparing quorum containing
the leader whose weight is at most floor(total/2)+2. Its other identities are
an actual prefix of the finite available-identity list. -/
theorem cap_two_preparing_prefix (s : Profile) (ha : Available s) (hb : Capped 2 s) :
    ∃ part suffix : List Nat, s.available = part ++ suffix ∧
      totalWeight s < 2 * (s.leader + part.sum) ∧
      s.leader + part.sum ≤ totalWeight s / 2 + 2 ∧
      s.leader + part.sum ≤ totalWeight s := by
  have ht : totalWeight s / 2 < s.leader + s.available.sum := by
    unfold totalWeight
    have hm := ha.2
    omega
  have hi : s.leader ≤ totalWeight s / 2 + 2 := by have := hb.1; omega
  obtain ⟨p, r, hsplit, hlo, hhi⟩ :=
    bounded_prefix_crossing 2 (totalWeight s / 2) s.leader s.available hb.2.1 hi ht
  refine ⟨p, r, hsplit, ?_, hhi, ?_⟩
  · omega
  · have hsum : s.available.sum = p.sum + r.sum := by rw [hsplit, List.sum_append]
    unfold totalWeight
    omega

/-- For every finite cap-two profile with an available majority and positive
live leader, a live prefix quorum and at most two phantom identities suffice.
The scalar majority inequalities certify both the live preparing quorum and
its abstract complement plus leader. No subset-minimisation premise remains. -/
theorem cap_two_phantom_construction (s : Profile) (ha : Available s) (hb : Capped 2 s) :
    ∃ part suffix : List Nat, ∃ x y : Nat,
      s.available = part ++ suffix ∧ x ≤ 2 ∧ y ≤ 2 ∧
      x + y = minimumPhantomWeight (totalWeight s) (s.leader + part.sum) s.leader ∧
      totalWeight s + x + y < 2 * (s.leader + part.sum) ∧
      totalWeight s + x + y <
        2 * (totalWeight s + x + y - (s.leader + part.sum) + s.leader) := by
  obtain ⟨p, r, hsplit, hmajority, hbound, htotal⟩ := cap_two_preparing_prefix s ha hb
  obtain ⟨x, y, hx, hy, hxy⟩ :=
    two_phantom_identities_suffice (totalWeight s) (s.leader + p.sum) s.leader ha.1 hbound
  have hm := minimum_phantom_arithmetic (totalWeight s) (s.leader + p.sum)
    s.leader ha.1 htotal hmajority
  dsimp only at hm
  refine ⟨p, r, x, y, hsplit, hx, hy, hxy, ?_, ?_⟩
  · omega
  · omega

theorem OneUp.length {xs ys : List Nat} (h : OneUp xs ys) : xs.length = ys.length := by
  induction h with
  | head => rfl
  | tail _ _ ih => simp only [List.length_cons, ih]

theorem OneUp.append_right {xs ys : List Nat} (h : OneUp xs ys) (zs : List Nat) :
    OneUp (xs ++ zs) (ys ++ zs) := by
  induction h with
  | head n xs => exact .head n (xs ++ zs)
  | tail n _ ih => exact .tail n ih

theorem OneUp.append_left {xs ys : List Nat} (h : OneUp xs ys) (zs : List Nat) :
    OneUp (zs ++ xs) (zs ++ ys) := by
  induction zs with
  | nil => exact h
  | cons z zs ih => exact .tail z ih

theorem total_map {A B : Type} (nodes : List A) (f : A → B) (w : B → Nat) :
    WeightedGeneral.total (nodes.map f) w = WeightedGeneral.total nodes (fun a => w (f a)) := by
  induction nodes with
  | nil => rfl
  | cons a nodes ih => simp [WeightedGeneral.total, ih]

theorem total_zero {A : Type} (nodes : List A) :
    WeightedGeneral.total nodes (fun _ => 0) = 0 := by
  induction nodes with
  | nil => rfl
  | cons a nodes ih => simp [WeightedGeneral.total, ih]

/-- The positional list edit has L1 distance exactly one in the existing
finite-support weighted quorum model. -/
theorem OneUp.distance {xs ys : List Nat} (h : OneUp xs ys) :
    WeightedGeneral.total (List.range xs.length)
      (fun i => WeightedGeneral.distance (xs.getD i 0) (ys.getD i 0)) = 1 := by
  induction h with
  | head n xs =>
    simp only [List.length_cons, List.range_succ_eq_map, WeightedGeneral.total, total_map,
      List.getD_cons_zero, List.getD_cons_succ]
    simp only [WeightedGeneral.distance, Nat.sub_self, Nat.add_zero]
    rw [total_zero]
    omega
  | tail n h ih =>
    simp only [List.length_cons, List.range_succ_eq_map, WeightedGeneral.total, total_map,
      List.getD_cons_zero, List.getD_cons_succ]
    simpa only [WeightedGeneral.distance, Nat.sub_self, Nat.zero_add] using ih

def vector (s : Profile) : List Nat := s.leader :: (s.available ++ s.unavailable)

def quorums (s : Profile) : QSys Nat :=
  WeightedGeneral.majority (List.range (vector s).length) (fun i => (vector s).getD i 0)

theorem OneUp.overlap {xs ys : List Nat} (h : OneUp xs ys) :
    Frown (WeightedGeneral.majority (List.range xs.length) (fun i => xs.getD i 0))
      (WeightedGeneral.majority (List.range ys.length) (fun i => ys.getD i 0)) := by
  rw [← h.length]
  apply WeightedGeneral.unit_change_overlap
  exact Nat.le_of_eq h.distance

/-- Every legal coordinate edge satisfies cross-configuration weighted quorum
intersection, connecting the reachability model directly to `WeightedGeneral`. -/
theorem UnitStep.overlap {s t : Profile} (h : UnitStep s t) :
    Frown (quorums s) (quorums t) := by
  induction h with
  | leader l a d => exact (OneUp.head l (a ++ d)).overlap
  | available l d h => exact (OneUp.tail l (h.append_right d)).overlap
  | unavailable l a h => exact (OneUp.tail l (h.append_left a)).overlap
  | reverse _ ih =>
    intro q r hq hr
    obtain ⟨a, ha, hb⟩ := ih r q hr hq
    exact ⟨a, hb, ha⟩

end WeightedReachability
