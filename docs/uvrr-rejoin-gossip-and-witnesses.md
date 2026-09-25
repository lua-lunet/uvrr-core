# uVRR rejoin gossip and witnesses

## 1. Model statement

Rejoining is a **gossip protocol outside the main uVRR protocol**. A node that
is not part of the cluster never assumes the cluster will come to it: it
gossips to find the cluster, and the cluster streams it until it is promoted.
The gossip layer exists so that the reincarnated node arrives at its first
fence at the cluster's current era, having seen a commit, which makes the
engine's one-era catch-up window (`uvrr-reincarnation.md` §10) unreachable by
construction. The main protocol is unchanged: the gossip layer owns era
discovery and witness streaming; the engine still owns fencing, voting, and
the superblock lifecycle.

## 2. The join gossip (joiner side)

- A node outside the cluster, crash-restarted, reincarnated, or cold,
  gossips "I want to join" to **every node it knows about**, including its
  **frontiers**. The gossip runs under the deployment's authenticated
  transport (paxe with TLS 1.3 PSK), so every identity a sysadmin configured
  is known; the protocol does not depend on that beyond ordinary
  authentication.
- Any node that hears a join gossip responds with what it knows: the current
  era and the cluster configuration.
- The rejoining node adopts the max era it hears, takes the configuration
  from a responder, and **flushes the configuration to disk as JSON whenever
  it increments the era**. After a crash it comes back blank at era 0 and
  re-adopts from the responses, it never comes back at a remembered era.
- The rejoining node **never assumes it has heard the latest era**. It may
  straddle a partial network partition and hear an isolated node's newer
  configuration alongside a stale era. It therefore keeps its own resend
  timer and always sends all gossip messages to all nodes it has learned are
  in the cluster.
- When the gossip delivers a piggybacked commit, the node has seen leader
  activity and starts its phi leader-timeout baseline from it.

## 3. The gossip-witness list (all nodes, leader streams)

- **Every** node that hears a join gossip adds the sender to its
  **gossip-witness list**. Only the leader acts on the list: it replies
  and, based on the sender's frontiers, pushes all the phase-2 messages
  the sender needs to catch up, then a commit message.
- From then on the leader pushes **all phase-2s and all commits** to the
  gossip-witness list, as if those nodes were part of the cluster. The
  witness is a passive data sink outside the roster: it never votes, and its
  votes are refused by the standard membership checks
  (`uvrr-reincarnation.md` §6).
- A joining node stays on the leader's witness list **until its membership is
  committed**: the drop on a committed JOIN (below) fires at the commit and
  never earlier, so a node is never served under two roles at once. During a
  join the leader owes the host the at-least-once obligation: every slot's
  traffic reaches each target at least once, and the last once may arrive
  more than once. The sender may deduplicate on the triple
  (slot, target, type). The view needs no ledger of its own: a leader that
  loses leadership crashes and returns under a new identity, so every resend
  ledger lives and dies inside one identity's leadership.
- Because the witness holds a live commit stream, it stays current
  **indefinitely**. Two worked scenarios show what this buys across a heal
  (§3.1).
- When a reconfiguration carrying a JOIN commits, **every** node scans its
  gossip-witness list and drops the promoted node. Sending to the node as
  both voter and witness is safe, duplicate delivery is idempotent, but
  wastes IO; dropping is a dedup optimisation, not a safety rule.
- Because every node keeps the list, **failover does not interrupt the
  stream**: the successor leader already carries the joiner in its own
  gossip-witness list and starts streaming on election. The joiner's
  resend timer covers a gossip lost in flight, it is the reliability
  mechanism of the gossip itself, not a failover re-discovery path.
- In this manner **any node outside a cluster can gossip to find the leader
  during failovers**: tracking the cluster through the witness stream
  replaces a lucky first message.

### 3.1 Worked scenarios

**Three nodes: stream through the heal.** Take `n1, n2(l), n3` with `n1`
isolated and `n3` crashed. `n3` gossips its desire to join to `n2`, which
streams phase-2 messages to it, yet nothing commits, because no majority
responds. The network heals: the phase-2 messages hit `n1`, which acks and
requests retransmission of the messages it did not get. `n2` gains a majority
response for a high slot but still has a gap, so it cannot commit; then `n1`
catches up and responds to all slots, and `n2` commits in slot order and
sends the commit to both nodes. Because `n3` gossiped that it wanted to join,
`n2` computed the JOIN cluster commands as the next values to be chosen,
those were streamed too, so upon contiguous commits in slot order `n3` has
joined the cluster and leaves the leader's witness list.

**Five nodes: the isolated leader loses the quorum.** Take a five-node
cluster where the leader sits in a minority partition together with the
reincarnated node: the leader plays keep-up, streaming to the witness as
usual. The leader crashes. The network heals; the three surviving nodes form
a quorum and a new leader emerges. What happens to the reincarnated node?
Every surviving node heard its original gossip, so the new leader already
carries it in its gossip-witness list: on election it streams the messages
the surviving majority had committed. These may override the old leader's
uncommitted slots, and they add in and commit the new join attempt at
slots chosen by the new leader, the witness arrives at the new leader
already at the committed frontier and the join completes under the
configuration the quorum chose. The joiner's resend timer keeps gossiping
regardless: a node that missed the original gossip learns of the joiner
from a resend.

## 4. Witnesses as a first-class configuration role

- The startup cluster configuration carries a **witness list** in addition
  to the member list: regular members **plus witnesses**, where a witness
  cannot also be a member. Startup witnesses are the pre-provisioned form,
  e.g. out-of-region cold-backup nodes. Every node loads the list at
  startup and a statically registered witness is **never purged**: as the
  cluster fails over, each leader in turn streams to the full DR backup.
- The join method takes a **join type: `standard | witness`**.
- On a committed JOIN of a `standard` joiner, every node drops it from the
  gossip-witness list (§3): it is now a voting member and receives protocol
  traffic as one.
- On a committed **witness** join the list is **not** cleared. The purpose
  of the commit broadcast is so that **every node adds the witness to its own
  list**, so when failover to a new leader happens, the new leader forwards
  to the witnesses.
- A witness is therefore a durable role in the configuration, replicated by
  the ordinary commit path, not a leader-local memo.

## 5. Relationship to learner acquisition

`uvrr-reincarnation.md` §10 specifies weight-0 learner acquisition inside the
engine: a learner streams while never voting, and the 0→1 promotion finds it
caught up. The gossip layer is the outer complement to that mechanism:

- The **"find the cluster" guard** wraps the engine on the host side. In
  blank mode it accepts any inbound era greater than its known era, sets its
  own era, takes the cluster membership, and flushes it to disk if it
  differs from what it last sent. The engine beneath the guard still runs
  the fenced acquisition path of §10.
- The **witness stream** is the leader-side send-set extension: one stream,
  all phase-2s and commits, serves weight-0 members mid-acquisition,
  persistent witnesses, and a future DR service alike. A DR sink in another
  region is just a witness whose apply log is an AOF tape. The catchup
  traffic is gossip-like, request and response pairs outside the voting
  protocol, and it inherits the at-least-once and dedup obligations of §3.
