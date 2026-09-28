# The uVRR compliance suite

This document states the compliance suite: the executable form of the
protocol's obligations, runnable by any conforming harness in any
language. The suite is a corpus of test cases stated as data; a harness
conforms by implementing the abstract host interface, running the corpus,
and passing every case. The Rust reference runner (`tests/compliance.rs`)
does exactly this through the crate's public sans-I/O interface and is
part of the repository's gates; nothing in the corpus is expressed in
Rust, so a Zig, Lua, or any other implementation runs the same files over
its own bindings.

The key words MUST, MUST NOT, SHOULD, and MAY are to be interpreted as
described in RFC 2119 when they appear in this document or in a case's
clause.

## 1. The transition function

A case states one transition of the protocol:

```text
state x input -> state' + outputs
```

The input is one of:

- a protocol datagram, in its normative wire encoding (the fixed-width
  big-endian codec of the wire module), hex-encoded;
- a host timer event (a tick);
- an abstract host operation (a proposal, a reconfiguration, a crash, a
  restart, an announcement, a gossip, a shutdown).

The outputs are emissions on two channels:

- **the wire channel**: the datagrams the cluster emits in response, in
  the same encoding, delivered until the network is quiet; and
- **the storage channel**: the durable emissions to the boot-gate
  marker store, asserted semantically, never as bytes — the marker
  states written, and the drain the halt schedule forces between its
  rounds.

The observable post-state carries the status, the era and view, the
frontiers, the journal, the configuration, the marker schedule, and the
witness lists. A conforming harness MUST produce the exact outputs the
case names; where a case names alternatives, the harness MUST produce
one of them.

Boot-up fence matters are emissions ordering: the storage emission of
the bumped pair precedes the first wire emission from the reincarnated
node. The corpus's host interface carries the order structurally — the
`restart` of a crashed classification decides the pair, and the
`announce` that emits on the wire exists only after it — and the marker
schedule pins the deferral that follows: the durable `Joining` round at
the bumped identity is deferred to the seated witness, so a crash
between the announcement and the seat re-reads the old markers and
re-decides the same pair.

## 2. The abstract host interface

A conforming harness MUST provide these operations. They are the
sans-I/O boundary the crate already defines, mirrored as data:

| operation | meaning |
|---|---|
| `provision` | Build a genesis cluster: `nodes` members, each with the deterministic lawful identity half = roster position + 1, counter = 1. |
| `settle` | Drive the deterministic bootstrap: repeat tick-all and deliver-all until the network is quiet and every node is `Normal`. MUST be implemented as a bounded, deterministic loop; no wall-clock waits. |
| `propose` | Submit one operation for ordering from the named node. |
| `reconfigure` | Submit one typed cluster operation for ordering from the named node, over the ordinary consensus pipeline. |
| `tick` | Deliver one host timer event to the named node. |
| `tick_all` | One timer event to every live node. |
| `deliver` | Deliver one wire datagram to the named node, attributed to the named sender. |
| `deliver_all` | Drain the network once, delivering every queued datagram. |
| `crash` | Kill the named node: volatile state is lost, the durable markers remain as they are. |
| `restart` | Reopen the named node over its recorded disk: `clean` continues the identity, `crashed` bumps to the next life. |
| `halt` | The controlled shutdown of the named node: both marker rounds with the drain between. |
| `boot` | An outside identity boots over the deployment's genesis knowledge, as a joining member. |
| `announce` | The reincarnated node announces its replacement pair to the cluster. |
| `gossip` | The outside node gossips to every live node: the datagram is attributed to the roster's one-past-the-end identity. |

Every operation MUST be deterministic. A case's setup script is a list
of these operations; the replay MUST reach the same state on every run.

## 3. The case record

A case is one JSON object:

```json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A backup in the current view MUST accept a Prepare whose slot is the successor of its accepted frontier and journal the entry",
  "keywords": ["MUST"],
  "setup": [
    {"op": "provision", "nodes": 3, "timeout": 3},
    {"op": "settle"}
  ],
  "input": {"op": "deliver", "to": "3:1", "from": "2:1", "wire": "0000000d..."},
  "expect": {
    "deliveries": [{"from": "3:1", "to": "2:1", "wire": "0000000e..."}],
    "post": [
      {"node": "3:1", "status": "Normal", "era": 1, "view": 1, "accepted": 3, "committed": 3}
    ]
  }
}
```

- `id` MUST be stable and unique; it names the case in every report.
- `theorem` names the Lean theorem the case pins
  (`formal/uvrr-lean/`): the corpus and the formalization share one
  vocabulary, and a family's cases name the theorems of the rung the
  family is seeded from.
- `clause` states the obligation in RFC 2119 terms; `keywords` carries
  the normative keywords the case enforces.
- `setup` is the state-building script over the host interface.
- `input` is the single transition under test.
- `expect.deliveries` is the exact multiset of datagrams delivered
  while settling after the input: each named datagram MUST be delivered
  exactly once, and no further datagram MUST be delivered. An empty list
  asserts silence. Traffic to a down node is delivered and recorded
  undeliverable; it stays named in the list.
- `expect.post` is the per-node post-state. Every named field MUST
  match; unnamed fields are unconstrained. The fields:

| field | meaning |
|---|---|
| `node` | The node's identity, as the explicit pair `system:counter`. The roster members of a provisioned cluster are `1:1` through `nodes:1`; a bumped life is `system:2` and onward; the gossip sender is `nodes+1:1`. |
| `status` | One of `Normal`, `ViewChange`, `Restarting`, `Replaying`, `Joining`; `null` when the node is down. |
| `era`, `view` | The node's current era and view. |
| `accepted`, `committed`, `applied` | The frontiers, as slot numbers. |
| `journal` | The journal's entries in slot order, each rendered by the grammar below. |
| `members`, `weights` | The configuration the node holds: the succession order as identities, and each member's weight in the same order. |
| `markers` | The marker schedule: each marker round the boot gate wrote, as `Marker@system:counter` with `Marker` one of `Stopping`, `Stopped`, `Restarting`, `Joining`, interleaved with the `drain` the halt schedule forces between its rounds. |
| `witnesses` | The node's gossip-witness list, as identities. |

The journal grammar states each entry's payload as abstract text: a
host operation renders as its UTF-8 payload text; a typed cluster
operation renders as its abstract operation name with identity
arguments as `system:counter` pairs — `void`, `init order=1:1,2:1,3:1`,
`increment 2:1`, `decrement 2:1`, `double`, `halve`, `join 4:1 at 3`,
`leave 4:1`, and `batch [<op>, <op>]` for a batch's sub-operations.
Rust debug formatting MUST NOT appear in journal content.

The `reconfigure` operation's typed argument uses the same abstract
operation names: `{"kind": "increment", "node": "2:1"}`, and likewise
`decrement`, `double`, `halve`, `join` (with `position`), `leave`, and
`batch`.

## 4. Vector identities

The corpus pins fixed lawful identities (the deterministic provision
identities) inside its wire bytes, exactly as published test vectors pin
fixed keys and nonces. The fixed identity is part of the fixture, not a
claim that these values are privileged: the never-zero, never-recycled,
durable-before-emission identity law is enforced separately by the
mint-based invariant corpus, which draws random lawful identities per
run. A conforming implementation MUST NOT special-case the vector
identities; it MUST treat them as ordinary identities.

## 5. Abstract host tests

Beyond per-message protocol cases, the corpus pins the host
obligations:

- **Fence posts.** A reopened node outside every configuration it can
  name — the reincarnated standby — MUST NOT vote, MUST NOT drive a
  view change, and MUST emit no datagram on ticks; a cleanly restarted
  member ticks the full protocol and suspects a silent primary like any
  member; a fabricated vote from a non-member MUST be discarded by
  name.
- **The controlled shutdown.** The halt MUST write `Stopping` to every
  marker copy, force the drain strictly between the rounds, and write
  `Stopped` to every copy; the marker schedule MUST show the two rounds
  with the drain between, and the next clean boot MUST continue the
  same identity.
- **The crash shape.** A crash MUST leave the markers as they are; the
  next boot MUST read no stopped quorum, bump the identity exactly one
  life, and reopen fenced; the durable `Joining` round at the bumped
  pair defers to the seated witness, and nothing MUST self-reset to
  zero.
- **Gossip and witnesses.** Every node that hears a join gossip MUST
  add the sender to its gossip-witness list, and the list MUST never
  name a voter; the leader MUST stream to the witness; a committed join
  MUST drop the promoted node from the list.

## 6. Conformance

An implementation conforms to the uVRR protocol when:

1. it implements the abstract host interface over its own bindings,
   including the normative wire codec;
2. it runs every case in the corpus and every named expectation passes;
3. it adds no behaviour the corpus forbids (emissions beyond the named
   multiset fail the case).

The corpus is generated from the reference implementation's own codec
and enumeration and reviewed as data, the same discipline as published
test vectors: the bytes are the mechanical consequence of the clause,
and the clause is the norm. The corpus is regenerated by the exporter
(`cargo test --all-features --test compliance -- --ignored
export_corpus`) and the regenerated files MUST equal the committed
files; a codec change that moves a wire byte is a corpus change,
visibly. The export and sync gate runs in CI: the runner replays the
committed corpus, and the regenerator rebuilds it in memory and
compares, so a drifted corpus fails the gate without writing.

## 7. Register

The corpus files live in `tests/compliance/corpus/`, one file per
family. The reference runner reads every file and executes every case;
a failure reports the case `id`, the clause, and the first mismatch.
The suite grows by families; a family MUST state its clauses before its
cases are generated.

The families are seeded per Lean rung, and each case names its theorem:

| family | rung | seed |
|---|---|---|
| `prepare-accept` | `Acceptor.lean` | The `Prepare` a backup receives, over the cross product of its dimensions. |
| `view-selection` | `ViewSelection.lean` | The failover's evidence collection and selection. |
| `agreement` | `Synod.lean` | A proposal's identical commit everywhere. |
| `casting-vote` | `CastingVote.lean` | A weighted member inside every majority. |
| `reconfiguration` | `Eras.lean` | The establishing commit and the weight-0 join. |
| `reincarnation-safety` | `ReincarnationGeneral.lean` | The forced sequence of a bumped life. |
| `fuse` | `Fuse.lean` | The fused batch's atomic accept and refusal. |
| `identity` | `IdentityLaw.lean` | The durable pair: the bump and the wire packing. |
| `witness` | `Witness.lean` | The gossip-witness list and the leader's stream. |
| `boot-gate` | the phase machine of `ReincarnationGeneral.lean` and `IdentityLaw.lean` | The marker schedules and the fence posts. |

One input is out of the normative corpus and recorded as reference
behaviour: the `Prepare` whose piggybacked commit frontier claims the
arriving slot itself. No conforming primary emits it — the leader
commits the slot only after a quorum accepts it, and while transmitting
the `Prepare` it cannot simultaneously claim that commit — so the corpus
carries no clause for it; the reference refusal (`JournalEntryUnavailable`
at the arriving slot) is pinned by the mint-based exhaustive suite,
`tests/exhaustive_prepare.rs`'s `PiggyRefused` route.
