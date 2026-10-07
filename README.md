# Unbounded Viewstamped Replication Revisited (uVRR)

A **sans-io** [Viewstamped Replication Revisited](https://dspace.mit.edu/server/api/core/bitstreams/9f8c52b3-ea46-4fde-9dc9-354ed6d9c7d9/content)
core library in Rust, with unbounded cluster reconfigurations, that replaces
Crash-Recover behaviour with a low-latency Crash-Stop-Reincarnation. It has a C ABI
for LuaJIT FFI and a demo Maelstrom node for checking it.

This crate offers strong consistency during non-stop cluster reconfigurations
without disk flushes. This is achieved by porting to Viewstamped Replication
Revisited the leader casting vote technique from David Turner's technical
report on unbounded pipelining in dynamically reconfigurable clusters
(Tracsis, 2016–17, tessanddave.com).

This Rust crate exposes a C ABI for FFI. It scales down to offer a lightweight
and embeddable strong consistency model. With a small amount of data, such as
leader leases or advisory locks, it removes the need to run something like
ZooKeeper or etcd.

Sans-io means the whole protocol is a state machine: you hand a replica an input
and drain the outputs it produced. No sockets, no threads, no async runtime, no
callbacks. The host owns transport, timers, and durability.

```rust
use uvrr::effects::Stability;
use uvrr::ids::{NodeId, Operation, OperationId, Tick};
use uvrr::journal::{Journal, SegmentedLog};
use uvrr::quorum::WeightedMajority;
use uvrr::replica::{Input, Replica, TimedInput, ViewChangeKnobs};

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

The demo binary in this repo implements a trivial key-value store protocol so that the Maelstrom test harness may simulate network partitions and crashes. The Makefile runs the harness as a git submodule locally or in Docker.

## Why this exists

The concept of an external strong consistency service is one that has very sharp edges. It splits responsibility for quality, legitimacy, and performance across two product teams. Every client connected to the core is part of the full distributed system and must experience consistency. When there are silos of responsibility on the critical path then often no-ones hold themselves accountable the removal of every single source of outages. Crash safety is Crash-Stop-Self-Evict ("reincarnation"): see [docs/uvrr-protocols.md](docs/uvrr-protocols.md) (the reincarnation chapter).

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
after *every* single step), plus proptest companions. The default feature set
never boots the Maelstrom harness: the library, not the harness, is the
deliverable. Run the full lane with the harness before any push:

```shell
cargo test --features maelstrom
```

### The compliance corpus

The protocol's obligations are a corpus of test cases stated as data
(`docs/uvrr-host-compliance.md`), and the corpus is replayable over a
network transport, so a second implementation can prove conformance
without re-deriving the harness. The `uvrr-conformance` binary serves the
abstract host interface over loopback HTTP and a [Hurl](https://hurl.dev)
suite drives it as a client that is not this crate:

```shell
cargo run --features conformance_host --bin uvrr-conformance &
hurl --test --variable base_url=http://127.0.0.1:8099 tests/hurl
```

Both lanes are gates. `make conformance` runs them in process: every case
must return `pass` over the transport, the capture served over HTTP must be
byte-for-byte the capture the in-process runner derives, the committed Hurl
suite must equal the suite generated from the committed corpus, and the
Hurl client must report the whole corpus passing.

### The pre-push hook

`make hooks` installs the repository's pre-push gate (it points git at the
committed hooks with `git config core.hooksPath .githooks`). Before anything
reaches the remote the hook runs the full verification gate, including the
slow and expensive maelstrom lane. It is skippable with
`git push --no-verify`; please do not skip it.

In order to run the maelstrom targets you need to fetch maelstrom as a submodule with 

```shell
git submodule init
git submodule update
```

`maelstrom-lin-kv` runs the core as a [Maelstrom](https://github.com/jepsen-io/maelstrom)
node so Jepsen's Knossos checker verifies **linearizability**, strictly
stronger than [sequential consistency](https://jepsen.io/consistency/models/sequential),
under partition, process kill, and process pause. The K=5 configuration runs
under all three nemeses with `:valid? true` and no failures on recent runs.

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
docker run --rm -e NODES=5 -e TIME_LIMIT=180 -e RATE=20 -e INTERVAL=10 uvrr-core-maelstrom test-all
```

See `Dockerfile.maelstrom` for the build definition.

## Two things that will mislead you

**Cluster size and the fault budget.** Jepsen's default kill targets include
`:majority` and `:all`. At `NODES=3` (f=1) that routinely kills two of three
nodes, and a crash-fault protocol tolerating one failure then cannot make
progress *by construction*, you get `ok-count 0`, `:valid? false`, and a
vacuously true `:linearizable`. Safety still held; liveness was impossible. The
default here is `NODES=5` (f=2) for that reason. Read `ok-count` before reading
the verdict.

## Status

**Release advisory.** v0.13.0 is buggy on the fused-reconfiguration acceptance
path: the per-slot `Prepare` acceptance perimeter judged a system entry against
the committed configuration alone, so the accepted-but-uncommitted predecessors
the `Fuse` envelope folds whole were invisible to it, and a schedule the envelope
accepted and committed entire was dropped entry by entry when it arrived as
individual `Prepare`s (`docs/uvrr-fuse.md` §6). Fixed in 0.13.1.

Pre-alpha: the API is not stable. The protocol is covered by the tests above
and by Maelstrom. The codebase is intended to stay small and has adversarial
tests; an absence of new features is an absence of bugs and regressions. The
core is open to extension and closed to modification that would invalidate
the invariants of the algorithm.

## Attribution

This project uses the following open-source tools for testing and validation:

- **[Maelstrom](https://github.com/jepsen-io/maelstrom)**, a workbench for learning
  distributed systems by writing your own, created by [Kyle Kingsbury](https://jepsen.io)
  and the [Jepsen](https://jepsen.io) team. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

- **[Jepsen](https://github.com/jepsen-io/jepsen)**, a framework for testing
  distributed systems, also by Kyle Kingsbury. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

- **[Knossos](https://github.com/jepsen-io/knossos)**, Jepsen's linearizability
  checker. Licensed under the [Eclipse Public
  License 1.0](https://www.eclipse.org/legal/epl-v10.html).

The `maelstrom-lin-kv` binary implements a [Fly.io Distributed Systems
Challenge](https://fly.io/dist-sys/) style linearizable key-value store using this
core, allowing it to be tested against Maelstrom's `lin-kv` workload and
Knossos checker under network partitions, process kills, and pauses.

## Licence

MIT.
