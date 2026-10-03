# Client-facing deployment: the load-balanced circuit

This document states how client traffic reaches a uVRR cluster in a
reference deployment, and how the client table of the 2012 paper lands
inside the host. The core is sans-IO: it owns no socket, no connection,
no retry policy, and no client identity. Everything below is host
territory: the library does not dictate it. The model here is a worked
example of a shape that keeps the client story honest without asking the
protocol to know that clients exist.

## What VRR-2012 assumes, restated

The 2012 paper (Liskov and Cowling, *Viewstamped Replication Revisited*)
gives the client side a small state machine. The client library records
the cluster configuration and the current view number, so the client
addresses the primary directly: it sends `REQUEST op, c, s` to whichever
replica its own view arithmetic names as primary, and the replicas answer
it. A client is allowed to have just one outstanding request at a time;
each request carries a request number strictly greater than all earlier
ones; the primary keeps a client table of the latest request number per
client and its cached result, so a re-sent request whose number is not
greater is dropped, with the cached response re-sent when the request is
the most recent already executed. The paper's convenience assumptions:
a request/response socket shape, and a client that can compute the
primary's identity. Neither assumption survives contact with a generic
embeddable core, for the reasons `docs/architecture.md` B2 records.

## The reference deployment: two sites, two nodes each, behind a layer-4 load balancer

Four cluster members: two in each of two data centres, speaking to each
other over the internet. Vote weights are a deployment policy; the
membership law (at least three voting members, R14's unit-mass rule)
applies unchanged regardless of where members sit.

Clients open TCP connections to the load balancer's address. The load
balancer distributes new connections across the nodes, so with one leader
and three followers roughly three quarters of the client connections
terminate on followers. The load balancer polls each node's listening
port with a health check; when a node dies the check fails, the load
balancer resets the connections that were mapped to it, and new
connections land only on the survivors. The check keeps polling the dead
node's port, so when it returns, fresh connections start balancing onto
it again. Nothing in this paragraph names the leader: clients never need
to know.

The load balancer products that provide exactly this behaviour:

| Provider | Product | Shape | Health check | On unhealthy target |
|---|---|---|---|---|
| AWS | Network Load Balancer | regional L4 (TCP/UDP/TLS), static IP per zone | TCP probe by default | sends TCP RST on connections of the unhealthy target; fails open only when every target is unhealthy |
| Azure | Load Balancer (Standard) | regional L4 (TCP/UDP), static frontend | TCP/HTTP/HTTPS probe | probes stop new connections to the failed node; existing connections reset |
| Google Cloud | External passthrough Network Load Balancer | regional L4 (TCP/UDP), static IP | health checks from Google's probe ranges | stops routing to the failed backend; reconnections land on the healthy pool |

All three poll the dead backend's port and resume sending new connections
to it once it passes again. The pattern a host depends on is the common
L4 one: connect-time distribution, probe-driven failover, reset on
failure, poll-driven rejoin.

## The command circuit

A client issues a command over its TCP connection to whichever node the
load balancer selected. The command carries an operation identifier (the
core's opaque 128-bit `OperationId`, so one client command is one
`OperationId`).

1. **Accept.** If the node is a follower, it forwards the command to the
   leader over the cluster transport. If it is the leader, it enqueues
   directly. Either way the node remembers the client socket in a map
   keyed by the command's identifier.
2. **Batch.** The leader's loop drains the queue and packs selected
   commands into one or more slots (a batch is a slot group; slots are
   the unit of ordering).
3. **Phase 2.** The leader runs the accept phase against the cohort: one
   round trip, a quorum of acceptances.
4. **Commit.** The commit fact propagates, carried either as its own
   message or as a rider on the front of the next phase 2.
5. **Upcall and resolve.** On commit each node applies the slot to its
   application state. The node holding the client connection reads the
   command's identifier, checks its command-to-socket map, and if a
   client connection is still on hold it serialises the result onto that
   TCP socket and drops the map entry. A void method yields a void
   result, which still flows back: the client learns the command
   completed.

End to end this is two round trips of wide-area latency in the absence of
straggler traffic: the phase 2 round trip, and the commit leg that
completes the circuit back to the client at the node where it connected.
The follower forward adds one local or intra-site hop on the front, which
is why three quarters of the traffic paying it is still the cheap path:
the wide-area quorum is the phase, not the fan-out.

If the node holding the client socket crashes before replying, the load
balancer has already reset the connection; the client reconnects, lands
on a survivor, and resubmits. Whether resubmission deduplicates is the
host's policy (`docs/architecture.md` B2): the core treats a repeated
identifier as a new arrival, so the host decides between retry, replay,
and reject. A crash clears the map; an I/O failure leaves the write
outcome unknowable to the caller, per the same section.

## The one-outstanding-request obligation, mapped

VRR-2012's constraint that a client has at most one outstanding request
maps, in this deployment, onto the map itself: at the node where the
connection terminates, there is at most one in-flight entry per client,
keyed by the command's identifier. The client's role in the paper (track
the view, address the primary, deduplicate by request number) dissolves
into the load balancer plus the host map: the client keeps a TCP socket
and reads from it; everything else is server side. A client that pipelines
requests down one socket is a host choice; the map is per command, not
per connection, so pipelining is supportable if the host wants it, but the
reference shape stays one outstanding command per client.

## Transport note: PAXE

Cluster links and forwarded commands can ride
[paxe-core](https://github.com/lua-lunet/paxe-core): connectionless,
encrypted datagrams (AES-256-GCM under host-provisioned pre-shared keys,
no handshake). Its wire header addresses a `u16` peer identity and a
`u32` channel, so a deployment can address 65,536 nodes and multiplex up
to four billion logical channels per peer pair: the channel field is how
many client connections' traffic can be multiplexed over one node's
datagram transport without sharing ordering state. PAXE is sans-IO in
the same sense this library is: it protects datagrams and owns neither
sockets, retransmission, nor ordering, so the load balancer circuit above
is unchanged whether the cluster links are TCP or PAXE.
