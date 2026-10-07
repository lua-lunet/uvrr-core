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

### 6.1 Regenerating the generated artefacts

Two committed file sets are generated from the reference behaviour, and
they are regenerated in that order, because the second is a function of
the first:

| artefact | directory | command |
|---|---|---|
| the corpus | `tests/compliance/corpus/` | `make corpus-export`, which runs `cargo test --all-features --test compliance -- --ignored export_corpus` |
| the Hurl suite | `tests/hurl/` | `make hurl-export`, which runs `cargo test --features conformance_host --test conformance_host -- --ignored export_hurl` |

Both artefacts are committed files and both are gates. A pull request
whose committed artefacts differ from the regenerated ones fails a
named CI step, `the generated compliance artefacts are current`, and
the two in-process gates fail with the file that differs and the
command that regenerates it. A tag regenerates both in its own
checkout and attaches them to the release.

The exporters overwrite without guard: a hand edit inside either
artefact directory is destroyed by the next export, silently. An edit
belongs in the reference behaviour (`tests/compliance.rs`), and the
artefact is the mechanical consequence of it.

## 7. Register

The corpus files live in `tests/compliance/corpus/`, one file per
family. The reference runner reads every file and executes every case;
a failure reports the case `id`, the clause, and the first mismatch.
The suite grows by families; a family MUST state its clauses before its
cases are generated.

The corpus has a second copy downstream: `lunet-locks` pins this crate
at `ext/uvrr-core` and carries its own copy of `tests/compliance/corpus`
at the pinned version, so a regeneration here is a change the
downstream lock must re-sync before its own consumers see the corpus
the release carries.

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

## 8. The transport

The corpus is data, so a case is replayable over any transport. A
conforming host MAY expose the abstract host interface of §2 over HTTP,
which lets a client that is not the host's own language replay the
corpus without re-deriving the executor, the exact-multiset comparison,
or the two rendering grammars of §3: the case arrives as the request
body, and the host answers with what its own replica did.

This clause states the endpoint contract. A host that serves these
endpoints, and a client that drives them, conform independently of each
other's implementation language; neither needs the other's source. The
reference host is the repository's conformance host
(`tests/conformance_host.rs`), and the reference client is the Hurl
suite in `tests/hurl/`, generated from the committed corpus.

### 8.1 The case endpoint

`POST /case` takes one case object — the record of §3 verbatim, its
`expect` included — and replays it. The response is:

| field | meaning |
|---|---|
| `id` | The case's `id`, echoed. |
| `family` | The case's `family`, echoed. |
| `verdict` | `pass` when every expectation the case names is met, `fail` otherwise. |
| `mismatch` | Absent on `pass`. On `fail`, the named field that differs, in the form `<field>: expected <named>, captured <observed>`. |
| `expect` | The full capture: the delivered datagram multiset and the rendered per-node post-state, in the §3 grammars. |

The request carries the expectation, so a host holds no corpus of its
own and a client needs none: the case is the whole contract. A host MUST
NOT regenerate the case it is given, and a host MUST NOT answer `pass`
by comparing anything other than its own capture against the `expect`
the request carries. A codec divergence MUST therefore surface as a
`fail` verdict with a `mismatch`, never as a rewritten fixture.

### 8.2 The session endpoints

`POST /session` takes `{"nodes": <usize>, "timeout": <u64>}` — the
`provision` operation's own arguments — and answers `{"session":
<id>}`. Three further endpoints drive the named session:

| endpoint | method and body | effect |
|---|---|---|
| `/session/<id>/op` | `POST`, one §2 operation in the case record's `op` tagged form | Applies the operation. Answers `{"ok": true}`, or `{"ok": false, "error": <reason>}` when the operation is refused. |
| `/session/<id>/capture` | `GET` | Drains the network and answers the full capture in the `expect` shape of §8.1, without a verdict. |

The operations are one endpoint, not fourteen: the operation is already
data, so a route per operation would restate the record's `op` tag as a
path segment and make the two disagree in two places. A client that
prefers one route per operation derives it from the same tag.

`capture` is the settling window of §3 made explicit. It MUST drain
until the network is quiet, within the same bounded, deterministic loop
the in-process runner uses, with no wall-clock wait, and it MUST return
the complete delivered set: an empty `deliveries` asserts silence, and a
client can only assert that silence from a body that carries the whole
set rather than a prefix.

### 8.3 What the transport must not change

- **Determinism.** The same case against the same host MUST produce the
  same capture on every run, so the host MUST NOT introduce ordering the
  case does not state and MUST NOT serialise concurrent sessions into
  one another's state.
- **The grammars.** `journal`, `markers`, `status` and the abstract
  operation names are the §3 grammars. A host MUST render them as that
  text, never as its own language's debug formatting: the client
  compares them literally.
- **Silence.** A case naming no deliveries fails a host that emitted
  one, and the response is where that is provable.

### 8.4 Conformance over the transport

An implementation conforms over the transport when it serves the
endpoints of §8.1 and §8.2 over its own bindings and every case in the
suite returns `pass`. The reference host's equivalence gate asserts the
stronger property that makes the transport trustworthy: the capture
served over HTTP and the capture the in-process runner derives MUST be
the same capture, on every case, byte for byte. A divergence between
the two runners is itself a finding.
