# vrr-core

A **sans-io** [Viewstamped Replication Revisited](https://dspace.mit.edu/server/api/core/bitstreams/9f8c52b3-ea46-4fde-9dc9-354ed6d9c7d9/content)
core library in Rust, with a C ABI for LuaJIT FFI and a demo Maelstrom node for checking it.

Sans-io means the whole protocol is a state machine: you hand a replica an input
and drain the outputs it produced. No sockets, no threads, no async runtime, no
callbacks. The host owns transport, timers, and durability.

```rust
use vrr::effects::Stability;
use vrr::ids::{NodeId, Operation, OperationId, Tick};
use vrr::journal::{Journal, SegmentedLog};
use vrr::quorum::WeightedMajority;
use vrr::replica::{Input, Replica, TimedInput, ViewChangeKnobs};

let mut replica = Replica::provision(
    NodeId(0),
    vec![NodeId(0), NodeId(1), NodeId(2)],
    WeightedMajority,
    SegmentedLog::new(),
    Stability::Volatile,
    ViewChangeKnobs { primary_timeout: 0, view_change_budget: usize::MAX },
)
.expect("provision");

// The genesis primary promotes itself on the first tick.
let view = replica.journal().view();
let planned = replica
    .plan(&TimedInput { at: Tick(1), event: Input::Tick }, &view)
    .expect("plan tick");
replica.publish(planned).expect("publish tick");

// Propose a host operation for ordering; the core carries it opaque.
let proposal = TimedInput {
    at: Tick(2),
    event: Input::Propose {
        operation: Operation {
            id: OperationId { msb: 0, lsb: 1 },
            payload: b"hello".to_vec().into_boxed_slice(),
        },
    },
};
let view = replica.journal().view();
let planned = replica.plan(&proposal, &view).expect("plan propose");
let _outcome = replica.publish(planned).expect("publish propose"); // PublishOutcome::Published { revision, effects }
```

To build and test without running the crash testing and network partitioning simply:

```text
cargo test
```

This repo includes an example binary that conforms to a trivial key-value store protocol so that the Kyle Kingsbury Maelstrom test harness may simulate network partitions, crashes and other error conditions. The Makefile can install and run the Kyle Kingsbury Maelstrom test harness as either a git submodule run run locally else in docker. See below. 

## Why this exists

This create simply offers the strong consistency during non-stop cluster reconfigurations without disk flushes. This is achieved by porting to Viewstemped Replication Revisited the leader casting vote technique from David Turner's technical report on unbounded pipelining in dynamically reconfigurable clusters (Tracsis, 2016, tessanddave.com).

This Rust crate exposes a C ABI for FFI. It scales down to offer a lightweight and embeddable strong consistency model. With a small amount of data, such as leader leases or advisory locks, it removes the need to run something like Zookeeper or etcd. 

In my experience the concept of external strong consistency service is one that has very sharp edges. It splits responsibility for quality, legitimacy, and performance across two product teams. Every client connected to the core is part of the full distributed system and must experience consistency. When there are silos of responsibility on the critical path then often no-ones hold themselves accountable the removal of every single source of outages. 

If you are curious to see if embedding strong consistency directly into your application reduces the complexity, costs and latencies of your system then try this crate. 

## Evidence

The demo kv replication passing Maelstrom testing is not evidence of zero bugs. Yet it is a demonstration of an absence of shallow bugs and that the library has some resilience to network partitions, crashes, or combinations of both. If you build a system on top of this library the bugs may be any  combination of all the code. You should consider writing custom Maelstrom logic to validate your entire system. 

`make e2e` does a docker build to run the end-to-end Maelstrom test suite.

`make tla` builds a self-contained TLC image and exhaustively checks the
finite [TLA+ correspondence model](formal/README.md) for normal operation,
view change, and the fenced crash-stop event. The image embeds the model: this path
uses classic Docker commands and requires neither BuildKit nor a volume mount.

`cargo test` runs the protocol-path tests, targeted
regressions, and a deterministic seeded multi-replica cluster harness (K=3..7,
loss / reorder / duplication / partition / crash-restart, safety asserted
after *every* single step), plus proptest companions.

In order to run the maelstrom targets you need to fetch maelstrom as a submodule with 

```shell
git submodule init
git submodule update
```

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
docker run --rm -e NODES=5 -e TIME_LIMIT=180 -e RATE=20 -e INTERVAL=10 vrr-core-maelstrom test-all
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

## Status

Pre-alpha. The protocol is covered by the tests above and by Maelstrom; the API is not stable. It is intended to be open to extension yet closed to modifications of the invalidate the invariants of the algorithm. This means that it is only like to change if new extension points are needed or if a bug is found. 

The codebase is intented to stay small and has advasorial tests. An absence of new feature being pushed is an absence of bugs and regressions. 

Due to the Yeti nature of the superior but little advertised technology we are unlikely to see a ton of users leading to a 1.0.0 release. Yet I am more than open for to the idea. 

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

MIT.
