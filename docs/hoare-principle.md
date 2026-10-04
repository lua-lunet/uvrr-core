# Hoare's principle: the invariant of a repetitive construct

This document states the invariant discipline for a repetitive construct in the
notation in which E. W. Dijkstra writes it on the whiteboard of his 1990
lecture [EWD-1990], and states what that discipline is called in this
repository's own induction idiom. The notation is Dijkstra's, as it stands on
the board; the principle is C. A. R. Hoare's.

## The notation

```text
{Q}   S   {R}          the general triple
{P}                    the side condition established before entry
do  B →
   {P ∧ B}   S   {P}   the body: the induction step
od {P ∧ ¬B}             the exit condition
```

Dijkstra calls this the general pattern used over and over again to prove
things about the repetitive construct. It is a pattern of assertions, not of
syntax: the body of the loop is an ordinary statement `S` with an assertion
about it, and every use of the pattern supplies its own `P`, `B` and `S`.

## The three statements

- `P` is the invariant: the property that holds before the repetition and is
  re-established by each execution of the body.
- `B` is the guard: the property that selects the iterations that are executed
  at all. The body runs only where `B` holds.
- `S` is the statement: the one step the iteration performs.

The assertion `{P ∧ B} S {P}` is the induction step. It is the very same
statement `S`, with `P` in its postcondition and `P ∧ B` in its
precondition, and that identity of statement between precondition and
postcondition is what makes the repetition one induction rather than a
sequence of unrelated steps. The assertion `od {P ∧ ¬B}` is the exit
condition: on leaving the loop the invariant still holds and the guard does
not.

## The three obligations

1. **Entry.** `P` holds before the first execution of the body. The side
   condition `{P}` is established once, before the `do`.
2. **Preservation.** The guard's guarantee: where `B` holds and `S` completes,
   `P` holds again. This obligation is `{P ∧ B} S {P}` and it is the only
   obligation about the body.
3. **Exit.** Termination is what supplies `¬B`. The exit condition is reached
   because the repetition ends, not because the invariant implies it: an
   invariant that holds at every step of a repetition is consistent with a
   repetition that never ends.

Safety and termination are separate obligations here, and the second is not a
corollary of the first. Dijkstra's exit condition names both halves for that
reason.

## The finite form: telescoping

Dijkstra's `do B → S od` is an unbounded repetition. This repository's
induction idiom is a finite telescoping over a configuration sequence, with a
named preservation step. The correspondence is one of shape, and it is stated
as a correspondence rather than an identity.

```text
E0 ──▶ E1 ──▶ E2
```

Here `E0`, `E1`, `E2` are the configurations of a reconfiguration schedule's
era sequence. The telescoping theorem is `Fuse.Telescope.telescope_pass`: in a
three-node cluster whose schedule has two transitions, `E0 → E1 → E2`, under
the fuse constraints — one datagram per batch, the atomic batch property, and
the fuse header ballot carrying both Phase 1 and Phase 2 for every command —
the first transition passing, the response set's quorum at `E0`, together
with the preservation step across `E0 → E1`, telescopes the remaining slots:
every slot `i ≤ 2` holds quorum-backed evidence.

Read against Dijkstra's pattern:

| Dijkstra | This repository |
|---|---|
| `P`, the invariant | the property carried by every configuration of the sequence |
| `B`, the guard | the premise under which the transition is admitted |
| `S`, the statement | one configuration transition |
| the body `{P ∧ B} S {P}` | the preservation step |
| `do B → S od` | the finite configuration sequence `E0 → E1 → E2` |
| termination supplying `¬B` | the final slot of the range |

## The preservation step

The preservation step is named, not inlined, and in the ladder it appears in
three shapes.

- **Across a reconfiguration boundary.** The preservation step across
  `E0 → E1` is a hypothesis of the telescoping theorem, not a corollary of the
  first transition. Quorum eligibility is preserved through the finite
  configuration sequence by `eligible_through`: a response set eligible at one
  era is eligible at the next under the preservation hypothesis.
- **The identity.** Under a fixed configuration — the stable-leader setting,
  no view change and no era change inside the range, one quorum family — the
  preservation step is the identity and the telescope degenerates to the
  commit direction. Per-node acceptance is downward closed, so a quorum's
  acceptance of the range's last slot is already its acceptance of every slot
  in the range (`atomic_of_last`); `last_acceptance_decides_all` and
  `commit_telescopes` then conclude that every slot holds quorum-backed chosen
  evidence and is decidable.
- **Over a monotone history.** The witness's accept history is the pairs
  `(view, accepted-prefix)` the stream delivered, and `Monotone` — views
  strictly increase — is the named premise under which `fence_of_monotone`
  carries the promise floor across the history.

The message induction, the transition induction and the strong induction of
the ladder all state their induction premises explicitly rather than assuming
them, which is Dijkstra's `{P ∧ B} S {P}` carried as a hypothesis instead of
discharged. `Fuse.Telescope.telescope_pass` has the same shape: the
preservation step sits in its hypotheses.

## The negative control

Dijkstra's notation has no negative control. This repository's idiom has one,
and it is load-bearing: a step shown to fail when its premise is removed is
what distinguishes a proved induction step from an assumed one.

- `swap_control`, the equal-total-swap control, shows that the preservation
  hypothesis is not free.
- `gap_breaks_atomicity` shows that the next-slot guard is load-bearing: a
  responder that accepted slot 1 while skipping slot 0 defeats atomicity even
  with the last slot unanimous. The guard is `m.slot = log.length + 1`, the
  condition the preservation step's conclusion rests on.

A preservation step with a negative control beside it is the finite form of
`{P ∧ B} S {P}` with the counterexample to dropping `B` attached.

## What the step delivers

The telescoped conclusion is per-slot evidence; agreement is what fixes the
values. Theorem 8 is agreement for a Synod over one fixed configuration, from
the six Synod invariants `S1`–`S6`. Theorem 10 is per-instance agreement under
reconfiguration: it is Theorem 8 instantiated on the per-instance projection of
the history, with `S1` supplied by Lemma 9 from the era constraints. The quorum
families' intersection — the frown `⌢` of [FROWN-2020] — enters as `P1`, and
the era family `E0, E1, E2, E2, …` satisfies `P1` at arbitrary scale
(`sequence_p1`), so Theorem 10 discharges agreement at every instance of the
family. A view may jump by more than one during a reconfiguration without
violating safety precisely because the safety requirements do not demand that
the views of commits advance uniformly, so Theorem 10's hypotheses never
quantify over intermediate ballots.

## What this correspondence is not

- **It is not an identity.** Dijkstra's loop is an unbounded repetition over
  an unknown number of executions, and its exit condition is supplied by
  termination. The telescoping is bounded: a named first slot, a named last
  slot, and a finite sequence of transitions between them. It discharges the
  shape of the inference over that horizon and says nothing about executions
  outside it.
- **It is not a program proof.** The predicate carried along is a predicate
  about a configuration — an era's quorum families, a log, a progress record —
  and not a property of a program's control state. The guard is a receiver
  guard on a message, not a branch condition chosen by the code.
- **It is not the weakest precondition.** The triple `{Q} S {R}` is a
  relational assertion about a statement. The weakest precondition of the same
  name is a predicate transformer for atomic commands
  (`research/glossary.md`), a different notion with a different shape. This
  document states no transformer and no composition law for one.