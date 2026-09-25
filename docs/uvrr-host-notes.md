# uVRR host notes: discussion points, not obligations

> These notes are discussion points, not obligations. The library is
> sans-IO: it cannot and does not prescribe how a host names its files,
> lays out its disks, or rotates its machines. Your mileage will vary.
> Where a worked example helps, consult the latest official demos; nothing
> here is kept current, and nothing here is authoritative.

The normative host obligations live elsewhere and they rule: the boot gate
(`uvrr-boot-gate.md`), the termination obligations
(`uvrr-termination-obligations.md`), and the identity law
(`uvrr-boot-gate.md` §5). This document holds the practices a deployment
has found useful and nothing more.

## The system-identifier practice

- The system identifier is sysadmin-assigned. It is burnt into the boot
  marker before first boot, and where the deployment keeps a WAL or
  superblock header, the same identifier may be burnt there for the same
  reason: every durable artifact names the system it belongs to.
- A separate genesis command is the only writer of that field. It runs
  once, under the operator's hand, and the running process never mints a
  system identifier of its own.
- The process command line takes its system identifier and aborts on a
  boot-fence mismatch. The identifier never begins at zero: a zero read is
  an uninitialised field or a corrupt marker, never an identity.

## The halt-and-move practice

Cluster nodes may be halted, their files moved between machines, and the
system booted elsewhere. Opening the durable artifacts against the
command-line system identifier is what prevents a node from accidentally
mixing state between machines: a file that names another system is refused
at open, before any protocol state is read.

## The crash-loop practice

The identity bump refuses at the counter bound: the sixteenth bit of lives
is the last the packing holds, and a bump past it is a refusal, never a
wrap. A host process monitor that retries a failing node indefinitely
walks that bound one life at a time. Kubernetes-style health and readiness
checks are the sharp edge: a node whose startup is slow can be killed and
retried before it ever joins, and a perma crash loop depletes the
deployment's headroom one burned identity per attempt.

A node that was never able to join the cluster owes the cluster nothing,
and its counter carries no commitment the cluster ever saw. Resetting such
a node's counter to a number just above its last committed membership
value does not harm safety: the identity law rules the identities the
cluster has seen, and a life that never joined was never seen. The reset
is the host's deliberate act, taken with the same care as the genesis
mint, and never applied to a node the cluster has seated.

This suggests what the cluster-membership history should record: the host
id alongside the separate halves. The actual limit is the highest
committed crash counter any node has held while a member of the cluster at
any point, which exhausts at `u16::MAX` rejoins of the cluster: a minimum
of `u16::MAX` cluster reconfigurations of headroom, which is ample.
