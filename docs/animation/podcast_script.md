# The uVRR Explainer: A Walk Up the Consistency Ladder

## Format

- **Narration**: one voice, female (Jane, British English), normal reading pace.
- **Form**: short clips, one per rung of the ladder; each clip is a bookmark.
- **Player**: one long-play scrubber with bookmark ticks; forward and back
  skip between bookmarks; the stage carries an informational 2D/2.5D SVG
  animation with short transitions between clips.
- **Narrative**: from one node and no resilience to the replicated protocol:
  VSR and consistency from 101 to done. No product pitch; the ladder is the
  product.

---

## Clip 1, One node: your data, one machine

**[Visual]**: a single server card centre-stage; data blocks stack on it; the
power cuts, the blocks scatter and fade; the request queue empties.

Start with the simplest thing that can possibly work: one machine, holding
your data, answering your requests. It works, until the day it doesn't. The
power fails, the disk dies, the kernel panics. In one moment, everything you
stored is gone, and every request goes unanswered. Resilience is not an
optimisation you add later; it is the first requirement. So we need more than
one machine. The question is: how many, and how do they agree?

## Clip 2, Two nodes: the split brain

**[Visual]**: two mirrored nodes joined by a wire; the wire breaks; each side
answers a different value to the same question; a brain glyph splits.

The obvious answer is two: keep a mirror, copy every write to both. But now
the two machines must agree, and the wire between them can break. When it
does, each side carries on alone, certain it is the whole system. Ask a
question, and you can get two different answers, two histories of the same
truth. This is the split brain, and it is the central problem of replication.
A mirror does not remove the failure; it duplicates the authority. Agreement
needs something stronger than copying.

## Clip 3, Three nodes: the majority

**[Visual]**: three nodes in a triangle; a vote lights two of three green;
backdrop shifts to a cloud map with three region pins, each with its own
power and network glyphs, joined by fast interconnect lines.

Add a third machine, and something remarkable appears: the majority. Any two
of the three overlap, so any decision that two nodes have made is a decision
the whole system must remember. One machine can die, and its memory is not
lost, it survives in the pair that remain. And the cloud was built for
exactly this shape: three regions, each with its own power and its own
networking, joined by fast interconnects. Three is not just a number; it is
the smallest system that can lose a part and keep its mind.

## Clip 4, The arithmetic: 2F+1

**[Visual]**: a formula panel builds 2F+1; F=1 fills three node dots, F=2
fills five; a minority side goes grey and the system pauses, refusing to
answer is safe.

Here is the arithmetic that governs every replication system. To tolerate F
failed machines, you need two F plus one: with five nodes you can lose two;
with three, you can lose one. A majority must always survive, because the
majority is the memory of the system, the place where every committed
decision lives. Fewer than a majority, and the system must stop and wait:
refusing to answer is safe; answering from a stale memory is not.

## Clip 5, Five, then back to three

**[Visual]**: five node dots; two fade out, the service bar never dips; the
five collapse into the three-node triangle used for the rest of the walk.

So use five, and lose any two, the service never notices. But five is a lot
of machines to watch in an explainer, so from here on we draw three, and
everything we show scales by the same arithmetic. Three nodes, one majority,
one shared truth. Now: how do the three actually work together? That is the
protocol, Viewstamped Replication.

## Clip 6, Views and the primary

**[Visual]**: the triangle returns; a crown settles on one node; the view
counter ticks v to v plus one when the crown moves.

Viewstamped Replication organises time into views. In each view, one node is
the primary, the leader, and the others are backups. The primary decides
the order of every operation; the backups follow. There is no lock, no lease
file on disk: leadership is a fact the majority holds in memory, and it is
numbered by the view. When the view changes, the leadership can move,
cleanly, and by agreement.

## Clip 7, Normal operation: two round trips, no disk

**[Visual]**: a request dot travels client to primary; prepare fans out to
the backups; acknowledgements return; commit fans out; the log fills in
order; a disk glyph appears struck through.

A client sends a request to the primary. The primary appends it to the log
and sends a prepare to the backups. Each backup appends and acknowledges,
and when the majority, the primary plus the backups, has answered, the
operation is committed: the primary sends the commit, and every node applies
it in order. Two network round trips, and not one disk write on the path.
The speed of the protocol comes from what it refuses to do: it does not wait
for a disk; it trusts the majority's memory.

## Clip 8, Failover: the view change

**[Visual]**: the primary greys out; suspicion ripples widen from the
survivors; view-change arrows cross between them; the crown moves to a
survivor; the view ticks up; the service bar resumes.

Now the primary itself dies. The backups stop hearing its heartbeat, and
suspicion accrues. The surviving pair, still a majority, runs a view
change: they exchange what they know, elect a new primary, and install the
next view. Because the pair overlaps every past majority, nothing committed
is forgotten; the new primary continues the log exactly where the old one
left it. The failover costs round trips, not disk flushes, the memory that
matters was never on one machine.
