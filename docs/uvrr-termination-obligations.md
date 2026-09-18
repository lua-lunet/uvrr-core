# uVRR termination obligations: what every host owes an embedded replica

uVRR is a sans-I/O library embedded through an FFI boundary into any host
runtime (a Lua runtime, a Zig process, a Rust service). The library owns the
protocol; the host owns the event loop, the wire, and the disk. Termination
is therefore a contract between the two: the library defines the obligations,
and every future host application — an unbounded set — must meet them to
embed uVRR safely. The obligations are recorded here as the formal contract.
The same requirement is raised with the reference host runtime as
lua-lunet/lunet#163. Termination's dual — the classification of a start as
clean or crashed, and the write schedules each path owes — is the boot gate
(`uvrr-boot-gate.md`).

## 1. The obligations, ranked

Two obligations rest on the host runtime at termination. They are not equal
in rank.

**Mandatory — the drain point closes the wire.** Once the host has said the
replica is stopped, no further inbound messages are picked up: no new reads,
no new accepts, no new task processing at or after the drain point. This is
the obligation that makes everything else possible, because it is the only
thing that makes the in-memory state final — the state in memory is the
state, so the in-flight state can be written to disk at the drain point and
will not be contradicted by later work arriving from the wire.

**Desirable — outbound is flushed.** Knowing that every outbound message has
been flushed is nice but not mandatory. Outbound writes either complete or
are abandoned safely: a message that was never sent is safe to abandon,
because a receiver cannot tell "never sent" from "lost on the wire". The host
must not let the desirable obligation delay the mandatory one.

The mandatory obligation is the termination dual of the restart rule in
`uvrr-reincarnation.md`: a clean stop is a protocol-visible event, and what
makes it clean is that the wire is closed before the state is declared final.

## 2. The lifecycle

The lifecycle below is stated in marker-agnostic terms; the deployed marker
transition machine (`docs/vrr-durability-model.md` §5.1; the code twins:
`src/lifecycle.rs`, `zig/vsr/superblock.zig`) implements it with
the ordered states `Stopping → Stopped → Restarting/Joining`. The
terminology is one language: `running` here means the marker's **not-`Stopped`**
operational states (`Restarting` after a clean stop, `Joining` after a bump —
the `unflushed` of the reincarnation doc), and `flushed` here means the
**`Stopped` quorum after the drain** — the copy that vouches for the WAL
under it. `Running` itself is never written: the boot writes `Restarting`
or `Joining` before the first message, and no safety logic looks for
anything else.

```
startup:  marker := running               (before the loop starts)
stop:     marker := stopped               (termination begins; wire closed)
drained:  WAL write, marker := flushed    (at the drain point)
```

The transitions flip the usual expectation in a useful direction: a copy that
has transitioned to `stopped` or `flushed` is by that fact not running, so a
marker in those states is evidence of a controlled ending.

The write ordering carries the safety argument. The durable state write (WAL
or the host's equivalent) completes **before** the `flushed` marker is
written, and the marker is written only at the drain point where no new task
processing can follow. A marker at `stopped` or later therefore vouches for
the durable state beneath it. A shutdown that dies partway through the marker
writes leaves some copies advanced and some not, and the advanced ones are
still truthful — which is why partial marker writes read as clean, not as a
crash.

## 3. Startup classification

On start the marker is read before the loop starts, and `running` is written
before any message is processed. The classification reads the *minimum*
progress observed across the marker copies, because a crash during the marker
writes must never be mistaken for a clean stop:

- **Clean stop — no resurrection needed.** Every copy in the working read
  shows `stopped` or `flushed` (a `flushed` copy mixed with `stopped` copies
  is the normal mid-flush shape: the previous process got through the drain
  point and was completing its flush). The state is final; the node continues
  normally without the reincarnation path.
- **Crashed — error on crashed.** Any copy in the working read is still at
  `running` — whether the process was killed outright (all copies at
  `running`) or died mid-shutdown (some copies at `stopped`, some still at
  `running`) — the previous process cannot be shown to have reached the drain
  point. This is error-on-crashed: the reincarnation path runs, and the
  runtime must not silently resume as a clean restart.

The classification applies to every identity start, the bumped one included:
the bump write claims (X+1, `Joining`) as the wire-phase marker — it claims
no `Stopped` checkpoint, because the bumped identity has no WAL under it to
vouch for — so a second crash mid-wire-phase reads no stopped quorum and
bumps again (X+2). The lifecycle makes same-identity re-entry after
volatile-state loss unrepresentable **by construction**, not by argument.

## 4. The marker storage: expectation and example

The lifecycle requires one durable marker write at each transition. The
obligation is stated as an expectation, not a mechanism: whatever storage the
host has that is reliable enough to answer "was this process stopped?" after
a crash is acceptable. Superblock writes are not prescribed, because there
may be other very reliable storage to flush; the marker is the host's concern
once the drain point has made the in-memory state final.

The worked example of how such a marker is made trustworthy is TigerBeetle's
superblock, which the vendored store already uses. Its construction:

- **Four copies** of the superblock in fixed, sector-aligned zones of the
  data file, each with a checksum and a hash-chained `sequence`/`parent`
  chain, so a torn, misdirected, or rotted sector is detectable rather than
  trusted.
- **Quorum writes and quorum reads**: a write completes when a quorum of
  copies is durable; a read takes the working quorum, resolves by highest
  identity within it, and repairs the lagging copies. A single lying sector
  cannot decide the read, and no single-sector atomicity is assumed.
- **Ordering over flushing**: writes are forced where the marker must vouch
  for data beneath it, and the marker write happens only after the data write
  completes.

This is the same trick as stable storage itself: Lampson and Sturgis (1979,
*Crash Recovery in a Distributed Data Storage System*, §5.1) build stable
storage from ordinary unreliable disk by writing two copies and reading "the
good, the complete, or the newest" — the quorum-of-copies scheme is the
generalisation of that idea to four copies and checksums.

The expectation is also stated against the flush literature, because a marker
built on `fsync` trust alone is not reliable enough:

- "All File Systems Are Not Created Equal: On the Complexity of Crafting
  Crash Consistent Applications" (Pillai et al., OSDI 2014) shows file
  systems, drivers, and virtual machines that silently ignore or degrade
  flushes.
- "Optimistic Crash Consistency" (Chidambaram et al., SOSP 2013) documents
  how `fsync` conflates ordering with durability and how real systems
  (macOS's `F_FULLFSYNC`, per-filesystem quirks) fail to deliver what
  applications assume it delivers.

Hence the example: a multi-copy, checksummed, quorum-read marker written with
forced I/O is the construction whose guarantees survive those filesystem
behaviours; a single `fsync`ed flag file is not.
