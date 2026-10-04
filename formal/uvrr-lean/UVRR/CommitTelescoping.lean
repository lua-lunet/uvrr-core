import UVRR.Fuse

/-! Rung 33 — Commit telescoping: the last slot's quorum certifies the range.

Rung 27's `chosen_all` takes atomic acceptance of the whole range as a
hypothesis (`Fuse.AtomicAcceptance`). The commit direction runs the other
way: a node that accepts a slot has accepted every earlier slot, because the
accept path admits an entry only as the next slot (`NormalLog.lean`'s
receiver checks `m.slot = log.length + 1`). So a quorum's acceptance of the
*last* slot of a range is already that quorum's acceptance of every slot in
it: `atomic_of_last` derives the atomicity, and the fused commit message —
the prior commit piggybacked, then the commit of the batch's last slot —
authorises a backup to commit every intervening slot without probing them.

Stated for a fixed configuration: one quorum family across the range, which
is the stable-leader setting — no view change and no era change inside the
range, so `Q` is constant and rung 27's preservation step is the identity.
Per-slot value uniqueness is the per-slot Synod agreement of rung 3 applied
at the shared ballot; this rung is the quorum-evidence kernel. -/

namespace CommitTelescoping

/-- Per-node in-order acceptance: accepting a slot certifies every earlier
slot. The accept path's next-slot guard makes this an invariant of every
reachable acceptor. -/
def AcceptsInOrder {A : Type} (accepted : Nat → A → Prop) : Prop :=
  ∀ a i j, i ≤ j → accepted j a → accepted i a

/-- Acceptance of the last slot by a response set, under in-order acceptance,
is atomic acceptance of the whole range. -/
theorem atomic_of_last {A : Type} {accepted : Nat → A → Prop} {R : NSet A}
    {n : Nat} (hord : AcceptsInOrder accepted)
    (hlast : ∀ a, R a → accepted n a) :
    Fuse.AtomicAcceptance accepted R n :=
  fun a ha i hi => hord a i n hi (hlast a ha)

/-- Fast-forward commit, evidence form: with a fixed quorum family across the
range, in-order acceptance, and the last slot accepted by a quorum, every
slot of the range holds quorum-backed chosen evidence. -/
theorem last_acceptance_decides_all {A : Type} {Q : QSys A} {R : NSet A}
    {accepted : Nat → A → Prop} {n : Nat}
    (hQ : Q R) (hord : AcceptsInOrder accepted)
    (hlast : ∀ a, R a → accepted n a) :
    ∀ i, i ≤ n → Fuse.ChosenEvidence (fun _ => Q) accepted i :=
  Fuse.chosen_all (fun _ => Q) R n accepted hQ (fun _ _ h => h)
    (atomic_of_last hord hlast)

/-- Fast-forward commit, decision form: a decision rule consuming that
evidence may mark every slot of the range — the backup that learns the last
slot's commit commits the intervening slots and finds no gap. -/
theorem commit_telescopes {A : Type} {Q : QSys A} {R : NSet A}
    {accepted : Nat → A → Prop} {chosen : Nat → Prop} {n : Nat}
    (hQ : Q R) (hord : AcceptsInOrder accepted)
    (hlast : ∀ a, R a → accepted n a)
    (hdecide : ∀ i, i ≤ n →
      Fuse.ChosenEvidence (fun _ => Q) accepted i → chosen i) :
    ∀ i, i ≤ n → chosen i :=
  Fuse.decide_all (fun _ => Q) R n accepted chosen hQ (fun _ _ h => h)
    (atomic_of_last hord hlast) hdecide

/-- Negative control: without in-order acceptance the last slot's acceptance
certifies nothing below it. The gapped responder accepted slot 1 but not slot
0, so atomic acceptance of the range fails even though the last slot is
unanimous. The accept path's next-slot guard is load-bearing. -/
theorem gap_breaks_atomicity :
    ¬ Fuse.AtomicAcceptance (fun i (_ : Bool) => i = 1) (fun a => a = true) 1 := by
  intro h
  exact absurd (h true rfl 0 (Nat.zero_le 1)) (by decide)

end CommitTelescoping
