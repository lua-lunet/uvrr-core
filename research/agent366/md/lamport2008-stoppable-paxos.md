                   Stoppable Paxos
Leslie Lamport               Dahlia Malkhi                 Lidong Zhou
                          Microsoft Research

                               April 28, 2008


             Contact Author: Dahlia Malkhi
                             Microsoft Research
                             1065 La Avenida
                             Mountain View, CA 94043
                             U.S.A.
                             +1 650 693-1362
                             dalia@microsoft.com


                            regular submission
                            not a student paper




                                  Abstract
  A stoppable state machine is one whose execution can be terminated by a
  special stopping command. Stoppable state machines can be used to imple-
  ment reconfiguration in a replicated state machine; a reconfigurable state
  machine is implemented by a sequence of stoppable state machines, each
  running in a fixed configuration. Stoppable Paxos, a variant of the ordinary
  Paxos algorithm, implements a replicated stoppable state machine.
Contents
1 Introduction                                                               1

2 Paxos Revisited                                                            3
  2.1 The Paxos Consensus Protocol . . . . . . . . . . . . . . . . . . .     4
  2.2 The Paxos State Machine Implementation . . . . . . . . . . . . .       6

3 The Stoppable Paxos Algorithm                                              7

4 Correctness                                                                9

References                                                                  10

Clearly Marked Appendix: The Proof of Correctness                            13
   A.1 The Proof of Safety . . . . . . . . . . . . . . . . . . . . . . . . . 13
   A.2 The Proof of Progress . . . . . . . . . . . . . . . . . . . . . . . . 19
1    Introduction
State machine replication is a well-known method of implementing a fault-
tolerant service [11, 14]. The service is described as a deterministic state ma-
chine that accepts client commands and produces outputs, and multiple replicas
of the state machine are implemented. The different replicas operate indepen-
dently and asynchronously. However, they all have the same initial state and
execute the same sequence of commands, so they all produce the same sequence
of outputs. Since each replica can respond to any client request, using f + 1
replicas allows the system to tolerate the failure of f processes.
     Implementing a replicated state machine requires a fault-tolerant algorithm
for choosing the sequence of state machine commands executed by the replicas.
Such an algorithm must guarantee that, for each i , if a replica executes c as
the i th command in the command sequence, then (i) c was issued by a client,
and (ii) no replica executes a different command as the i th command in the
sequence.
     Different replicas operate asynchronously, so they may execute the same
command at different times. Moreover, if the output produced by executing
command number i does not depend on what commands are executed as num-
bers 1 through i − 1, then a replica may generate the output for command i
before it generates the output for command i − 1. Hence, although the replicas
all produce the same sequence of outputs, the outputs in that sequence could
be generated in different orders.
     An asynchronous algorithm for choosing the sequence of state machine com-
mands requires at least 2f + 1 processes to tolerate the (non-malicious) failure
of f of them [4]. Hence, we need more processes to choose the commands than
to execute the replicas. The processes that choose the sequence of commands
are called acceptors, and the ones that execute the replicas are called learners.
     We can choose a sequence of commands by using a separate consensus pro-
tocol to choose each one, where the i th consensus protocol chooses the i th
command. The protocol used to choose the i th command will be called the
i th consensus instance. Separate consensus instances need not have disjoint
implementations—for example, messages belonging to separate instances may
be batched in a single physical message. However, the separate instances are
logically independent, which makes reasoning about their correctness easier.
     Different commands in the command sequence can be chosen concurrently.
Processes can begin the i th consensus instance without waiting for instances
1 to i − 1 to terminate. This concurrent processing is vital to the efficiency
of an asynchronous distributed system. For example, in a typical leader-based
protocol, the current leader can send proposals for commands one after another,
without waiting for acknowledgements of previous proposals.
     In a static system, all consensus instances are instances of the same algo-
rithm. In particular, they all use the same sets of acceptors and learners—sets
we call the configuration. However, achieving long-term resilience requires a
reconfigurable system, in which the configuration can change. A reconfigurable


                                       1
system requires that, for each i , there be agreement on the configuration that
is to execute consensus instance i .
    In state machine replication, reconfiguration has traditionally been done
by using the state machine itself to perform special reconfiguration com-
mands [10, 14, 15]. The obvious method is to have a reconfiguration command
change the configuration for all subsequent instances until the next reconfigu-
ration command. However, since a consensus instance cannot be executed until
the configuration executing it is known, this prevents concurrent execution of
different instances. We can permit concurrent execution of up to α consensus
instances by instead letting a reconfiguration command executed as command
number i determine the configuration starting from instance i + α, where α is
a system parameter [10]. Instances i + 1 through i + α can begin execution
once commands 1 through i have been chosen. In practice, α can be made large
enough so the system never has to wait to learn the current configuration, if
no reconfiguration command has been issued. However, this approach has two
somewhat awkward properties:

   • To force a reconfiguration to happen quickly, a reconfiguration command
     must be followed by α − 1 no-op commands that have no effect.

   • If α > 1, then several reconfiguration command could appear among com-
     mands i through i + α − 1, meaning that one configuration is choosing the
     next several configurations. This can happen when using a leader-based
     consensus algorithm if a failure causes multiple processes to each think it
     is the leader.

In this paper, we propose an alternative reconfiguration procedure based on
stoppable state machines. A stoppable state machine has a special class of stop-
ping commands that terminate the state machine. If a stopping command is
chosen as the i th command, then the complete sequence of chosen state ma-
chine commands has length i . That is, if a stopping command is chosen as
command i , then no command can ever be chosen as command j for j > i .
We implement the system state machine by executing a sequence of stopping
state machines. Each consensus instance of a single stopping state machine is
executed by the same configuration. Reconfiguration is performed by stopping
the current state machine and starting a new one with a new configuration. The
stopping command specifies the configuration used to execute the new stopping
state machine. The system’s complete sequence of state machine commands is
the concatenation of the command sequences of the individual stopping state
machines. If one stopping state machine is terminated by executing a stopping
command as command number i , then we can number the commands of the next
stopping state machine starting with i + 1. This provides consecutive numbers
for the commands in the system state machine.
    This method of reconfiguring with stoppable state machines seems to corre-
spond more closely to the way engineers have traditionally approached recon-
figuration. It is similar to view changing in group communication [1, 2, 3, 5,

                                       2
6, 7, 8, 12]. However, the purpose of this paper is to present Stoppable Paxos,
an algorithm for implementing a stoppable state machine. We have discussed
reconfiguration only to indicate why stoppable state machines may be useful.
A detailed description of how stoppable state machines are used for reconfigu-
ration and how they relate to group communication is beyond the scope of this
paper.
     Stoppable Paxos is a variant of Paxos [10]. Our goal was to devise an
algorithm that is as efficient as Paxos in the absence of a stopping command.
This is not easy to do because Paxos allows the choosing of the i th command to
be performed concurrently for different values of i . We must avoid the possibility
that the i th command is chosen and a stopping command is concurrently chosen
as the j th command for j < i . The obvious method is to delay the choice
of the i th command until all previous commands are chosen, but this would
considerably degrade the performance. Stoppable Paxos adds no messages or
delays to ordinary Paxos, except that a leader cannot propose an i th command
if, in the normal course of execution, it learns that a stopping command has
been chosen or was proposed and may have been chosen as the j th command
for some j < i . Although the basic idea of the algorithm is not complicated,
getting the details right was not easy.
     The following section reviews ordinary Paxos. The Stoppable Paxos algo-
rithm is described in Section 3, and its correctness properties are stated in
Section 4. A proof of correctness appears in the appendix for reading at the
program committee’s discretion.


2    Paxos Revisited
Ordinary Paxos assumes a distributed system of processes communicating by
messages. Processes can fail only by stopping, and messages can be lost or
duplicated but not corrupted. Timely actions by non-failed processes and timely
delivery of messages among them is required for progress; safety is maintained
despite arbitrary delays and any number of failures.
    The core of Paxos is a consensus algorithm (originally called the Synod
algorithm). In a consensus algorithm, client processes can propose values, and
learner processes each learn a value. We use the term command for a value that
may be proposed. A consensus algorithm must satisfy two safety properties:
Consistency No two learners can learn different commands.

Nontriviality Any command learned must have been proposed.
For almost all consensus algorithms, including Paxos, nontriviality is easily seen
to hold. We therefore ignore it and consider only consistency. A consensus
algorithm must also ensure that, under some suitable hypothesis, a command is
learned.
    A replicated state machine is implemented with a sequence of instances of a
consensus algorithm, the i th instance choosing the i th state-machine command.

                                        3
We briefly review the Paxos consensus algorithm and how it is used to implement
a state machine. We consider the static case, in which the same processes
implement all consensus instances.

2.1   The Paxos Consensus Protocol
The Paxos consensus algorithm assumes three sets of processes: leaders that
propose commands, acceptors that choose a command, and learners that learn
the chosen command. These sets are not necessarily disjoint—in particular,
leaders are usually learners. We ignore the clients, which provide commands
for the leaders to propose. Leaders propose commands in numbered ballots.
For simplicity, we take ballot numbers to be natural numbers. A configuration
assigns to each ballot a unique leader that performs actions of that ballot. For
example, the leader of ballot number b may be determined by the low-order bits
of b. We also assume certain sets of acceptors to be quorums, subject only to
the requirement that the intersection of any pair of quorums is non-empty.
    Acceptors accept and store proposed commands and their ballot numbers.
In particular, each acceptor a maintains the value bal [a] that records the highest
ballot number that a has received, “voted for”, and acknowledged, and the value
vote[a][b] that records the command proposed with ballot number b that a has
voted for. Initially, bal [a] equals −∞ and vote[a][b] equals >, a special value
that is not a command.
    For any acceptor a, let maxbal (a) be the largest ballot number b for which
vote[a][b] 6= >, and to equal −∞ if vote[a][b] = > for all b. Define maxvote(a)
to equal vote[a][maxbal (a)], or > if maxbal (a) = −∞. Instead of maintaining
the entire array vote[a], acceptor a need only record the values maxbal (a) and
maxvote(a). For simplicity, we ignore this optimization.
    At any point during the execution of the consensus algorithm, the state
consists of the values of the arrays bal and vote and the sets of messages that
have been sent and received by the processes. A state function is an expression
whose value depends on the state.
    A ballot consists of two phases, each with two sub-phases. In the first phase,
the leader determines whether a command may have been chosen in a lower-
numbered ballot. In the second phase, it proposes a command and the acceptors
vote for that command. The command is chosen if a quorum of acceptors vote
for it.
    The heart of the algorithm is the state function val 2a(b, Q), which the ballot
b leader computes on the basis of messages it receives in phase 1 from acceptors
in the quorum Q. If val 2a(b, Q) equals a command v , then v might have been
chosen in a lower-numbered ballot and the leader must propose it in phase 2.
If val 2a(b, Q) equals >, then no command has been or ever will be chosen in a
lower-numbered ballot, and the leader can propose any value. We define val 2a
below. First, we describe the following actions that the algorithm can perform.
Phase1a(b) The leader of ballot number b sends the message h“1a”, b i to all
     acceptors. (This action is always enabled.)

                                        4
Phase1b(a, b) When acceptor a receives a h“1a”, b i message with b > bal [a], it
     sets bal [a] to b and sends the message

            h“1b”, a, b, hmaxbal (a), maxvote(a)ii

      to the ballot b leader. (The acceptor ignores a h“1a”, b i message if bal [a] ≥
      b.)

Phase2a(b, v , Q) This action is performed by the ballot b leader, for a command
     v and quorum Q. It is enabled iff the following three conditions are
     satisfied:

      E 1(b, Q) The leader has received a message of the form h“1b”, a, b, r i
           from every acceptor a in Q.
      E 2(b) The leader has not executed a Phase2a(b, w , U ) action for any w
           and any quorum U .
      E 3(b, Q, v ) If val 2a(b, Q) 6= > then v = val 2a(b, Q).

      The action sends the message h“2a”, b, v i to all acceptors.

Phase2b(a, b, v ) When acceptor a receives a h“2a”, b, v i message from the bal-
     lot b leader and bal [a] ≤ b, it sets bal [a] to b and vote[a][b] to v and it
     sends a h“2b”, b, v i message to the learners. (The h“2a”, b, v i message is
     ignored if bal [a] > b.)

We omit the action by which a learner learns a command. It is enabled by the
receipt of a h“2b”, a, b, v i message from every acceptor a in a quorum. Instead,
we say that a command v is chosen if there exists a ballot number b and a
quorum Q such that vote[a][b] = v for all a in Q. Consistency is obviously
satisfied by ensuring that, if any commands v and w are chosen, then v = w .
    The state function val 2a(b, Q) is defined as follows. Let R be the set of all r
such that the ballot b leader has received from some acceptor a in Q the message
h“1b”, a, b, r i. (The elements of R are pairs hc, v i with either c a ballot number
and v a command, or c = −∞ and v = >.) Let hc, v i be an element of R such
that c ≥ d for all hd , w i ∈ R, and define val 2a(b, Q) to equal v . (For any state
reachable during execution of the algorithm, hc, v i ∈ R and hc, w i ∈ R imply
v = w , so this uniquely defines val 2a(b, Q).) For later reference, we also define
mbal 2a(b, Q) to equal c.
    Although the algorithm executes a sequence of ballots, those ballots need not
be executed sequentially. It is possible for two or more leaders to be executing
Phase1a and/or Phase2a actions concurrently, and for the resulting messages to
be received by different acceptors in different orders. This can impede progress
but cannot cause inconsistency.
    To achieve progress, Paxos uses some algorithm to select a unique active
leader. The active leader starts a new ballot with a number higher than that
of any other ballot it knows to have been started. An acceptor a informs the

                                         5
leader it has chosen too low a ballot number if it receives a ballot b message
with b < bal [a], causing the leader to choose a higher-numbered ballot. This
achieves progress if there is a unique active leader and a quorum of acceptors
that are nonfaulty and can communicate in a timely fashion. For most systems,
it is easy to devise a leader-selection algorithm that works properly when the
system is behaving normally.

2.2   The Paxos State Machine Implementation
Paxos executes a sequence of instances of the Paxos consensus algorithm. For
each instance i , it maintains an array vote i , where vote i [a][b] is the value of
vote[a][b] for instance i of the consensus algorithm. Paxos achieves its efficiency
by executing Phase 1 simultaneously for all instances of the consensus algorithm
as follows, using the same value of bal [a] for all of them. More precisely:

   1. The ballot b leader simultaneously executes Phase1a(b) for all instances,
      sending a single Phase1a message to each acceptor.

   2. Upon receipt of a Phase1a message, an acceptor simultaneously executes
      Phase1b actions for all instances, bundling the infinite set of Phase1b
      messages in a single physical message. (That physical message contains
      only a finite amount of information because, for any acceptor a and ballot
      number b, the value of vote i [a][b] is > for all but a finite number of
      instances i .)

We add an extra instance parameter to the Phase2a and Phase2b actions and
to the val 2a and mbal 2a state functions. For example, Phase2a(i , b, v , Q) is
the Phase2a(b, v , Q) action of instance i and val 2a(i , b, Q) is the state function
val 2a(b, Q) of instance i . We subscript messages with instance numbers, so
h“1b”, . . .ii is a Phase1b message sent for instance i (and bundled with Phase1b
messages sent for other instances).
    In normal operation, there is a single active leader that receives client com-
mands and performs Phase2a actions for them. When the active leader fails, a
new active leader is selected that performs a Phase1a(b) action for a new ballot
number b higher than any that has been used so far. When the active leader
receives Phase1b messages from a quorum Q, for each i it finds either:

   1. val 2a(i , b, Q) is a command v , meaning that v may have been chosen in
      instance i at some ballot less than b.

   2. val 2a(i , b, Q) = >, meaning that no command can have been (or can ever
      be) chosen in instance i at any ballot less than b.

In case 1, it performs a Phase2a(i , b, v , Q) action to try to get v chosen. Let
k be the largest instance for which this case holds, so it is the highest instance
in which any acceptor in Q has voted. For all instances i with i < k such that
val 2a(i , b, Q) = >, the leader performs a Phase2a(i , b, noop, Q) action to try


                                         6
to choose a noop command that does nothing. Without waiting for responses
to its Phase2a messages, the leader can begin performing Phase2a actions in
instances higher than k for client commands.
    It is possible for a leader to learn a set of commands that have already been
chosen and to optimize this procedure to avoid unnecessary actions for those
chosen commands. This optimization is straightforward and we will not discuss
it.
    Remember that what we have just described is how Paxos works in the
normal case when there is a single active leader. Consistency is maintained
even if multiple processes believe themselves to be the active leader. A single
active leader is required only to ensure progress.


3    The Stoppable Paxos Algorithm
Stoppable Paxos uses the same variables and sends the same messages as Paxos.
Before describing the actual algorithm, we sketch how the Stoppable Paxos
algorithm works in the normal case when a (single) new active leader is selected.
     As in ordinary Paxos, the new active leader performs a Phase1a(b) action
for a suitable ballot number b. The algorithm differs from Paxos if the leader
finds that a stopping command stp might have been chosen in some instance i .
In that case, the leader performs Phase2a actions for lower-numbered instances
as before. However, to ensure that the state machine stops when it should, we
must ensure that the leader does not perform a Phase2a action for any instance
greater than i if the stopping command actually was chosen in instance i .
     The problem is to decide what the leader should do if it finds val 2a(i , b, Q)
equal to a stopping command stp and val 2a(j , b, Q) equal to any command, for
some i and j with j > i . The answer depends on the values of mbal 2a(i , b, Q)
and mbal 2a(j , b, Q). Remember that, for any k , the value of mbal 2a(k , b, Q)
is the highest ballot number less than b for which some acceptor a in Q set
voted k [a]. If mbal 2a(j , b, Q) > mbal 2a(i , b, Q), then the stopping command c
could not have been chosen in (a lower ballot of) instance i , so stp is voided —
meaning that the leader acts as if val 2a(i , b, Q) equals >. Otherwise, the leader
performs a Phase2a(i , b, stp, Q) action to try to get stp chosen and does nothing
in any higher-numbered instance, including instance j .
     If the leader performs a Phase2a action for a stopping command in instance
i , then it performs no Phase2a actions for instances greater than i . Otherwise, it
begins processing new client commands as in ordinary Paxos. It continues until
it performs a Phase2a action for a stopping command, whereupon it performs
no further Phase2a actions for any higher-numbered instance. Except when
the leader is prevented from performing Phase2a actions because of a stopping
command, Stoppable Paxos allows all the concurrent execution that ordinary
Paxos does.
    We now begin our description of the actual Stoppable Paxos algorithm. As
in ordinary Paxos, the algorithm can be optimized to take advantage of knowl-


                                         7
edge of already-chosen commands. For simplicity, we ignore this optimization.
Stoppable Paxos then differs from Paxos only in the enabling conditions of the
Phase2a action. We begin with an intuitive description of these enabling con-
ditions, which are labeled E 1–E 6.
    Conditions E 1–E 3 are the same as for ordinary Paxos except that, in E 3,
we replace val 2a by a new state function sval 2a. Recall that E 3 requires
val 2a(i , b, Q) to be the proposed command if it does not equal >, because
in that case it might have been chosen in a lower-numbered ballot. We will
define sval 2a(i , b, Q) to be the same as val 2a(i , b, Q) except that it equals >
if val 2a(i , b, Q) is a stopping command that is voided. As indicated above, a
stopping command is voided if information about higher-numbered instances
implies that the command could not have been chosen in this instance in a
lower-numbered ballot.
    Enabling condition E 4 applies iff v is a stopping command, in which case it
requires the two conditions:

E 4a A Phase2a action must not have been performed for ballot b of a higher-
     numbered instance.

E 4b If the leader was not forced (by the value of sval 2a) to propose the stopping
     command, then it must not be forced to propose any command in a higher-
     numbered instance.

Condition E 5 asserts that the leader has not performed a Phase2a action for
a stopping command in ballot b of a lower-numbered instance, and condition
E 6 asserts that the value of sval 2a does not force the leader to propose such a
value.
    It appears that progress is impossible if the ballot b leader is forced (by E 3)
to propose a stopping command in an instance i and to propose some command
in another instance j > i . If it executes the Phase2a action for instance i , then
E 5 prevents it from executing the Phase2a action for instance j . If it executes
the action for instance j first, then E 4a prevents it from executing the action
for instance i . This situation is prevented by voiding. The definition of sval 2a
ensures that the Phase1b messages for instance j void the stopping command
in instance i .
    Unlike in ordinary Paxos, in Stoppable Paxos the separate consensus in-
stances are not logically separate. Enabling conditions E 4–E 6 and the definition
of sval 2a for a ballot in one instance depend on Phase2a actions performed and
Phase1b messages received for that ballot in other instances. However, this im-
plies no extra messages or delays. As in ordinary Paxos, the Phase1b messages
for all instances are bundled together; and no enabling condition requires that
a Phase2a action for another instance be done first. The enabling conditions
require only that certain actions not have been done.
    We now precisely define sval 2a and E 1–E 6. For clarity, we write mathe-
matical formulas in mathematics, using English only where necessary to avoid



                                         8
distracting formalization. We let the range over which a variable is quantified,
if not stated explicitly, depend on the variable name as follows:
      i , j , k : instance numbers            b, c : ballot numbers    Q : quorums
      u, v , w : commands                     a, q : acceptors

We use customary abbreviations such as ∃ i < j : P for ∃ i : (i < j ) ∧ P . The
definition of sval 2a is:
                    ∆
sval 2a(i , b, Q) = if              (val 2a(i , b, Q) ∈ StopCmd )
                                 ∧ (∃ j > i : mbal 2a(j , b, Q) ≥ mbal 2a(i , b, Q))
                                then >
                                else val 2a(i , b, Q)

Define Done2a(i , b, v ) to be the state function that is true iff a
Phase2a(i , b, v , Q) action has been executed for some quorum Q. (More pre-
cisely, it is true iff there is a h“2a”, b, v ii message in the set of sent messages.)
The enabling conditions of the Phase2a(i , b, v , Q) action are:
            ∆
E 1(b, Q) = ∀ a ∈ Q, i : the ballot b leader has received a message of the
                         form h“1b”, a, b, rii from a
            ∆
E 2(i , b) = ∀ w : ¬Done2a(i , b, w )
                    ∆
E 3(i , b, Q, v ) = (sval 2a(i , b, Q) 6= >) ⇒ (v = val 2a(i , b, Q))
                    ∆
E 4(i , b, Q, v ) = (v ∈ StopCmd ) ⇒ E 4a(i , b, v ) ∧ E 4b(i , b, Q, v )
    where
                        ∆
    E 4a(i , b, v ) = ∀ j > i , w : ¬Done2a(j , b, w )
                            ∆
    E 4b(i , b, Q, v ) = ∀ j > i : (sval 2a(i , b, Q) = >) ⇒ (sval 2a(j , b, Q) = >)
            ∆
E 5(i , b) = ∀ j < i , w ∈ StopCmd : ¬Done2a(j , b, w )
                ∆
E 6(i , b, Q) = ∀ j < i : (sval 2a(j , b, Q) 6= >) ⇒ (sval 2a(j , b, Q) ∈
                                                                        / StopCmd )

The ballot b leader can propose commands in different instances in any or-
der. This applies to a stopping command as well. In particular, the leader can
propose a stopping command as command number i and then propose lower-
numbered commands. Since stopping commands are used for reconfiguration,
this allows the leader to let i be the next available command number for recon-
figurations that must occur quickly and to be larger for reconfigurations that
may occur lazily.


4     Correctness
We now state the correctness properties satisfied by Stoppable Paxos. A rigor-
ous informal proof of these properties appears in the appendix. We have also


                                                  9
written a formal hand proof that gives us greater confidence in the algorithm’s
correctness than such an informal proof can provide.
    First, we define Chosen(i , b, v ) to assert that command v is chosen in ballot
b of instance i . As in ordinary Paxos, Chosen(i , b, v ) is defined to be true iff
there is a quorum Q such that vote i [a][b] = v holds for all a in Q.
                       ∆
Chosen(i , b, v ) = ∃ Q : ∀ a ∈ Q : vote i [a][b] = v

Our algorithm satisfies the same consistency property as ordinary Paxos plus the
property that a stopping command stops the state machine. These properties
are expressed by the invariance of the following state predicates.
                   ∆
Consistency = ∀ i , b, c, v , w : Chosen(i , b, v ) ∧ Chosen(i , c, w ) ⇒ (b = c)
           ∆
Stopping = ∀ i , j < i , v ∈ StopCmd , w :
                 Chosen(j , b, v ) ⇒ ¬Chosen(i , c, w )
Like ordinary Paxos, our Stoppable Paxos assures progress if eventually there
is a unique leader for a high enough ballot number that is nonfaulty and can
communicate with a nonfaulty quorum. The precise property we prove is that
the following condition holds, for all b and Q.
                       ∆
Progress(b, Q) =
  P 1(b, Q) ∧ P 2(b, Q) ∧ P 3(b) ⇒
          eventually     (∃ v : Chosen(i , b, v ))
                      ∨ (∃ j < i , v ∈ StopCmd : Chosen(j , b, v ))
  where
                       ∆
    P 1(b, Q) = No ballot b action of the ballot b leader or of an acceptor
                in Q can become forever enabled and never executed.
                       ∆
    P 2(b, Q) = Every ballot b message sent between the ballot b leader and
                the acceptors in Q is eventually received.
               ∆
    P 3(b) = ∀ c > b : No P hase1a(c) action is ever executed.

Condition P 1(b, Q) means that the ballot b leader eventually executes the
Phase1a(b) action, and that it and the acceptors in Q perform ballot b actions
that are enabled by the receipt of messages. Condition P 2(b, Q) is satisfied if
eventually the leader and the acceptors in Q are nonfaulty and communicate
reliably with one another, using a retransmission protocol to recover from lost
messages. Condition P 3(b) asserts that no ballot numbered greater than b is
ever started.
    Conditions P 1–P 3 are the same ones under which ordinary Paxos achieves
progress. As with ordinary Paxos, they are satisfied in practice by using a
leader-selection algorithm. However, because of the extra enabling conditions
in the Phase2a action, the proof that they ensure progress is more difficult.




                                         10
References
 [1] Y. Amir, L. E. Moser, P. M. Melliar-Smith, D. A. Agarwal, and P. Ciarfella.
     The Totem single-ring ordering and membership protocol. tocs, 13(4):311–
     342, November 1995.

 [2] Özalp Babaoglu, Renzo Davoli, and Alberto Montresor. Group communi-
     cation in partitionable systems: Specification and algorithms. IEEE Trans-
     actions on Software Engineering, 27(4):308–336, 2001.

 [3] Kenneth P. Birman and Thomas A. Joseph. Reliable communication in the
     presence of failures. ACM Transactions on Computer Systems, 5(1):47–76,
     February 1987.

 [4] Bernadette Charron-Bost and André Schiper. Uniform consensus is harder
     than consensus (extended abstract). Technical Report DSC/2000/028,
     École Polytechnique Fédérale de Lausanne, Switzerland, May 2000.

 [5] Gregory V. Chockler, Idit Keidar, and Roman Vitenberg. Group commu-
     nication specifications: A comprehensive study. ACM Computing Surveys,
     33(4):427–469, December 2001.

 [6] Danny Dolev and Dalia Malki. The Transis approach to high availability
     cluster communication. Communications of the ACM, 39(4):64–70, April
     1996.

 [7] Alan Fekete, Nancy Lynch, and Alex Shvartsman. Specifying and using a
     partitionable group communication service. ACM Transactions on Com-
     puter Systems, 19(2):171–216, May 2001.

 [8] Idit Keidar and Danny Dolev. Efficient message ordering in dynamic net-
     works. In Proceedings of the Fifteenth Annual ACM Symposium on Prin-
     ciples of Distributed Computing, New York, NY, May 1996. ACM.

 [9] Leslie Lamport. How to write a proof. American Mathematical Monthly,
     102(7):600–608, August-September 1995.

[10] Leslie Lamport. The part-time parliament. ACM Transactions on Com-
     puter Systems, 16(2):133–169, May 1998.

[11] Butler W. Lampson. How to build a highly available system using con-
     sensus. In Ozalp Babaoglu and Keith Marzullo, editors, Distributed Al-
     gorithms, volume 1151 of Lecture Notes in Computer Science, pages 1–17,
     Berlin, 1996. Springer-Verlag.

[12] L. E. Moser, Y. Amir, P. M. Melliar-Smith, and D. A. Agarwal. Extended
     virtual synchrony. In The 14th IEEE International Conference on Distrib-
     uted Computing Systems (ICDCS), pages 56–65, 1994.


                                      11
[13] Amir Pnueli. The temporal logic of programs. In Proceedings of the 18th
     Annual Symposium on the Foundations of Computer Science, pages 46–57.
     IEEE, November 1977.

[14] Fred B. Schneider. Implementing fault-tolerant services using the state
     machine approach: A tutorial. ACM Computing Surveys, 22(4):299–319,
     December 1990.

[15] J. Wensley et al. SIFT: Design and analysis of a fault-tolerant computer
     for aircraft control. Proceedings of the IEEE, 66(10):1240–1254, October
     1978.




                                     12
Appendix: The Proof of Correctness
We now prove that Stoppable Paxos satisfies its safety and liveness properties.
For clarity and conciseness, we write simple temporal logic formulas with two
temporal operators: 2 meaning always, and 3 meaning eventually [13]. We use
                                                     ∆
a linear-time logic, so 3 can be defined by 3F = ¬2¬F , for any formula F .
For a state predicate P , the formula 2P asserts that P is an invariant, meaning
that it is true for every reachable state. The temporal formula 32P asserts
that at some point in the execution, P holds from that point onward.
    We define a predicate P to be stable iff it satisfies the following condition:
if P is true in any reachable state s, then P is true in any state reachable from
s by any action of the algorithm. We let stable P be the assertion that state
predicate P is stable. It is clear that a stable predicate is invariant if it is true
in the initial state. Because stability is an assertion only about reachable states
s, we can assume that all invariants of the algorithm are true in state s when
proving stability.
    Our proofs are informal, but careful. The two complicated, multi-page proofs
are written with a hierarchical numbering scheme in which hx iy is the number
of the y th step of the current level-x proof [9]. Although it may appear intimi-
dating, this kind of proof is easy to check and helps to avoid errors.

A.1 The Proof of Safety
We now prove that Consistency and Stopping are invariants of Stoppable Paxos.
First, we define:
                          ∆
NotChoosable(i , b, v ) =
      ( ∃ Q : ∀ a ∈ Q : (bal [a] > b) ∧ (vote i [a][b] 6= v ) )
    ∨ ( ∃ j < i , w ∈ StopCmd : Done2a(j , b, w ) )
    ∨ ( (v ∈ StopCmd ) ∧ (∃ j > i , w : Done2a(j , b, w )) )
We next prove a number of simple invariance and stability properties of the
algorithm.

Lemma 1
  1. ∀ i , b, v : 2 (Chosen(i , b, v ) ⇒ Done2a(i , b, v )).
  2. ∀ i , b, v , w : 2 ((Done2a(i , b, v ) ∧ Done2a(i , b, w ) ⇒ (v = w ))
  3. ∀i , b, a, v : 2 ((vote i [a][b] = v ) ⇒ Done2a(i , b, v ))
  4. ∀i , b, v , a, q : 2 ((vote i [a][b] = v ) ⇒ (vote i [q][b] ∈ {v , >}))
  5. (a) ∀ i , a, b, v : stable ((bal [a] > b) ∧ (vote i [a][b] = v ))
     (b) ∀ i , a, b : stable ((bal [a] > b) ∧ (vote i [a][b] = >))
  6. ∀ i , j < i , b, w ∈ StopCmd , v :
           2 (Done2a(j , b, w ) ⇒ ¬Done2a(i , b, v ))
  7. ∀ i , b, v : stable NotChoosable(i , b, v )

                                             13
  8. ∀ i , b, Q : 2 (E 1(b, Q) ⇒ (mbal 2a(i , b, Q) < b))
Proof:
  1. Chosen(i , b, v ) implies that vote i [a][b] = v for some acceptor a, which implies a
     received a h“2a”, b, v ii message, which implies Done2a(i , b, v ).
  2. This follows from enabling condition E 2 for the Phase2a action.
  3. vote i [a][b] = v implies that acceptor a must have received the Phase2a message
     sent by executing Phase2a(i , b, v , Q) for some quorum Q.
  4. This follows from Lemmas 1.2 and 1.3.
  5. No action decreases bal [a], and vote i [a][b] is changed to (a command) u only by
     a Phase2b(i , a, b, u) action, which is enabled only if bal [a] ≤ b.
  6. Done2a(j , b, w ) ⇒ ¬Done2a(i , b, v ) is obviously true initially. It is sta-
     ble because enabling condition E 4a(j , b, w ) of Phase2a(j , b, w , Q) implies that
     Done2a(j , b, w ) can become true only when ¬Done2a(i , b, v ) is true, and enabling
     condition E 5(i , b) of Phase2a(i , b, v , Q) implies that ¬Done2a(i , b, v ) can become
     false only when Done2a(j , b, w ) is false.
  7. It suffices to show that each of the disjuncts in the definition of
     NotChoosable(i , b, v ) is stable. The first disjunct is the conjunction of formulas
     (bal [a] > b) ∧ (vote i [a][b] 6= v ), each of which can be written as the conjunction of
     formulas (bal [a] > b) ∧ (vote i [a][b] = w ) (for w a command or >) which are stable
     by part 5 of this lemma. The stability of the second and third conjuncts follows
     easily from the obvious stability of Done2a(j , b, w ) for all j and w .
  8. An acceptor a changes vote i [a][b] only by performing a Phase2b action that sets
     bal [a] to b. Because bal [a] is never decreased, (vote i [a][b] 6= >) ⇒ (bal [a] ≥ b)
     is an invariant. A h“1b”, a, b, hc, v iii message is sent by a Phase1b(a, b) action
     that is enabled only if b > bal [a], so c < b for any such message. The definition
     of mbal 2a then implies that mbal 2a(i , b, Q) < b if some acceptor in Q has sent a
     Phase1b message for ballot b of instance i , which is the case if E 1(b, Q) is true.
We now prove some less obvious invariants.

Lemma 2 ∀ i , b, v : 2 (NotChoosable(i , b, v ) ⇒ ¬Chosen(i , b, v ))
Proof: We assume NotChoosable(i , b, v ) is true in a reachable state (so all invariants
are true) and prove ¬Chosen(i , b, v ). By definition of NotChoosable, there are three
cases to consider.
1. Case: ∃ Q : ∀ a ∈ Q : (bal [a] > b) ∧ (vote i [a][b] 6= v )
   Proof: Since any two quorums have non-empty intersection, any quorum contains
   an acceptor a in Q, for which the case assumption implies vote i [a][b] 6= v . By
   definition of Chosen, this implies ¬Chosen(i , b, v ).
2. Case: ∃ j < i , w ∈ StopCmd : Done2a(j , b, w )
   Proof: Lemma 1.6 implies ¬Done2a(i , b, v ), and Lemma 1.1 then implies
   ¬Chosen(i , b, v ).
3. Case: ∃ j > i , w : Done2a(j , b, w ).
   Proof: Lemma 1.6 (with i ↔ j and v ↔ w ) implies ¬Done2a(i , b, v ), and
   Lemma 1.1 then implies ¬Chosen(i , b, v ).



                                             14
Lemma 3 ∀ i , b, c < b, w , Q :
              2 ( Hyp(i , b, c, Q) ∧ E 1(b, Q) ⇒ NotChoosable(i , c, w ) )
                                            ∆
                where Hyp(i , b, c, Q) =
                          (mbal 2a(i , b, Q) < c)
                        ∨ ( (mbal 2a(i , b, Q) = c) ∧ (w 6= val 2a(i , b, Q)) )
Proof: We assume c               < b, Hyp(i , b, c, Q), and E 1(b, Q) and prove
NotChoosable(i , c, w ). Assumption E 1(b, Q) implies that every acceptor a in Q
has sent a “1b” message for ballot b of instance i . Since Phase1b(a, b) is enabled
only if b > bal [a] and sets bal [a] to b, acceptor a can have sent only one such “1b”
message. Let h“1b”, a, b, hb a , v a iii be that message. We consider the two disjuncts of
the assumption Hyp(i , b, c, Q) separately.
1. Case: mbal 2a(i , b, Q) < c
   Proof: Let a be any acceptor in Q. The case assumption implies b a < c, so
   val i [a][c] equaled > when a executed its Phase1b(a, b) action. That action made
   bal [a] = b true, so c < b and Lemma 1.5 imply val i [a][c] = > is still true. Every
   quorum contains an acceptor a in Q, for which we have shown that val i [a][c] = >,
   so NotChoosable(i , c, w ) is true.
2. Case: mbal 2a(i , b, Q) = c and w 6= val 2a(i , b, Q)
   Proof: Let a be any acceptor in Q. The assumption mbal 2a(i , b, Q) = c implies
   b a ≤ c. The definitions of b a and v a imply that, when a executed its Phase1b(a, b)
   action, the value of vote i [a][c] was v a if b a = c and was > if b a < c. Since the action
   set bal [a] to b and c < b, Lemma 1.5 implies that vote i [a][c] still has that value.
   If b a = c, then Lemma 1.4 and the definition of val 2a imply v a = val 2a(i , b, Q).
   The case assumption w 6= val 2a(i , b, Q) therefore implies that vote i [a][c] 6= w for
   all acceptors a in Q. Since every quorum contains an acceptor in a, this implies
   NotChoosable(i , c, w ).
We now make some more definitions, culminating in the key invariant
PropInv (i , b, v ).
                   ∆
SafeAt(i , b, v ) = ∀ c < b, w 6= v : NotChoosable(i , c, w )
                             ∆
NoReconfigBefore(i , b) =
   ∀ j < i , c ≤ b, w ∈ StopCmd : NotChoosable(j , c, w )
                                   ∆
NoneChoosableAfter (i , b, v ) =
   (v ∈ StopCmd ) ⇒ ∀ j > i , c < b, w : NotChoosable(j , c, w )
                       ∆
PropInv (i , b, v ) = Done2a(i , b, v ) ⇒         SafeAt(i , b, v )
                                                ∧ NoReconfigBefore(i , b)
                                                ∧ NoneChoosableAfter (i , b, v )
The heart of the safety proof is the following proof that PropInv is invariant.
Lemma 4 2 (∀ i , b, v : PropInv (i , b, v ))
Proof: ∀, i , b, v : PropInv (i , b, v ) is true in the initial state because Done2a(. . .) is
initially false. We therefore need only show that it is stable. We do this by assuming
that it is true in a state s and proving it is true in state t. For any state function f we
let f be its value in state s and f 0 be its value in state t.

                                              15
h1i1. It suffices to
        Assume: 1. ∀ j , c, w : PropInv (j , c, w )
                    2. i is an instance number, b a ballot number, v a command, and Q a
                       quorum.
                    3. s → t is a Phase2a(i , b, v , Q) step.
                    4. E 1(b, Q)
        Prove:         SafeAt(i , b, v )0
                    ∧ NoReconfigBefore(i , b)0
                    ∧ NoneChoosableAfter (i , b, v )0
   Proof: To prove (∀, i , b, v : PropInv (i , b, v ))0 , it suffices to prove it for a particular
   i , b, and v . It follows from Lemma 1.7 (the stability of NotChoosable(. . .)) that
        SafeAt(i , b, v ) ∧ NoReconfigBefore(i , b) ∧ NoneChoosableAfter (i , b, v )
  is stable. Hence, the first step that can possibly make PropInv (i , b, v ) false is one that
  makes Done2a(i , b, v ) true. We can therefore assume s → t is a Phase2a(i , b, v , Q)
  step for some quorum Q. Formula E 1(b, Q) holds because it is an enabling condition
  of the Phase2a(i , b, v , Q) action.
The three primed formulas of the “Prove” clause of h1i1 are proved as steps h1i5,
h1i6, and h1i7 below. The next three steps are used in their proofs.

h1i2. ∀ j : (mbal 2a(j , b, Q) 6= −∞) ⇒ Done2a(j , mbal 2a(j , b, Q), val 2a(j , b, Q))
   Proof: Assume mbal 2a(j , b, Q) 6= −∞. By definition of mbal 2a, this implies
   val 2a(j , b, Q) is a command (and not >). Since E 1(b, Q) holds by assump-
   tion h1i1.4, the definitions of mbal 2a and val 2a imply that some acceptor a in
   Q has sent a h“1b”, a, b, hmbal 2a(j , b, Q), val 2a(j , b, Q)iij message, which implies
   vote j [a][mbal 2a(j , b, Q)] = val 2a(j , b, Q) when the message was sent. Lemma 1.3
   then implies Done2a(j , mbal 2a(j , b, Q), val 2a(j , b, Q)) was true when the message
   was sent, and is still true because Done2a(. . .) is stable.
h1i3. ∀ j , c < b, w : (c ≤ mbal 2a(j , b, Q)) ∧ (w 6= val 2a(j , b, Q)) ⇒
                                 NotChoosable(j , c, w )
   Proof: We assume c ≤ mbal 2a(j , b, Q) and w 6= val 2a(j , b, Q) and
   we prove NotChoosable(j , c, w ).           Since −∞ < c ≤ mbal 2a(j , b, Q), step
   h1i2 implies Done2a(j , mbal 2a(j , b, Q), val 2a(j , b, Q)).     By assumption h1i1.1,
   this implies SafeAt(j , mbal 2a(j , b, Q), val 2a(j , b, Q)).     The assumption c ≤
   mbal 2a(j , b, Q), together with assumption h1i1.4 and Lemma 1.8 (which imply
   mbal 2a(j , b, Q) < b), implies c < b. The assumption w 6= val 2a(j , b, Q) and
   SafeAt(j , mbal 2a(j , b, Q), val 2a(j , b, Q)) then imply NotChoosable(j , c, w ).
h1i4. ∀ j , c < b, w : (sval 2a(j , b, Q) = >) ⇒ NotChoosable(j , c, w )
   Proof: We assume c < b and sval 2a(j , b, Q) = > and prove NotChoosable(j , c, w ).
   We split the proof into two cases.
   h2i1. Case: mbal 2a(j , b, Q) = −∞
      Proof: The case assumption implies mbal 2a(j , b, Q) < c, so assumption h1i1.4
      and Lemma 3 imply NotChoosable(j , c, w ).
  h2i2. Case: mbal 2a(j , b, Q) 6= −∞
     Proof: Since c < b, we can split the proof into the following three cases.
     h3i1. Case: mbal 2a(j , b, Q) < c < b
        Proof: By assumption h1i1.4, the case assumption and Lemma 3 imply
        NotChoosable(j , c, w ).


                                               16
     h3i2. Case: c ≤ mbal 2a(j , b, Q) and w 6= val 2a(j , b, Q)
        Proof: By h1i3.
     h3i3. Case: c ≤ mbal 2a(j , b, Q) and w = val 2a(j , b, Q)
        h4i1. val 2a(j , b, Q) ∈ StopCmd and we can choose k > j such that
               mbal 2a(k , b, Q) ≥ mbal 2a(j , b, Q).
           Proof: We deduce that val 2a(j , b, Q) ∈ StopCmd and such a k exists by the
           h2i2 case assumption, the assumption sval 2a(j , b, Q) = >, and the definition
           of sval 2a.
        h4i2. Done2a(k , mbal 2a(k , b, Q), val 2a(k , b, Q))
           Proof: The h3i3 case assumption and h4i1 imply mbal 2a(k , b, Q) 6= −∞.
           Step h1i2 then proves h4i2.
        h4i3. NotChoosable(j , c, w )
           Proof: Assumption h1i1.1 (with j ← k , c ← mbal 2a(k , b, Q), and w ←
           val 2a(k , b, Q)) and h4i2 imply NoReconfigBefore(k , mbal 2a(k , b, Q)). Step
           h4i1 asserts j < k ; case assumption h3i3 and h4i1 imply c ≤ mbal 2a(k , b, Q);
           and h4i1 and case assumption h3i3 imply w ∈ StopCmd .                Therefore,
           NoReconfigBefore(k , mbal 2a(k , b, Q)) implies NotChoosable(j , c, w ).
h1i5. SafeAt(i , b, v )0
Proof: We assume c < b and w 6= v and prove NotChoosable(i , c, w )0 . By Lemma 1.7,
it suffices to prove NotChoosable(i , c, w ). We split the proof into two cases.
   h2i1. Case: sval 2a(i , b, Q) = >
      Proof: h1i4 (substituting j ← i ) implies NotChoosable(i , c, w ).
  h2i2. Case: sval 2a(i , b, Q) 6= >
     Proof: Since c < b, we can break the proof into two sub-cases.
     h3i1. Case: mbal 2a(i , b, Q) < c < b
        Proof: Assumption h1i1.4 and Lemma 3 imply NotChoosable(i , c, w )
     h3i2. Case: c ≤ mbal 2a(i , b, Q)
        Proof: Assumption h1i1.3 implies E 3(i , b, Q, v ). Case assumption h2i2 and
        E 3(i , b, Q, v ) imply v = sval 2a(i , b, Q). Case assumption h2i2 and the defini-
        tion of sval 2a then imply v = val 2a(i , b, Q). Case assumption h3i2, the assump-
        tion w 6= v , and step h1i3 (substituting j ← i ) then imply NotChoosable(i , c, w ).
h1i6. NoReconfigBefore(i , b)0
Proof: We assume j < i , w ∈ StopCmd , and c ≤ b and we prove
NotChoosable(j , c, w )0 . By Lemma 1.7, it suffices to prove NotChoosable(j , c, w ). Since
c ≤ b, we need consider only the following two cases.
   h2i1. Case: b = c
      Proof: Assumption h1i1.3 implies Done2a(i , b, v )0 .           Since i > j and
      w ∈ StopCmd , this implies the third disjunct of NotChoosable(j , b, w )0 (substitut-
      ing i and v for the existentially quantified variables), which by the case assumption
      proves NotChoosable(j , c, w )0 .
  h2i2. Case: c < b
     Proof: We consider two sub-cases.
     h3i1. Case: sval 2a(j , b, Q) = >
        Proof: h1i4 and case assumption h2i2 imply NotChoosable(j , c, w ).
     h3i2. Case: sval 2a(j , b, Q) 6= >
        Proof: By case assumption h2i2, we have the following two sub-cases.
        h4i1. Case: mval 2a(j , b, Q) < c < b


                                            17
          Proof: Assumption h1i1.4, the case assumption, and Lemma 3 imply
          NotChoosable(j , c, w ).
       h4i2. Case: c ≤ mval 2a(j , b, Q)
          Proof: Assumption h1i1.3 implies E 6(i , b, Q). The h3i2 case assumption,
          the assumption j < i , and E 6(i , b, Q) imply sval 2a(j , b, Q) ∈
                                                                           / StopCmd . The
          assumption w ∈ StopCmd then implies w 6= sval 2a(j , b, Q). By the h3i2 case
          assumption and the definition of sval 2a, we then have w 6= val 2a(j , b, Q).
          The h4i2 case assumption (which implies mval 2a(j , b, Q) 6= −∞) and h1i3
          then imply NotChoosable(j , c, w ).
h1i7. NoneChoosableAfter (i , b, v )0
Proof: We assume v ∈ StopCmd , j > i , c < b, and w any command and we prove
NotChoosable(j , c, w )0 . By Lemma 1.7, it suffices to prove NotChoosable(j , c, w ). We
split the proof into two cases.
   h2i1. Case: sval 2a(i , b, Q) = >
      Proof: Assumption h1i1.3 implies E 4(i , b, Q, v ), so the assumption v ∈ StopCmd
      implies E 4b(i , b, Q, v ). The case assumption, the assumption j > i , and
      E 4b(i , b, Q, v ) imply sval 2a(j , b, Q) = >. The assumption c < b and step h1i4
      then imply NotChoosable(j , c, w ).
  h2i2. Case: sval 2a(i , b, Q) 6= >
     h3i1. sval 2a(i , b, Q) = val 2a(i , b, Q) = v
        Proof: Assumption h1i1.3 implies E 3(i , b, Q, v ),                 which implies
        sval 2a(i , b, Q) = v . The case assumption and the definition of sval 2a
        then implies val 2a(i , b, Q) = v .
     h3i2. Done2a(i , mbal 2a(i , b, Q), v )
        Proof: h3i1, assumption h1i1.4, and the definition of val 2a imply
        vote i [a][mbal 2a(i , b, Q)] = v for some acceptor a in Q, which by Lemma 1.3
        implies Done2a(i , mbal 2a(i , b, Q), v ).
     By the assumption c < b, it suffices to consider the following two cases.
     h3i3. Case: c < mbal 2a(i , b, Q)
        Proof:           Step        h3i2      and       assumption       h1i1.1       imply
        NoneChoosableAfter (i , mbal 2a(i , b, Q), v ).    By the case assumption and
        the assumptions v ∈ StopCmd and j > i , this implies NotChoosable(j , c, w ).
     h3i4. Case: mbal 2a(i , b, Q) ≤ c < b
        h4i1. mbal 2a(j , b, Q) < mbal 2a(i , b, Q)
           Proof:         The      assumption       v ∈ StopCmd      and       h3i1    imply
           sval 2a(i , b, Q) ∈ StopCmd .      Case assumption h2i2 and the definition
           of sval 2a then imply mbal 2a(k , b, Q) < mbal 2a(i , b, Q) for all k > i .
        h4i2. NotChoosable(j , c, w )
           Proof: h4i1 and case assumption h3i4 imply mbal 2a(j , b, Q) < c < b. By
           assumption h1i1.4, Lemma 3 implies NotChoosable(j , c, w ).

Theorem 1 2 Consistency
Proof: By definition of Consistency, it suffices to assume Chosen(i , b, v ) and
Chosen(i , c, w ) and to prove v = w . Without loss of generality, we can assume b ≤ c.
We then have two cases.
1. Case: b < c
   Proof: We assume v 6= w and obtain a contradiction.                 Lemma 1.1 and
   Chosen(i , c, w ) imply Done2a(i , c, w ). By Lemma 4, this implies SafeAt(i , c, w ).


                                            18
   The assumptions b < c, an v 6= w then imply NotChoosable(i , b, v ). By Lemma 2,
   this contradicts the assumption Chosen(i , b, v ).
2. Case: b = c
   Proof: Lemma 1.1 implies Done2a(i , b, v ) ∧ Done2a(i , c, w ), which by Lemma 1.2
   implies b = c.

Theorem 2 2 Stopping
Proof: By definition of Stopping, it suffices to assume Chosen(i , b, v ), Chosen(j , c, w ),
v ∈ StopCmd , and j > i and to obtain a contradiction. We split the proof into two
cases.
1. Case: c < b
   Proof: Chosen(i , b, v ) and Lemma 1.1 imply Done2a(i , b, v ). This and Lemma 4
   imply NoneChoosableAfter (i , b, v ), which by the case assumption and the assump-
   tions v ∈ StopCmd and j > i implies NotChoosable(j , c, w ). The assumption
   Chosen(j , c, w ) and Lemma 2 then provide the required contradiction.
2. Case: c ≥ b
   Proof: Chosen(j , c, w ) and Lemma 1.1 imply Done2a(j , c, w ). Lemma 4 then im-
   plies NoReconfigBefore(j , c). The case assumption, the assumptions v ∈ StopCmd
   and j > i , and NoReconfigBefore(j , c) imply NotChoosable(i , b, v ). The assumption
   Chosen(i , b, v ) and Lemma 2 then provide the required contradiction.

A.2 The Proof of Progress.
Theorem 3 ∀ b, Q : Progress(b, Q)
Proof: We assume P 1(b, Q), P 2(b, Q) and P 3(b) and we must prove that there exists
a v such that either 3Chosen(i , b, v ) or (v ∈ StopCmd ) ∧ 3Chosen(j , b, v ), for some
j < i.
h1i1. 32E 1(b, Q)
   Proof: P 1(b, Q) implies that the ballot b leader eventually executes a Phase1a(b)
   action. By P 2(b, Q), every acceptor a in Q eventually receives the Phase1a messages.
   Because bal [a] is set to a value c only by receiving a ballot c message, assumption
   P 3(b) implies bal [a] ≤ b. Hence, a must eventually receive the Phase1a message
   and execute Phase1b(a, b). By P 2(b, Q), the Phase1b message it sends is eventually
   received by the leader.
h1i2. ∀ i , w : 2(Done2a(i , b, w ) ⇒ 3Chosen(i , b, w ))
   Proof: Done2a(i , b, w ) means that a Phase2a(i , b, w ) action has been executed
   sending a h“2a”, b, w ii message to every acceptor a. If a is in Q, then assump-
   tion P 2(b, Q) implies that it eventually receives that message. Assumption P 3(b)
   implies bal [a] ≤ b, so P 1(b, Q) implies that every a in Q eventually executes
   Phase2b(i , a, b, w ), setting vote i [a][b] to w . Hence, eventually Chosen(i , b, w ) be-
   comes true.
Since ¬3F is equivalent to 2¬F , for any formula F , we can split the proof into the
following two cases.
h1i3. Case: ∃ k > i , w : 3Done2a(k , b, w )
   h2i1. 2 E 5(i , b)
      Proof: By definition of E 5(i , b), it suffices to assume j < i , v ∈ StopCmd , and
      3Done2a(j , b, v ) and obtain a contradiction. By the h1i3 case assumption, we


                                             19
     have 3Done2a(k , b, w ) for k > i > j . Since k 6= j , the following two cases are
     exhaustive.
     h3i1. Case: Phase2a(k , b, w ) is executed after Phase2a(j , b, v )
        Proof: This is impossible because the enabling condition E 5(k , b) of
        Phase2a(k , b, w ) implies ¬Done2a(j , b, v ).
     h3i2. Case: Phase2a(j , b, v ) is executed after Phase2a(k , b, w )
        Proof: This is impossible because E 4a(j , b, v ), which by the assump-
        tion v ∈ StopCmd is an enabling condition of Phase2a(j , b, v ), implies
        ¬Done2a(k , b, w ).
  h2i2. Pick a quorum U such that 32(E 1(b, U ) ∧ E 6(i , b, U ))
     Proof: Case assumption h1i3 implies that we can choose U such that
     3(E 1(b, U ) ∧ E 6(k , b, U )). By definition of sval 2a, we have E 1(b, U ) implies
     E 6(k , b, U ) is stable. Since E 1(b, U ) is obviously stable, 3(E 1(b, U )∧E 6(k , b, U ))
     implies 32 (E 1(b, U ) ∧ E 6(k , b, U )). The assumption k > i and the definition of
     E 6 imply 2 (E 6(k , b, U ) ⇒ E 6(i , b, U )).
  h2i3. Pick w ∈   / StopCmd such that 32E 3(i , b, U , w )
     Proof: By h2i2, we can choose a point in the execution at which 2(E 1(b, U ) ∧
     E 6(k , b, U )) holds. By 2E 1(b, U ), the value of sval 2a(i , b, U ) remains constant
     from that point on. If sval 2a(i , b, U ) = >, let w be any command not in StopCmd .
     Otherwise, let w = val 2a(i , b, U ), which by E 6(k , b, U ) and the assumption k > i
     is not in StopCmd .
  h2i4. 3Chosen(i , b, w )
     Proof: The theorem is proved if 3Chosen(i , b, u, V ) for any u and V . Hence,
     by h1i2 it suffices to assume 2 ∀u : ¬Done2a(i , b, u), which is 2 E 2(i , b).
     We have proved 2 E 5(i , b) (h2i1), 32 E 3(i , b, U , w ) (h2i3), and 32(E 1(b, U ) ∧
     E 6(i , b, U )) (h2i2). By h2i3 (w ∈
                                        / StopCmd ), E 4(i , b, U , w ) holds trivially. Hence,
     the Phase2a(i , b, w , U ) action is eventually always enabled, so by assumption
     P 1(b, Q) it is eventually executed by the ballot b leader. Step h1i2 then implies
     3Chosen(i , b, w ).
h1i4. Case: ∀ k > i , w : 2¬Done2a(k , b, w )
   h2i1. ∃ j , v : 3Done2a(j , b, v )
      Proof: We assume ∀ j , v : 2¬Done2a(j , b, v ) and prove that eventually a
      Phase2a(1, b, v ) step occurs for some v . (Recall that 1 is the lowest instance
      number.)
      h3i1. 2 E 2(1, b)
         Proof: By the assumption ∀ j , v : 2¬Done2a(j , b, v ).
      h3i2. 2 (E 5(1, b) ∧ E 6(1, b, Q))
         Proof: Conditions E 5 and E 6 are vacuously true for instance 1.
      h3i3. ∃ v : 32 (E 3(1, b, Q, v ) ∧ E 4(1, b, Q, v ))
         Proof: h1i1 implies either (a) 32(sval 2a(1, b, Q)                        =    >) or
         (b) 32(sval 2a(1, b, Q) = v ) for some command v . In case (a), E 3(1, b, Q, v )
         and E 4(1, b, Q, v ) are trivially satisfied for any command v not in StopCmd . In
         case (b), let v = sval 2a(1, b, Q), so E 3(1, b, Q, v ) is satisfied. If v ∈
                                                                                    / StopCmd ,
         then E 4(1, b, Q, v ) is trivially satisfied. If v ∈ StopCmd , then E 4a(i , b, v ) is
         satisfied by the assumption ∀ j , v : 2¬Done2a(j , b, v ) and E 4b(i , b, v , Q) is
         trivially satisfied.
      h3i4. ∃ v : 3Done2a(1, b, v )


                                              20
     Proof: h1i1, h3i1, h3i2, and h3i3 imply that the Phase2a(1, b, v , Q) action is
     eventually always enabled. By P 1(b, Q), this action must eventually be exe-
     cuted.
h2i2. It suffices to:
       Assume: 1. h an instance number, v h a command not in StopCmd , and
                     3Done2a(h, b, v h )
                  2. ∀ j > h, v : 2¬Done2a(j , b, v )
       Prove: ∃v : 3Done2a(h + 1, b, v )
   Proof: h2i1 and case assumption h1i4 implies that there is a largest instance
   number h and a command v h such that 3Done2a(h, b, v h ), and that h ≤ i . If
   v h ∈ StopCmd , then h1i2 implies 3Chosen(h, b, v h ), and h ≤ i then implies we are
   done. Therefore, it suffices to assume v h ∈
                                              / StopCmd and obtain a contradiction,
   which we do by proving that the assumptions imply the Prove clause.
h2i3. 32 E 5(h + 1, b)
   Proof: Assumption h2i2.1 asserts 3Done2a(h, b, v h ), which implies 3E 5(h, b).
   Since Done2a(h, b, v h ) implies ∀ j < h, w ∈ StopCmd : ¬E 4a(j , b, w ), it implies
   that Phase2a(j , b, w , U ) is not enabled for any j < i , w ∈ StopCmd , and quorum
   U , which implies that E 5(h, b) is stable, proving 32 E 5(h, b). Assumption h2i2.1
   and Lemma 1.2 imply ∀v ∈ StopCmd : 2¬Done2a(h, b, v ), which together with
   32 E 5(h, b) implies 32 E 5(h + 1, b).
h2i4. Choose a quorum U such that Phase2a(h, b, v h , U ) is eventually executed.
   Proof: U exists by assumption h2i2.1.
h2i5. 32 E 1(b, U )
   Proof: By h2i4 and the stability of E 1(b, U ).
h2i6. 32 E 6(h + 1, b, U )
   Proof: h2i4, h2i5, and the enabling condition E 6(h, b, U ) imply
  (∗) ∀ j < h : 32((sval 2a(j , b, U ) 6= >) ⇒ (sval 2a(j , b, U ) ∈
                                                                   / StopCmd ))
  Step h2i4, assumption h2i2.1 (which implies v h ∈    / StopCmd ), and enabling condi-
  tion E 3(h, b, U , v h ) imply 32(sval 2a(h, b, U ) ∈
                                                      / StopCmd ). This and (∗) imply
  32 E 6(h + 1, b, U ).
h2i7. ∃ v : 32(E 3(h + 1, b, U , v ) ∧ E 4(h + 1, b, U , v ))
   Proof: h2i5 implies that it suffices to consider the following two cases.
   h3i1. Case: 32(sval 2a(h + 1, b, U ) = v ) for some command v .
      Proof: The case assumption implies 32 E 3(h + 1, b, U , v ).            Assump-
      tion h2i2.2 implies 2 E 4a(h, b, v ), and the case assumption trivially implies
      32 E 4b(h, b, U , v ).
   h3i2. Case: 32 (sval 2a(h + 1, b, U ) = >)
      Proof: The case assumption implies 32(E 3(h + 1, b, U , v ) ∧ E 4(h + 1, b, U , v ))
      for any command v not in StopCmd .
h2i8. 2 E 2(h + 1, b)
   Proof: Assumption h2i2.2.
h2i9. ∃ v : 3Done2a(h + 1, b, v )
   Proof: h2i3, h2i5, h2i6, h2i7, and h2i8 show that the Phase2a(h +1, b, v , U ) action
   is eventually always enabled for some v . Assumption P 1(b, Q) implies that the
   ballot b leader eventually executes this action. By h2i2, this completes the proof



                                          21
of h1i4.




           22
