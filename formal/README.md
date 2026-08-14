# vrr-core TLA+ model

`VrrCore.tla` is an executable safety model of the protocol-visible state and
transitions implemented by this crate. It is deliberately a model of this
codebase's contract, not a copy of a paper model.

The correspondence is:

| TLA+ | Rust |
|---|---|
| `status`, `currentView`, `retainedView`, `committed`, `applied` | `progress::Progress` |
| `logs` and `Len(logs[r])` | `Journal` and `Progress::accepted()` |
| `Propose`, `ReceivePrepare`, `CommitNext`, `ReceiveCommit` | `replica/normal.rs` |
| `EnterViewChange` through `ReceiveStartView` | `replica/view_change.rs` |
| `Crash` through `CompleteRecovery` | `Replica::reopen` and `replica/recovery.rs` |
| `ApplyNext` | `Input::Applied` |
| `messages` | published `Effect::Send` values plus adversarial transport |

The model makes loss, reordering, delay, and duplication implicit: messages
remain in a set forever, and a receive action is never required to occur. A
message can therefore be ignored, received later than newer messages, or
received repeatedly.

The model abstracts bounded suffix transfer to an atomic full-history install.
This is sound for the checked safety properties because transfer can only make
the already-selected history constructible; it cannot select a different
history. The `plan`/`publish`/confirmation handshake is likewise one atomic
TLA+ step: the Rust publication gate is responsible for refining that step so
no effect becomes visible before its supporting state. Reconfiguration is not
modelled because `Input::Reconfigure` is currently a named unsupported path.

The model does not claim liveness, correctness of suffix chunk assembly,
durability-barrier ordering, or exactly-once emission of an `Effect::Apply`
before the host acknowledges it with `Input::Applied`. Those are separate
implementation contracts exercised by the Rust suites; `applied` here records
only completed application inputs.

Two finite models are checked:

- `VrrCore.cfg`: normal operation and view change with three replicas, two
  commands, and views 0 and 1.
- `VrrCoreRecovery.cfg`: normal operation, view change, and one fenced
  amnesiac crash/recovery event with three replicas and one command. The crash
  target is nondeterministic, so primary and backup loss are both explored.

Run both with the repository's no-volume Docker path:

```sh
make tla
```

This uses ordinary `docker build` and `docker run`; it does not require
BuildKit or a bind mount. The default is explicitly `linux/arm64`, the built
image architecture is checked before either model runs, and TLC uses two
workers to limit sustained thermal load. These can be overridden with
`TLA_PLATFORM`, `TLA_ARCH`, and `TLA_WORKERS`. For a local Java 11+
installation and a downloaded `tla2tools.jar`:

```sh
make tla-local TLA2TOOLS_JAR=/absolute/path/to/tla2tools.jar
```

The principal checked theorem is prefix agreement: any two replicas agree at
every slot both consider committed. The other invariants enforce the frontier
order, application-prefix agreement, the current/retained view distinction,
recovery fencing, and survival of every locally committed entry.
