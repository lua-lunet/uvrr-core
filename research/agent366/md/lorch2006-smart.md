     The SMART Way to Migrate Replicated Stateful Services

                            Jacob R. Lorch, Atul Adya, William J. Bolosky, Ronnie Chaiken,
                                          John R. Douceur, and Jon Howell
                                                                 Microsoft Research
                                          {lorch, adya, bolosky, rchaiken, johndo, howell}@microsoft.com




Abstract                                                                             cate it on several machines. However, replication can only
  Many stateful services use the replicated state machine                            mask a limited number of failures, and the longer the service
approach for high availability. In this approach, a service                          runs the more likely the failure count will exceed this num-
runs on multiple machines to survive machine failures. This                          ber. Therefore, a service must replace failed machines in a
paper describes SMART, a new technique for changing the                              timely fashion, and this requires that the service be able to
set of machines where such a service runs, i.e., migrating                           change its configuration, i.e., the set of machines replicating
the service. SMART improves upon existing techniques in                              it. Changing the conﬁguration, also called migration, has
three important ways. First, SMART allows migrations that                            other purposes, e.g., moving replicas from highly loaded ma-
replace non-failed machines. Thus, SMART enables load                                chines to lightly loaded ones, or changing the number of ma-
balancing and lets an automated system replace failed ma-                            chines replicating the service. This paper presents the Ser-
chines. Such autonomic migration is an important step to-                            vice Migration And Replication Technique, a.k.a. SMART,
ward full autonomic operation, in which administrators play                          our technique for migrating a replicated service.
a minor role and need not be available twenty-four hours a                              It is easy to achieve consistency in a replicated service
day, seven days a week. Second, SMART can pipeline con-                              with no changing state, so this paper concerns only stateful
current requests, a useful performance optimization. Third,                          services, such as ﬁle systems, databases, or market trading
prior published migration techniques are described in in-                            systems. A stateful service must replicate its state so that
suﬃcient detail to admit implementation, whereas our de-                             clients never see inconsistencies, even when failures occur.
scription of SMART is complete. In addition to describ-                                 The state of the art for building consistent, replicated
ing SMART, we also demonstrate its practicality by imple-                            services is the replicated state machine approach [13, 20,
menting it, evaluating our implementation’s performance,                             22], so SMART targets services using this approach. In
and using it to build a consistent, replicated, migratable                           this approach, a replica, i.e., a copy of the service, runs
ﬁle system. Our experiments demonstrate the performance                              on each machine; these replicas use the Paxos protocol to
advantage of pipelining concurrent requests, and show that                           stay synchronized. System designers are increasingly using
migration has only a minor and temporary eﬀect on perfor-                            this approach for several reasons. First, it works on cheap
mance.                                                                               and easily-administered hardware, such as PCs connected by
                                                                                     Ethernet. It does not, e.g., require storage-area networks,
                                                                                     RAID, or networks with partition-free or real-time guaran-
Categories and Subject Descriptors: C.2.4 [Computer-                                 tees. Second, its requirement of service determinism is be-
Communication Networks]: Distributed Systems                                         coming easier to satisfy with the advent of new techniques
General Terms: Algorithms, reliability                                               and tools [3, 21]. Third, the approach can be extended to
Keywords: Migration, replication, reconﬁguration, Paxos,                             deal with Byzantine server failures, i.e., failures that cause
replicated state machine                                                             behavior other than stopping [3]. This is increasingly impor-
                                                                                     tant as services move to less reliable infrastructures, such as
                                                                                     on-demand computing systems and peer-to-peer overlays,
1.     INTRODUCTION                                                                  and also as people increasingly exploit security weaknesses.
   Increasingly, services are being designed to seamlessly tol-                         Existing migratable replicated state machine implemen-
erate machine failures so they can use inexpensive, unre-                            tations have several restrictions that limit their adoption in
liable hardware yet still be highly available. A common                              a general setting [23]. First, they cannot perform migra-
way to make a service tolerate machine failures is to repli-                         tions that remove or replace a non-failed machine. So, for
                                                                                     instance, they cannot move a replica from a highly loaded
                                                                                     machine to a less loaded machine, and they make it dan-
                                                                                     gerous to rely on an imperfect failure detector, such as an
Permission to make digital or hard copies of all or part of this work for            autonomic system, to decide whether to replace a machine.
personal or classroom use is granted without fee provided that copies are            Second, they cannot process requests in parallel, a useful
not made or distributed for profit or commercial advantage and that copies           performance optimization for services with concurrent re-
bear this notice and the full citation on the first page. To copy otherwise, to      quests. Third, important details of these systems have not
republish, to post on servers or to redistribute to lists, requires prior specific
permission and/or a fee.
                                                                                     been published, making it diﬃcult for others to duplicate
EuroSys’06, April 18–21, 2006, Leuven, Belgium.                                      their designs. We are only aware of one publication de-
Copyright 2006 ACM 1-59593-322-0/06/0004 ...$5.00.
scribing a migratable replicated state machine, and it only      2. BACKGROUND: PAXOS
describes its approach at a high level [16].                       This section provides enough detail about Paxos and the
   Consequently, we developed a new technique, SMART, for        replicated state machine approach to understand SMART.
migrating replicated state machines. SMART can pipeline          Other sources contain complete details [13, 14, 22].
concurrent requests, and it allows arbitrary migrations.
Most notably, it can safely perform a migration that re-         2.1    Assumptions
places a machine, even if it is not certain that the replaced       First, we describe the assumptions that must hold for the
machine has failed. This ability enables autonomic service       replicated state machine approach to be applicable. Except
management.                                                      where noted, we make the same assumptions for SMART.
   A key feature of SMART is configuration-specific replicas.       We assume the service runs only on fail-stop machines,
Each replica is associated with one and only one conﬁgura-       i.e., machines that fail only by stopping. In §8, we discuss
tion, so a migration creates a new set of replicas, one for      how SMART could be extended to deal with Byzantine fail-
each machine in the new conﬁguration. For example, when          ures. Note that if a machine crashes and eventually recovers,
the service migrates from {A, B, C} to {A, B, D}, SMART          this is temporary unavailability, not a failure.
does not create a new replica on D and destroy the one              Paxos assumes that fewer than half the machines will fail.
on C, as current approaches do. Instead, it starts three         In other words, there is always some future time when a
new conﬁguration-2 replicas on A, B, and D, and keeps the        quorum of replicas will be alive, where a quorum is a simple
three conﬁguration-1 replicas running on A, B, and C until       majority. SMART weakens this assumption in that once a
the new conﬁguration is established. This feature substan-       new conﬁguration is established, machines in older conﬁgu-
tially simpliﬁes the process of migration and thereby allows     rations can fail. After all, one purpose of service migration is
SMART to overcome the problems of existing approaches.           to eliminate dependence on the current conﬁguration when
However, it makes implementation ineﬃcient, so we intro-         one believes it may stop operating properly. Consequently,
duce the use of shared execution modules to eliminate this       SMART only assumes that fewer than half the machines
ineﬃciency.                                                      in a conﬁguration will fail before the next conﬁguration is
   We implemented SMART to demonstrate its practicality          established.
and to evaluate its performance. Using this implementa-             The service must be deterministic, i.e., its state changes
tion, called LibSMART, we built a ﬁle system that is fully       and outputs can depend only on its state, its input requests,
consistent, replicated, and migratable. We also built a less     and the order of those requests. So, for example, the ser-
complex service to enable microbenchmarks. We show that          vice cannot use local random number generators or clocks,
allowing pipelining reduces latency by 14% when there are        and must be single-threaded to avoid non-deterministic ef-
multiple clients submitting requests concurrently. We also       fects of thread scheduling. Many techniques and tools sim-
show that clients observe little additional latency due to mi-   plify building deterministic services [3, 21]. For instance,
gration. During migration, one or two client requests see        there are deterministic techniques to approximate the cur-
about 25 ms of additional latency, and for a short while af-     rent time, schedule future events, and choose pseudorandom
ter migration, clients see about 0.4 ms of extra latency per     numbers. It may also be possible to use a virtual machine
request.                                                         monitor to make arbitrary non-deterministic services run
   The contributions of this paper are as follows:               deterministically [11]. In our discussion of future work in
                                                                 §8, we sketch how SMART might be modiﬁed to allow non-
   • We introduce the concept of conﬁguration-speciﬁc
                                                                 deterministic services.
     replicas, which enables arbitrary migrations and
                                                                    All we assume about network reliability is that if one alive
     pipelining of concurrent requests.
                                                                 process sends a message to another alive process an inﬁnite
   • We describe how to use shared execution modules to          number of times, it is eventually received.
     eﬃciently implement conﬁguration-speciﬁc replicas.
                                                                 2.2    Overview
   • We describe SMART, our technique for migrating                 In the replicated state machine approach [13, 20, 22], a
     replicated state machines. We are the ﬁrst to describe      replica, i.e., a copy of the service, runs on each of several
     all details necessary to implement such migration.          machines. These replicas run the Paxos protocol to ensure
                                                                 they all execute the same client requests in the same order.
   • We demonstrate SMART’s practicality by implement-           Then, since the service is deterministic, the replicas change
     ing it and evaluating that implementation.                  their states in the same way and produce identical outputs,
                                                                 e.g., replies to clients. The replicas thereby act like a single
For space reasons, we only sketch our arguments for              copy of the service that is more resilient to failure than any
SMART’s correctness. Our technical report [12] contains          individual machine.
a formal proof.                                                     Paxos’s goal is to ensure all replicas execute the same
  The paper is structured as follows. §2 provides back-          requests in the same order. In other words, Paxos must
ground about Paxos and the replicated state machine ap-          decide what request to execute ﬁrst, what request to execute
proach. §3 describes current approaches to service migration     second, etc. In general, we say that Paxos assigns requests
and explains their limitations. §4 describes SMART, our          to slots, where the request it assigns to slot n is the one each
technique for service migration. §5 sketches our arguments       replica will execute as its nth request.
for SMART’s correctness. §6 describes our implementation            Figure 1 illustrates how Paxos works. One of the repli-
and our experimental evaluation of it. §7 describes related      cas is the leader. To submit a request, a client sends it to
work, and §8 discusses avenues for future work. Finally, §9      this leader. The leader chooses an unused slot and sends
concludes.                                                       each replica a proposal, which is a tentative suggestion that
                        RE                            RE Q                                                      Y
           Client          Q                       CT     UES                                              E PL
                               UE               RE            T PROPOSE          LOGGED DECIDED           R
           Replica A              S   T    E DI                         logging                 executing
                                          R
           Replica B                                                     logging                executing

           Replica C                                                     logging                         executing
                           (a)              (b)       (c)        (d)            (e)              (f)            (g)

Figure 1: Example of Paxos operation. (a) The client sends a request to the replica it thinks is leader. (b)
The client is wrong, so the recipient redirects the client to the correct leader. (Normally, the client is correct
and thus skips steps a and b.) (c) The client sends its request to the correct leader. (d) The leader selects
the next available slot and sends a message to each replica (including itself ) proposing that the request ﬁll
the selected slot. (e) The replicas log the proposal, then notify the leader. (f ) When the leader receives
LOGGED messages from a quorum, it sends a message declaring the proposal decided. (g) Each replica
receiving a DECIDED message executes the request and sends a reply to the client. The client ignores all
but the ﬁrst reply.


the given client request should occupy the given slot. Each            checkpoint with index at least n.
replica receiving a proposal logs it, i.e., writes it to stable           We say an index is stable once there is a quorum of repli-
storage, then sends the leader a LOGGED message. Once                  cas each having a checkpoint with that index or higher. If
the leader receives LOGGED messages from a quorum, it                  an index is stable, there will always eventually be an alive
announces that the proposal has been decided, i.e., the re-            replica with a checkpoint with that index or higher. This
quest has been assigned to the slot.                                   is because there will always eventually be a quorum of alive
   One complication is that the leader may fail.             So,       replicas, and this quorum must overlap the quorum that
the leader sends periodic HEARTBEAT messages to each                   made the index stable.
replica, and a replica that does not hear one for a while elects
a new leader by sending ELECT messages to each replica.                State transfer
Before a new leader does anything, it must learn enough                   Normally, a replica reaches the state following request n
about the actions of previous leaders to ensure it does not            by executing requests 1 through n. However, it can some-
make conﬂicting assignments. So, it polls the replicas, ask-           times accomplish this more expediently via state transfer,
ing what proposals they logged from previous leaders. Once             i.e., by receiving a copy of some other replica’s checkpoint
it gets poll responses from a quorum of replicas, it can ensure        with index n. For instance, if a replica has only executed
it does not make conﬂicting assignments.                               100 requests, but receives a copy of a state checkpoint with
   The Paxos leader change protocol includes other details             index 200 from another replica, it can make this its cur-
that we have not described, but they are not relevant                  rent service state and thereby avoid executing requests 101
to understanding SMART. As we will see, SMART uses                     through 200.
conﬁguration-speciﬁc replicas, so the Paxos leader change
algorithm runs only on static conﬁgurations and thus needs             Log truncation
no modiﬁcation.
   A client may think a non-leader is leader, e.g., because of            Because stable storage is limited, we cannot force replicas
a recent leader change, but this gets corrected promptly. If a         to maintain every logged proposal forever. We thus require
non-leader receives a request, it cannot propose the request           a way to let replicas discard old proposals from their logs,
so it replies with a REDIRECT message indicating the cur-              i.e., to perform log truncation.
rent leader. If the client does not receive a timely reply to             Once n is a stable index, there will always eventually be
a request, it broadcasts the request to all replicas.                  an alive replica with a checkpoint with index n or more.
   Each replica executes requests in slot order. After execut-         This replica can transfer this checkpoint to any replica that
ing the request in slot n, it waits to learn what request gets         has not yet executed request n, allowing the recipient to
assigned to slot n + 1, then executes that request.                    skip executing that request. This means no replica need
                                                                       ever know what request n is, so it is safe to discard logged
2.3    Practical implementation details                                proposals for slot n.
                                                                          We can thus place a bound, MaximumLogSize, on the num-
                                                                       ber of logged proposals a replica need ever hold in stable
Checkpoints                                                            storage, by making the following rule. A leader never pro-
   If a replica loses its volatile state, e.g., due to a reboot, it    poses a request to ﬁll slot n until index n − MaximumLogSize
may have to start over from the initial state and re-execute           is stable. This way, a replica need not log a proposal for slot
all requests. So, each replica periodically saves a checkpoint,        n until it can discard logged proposals for all but the pre-
i.e., a copy of its service state on stable storage. It restores       ceding MaximumLogSize slots.
its latest checkpoint after losing its volatile state.                    Limiting the log size can improve performance when repli-
   We call the last slot executed before saving a checkpoint           cas have NVRAM available. As long as the log ﬁts in
the index of that checkpoint. A replica discards a checkpoint          NVRAM, replicas can log proposals without expensive disk
only if it has a checkpoint with a higher index, so once a             writes. This, in turn, speeds up the critical path of request
replica has a checkpoint with index n it will always have a            handling.
3.    MIGRATION: CURRENT METHODS                                  grates from conﬁguration {A, B, C} to {A, B, D}, a failure of
  In this section, we discuss current approaches to migrat-       A at the wrong time can halt the service forever. Suppose
ing replicated state machines. §3.1 presents Lamport’s idea,      the leader, A, crashes while sending DECIDED messages for
which all current approaches use. §3.2 describes challenges       the request that changes the conﬁguration. Only C receives
in implementing this idea, and §3.3 shows how current im-         a DECIDED message, and when it executes the request, it
plementations address them only by signiﬁcantly restricting       learns that it has been removed from the conﬁguration and
functionality.                                                    terminates. Now, some replica must become the new leader.
                                                                  B thinks the only replicas are A, B, and C, so it will never
3.1    Lamport’s idea                                             receive a quorum of poll responses; C has terminated; and D
   Lamport’s description of Paxos includes the following idea     is not even aware yet that it is part of the conﬁguration. So,
about how to perform migration [13]. The service state in-        no replica can become leader, and the service halts forever
cludes the conﬁguration, and the service migrates when a          even though only one machine has failed.
request changes this conﬁguration. The migration does not            It might seem we could avoid this problem by ﬁrst migrat-
happen immediately; it takes eﬀect α slots later, where α is      ing to {A, B, C, D} and then migrating to {A, B, D}, but this
some positive constant. In other words, if n is the slot of       also has a window of vulnerability. The leader, A, may crash
the request that changes the conﬁguration, then this change       while sending DECIDED messages for the request that re-
takes eﬀect starting with slot n + α.                             moves C. Only C receives a DECIDED message, and it ex-
   Using a small α can hurt performance, because the leader       ecutes the request and terminates. Now, some replica must
cannot make a proposal for slot n + α until it has executed       become the new leader. However, A has failed and C has
slot n. Until it executes slot n, it cannot know whether the      self-terminated, so at most two replicas, B and D, can reply
request in that slot excludes the leader’s machine from the       to polls. Since {B, D} is not a quorum of {A, B, C, D}, no
conﬁguration as of slot n + α. For instance, if the leader has    replica can receive a quorum of poll responses and become
only executed slots 1–100 and α is 2, it can only propose slots   leader.
101–102 because as far as it knows it is excluded as of slot
103. The bigger α is, the less likely the leader must wait to
                                                                  Extended-disconnection challenge: After a long dis-
propose a request until it has ﬁnished executing some other
                                                                  connection, a client may be unable to find the service.
one, and thus the more undecided proposals the leader can            If a client reconnects after a long disconnection, its idea
have outstanding. We call the use of concurrent undecided         of the latest conﬁguration may be out of date due to migra-
proposals pipelining, reﬂecting how this overlaps the Paxos       tions during the disconnection. Furthermore, all machines in
network delays for multiple requests.                             the conﬁguration it knows of may have permanently failed.
                                                                  Thus, every machine it contacts will never respond, it will
3.2    Implementation challenges                                  never learn of a working conﬁguration, and it will not be
  Storing the conﬁguration in the state is an elegant idea        able to submit requests.
for migrating replicated state machines. However, it is not
a complete solution to the problem. Implementing the idea         Consecutive-migration challenge: If request n changes
involves addressing several challenges, including the follow-     the configuration, requests n+1 through n+α−1 cannot
ing ﬁve.                                                          change the configuration.
                                                                    Suppose request n removes machine C from the conﬁgu-
Unaware-leader challenge: A new leader may not know               ration as of slot n + α, and request n + 1 restores C to the
the latest configuration.                                         conﬁguration as of slot n + α + 1. Suppose also that C does
   A new leader must poll the replicas to ensure its assign-      not execute request n because it receives a state transfer of
ments do not conﬂict with previous leaders’ assignments.          the checkpoint with index n + 1. In this case, it will never
However, when it is elected, it may have not yet executed a       see the conﬁguration that removes it from the conﬁguration,
request that changed the conﬁguration. It may then poll the       and will incorrectly think it is responsible for slot n + α.
wrong set of replicas, not learn about a recent assignment,
and make a proposal that conﬂicts with that assignment.           Multiple-poll challenge: A new leader may have to poll
   For example, suppose that after the service migrates from      several configurations.
{A, B, C} to {A, B, D}, the leader, A, fails. Now, B initiates       It is possible for several undecided proposals to be out-
a leader change, but B has not yet executed the request that      standing at the time of a leader change. Thus, it is possible
migrated the service. So, it only polls A, B, and C, and is       that some of these are managed by one conﬁguration and
content getting responses only from B and C, since they are       others are managed by another. The leader change algo-
a quorum of the conﬁguration it knows. However, A and D           rithm must be modiﬁed so it can poll multiple conﬁgura-
together constitute a quorum of the new conﬁguration, so          tions.
they may have assigned some request to a slot. B does not
learn of these assignments, and may propose requests that         3.3    Current approaches
conﬂict with them.                                                  Current approaches to migration address these challenges
                                                                  at the cost of restricted functionality. In this section, we dis-
Window-of-vulnerability challenge: Migrations that                cuss a typical example of these approaches, Petal’s global
remove or replace a machine can create a period of                state manager (GSM) [16, 23]. Anecdotal evidence sug-
reduced fault tolerance.                                          gests that unpublished commercial systems use similar tech-
  During migration, a service can have reduced fault toler-       niques.
ance. For example, a service replicated on three machines           To address the unaware-leader challenge, Petal’s GSM
should be able to survive one machine failure. But, if it mi-     uses the following unpublished technique [23]. After a new
leader collects poll responses from a quorum of replicas, it                     Replica 1A
fetches the conﬁguration from the responder that executed                   A
the most requests. If this conﬁguration is diﬀerent than the
                                                                                 Replica 2A                Paxos #1
leader’s, it updates its conﬁguration and restarts the leader                    Replica 1B
change process. Petal’s GSM also enforces two additional                    B
restrictions that, together with the preceding technique, are                    Replica 2B
suﬃcient to address the unaware-leader challenge. First,
                                                                            C    Replica 1C                Paxos #2
requests are not pipelined. Second, only two types of mi-
gration are allowed: adding one machine, or removing one
machine. This ensures quorums from consecutive conﬁgura-                    D    Replica 2D
tions always overlap. Since SMART addresses the unaware-
leader challenge in a diﬀerent way, it can pipeline requests
for performance and it does not require quorums from con-       Figure 2: Example of conﬁguration-speciﬁc replicas,
secutive conﬁgurations to overlap. In fact, SMART does          where conﬁguration 1 is {A, B, C} and conﬁguration
not require any overlap at all between consecutive conﬁgu-      2 is {A, B, D}.
rations.
   To sidestep the window-of-vulnerability challenge, Petal’s
GSM removes a machine via migration only if a human             lenge; we discuss this further in §4.4. While both conﬁgura-
knows it is failed and beyond repair. Thus, although            tions are running, if one machine is in both conﬁgurations,
such a migration reduces the number of tolerable perma-         that machine will simultaneously run two replicas, one for
nent failures by one, this is acceptable because one ma-        each conﬁguration.
chine is already known to have permanently failed. For             Each conﬁguration uses its own instance of Paxos. The
instance, Petal’s GSM will only migrate from {A, B, C, D}       replicas of that conﬁguration are cohorts of each other, i.e.,
to {A, B, D} when C is permanently failed. It is rea-           participants in the same Paxos instance. For example, each
sonable to not survive the failure of A during migration,       conﬁguration’s Paxos has its own leader, one of the repli-
since the service is not expected to survive two simulta-       cas in that conﬁguration. Leaders of diﬀerent conﬁgura-
neous machine failures. SMART addresses the window-of-          tions may or may not happen to be replicas on the same
vulnerability challenge diﬀerently, by eliminating the win-     machine. Since each instance of Paxos has a static conﬁg-
dow of vulnerability. Thus, unlike Petal’s GSM, it can re-      uration, the leader change algorithm need not deal with a
move a non-failed machine via migration. In particular, it      changing conﬁguration or multiple simultaneous conﬁgura-
can migrate replicas from highly loaded machines to lightly     tions. This straightforwardly addresses the unaware-leader
loaded ones, and it can safely rely on an autonomic system      and multiple-poll challenges.
with an imperfect failure detector to decide when to replace       We discuss how we address the extended-disconnection
a machine.                                                      and consecutive-migration challenges in §4.5 and §4.9, re-
   Petal’s GSM addresses the extended-disconnection chal-       spectively.
lenge similarly to how SMART does. We discuss how                  When a replica executes a request that creates a new con-
SMART addresses it in §4.5.                                     ﬁguration, it sends JOIN messages to each machine in this
   Petal’s GSM avoids the consecutive-migration and             new set, telling them to join the conﬁguration if they have
multiple-poll challenges by requiring α = 1. This precludes     not done so already. A machine joins a conﬁguration by
pipelining concurrent requests. SMART uses a diﬀerent ap-       starting a replica associated with that conﬁguration.
proach, so it can pipeline concurrent requests.
                                                                4.2    Avoiding inter-configuration conflict
4.    SMART                                                        Separate conﬁgurations use separate instances of Paxos,
   This section describes SMART incrementally, adding de-       so we must ensure they do not assign diﬀerent requests to
tails and optimizations as it proceeds. Figure 3, appearing     the same slot. We achieve this by making each conﬁguration
later, will present an overview of the protocol this section    responsible for a range of slots, FirstSlot through LastSlot,
describes.                                                      with no two ranges overlapping. If the request that creates
                                                                a conﬁguration is in slot n, then the new conﬁguration’s
4.1   Configuration-specific replicas                           FirstSlot is n + α, and the old conﬁguration’s LastSlot is
   A key feature of SMART is configuration-specific replicas:   n + α − 1. Each replica only assigns and executes slots in
each replica is associated with one and only one conﬁgura-      its range because a leader only proposes slots in its range.
tion. A migration request creates a new set of replicas, one       A leader can avoid proposing slots less than FirstSlot be-
for each machine in the new conﬁguration. Figure 2 illus-       cause each replica is created knowing its FirstSlot. If it is
trates an example. When the service migrates from conﬁgu-       in conﬁguration 1, it knows FirstSlot is 1. Otherwise, it
ration {A, B, C} to {A, B, D}, we do not create a new replica   was created due to a JOIN message, and the JOIN message
on D and destroy the one on C, as current approaches do.        speciﬁed this value.
Instead, we keep the three conﬁguration-1 replicas running         It is trickier to avoid proposing slots after LastSlot, since
on A, B, and C, and start three new conﬁguration-2 replicas     often the leader will not know this value because it has not
on A, B, and D.                                                 yet created a successor conﬁguration. For this case, we use
   The old conﬁguration’s replicas continue running even af-    Lamport’s idea: the leader may not make a proposal for slot
ter creating the new conﬁguration. They remain running          n + α until it has executed slot n. Thus, as usual, α controls
long enough to ensure there is no period of reduced fault       the degree to which the leader’s proposals can get ahead of
tolerance, i.e., to address the window-of-vulnerability chal-   its execution.
For each server machine:                                                  For each replica on each server machine:
Whenever you learn that conﬁguration n is defunct, and n > d              After executing a request that creates a new conﬁguration,
where d is the highest one you know is defunct,                              Send a JOIN message to each machine in that
     Set d := n and destroy all replicas in conﬁgurations ≤ n.               conﬁguration.
Upon initial startup, if you are in the initial service conﬁguration,     After executing the last slot of your conﬁguration,
    Start a replica in conﬁguration 1.                                       From now until you are eventually destroyed, periodically
Upon receipt of a JOIN message for a conﬁguration n > d,                     send a FINISHED message to each machine in the next
    Start a replica in conﬁguration n if one isn’t running already.          conﬁguration.
Upon receipt of a FINISHED, HEARTBEAT, or ELECT message                   Upon receipt of a FINISHED message,
addressed to a replica in conﬁguration n > d,                                If you have a checkpoint at or after your starting state,
    If there is no local running replica in that conﬁguration,                  Reply with a READY message;
         Reply with a JOIN-REQUEST message for conﬁguration n.               Otherwise, if you don’t have your starting state,
Upon receipt of a client request to a replica in conﬁguration n ≤ d,            Request your starting state from the sender.
    Reply with a NEW-CONFIGURATION message.                               Upon receipt of READY messages from a quorum of your
Upon receipt of any other message to a replica in                         successor conﬁguration,
conﬁguration n ≤ d,                                                          Note that your conﬁguration is defunct. (This will cause
    Reply with a DEFUNCT message.                                            you to be destroyed.)
Upon receipt of a DEFUNCT message regarding conﬁguration n,               Upon receipt of a JOIN-REQUEST message,
    Note that conﬁguration n is defunct.                                     Reply with a JOIN message for the requested
                                                                             conﬁguration.


                                        Figure 3: Overview of the SMART protocol


4.3    Transfer of responsibility                                       to complete its work, namely to eventually assign a request
   We now discuss how a new conﬁguration begins receiving               to LastSlot, execute that request, then propagate the result-
and executing client requests.                                          ing state to a successor conﬁguration so that conﬁguration
   When a leader has ﬁlled up all slots through LastSlot, it            can become established.
cannot make any more proposals. So, it responds to client                  We use the following two-part protocol to ensure that a
requests with a NEW-CONFIGURATION message, telling                      defunct, alive replica eventually destroys itself. First, when
the client to start using the new conﬁguration. The client              a machine receives a message addressed to a replica that has
ignores the message if it already knows about a later conﬁg-            destroyed itself, it responds with a DEFUNCT message. A
uration.                                                                replica destroys itself if it hears a DEFUNCT message indi-
   Before a new replica executes any requests, it must initial-         cating a cohort or a member of a successor conﬁguration has
ize its service state to an appropriate starting state. Since           destroyed itself. Second, when a replica is ﬁnished, it cre-
the ﬁrst request this replica can execute is the one in slot            ates a thread that periodically sends a FINISHED message
FirstSlot, a proper starting state is a state reﬂecting the ex-         to each replica in its successor conﬁguration. This message
ecution of at least slots 1 through FirstSlot − 1. Thus, it can         asks the recipient to reply with a READY message if it has
acquire its starting state from a replica from the previous             saved a checkpoint at or after its starting state. Once the
conﬁguration that has executed its LastSlot, i.e., a finished           ﬁnished replica has received a READY reply from a quorum
replica from the previous conﬁguration. In some cases, it               of successor replicas, or one DEFUNCT reply, it knows some
may acquire its starting state from a cohort instead.                   successor conﬁguration is established and destroys itself.
                                                                           When a replica destroys itself, its machine must remem-
                                                                        ber its conﬁguration number and successor conﬁguration
4.4    Destroying defunct replicas                                      so it can send DEFUNCT and NEW-CONFIGURATION
   In §3.2, we showed how a window of vulnerability arises              messages as appropriate.         As an optimization, a ma-
when a replica terminates before the next conﬁguration is               chine only remembers this information for the highest-
able to make independent forward progress. In SMART,                    numbered conﬁguration. It can then send DEFUNCT or
we eliminate this window of vulnerability by not destroying             NEW-CONFIGURATION messages on behalf of any lower-
replicas of an old conﬁguration immediately. In this subsec-            numbered conﬁgurations, which are necessarily also defunct.
tion, we discuss when SMART can destroy a replica of an                 The latter message may inform a client of a successor con-
old conﬁguration.                                                       ﬁguration that is not the immediate successor of the one the
   We say a conﬁguration is established once FirstSlot − 1 is a         client was addressing, but this poses no problem.
stable index of that conﬁguration. After this, it will always
eventually have an alive replica with a copy of the state               4.5   Configuration repository
reﬂecting slots 1 through FirstSlot − 1, obviating the need               Next, we discuss how we address the extended-
for any information about those slots. Since these slots are            disconnection challenge. If a machine reconnects after a long
exactly those that previous conﬁgurations are responsible               disconnection, it may not know the latest conﬁguration due
for, those conﬁgurations and their replicas are defunct, i.e.,          to migrations during the disconnection. Furthermore, due to
safe to destroy.                                                        the weak reliability assumption in §2.1, all servers it knows
   The replicas of a conﬁguration must continue operating               of may have permanently failed, so it may never discover
until a successor conﬁguration is established. Recall from              a working conﬁguration. If it is a client, its requests may
§2.1 that we assume a quorum of replicas in a conﬁguration              never get executed. If it has a defunct replica, that replica
will continue operating at least until a successor is estab-            may never learn it is defunct. This is a fundamental problem
lished. The liveness of a quorum enables that conﬁguration              for any self-migrating service, and necessitates a configura-
tion repository where the service can store its most recent         is established, and eventually an alive cohort can provide a
conﬁguration information.                                           state copy with index at least FirstSlot − 1.
   We use the repository as follows. Periodically, e.g., every
ﬁve minutes, a leader of an established conﬁguration writes         4.8    Null requests
its conﬁguration information to the repository. A process              In Paxos [13], sometimes upon election a new leader must
suspecting a conﬁguration is defunct reads the repository to        propose a null request to ﬁll a slot. A null request is simply
try to learn a newer conﬁguration. Also, a leader reads the         an extra request whose execution does nothing, so it is al-
repository before writing its own information; if it ﬁnds an        ways safe to propose a null request. SMART has the leader
earlier conﬁguration, it sends DEFUNCT messages to the              submit null requests in an additional scenario.
replicas in that conﬁguration.                                         When a leader knows its LastSlot, it proposes null requests
   This protocol ensures correct operation even if the repos-       for all remaining unproposed slots. This hastens establish-
itory is not consistent or durable. Furthermore, the service        ment of the successor conﬁguration, reducing the time that
can be highly available even if the repository is less available,   correctness relies on the old conﬁguration’s machines and
because the repository is only necessary on those rare occa-        letting replicas of the old conﬁguration destroy themselves
sions when a machine reconnects after an extremely long             sooner. It also serves to prevent deadlock, as follows.
disconnection. It is important that the repository need not            Suppose a leader with LastSlot of 100 has made propos-
be consistent, durable, and highly available, since otherwise       als for slots 1–100. Having ﬁlled up all its slots, it starts
we could not build a service with those properties unless we        redirecting clients to the next conﬁguration as discussed in
already had a repository service with those properties.             §4.3. Now, the leader crashes, and the network loses all its
   Because there are such weak requirements for the reposi-         proposal messages for slots 98–100. A new leader is elected,
tory, there are several simple ways to build it. For instance,      learns about slots 1–97 in its poll, and waits for a client re-
one can store the conﬁguration information in a DNS entry.          quest to propose for slot 98. However, it waits in vain: all
                                                                    clients were redirected to the next conﬁguration, so they will
4.6    Ensuring new replicas are created                            not submit a request to that leader. This next conﬁguration
   Next, we discuss how we ensure that every alive machine          may assign requests to slots 101 and beyond, but not slots
in a non-defunct conﬁguration eventually joins that conﬁg-          98–100. No request ever gets assigned to slot 98, so the ser-
uration.                                                            vice stops forever. Our technique avoids this scenario since
   We cannot rely on the JOIN messages replicas send when           the newly elected leader will propose null requests for slots
they create a new conﬁguration. There are a ﬁnite number            98–100, allowing the service to make progress.
of these messages, so they may all be lost. We can use
them as an optimization to start the replicas quickly, but          4.9    Configuration information in state
not as a guarantee that every machine that should join the             Often, a replica needs to obtain from its service state in-
conﬁguration does so.                                               formation about its successor conﬁguration: whether it has
   If a machine receives a FINISHED message addressed to            been created and, if so, its FirstSlot and its set of machines.
a not-yet-created replica on that machine, it creates the ad-       A leader needs this to determine for what slots it may pro-
dressed replica. Since some replica will send FINISHED              pose. A replica needs this to determine whether it is ﬁnished
messages repeatedly until it receives a READY message               and, if so, where to send FINISHED messages.
from a quorum, a quorum of the new conﬁguration will                   The consecutive-migration challenge arises because some-
eventually join. This does not completely solve our prob-           times this information may not be present, having been
lem, since we want every alive machine to eventually join.          overwritten by a later conﬁguration. However, in SMART,
So, in addition, if a machine receives a HEARTBEAT or               the service state includes information not just about the
ELECT message addressed to a not-yet-created replica, it            latest conﬁguration, as Lamport recommended, but also
creates the addressed replica.                                      about older, non-defunct conﬁgurations. Consequently, the
   A small problem with this is that a machine must join            consecutive-migration challenge does not arise, and SMART
upon receiving a FINISHED, HEARTBEAT, or ELECT                      can let any request change the conﬁguration.
message, but such a message does not specify the conﬁgura-             The service state may also include information about de-
tion’s set of machines or FirstSlot. Thus, a machine does not       funct conﬁgurations. However, the service should eventually
join immediately upon receiving such a message. It sends            discard such information to prevent its state from growing
back a JOIN-REQUEST message, and the recipient replies              indeﬁnitely. Since it is deterministic, it can only change its
with a JOIN message.                                                state while executing requests. So, at the beginning of ex-
                                                                    ecuting request n, it checks whether n − MaximumLogSize
4.7    Acquiring starting state                                     is the LastSlot of some conﬁguration C. If so, it knows that
   Recall that a new replica generally obtains its starting         C is defunct, and discards information about it. It knows
state from a ﬁnished replica in its predecessor conﬁgura-           C is defunct because, as we discussed in §2.3, no leader can
tion. Instead of repeatedly polling those replicas, waiting         propose request n unless n − MaximumLogSize is a stable
for one to be ﬁnished, a new replica just waits to receive a        index of its conﬁguration. This means that LastSlot of C is a
FINISHED message, then asks the sender for a state copy.            stable index of some successor conﬁguration, so C is defunct.
   However, there is a wrinkle. Once the predecessor conﬁgu-
ration becomes defunct, its replicas may destroy themselves         4.10    Shared execution modules
and become unable to provide a state copy. So, sometimes a            The scheme described thus far is ineﬃcient in that a new
new replica must acquire its starting state from one of its co-     replica must always copy its starting state from another
horts. A replica can always eventually do this, because if its      replica. For many services, this state can be quite large, so
predecessor conﬁguration is defunct then its conﬁguration           copying it from one machine to another, or even just copy-
        A                          B                               with a checkpoint copy reﬂecting slots that conﬁguration 4
                Replica 1A                 Replica 1B
                                                                   is responsible for. However, this does not pose a problem,
             Replica 2A                 Replica 2B                 since the only reason a replica ever inspects its EM’s state
                                                                   is to obtain information about its successor conﬁguration.
                 EM A                       EM B                   As we discussed in §4.9, executing further requests cannot
        C                          D                               change this information due to the way we manage conﬁgu-
                                                                   ration information in the state.
               Replica 1C                Replica 2D
                 EM C                       EM D
                                                                   5. CORRECTNESS
              replica            execution module (EM)                In this section, we sketch our arguments about SMART’s
                                                                   correctness. For the full proof, see our technical report [12],
                                                                   which speciﬁes our system and proves its safety property in
Figure 4: This ﬁgure illustrates shared execution
                                                                   the formal systems-speciﬁcation language TLA+ [15].
modules, i.e., that all replicas on a machine share
                                                                      SMART’s safety property is that no two replicas ever as-
a single EM. Here, conﬁguration 1 is {A, B, C} and
                                                                   sign diﬀerent requests to the same slot.
conﬁguration 2 is {A, B, D}.
                                                                      Basic Paxos uses the following proof. Suppose two lead-
                                                                   ers assign requests to the same slot. The earlier leader must
                                                                   have gotten a quorum of replicas to log its corresponding
ing it between processes on the same machine, can be time-         proposal, and the second leader must have gotten a quorum
consuming. In this subsection, we describe an optimization         of replicas to reply to its poll so it could make any proposal
we call shared execution modules that obviates most copying        at all. Quorums overlap, so the second leader must have
and saves space.                                                   heard about the ﬁrst leader’s logged proposal and thus pro-
   The optimization factors some of the functionality out of       posed the same request for the slot. So, both leaders assign
each replica into a separate module called an execution mod-       the same request to the slot.
ule (EM). This functionality includes storing service state,          However, this proof does not work for SMART, since not
modifying state by executing requests, and saving and trans-       all quorum pairs overlap. For instance, one quorum might
ferring state checkpoints. The remaining functionality re-         be {A, C} from conﬁguration {A, B, C} and another might
mains in the replica, including acting as leader, logging pro-     be {B, D} from conﬁguration {A, B, D}.
posals, and electing a new leader. This way, we can share             We start by proving the following lemma: if only one con-
a single EM among all replicas on the same machine, as             ﬁguration is responsible for a slot, only one request is as-
illustrated in Figure 4.                                           signed to that slot. Conﬁguration-speciﬁc replicas make this
   Shared EMs reduce state copying as follows. Frequently,         proof straightforward. A replica only sends Paxos messages
successive conﬁgurations will overlap, so several new repli-       to replicas of the same conﬁguration. Therefore, if only one
cas will be colocated with replicas of the old conﬁguration.       conﬁguration is responsible for a slot, the quorum of replicas
For instance, an autonomic system might replace a failed           that logs a proposal for that slot must come from the same
machine C in conﬁguration {A, B, C} by reconﬁguring to             conﬁguration as any quorum that replies to a poll that en-
{A, B, D}, so the new replicas on A and B ﬁnd themselves           ables a future leader to create a new proposal for that slot.
with colocated replicas. Each new replica that is colocated        Reasoning as in the basic Paxos proof, the lemma follows.
with a replica from its predecessor conﬁguration defers copy-         We now prove that only one request is assigned to each
ing its starting state. It hopes that the colocated replica will   slot by induction on slot. Only the initial conﬁguration is
soon ﬁnish executing its ﬁnal slot, thereby putting the EM         responsible for slot 1, so by the lemma, the induction con-
in exactly the state the new replica needs and obviating a         dition holds for slot 1. Now, suppose it holds for slots 1
copy.                                                              through n. This implies only one request is assigned to
   However, its hopes may be dashed, and the colocated             each of those slots. The state machine is deterministic, so
replica may not reach its ﬁnal state before destroying itself.     the outcome of executing requests 1 through n will be the
Fortunately, even in this case there is opportunity to save        same everywhere such an outcome is observed. This out-
state-copying costs. Although the EM did not execute all           come speciﬁes the conﬁguration responsible for slot n + 1,
requests from the previous conﬁguration, it likely executed        and the uniqueness of that conﬁguration means only one re-
most of them, so its state is close to the required starting       quest will be assigned to that slot according to the lemma.
state. Thus, the replica can substantially reduce copying          This completes the inductive step and thus the proof.
time with an incremental state transfer [3].                          The proof in the companion technical report [12] is for-
   Some things a replica used to do internally now involve         mal and far more detailed. One illustration of this is that it
communication with the EM. For instance, it no longer ex-          showed us a bug in both the implementation and the proto-
ecutes requests. Instead, it tells the EM when requests are        col speciﬁcation. Our speciﬁcation considers volatile state
assigned to slots, and the EM executes those requests when         by admitting a “crash” action that erases certain variables.
it is ready.                                                       We discovered that after a crash, a leader forgot which slots
   A seemingly problematic aspect of EMs is that a replica         it had proposed for. If it recovered quickly enough that
may ﬁnd its EM state progressing beyond LastSlot. For in-          none of its cohorts noticed its failure, it could remain leader
stance, if replicas in conﬁgurations 3 and 4 coexist on a ma-      throughout its crash and recovery. Then, it could propose
chine, then the replica in conﬁguration 4 may cause the EM         new requests in slots it had already used, possibly leading
to execute a slot that conﬁguration 4 is responsible for. Or,      to conﬂicting requests for the same slot. Once we discovered
the replica in conﬁguration 4 may overwrite the EM state           this bug, it was easy to ﬁx.
6.    EXPERIMENTAL RESULTS                                                           Waiting for reply         Sending proposal
   To evaluate SMART, we built a prototype implementation                            Logging proposal          Executing request
of it and performed experiments to evaluate its performance.
This section describes that implementation, the methodol-              Client
ogy of our experiments, and our experimental results.               Leader A
6.1    Implementation                                               Replica B
                                                                    Replica C
  We implemented SMART as a library called LibSMART.
Service implementations can use this library to obtain repli-
                                                                                0                1                 2                    3
cation and migratability. LibSMART presents a similar
interface to BFT [3], enabling us to port existing services                                          Time (ms)
written for that interface. LibSMART’s implementation of
SMART is complete except for the conﬁguration repository.            Figure 5: Timeline of one request selected for illus-
  To demonstrate SMART’s practicality, we used LibS-                 tration
MART to build a real service: a consistent, replicated,
migratable ﬁle system. We ran thousands of hours of ﬁle
system traces on this new ﬁle system, and thousands of               logged proposals from surviving a machine crash. Therefore,
runs that veriﬁed the correctness of our implementation dur-         in a real deployment we would have to disable these disks’
ing disconnections, reboots, and migrations. Furthermore,            write caches. However, since real servers’ hard drives gener-
building this service was fairly straightforward. We already         ally use NVRAM write caches, and this will be increasingly
had a ﬁle system that used BFT for consistency and repli-            true in the future, for most of our experiments we simulate
cation [1], so by porting it to LibSMART we achieved mi-             the presence of such NVRAM by enabling write caching on
gratability. Note that by doing so we replaced its Byzantine         the disks.
fault tolerance with mere fail-stop fault tolerance; it is future
work to enable Byzantine fault tolerance in SMART.                   6.3    Request timeline
  The ﬁle system uses highly eﬀective techniques to hide                The goal of our ﬁrst experiment is to present a timeline
the latency of server operations, so evaluating its perfor-          of the interesting events contributing to the latency of a
mance sheds little light on the performance of LibSMART.             single client request. This timeline will help frame further
Therefore, in this section, we benchmark LibSMART with               experimental results. To provide correspondence between
a simple key/value service. This service permits client re-          times measured on diﬀerent machines, we synchronize the
quests that read, write, and delete string values associated         machines’ clocks by broadcasting reference Ethernet pack-
with integer keys. Each replica of the service caches infor-         ets [7].
mation in memory for the most recently accessed 1,000 keys.             Figure 5 shows the timeline that results from a single re-
Thus, it must access the disk when processing a request for          quest we selected for illustration. The client sends a request,
a non-recently accessed key.                                         and after some communication delay the leader receives it
                                                                     and sends a proposal. The leader logs this proposal immedi-
6.2    Methodology                                                   ately, while the other replicas must wait until they receive it.
   Our experimental test bed uses seven identical HP D530            Once the leader receives LOGGED messages from a quorum,
convertible mini-towers, each with a 3.2 GHz Intel Pentium 4         it begins executing the request. In this case, the quorum
processor with 800 MHz Front Side Bus, 512 KB L2 cache               consists of the leader and machine B, since machine C takes
with HyperThreading enabled, 1 GB 400 MHz DDR Dual                   longer to receive and log the proposal. The leader sends a
Channel RAM, and 80 GB 7200 RPM PATA hard drive.                     reply to the client, which considers the request complete.
Each runs Windows XP Professional with Service Pack 2.               Machines B and C eventually hear the leader’s decision and
Each is connected to the same LAN subnet via built-in 100            execute the request, but this is not on the critical path seen
Mb/s Ethernet. This subnet provides sub-millisecond ping             by the client. Note that the leader takes longer to execute
times.                                                               than the other replicas, because as an optimization in our
   The ﬁrst machine runs service clients. The next three,            implementation, only the leader sends a reply.
which we call A, B, and C, act as servers in the service’s ini-
tial conﬁguration, with A as the initial Paxos leader. Two           6.4    Effect of NVRAM
other machines, which we call D and E, act as servers in                In the next experiment, we measure the eﬀect of using
one experiment that requires ﬁve machines instead of three           NVRAM for disk write caching. A client submits 20,000
in the initial conﬁguration. The last machine acts as a re-          consecutive requests and we measure the average latency.
placement during migration; accordingly, we call it R. We            We do this once with disk write caching on, simulating the
coordinate experiments using a host machine on the same              presence of NVRAM, and once with disk write caching oﬀ.
subnet.                                                                 We ﬁnd that the 95% conﬁdence intervals for average la-
   For most experiments, the only client requests we measure         tency are 2.98 ms ± 0.04 ms with NVRAM and 8.67 ms
are ones that read a key expected to be in each server’s             ± 0.06 ms without it. We conclude that NVRAM reduces
cache. We chose these requests since servers can execute             latency by approximately 5.7 ms. Figure 6 breaks down
them quickly. This highlights the more interesting sources           the latency into major components, showing, as expected,
of latency, e.g., those due to replication and migration.            that the main reason for this diﬀerence is that logging takes
   Our test bed uses Seagate Barracuda ST380011A hard                longer without NVRAM.
drives. Since these drives’ internal write caches are volatile,         Most of the remaining time is spent in communication,
write caching could prevent SMART’s checkpoints and                  which includes waiting for network transmission, waiting for
                                                                           Latency (ms)
                      Log       Execute   Communicate    Miscellaneous                    9
                                                                                          6
                                                                                          3
                        With NVRAM
                                                                                          0
                      Without NVRAM                                                           0   200     400    600   800     1000
                                                                                                           Request #
                                          0 1 2 3 4 5 6 7 8 9
                                                                         Figure 8: Request latencies seen by client before
                                                Milliseconds
                                                                         and after disconnection of machine C, shown as a
                                                                         vertical dashed line
Figure 6: Breakdown of average request latency
seen by client. “Miscellaneous” includes time spent
proposing requests and time spent waiting for check-                       With pipelining enabled, average latency is 4.3 ms; with-
points.                                                                  out pipelining, it is 4.9 ms. We see that in both cases
                                                                         the submission of requests by another client reduces per-
                      150                                                formance. However, this eﬀect is substantially reduced by



   Avg latency (ms)
                                                                         pipelining. In all, pipelining reduces latency by 14%. We
                      100                                                conclude that SMART’s ability to pipeline is useful in re-
                      50                                                 ducing request latency when there are concurrent requests.
                                                                           Note that our implementation does not batch multiple
                       0                                                 requests, which would normally mitigate the eﬀect of lack
                            0             10            20          30   of pipelining. However, this is irrelevant for this experi-
                                                                         ment, since with two clients at most one request is wait-
                                     Added network delay (ms)            ing for proposal at a time and thus there is no opportunity
                                                                         for batching. Batching would reduce further performance
Figure 7: Eﬀect of additional network delay on av-                       degradation from a third client, a fourth client, etc., but it
erage request latency seen by client                                     cannot help with the performance degradation from the ﬁrst
                                                                         additional client.

message handlers to be scheduled, and encrypting and de-                 6.7              Disconnection
crypting messages. Executing requests is minor, because we                  We next perform a simple experiment to demonstrate re-
have chosen requests that are quickly executed. Incidentally,            silience to a single failure. A client submits 1,000 requests;
our implementation delays execution of a request while sav-              partway through, we disconnect machine C. Figure 8 shows
ing a checkpoint, which happens after every 50th request;                the results, with the following two things most notable.
repairing this is future work. However, in this example, the             First, there is no pause in service operation at the time
state changes are minor, so requests spend almost no time                of disconnection, since Paxos performs no special process-
waiting for checkpoints.                                                 ing to exclude a disconnected non-leader machine. Second,
                                                                         average latency is the same before and after disconnection:
6.5                   Effect of network latency                          2.6 ms in both cases. The critical path of request handling
   Our next experiment measures the eﬀect of network delay               involves the time it takes the fastest quorum of replicas to
on client latency. A client submits 20,000 consecutive re-               log the proposal and respond to the leader, so it goes essen-
quests and we measure the average latency. We do this four               tially just as quickly with machine C disconnected as with
times, each time simulating a diﬀerent additional network                all machines alive.
delay. The added delays range from 0–30 ms.                                 In another experiment, we disconnect the leader instead.
   Figure 7 shows the results. We see that for every 1 ms                The service continues operating correctly, but in this case
added to network latency, request latency goes up by ap-                 the performance eﬀect is far higher: after the disconnection,
proximately 4 ms. This is as expected since, as illustrated              the next client request has a latency of 3.1 sec. This large
in Figure 5, there are four network hops on the critical path            delay is mostly due to the tunable 3-second timeout before
of handling a client request: the client sends the request to            another replica concludes the leader has failed and initiates
the leader, the leader sends a proposal to the other replicas,           a leader change.
the replicas send the leader a LOGGED message, and the
leader sends the client a reply.                                         6.8              Migration
                                                                            In this section, we discuss experiments demonstrating the
6.6                   Concurrent requests                                performance impact of service migration in our implemen-
  Our next experiment measures the eﬀect of pipelining on                tation. Speciﬁcally, we show the extra request latency that
the latency of concurrent requests. Two clients run simulta-             clients see during a service migration, and examine the
neously, each submitting a stream of consecutive requests.               sources of that additional latency.
We measure the average latency of 20,000 consecutive re-                    To set up the ﬁrst experiment, our client writes various
quests on one of the two clients. We run this experiment                 100-byte values to 10,000 keys. Then, the client submits
twice, once with pipelining enabled and once with it dis-                a stream of read requests. Partway through this stream,
abled. We disable pipelining by setting α to 1 instead of its            another client running on the host machine issues an ad-
default 10.                                                              ministrative request to migrate the service to conﬁguration
                   10                                                                   10


    Latency (ms)
                    8


                                                                         Latency (ms)
                                                                                         8
                    6
                                                                                         6
                    4
                    2                  x
                                                                                         4
                    0                                                                    2
                        0   5000    10000 15000   20000 25000                            0
                                     Request #                                               0   5000    10000   15000   20000   25000
                   20


   Latency (ms)
                                                                                                          Request #
                   15                                                                   25



                                                                         Latency (ms)
                   10                                                                   20
                    5                                                                   15
                   0                                                                    10
                    3060     3070          3080   3090   3100                            5
                                     Request #                                           0
                                                                                          2900    2910       2920        2930     2940
Figure 9: The top graph shows request latencies be-
fore and after migration of the service from {A, B, C}                                                    Request #
to {A, B, R}. The migration is shown as a vertical
dashed line, and the period during which machine                   Figure 10: The top graph shows request laten-
B was copying state to machine R is shown with a                   cies before and after migration of the service from
gray background. For scale, several uninteresting                  {A, B, C, D, E} to {A, B, R, D, E}. The migration is
outliers above 10 ms are not shown. The bottom                     shown as a vertical dashed line, and the period dur-
graph shows the period surrounding the migration.                  ing which machine D was copying state to machine
                                                                   R is shown with a gray background. For scale, un-
                                                                   interesting outliers above 10 ms are not shown. The
{A, B, R}. Figure 9 shows the request latencies observed by        bottom graph shows the period surrounding the mi-
the reading client throughout this process.                        gration.
   The ﬁrst two requests following migration incur a higher
latency than typical, 8 ms and 16 ms. We explore the cause
of this in §6.9. After those, request latencies return to the        Incidentally, the two client requests immediately following
typical amounts seen before migration. Then, a short while         migration have elevated latencies of 10 ms and 21 ms, similar
later, there is a period of increased average latency, up to       to what we observed in the previous experiment.
about 3.4 ms from the typical 3.0 ms. During this time, ma-
chine B takes slightly longer to process proposals as in the       6.9                  Sources of delay following migration
background it is sending a checkpoint to machine R. This              To understand the source of the latency increases immedi-
takes about 53 seconds, mostly because our state transfer          ately following migration, we conduct an additional exper-
code has not been tuned for performance. After this trans-         iment. In this experiment, we perform 200 consecutive mi-
fer, latency returns to normal for the rest of the run. Overall,   grations, alternating between conﬁgurations {A, B, C} and
we conclude that migration has only a minor and temporary          {A, B, R}. We ﬁnd that the delays observed by the client
eﬀect on observed latency, demonstrating the eﬃciency of           correspond to certain phases leader machine A goes through
our migration technique and implementation.                        during migration, shown in Figure 11. Phase 1, taking 3 ms
   In our next experiment, we show that the delay caused           on average, is waiting for the migration request to be logged.
by the checkpoint transfer can be eliminated by replicating        Phase 2, taking 4 ms on average, is executing that request,
the service on ﬁve machines instead of three. This exper-          including sending JOIN messages and proposing null re-
iment is just like the previous one except that the initial        quests. Phase 3, taking 12 ms on average, is waiting for
conﬁguration is {A, B, C, D, E} and the subsequent conﬁgu-         null requests to be logged and then executing them. Phase
ration is {A, B, R, D, E}. Figure 10 shows the results of this     4, taking 8 ms on average, is waiting for a checkpoint save;
experiment. We see that, indeed, using a ﬁve-machine con-          our implementation always saves a checkpoint upon reach-
ﬁguration eliminates the period of slightly elevated latencies     ing a conﬁguration’s ﬁnal state, to hasten the establishment
during the gray region representing when the new machine           of the next conﬁguration. Depending on whether any client
is copying a checkpoint. The reason for this is as follows.        request arrives during phase 1, either one request observes
When we replace one of ﬁve machines, four machines from            the latency of phase 2 and another observes the latency of
the original conﬁguration are also in the subsequent conﬁg-        phase 4, or a single request observes the latency of most of
uration. While one of these four is transferring state to the      phases 2–4. The former case happened in our earlier migra-
new machine, three machines are unaﬀected. These three             tion experiments, explaining the two slightly elevated laten-
machines by themselves constitute a quorum of the conﬁgu-          cies immediately following migration. In our 200 migrations,
ration, so it is their speed that determines the latency seen      the former case happens 83% of the time and when it does,
by clients.                                                        the two delayed requests have average latency of 11 ms and
       Waiting for migration request to be logged (3 ms)          rable overhead only when the leader becomes temporarily
                Executing migration request (4 ms)                unavailable or the conﬁguration changes.
                                                                     Another method for building replicated services is to build
      1     2              3                     4                them out of replicated objects, as in RAMBO [17] and Mar-
                                                                  tin et al.’s reconﬁgurable Byzantine quorum system [19]. In
     Waiting for null requests to be logged (12 ms)               such a system, an object is replicated among several servers,
                   Saving state checkpoint (8 ms)                 and the set of servers can change. A client request can ei-
                                                                  ther read or write an object. The main disadvantage of
Figure 11: This rough timeline shows the phases                   this approach is that some services cannot be composed out
leader machine A goes through during a migration.                 of read/write objects. For instance, some services need to
The two main sources of latency observed by clients               atomically read and conditionally modify an object, or to
are described in italics.                                         atomically modify two objects. This is why a ﬁle service
                                                                  built on a replicated object system, such as Om [26], cannot
                                                                  provide namespace operations such as atomically moving a
17 ms. The latter case happens 17% of the time during             ﬁle out of one directory and into another.
our 200 migrations, and then the average latency of the one          Finally, a service can obtain migratability by using a sepa-
delayed request is 28 ms.                                         rate conﬁguration service, as done by Boxwood [18], GFS [9],
   Incidentally, if our implementation did not force requests     and chain replication [24]. In this method, a separate service
to wait to execute during checkpoint saves, we would not          determines the current conﬁguration of the main service.
see the latency of phase 4. Instead, we would see addi-           This approach is reasonable, but still requires a mechanism
tional latency due to client redirection. Recall that a client    like SMART to allow the conﬁguration service itself to mi-
request submitted after migration must incur two extra net-       grate. For instance, most components in the Boxwood sys-
work transmission delays: one for the old leader to send a        tem are migratable, with the notable exception of the master
REDIRECT message to the client and one for the client to          conﬁguration service, a limitation acknowledged by its au-
resubmit its request to the new leader. In our experiments,       thors [18]. SMART would be an excellent tool for building
since requests were delayed until the end of phase 4, this        a migratable master conﬁguration service for Boxwood.
redirection latency was not on the critical path, and so con-
tributed nothing to observed latency.
                                                                  8. FUTURE WORK
                                                                     In this section, we discuss two promising avenues of future
7.    RELATED WORK                                                work. One is modifying SMART to allow non-deterministic
   Yin et al. [25] argue for the separation of agreement from     services. The other is modifying it to survive a limited num-
execution in Byzantine fault-tolerant state machine replica-      ber of Byzantine server failures, i.e., failures that cause be-
tion, to reduce the number of execution modules in a static       havior other than merely stopping.
conﬁguration and to enable the use of a privacy ﬁrewall.             For some services, determinism is impractical, such as a
SMART also separates agreement from execution, but for            multi-threaded service whose behavior is aﬀected by thread
a diﬀerent purpose, namely to share one execution module          scheduling. A standard way to deal with non-determinism
among replicas of diﬀerent conﬁgurations on the same ma-          in Paxos is semi-passive replication [5]. In this method, a
chine.                                                            leader does not propose requests. Instead, it tentatively ex-
   Our use of conﬁguration-speciﬁc service replicas is similar    ecutes requests, recording state changes and buﬀering out-
to the use of conﬁguration-speciﬁc object replicas in systems     puts. What it proposes are those state changes and outputs.
such as RAMBO [17]. We are the ﬁrst to use conﬁguration-          When another replica learns a proposal is decided, it merely
speciﬁc replicas to migrate replicated state machines, and        applies the state changes. Thus, only one replica executes
to use shared execution modules to make this approach ef-         requests, as is appropriate considering the service is non-
ﬁcient.                                                           deterministic and thus may execute diﬀerently on diﬀerent
   Researchers have developed several methods for migrating       replicas. We believe SMART can be used with semi-passive
replicated services that do not involve migrating replicated      replication, thereby enabling non-deterministic services.
state machines. In the remainder of this section, we discuss         There are well-known approaches, such as BFT [3], that
these methods and compare them to SMART.                          let replicated state machines operate correctly despite lim-
   One method is to use a view-oriented group communica-          ited Byzantine behavior. In principle, SMART could use
tion system (GCS) such as ISIS [2], Transis [6], or Horus [8].    BFT instead of basic Paxos, and thereby also tolerate such
Such a system allows a process to send a message to a group       failures. However, many changes to SMART would be nec-
of processes, and allows the set of processes in this group,      essary. For instance, we would require a method like Martin
called the view, to change. This is a useful building block for   et al.’s forgetting protocol [19] to ensure that old conﬁgura-
replicated state machines, since it allows client processes to    tions cannot mislead clients if they become faulty.
send requests to a changing group of servers. Most GCSes
also provide virtual synchrony, meaning that processes in
consecutive views see the same set of messages in the ear-        9. CONCLUSIONS
lier view [4]. This simpliﬁes coordination of replicas when         In this paper, we presented SMART, our technique for
views change. The main advantage SMART has over GCSes             migrating replicated stateful services. Unlike similar ap-
for replicating services is that Paxos deals more eﬃciently       proaches, SMART can overlap processing of multiple re-
with temporarily unavailable machines. A GCS must incur           quests, allows migrations with no overlap between consec-
the overhead of view change whenever any machine becomes          utive conﬁgurations, and allows migrations that remove or
temporarily unavailable [10, 17], while Paxos incurs compa-       replace a non-failed machine. Thus, it can migrate services
to balance load, and it can safely rely on autonomic systems      [7] J. Elson, L. Girod, and D. Estrin. Fine-grained network
to decide when to migrate. Also, we are the ﬁrst to publish           time synchronization using reference broadcasts. In Proc.
full details of how our migration technique works.                    5th OSDI, pages 147–163, Boston, MA, Dec. 2002.
   A key element of SMART is its use of conﬁguration-             [8] R. Friedman and A. Vaysburd. Fast replicated state
                                                                      machines over partitionable networks. In Proc. 16th SRDS,
speciﬁc replicas. We create a new conﬁguration by starting            pages 130–137, Durham, NC, Oct. 1997.
a new replica of the service on each machine in the new con-      [9] S. Ghemawat, H. Gobioﬀ, and S.-T. Leung. The Google ﬁle
ﬁguration. These replicas run concurrently with the repli-            system. In Proc. 19th SOSP, pages 29–43, Bolton Landing,
cas from the old conﬁguration until the new conﬁguration              NY, Oct. 2003.
is established. The replicas of each conﬁguration run a per-     [10] R. Guerraoui and A. Schiper. Consensus service: a
conﬁguration instance of Paxos; this simpliﬁes the protocol           modular approach for building agreement protocols in
since Paxos never has to run across multiple conﬁgurations            distributed systems. In Proc. 26th International
                                                                      Symposium on Fault-Tolerant Computing (FTCS-26),
simultaneously.                                                       pages 168–177, Sendai, Japan, June 1996.
   When multiple replicas run on the same machine, there is      [11] J. Howell and J. Douceur. Replicated virtual machines.
unnecessary duplication of service state, which can lead to           Technical report MSR-TR-2005-119, Microsoft Research,
expensive copying of service state across machines and/or             2005.
processes. Thus, we use shared execution modules to let          [12] J. Howell, J. R. Lorch, and J. Douceur. Correctness of
replicas on the same machine share service state.                     Paxos with replica-set-speciﬁc views. Technical report
   We evaluated the performance of our technique by run-              MSR-TR-2004-45, Microsoft Research, 2004.
ning experiments on our implementation of it. We found           [13] L. Lamport. The part-time parliament. ACM Transactions
                                                                      on Computer Systems, 16(2):133–169, May 1998.
that SMART’s ability to overlap processing of multiple re-
                                                                 [14] L. Lamport. Paxos made simple. ACM SIGACT News,
quests reduces latency of requests when there are concurrent          32(4):18–25, Dec. 2001.
requests. We also found that migration has only a small and      [15] L. Lamport. Specifying Systems: The TLA+ Language and
temporary eﬀect on performance.                                       Tools for Hardware and Software Engineers. Addison
   SMART allows a service to migrate and still preserve ab-           Wesley, 2003.
solute consistency. Any deterministic algorithm may guide        [16] E. K. Lee and C. A. Thekkath. Petal: Distributed virtual
this migration, permitting any desired mechanism for bal-             disks. In Proc. 7th ASPLOS, pages 84–92, Cambridge, MA,
ancing load and for recovering from lost fault tolerance due          Oct. 1996.
to failures. Even if the algorithm sometimes incorrectly de-     [17] N. Lynch and A. A. Shvartsman. RAMBO: A
                                                                      reconﬁgurable atomic memory service for dynamic
cides that a machine has failed and needs replacement, it             networks. In Proc. 16th International Symposium on
does not risk halting the service forever. Such autonomic             Distributed Computing, pages 173–190, Toulouse, France,
migration is an important step toward full autonomic oper-            Oct. 2002.
ation, in which administrators play a minor role and need        [18] J. MacCormick, N. Murphy, M. Najork, C. A. Thekkath,
not be constantly available.                                          and L. Zhou. Boxwood: Abstractions as the foundation for
                                                                      storage infrastructure. In Proc. 6th OSDI, pages 105–120,
                                                                      San Francisco, CA, Dec. 2004.
10.   ACKNOWLEDGMENTS                                            [19] J.-P. Martin and L. Alvisi. A framework for dynamic
  The authors would like to thank the people whose com-               Byzantine storage. In Proc. 2004 International Conference
ments on early drafts helped reﬁne this paper: Leslie Lam-            on Dependable Systems and Networks (DSN’04), pages
port, Marvin Theimer, Helen Wang, Dahlia Malkhi, Mike                 325–334, Florence, Italy, Jun. 2004.
                                                                 [20] B. M. Oki. Viewstamped replication for highly available
Schroeder, Lidong Zhou, and especially Chandu Thekkath,
                                                                      distributed systems. Ph.D. thesis technical report
who was always available to answer our detailed questions             MIT/LCS/TR-423, MIT, Aug. 1988.
about Petal. Finally, we thank the anonymous reviewers           [21] R. Rodrigues, M. Castro, and B. Liskov. BASE: Using
and our shepherd, Maurice Herlihy, for their many helpful             abstractions to improve fault tolerance. In Proc. 18th
comments and suggestions.                                             SOSP, pages 15–28, Banﬀ, Canada, Oct. 2001.
                                                                 [22] F. B. Schneider. Implementing fault-tolerant services using
                                                                      the state machine approach: a tutorial. ACM Computing
11.   REFERENCES                                                      Surveys, 22(4):299–319, Dec. 1990.
 [1] A. Adya, W. Bolosky, M. Castro, G. Cermak, R. Chaiken,
                                                                 [23] C. A. Thekkath. Personal communication. 2005.
     J. Douceur, J. Howell, J. Lorch, M. Theimer, and R. P.
     Wattenhofer. FARSITE: Federated, available, and reliable    [24] R. van Renesse and F. B. Schneider. Chain replication for
                                                                      supporting high throughput and availability. In Proc. 6th
     storage for an incompletely trusted environment. In Proc.
                                                                      OSDI, pages 91–104, San Francisco, CA, Dec. 2004.
     5th OSDI, pages 1–14, Boston, MA, Dec. 2002.
                                                                 [25] J. Yin, J.-P. Martin, A. Venkataramani, L. Alvisi, and
 [2] K. P. Birman. Replication and fault-tolerance in the ISIS
                                                                      M. Dahlin. Separating agreement from execution for
     system. In Proc. 10th SOSP, pages 79–86, Orcas Island,
     WA, Dec. 1985.                                                   Byzantine fault tolerant services. In Proc. 19th SOSP,
                                                                      pages 253–267, Bolton Landing, NY, Oct. 2003.
 [3] M. Castro and B. Liskov. Practical Byzantine fault
                                                                 [26] H. Yu and A. Vahdat. Consistent and automatic replica
     tolerance. In Proc. 3rd OSDI, pages 173–186, New Orleans,
                                                                      regeneration. In Proc. 1st NSDI, pages 323–336, San
     LA, Feb. 1999.
                                                                      Francisco, CA, Mar. 2004.
 [4] G. V. Chockler, I. Keidar, and R. Vitenberg. Group
     communication speciﬁcations: a comprehensive study.
     ACM Computing Surveys, 33(4):427–469, Dec. 2001.
 [5] X. Défago, A. Schiper, and N. Sergent. Semi-passive
     replication. In Proc. 17th SRDS, pages 43–50, West
     Lafayette, IN, Oct. 1998.
 [6] D. Dolev and D. Malkhi. The Transis approach to high
     availability cluster communication. Communications of the
     ACM, 39(4), Apr. 1996.
