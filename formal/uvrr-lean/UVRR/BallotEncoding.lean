import UVRR.LexBallot

/-! Rung 32 — Ballot encodings: when a scalar host representation is sound.

The ladder's ballots are `(era, view)` pairs under the lexicographic order
`bLt` (`LexBallot.lean`). A host is free to carry one scalar instead — a byte
packing, a scaled integer, an IEEE binary64 — provided the encoding is
strictly monotone into a well-founded strict total order. This module proves
the transfer theorem that licenses any such encoding: the pulled-back scalar
comparison *is* `bLt`, so every order-theoretic side condition of the
agreement theorem (`Paxos.theorem10`) is inherited unchanged.

The float reading: a binary64 significand is 53 bits, so an integer-valued
scalar is exact below `2^53`. With 32 view bits the era horizon is `2^21`;
inside it the scaled packing is exact and strictly monotone (`pack_strictMono`,
`pack_u32_below_f64_horizon`). Past the horizon the representation rounds, two
ballots share one scalar, and totality of the pulled-back order fails at
exactly the colliding pair (`collision_not_total`). -/

namespace BallotEncoding

/-- The relation a host actually decides: comparison of the scalar encodings. -/
def encodedLt {α : Type} (encode : Ballot → α) (lt : α → α → Prop) :
    Ballot → Ballot → Prop :=
  fun a b => lt (encode a) (encode b)

/-- The requirements on the scalar side: a well-founded strict total order. -/
structure StrictTotalOrderSpec {α : Type} (lt : α → α → Prop) : Prop where
  wf : WellFounded lt
  trans : ∀ a b c, lt a b → lt b c → lt a c
  irrefl : ∀ a, ¬ lt a a
  total : ∀ a b, lt a b ∨ a = b ∨ lt b a

/-- The encoding ruling: a strictly monotone encoding decides exactly the
lexicographic ballot order. The converse direction is where monotonicity is
load-bearing: were two distinct ballots to share one scalar, neither
trichotomy branch could be recovered. -/
theorem encodedLt_iff_bLt {α : Type} {lt : α → α → Prop}
    (ho : StrictTotalOrderSpec lt) {encode : Ballot → α}
    (hmono : ∀ a b, bLt a b → lt (encode a) (encode b)) :
    ∀ a b, encodedLt encode lt a b ↔ bLt a b := by
  intro a b
  constructor
  · intro h
    rcases bLt_total a b with hab | hab | hab
    · exact hab
    · subst hab; exact absurd h (ho.irrefl _)
    · exact absurd (ho.trans _ _ _ h (hmono _ _ hab)) (ho.irrefl _)
  · exact hmono a b

/-- The pulled-back relation and `bLt` are the same relation. -/
theorem encodedLt_eq_bLt {α : Type} {lt : α → α → Prop}
    (ho : StrictTotalOrderSpec lt) {encode : Ballot → α}
    (hmono : ∀ a b, bLt a b → lt (encode a) (encode b)) :
    encodedLt encode lt = bLt := by
  funext a b
  exact propext (encodedLt_iff_bLt ho hmono a b)

/-- Any Synod history whose ballot order is the pulled-back scalar comparison
satisfies the order side conditions outright. -/
theorem Synod.order_encoded {A V : Type} (S : Synod A Ballot V)
    {α : Type} {lt : α → α → Prop} (ho : StrictTotalOrderSpec lt)
    {encode : Ballot → α} (hmono : ∀ a b, bLt a b → lt (encode a) (encode b))
    (hlt : S.lt = encodedLt encode lt) : S.Order := by
  have hb : S.lt = bLt := by rw [hlt, encodedLt_eq_bLt ho hmono]
  refine ⟨?_, ?_, ?_, ?_⟩ <;> rw [hb]
  · exact bLt_wf
  · exact bLt_trans
  · exact bLt_irrefl
  · exact bLt_total

/-- Theorem 10 specialised to scalar-encoded ballots: agreement is inherited
through any sound encoding, exactly as for the literal lexicographic pair. -/
theorem Paxos.theorem10_encoded {A V : Type} (P : Paxos A Ballot V)
    {α : Type} {lt : α → α → Prop} (ho : StrictTotalOrderSpec lt)
    {encode : Ballot → α} (hmono : ∀ a b, bLt a b → lt (encode a) (encode b))
    (hlt : P.lt = encodedLt encode lt) (_hera : P.era = Ballot.era)
    (hI : P.Inv) (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {b1 b2 : Ballot} (h1 : P.chosen i b1) (h2 : P.chosen i b2) :
    P.v i b1 = P.v i b2 := by
  apply Paxos.theorem10 hI _ hne h1 h2
  rw [hlt, encodedLt_eq_bLt ho hmono]
  exact ⟨bLt_wf, bLt_trans, bLt_irrefl, bLt_total⟩

/-- The scaled packing: era in the high position, view below the modulus. -/
def pack (m : Nat) (b : Ballot) : Nat := b.1 * m + b.2

/-- The scaled packing is strictly monotone in `bLt` on its domain (views
below the modulus): era dominates because a full view cycle is one era step. -/
theorem pack_strictMono {m : Nat} {a b : Ballot} (ha : a.2 < m) (_hb : b.2 < m)
    (h : bLt a b) : pack m a < pack m b := by
  rw [bLt_iff] at h
  obtain ⟨a1, a2⟩ := a
  obtain ⟨b1, b2⟩ := b
  rcases h with h | ⟨he, hv⟩
  · have h1 : a1 * m + a2 < a1 * m + m := Nat.add_lt_add_left ha _
    have h2 : a1 * m + m = (a1 + 1) * m := (Nat.succ_mul a1 m).symm
    have h3 : (a1 + 1) * m ≤ b1 * m := Nat.mul_le_mul_right m (Nat.succ_le_of_lt h)
    have h4 : b1 * m ≤ b1 * m + b2 := Nat.le_add_right _ _
    exact Nat.lt_of_lt_of_le h1 (h2 ▸ Nat.le_trans h3 h4)
  · subst he
    exact Nat.add_lt_add_left hv _

/-- The natural-number order is a well-founded strict total order. -/
theorem natOrder : StrictTotalOrderSpec (· < · : Nat → Nat → Prop) where
  wf := Nat.lt_wfRel.wf
  trans := fun _ _ _ => Nat.lt_trans
  irrefl := Nat.lt_irrefl
  total := fun _ _ => by omega

/-- The IEEE binary64 exactness horizon: `2^53`. Integer-valued scalars below
this are exactly representable, so a host comparing packed ballots as
binary64 values decides the same order as `Nat` comparison. -/
def f64ExactHorizon : Nat := 9007199254740992

/-- With thirty-two view bits, era `2^21` is the horizon: every scaled packing
below it is exactly representable in binary64. -/
theorem pack_u32_below_f64_horizon {e v : Nat}
    (he : e < 2097152) (hv : v < 4294967296) :
    pack 4294967296 (e, v) < f64ExactHorizon := by
  have h1 : e * 4294967296 + (v + 1) ≤ (e + 1) * 4294967296 := by
    rw [Nat.succ_mul]
    exact Nat.add_le_add_left (Nat.succ_le_of_lt hv) _
  have h2 : (e + 1) * 4294967296 ≤ 2097152 * 4294967296 :=
    Nat.mul_le_mul_right _ (Nat.succ_le_of_lt he)
  show e * 4294967296 + v < 9007199254740992
  exact Nat.lt_of_lt_of_le (Nat.lt_succ_self _) (Nat.le_trans h1 h2)

/-- The u64 wire instance: inside the domain (both views below the modulus),
the scalar comparison of the packings decides `bLt` in both directions. The
strict monotonicity of the packing holds only on the domain, so the ruling is
stated per compared pair — which is what a host's comparison site discharges. -/
theorem pack32_decides_bLt {a b : Ballot}
    (ha : a.2 < 4294967296) (hb : b.2 < 4294967296) :
    encodedLt (pack 4294967296) (· < ·) a b ↔ bLt a b := by
  constructor
  · intro h
    rcases bLt_total a b with hab | hab | hab
    · exact hab
    · subst hab; exact absurd h (Nat.lt_irrefl _)
    · exact absurd (Nat.lt_trans h (pack_strictMono hb ha hab)) (Nat.lt_irrefl _)
  · intro h
    exact pack_strictMono ha hb h

/-- The colliding encoding: the scalar retains the view only. -/
def viewOnly (b : Ballot) : Nat := b.2

/-- Negative control: two ballots of different eras share one scalar, and the
pulled-back relation cannot decide them — totality fails at exactly the
colliding pair. This is the shape of the float failure past the exactness
horizon: rounding is a collision. -/
theorem collision_not_total :
    ¬ (encodedLt viewOnly (· < ·) (1, 0) (0, 0) ∨ (1, 0) = (0, 0) ∨
        encodedLt viewOnly (· < ·) (0, 0) (1, 0)) := by
  simp [encodedLt, viewOnly]

end BallotEncoding
