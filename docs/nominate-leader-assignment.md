# NOMINATE leader assignment across reconfiguration

The leader of a view is `voters[view % voters]` under the view's era
configuration (`Configuration::primary`, §1.2, §8.4): the primary is a
ballot outcome, not a function of membership. A reconfiguration that
changes the positive-weight sequence moves the arithmetic mid-plan, so no
node can independently compute a stable leader through the transition.
This document states the NOMINATE letter that settles the assignment: a
view increment carried as a compare-and-swap at a committed slot, so the
leader the old arithmetic elected is the leader the new arithmetic
re-elects, at the very slot the arithmetic moves.

`docs/weighted-reconfiguration-solver.md` states the plans; this document
states the nomination that rides them. The safety licence for the jump is
the view-jump lemma, `formal/uvrr-lean/UVRR/ViewJump.lean`: a view may
jump by more than one during a reconfiguration and agreement is
unaffected, because no hypothesis of the agreement argument quantifies
over intermediate ballots; eras may not jump, the rule's adjacency
discipline; the only risk of a jump is exhaustion of the view space,
bounded code-side by the CAS delta.

## The case semantics at a moving slot

`SystemOperation::Nominate { from: View, offset: u32 }` is a letter of
the §8.7.2 alphabet. `from` names the view number the proposing leader
believes the cluster holds; `offset` names the increment `u`, strictly
positive. The command rides the committed log like every system
operation: a slot is committed, it happens everywhere, and every node
applies the same command at the same slot.

The CAS pair is the serving state and the view number. A node applying a
nomination at commit publishes the bumped view
`ViewId { era: the era the covering fold established, view: from + offset }`
as its current view, on the same published transition that carries the
folded table, if and only if it is `Normal`
and its published view number equals `from`. The bump advances the serving
view by a committed command and re-selects no history: the retained view,
the retained history's provenance, stays what it was, and a `Normal` node
serves at or past its retained view. The bumped view's era is
the era the nomination's own establishing run established, so the bump
enters the new arithmetic directly: the leader under the new
configuration is `record(era).primary(from + offset)`, the same node the
old arithmetic elected. The era component is never a claim the message
carries: it is the fold's own answer, and the §8.7.3 accept-time
relation already bounds it, an entry two eras past a node's view era is
refused at accept, so no committed nomination can name a two-era jump.

The nomination issues no `StartViewChange`, no fence, no evidence, no
install: the bump is not a view change, it is a commit. At the wrap the
bump IS the era-boundary crossing: the view's era must equal the
established era before the next establishing operation is proposed
(§8.7.8's gate, `PlanRefusal::EraTransitionOutstanding`), and the bump
enters it on the same published transition that carries the folded table,
so the wrap's boundary needs no view-change message at all. A node
not serving the named view, fenced, electing, replaying, or simply past
`from`, is outside the nomination's authority and does not bump; its own
path, the fence, the install, or the acquisition, sets its view. Install
transitions supersede the nomination outright: a `StartView` or NewState
install that covers a nomination carries its own view authority, and a
node whose election carried an old leader's nomination commits it as a
no-op, the cluster has moved on. Several nominations inside one covered
advance apply in slot order, each CAS predicated on the running number
the prior bumps of the same advance produced.

## The fold's seats

The nomination moves no mass and no membership. Folded in era it yields
the identical configuration, so inside a `SystemOperation::Batch` it is
a zero-mass rider under R14, and the seat it takes is the last
sub-operation of every establishing batch that carries no scaling
operation: the rider shares the step's own slot, so the bump lands
exactly when the step's era is established. A solitary `Nominate` entry
is refused by the fold with
`ConfigError::SolitaryNomination`, a nomination never establishes an era
of its own. A zero offset is refused with
`ConfigError::ZeroNominationOffset`, the jump strictly increases
(ViewJump's rule). A batch of one nomination is a legal zero-mass batch,
a rotation instrument the solver does not use.

R13's solitude reserves the scaling eras: `Double` and `Halve` preserve
the positive-weight sequence elementwise, a positive weight stays
positive and a zero stays zero, and they touch no order, so the wrap
never coincides with a scaling operation and the rider never needs a
scaling seat. `Join` and `Leave` at weight zero insert and remove
learners only, the voter subsequence is unchanged. The wrap is exactly a
sub-operation that moves a member across weight zero: an `Increment`
from 0, or a `Decrement` to 0.

## The solver's emission

`solve` and `solve_replacement` take the serving view, the view number
the leader holds when the plan begins, and name it in every emitted
nomination. The emission pass runs over the computed steps: walking with
the running view `v`, the view the parameter names plus the advances of
the steps already walked, and the previous step's configuration, every
step whose batch carries no scaling operation gains a
`Nominate { from: v, offset: u }` as its last sub-operation, `u` the
least positive offset with `primary(next, v + u) == primary(previous, v)`.
The rider's bump is the era entry the §8.7.8 gate demands: the
re-electing offset across a wrap that would move the leader, and the
count, the least offset that preserves the index, at a step that keeps
it, which is the same view the machinery's own §13.4 selector names.

A scaling step carries no rider: R13 keeps `Double` and `Halve`
solitary, and the scaling preserves the positive-weight sequence
elementwise, so the leader never moves at one. That step's era boundary
is crossed by the leader-preserving view change the host drives (§14.2),
the least view past the running one selecting the constant leader, the
same number `u` the rider would have named, so the chained views agree
whichever carries the boundary. The running view advances by `u` at
every step; the chained `from` values are the views the cluster actually
holds, each rider's bump landing at its step's commit. A step that
evicts the serving leader from the voter sequence cannot keep it: no
offset re-elects the evicted, the rider names the era-entering increment
alone, and the leadership passes to the arithmetic's choice; the next
step's emission continues from the leader the new arithmetic names.

The forced-reincarnation machine's own runtime recomputation
(`replica::forced_steps`) emits no nominations: its §6 schedule is
crash-restart idempotent and its leadership is the fence machinery's
business. `solve_replacement` prefers that schedule and the emission pass
wraps it, so the solver's plans carry the nominations and the machine's
ticks do not. The operator tool's `plan` takes the serving view as it
takes the availability snapshot.

The routes the emission wraps are the common-prefix routes: the longest
prefix the two orders share, member and weight alike, stays put; the
current's tail drains and leaves; and the target's tail joins and takes
its weights, each live joiner promoted at its join (the expansion's
shape: join at weight zero, increment, join, increment), the dead
joiners' weights restored last. The prefix-only intermediate must itself
be a legal, available configuration; a prefix whose mass cannot hold a
majority falls back to the anchor route, which reserves one live voter
and is always available. The prefix retention keeps the voter count at
two or more through every prefix the ladder drives, so the establishing
rounds always have a voter to acknowledge them: the anchor route's
drained intermediates, one voter and then one member, are the standing
question's and the host's business, not the plan's.

The standing question (§4, R4): a weight-zero member's acknowledgement
counts against no quorum and is not recorded, but its arrival still asks
the leader whether its own implicit vote and the recorded
acknowledgements already commit anything outstanding. A drained
intermediate's sole voter is its own commit quorum, so the zero-mass
steps at one-voter eras commit without another voter's vote; at any era
where the recorded votes hold no quorum the question changes nothing.

## The invariants

* **The constant leader through the wrap.** At every committed slot, every
  live member whose own committed configuration gives it positive weight
  computes the same leader: the configuration its committed history has
  established, evaluated at its current view number, names the node the
  plan started under, at every step of every scenario.
* **Era adjacency.** The bump's era is the establishing run's era, exactly
  one past the receiver's pre-fold row, and the accept-time §8.7.3
  relation enforces the bound before the commit ever sees the entry.
* **Weight-zero joins cannot vote.** The nomination moves no weights, and
  the §8.4 learner stays what it was: invisible to every majority and
  never the primary.

## The failure modes ruled out

* **Leader bounce.** The rider shares the wrap's establishing slot: there
  is no era between the old arithmetic and the bump in which the modulo
  rule would name the wrong leader, and no era after it in which the bump
  has not yet landed.
* **Era ambiguity.** The CAS predicate is the view number the sender
  believes the cluster holds, never a configuration-era claim; the bump's
  era component is the fold's own answer. No absolute era is committed at
  a slot a prior leader chose: the nomination's authority is the CAS, and
  a cluster that has moved on makes the commit a no-op.
* **Crash mid-transition.** A new leader committing an old leader's
  nomination finds the view moved, the CAS fails, and nothing bumps. An
  isolated node that bumped on its own is converged by later messages,
  the ordinary exchange; an overflowing `from + offset` is inert, the view
  space's exhaustion is the nomination's own dead end, never the node's.

## The force-feed test ladder

`tests/nominate_leader_assignment.rs` is the proof carrier. The driver,
per scenario: provision and bootstrap; force the cluster to the scenario's
serving view with the host-forced view change (§14.2), whose primary is
the scenario's constant leader; then for each step of the solver's plan,
the leader proposes the step as ONE establishing operation,
`SystemOperation::Batch(step.ops)`, through the ordinary §8.7.4 pipeline,
one entry at one slot, never a `Fuse` envelope, and the script feeds every
released message to its addressee until the network is empty; after each
step's quiesce the ladder asserts the constant leader at every live voter
(the node's own committed configuration, evaluated at its current view
number) and the cluster safety checker runs; and when the view lags the
established era after a step whose batch carried a scaling operation, the
script drives the era-boundary view change (§14.2) to the
leader-preserving view, the least view past the current one selecting the
constant leader under the era's voters, the same number the rider the
step would have carried names, and the plan continues.

The scenarios, in order:

1. **Expansion 3 to 5** at serving view 3, leader `n(0)`: `solve` from the
   genesis `(1, 1, 1)` to `(1, 1, 1, 1, 1)`. The common-prefix route keeps
   the genesis membership as the target's prefix and appends: join at
   weight zero, increment, second join, second increment, the promotions
   the wraps that carry the re-electing riders; four steps, four riders,
   the plan ends at view 15 with the unit-weight five.
2. **The three-node replacement** at serving view 3, leader `n(0)`:
   `solve` from `(n0, n1, n2)` to `(n0, n1, n3)`. The common-prefix
   route keeps the shared prefix, drains and leaves the old tail, joins
   and promotes the fresh one, the drain and the promotion the moving
   wraps; four steps, four riders, the plan ends at view 9 with the old
   identity evicted and the fresh one seated at unit weight.
3. **The five-node crash-reincarnation replace** at serving view 5,
   leader `n(0)`: crash `n(4)`, reopen its bumped life, and
   `solve_replacement` over the six-era forced schedule, `Double`, the
   join and promotion of the bumped identity, the drain of the old, its
   departure, the promotion, and `Halve`. The four unit-mass steps carry
   their riders, the moving wraps (the bumped identity's promotion,
   voters 5 to 6, and the old identity's drain to zero, voters 6 to 5)
   re-electing; the scaling eras' boundaries are the leader-preserving
   view changes; the plan ends at view 30 with the unit-weight five, the
   bumped identity in the old seat.

The Red rung is the ladder run against the un-nominated solver, and it
fails twice over: the assertion fails at the first wrap's committed slot,
the modulo rule names a different node at the unchanged view number (the
expansion and the replacement both name the drained member's successor at
view 3), and the plan cannot even cross its era boundaries, the §8.7.8
gate refuses the second establishing operation while the view lags the
established era (`EraTransitionOutstanding`, the face the
crash-reincarnation scenario reaches first, at its second step). The
emission pass and the commit-time bump make the same ladder green, the
constant leader at every committed slot of all three scenarios. The
bumped life's own seating is the §10 acquisition, the rejoin path's
business, and is not the ladder's assertion: the scenario asserts the
plan's commits and the leader's constancy at the serving voters.
