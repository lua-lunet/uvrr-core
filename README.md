# vrr-core

A **sans-io** [Viewstamped Replication Revisited](https://dspace.mit.edu/server/api/core/bitstreams/9f8c52b3-ea46-4fde-9dc9-354ed6d9c7d9/content)
core library in Rust, with a C ABI for LuaJIT FFI and a demo Maelstrom node for checking it.

Sans-io means the whole protocol is a state machine: you hand a replica an input
and drain the outputs it produced. No sockets, no threads, no async runtime, no
callbacks. The host owns transport, timers, and durability.

```rust
let mut replica = Replica::new(vec!["n0".into(), "n1".into(), "n2".into()], "n0")?;
let outputs = replica.step(Input::Request { /* ... */ });
```

The Maelstrom node is a demo node a simple bin that does a trivial key-value store over stdin/stdout so that the Kyle Kingsbury Maelstrom test harness can create a cluster of nodes to then inject network partitions, crashes and other error conditions to test the core VSS library.

The fact that the software passing Maelstrom testing is not evidence of no bugs. Yet it is a demonstration that there are no shallow bugs and that the library has some resiliance under only network partitions, only crashes, or combinations of both. Yet read the MIT license if you build a system on top of this library the bugs may be in the combination of all the code. You should consider writing custom Maelstrom logic to validate your entire system. 

## Why this exists

As at 2026-08 nothing published covers the hard half of VRR such as view change, recovery and reconfiguration. No crate exposes a C ABI, and the async ones are
the wrong shape for FFI. You would be pumping a Tokio runtime from inside
LuaJIT. 

This crate is extracted from my own system where a lightweight and embeedable strong consistency model over a small amount is more cost effective than Zookeeper or etcd. Just having one thing decided which is which of three data centers is the primary and being able to embed that into an openresty process is a very powerful capability.

This crate implements the parts of Viewstamped Replication Revsited that often get skipped:

| | |
|---|---|
| Normal operation | PREPARE / PREPARE_OK / COMMIT, quorum `Q = floor(K/2)+1` |
| Epoch change | START_EPOCH_CHANGE / DO_EPOCH_CHANGE / START_EPOCH |
| Recovery | RECOVERY / RECOVERY_RESPONSE, with restart amnesia handled |
| State transfer | whole-log, with monotonicity rules on adoption |
| C ABI | `cdylib` + `staticlib` + [`include/vrr.h`](include/vrr.h) |

Liskov and Cowling's paper has known defects in the recovery and state-transfer
sections — the recovery algorithm can leave the system inconsistent and state
transfer can lose data. This core does not implement those sections literally;
the adoption rules are monotonic in epoch, then in slot/commit/log-prefix, and
`tests/recovery_evidence_monotonicity_matrix.rs` pins that.

## Evidence

`make e2e` does a docker build to run the end-to-end Maelstrom test suite.

`cargo test` runs 90 tests: unit and matrix tests per protocol path, targeted
regressions, a deterministic seeded multi-replica cluster harness (K=3..7,
loss / reorder / duplication / partition / crash-with-amnesia, safety asserted
after *every* single step), and proptest companions.

`maelstrom-lin-kv` runs the core as a [Maelstrom](https://github.com/jepsen-io/maelstrom)
node so Jepsen's Knossos checker verifies **linearizability** — strictly
stronger than [sequential consistency](https://jepsen.io/consistency/models/sequential) —
under partition, process kill, and process pause. Latest: 10,880 operations at
K=5 under all three nemeses, `:valid? true`, no failures.

```bash
mise install          # JDK 25 + Leiningen 2.11.2
brew install gnuplot  # only for Jepsen's plots; checkers work without it
make test             # the Rust suite
make test-all         # lin-kv under partition + kill + pause
make serve            # browse results at http://localhost:8080
```

`make help` lists the tunables (`NODES`, `TIME_LIMIT`, `RATE`, `INTERVAL`).

### Docker

For environments where installing Java, Leiningen, and Rust is difficult, use
the Docker image:

```bash
make e2e
```

Or manually:

```bash
make docker-build
docker run --rm -e NODES=5 -e TIME_LIMIT=60 -e RATE=20 -e INTERVAL=20 vrr-core-maelstrom test-all
```

See `Dockerfile.maelstrom` for the build definition.

## Two things that will mislead you

**Cluster size and the fault budget.** Jepsen's default kill targets include
`:majority` and `:all`. At `NODES=3` (f=1) that routinely kills two of three
nodes, and a crash-fault protocol tolerating one failure then cannot make
progress *by construction* — you get `ok-count 0`, `:valid? false`, and a
vacuously true `:linearizable`. Safety still held; liveness was impossible. The
default here is `NODES=5` (f=2) for that reason. Read `ok-count` before reading
the verdict.

**Recovery needs every other member below K=5.** Recovery requires `Q` distinct
*other* Normal responders. At K=3 and K=4 that is every remaining member, so a
recovering replica tolerates zero further failures. At K=5, `Q=3` of 4 others,
leaving one spare. Rolling restarts only make progress from K=5 up.

## The whole-log transfer ceiling

`MAX_DATAGRAM` is 65,507 bytes — one UDP payload. Epoch change and recovery move
the log **whole**, so with realistic payloads a transfer stops fitting at about
85 entries (509 even with empty payloads, since `Vec<u8>` serialises as a JSON
array of decimal numbers). Past that the transfer is refused with
`VRR_TOO_LARGE`.

The refusal is **atomic** — state rolls back and the node keeps serving, which
`tests/datagram_boundary_matrix.rs` pins on both arms of the boundary. So this
is a liveness limit, not a safety one: a node that has fallen too far behind
cannot be caught up in one datagram.

It is deliberately not papered over. Catch-up wants a different mechanism than
"push the log in a packet" — a chunked stream with a sequence counter, a
side-channel fetch, or a snapshot/restore, the way a database ships a backup
rather than the database. Consumers that only ever hold small, expiring state
may never reach the ceiling.

Note this ceiling lives in `ffi.rs`, not in `Replica`. Using the crate as a Rust
library bypasses it entirely — which is why `maelstrom-lin-kv` links the library
directly and its evidence says nothing about this limit.

## Status

Pre-alpha. The protocol is covered by the tests above and by Maelstrom; the API
is not stable and there has been no production use.

## Attribution

This project uses the following open-source tools for testing and validation:

- **[Maelstrom](https://github.com/jepsen-io/maelstrom)** — a workbench for learning
  distributed systems by writing your own, created by [Kyle Kingsbury](https://jepsen.io)
  and the [Jepsen](https://jepsen.io) team. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

- **[Jepsen](https://github.com/jepsen-io/jepsen)** — a framework for testing
  distributed systems, also by Kyle Kingsbury. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

- **[Knossos](https://github.com/jepsen-io/knossos)** — Jepsen's linearizability
  checker. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

The `maelstrom-lin-kv` binary implements a [Fly.io Distributed Systems
Challenge](https://fly.io/dist-sys/) style linearizable key-value store using this
core, allowing it to be tested against Maelstrom's `lin-kv` workload and
Knossos checker under network partitions, process kills, and pauses.

## Licence

Apache-2.0.
