# uVRR application protocol: the gateway, the session, and the obligations of a host

The architecture decision B2 (`docs/architecture.md`) removes the
per-client table from the core: the core orders opaque operations and
nothing else, and duplicate policy, result caching, and reply delivery
belong to the host. What B2 leaves unnamed is the role within a host that
terminates alien application traffic. This document names that role, the
**gateway**, and states the normative obligations an application protocol
over uVRR owes it. A worked example, built against this document, is
maintained in the
[lunet-locks](https://github.com/lua-lunet/lunet-locks) repository at
`docs/src/rfc-application-protocol.md`.

## Terminology

The key words MUST, MUST NOT, REQUIRED, SHALL, SHOULD, SHOULD NOT,
RECOMMENDED, MAY and OPTIONAL are to be interpreted as described in
RFC 2119.

**application**: the party on the far side of a network that is unknown
to uVRR: a phone application, a browser, another machine. An application
is never referred to as a client. The service never holds an application
identity.

**gateway**: the role a node holds when it terminates alien application
traffic. A gateway accepts an application connection, holds the session
that connection carries, and forwards that session's commands to the
replication protocol. Every node is a gateway in turn; the role is not a
process, a binary, or a deployment unit.

**host**: the distributed system that runs an application protocol over
uVRR. A host is one or more processes and is not a single thing.

**session**: the state a gateway holds for one application connection's
command stream, identified by a `session_id`.

**session_id**: the unsigned 64-bit identity of a session, unique across
the cluster for the lifetime of the cluster. A session's `session_id` IS
the log slot committed by the command that established it.

**counter**: the unsigned 64-bit per-session sequence number, owned by
the gateway, incremented once per command accepted on that session.

**uuid**: the 128-bit correlation identity of one command:
`pack_128(session_id, counter)`. The low 64 bits are the counter, the
high 64 bits the `session_id`.

**bearer**: the unforgeable token derived from a `session_id` that an
application presents to prove which session it holds. A `session_id`
never travels to an application in the clear.

**nexus**: a gateway's map from a command's `uuid` to the opaque handle
the command arrived on, together with the command in flight.

## Normative obligations

A gateway MUST satisfy all of the following.

1. A gateway MUST offer at most one outstanding command per session. The
   reason is ordering, not throughput: a command written before its reply
   has been read risks being reordered past it. A second, different
   command arriving on a session that already holds one MUST be refused
   by name, never queued behind it. A re-send of the command that is
   outstanding (the same `uuid`) is not a second outstanding command;
   it attaches to the command already in flight.
2. A gateway MUST assign `uuid = pack_128(session_id, counter)` and MUST
   increment `counter` exactly once per accepted command. The `uuid` is
   therefore unique by construction: two nodes can never compute the
   same one, because the `session_id` is a committed log slot.
3. A gateway MUST NOT let an application observe a `session_id` in the
   clear. A `session_id` MUST reach an application only as a bearer,
   which an attacker cannot invert to enumerate adjacent sessions.
4. A gateway MUST write a command's result from a nexus entry taken at
   the moment of writing, and MUST NOT write a result twice. This is the
   at-most-once property, and it is structural rather than defended: the
   entry is taken, uVRR commits each operation exactly once, and a crash
   between the take and the write destroys the session, leaving nothing
   to write the result from. A gateway MUST NOT attempt to retransmit a
   result.
5. A gateway MUST treat a timeout, a closed socket, or an I/O error as
   UNKNOWN. It is not evidence that the command did not happen, only
   that the outcome was not learned. The application re-drives by
   establishing a new session; a re-drive is a new command, never a
   resurrection of the old one.
6. A gateway MUST NOT rely on a load balancer for session affinity. A
   balancer terminates nothing: it distributes connections. A session's
   authority is the leader's, not the connection's.

## Advisory deployments

An application MAY be written against any transport. Two shapes are
anticipated.

Behind a layer-4 load balancer (the reference deployment of
`docs/clients.md`), a set of gateways holds the sessions and the balancer
distributes connections. If the balancer routes an application to a
gateway that does not hold its session, the application MUST establish a
new one, and the old session is gone with the gateway that held it.

Over a datagram transport, a gateway has no per-session socket and MUST
re-establish the ordering the session depends on: it allocates the
counter per session and forwards that session's commands to the leader as
one unit. The nexus then maps the `uuid` to whatever handle the gateway
holds, and the obligations above apply unchanged.

## What the replication protocol owes the session

uVRR commits each opaque operation exactly once and in one order. That is
what makes obligation 4 structural: the commit is the commit, and the
reply is a courier for it that may be lost but never duplicated. The
gateway supplies everything the core deliberately does not: identity,
ordering, deduplication, delivery: and the session is the unit it
supplies them in.
