                     Cheap Paxos
             Leslie Lamport and Mike Massa




                          Appeared in
The International Conference on Dependable Systems and Networks
                          (DSN 2004 )
                                                 Cheap Paxos

                                        Leslie Lamport and Mike Massa
                                                  Microsoft


                      Abstract                               processor is necessary. Suppose the system is imple-
                                                             mented by only two processors, p and q, and suppose
   Asynchronous algorithms for implementing a fault-         that q fails. The requirement that the system continue
tolerant distributed system, which can make progress         to make progress despite a single failed processor means
despite the failure of any F processors, require 2F + 1      that p must continue operating the system. Now sup-
processors. Cheap Paxos, a variant of the Paxos al-          pose that p fails and then q is repaired. Since there
gorithm, guarantees liveness under the additional as-        is only one failed processor, q must be able to resume
sumption that the set of nonfaulty processors does not       operating the system. But this is clearly impossible,
“jump around” too fast, but uses only F + 1 main pro-        since q does not know the current state of the system
cessors that actually execute the system and F auxiliary     because it does not know what p did after q failed.
processors that are used only to handle the failure of a     Some third processor is needed—for example, a disk
main processor. The auxiliary processors take part in        that can be accessed by both p and q.
reconfiguring the system to remove the failed processor,        Suppose we are willing to weaken the liveness re-
after which they can remain idle until another main          quirement, so that if q fails and then p fails before q is
processor fails.                                             repaired, then the system may halt until p is repaired.
                                                             Two processors are still not enough if we require that
                                                             consistency be maintained despite communication fail-
1    Introduction                                            ure. With only two processors p and q, one processor
                                                             cannot distinguish failure of the other processor from
The state-machine approach consists of describing a          failure of the communication medium. Since consis-
system as a state machine that takes as input a se-          tency is lost if each processor continues operating the
quence of client commands and produces a sequence of         system by itself, the system cannot allow each proces-
states and outputs [4, 10]. The state machine is im-         sor to continue just because it thinks that the other
plemented by a collection of servers. It reduces the         processor has failed. A third processor is needed. How-
problem of implementing a distributed system to that         ever, that third processor does not have to participate
of having the servers choose a sequence of commands.         in choosing the sequence of commands. It must take
Making the system reliable requires that all processors      action only in case p or q fails, after which it does
agree on each command in the sequence, despite the           nothing while either p or q continues to operate the
failure of some components. For asynchronous systems,        system by itself. The third processor can therefore be
we require that consistency be maintained in the face        a small/slow/cheap one, or a processor primarily de-
of any number of non-malicious (non-Byzantine) fail-         voted to other tasks.
ures, and that progress be ensured when enough pro-              This argument suggests that there exists a method
cessors are nonfaulty and can communicate with one           of implementing a one-fault tolerant system, satisfying
another in a timely manner [2]. The “classic” Paxos          the consistency property of classic Paxos and a weaker
algorithm is an efficient, practical algorithm for achiev-   liveness property, using two main processors plus a
ing this [1, 5, 7].                                          third auxiliary processor. This paper describes Cheap
   Consider the problem of implementing a distributed        Paxos, a generalization of such an algorithm that tol-
system that can make progress if all but one processor       erates F faults with F + 1 main processors and F aux-
is working. Previous algorithms, such as classic Paxos,      iliary processors. It maintains liveness under a sort of
require three processors. Only two of those processors       “amoeba” assumption [3], under which the subnetwork
need maintain the system state; but a third proces-          of working main processors does not move around too
sor must participate in choosing the sequence of com-        quickly. The assumption can be described as follows.
mands. The following argument shows that this third          A nonfaulty processor maintains a certain knowledge
of the system’s state. When a faulty processor is re-             2.1    The Paxos Consensus Algorithm
paired, it can, in a finite length of time, re-acquire this
                                                                  To implement a distributed system as a state machine,
knowledge from any other processor that possesses it.
                                                                  the processors of the system must choose a sequence
Liveness is maintained as long as there is at least one
                                                                  of commands. This is done by executing a sequence
main processor with knowledge of the system state and
                                                                  of instances of a consensus algorithm, the i th instance
F + 1 processors (main or auxiliary) that are nonfaulty
                                                                  choosing the i th command in the sequence. We now
and can communicate with one another in a timely
                                                                  review the Paxos consensus algorithm.
manner. Consistency is always maintained (assuming
                                                                     The goal of a consensus algorithm is for a collection
non-malicious failures).
                                                                  of processes to agree upon a value. It is most conve-
    There are two threads of previous work that super-
                                                                  nient to phrase the consensus problem in terms of three
ficially resemble Cheap Paxos. The first is the use of
                                                                  classes of agents: proposers that propose values, accep-
main processors that are replaced by spares if they
                                                                  tors that cooperate to choose a single proposed value,
fail [8]. Indeed, classic Paxos requires only F + 1
                                                                  and learners that must learn what value has been cho-
working processors to operate a system that tolerates
                                                                  sen [6]. A single processor can act as more than one
F faults; the remaining F processors can be used as
                                                                  kind of agent. The safety properties that a consensus
spares. However, unlike the auxiliary processors of
                                                                  algorithm must satisfy are:
Cheap Paxos, spares must have the necessary comput-
ing power to replace a failed main processor. The sec-            Nontriviality Only a value that has been proposed
ond thread is the use of dynamic quorum algorithms for                          may be chosen,
maintaining multiple copies of a database. These algo-
rithms can employ “witness” processors that need not              Consistency      Only a single value may be chosen.
maintain the data [9]. However, unlike the auxiliary
                                                                  Conservatism Only a chosen value may be learned.
processors of Cheap Paxos, these witnesses participate
in each operation.                                                There is also a liveness requirement that we do not try
    Two moderately recent developments in computing               to state precisely; it is discussed informally below.
may make Cheap Paxos useful. First, improvements                     The Paxos consensus algorithm has been discussed
to hardware and operating systems make computers                  elsewhere [1, 5, 6, 7], so we do not explain here exactly
less likely to crash. The weaker liveness guarantee of            how it works. Instead, we just describe its actions.
Cheap Paxos may therefore still provide sufficient relia-            Paxos assumes an underlying procedure for select-
bility. Second, the widespread use of computers makes             ing a leader. Safety is guaranteed even if no leader
it more likely that an organization will have additional          or multiple leaders are selected, but a unique leader
machines from which cycles can be “stolen” to imple-              is required to ensure progress. Proposers send their
ment the auxiliary processors.                                    proposals to the leader.
    One might think that the low cost of computers                   The consensus algorithm assumes predefined sets of
would make Cheap Paxos uninteresting. However, we                 acceptors called quorums. The only requirement on
have observed that people are no more willing to use              the quorums is that any two quorums have at least one
extra hardware to make a system simpler and more re-              acceptor in common. Paxos also assumes a set of ballot
liable than they were 40 years ago, even though that              numbers, which for simplicity we take to be the natural
hardware has become orders of magnitude cheaper.                  numbers. The ballot numbers are partitioned among
    The following section reviews Paxos, and Section 3            potential leaders, each possible leader having its own
describes Cheap Paxos. The obligatory conclusion fol-             disjoint set of ballot numbers.
lows.                                                                The consensus algorithm has two phases, each with
                                                                  two subphases. The algorithm’s actions are described
                                                                  below. The algorithm sends messages between learners
2     A Review of Paxos                                           and acceptors, and from acceptors to learners. Since
                                                                  the same processor may be playing multiple roles, it
The Paxos algorithm for implementing a distributed                can send messages to itself.
state machine was introduced in [5]. We consider two              Phase1a(l , b) Leader l chooses a number b from among
versions of Paxos. In the basic version, to which we                  its ballot numbers and sends h“1a”, b i messages to
give the name Static Paxos, the set of servers is fixed.              the acceptors.
A variation that we call Dynamic Paxos, mentioned
briefly in [5], uses state machine commands to change             Phase1b(a, b) When acceptor a receives a h“1a”, b i
the set of servers. We begin by considering Static                    message from a leader l , if it has not received any
Paxos; Dynamic Paxos is explained in Section 2.3.                     message with a ballot number greater than b, then

                                                              2
     it replies to l with a h“1b”, b, . . .i message, where             As described here, the algorithm never terminates.
     the precise contents of the message do not concern              A leader can at any time perform a Phase1a action for
     us. If a has received a message with ballot number              a new ballot number. In an application, there will be
     greater than b, it sends a reply to l indicating that           some point at which enough processes have learned the
     it is ignoring the h“1a”, b i message. (Upon receiv-            chosen value, after which processes can forget all about
     ing that message, l will perform a Phase1a(l , b 0 )            this instance of the algorithm, erasing any information
     action for b 0 > b, if it still believes itself to be the       about it from their stable storage.
     leader.)                                                           For later reference, we make the following observa-
                                                                     tions.
Phase2a(l , b) If leader l has received h“1b”, b, . . .i mes-
    sages from a quorum of acceptors, then it sends a
    h“2a”, b, v i message to the acceptors where, de-                O1. We can save messages at the cost of an extra mes-
    pending on the contents of those “1b” messages,                      sage delay by having a single distinguished learner
    either:                                                              that informs the other learners when it finds out
                                                                         that a value has been chosen. Acceptors then send
        • The value of v is determined by the “1b” mes-                  “2b” messages only to the distinguished learner.
          sages, or                                                      In most applications, the roles of leader and dis-
        • l chooses v arbitrarily from among the pro-                    tinguished learner are performed by the same pro-
          posals it has received.                                        cessor.

     This action may not be performed twice for differ-
     ent values of v (with the same b).                              O2. A leader can send its “1a” and “2a” messages just
                                                                         to a quorum of acceptors. As long as all acceptors
Phase2b(a, b, v ) If acceptor a receives a h“2a”, b, v i                 in that quorum are working and can communicate
    message and it has not already received any mes-                     with the leader and the learners, there is no need
    sage with a ballot number greater than b, it sends                   for acceptors not in the quorum to do anything.
    a h“2b”, b, v i message to every learner.

Learn(r , v , b) If learner r has received h“2b”, b, v i mes-        O3. Acceptors do not care what value is chosen. They
    sages from a quorum of acceptors, then it learns                     simply respond to “1a” and “2a” messages, using
    that the value v has been chosen.                                    their stable storage to ensure that, despite fail-
                                                                         ures, only a single value can be chosen. However,
In normal execution, the actions occur in the order                      if an acceptor does learn what value has been cho-
listed above, starting with the leader’s Phase1a ac-                     sen, it can store the value in stable storage and
tion. However, processes may fail, messages may be                       erase any other information it has saved there. If
lost or delivered out of order, and several processors                   the acceptor later receives a “1a” or “2a” message,
could simultaneously think they are the leader, caus-                    instead of performing its Phase1b or Phase2b ac-
ing “1a” and “2a” messages for several different ballot                  tion, it can simply inform the leader of the chosen
numbers to be sent concurrently. Nevertheless, the al-                   value.
gorithm maintains its three safety properties, nontrivi-
ality, consistency, and conservatism. (We are assuming
non-Byzantine failures in which a process can halt, but              O4. Instead of sending the value v , the leader can send
does not perform incorrect actions.) Moreover, if there                  a hash of v to some acceptors in its “2a” mes-
is a single working processor l that believes itself to be               sages. (A hash is a function H from values to a
the leader, has received a proposal, and can communi-                    smaller set such that there is a negligible chance
cate with a quorum of acceptors, then some value will                    that H (v ) equals H (v 0 ) for two different values
eventually be chosen. Any learner that can communi-                      v and v 0 .) A learner will learn that v is cho-
cate with this quorum of acceptors will then learn the                   sen if it receives “2b” messages for either v or its
chosen value.                                                            hash from a quorum of acceptors, and at least
    We can allow failed processes to be restarted if they                one of those messages contains v rather than its
have stable storage that survives a failure. Processes                   hash. However, a leader could receive “1b” mes-
must maintain the following amounts of information                       sages that tell it the hash of a value v that it must
in stable storage: an acceptor must keep two ballot                      use in its Phase2a action without telling it the ac-
numbers and one proposed value, and a leader must                        tual value of v . If that happens, the leader cannot
keep one ballot number (the largest one for which it                     execute its Phase2a action until it communicates
has performed a Phase2a action).                                         with some process that knows v .


                                                                 3
2.2    Implementing a State Machine                                    137 and 140, respectively.
In the state machine approach, a set of servers execute            • For instance 136 and for all instances greater than
commands submitted by clients. For simplicity, we as-                140, it can use any proposed command v in its
sume that each server keeps in stable storage the entire             Phase2a(l , b) action.
sequence of state machine commands that have been
chosen so far. In many applications, a server would              Leader l then does the following:
keep only a recent checkpoint of the state machine’s               • It performs Phase2a(l , b) actions for instances 137
state and the commands after that checkpoint.                        and 140, using the commands v 137 and v 140 deter-
   In the traditional Paxos algorithm, the clients are               mined by the “1b” messages it received.
the proposers and each server acts as an acceptor, a
learner, and a potential leader in each instance of the            • It performs the Phase2a(l , b) action for instance
consensus algorithm. A quorum consists of a majority                 136, using as the command v a special no-op state
of the servers. The leader receives client commands,                 machine command that does nothing.
assigns each one a number, and tries to get the i th
command to be chosen by the i th instance of the Paxos             • In some manner that does not concern us, it en-
consensus algorithm.                                                 sures that all servers know commands 1–135, 138,
   To understand how Static Paxos works, suppose the                 and 139.
system has been operating for a while when the leader            If a majority of the servers are working, they will per-
fails. A new server l is then selected to be leader. Since       form Phase2b actions for instances 136, 137, and 140,
l is a learner, it should know most of the commands              and all servers will learn the commands chosen for
that have already been chosen. Suppose it knows com-             all instances 1–140 of the consensus algorithm. How-
mands 1–134, 138, and 139—that is, the commands                  ever, even before that has happened, leader l can re-
chosen in instances 1–134, 138, and 139 of the consen-           sume normal operation. It assigns the number 141 to
sus algorithm. (Such a gap in its knowledge is possible          the first client command it receives, and it executes
because multiple instances of the consensus algorithm            Phase2a(l , b) for instance 141 using that command as
can be executed concurrently.) Server l chooses a bal-           the value v . It assigns number 142 to the next client
lot number b that it believes to be greater than any             command and executes Phase2a(l , b) for that instance
ballot number used by previous leaders. (The elec-               and that command as value v . And so on.
tion algorithm can be used to choose b as well as l .)               Since each server is a learner, it learns the sequence
It then simultaneously executes Phase1a(b, l ) for in-           of chosen commands. In most applications, the leader
stances 135–137 and for all instances greater than 139           will act as the distinguished learner (mentioned in ob-
of the consensus algorithm, sending “1a” messages to             servation O1 above) to which “2b” messages are sent.
all the servers. (Some of those messages are to itself,          Once a server has learned what command the i th com-
since the leader is chosen from among the servers.) It           mand is, it can delete all other information about the
can obviously send these infinitely many virtual mes-            i th instance of the consensus protocol from its storage.
sages in a single physical message.                                  When a failed server is repaired, it must be brought
   Each server then simultaneously executes Phase1b              up to date so it knows all the commands that have
actions in response to those virtual “1a” messages,              already been chosen. In principle, this is a straightfor-
sending infinitely many virtual “1b” messages back to l .        ward matter of having the newly repaired server ob-
Since those “1b” messages contain information only for           tain the information from some working server. If a
instances for which actions have been performed, those           server maintains only recent commands and a check-
virtual messages will contain only a finite amount of            point of the state, then the repaired server must update
information that can usually be fit into a single real           its saved checkpoint. If the state machine maintains a
message. By observation O3 above, if a server knows              large state, this must be done in such a way that only
that a command was already chosen by some instance,              the part of the state that has changed is sent to the
it responds with the chosen command rather than a                repaired server.
“1b” message for that instance.
   Suppose that, from these messages, l learned:
                                                                 2.3     Dynamic Paxos
  • The command that was chosen in instance 135
    (sent by some server instead of an instance 135              So far, we have described Static Paxos, in which the set
    “1b” message).                                               of acceptors and the quorums are constant and fixed in
                                                                 advance. A system that must continue working despite
  • Commands v 137 and v 140 that it must use as the             the failure of any F processors then requires 2F + 1
    value v in its Phase2a(l , b) actions for instances          servers. For example, with Static Paxos, it takes seven

                                                             4
servers to tolerate three failures. In many systems, the          easily be implemented with the state machine.
best way to achieve the desired degree of fault tolerance            When a new server is added to the system, it must
is to reconfigure the system to replace failed servers by         learn the current state machine state. This is essen-
spares. With reconfiguration, a system that uses three            tially the same problem as bringing a failed server up
active servers and two spares can tolerate a total of             to date, which is discussed in Section 2.2 above.
three failures, if a failed server can be replaced by a
spare before another failure occurs. Reconfiguration
therefore allows fewer processors to tolerate the same            3     Cheap Paxos
total number of failures, though not the same number
of simultaneous failures. (In most systems, simultane-            3.1    The Algorithm
ous failures are much less likely than successive ones.)
                                                                  We now develop Cheap Paxos as an instance of Dy-
    In Dynamic Paxos, the set of acceptors and the                namic Paxos. In Cheap Paxos, we posit a system of
quorums are determined by the state machine itself.               F + 1 main processors and F auxiliary processors. The
Reconfiguration is performed by state machine com-                main processors act as the servers in a distributed state
mands. To explain how this works, let state k be the              machine implementation. The auxiliary processors per-
state machine’s state after executing command k . For             form actions only in the event of the failure of a main
k ≤ 0, define state k to be the initial state. For some           processor, after which the main processors continue to
fixed constant α, we let the acceptors and quorums                operate the system by themselves.
used for instance i of the consensus algorithm be deter-             The key to Cheap Paxos is observation O2. In nor-
mined by state i − α. Before performing any action for            mal operation of the Paxos consensus algorithm, the
instance i , a leader waits until it knows state i − α. In        leader sends only “2a” messages. By O2, those mes-
other words, a leader must wait until it knows all com-           sages need be sent to and acted upon by only a quo-
mands through command number i −α before it knows                 rum of acceptors. Hence, to implement Cheap Paxos,
to which acceptors it should send its “2a” messages for           we use Dynamic Paxos to configure the system so that
the i th instance of the Paxos consensus algorithm.               the set of all working main processors forms a quorum.
    As a simple example of how this might work, con-              As long as these processors continue working, they can
sider a system with a fixed set S of processors that can          execute the system. If one of them fails, then the quo-
act as servers. Let the set of servers currently execut-          rum consisting only of main processors can no longer
ing the system (and acting as acceptors) be G, and let            succeed in choosing commands. A different quorum,
a quorum consist of a majority of the processors in G.            containing one or more auxiliary processors, is then
Suppose we want a processor to be declared to have                used to (i) complete the execution of any of the in-
failed or have been repaired if a majority of the pro-            stances of the Paxos consensus algorithm that were in
cessors in G believe it has. The state machine’s state            progress when the failure occurred, and (ii) propose
would contain the set G together with a Boolean ar-               and choose the necessary state machine commands to
ray good , where good [p, q] indicates whether processor          reconfigure the system. The reconfiguration removes
p believes processor q is nonfaulty, for all p, q ∈ S . A         the failed processor and modifies the set of quorums so
processor r would issue a state machine command to                the remaining main processors form a quorum. These
change the value of good [r , s] when it believes processor       main processors can then resume executing the system,
s has failed or been repaired. Such a command would               while the auxiliary processors once more become idle.
set the new value good 0 of the array good in the obvious            Cheap Paxos uses Dynamic Paxos, where the set
way, and it would set the new value G 0 of G to equal the         G of all processors that are currently acceptors is de-
set of all processors q ∈ S such that good 0 [p, q] equals        termined by the state machine state. Let M be the
true for a majority of processors p ∈ G. (Remem-                  subset of G consisting of all the main processors in
ber that a change to G caused by command number i                 G. We want M to be a quorum. Since the only re-
takes effect beginning with instance i + α of the Paxos           quirement on quorums is that any two of them have
consensus algorithm.)                                             a non-empty intersection, we can let M be a quorum
    In practice, deciding when to reconfigure a system            and let the other quorums consist of all sets contain-
is not easy. Replacing servers that have not failed can           ing a majority of the processors in G and at least one
cause the system to run out of servers; but not re-               processor in M . (If M contains only a single processor
placing a failed server lowers the system’s tolerance to          p, then a quorum can consist of any set containing p,
additional failures. One would probably not use so                of course including M itself.) We require that G con-
naive an algorithm as the one just described. Instead,            tain at least one main processor—a condition that is
one would use a more sophisticated algorithm, tuned               satisfied by any sensible reconfiguration algorithm be-
for the particular system. Any desired algorithm can              cause failure of all main processors implies that there

                                                              5
is no quorum of working processors, so no state ma-              main processors have learned the commands chosen in
chine commands can be chosen until a main processor              those instances. Therefore, before the system resumes
is repaired. (A non-sensible reconfiguration algorithm           normal execution, the following steps should also be
could gratuitously remove the last working main pro-             performed.
cessor from G.)
                                                                  6. The leader ensures that all processors in (the new)
   In normal operation, the processors in M execute
                                                                     M know all commands through command number
phase 2 of successive instances of the Paxos consensus
                                                                     j (defined in step 5).
algorithm to choose the sequence of state machine com-
mands. They can perform reconfiguration commands                  7. The leader instructs all auxiliary processors to re-
to add a repaired main server to G and M . However,                  member in stable storage that instances 1 through
if a main processor fails, then there will no longer be              j of the consensus algorithm have chosen com-
a working quorum consisting only of main processors.                 mands, and that they have performed no actions
The following sequence of steps is then performed.                   for any other instances. (They need not record
                                                                     any of the chosen commands.) The auxiliary pro-
 1. If the failed processor was the leader, a new leader
                                                                     cessors can then erase from stable storage all in-
    is selected from among the processors in M that
                                                                     formation relevant to any of the first j instances
    are still working.
                                                                     of the consensus algorithm.
 2. The leader interrogates the other working main
                                                                    This sequence of steps describes what normally hap-
    processors to learn all chosen commands that any
                                                                 pens when a main processor fails. A complete algo-
    main processor knows about.
                                                                 rithm must handle abnormal cases as well—for exam-
 3. The leader completes the execution of any in-                ple, if the leader fails while these steps are being per-
    stances of the Paxos consensus algorithm that                formed, or if two processors both believe they are the
    were in progress when the main processor failed,             leader and have different views of what processor has
    using a quorum that contains one or more aux-                failed. But the actions performed in these steps are just
    iliary processors. If a new leader was chosen in             implementing Dynamic Paxos. (Steps 2 and 6 simply
    step 1, it does this as described in Section 2.2             disseminate knowledge of what commands have been
    above by choosing a new ballot number and ini-               chosen.) The precise definition of these actions is there-
    tiating phase 1 with that ballot number for all rel-         fore the same as in ordinary Paxos. The only difference
    evant instances of the consensus algorithm. If the           is that an auxiliary processor may not be able to re-
    old leader is still working, it just sends to the aux-       spond appropriately to a “1a” or “2a” message because
    iliary processors the same “2a” messages that it             it has erased the needed information in step 7. In that
    had already sent to the main processors.                     case, instead of replying with the chosen command as
                                                                 indicated in observation O3, it must ignore the mes-
 4. As in standard Dynamic Paxos, the working pro-               sage. (It could report to the leader why it is ignoring
    cessors propose and choose a sequence of state ma-           the message, advising the leader to ask a main proces-
    chine commands to reconfigure the system so the              sor what command was chosen.)
    failed main processor is removed from the set G of              The auxiliary processors are needed only in the event
    acceptors (and hence from M ).                               of failure of one of the main processors, at which time
                                                                 they must participate in the execution of only a small
 5. The leader proposes and gets chosen a sequence of
                                                                 number of instances of the Paxos consensus algorithm.
    α no-op state machine commands. Let j be the
                                                                 This would seem to imply that they do not need much
    command number of the last of these commands.
                                                                 processing power. However, the consensus algorithm
After step 5, the new set G of acceptors chosen in step 4        requires them to write proposed commands to stable
is in effect for subsequent instances of the consensus al-       storage. In some applications, such commands could
gorithm. This means that the set M of main processors            be quite big, and writing them to stable storage could
in G constitute a quorum, so they can resume execut-             be expensive. If this is the case, we can apply obser-
ing the system. However, remember that the Paxos                 vation O4 and have auxiliary processors receive and
consensus algorithm’s ability to recover from failures           store only hashes of proposed commands. Since every
rests on acceptors maintaining certain information in            quorum contains a main processor, a learner receiving
stable storage. To bound the amount of storage re-               “2b” messages from a quorum must receive at least one
quired by auxiliary processors, they need to be able to          that contains the command rather than its hash. How-
forget the information they saved in steps 3–5, where            ever, we need to prevent the problem mentioned in O4,
they participated in executing instances of the consen-          in which progress is prevented because a leader knows
sus algorithm. They can do that after the working                only the hash of a value without knowing the value

                                                             6
itself. This is done by having the leader delay send-           In particular, two different servers can never disagree
ing a “2a” message with the hash of a value v to any            about the value of the i th command, for any i .
auxiliary processor until all the working main proces-              The liveness properties of Cheap Paxos can also be
sors have acknowledged receipt of their “2a” messages           inferred from those of the Paxos consensus algorithm.
containing v .                                                  However, this is complicated because Cheap Paxos is
   As in ordinary Dynamic Paxos, we are not commit-             an implementation of Dynamic Paxos, in which liveness
ted to any particular algorithm for determining that a          depends on precisely how the reconfiguration is per-
processor has failed or has been repaired. Since the            formed. For example, the system can make no progress
reconfiguration is performed by state machine com-              if a reconfiguration selects a set of failed or nonexistent
mands, any algorithm can be used. In practice, the al-          processors as acceptors. Moreover, simply being able
gorithm used by Cheap Paxos for determining whether             to choose new commands doesn’t ensure progress. To
a main processor has failed will be quite different from        be able to execute the i th command, a server needs
that used in traditional Dynamic Paxos. In traditional          to know not just that command, but also all previous
implementations of Dynamic Paxos, any majority of               commands. For example, the system could make no
acceptors constitutes a quorum, so system can continue          progress if command 1 had been chosen, but no work-
to make progress even if a server has failed. In that           ing server knew its value. Cheap Paxos also has the
case, one can afford to wait to make sure the server            additional complication that auxiliary processors for-
has really failed before reconfiguring it out of the sys-       get information that can be used by ordinary Paxos to
tem. But Cheap Paxos stops making progress as soon              recover from certain failures.
as one main processor fails. It must therefore be more              To state the liveness property satisfied by Cheap
aggressive in removing processors that may have failed.         Paxos, we need some definitions. We call a set of pro-
Although this difference affects the details of deciding        cessors nonfaulty if they are all working and can com-
what processors are working, it does not change the             municate with one another in a timely fashion. Define
basic state machine algorithm.                                  command number i to be recorded if some auxiliary
   We have been tacitly assuming that the set of all            processor has stored in its stable storage the fact that
processors (main plus auxiliary) is fixed. Reconfigura-         i has been chosen. (That is, the auxiliary processor
tion can also be used to change the set of processors.          has recorded in step 7 of the reconfiguration procedure
We are already reconfiguring the set of acceptors to            that all commands numbered through j have been cho-
remove failed main processors and add repaired ones.            sen, for some j ≥i .) A command number i is said to be
Additional fault tolerance can be obtained by replacing         active if i has not been recorded, but a “2a” message
failed auxiliary processors with spares, just as in ordi-       has been sent for instance i of the consensus algorithm.
nary Dynamic Paxos. This reconfiguration can be per-            We define a main processor to be up-to-date if it knows
formed with state machine commands executed by only             all recorded commands. (If auxiliary processors store
the main processors; auxiliary processors need only pe-         only hashes of values, then for a main processor p to
riodically inform the main processors that that are still       be up-to-date, it must also satisfy the following condi-
working. A newly installed auxiliary processor needs            tion: For every active command number i and every
to remember in its stable storage only that it has not          “2a” message sent to an auxiliary processor in instance
performed any actions for any instances of the Paxos            i of the consensus algorithm, p must have received its
consensus algorithm.                                            corresponding “2a” message.)
   An auxiliary processor acts only as an acceptor, not             We can now state the liveness property satisfied by
a learner, so it does not know what commands are cho-           Cheap Paxos. Step 6 of the reconfiguration procedure
sen. Hence, if reconfiguration can change the set of            assumes some method of propagating knowledge of cho-
auxiliary processors, an auxiliary processor does not           sen commands. We assume that knowledge is contin-
know whether it is an acceptor. This makes no dif-              ually exchanged among the main processors so that, if
ference. As observed in O3, acceptors simply respond            p and q are main processors, p knows a command, and
to “1a” and “2a” messages. Only leaders and learn-              the set {p, q} is nonfaulty for long enough, then q will
ers, which are roles played by main processors, need to         learn that command. The liveness property satisfied
know the set of acceptors and the quorums.                      by Cheap Paxos is then:
                                                                    The system makes progress if there is a non-
3.2    Correctness of Cheap Paxos                                   faulty set of processors containing a unique
                                                                    leader, at least one up-to-date main proces-
Cheap Paxos uses the Paxos consensus algorithm to                   sor, and, for all active command numbers i , a
choose commands. Its safety properties follow directly              quorum for instance i of the consensus algo-
from the safety properties of the consensus algorithm.              rithm.

                                                            7
Informally, we can view the set of nonfaulty main pro-            [7] B. W. Lampson. How to build a highly available sys-
cessors as an “amoeba” that withdraws a pseudopod                     tem using consensus. In O. Babaoglu and K. Marzullo,
when a processor fails and extends one when a pro-                    editors, Distributed Algorithms, volume 1151 of Lec-
cessor is repaired. It takes time for knowledge of cho-               ture Notes in Computer Science, pages 1–17, Berlin,
sen commands to flow into a new pseudopod. If the                     1996. Springer-Verlag.
                                                                  [8] B. Liskov, S. Ghemawat, R. Gruber, P. Johnson, and
amoeba were to move around too quickly, knowledge
                                                                      L. Shrira. Replication in the harp file system. In Pro-
could be lost because processors failed before their                  ceedings of the thirteenth ACM symposium on Oper-
knowledge could flow to newly repaired processors.                    ating systems principles, pages 226–238. ACM Press,
Assuming that the total number of nonfaulty proces-                   1991.
sors (main plus auxiliary) remains large enough, Cheap            [9] J.-F. Pâris and D. D. E. Long. Voting with regenerable
Paxos guarantees that the system will continue to make                volatile witnesses. In Proceedings of the Seventh Inter-
progress as long as the amoeba moves slowly enough                    national Conference on Data Engineering, April 8-12,
that such knowledge is not lost.                                      1991, Kobe, Japan, pages 112–119. IEEE Computer
                                                                      Society, 1991.
                                                                 [10] F. B. Schneider. Implementing fault-tolerant services
4    Conclusion                                                       using the state machine approach: A tutorial. ACM
                                                                      Computing Surveys, 22(4):299–319, Dec. 1990.
Cheap Paxos is a variant of the Paxos algorithm that
can make progress in the face of up to F failures by us-
ing F + 1 main processors plus F auxiliary processors.
Unlike the spare processors used in previous systems,
our auxiliary processors need do nothing except for a
brief period after a main processor fails. The auxil-
iary processors therefore do not require nearly as much
processing power or storage as the main processors. By
using auxiliary processors, Cheap Paxos can lead to a
system that achieves greater fault tolerance than other
algorithms with the same number of main processors.

Acknowledgement
Butler Lampson pointed out to us the problem of large
commands, and its solution through observation O4.
He was also first to observe that, in a configuration with
a single working main processor, every set of processors
containing that processor is a quorum.

References
 [1] R. De Prisco, B. Lampson, and N. Lynch. Revisit-
     ing the paxos algorithm. Theoretical Comput. Sci.,
     243:35–91, 2000.
 [2] C. Dwork, N. Lynch, and L. Stockmeyer. Consensus in
     the presence of partial synchrony. J. ACM, 35(2):288–
     323, Apr. 1988.
 [3] L. Lamport. The implementation of reliable dis-
     tributed multiprocess systems. Computer Networks,
     2:95–114, 1978.
 [4] L. Lamport. Time, clocks, and the ordering of events
     in a distributed system. Commun. ACM, 21(7):558–
     565, July 1978.
 [5] L. Lamport. The part-time parliament. ACM Trans.
     Comput. Syst., 16(2):133–169, May 1998.
 [6] L. Lamport. Paxos made simple. ACM SIGACT News
     (Distributed Computing Column), 32(4 (Whole Num-
     ber 121)):18–25, Dec. 2001.


                                                             8
