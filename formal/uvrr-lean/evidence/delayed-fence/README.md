# Repeated-recovery counterexample — unresolved

On unchanged production code, the public harness reports
`CommittedDivergence { a: NodeId(0), b: NodeId(1), slot: Slot(3) }`.
Node 0 commits operation `x`; node 1 later commits operation `y` at the same
slot. This is an observed counterexample, not a proposed repair.

The final reproduction uses ordinary backup timeouts, three serial amnesiac
recoveries, fresh recovery ticks, and only packets previously emitted by the
core. At most one replica is down or recovering at any time. It uses the
existing public planning/publishing path and the existing harness lifecycle;
it does not fabricate protocol packets or edit production state.

| Stage | Observed execution |
|---|---|
| 1 | Node 1 enters view change 1, emits its first-round message, then crashes before collecting a fence quorum. |
| 2 | Nodes 0 and 2 send fresh view-0 recovery replies to node 1. Before those replies are delivered, node 2 receives node 1's delayed first-round message and emits its own first-round message and a view-change report for view 1. |
| 3 | Node 1 completes recovery into view 0 from the replies already sent. Node 2 then crashes and recovers into view 0 from nodes 0 and 1. |
| 4 | Nodes 0 and 2 commit `x` at slot 3; node 1 has not received it. |
| 5 | Node 1 times out again. The delayed first-round message and view-change report from node 2 are delivered. Node 1 activates view 1 with only genesis in its log. |
| 6 | Node 2 crashes again. Fresh responses report node 0's view-0 committed log and node 1's view-1 genesis log. Node 2 completes recovery from the latest-view primary, node 1. |
| 7 | Nodes 1 and 2 commit `y` at slot 3. Node 0 still has committed `x` there. |

The ordinary safety assertion is the failing assertion. Both host-forced and
ordinary-timeout variants reproduced it; the final archived source uses
ordinary timeouts. The core emitted no declared fault during the schedule.

## Replay

From the repository root:

```sh
python3 formal/uvrr-lean/check_recovery_counterexample.py
```

The runner temporarily installs the archived source as an integration test,
executes it, and removes only that temporary file. **Its zero exit means the
known Red counterexample reproduced, not that the protocol is safe.** It
requires the exact committed-divergence diagnostic; a changed outcome needs
investigation. The archived test itself asserts safety and exits with failure.
There is no silently ignored regression in the ordinary suite.

## Separate TLA witness

`tla/DelayedFence.tla` directs 34 actions through the archived research copy of
`VrrCoreEras.tla`. It reproduces `CommittedLogsAgree` failure in 35 states. The
configuration also checks `TypeOK` and `OneRecovering` before agreement.
`tla/research-extension.patch` lists every difference from the repository model:
allow serial crashes while forbidding a second concurrently recovering node,
and use the incremented episode as a fresh recovery nonce. No agreement,
transfer-prefix, normal-mode, or quorum guard was removed. The original model
is unchanged. The Rust witness independently uses the existing fresh host ticks.

With TLC 2.19 on the classpath, run from this directory's `tla/` subdirectory:

```sh
java -XX:+UseParallelGC -Xmx512m -cp "$TLC_JAR" tlc2.TLC \
  -workers 1 -metadir /tmp/uvrr-delayed-fence-states \
  -config DelayedFence.cfg DelayedFence.tla
```

Expected outcome: invariant failure, not a successful safety check. The complete
output is `tla/red.log`; hashes and the production revision are in
`provenance.json`. This is a directed witness, not exhaustive exploration of all
repeated-crash executions.

## Proof boundary

The crash-free shared-model theorem remains valid. The replicated-fence theorem
starts from an explicitly online supporting quorum; that checkpoint condition
has not been derived for asynchronous first-round evidence across recoveries.
The full requested diskless uVRR proof cannot be claimed for the current code
while this schedule remains admitted. No production repair has been implemented.
