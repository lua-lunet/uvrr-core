Reconfigurable State Machine Replication from
Non-Reconfigurable Building Blocks

arXiv:1512.08943v1 [cs.DC] 30 Dec 2015

Vita Bortnikov1 , Gregory Chockler1 , Dmitri Perelman2 , Alexey Roytman1 , Shlomit Shachor1 , and
Ilya Shnayderman1
1

2

IBM Research
Technion, Israel Institute of Technology

Abstract
Reconfigurable state machine replication is an important enabler of elasticity for replicated cloud
services, which must be able to dynamically adjust their size as a function of changing load and resource
availability. We introduce a new generic framework to allow the reconfigurable state machine implementation to be derived from a collection of arbitrary non-reconfigurable state machines. Our reduction
framework follows the black box approach, and does not make any assumptions with respect to its execution environment apart from reliable channels. It allows higher-level services to leverage speculative
command execution to ensure uninterrupted progress during the reconfiguration periods as well as in situations where failures prevent the reconfiguration agreement from being reached in a timely fashion. We
apply our framework to obtain a reconfigurable speculative state machine from the non-reconfigurable
Paxos implementation, and analyze its performance on a realistic distributed testbed. Our results show
that our framework incurs negligible overheads in the absence of reconfiguration, and allows steady
throughput to be maintained throughout the reconfiguration periods.

Eligible for the best student paper award: Dmitri Perelman is a full-time graduate student

The paper includes 12.5 pages out of which approximately 2 are occupied by the figures.

1

Introduction

Replicated state machine, or RSM [11], is an important tool for maintaining integrity of distributed applications and services in failure-prone data center and cloud computing environments. In these settings, the
infrastructure needs to adapt to changing resource availability, load fluctuations, variable power consumption, and data locality constraints. In order to meet these requirements, RSM must support reconfiguration,
i.e., dynamic changes to replica set, or quorum system. It is essential to ensure that reconfiguration incurs
minimum disruption to availability and performance, in order to enable building truly elastic services. The
ability to perform reconfigurations in a non-disruptive fashion provides system designers with a powerful
paradigm that can enable many optimizations. This includes proactive replacement of suspected or slow
nodes at low cost, adapting to the changing environment characteristics (e.g. network delay, or diurnal load
fluctuations, and many others).
Reconfigurable RSM has been proposed in multiple contexts (e.g., [14, 15, 10]). Each solution implemented a slightly different set of requirements, and all proved nontrivial. Designing this functionality for a
realistic environment with minimal impact on performance is even more challenging. Naı̈ve constructions
follow the “brick-wall approach” in which the flow of user commands is stalled until the new configuration
is installed and the state is transferred to it. Ideally, systems should strive to avoid this, and favor implementations with near-seamless hand-on, that maintain steady throughput and latency during the transition
periods.
This paper introduces a framework for constructing reconfigurable state machines from collections of
non-reconfigurable ones. We follow the black box approach that assumes nothing about the execution environment except the existence of reliable communication channels. Our reduction is both simple and generic,
i.e., the underlying RSM implementation is completely opaque to the framework. Furthermore, it does
not compromise efficiency, incurring negligible overhead in the absence of reconfigurations and avoiding
system stalls upon reconfiguration.
The main ideas underlying our framework are as follows. Each newly proposed configuration is associated with its own instance of RSM, and all active RSM are executed concurrently to each other. The
globally consistent trunk of commands is created by gluing together the totally ordered command sequences
produced by each RSM. When switching from one RSM to another, the latter is chosen based on the outcome of the configuration agreement in the former. Our framework also relieves RSMs of state transfer
responsibilities by ensuring that the latest trunk is transferred to the new configuration concurrently with
the RSM execution. This way, each newly created RSM is completely independent from its predecessor,
and in particular, can start executing from its initial state. We leverage this capability in our Paxos-based
reconfigurable RSM implementation to supply each newly created Paxos instance with the identifier of a
deterministically chosen leader thus eschewing the first phase of Paxos if the configured leader does not fail.
Another important optimization made possible by the RSM independence is the ability to speculatively
overlap their execution with the reconfiguration protocol, thus considerably reducing the command latency
during reconfiguration periods. Specifically, each RSM is made available for accepting commands for the
new configuration as soon as it is proposed, and without waiting for it to be agreed upon by the parent RSM.
The proposals associated with the new configuration proceed to be ordered concurrently with the reconfiguration agreement, and are added to the trunk as soon as the configuration is agreed upon. This way, our
framework allows unbounded degree of parallelism in the command execution during the transition periods
avoiding the performance problems of the “brick-wall” solutions. In addition, the benefits of speculation become more substantial as the network delay grows thus making speculative solutions attractive in wide-area
network settings.

1

The modularity of our framework enables a range of additional features useful in practical settings. One
such feature is supporting rolling software upgrades: i.e., the implementation of the deployed RSMs can
be replaced with the newer one without stopping the system. Likewise, misbehaving or buggy RSMs can
be restarted or replaced on-the-fly with the minimum impact on the system operation as per the recoveryoriented computing (ROC) [19] guidelines.
We used our framework to implement a full-fledged reconfigurable replication platform using nonreconfigurable Paxos [12, 13] as its underlying non-reconfigurable RSM, and experimentally studied its
performance. The results demonstrate that our system achieves high throughput and low latency in the absence of reconfigurations, which stay almost unchanged under highly dynamic reconfiguration scenarios.
Specifically, the throughput is unaffected in the runs with reconfiguration rate of 5 per second, and degrades
only by 20% when reconfiguration rate achieves that of 20 per second. In addition, our study indicates that
the command latency in the vicinity of reconfiguration stays the same as that in the absence thereof.
In summary, the contributions made by our work are as follows:
• new generic framework for constructing reconfigurable RSM from non-reconfigurable ones, which is
simple, modular, and efficient;
• new speculative approach to enable overlapping command ordering with the reconfiguration protocol
thus reducing the reconfiguration latencies;
• practical implementation that leverages our framework to transform Paxos-based RSM to a highly
efficient reconfigurable RSM;
• detailed experimental study of the implementation above demonstrating the benefits of our framework
in practical settings.
The rest of the paper is organized as follows. Related work is discussed in Section 2. System model
is presented in Section 3. Section 4 introduces non-reconfigurable and reconfigurable variants of RSM,
and rigorously specify their properties. In Section 5, we present our reduction algorithm, and argue its
correctness. The replication platform built on top of our framework, and its performance are discussed in
Section 6. Section 7 concludes the paper.

2

Related work

Modular approaches to specifying RSM and their constituent building blocks have been addressed in several
prior works. Boichat et al. [5] introduced modular decomposition of Paxos into weak leader election and
round-based consensus abstraction, and [7] studied the abstraction of ranked register capturing the essence of
the Paxos consensus algorithm. These papers however, do not address reconfiguration. Stoppable Paxos [10]
presents a framework for combining non-reconfigurable Paxos instances into a reconfigurable RSM, but does
not attempt to abstract away the internals of the underlying Paxos implementation. In addition, [10] does
not support concurrent execution of the individual Paxos instances.
Several approaches to alleviating reconfiguration bottleneck in reconfigurable state machines have been
proposed. The original idea by Lamport, described in [12, 13, 14], and implemented in SMART [16], was
to delay the effect of the configuration agreed in a specific consensus instance by a fixed number α of
successive consensus instances. If the configuration must take effect immediately, the remaining instances
can be skipped by passing a special “window closure” decree consisting of α consecutive noop instances.
Although this approach allows up to α consecutive commands to be executed concurrently, choosing the
2

right value of α is nontrivial. On the one hand, choosing α to be too small may under-utilize the available
resources. On the other hand, large values of α may not match the actual service reconfiguration rate
resulting in too frequent invocations of the window closure decrees (which must complete synchronously).
Chubby [6] and ZooKeeper [9] expose high-level synchronization primitives (respectively, locks and
watches) that can be used to implement a reconfigurable state machine within the client groups. The solutions based on this approach are however, vulnerable to timing failures, and therefore, either restrict their
failure model [25], or rely on additional synchronization protocols within the replication groups themselves
to maintain consistency [15, 21].
Vertical Paxos [15] removes the configuration agreement overhead from the critical path by delegating it
to an auxiliary “configuration master”. The reconfiguration involves an extra step of synchronizing with the
read quorums of all preceding configurations causing throughput degradation. In addition, the configuration
master itself is implemented using the α-based reconfigurable Paxos protocol, and therefore, suffers from
the limitations similar to those discussed above.
Dynamic reconfiguration has been extensively studied in the context of virtually synchronous group
communication [22, 4, 8, 20], and reconfigurable read/write registers [17, 1]. The reconfiguration protocols described in these papers do not aim to support consistency semantics as strong as those of state
machine replication, and therefore do not directly apply in our context. Birman et al. [3] present a replication framework unifying reconfigurable state machine and virtual synchrony in which the normal operation
is suspended during the reconfiguration periods.
Optimistic and speculative approaches to mask the coordination latency have been extensively studied
in the past in a variety of contexts (such as e.g., group communication [23], and database replication [2]).
However, to the best of our knowledge, speculative reconfigurable state machine replication studied in this
paper has not yet been addressed in the prior work.

3

System Model

We consider an asynchronous message-passing system consisting of the (possibly infinite) set of processes
P . Each pair of processes is connected by a point-to-point FIFO channel. For simplicity, we assume the
crash failure model: i.e., the process experiencing a crash failure stops executing any further instructions
forever. A process that never crashes throughout the system execution is called correct. The channels are
reliable in the sense that a message sent by a correct process p to another correct process q is eventually
received by q. Extending our techniques to support stronger benign failure models (such as crash/recovery,
message omission, and network partitions and merges) is straightforward, and our prototype implementation
described in Section 6 is capable of doing that.

4

State Machine Replication

In this section, we introduce the notions of non-reconfigurable and reconfigurable Replicated State Machines
(RSM), and specify their properties. Our specification style loosely follows the I/O automata formalism
of [18].
For the following we will fix Command to be the set of the user commands, and Config be the set of
the configuration identifiers. Each configuration C is associated with a finite set of processes in P , denoted
members(C).

3

Inputs:
joinC,p
propose(cmd)C,p , cmd ∈ Command
Outputs:
learn(cmd)C,p , cmd ∈ Command
Figure 1: The NR - RSM(C)p , C ∈ Config, signature for process p ∈ members(C):

4.1

Non-Reconfigurable RSM (NR - RSM)

The Non-Reconfigurable Replicated State Machine (NR - RSM) is parametrized by a configuration C ∈
Config. The interface supported by NR - RSM(C) for each process p ∈ members(C) appears in Figure 4.1.
NR - RSM (C) accepts the commands cmd ∈ Command submitted by the C’s members through the propose(cmd)C ,
requests and outputs a totally ordered sequence of learn(cmd)C notifications.
The process p’s instance of NR - RSM(C), denoted NR - RSM(C)p , is activated by the joinC,p request.
NR - RSM (C) becomes operational once sufficiently many members p of C have executed the joinC,p request. The precise initial membership depends on the NR - RSM implementation being used, desired resiliency, failure model, and environment properties. For example, if NR - RSM is implemented using the
non-reconfigurable Paxos algorithm [13], NR - RSM(C) may become operational as soon as a majority of
members(C) have executed joinC,p . We do not explicitly model the process “leaves” since they are equivalent to the process crashes in the crash failure model.
We now specify the set of properties that must be satisfied by the correct implementations of NR - RSM.
The properties are defined in terms of the sequences of actions α of its external interface in Figure 4.1. We
start by specifying the environment assumptions that must hold in order for the NR - RSM implementation to
be correct:
Property 1. (Well-Formedness)
• If e = proposeC,p is an event in α, then there exists joinC,p that precedes e in α.
• For each process p ∈ members(C), at most one joinC,p occurs in α.
• For each command cmd ∈ Command, at most one propose(cmd)∗,∗ occurs in α.
Next, we specify the NR - RSM(C) safety properties. The Integrity property below asserts the following
two facts: (1) a process cannot output any learn events before it has joined NR - RSM(C), and (2) each learnt
command must be previously proposed. Formally,
Property 2 (Integrity). Let e = learn(cmd)C,p be an event in α. Then, the following holds:
• There exists joinC,p event that precedes e in α.
• There exists propose(cmd)C,q event that precedes e in α.
The next property states that each command is learnt at most once by each process:
Property 3 (No Duplication). For each command cmd ∈ Command and process p, there exists at most one
learn(cmd)p event in α.
4

The following property requires that the commands are learnt in the same order by all members of C.
Formally:
Property 4 (Linearizability). There exists a sequence of commands x̄ = x1 , x2 , . . . such that for each
process p ∈ members(C), if π̄ = π1 , π2 , . . . is the sequence of the learnp events output by p in α, then, for
all i ≥ 1, πi = learn(xi )C,p .
We now turn to specifying the NR - RSM liveness. The property below asserts that after some time t
(which is the parameter of the property), each command proposed by a correct member of C is eventually
learnt by all correct members of C provided they have joined NR - RSM(C). Formally, NR - RSM is said to be
live after time t in α if the following holds:
Property 5 (Liveness). If propose(cmd)p is an event occurring at a correct process p after t, then for each
correct process q ∈ members(C) such that joinC,q is an event in α, there exists learn(cmd)C,q that occurs
at time t0 > t in α.

4.2

Reconfigurable RSM (R - RSM)
Inputs:
propose(C, cmd)p , C ∈ Config, cmd ∈ Command, p ∈ members(C)
recon(C, C 0 )p , C, C 0 ∈ Config, p ∈ members(C)
Outputs:
learn(cmd)p , cmd ∈ Command
new-conf(C)p , C ∈ Config
ready(C)p , C ∈ Config, p ∈ members(C)
Figure 2: The R - RSM Signature for Process p ∈ P :

The R - RSM external interface appears in Figure 4.2. In contrast to the non-reconfigurable RSM, the
configuration of Reconfigurable RSM (R - RSM) can be changed dynamically through the user inputs. Initially, the R - RSM’s configuration is the distinguished configuration C0 . Afterwards, new configurations are
proposed through the recon(C, C 0 ) requests, which include two parameters: a known configuration C, and
the target configuration C 0 to which C is requested to reconfigure.
The recon requests can only be initiated at one of the members of C. The members of C 0 respond with
ready(C 0 ) events once they are aware of C 0 , and ready to accept proposals associated with C 0 . If C 0 is
eventually accepted as the configuration to follow C in the global order, the members q of both C and all of
the subsequent configurations (including C 0 ) will be notified of that through the new-conf(C 0 )q events.
Similarly to NR - RSM, R - RSM accepts commands submitted through the propose requests, and outputs
learn notifications for the commands whose slot in the total order has been finalized. In addition, each
propose request also includes the configuration parameter that indicates the desired configuration to be used
to order the provided command.
Note that the R - RSM interface allows the proposals (of both commands and configurations) to be issued
against configurations, which have been reported as ready, but whose ordering has not yet been completed
(and may subsequently fail). This provides implementations with flexibility in choosing how to deal with the
proposals that have been received by R - RSM while reconfiguration is still in progress. In our implementation
(see Section 5), these proposals are ordered speculatively using a separate instance of the non-reconfigurable
5

C0
C1

C4

C2

C3

C5 C6

C7

C9

C10

C8

Figure 3: The configuration tree formed by the recon events. The trunk configurations are highlighted.
state machine being executed concurrently with the reconfiguration protocol. As we show in Section 6, this
leads to substantial reductions in the reconfiguration delays provided the proposed configuration will be
eventually included into the total order (which is a common occurrence in practice). The implementation
can limit or completely eliminate speculative behavior by tweaking the timing of the ready reports.
All configurations submitted in the course of the R - RSM execution α form the tree with the root C0 ,
and C being the parent of C 0 for all C, C 0 ∈ Config iff recon(C, C 0 ) has occurred in α (see Figure 3).
The configurations reported in the new-conf notifications form a continuous path, called the trunk, in the
configuration tree starting from C0 .
As before, we specify the correctness properties for the R - RSM implementation in terms of the sequences
of actions α of its external interface in Figure 4.2. We start by defining the well-formedness conditions,
which capture the permissible interaction patterns between the R - RSM and its environment:
Property 6. (Well-Formedness)
• If e ∈ {propose(C, ∗)p , recon(C, ∗)p } is an event in α, then there exists ready(C)p that precedes e in
α unless C = C0
• For each command cmd ∈ Command, there exists at most one propose(cmd)∗ in α.
• For each configuration C ∈ Config, there exists at most one recon(∗, C)∗ in α1 .
Next, we specify the R - RSM safety properties. The Integrity property below asserts that each learn (resp.,
new-conf) notification delivered at a process p must be preceded by both: (1) p’s join, and (2) propose(C, ∗)
(recon(C, ∗)) where C is the latest configuration that has been delivered at p before the notification event.
Formally:
Property 7 (Integrity). Let e ∈ {learn(cmd)p , new-conf(C 0 )p , ready(C 0 )p }, be an event in α, and C be the
configuration delivered by the latest new-conf event that precedes e at p. Then, all of the following holds:
• If e = learn(cmd)p , then there exists q ∈ members(C) such that propose(C, cmd)q (resp., recon(C, C 0 )q )
is an event in α.
• If e ∈ {new-conf(C 0 )p , ready(C 0 )p }, then there exists q ∈ members(C) such that recon(C, C 0 )q ) is
an event in α.
1

Note that only the proposed configuration identifiers are required to be unique, not the set of members associated with those
identifiers.

6

The next property states that each command (resp., configuration) is learnt (resp., delivered) at most
once by each process:
Property 8 (No Duplication). For each cmd ∈ Command, there exists at most one learn(cmd)p , and for
each C ∈ Config, there is at most one ready(C)p , and at most one new-conf(C)p event that occurs at each
process p ∈ P in α.
The most important safety property supported by R - RSM is linearizability, which requires that all processes deliver the same sequence of learn and new-conf events. Formally,
Property 9 (Linearizability). There exists a sequence of commands and configurations x̄ = x1 , x2 , . . . such
that for each process p ∈ P , if π̄ = π1 , π2 , . . . is the sequence of the learnp and new-confp events output
by p in α, then for all i ≥ 1, if xi ∈ Config, then πi = new-conf(xi )p , and if xi ∈ Command, then
πi = learn(xi )p .
We now specify the R - RSM liveness. The following property says that if every proposed configuration
includes at least one correct member, then eventually, the system stabilizes in the sense that (1) each command proposed by a correct process is eventually learnt provided new reconfigurations attempts cease to
be initiated, and (2) each proposal to reconfigure a previously installed configuration eventually triggers a
new-conf event. Formally,
Property 10 (Liveness). Suppose that every configuration proposed in α includes at least one correct member. Then, there exists Global Stabilization Time (GST) such that for all times t > GST , if C0 , C1 , . . . , Ck
is the sequence of configurations induced by the longest sequence of new-conf events delivered by some
process before t, then, both of the following holds:
• If no recon(C, ∗) events with C = Ck occur after GST , then for each correct p ∈ members(Ck ), if
propose(Ck , cmd)p is an event that occurs after t in α, then it is eventually followed by learn(cmd)q
at all correct q ∈ members(Ck ).
• There exists configuration C such that if a correct p ∈ members(Ck ) invokes recon(Ck , ∗)p after t,
then it is eventually followed by new-conf(C)q at all correct q ∈ members(Ck ) ∪ members(C).
In addition, we require that each recon(∗, C) invoked by a correct process is eventually followed by the
ready(C) notification delivered at each correct member of C.
Property 11. If recon(∗, C)p is an event that occurs at a correct process p in α, then it is followed by
ready(C)q for each correct q ∈ members(C).

5

Reduction Algorithm

In this section, we present the implementation of the algorithm to transform a collection of NR - RSM(C),
C ∈ Config to R - RSM, and argue its correctness. The implementation is the composition of the automata
R - RSM p for each process p ∈ P . The code executed by each R - RSM p appears in Algorithm 1.
The implementation of R - RSM can be viewed as consisting of the following three phases:
Configuration-specific ordering: Each proposed configuration C is associated with the non-reconfigurable
state machine NR - RSM(C), which is used to order the commands and configurations associated with
C. The local outputs of each NR - RSM(C)p are stored in the branchesp data structure (line 1.7), which
maps each C to the sequence of commands and configurations output by learnC,p events.
7

Algorithm 1 R - RSM from {NR - RSM(C)}. The R - RSMp code.
40:
for all i = length(trunk) to length(tr) − 1 do
41:
learnNext(tr[i])
42:
end for
43:
view ← view ∪ v ∪ {q}
44: end upon

1: Types
2:
Config: the set of configuration identifiers with the initial identifier C0
3:
Command: the set of command indetifiers
4: var
5:
status ∈ {active, idle}
6:
view ⊆ P
7:
branches: mapping from Config to seqOf(Config ∪ Command) ∪
{⊥}
8:
trunk: seqOf(Config ∪ Command)
9:
next ∈ N≥0
10:
curConf ∈ Config

45: upon receiving hJOIN, C, C 0 i message from q ∈ members(C) do
46:
if branches(C 0 ) = ⊥ then
47:
joinConf(C 0 )
48:
Output ready(C 0 )
49:
view ← view ∪ members(C) ∪ members(C 0 )
50:
end if
51: end upon

11: Initialisation:
12:
branches(C) ← ⊥ for all C ∈ Config
13:
trunk ← []
14:
next ← 0
15:
curConf ← C0
16:
view ← ∅
17:
if p ∈ members(C0 ) then
18:
joinConf(C0 )
19:
view ← members(C0 )
20:
end if

52: task State Transfer
53:
if status = active then
54:
Periodically:
55:
for all q ∈ view do
56:
send(hSTATE, trunk, viewi)p,q
57:
end for
58:
end if
59: end
60: procedure learnNext(x), x ∈ Command ∪ Config
61:
Append x to trunk
62:
if x ∈ Config then
63:
if branches(x) = ⊥ ∧ p ∈ members(x) then
64:
joinConf(x)
65:
Output ready(C)
66:
view ← view ∪ members(x)
67:
end if
68:
Output new-conf(x)
69:
next ← 0
70:
curConf ← x
71:
else
72:
Output learn(x)
73:
next ← next + 1
74:
end if
75: end

21: propose(C, cmd):
22:
if branches(C) 6= ⊥ ∧ branches(C)[i] 6∈ Config for all 0 ≤
i ≤ length(branches(C)) then
23:
propose(cmd)C
24:
end if
25: recon(C, C 0 ):
26:
if branches(C) 6= ⊥ ∧ branches(C)[i] 6∈ Config for all 0 ≤
i ≤ length(branches(C)) then
27:
for all q ∈ members(C 0 ) do
28:
send(hJOIN, C, C 0 i)p,q
29:
end for
30:
propose(C 0 )C
31:
view ← view ∪ {C 0 }
32:
end if
33: upon learn(x)C : do
34:
Append x to branches(C)
35:
if C = curConf ∧ next < length(branches(curConf)) then
36:
learnNext(branches(curConf)[next])
37:
end if
38: end upon

76: procedure joinConf(C), C ∈ Config
77:
if status = idle then
78:
status ← active
79:
end if
80:
branches(C) ← []
81:
joinC
82: end

39: upon receiving hSTATE, tr, vi message from q ∈ P do

Configuration tree pruning: The branches of the configuration tree in Figure 3 are pruned so that the
surviving configurations form a single consistent trunk. To ensure that all processes prune the branches
in a consistent fashion, the successor C 0 of each configuration C is determined by the output of
NR - RSM (C).
Total order construction: Each process p concatenates the totally ordered fragments stored in branchesp in
the order induced by the configuration trunk to produce a globally-consistent sequence of commands
and configurations, which is stored in the trunkp variable. trunkp is then iterated to produce a globally
consistent sequence of the learn and new-conf outputs.

8

The above three phases are executed concurrently, with the configuration-specific ordering phase being
executed concurrently for each individual configuration. Below we describe the implementation of each of
the above phases in more detail.
Configuration-Specific Ordering: A process p joins NR - RSM(C) when it either receives the hJOIN, Ci
message (lines 1.46–1.50), or encounters C in the course of the total order construction phase (lines 1.63–
1.67). From that point on, all commands and configurations x submitted by process p through either
propose(C, x)p or recon(C, x)p are forwarded to the p’s local instance of NR - RSM(C) through the propose(x)C,p
requests (see lines 1.23 and 1.23). Each ordered command or configuration delivered by the learnC,p event
is then appended to branches(C)p .
The state machines created upon the reception of the JOIN messages are used to speculatively order the
proposals corresponding to the configurations, which have not yet been incorporated into the global total
order, thus reducing the reconfiguration time. We discuss speculation in more detail in Section 5.3 below.
Configuration Tree Pruning: The configuration tree is pruned by selecting the first configuration appearing in branches(C)p to be the C’s successor by all processes p ∈ members(C). Since all entries of
branches(C)p appear in the same order at all processes p ∈ members(C), they all will choose the same configuration C 0 s as the C’s successor. All other processes will learn of C 0 through state transfer (see below).
The successor chosen for each configuration C becomes explicit when it is incorporated into trunkp in the
course of the total order construction phase (lines 1.68–1.67).
Total Order Construction: The chain of the surviving branches currently known to each process p is
kept in the trunkp variable (see line 1.8). In addition, curConfp (see line 1.10) holds the configuration
corresponding to the last branch on trunkp . Initially, curConfp = C0 (line 1.15) so that trunkp consists of
a single branch corresponding to the root of the configuration tree C0 .
As the entries are added to branches(curConfp ) (lines 1.34–1.37), they are copied to trunkp (line 1.61)
and learnt one-by-one (line 1.72–1.73) until a new configuration C is encountered. At this point, curConfp
is reassigned to C thus choosing branches(curConfp ) as the next branch to feed trunkp (lines 1.68–1.67).
Since as we argued above, C is the only configuration that can be chosen as the curConfp ’s successor at
all processes q ∈ P , branches(C) can be the only one selected to follow branches(curConfq ) in trunkq . We
therefore, proved the following:
Lemma 1. All well-formed executions of the R - RSM implementation in Algorithm 1 satisfy Property 9

5.1

State Propagation

Since for each configuration C, the commands and configurations ordered by NR - RSM(C) are only delivered
to the members of C, we need an additional mechanism to ensure they propagate down the configuration
trunk. This mechanism is implemented by the State Transfer task (lines 1.52– 1.59), which is executed in
the background at each process p that have joined at least one of the proposed configurations.
It proceeds by continuously gossipping its entire trunkp (line 1.56) to the set of processes being accumulated in the viewp variable2 . By line 1.66, viewp is guaranteed to include the members of all configurations
comprising the configuration trunk known to p. In particular, this includes the members of the configuration
2

Note that in reality the state transfer can be made considerably more efficient through a variety of techniques, such as e.g.,
push-pull gossip[24], and log contraction [14]

9

C 0 that immediately follows C in trunkp . If p is correct, all correct members of C 0 will eventually receive
trunkp that will include the entire sequence of commands ordered within C up to and inclusively of C 0 .
Moreover, once there is a correct q ∈ members(C 0 ) that delivers new-conf(C 0 ), all correct members of both
C 0 and its immediate successor (if any) are guaranteed to receive the entire trunk known to the members of
C even if all members of C stop gossipping their trunks. We therefore, have the following:
Lemma 2 (State Propagation). Let C and C 0 be two configurations such that C 0 follows C in the configuration trunk. Then, if there exists a correct member p of C that continues to gossip trunkp until new-conf(C 0 )
is delivered by a correct member q of C 0 , then all correct members of C 0 will eventually incorporate all
commands and configurations ordered within C into their trunks.
The result above establishes criteria that can be used in practice for determining when the members of
old configurations can be taken off line without compromising the system liveness. For example, suppose
that a majority of the members of each installed configuration do not crash, unless they are taken off line in
an orderly fashion. Then, for each configuration C, a minority of members(C) can be taken off line once
a majority of members(C) deliver new-conf(C 0 ) notification for some configuration C 0 ; and the remaining
members can be disconnected once a majority of C 0 delivers new-conf(C 0 ).

5.2

Liveness

The liveness of our implementation follows from the liveness of its constituent non-reconfigurable state
machines and the State Propagation property above.
Specifically, assume that all state machines instantiated in the course of the execution of the code in
Algorithm 1 become live (in the sense of Property 5) after some time t. Suppose that C is the last configuration of the system. Since all configurations include at least one correct process, by Lemma 2, there is a time
t0 > t such that all correct members of C will incorporate C into their trunks and join NR - RSM(C) before
t0 .
If no new reconfiguration attempts are made from this point onwards, every command cmd proposed
by a correct member of C through propose(C, cmd) is guaranteed to appear in branches(C)q , followed by
trunkq , until it eventually output through learn(cmd)q . Otherwise, there will be C 0 such that recon(C, C 0 )
will result in C 0 to be included into trunkp of all correct processes in C, from where by Lemma 2, it is
guaranteed to eventually propagate to all correct members of C 0 . Hence, we proved the following:
Lemma 3. Each well-formed execution α of the R - RSM implementation in Algorithm 1 satisfies Property 10
provided all NR - RSM implementations instantiated in α are live after some t.

5.3

Speculation

One important property of our R - RSM implementation is that the non-reconfigurable state machine for each
configuration C is completely independent from the state machines for other configurations, and could proceed ordering the commands and configurations submitted under C concurrently with them. In particular,
since the members of C are notified of the new configuration immediately upon receiving its corresponding reconfiguration request (line 1.27– 1.29), this also applies to the commands submitted under C while
the successor of its parent configuration is still being chosen. These commands will be available for incorporating into the trunk and subsequent learning as soon as C becomes the chosen successor of its parent
configuration, thus reducing (or eliminating altogether) reconfiguration delays.
The precise degree of the performance improvement depends on the choice of the underlying nonreconfigurable state machine implementation, and the network parameters (primarily, the communication
10

delay). As we show in Section 6, for the Paxos-based R - RSM implementation, speculation helps to eliminate penalties associated with throughput and latency during reconfiguration times, making them almost
indistinguishable from those achieved during the normal operation.

6

Implementation and Evaluation

We applied our reduction framework to implement a full-fledged reconfigurable replication platform using
non-reconfigurable Paxos [12, 13] as its underlying NR - RSM implementation.
To shield the users from intricacies of selecting viable configurations, our replication platform augments
R - RSM with two additional modules: Configuration Manager (CM) and Command Queue (CQ). CM implements the logic to detect failures, and produce new configurations, which are then submitted to R - RSM
through the recon requests. The configuration membership is based on the failure suspicions and user policies. An example policy may ask for every configuration to include sufficiently many healthy nodes. CQ is
responsible for associating user commands with configurations and submitting them to the platform through
the propose requests. The configurations are chosen based on the ready notifications thus leveraging the
speculation capabilities of R - RSM to reduce reconfiguration delays.
Each Paxos instance created by R - RSM is supplied with the identifier of a deterministically chosen leader
thus avoiding the overheads of the first phase of Paxos if the configured leader does not fail. Thus, normally,
only the second phase of Paxos is needed to complete each individual agreement instance resulting in a
single round-trip delay to order a command.
We studied the performance of our implementation experimentally using the testbed comprised of 4
IBM HS22 blades equipped with Intel Xeon X5670 processor with 24 2.93GHz cores and 64GB RAM.
Each machine was equipped with 1GB network card, and ran Red Hat Linux. A single replica was hosted
on each machine.
The replicas were subjected to request streams generated in either synchronous or asynchronous fashion.
In the synchronous mode, the requests were issued from multiple threads each of which was waiting for the
response to the previously submitted request to arrive before submitting the new one. The synchronous mode
allowed us to exercise high degree of control over the offered system load by varying the number of simultaneously executed synchronous threads. In contrast, in the asynchronous mode, each thread was submitting
requests as they were generated without waiting for the prior requests completion. The asynchronous mode
was used to drive the system to its maximum utilization.
In the first experiment, we studied the throughput and latency as a function of the offered system load
in the absence of reconfiguration using a single configuration consisting of 3 replicas. The results (see
Figure 4) show that our system is capable of achieving the sustained throughput of 12K request/second
before the latency starts to rapidly grow.
In the next experiment, we studied the impact of our speculative reconfiguration mechanism on the
command throughput. For that, the system was subjected to a series of reconfiguration requests submitted at
varied frequency. The normal commands were submitted using 10 synchronous threads, which corresponds
to the maximum sustained system load of 12K requests/second (see Figure 4). The measurements are
depicted (see Figure 5) as absolute command throughput in the presence of reconfiguration, and percentage
of degradation relative to the maximum sustained throughput (12K). The results show that the throughput
in the presence of reconfigurations is almost identical to that achieved in the absence thereof, reaching
maximum 20% of degradation when the system is reconfigured 20 times/second.
In the last experiment, we studied the degree of improvement produced by speculation in terms of the
latencies of the commands submitted in close proximity of the reconfiguration requests. The results (see
11

Average Throughput

Average Command Latency

16000

5

4.5

14000

4
12000

3

8000

2.5

Latency (ms)

Throughput (req/s)

3.5
10000

2

6000

1.5
4000
1
2000

0.5

0

0
1

2

3

4

5

6

7

8

9

10

12

14

16

18

20

25

30

35

40

45

50

55

Load (# of concurrent clients)

Figure 4: Throughput and Latency in the Absence of Reconfiguration.
Speculative Reconfiguration (throughput)

Speculative Reconfiguration (%)

14000

120

12000

100

10000

Throughput (1/s)

80
8000
60 %
6000
40
4000

20

2000

0

0
2

5

10

20

Reconfiguration Frequency (1/s)

Figure 5: Throughput With Varied Reconfiguration Frequency.
Figure 6) indicate that with speculation, the latency is unaffected by reconfiguration staying closely to that
of the normal mode (i.e., in the absence of reconfiguration) throughout the entire reconfiguration period. In
contrast, without speculation, the latency increases sharply for the first command, and then keeps decreasing
slowly until reaching the normal mode value in the proximity of the 100th command.

7

Conclusions

We have introduced a modular framework for transforming a collection of arbitrary non-reconfigurable
replicated state machines (RSM) into the reconfigurable RSM implementation. Our framework follows
the black box approach, and does not make any assumptions with respect to its execution environment
apart from reliable channels. The individual state machines instantiated by our implementation can be
executed concurrently to one another, in particular, overlap each other execution in a speculative fashion
to mask the reconfiguration delays. We have applied our framework to build a prototype of an end-to-end
dynamic replication platform using non-reconfigurable Paxos as its underlying building block, and studied
12

Non-Speculative Reconfiguration

Speculative Reconfiguration

31

71

450
400
350

Latency (ms)

300
250
200
150
reconfigurations

100
50
0
1

11

21

41

51

61

81

91

101

111

Commands

Figure 6: Command Latency in the Vicinity of Reconfiguration. The network delay was simulated to be
100ms on average.
its performance on a realistic distributed testbed. Our results demonstrated that our our platform achieves
high throughput and low latency in the absence of reconfigurations, which stay almost unchanged under
highly dynamic reconfiguration scenarios.
Our platform is being developed at IBM Research, and slated to be included into the future IBM’s
platform-as-a-service offerings as a foundational tool. In the future, we intend to explore the ways to extend
our speculation-based approach to support optimistic replication in wide-area network settings, and augment
it with an on-line optimization framework to facilitate selection of configurations based on the changing
network parameters.

13

References
[1] M. K. Aguilera, I. Keidar, D. Malkhi, and A. Shraer. Dynamic atomic storage without consensus. J.
ACM, 58(2):7, 2011.
[2] Y. Amir and C. Tutu. From total order to database replication. In ICDCS, pages 494–, 2002.
[3] K. Birman, D. Malkhi, and R. van Renesse. Virtually synchronous methodology for dynamic service
replication. Technical Report MSR-TR-2010-151, Microsoft Research, 2010.
[4] K. P. Birman and T. A. Joseph. Exploiting virtual synchrony in distributed systems. In SOSP, pages
123–138, 1987.
[5] R. Boichat, P. Dutta, S. Frølund, and R. Guerraoui. Deconstructing paxos. SIGACT News, 34(1):47–67,
2003.
[6] M. Burrows. The chubby lock service for loosely-coupled distributed systems. In Proceedings of the
7th symposium on Operating systems design and implementation, OSDI ’06, pages 335–350, Berkeley,
CA, USA, 2006. USENIX Association.
[7] G. Chockler and D. Malkhi. Active disk paxos with infinitely many processes. Distributed Computing,
18(1):73–84, 2005.
[8] D. Dolev, I. Keidar, and E. Y. Lotem. Dynamic voting for consistent primary components. In Proceedings of the sixteenth annual ACM symposium on Principles of distributed computing, PODC ’97,
pages 63–71, New York, NY, USA, 1997. ACM.
[9] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed. Zookeeper: wait-free coordination for internet-scale
systems. In Proceedings of the 2010 USENIX conference on USENIX annual technical conference,
USENIXATC’10, pages 11–11, Berkeley, CA, USA, 2010. USENIX Association.
[10] L. Lampor, D. Malkhi, and L. Zhou.
Stoppable paxos.
http://research.microsoft.com/apps/pubs/default.aspx?id=101826, 2008.

[Online]. Available:

[11] L. Lamport. Time, clocks, and the ordering of events in a distributed system. Commun. ACM, 21:558–
565, July 1978.
[12] L. Lamport. The part-time parliament. ACM Trans. Comput. Syst., 16(2):133–169, 1998.
[13] L. Lamport. Paxos made simple. ACM SIGACT News, 32(4):18–25, December 2001.
[14] L. Lamport, D. Malkhi, and L. Zhou. Reconfiguring a state machine. Technical report, Microsoft
Research, 2008.
[15] L. Lamport, D. Malkhi, and L. Zhou. Vertical paxos and primary-backup replication. In PODC ’09:
Proceedings of the 28th ACM symposium on Principles of distributed computing, pages 312–313, New
York, NY, USA, 2009. ACM.
[16] J. R. Lorch, A. Adya, W. J. Bolosky, R. Chaiken, J. R. Douceur, and J. Howell. The SMART way to
migrate replicated stateful services. In In Proc. EuroSys06, 2006.

14

[17] N. Lynch and A. A. Shvartsman. Rambo: A reconfigurable atomic memory service for dynamic
networks. In In DISC, pages 173–190, 2002.
[18] N. Lynch and M. Tuttle. An introduction to input/output automata. CWI-Quarterly, 2(3):219–246,
1989.
[19] D. A. Patterson, A. Brown, P. Broadwell, G. Candea, M. Chen, J. Cutler, P. Enriquez, A. Fox,
E. Kiciman, M. Merzbacher, D. Oppenheimer, N. Sastry, W. Tetzlaff, J. Traupman, and N. Treuhaft.
Recovery-oriented computing (roc): Motivation, definition, techniques, and case studies. Technical
report, UC Berkeley Computer Science Technical Report UCB//CSD-02-1175, 2002.
[20] R. D. Prisco, A. Fekete, N. A. Lynch, and A. A. Shvartsman. A dynamic primary configuration group
communication service. In DISC, pages 64–78, 1999.
[21] J. Rao, E. J. Shekita, and S. Tata. Using paxos to build a scalable, consistent, and highly available
datastore. Proc. VLDB Endow., 4:243–254, January 2011.
[22] A. Ricciardi and K. Birman. Process Membership in Asynchronous Environments. Technical Report
TR93-1328, Cornell University, February 1993.
[23] J. Sussman and K. Marzullo. The bancomat problem: an example of resource allocation in a partitionable asynchronous system. Theor. Comput. Sci., 291:103–131, January 2003.
[24] R. van Renesse, D. Dumitriu, V. Gough, and C. Thomas. Efficient reconciliation and flow control for
anti-entropy protocols. In Large-Scale Distributed Systems and Middleware (LADIS 2008), September
2008.
[25] R. van Renesse and F. B. Schneider. Chain replication for supporting high throughput and availability.
In Proceedings of the 6th conference on Symposium on Opearting Systems Design & Implementation
- Volume 6, pages 7–7, Berkeley, CA, USA, 2004. USENIX Association.

15

