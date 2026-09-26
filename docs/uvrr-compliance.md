# The uVRR compliance suite

This document states the compliance suite: the executable form of the
protocol's obligations, runnable by any conforming harness in any
language. The suite is a corpus of test cases stated as data; a harness
conforms by implementing the abstract host interface, running the corpus,
and passing every case. The Rust reference runner
(`tests/compliance.rs`) does exactly this through the crate's public
sans-I/O interface and is part of the repository's gates; nothing in the
corpus is expressed in Rust, so a Zig, Lua, or any other implementation
runs the same files over its own bindings.

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
- an abstract host operation (a proposal, a crash, a restart, a gossip,
  a shutdown).

The outputs are the datagrams the cluster emits in response, in the same
encoding, and the observable post-state: the status, the frontiers, the
journal, the marker schedule, the witness lists. A conforming harness
MUST produce the exact outputs the case names; where a case names
alternatives, the harness MUST produce one of them.

## 2. The abstract host interface

A conforming harness MUST provide these operations. They are the
sans-I/O boundary the crate already defines, mirrored as data:

| operation | meaning |
|---|---|
| `provision` | Build a genesis cluster: `nodes` members, each with the deterministic lawful identity system half = roster position + 1, counter = 1. |
| `settle` | Drive the deterministic bootstrap: repeat tick-all and deliver-all until the network is quiet and every node is `Normal`. MUST be implemented as a bounded, deterministic loop; no wall-clock waits. |
| `propose` | Submit one operation for ordering from the named node. |
| `tick` | Deliver one host timer event to the named node. |
| `tick_all` | One timer event to every live node. |
| `deliver` | Deliver one wire datagram to the named node, attributed to the named sender. |
| `deliver_all` | Drain the network once, delivering every queued datagram. |
| `crash` | Kill the named node: volatile state is lost, the durable markers remain as they are. |
| `restart` | Reopen the named node over its recorded disk: `clean` continues the identity, `crashed` bumps to the next life. |
| `halt` | The controlled shutdown of the named node: both marker rounds with the drain between. |
| `gossip` | The named outside node gossips its desire to join to every live node. |

Every operation MUST be deterministic. A case's setup script is a list
of these operations; the replay MUST reach the same state on every run.

## 3. The case record

A case is one JSON object:

```json
{
  "id": "prepare-backup-normal-view-equal-slot-equal",
  "family": "prepare",
  "clause": "A backup in the current view MUST accept a Prepare whose slot is past its accepted frontier and journal the entry",
  "keywords": ["MUST"],
  "setup": [
    {"op": "provision", "nodes": 3},
    {"op": "settle"}
  ],
  "input": {"op": "deliver", "to": 1, "from": 0, "wire": "0000000d..."},
  "expect": {
    "deliveries": [{"from": 1, "to": 0, "wire": "0000000e..."}],
    "post": [
      {"node": 1, "status": "Normal", "accepted": 3, "committed": 3}
    ]
  }
}
```

- `id` MUST be stable and unique; it names the case in every report.
- `clause` states the obligation in RFC 2119 terms; `keywords` carries
  the normative keywords the case enforces.
- `setup` is the state-building script over the host interface.
- `input` is the single transition under test.
- `expect.deliveries` is the exact multiset of datagrams delivered
  while settling after the input: each named datagram MUST be delivered
  exactly once, and no further datagram MUST be delivered. An empty list
  asserts silence.
- `expect.post` is the per-node post-state: `status`, the `accepted`,
  `committed`, `applied` frontiers as slot numbers, the journal payloads
  in slot order, the marker schedule (the gate's operation log), and the
  gossip-witness list. Every named field MUST match; unnamed fields are
  unconstrained.

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

- **Fence posts.** A boot-fenced node (a `Restarting` or `Joining`
  status) MUST NOT vote, MUST NOT drive a view change, and MUST emit no
  datagram on ticks; a fabricated vote from a non-member MUST be
  discarded by name.
- **The controlled shutdown.** The halt MUST write `Stopping` to every
  marker copy, force the drain strictly between the rounds, and write
  `Stopped` to every copy; the gate's operation log MUST show the two
  rounds with the drain between, and the next clean boot MUST continue
  the same identity.
- **The crash shape.** A crash MUST leave the markers as they are; the
  next boot MUST read no stopped quorum, bump the identity exactly one
  life, and write `Joining`; nothing MUST self-reset to zero.
- **Gossip and witnesses.** Every node that hears a join gossip MUST add
  the sender to its gossip-witness list; the leader MUST stream to the
  witness; a committed join MUST drop the promoted node from the list.

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
files; a codec change that moves a wire byte is a corpus change, visibly.

## 7. Register

The corpus files live in `tests/compliance/corpus/`, one file per
family. The reference runner reads every file and executes every case;
a failure reports the case `id`, the clause, and the first mismatch.
The suite grows by families; a family MUST state its clauses before its
cases are generated.
