import UVRR.Reincarnation

/-! The identity law (`docs/uvrr-boot-gate.md` §5): a node identity is
universally unique, durable before use, and never recycled — the ballot
obligation of Paxos Made Simple applied to node identity. The durable marker
names the explicit pair `{systemId, crashCounter}`, one-indexed, both never
read as zero; a crashed boot increments the counter, commits the marker, and
only then announces.

This module states the law as an injectivity premise over identity
assignments, proves the lawful pair construction satisfies it in both
dimensions (origins and lives), proves the wire packing injective inside its
bounds, exhibits the three broken constructions that make each clause
necessary — the origin-blind mint (isolated nodes collide), the
use-before-durable mint (one origin recycles itself), and the zero read
(uninitialised bytes are not an identity) — ties the durability ordering to
the existing reincarnation phase machine (the wire phase is unreachable
without the bump's durable write), and discharges the weight-0 observation
ruling: a weight-0 member is invisible to every majority, so a seated
observation at weight 0 cannot be mistaken for a voter.
-/

namespace IdentityLaw

/-- The durable identity: the sysadmin-assigned system identifier and the
crash bump counter, durable in the same marker file. -/
structure Identity where
  systemId : Nat
  counter : Nat

/-- One-indexed: neither field is ever read as zero. A zero read is a
corrupt marker, not an identity. -/
def oneIndexed (p : Identity) : Prop := p.systemId ≠ 0 ∧ p.counter ≠ 0

/-- An assignment of identities to lives: origin and boot number to the
identity the boot announces. -/
abbrev Assignment := Nat → Nat → Identity

/-- The law, unique clause: no two distinct lives — of any origins — ever
receive the same identity. -/
def Unique (f : Assignment) : Prop :=
  ∀ s₁ s₂ k₁ k₂, f s₁ k₁ = f s₂ k₂ → s₁ = s₂ ∧ k₁ = k₂

/-! ### The lawful construction -/

/-- The lawful assignment: the sysadmin's system identifier, and the durable
crash counter incremented before the first emission — the k-th life carries
counter `k + 1`, one-indexed from the genesis life. -/
def lawful : Assignment := fun s k => ⟨s, k + 1⟩

/-- Origins never collide: the system identifier separates them. -/
theorem lawful_origin_separates {s₁ s₂ : Nat} (h : s₁ ≠ s₂) (k₁ k₂ : Nat) :
    lawful s₁ k₁ ≠ lawful s₂ k₂ := by
  intro heq
  have hsys : s₁ = s₂ := by
    have := congrArg Identity.systemId heq
    simpa [lawful] using this
  exact h hsys

/-- Lives never collide: the counter separates them. -/
theorem lawful_life_separates {s : Nat} {k₁ k₂ : Nat} (h : k₁ ≠ k₂) :
    lawful s k₁ ≠ lawful s k₂ := by
  intro heq
  have hcnt : k₁ + 1 = k₂ + 1 := by
    have := congrArg Identity.counter heq
    simpa [lawful] using this
  apply h
  omega

/-- The lawful assignment satisfies the law. -/
theorem lawful_unique : Unique lawful := by
  intro s₁ s₂ k₁ k₂ heq
  have hsys : s₁ = s₂ := by
    have := congrArg Identity.systemId heq
    simpa [lawful] using this
  have hcnt : k₁ = k₂ := by
    have := congrArg Identity.counter heq
    simpa [lawful] using this
  exact And.intro hsys hcnt

/-- The lawful construction is one-indexed whenever the system identifier
is. -/
theorem lawful_one_indexed {s : Nat} (hs : s ≠ 0) (k : Nat) :
    oneIndexed (lawful s k) := by
  unfold oneIndexed lawful
  have hcnt : k + 1 ≠ 0 := by omega
  exact And.intro hs hcnt

/-! ### The wire packing

The wire name is one `u32`: the system identifier in the high byte, the
counter in the low twenty-four bits. Inside the bounds the packing is
injective, so the wire name inherits the pair's uniqueness. -/

/-- The wire packing: `systemId * 2^24 + counter`. -/
def pack (p : Identity) : Nat := p.systemId * 16777216 + p.counter

/-- The counter is recoverable from the wire name. -/
theorem pack_counter (p : Identity) (h : p.counter < 16777216) :
    pack p % 16777216 = p.counter := by
  unfold pack
  omega

/-- The system identifier is recoverable from the wire name. -/
theorem pack_system (p : Identity) (h : p.counter < 16777216) :
    pack p / 16777216 = p.systemId := by
  unfold pack
  omega

/-- Inside the bounds, equal wire names are equal identities. -/
theorem pack_injective {p q : Identity} (hp : p.counter < 16777216)
    (hq : q.counter < 16777216) (h : pack p = pack q) : p = q := by
  have hcnt : p.counter = q.counter := by
    have := congrArg (fun x => x % 16777216) h
    rw [pack_counter p hp, pack_counter q hq] at this
    exact this
  have hsys : p.systemId = q.systemId := by
    have := congrArg (fun x => x / 16777216) h
    rw [pack_system p hp, pack_system q hq] at this
    exact this
  cases p; cases q; simp_all

/-! ### The three ways to break it

Each broken construction keeps the other stated clauses and violates the law
in its own way. These are independence witnesses in the `NegativeControls`
idiom: they show why each clause must be discharged by an implementation;
they do not claim every possible host breaks every clause. -/

/-- Broken one — the origin-blind mint. The wire name is the boot counter
alone (the isolated `max(observed) + 1` rule): every origin mints the same
stream, so two isolated systems collide on every life. The per-origin
premise still holds — no origin ever repeats itself. -/
def originBlind : Assignment := fun _ k => ⟨1, k + 1⟩

/-- The surviving premise: within one origin the stream never repeats. -/
theorem originBlind_fresh_per_origin (s : Nat) {k₁ k₂ : Nat} (h : k₁ ≠ k₂) :
    originBlind s k₁ ≠ originBlind s k₂ := by
  intro heq
  have hcnt : k₁ + 1 = k₂ + 1 := by
    have := congrArg Identity.counter heq
    simpa [originBlind] using this
  apply h
  omega

/-- The failure: two origins, same life, same identity. -/
theorem originBlind_collides (k : Nat) : originBlind 1 k = originBlind 2 k := by
  simp [originBlind]

theorem originBlind_not_unique : ¬ Unique originBlind := by
  intro huniq
  have h := huniq 1 2 0 0 (originBlind_collides 0)
  rcases h with ⟨hsys, _⟩
  omega

/- Broken two — use-before-durable. Each boot announces one more than the
DURABLE counter; a life that announced without flushing leaves the durable
counter unmoved, so the next boot announces the same value again. The system
component survives — origins never collide — and a fully flushed history
never repeats; the recycling is exactly the skipped flush. -/

/-- How many of the first `k` lives were flushed durable. -/
def durableCount (flushed : Nat → Bool) : Nat → Nat
  | 0 => 0
  | k + 1 => durableCount flushed k + (if flushed k then 1 else 0)

/-- The use-before-durable assignment: the announced counter is one more
than the count of lives that ever reached the disk. -/
def useBeforeDurable (flushed : Nat → Bool) : Assignment :=
  fun s k => ⟨s, durableCount flushed k + 1⟩

/-- The surviving premise: origins stay separated. -/
theorem useBeforeDurable_origin_separates (flushed : Nat → Bool)
    {s₁ s₂ : Nat} (h : s₁ ≠ s₂) (k₁ k₂ : Nat) :
    useBeforeDurable flushed s₁ k₁ ≠ useBeforeDurable flushed s₂ k₂ := by
  intro heq
  have hsys : s₁ = s₂ := by
    have := congrArg Identity.systemId heq
    simpa [useBeforeDurable] using this
  exact h hsys

/-- The failure: life `k` announced without flushing, so life `k + 1`
re-announces the same identity — one identity, two lives of one origin. -/
theorem useBeforeDurable_recycles (flushed : Nat → Bool) (s k : Nat)
    (hk : flushed k = false) :
    useBeforeDurable flushed s k = useBeforeDurable flushed s (k + 1) := by
  unfold useBeforeDurable
  simp [durableCount, hk]

theorem useBeforeDurable_not_unique (flushed : Nat → Bool) (k : Nat)
    (hk : flushed k = false) : ¬ Unique (useBeforeDurable flushed) := by
  intro huniq
  have h := huniq 1 1 k (k + 1) (useBeforeDurable_recycles flushed 1 k hk)
  rcases h with ⟨_, hk'⟩
  omega

/-- The counterpart: when every life flushes before announcing, the same
construction is lawful — durability is the whole difference. -/
theorem useBeforeDurable_unique_when_flushed :
    Unique (useBeforeDurable (fun _ => true)) := by
  have hctd : ∀ k, durableCount (fun _ => true) k = k := by
    intro k
    induction k with
    | zero => rfl
    | succ k ih => simp [durableCount, ih]
  intro s₁ s₂ k₁ k₂ heq
  have hsys : s₁ = s₂ := congrArg Identity.systemId heq
  have hcnt : durableCount (fun _ => true) k₁ + 1 =
      durableCount (fun _ => true) k₂ + 1 := congrArg Identity.counter heq
  rw [hctd, hctd] at hcnt
  exact ⟨hsys, by omega⟩

/-- Broken three — the zero read. Uninitialised bytes read as the identity
zero: every boot of every blank disk presents the same pair. The
never-read-as-zero assert is the refusal that keeps a corrupt marker out of
the identity space. -/
def zeroRead : Assignment := fun _ _ => ⟨0, 0⟩

theorem zeroRead_not_one_indexed (s k : Nat) : ¬ oneIndexed (zeroRead s k) := by
  unfold oneIndexed zeroRead
  simp

theorem zeroRead_not_unique : ¬ Unique zeroRead := by
  intro huniq
  have h := huniq 1 2 0 0 (by simp [zeroRead])
  rcases h with ⟨hsys, _⟩
  omega

/-! ### Durable before emission, in the existing phase machine

The reincarnation phase machine already orders the bump's durable write
before the wire phase (`dirty → bumped → reincarnating`): any forced run
that reaches the wire phase passed through the durable write. -/

/-- The wire phase is reachable from a crashed classification only through
the bump's durable write. -/
theorem wire_requires_bump
    (h : Reincarnation.ForcedRun .dirty .reincarnating) :
    Reincarnation.ForcedRun .dirty .bumped := by
  cases h with
  | step run st => cases st; exact run

/-! ### The weight-0 observation ruling

A weight-0 member contributes nothing to any response set's mass, so no
majority depends on its presence: its answers are invisible to every quorum
the cluster can form. A seated observation at weight 0 therefore cannot be
mistaken for a promoted voter. -/

/-- Weight 0 is never voting. -/
theorem weight0_not_voting {A : Type} (w : Reincarnation.Config A) (a : A)
    (h : w a = 0) : ¬ Reincarnation.voting w a := by
  simp [Reincarnation.voting, h]

/-- A weight-0 identity's presence never changes a set's mass. -/
theorem weight0_mass_irrelevant {A : Type} (nodes : List A) (w : A → Nat)
    (q : NSet A) (a : A) (h : w a = 0) :
    WeightedGeneral.mass nodes w q =
      WeightedGeneral.mass nodes w (fun x => x ≠ a ∧ q x) := by
  classical
  unfold WeightedGeneral.mass
  induction nodes with
  | nil => rfl
  | cons x rest ih =>
    simp only [WeightedGeneral.total]
    rw [ih]
    congr 1
    split <;> rename_i hq
    · split <;> rename_i hq'
      · rfl
      · have hxa : x = a := by
          by_cases hne : x = a
          · exact hne
          · exact absurd ⟨hne, hq⟩ hq'
        rw [hxa]; exact h
    · split <;> rename_i hq'
      · exact absurd hq'.2 hq
      · rfl

/-- So no majority family changes when a weight-0 identity is removed. -/
theorem weight0_majority_irrelevant {A : Type} (nodes : List A) (w : A → Nat)
    (q : NSet A) (a : A) (h : w a = 0) :
    WeightedGeneral.majority nodes w q ↔
      WeightedGeneral.majority nodes w (fun x => x ≠ a ∧ q x) := by
  rw [WeightedGeneral.majority, WeightedGeneral.majority, weight0_mass_irrelevant nodes w q a h]

end IdentityLaw
