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
