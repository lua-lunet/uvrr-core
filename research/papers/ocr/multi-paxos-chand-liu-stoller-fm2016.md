arXiv:1606.01387v4 [cs.DC] 11 Nov 2019

# Formal Verification of Multi-Paxos for Distributed Consensus

SAKSHAM CHAND, Stony Brook University

YANHONG A. LIU, Stony Brook University

SCOTT D. STOLLER, Stony Brook University

Paxos is an important algorithm for a set of distributed processes to agree on a single value or a sequence of values, for which it is called Basic Paxos or Multi-Paxos, respectively. Consensus is critical when distributed services are replicated for fault-tolerance, because non-faulty replicas must agree on the state of the system or the sequence of operations that have been performed. Unfortunately, consensus algorithms including Multi-Paxos in particular are well-known to be difficult to understand, and their accurate specifications and correctness proofs remain challenging, despite extensive studies ever since Lamport introduced Paxos.

This article describes formal specification and verification of Lamport's Multi-Paxos algorithm for distributed consensus. The specification is written in TLA+, Lamport's Temporal Logic of Actions. The proof is written and automatically checked using TLAPS, the TLA+ Proof System. The proof is for the safety property of the algorithm. Building on Lamport, Merz, and Doligez's specification and proof for Basic Paxos, we aim to facilitate the understanding of Multi-Paxos and its proof by minimizing the difference from those for Basic Paxos, and to demonstrate a general way of proving other variants of Paxos and other sophisticated distributed algorithms. We also discuss our general strategies and results for proving complex invariants using invariance lemmas and increments, for proving properties about sets and tuples to help the proof check succeed in significantly reduced time, and for overall proof improvement leading to considerably reduced proof size.

CCS Concepts: • Theory of computation → Logic and verification; Distributed algorithms;

## 1 INTRODUCTION

Distributed consensus is a fundamental problem in distributed systems, which are increasingly important in today's interconnected world. Distributed consensus requires that a set of processes agree on some values proposed by some processes. It is essential when distributed services are replicated for fault-tolerance, because non-faulty replicas must agree. Examples include replicated data storage services like Google File System [20], Apache ZooKeeper [23], Amazon DynamoDB [24], etc. Unfortunately, consensus is difficult when processes or communication channels may fail.

Paxos [35] is an important algorithm, developed by Lamport, for solving distributed consensus. Basic Paxos is for agreeing on a single value, such as whether to commit a database transaction. Multi-Paxos is for agreeing on a continuing sequence of values, for example, a stream of commands to execute. Multi-Paxos has been used in many important distributed services, for example, Google's Chubby [4, 8] and Microsoft's Autopilot [25]. There are other Paxos variants, for example, variants that reduce a message delay [38] or add preemption [36], but Multi-Paxos is the most important in making Paxos practical for distributed services that must execute a continuing sequence of operations.

Paxos handles processes that run concurrently without shared memory, where processes may crash and may later recover, and messages may be lost, delayed, reordered, or duplicated. In Basic Paxos, each process may repeatedly propose some value, and wait for appropriate replies from appropriate subsets of the processes while also replying appropriately to other processes; consensus is reached if eventually enough processes and channels are non-faulty to vote on some proposal thus agreeing on the proposed value. In Multi-Paxos, many more different attempts, proposals, and replies may happen in overlapping fashions to reach consensus on values in different slots in the continuing sequence.

---

Paxos has often been difficult to understand since it was first introduced [35]. Lamport later wrote a simpler description of the phases of the algorithm but only for Basic Paxos [36]. Lamport et al. [41] wrote a formal specification of Basic Paxos in TLA+, Lamport's Temporal Logic of Actions [37], and a proof of the safety property of the algorithm in TLAPS, the TLA+ Proof System [5]. Many efforts, especially in recent years, have been spent on formal specification and verification of Multi-Paxos, but they use more restricted or less direct language models, some with reformulated algorithms, and some mixed in large systems, as discussed in Section 7. What is lacking is formal specification and proof of the exact phases of Multi-Paxos, in a most direct way in a general language like TLA+ [37], with a complete proof that is mechanically checked, and a general method for doing such specifications and proofs in a more feasible way.

This article addresses this challenge. We describe a formal specification of Multi-Paxos written in TLA+, and a complete proof written and automatically checked using TLAPS. Building on Lamport et al.'s specification and proof for Basic Paxos, we aim to facilitate the understanding of Multi-Paxos and its proof by minimizing the difference from those for Basic Paxos. The key change in the specification is to replace operations involving two numbers with those involving a set of 3-tuples, for each process, exactly capturing the minimum conceptual difference between Basic Paxos and Multi-Paxos. However, the proof becomes significantly more difficult because of the handling of sets and tuples in place of two numbers.

This work also aims to show the minimum-change approach as a general way of specifying and verifying other variants of Paxos, and more generally of specifying and verifying other sophisticated algorithms by starting from the basics. We demonstrate this by further showing the extension of the specification and proof of Multi-Paxos to Multi-Paxos with Preemption—letting processes abandon proposals that are already preempted by other proposals [36, 59]. We also extended the specification and proof of Basic Paxos with Preemption, which is even easier. Our specifications and proofs can be found online at github.com/DistAlgo/proofs.

Additionally, we discuss a general method that we followed to prove complex invariants using invariance lemmas and increments, to tackle tedious and difficult proof obligations involving sets and tuples, instead of just scalars in the proof of Basic Paxos, and to perform overall proof improvement leading to considerably reduced proof size. For difficult properties involving unbounded sets, we use induction and direct the prover to focus on the increments to the set. For properties involving tuples, we change the ways of accessing and testing the elements to yield significantly reduced proof-checking time. Overall, we were able to keep the specification minimally changed for adding slots for Multi-Paxos and adding preemption, and keep the proof-checking time to about 3 minutes for both specifications while the prover checks the proofs for over 750 obligations for Multi-Paxos and over 800 obligations for Multi-Paxos with Preemption.

This article is a corrected, improved, and extended version of [7]. The main changes are as follows.

(1) The claim of a complete proof in [7] was incorrect, and the problem discovered is now fixed. The problem was due to an undocumented bug [49] in TLAPS that we discovered after the work in [7], which made us realize that the proof of Multi-Paxos with Preemption was incomplete. We added the missing proof. This is described in the new Section 5.1.
(2) The proofs are improved throughout, shortened by 19% for Multi-Paxos and 17% for Multi-Paxos with Preemption, even with about 4-6% added proof to overcome the TLAPS bug. In fact, it was during improvement of the proof that we discovered the bug. The improvements are described in the new Section 5.2. An overall proof summary is given in the new Section 5.3.

2

---

(3) Sections 4.1, 4.3, and 4.4 are extended to define and explain all auxiliary predicates, process invariants, and message invariants, respectively, that are used in the proofs. Sections 4.5 to 4.7 on proof strategies are newly structured, and Section 4.6 is extensively revised and simplified. The new Section 4.8 discusses specifying and proving a validity condition.
(4) Section 2 is extended with the new Sections 2.3 and 2.4 on TLA \( ^{+} \) and TLAPS, respectively. Section 3 is extended with the new Section 3.3 on fault model and implementation issues. Section 6 is extended with detailed results about the different versions of proofs. Section 7 is expanded with additional related works and details about other proofs.
(5) The complete resulting \(\mathrm{TLA^{+}}\) specification and TLAPS-checked proof of Multi-Paxos with Preemption are given in the new Appendices A to C, including about 1.5 pages of specification, 1 page of invariants, and 8.5 pages of proof, including comments.

The rest of the article is organized as follows. Section 2 covers preliminaries: distributed consensus, Paxos, \(\mathrm{TLA^{+}}\), and TLAPS. Section 3 presents the \(\mathrm{TLA^{+}}\) specification of Multi-Paxos and compares it with Lamport et al.'s specification of Basic Paxos [41]. Section 4 presents the invariants used and proved, and our general proof strategies. Section 5 describes the changes made for specifying and verifying Multi-Paxos with Preemption and the overall proof improvements. Section 6 summarizes the results from our specifications and proofs. Section 7 discusses related work and concludes. Appendices A to C contain the complete resulting specification, invariants, and proof for Multi-Paxos with Preemption.

## 2 PRELIMINARIES

### 2.1 Distributed consensus

A distributed system is a set of processes that process data locally and communicate with each other by sending and receiving messages. The processes may crash and may later recover, and the messages may be lost, delayed, recorded, and duplicated.

The basic consensus problem, called single-value consensus, is for a set of processes to agree on a single value. An algorithm for single-value consensus is said to be safe if it satisfies the following conditions [36]:

C1. Only a value that has been proposed may be chosen,
C2. Only a single value is chosen, and
C3. A process never learns that a value has been chosen unless it actually has been.

Conditions C1 and C3, called validity condition and learner condition, respectively, are straightforward and easy to prove. We do not include them in the formal proof, following Lamport et al. [41]. For completeness, we discuss C3 in Section 3.3 and C1 in Section 4.8.

The specifications and proofs presented in this article focus on C2 because it is the central and really challenging of the three conditions. C2 is formally defined as

\[
\text { Safe } _ {\text { basic }} \triangleq \forall v 1, v 2 \in \mathcal {V}: \text { Chosen } (v 1) \land \text { Chosen } (v 2) \Rightarrow v 1 = v 2 \tag {1}
\]

where V is the set of possible proposed values, and Chosen is a predicate that given a value v evaluates to true iff v was chosen by the algorithm. The specification of Chosen is part of the algorithm.

The more general consensus problem, called multi-value consensus, is to agree on a sequence of values, instead of a single value. Here we have

\[
\text { Safe } \triangleq \forall v 1, v 2 \in \mathcal {V}, s \in \mathcal {S}: \text { Chosen } (s, v 1) \land \text { Chosen } (s, v 2) \Rightarrow v 1 = v 2 \tag {2}
\]

where V is as above, S is a set of slots used to index the sequence of chosen values, and  \( \text{Chosen}(s, v) \)  is true iff for slot s, value v was chosen by the algorithm.

3

---

### 2.2 Basic Paxos and Multi-Paxos

Paxos solves the problem of consensus. Three main roles of the algorithm are performed by three kinds of processes:

- \(\mathcal{P}\), the set of proposers that propose values that can be chosen.
- \(\mathcal{A}\), the set of acceptors that vote for proposed values. A value is chosen when there are enough votes for it.
- \(\mathcal{L}\), the set of learners that learn chosen values. A learner learns a value when it receives enough votes for it.

These roles can be co-located, that is, a single process can take on more than one role.

A set \(Q\) of subsets of the acceptors, that is, \(Q \subseteq 2^{\mathcal{A}}\), is used as a quorum system. It must satisfy the property that any two quorums in \(Q\) overlap, that is, \(\forall Q1, Q2 \in Q: Q1 \cap Q2 \neq \emptyset\). The most commonly used quorum system \(Q\) takes any majority of acceptors as an element in \(Q\).

Basic Paxos solves the problem of single-value consensus. It defines predicate Chosen as

\[
\text { Chosen } (v) \triangleq \exists Q \in Q: \forall a \in Q: \exists b \in \mathcal {B}: \text { sent } (" 2 b", a, b, v) \tag {3}
\]

where \(\mathcal{B}\) is the set of proposal numbers, also called ballot numbers, which is any set that can be totally ordered. sent("2b", \(a, b, v\)) means that a message of type 2b with ballot number \(b\) and value \(v\) was sent by acceptor \(a\). An acceptor votes (for value \(v\)) by sending such a message.

Multi-Paxos solves the problem of multi-value consensus. It extends predicate Chosen to decide a value for each slot s in S:

\[
\text { Chosen } (s, v) \triangleq \exists Q \in Q: \forall a \in Q: \exists b \in \mathcal {B}: \text { sent } (" 2 b", a, b, s, v) \tag {4}
\]

To satisfy the Safe property, \( S \) can be any set. In practice, \( S \) is usually the set of natural numbers.

Multi-Paxos can be built from Basic Paxos by carefully adding slots as detailed in Section 3. However, in order to present these modifications, we need to first describe Basic Paxos. To this end, we present Lamport's description of Basic Paxos [36] in Fig. 1. It uses any majority of acceptors as a quorum. Following Lamport et al. [41], in the specifications presented in this article, the prepare requests and responses have been renamed to 1a and 1b messages, respectively, the accept requests and responses have been renamed to 2a and 2b messages, respectively, and the number n is renamed to b and bal. Modifications to be made to build Multi-Paxos are as follows:

(1) Phase 1a is essentially unchanged.
(2) In Phase 1b, the acceptors now respond with a set of triples in \(\mathcal{B} \times \mathcal{S} \times \mathcal{V}\) as opposed to just one ballot in \(\mathcal{B}\) and one value in \(\mathcal{V}\).
(3) In Phase 2a, the proposers now propose a set of pairs in \( S \times V \) instead of just one value in \( V \). Similar to Basic Paxos, a proposer executes Phase 2a once it has received a set of responses for its 1a message from a quorum of acceptors and picks the value with highest ballot number. But this is now performed separately for each slot in the set of triples received in the responses.
(4) In Phase 2b, the acceptors now respond with a set of pairs in \( S \times V \) as opposed to just one value in \( V \).

### 2.3 TLA \( ^{+} \)

The specifications presented in this article are written in the language TLA \( ^{+} \) [37, 47, 48], which is based on the Temporal Logic of Actions (TLA) [34], a logic for specifying concurrent and distributed systems and reasoning about their properties. In TLA, a state is an assignment of values to the variables of the specification. An action is a relation between a current state and a new state, specifying the effect of executing a sequence of instructions. For example, the instruction

4

---

Putting the actions of the proposer and acceptor together, we see that the algorithm operates in the following two phases.

Phase 1. (a) A proposer selects a proposal number \( n \) and sends a prepare request with number \( n \) to a majority of acceptors.

(b) If an acceptor receives a prepare request with number n greater than that of any prepare request to which it has already responded, then it responds to the request with a promise not to accept any more proposals numbered less than n and with the highest-numbered proposal (if any) that it has accepted.

Phase 2. (a) If the proposer receives a response to its prepare requests (numbered n) from a majority of acceptors, then it sends an accept request to each of those acceptors for a proposal numbered n with a value v, where v is the value of the highest-numbered proposal among the responses, or is any value if the responses reported no proposals. (b) If an acceptor receives an accept request for a proposal numbered n, it accepts the proposal unless it has already responded to a prepare request having a number greater than n.

A proposer can make multiple proposals, so long as it follows the algorithm for each one. ... It is probably a good idea to abandon a proposal if some proposer has begun trying to issue a high-numbered one. Therefore, if an acceptor ignores a prepare or accept request because it has already received a prepare request with a higher number, then it should probably inform the proposer, who should then abandon its proposal. This is a performance optimization that does not affect correctness.

To learn that a value has been chosen, a learner must find out that a proposal has been accepted by a majority of acceptors. The obvious algorithm is to have each acceptor, whenever it accepts a proposal, respond to all learners, sending them the proposal.

Fig. 1. Lamport's description of Basic Paxos in English [36].

\( x := x + 1 \) is represented in TLA and \( TLA^{+} \) by the action \( x' = x + 1 \). An action is represented by a formula over unprimed and primed variables where unprimed variables refer to the values of the variables in the current state and primed variables refer to the values of the variables in the new state.

A system is specified by its actions and initial states. Formally, a system is specified as \( Spec \triangleq Init \wedge \square [Next]_{vars} \) where \( Init \) is a predicate that holds for initial states of the system, \( Next \) is a disjunction of all actions of the system, and \( vars \) is the tuple of all variables. The expression \( [Next]_{vars} \) is true if either \( Next \) is true, implying some action is true and therefore executed, or \( vars \) stutters, that is, the values of the variables are same in the current and new states. \( \square \) is the temporal operator always. Thus, \( Spec \) defines a set of infinite sequences of steps where in each step either an action is true and the state changes or \( vars \) stutters. Such a sequence is called a behavior.

5

---

As a simple example, consider the following specification of a clock based on Lamport's logical clock [33] but on a shared memory system:

VARIABLE c

Max(S)  \( \triangleq \)  CHOOSE  \( e \in S : \forall f \in S : e \geq f \) 
Init  \( \triangleq c = [p \in \{0, 1\} \mapsto 0] \) 
LocalEvent(p)  \( \triangleq c' = [c \text{ EXCEPT } ![p] = c[p] + 1] \) 
ReceiveEvent(p)  \( \triangleq c' = [c \text{ EXCEPT } ![p] = Max(\{c[p], c[1 - p]\}) + 1] \) 
Next  \( \triangleq \exists p \in \{0, 1\} : LocalEvent(p) \lor ReceiveEvent(p) \) 
Spec  \( \triangleq Init \land \Box[Next]_{\langle c\rangle} \)

The system has two processes numbered 0 and 1. Variable c stores their current clock values as a function from process numbers to clock values. Both processes start with clock value 0, as specified in Init. LocalEvent(p) specifies that process p has executed some local action and therefore increments its clock value. The expression  \( c' = [c \text{ EXCEPT } ![p] = c[p] + 1] \)  means that function  \( c' \)  is the same as function c except that  \( c'[p] \)  is  \( c[p] + 1 \) . ReceiveEvent(p) specifies that process p updates its clock value to 1 greater than the higher of its and the other process' clock value. We define operator Max to obtain the highest of a set of values. CHOOSE returns an arbitrarily chosen value satisfying the body of the CHOOSE expression if one exists, or an arbitrary value otherwise.

### 2.4 TLAPS

TLA \( ^{+} \) Proof System (TLAPS) [5, 11, 16] is a tool that mechanically checks proofs of properties of systems specified in TLA \( ^{+} \). Proofs are written in a hierarchical style [40], and are transformed to individual proof obligations that are sent to backend theorem provers. An obligation is a logical formula of the form \( P \Rightarrow Q \). For proving an obligation, the default behaviour of TLAPS is to try three backend provers in succession: CVC3 (an SMT solver), Zenon, and Isabelle [51, 52, 57]. If none of them find a proof, TLAPS reports a failure on the obligation. Other SMT solvers supported by TLAPS are Z3, veriT, and Yices. Temporal formulas are proved using LS4, a PTL (Propositional Temporal Logic) prover. Users can specify which prover they want to use by using its name and can specify the timeout for each obligation separately.

As an example, we present the proof of a simple type invariant about the clock specification in (5) - It is always the case that \( c \in [\{0,1\} \to \mathbb{N}] \):

TypeOK  \( \triangleq c \in [\{0,1\} \to N] \) 

THEOREM Inv  \( \triangleq Spec \Rightarrow \Box(TypeOK) \) 

 \( \langle1\rangle. USE DEF TypeOK \) 

 \( \langle1\rangle1. Init \Rightarrow TypeOK BY DEF Init \) 

 \( \langle1\rangle2. TypeOK \land [Next]_{\langle c\rangle} \Rightarrow TypeOK' \) 

 \( \langle2\rangle. ASSUME TypeOK, [Next]_{\langle c\rangle} PROVE TypeOK' \) 

 \( \langle2\rangle1. CASE \exists p \in \{0,1\} : LocalEvent(p) BY \langle2\rangle1 DEF LocalEvent \) 

 \( \langle2\rangle2. CASE \exists p \in \{0,1\} : ReceiveEvent(p) BY \langle2\rangle2 DEF ReceiveEvent \) 

 \( \langle2\rangle3. CASE UNCHANGED \langle c\rangle BY \langle2\rangle3 \) 

 \( \langle2\rangle. QED BY \langle2\rangle1, \langle2\rangle2, \langle2\rangle3 DEF Next \) 

 \( \langle1\rangle. QED BY \langle1\rangle1, \langle1\rangle2, PTL DEF Spec \)

6

---

The proof of theorem Inv is written in a step-by-step fashion. It is proved by two steps, named  \( \langle1\rangle1 \)  and  \( \langle1\rangle2 \) , and the PTL solver. Proof steps in TLAPS are typically written as:

\[
\langle x \rangle y. \text {   Assertion   BY   } e _ {1}, \dots , e _ {m} \text {   DEF   } d _ {1}, \dots , d _ {n} \tag {7}
\]

which states that step number  \( \langle x\rangle y \)  proves Assertion by using  \( e_{1},\ldots,e_{m} \) , and expanding the definitions of  \( d_{1},\ldots,d_{n} \) . For example, step  \( \langle1\rangle1 \)  proves Init  \( \Rightarrow \)  TypeOK by expanding the definition of Init. If TLAPS does not know if  \( e_{i} \)  is true, it would try to prove  \( e_{i} \)  using  \( e_{1},\ldots,e_{i-1} \)  and the current context. If TLAPS is unable to prove  \( e_{i} \) , it would display both  \( e_{i} \)  and Assertion as failed obligations. The step “ \( \langle1\rangle \) . USE DEF TypeOK” instructs the prover to expand the definition of TypeOK in all proof steps till the QED step for  \( \langle1\rangle \) . The QED step for  \( \langle1\rangle \)  instructs TLAPS to invoke a PTL prover because Inv is a temporal formula.

To demonstrate the hierarchical proof style advocated in TLAPS, we break down the proof of step  \( \langle1\rangle2 \) . Step  \( \langle2\rangle \)  ASSUME ... PROVE specifies the assumptions and goal to be proved in the current proof level, which is level 2. The next two steps  \( \langle2\rangle1 \)  and  \( \langle2\rangle2 \)  prove the goal for the two actions specified in Next. Finally,  \( \langle2\rangle3 \)  proves the goal for the case of stuttering. Together,  \( \langle2\rangle1-3 \)  cover all cases of  \( [Next]_{\langle c\rangle} \) , thus concluding the proof.

## 3 SPECIFICATION OF MULTI-PAXOS

We develop a formal specification of Multi-Paxos by minimally extending that of Basic Paxos by Lamport et al. [41]. Lamport et al.'s specification of Basic Paxos formally specifies the phases executed by proposers and acceptors described by Lamport in [36] and shown in Fig. 1; it does not specify preemption, that is, abandoning proposals, and learners that are also in Fig. 1. We discuss learners at the end of this section, and add preemption in Section 5.

### 3.1 Constants and variables

Constants. The specification of Multi-Paxos has six global constants. It assumes that the sets of proposers, acceptors, and quorums are constant and are an input to the algorithm.

\(\mathcal{P}\) : the set of proposers.

\(\mathcal{A}\) : the set of acceptors.

Q: the set of quorums.

\(\mathcal{V}\) : the set of values that can be proposed.

\(\mathcal{B}\): the set of ballots. This is defined to be the set of natural numbers.

S: the set of slots. This is defined to be the set of natural numbers.

We do not specify how the quorum set is constructed, rather provide the property that it satisfies as an assumption:

\[
\text { ASSUME   QuorumAssumption } \triangleq Q \subseteq \text { SUBSET } \mathcal {A} \wedge \forall Q 1, Q 2 \in Q: Q 1 \cap Q 2 \neq \emptyset \tag {8}
\]

This allows for specifying different kinds of quorums like majority, tree-based [1], matrix-based [46], etc. Upon doing so, all one has to do is to prove that the specified quorum system satisfies QuorumAssumption and the rest of the proof needs no change.

Variables. The specification of Multi-Paxos has four global variables.

msgs: the set of messages that have been sent in the system. Processes read from or add to this set. This is the same as in the specification of Basic Paxos except that the contents of messages are more complex.

pBal: per proposer, the current ballot number of the proposer. This is not in the specification of Basic Paxos; it is added to support preemption.

7

---

|  Basic Paxos | Multi-Paxos  |
| --- | --- |
|  Phase1a(b ∈ B) ≜ ∧# m ∈ msgs : ∧m.type = “1a” ∧m.bal = b ∧Send([type ↦ “1a”, bal ↦ b]) ∧UNCHANGED ⟨maxVBal, maxBal, maxVal⟩ | Phase1a(p ∈ P) ≜ ∃b ∈ B : ∧# m ∈ msgs : ∧m.type = “1a” ∧m.bal = b ∧Send([type ↦ “1a”, from ↦ p, bal ↦ b]) ∧pBal' = [pBal EXCEPT ![p] = b] ∧UNCHANGED ⟨aBal, aVoted⟩  |

Fig. 2. Specifications of Phase 1a of Basic Paxos and Multi-Paxos

aBal: per acceptor, the highest ballot number seen by the acceptor. This is named maxBal in the specification of Basic Paxos.

aVoted: per acceptor, a set of triples in \(\mathcal{B} \times \mathcal{S} \times \mathcal{V}\) voted by the acceptor. For each slot only the triple with the highest ballot number is stored. This contrasts with two numbers per acceptor, in two variables, maxVBal and maxVal, in the specification of Basic Paxos, which respectively store the highest ballot in which the acceptor has voted, and the value that the acceptor voted for in this highest ballot.

### 3.2 Algorithm steps and complete specification

The algorithm consists of repeatedly executing two phases.

Phase 1a. Fig. 2 shows the specifications of Phase 1a for Basic Paxos and Multi-Paxos, which are in essence the same. Parameter ballot number \( b \) in Basic Paxos is replaced with proposer \( p \) executing this phase in Multi-Paxos, to allow extensions such as preemption that need to know the proposer of a ballot number; from \( \mapsto p \) is added in Send and \( pBal[p] \) is updated to \( b \). Send is a predicate for adding its argument to msgs, i.e., \( Send(m) \triangleq msgs' = msgs \cup \{m\} \). In this specification, 1a messages do not have a receiver, making them accessible to all processes. However, this is not required. For safety, it is enough to send this message to any subset of \( \mathcal{A} \), even \( \emptyset \). For liveness, the receiving set should contain at least one quorum. Because this work aims at proving safety, we do not specify any constraints on receiving messages to keep the specifications concise and to the point.

Phase 1b. Fig. 3 shows the specifications of Phase 1b. Parameter acceptor \( a \) executes this phase. The only key difference between the specifications is the set aVoted[a] of triples in Send of Multi-Paxos vs. the two numbers maxVBal[a] and maxVal[a] in Basic Paxos.

Phase 2a. Fig. 4 shows the specifications of Phase 2a. The key difference is, in Send, the bloating of a single value v in V in Basic Paxos to a set of pairs in S × V given by PropSV in Multi-Paxos. A proposal is a  \( \langle s, v \rangle \)  pair. The operation of finding the value with the highest ballot in Basic Paxos is performed for each slot by MaxSV in Multi-Paxos; MaxSV takes a set T of triples in B × S × V and returns a set of pairs in S × V. NewSV generates a set of pairs in S × V where values are proposed for slots not in MaxSV. This is significantly more sophisticated than running Basic Paxos for each slot, because the ballots are shared and changing for all slots, and slots are paired with values dynamically where slots that failed to reach consensus values earlier are also detected and reused.

Phase 2b. Fig. 5 shows the specifications of Phase 2b. In Basic Paxos, the acceptor replies with the value received in the 2a message whereas in Multi-Paxos, it replies with a set of pairs in \( S \times V \) received in the 2a message. Also, in Basic Paxos, the acceptor updates its voted pair

8

---

|  Basic Paxos | Multi-Paxos  |
| --- | --- |
|  Phase1b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “1a”∧m.bal > maxBal[a]∧Send([type ↦ “1b”,acc ↦ a,bal ↦ m.bal,maxVBal ↦ maxVBal[a],maxVal ↦ maxVal[a]])∧maxBal' = [maxBal EXCEPT ![a] = m.bal]∧UNCHANGED ⟨maxVBal, maxVal⟩ | Phase1b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “1a”∧m.bal > aBal[a]∧Send([type ↦ “1b”,from ↦ a,bal ↦ m.bal,voted ↦ aVoted[a]])∧aBal' = [aBal EXCEPT ![a] = m.bal]∧UNCHANGED ⟨pBal, aVoted⟩  |

Fig. 3. Specifications of Phase 1b of Basic Paxos and Multi-Paxos

|  Basic Paxos | Multi-Paxos  |
| --- | --- |
|  Phase2a(b ∈ B) ≜∧#m ∈ msgs : ∧m.type = “2a”∧m.bal = b∧∃v ∈ V :∧∃Q ∈ Q, S ⊆†{m ∈ msgs : m.type = “1b”∧m.bal = b} :∧∀a ∈ Q : ∃m ∈ S : m.acc = a∧ ∨ ∀m ∈ S : m.maxVBal = -1∨∃c ∈ 0..(b - 1) :∧∀m ∈ S : m.maxVBal ≤ c∧∃m ∈ S : (m.maxVBal = c)∧m.maxVal = v∧Send([type ↦ “2a”, bal ↦ b, val ↦ v])∧UNCHANGED ⟨maxBal, maxVBal, maxVal⟩ | Phase2a(p ∈ P) ≜∧#m ∈ msgs : ∧m.type = “2a”∧m.bal = pBal[p]∧∃Q ∈ Q, S ⊆ {m ∈ msgs : m.type = “1b”∧m.bal = pBal[p]} :∧∀a ∈ Q : ∃m ∈ S : m.from = a∧Send([type ↦ “2a”,from ↦ p,bal ↦ pBal[p],propSV ↦ PropSV(UNION{m.voted : m ∈ S})])∧UNCHANGED ⟨pBal, aBal, aVoted⟩where,MaxBSV(T) ≜ {t ∈ T : ∀t2 ∈ T :t2.slot = t.slot ⇒ t2.bal ≤ t.bal}MaxSV(T) ≜ {[slot ↦ t.slot,val ↦ t.val] : t ∈ MaxBSV(T)}UnusedS(T) ≜ {s ∈ S : #t ∈ T : t.slot = s}NewSV(T) ≜ CHOOSE D ⊆ [slot :UnusedS(T), val : V] : ∀d1, d2 ∈ D:d1.slot = d2.slot ⇒ d1 = d2PropSV(T) ≜ MaxSV(T) ∪ NewSV(T)  |

Fig. 4. Specifications of Phase 2a of Basic Paxos and Multi-Paxos.

\( ^{\dagger} \) TLA* forbids quantifier expressions of the form  \( Q S \subseteq T \)  where  \( Q \in \{\forall, \exists, CHOOSE, PICK\} \) , instead allowing  \( Q S \in SUBSET T \) . For convenience of presentation and spacing, we use the former in this article.

maxVBal[a] and maxVal[a] upon receipt of a 2a message of the highest ballot; in Multi-Paxos, this is performed for each slot. The acceptor updates a Voted to have all proposals

9

---

in the received 2a message and all previous values in a Voted for slots not mentioned in that message.

|  Basic Paxos | Multi-Paxos  |
| --- | --- |
|  Phase2b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “2a”∧m.bal ≥ maxBal[a]∧Send([type ↦ “2b”,acc ↦ a,bal ↦ m.bal,val ↦ m.val])∧maxBal' = [maxBal EXCEPT ![a] = m.bal]∧maxVBal' =[maxVBal EXCEPT ![a] = m.bal]∧maxVal' =[maxVal EXCEPT ![a] = m.val] | Phase2b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “2a”∧m.bal ≥ aBal[a]∧Send([type ↦ “2b”,from ↦ a,bal ↦ m.bal,propSV ↦ m.propSV])∧aBal' = [aBal EXCEPT ![a] = m.bal]∧aVoted' = [aVoted EXCEPT ![a] =∪{[bal ↦ m.bal, slot ↦ d.slot,val ↦ d.val] : d ∈ m.propSV}∪{e ∈ aVoted[a] :#r ∈ m.propSV : e.slot = r.slot}]∧UNCHANGED ⟨pBal⟩  |

Fig. 5. Specifications of Phase 2b of Basic Paxos and Multi-Paxos

Complete algorithm specification. To complete the algorithm specification, we specify the global constants of the system—the set of proposers, acceptors, quorums, and values, and define B, S, vars, Init, Next, and Spec, denoting the set of ballots, slots, variables, the initial state, possible actions leading to the next state, and the system specification, respectively:

\[
\text { vars } \triangleq \langle m s g s, p B a l, a B a l, a V o t e d \rangle
\]

\[
\text { Init } \triangleq m s g s = \emptyset \land p B a l = [ p \in \mathcal {P} \mapsto 0 ] \land a B a l = [ a \in \mathcal {A} \mapsto - 1 ] \land a V o t e d = [ a \in \mathcal {A} \mapsto \emptyset ]
\]

\[
\text { Next } \triangleq (\exists p \in \mathcal {P}: \text { Phase1a } (p) \lor \text { Phase2a } (p)) \lor (\exists a \in \mathcal {A}: \text { Phase1b } (a) \lor \text { Phase2b } (a))
\]

\[
\text { Spec } \triangleq \text { Init } \land \square [ \text { Next } ] _ {\text { vars }}
\]

(9)

We have specified \(\mathcal{P}\), \(\mathcal{A}\) and \(Q\) as constant. This means that the algorithm specified in this article does not include reconfiguration in which processes join and leave the system dynamically.

The complete specification of Multi-Paxos with Preemption is given in Appendix A. Preemption is discussed in Section 5. We only provide specification of Multi-Paxos with Preemption because it is an extension of Multi-Paxos, and giving also specification of Multi-Paxos would be redundant.

### 3.3 Fault model and implementation issues

Fault model. We explain how the fault model described in Section 2.1 is naturally specified due to the definition of Spec, especially due to msgs being a set and  \( \exists m \in msgs \)  being nondeterministic. Spec specifies the set of allowed behaviors of Multi-Paxos. Recalling the meaning of the definition of Spec from Section 2.3, in every allowable behavior of Multi-Paxos the next state is obtained from the current state either by a process performing one of the phases or by stuttering where vars remains unchanged.

10

---

**Message loss, delay, reordering, and duplication.** Because our model exhibits infinite stuttering steps, and choosing a next message to handle is nondeterministic, the model can always stutter and avoid handling any particular message in *msgs*, allowing for arbitrary message delays and loss. Also, the model can pick messages in any order because *msgs* is a set and pick a message any number of times because it never removes a message from *msgs*, allowing for arbitrary message reordering and duplication.

**Process crash and recovery.** We view a crashed process as one that cannot send or receive messages from any other process in the system. Then, similar to message delay, process crash is modeled by having stuttering steps instead of the crashed process executing some phase. Note that this way of modeling process crash requires that all state (*vars*) is stored in stable storage; use of stable storage is standard for recovery from crashes.

**Specification vs. implementation.** Following Lamport et al. [41], our specification of Multi-Paxos abstracts from certain implementation issues.

- The first conjunct of *Phase1a* in Lamport et al.'s and our specifications states that no **1a** message has been sent with ballot *b*. Similarly for *Phase2a* and **2a** messages. To implement this in practice, we need a mechanism that lets proposers choose ballots from disjoint sets, the union of which is a total order.

A common way of doing this is by defining a ballot as a pair in $\mathbb{N} \times \mathcal{P}$, and having $\mathcal{P} \subseteq \mathbb{N}$, as for instance in [45]. $(\mathbb{N} \times \mathcal{P}, \leq)$ is a totally ordered set such that $\forall (n1, p1), (n2, p2) \in \mathbb{N} \times \mathcal{P} : (n1, p1) \leq (n2, p2) \iff (n1 < n2 \lor (n1 = n2 \land p1 \leq p2))$.

- The purpose of our specification is to provide the core of Multi-Paxos, following Lamport et al. [41] for Basic Paxos. Therefore, we omit extensions and optimizations like reconfiguration, state reduction, and failure detection, which would distract from the essence of the algorithm.

For example, in our specifications, a **1b** message contains votes on all the slots in which the acceptor has ever voted, following van Renesse and Altinbuken [59]. In real implementations, an optimization would be used to reduce the size of **1b** messages.

**Learners and learner condition.** Learners learn chosen values when they receive votes in **2b** messages from a quorum of acceptors. Our specification of Multi-Paxos omits learners because our goal is to minimally extend [41], which omits learners.

Adding specification of learners is straightforward because it would be the same as *Chosen*. As a result, the learner condition C3 in Section 2.1, which states that only a chosen value can be learned, holds straightforwardly.

#### 4 VERIFICATION OF MULTI-PAXOS AND GENERAL PROOF STRATEGIES

We first define the auxiliary predicates and invariants used, by extending those for the proof of Basic Paxos with slots, and then describe our proof strategy, which proves *Safe* of Multi-Paxos.

We define and prove three kinds of invariants, following Lamport et al. [41]: type invariants, process invariants, and message invariants. These are sufficient because a distributed algorithm handles two kinds of data: messages communicated and process local data. Correspondingly, we obtain message invariants for the messages passed between processes and process invariants over the local data that these processes maintain. Type invariants ensure that the specification always uses data with correct types.

11

---

### 4.1 Auxiliary predicates and functions

These predicates and functions are used throughout the proof. Predicate  \( Chosen(s, v) \)  is true iff there exists some ballot b such that  \( ChosenIn(b, s, v) \)  holds.

\[
\text { Chosen } (s \in \mathcal {S}, v \in \mathcal {V}) \triangleq \exists b \in \mathcal {B}: \text { ChosenIn } (b, s, v) \tag {10}
\]

ChosenIn(b, s, v) is true iff there exists some quorum of acceptors such that for each acceptor in the quorum, VotedForIn(a, b, s, v) holds.

\[
\text { ChosenIn } (b \in \mathcal {B}, s \in \mathcal {S}, v \in \mathcal {V}) \triangleq \exists Q \in Q: \forall a \in Q: \text { VotedForIn } (a, b, s, v) \tag {11}
\]

VotedForIn(a, b, s, v) is true iff acceptor a has voted value v for slot s in ballot b. This is realized in the algorithm by sending a 2b message with ballot b and with pair  \( \langle s, v \rangle \)  in the message's propSV in Phase2b. Putting everything together, the algorithm chooses value v for slot s if there exist some quorum Q and ballot b such that every acceptor in Q has voted value v for slot s in ballot b:

\[
\text { VotedForIn } (a \in \mathcal {A}, b \in \mathcal {B}, s \in \mathcal {S}, v \in \mathcal {V}) \triangleq \exists m \in m s g s:
\]

\[
m. \text {type} = ^ {\prime \prime} 2 b ^ {\prime \prime} \wedge m. \text {from} = a \wedge m. \text {bal} = b \wedge \exists d \in m. \text {propSV}: d. \text {slot} = s \wedge d. \text {val} = v \tag {12}
\]

Predicate SafeAt(b ∈ B, s ∈ S, v ∈ V) means that no value except possibly v has been or will be chosen in any ballot lower than b for slot s. This is realized by asserting that for each ballot b2 < b, there exists some quorum Q such that for every acceptor in Q, either VotedForIn(a, b2, s, v) holds or WontVoteIn(a, b2, s) holds.

\[
\begin{array}{l} \text { SafeAt } (b \in \mathcal {B}, s \in \mathcal {S}, v \in \mathcal {V}) \triangleq \forall b 2 \in 0.. (b - 1): \exists Q \in Q: \\ \forall a \in Q: \text { VotedForIn } (a, b 2, s, v) \vee \text { WontVoteIn } (a, b 2, s) \tag {13} \\ \end{array}
\]

WontVoteIn(a, b, s) holds iff acceptor a has seen a higher ballot than b, and did not and will not vote any value in b for slot s

\[
\text { WontVoteIn } (a \in \mathcal {A}, b \in \mathcal {B}, s \in \mathcal {S}) \triangleq a B a l [ a ] > b \land \forall v \in \mathcal {V}: \neg \text { VotedForIn } (a, b, s, v) \tag {14}
\]

Function MaxBalInSlot(T ⊆ [bal : B, slot : S], s ∈ S) selects among set of elements in T with slot s, the highest ballot, or -1 if no element has slot s.

\[
\begin{array}{l} \operatorname{Max} (T) \triangleq \text {   CHOOSE   } e \in T: \forall f \in T: e \geq f \\ \text { MaxBalInSlot } (T \subseteq [ b a l: \mathcal {B}, s l o t: \mathcal {S} ], s \in \mathcal {S}) \triangleq \tag {15} \\ \text { LET } E \triangleq \{e \in T: e. s l o t = s \} \\ \text { IN } \quad \text { IF } E = \emptyset \text { THEN } - 1 \text { ELSE } M a x (\{e. b a l: e \in E \}) \\ \end{array}
\]

To prove the Safe property in (2) for the algorithm, we prove the following two lemmas:

(1) Lemma VotedInv. If any acceptor votes any triple \(\langle b, s, v \rangle\), then the predicate \(SafeAt(b, s, v)\) holds. That is, \(\forall a \in \mathcal{A}, b \in \mathcal{B}, s \in \mathcal{S}, v \in \mathcal{V}: VotedForIn(a, b, s, v) \Rightarrow SafeAt(b, s, v)\).
(2) Lemma VotedOnce. If acceptor \(a1\) votes triple \(\langle b, s, v1 \rangle\) and acceptor \(a2\) votes triple \(\langle b, s, v2 \rangle\), then \(v1 = v2\). That is, \(\forall a1, a2 \in \mathcal{A}, b \in \mathcal{B}, s \in \mathcal{S}, v1, v2 \in \mathcal{V}\): VotedForIn(a1, b, s, v1) ∧ VotedForIn(a2, b, s, v2) ⇒ \(v1 = v2\).

In fact, for other consensus algorithms, either Paxos extensions like Fast Paxos  \( [38] \)  and Byzantine Paxos  \( [39] \) , or Paxos alternatives like Viewstamped Replication  \( [43] \)  and Raft  \( [54] \) , safety can also be proved by asserting these two properties.

12

---

### 4.2 Type invariants

Type invariants are captured by TypeOK. They specify the sets of values that the variables of the system can hold. For example,  \( pBal \in [P \to B] \)  states that pBal is a function whose domain is P and whose range is B, i.e., for any p in P, pBal[p] is in B.

Messages  \( \triangleq \) 

 \( \cup [type : \{"1a"\}, bal : B, from : P] \) 

 \( \cup [type : \{"1b"\}, bal : B, voted : SUBSET [bal : B, slot : S, val : V], from : A] \) 

 \( \cup [type : \{"2a"\}, bal : B, propSV : SUBSET [slot : S, val : V], from : P] \) 

 \( \cup [type : \{"2b"\}, bal : B, propSV : SUBSET [slot : S, val : V], from : A] \) 

TypeOK  \( \triangleq \) 

 \( \wedge msgs \subseteq Messages \wedge pBal \in [P \to B] \wedge aBal \in [A \to B \cup \{-1\}] \) 

 \( \wedge aVoted \in [A \to SUBSET [bal : B, slot : S, val : V]] \)

### 4.3 Invariants about acceptors

The following predicate specifies invariants about acceptor processes. For each acceptor \( a \), the first conjunct establishes the initial condition. The second conjunct says that \( aBal[a] \) is higher than or equal to the ballot number of each triple in \( aVoted[a] \) and \( a \) has voted each triple in \( aVoted[a] \). The third conjunct states that if acceptor \( a \) has voted any value \( v \) for slot \( s \) in ballot \( b \), then there is some triple \( t \) in \( aVoted[a] \) such that \( t.bal \geq b \) and \( t.slot = s \). The last conjunct says that acceptor \( a \) has not voted for a value in any ballot higher than the highest it has seen per slot.

AccInv \(\triangleq \forall a\in \mathcal{A}\) :

\( \wedge (aBal[a] = -1) \Rightarrow (aVoted[a] = \emptyset) \)

\( \wedge \forall t \in aVoted[a] : aBal[a] \geq t.bal \wedge VotedForIn(a, t.bal, t.slot, t.val) \)

\( \wedge \forall b \in \mathcal{B}, s \in \mathcal{S}, v \in \mathcal{V} : VotedForIn(a, b, s, v) \Rightarrow \exists t \in aVoted[a] : t.bal \geq b \wedge t.slot = s \)

\( \wedge \forall b \in \mathcal{B}, s \in \mathcal{S}, v \in \mathcal{V} : b > MaxBalInSlot(aVoted[a], s) \Rightarrow \neg VotedForIn(a, b, s, v) \)

(17)

### 4.4 Invariants about messages

The following invariant is for a 1b message m, sent from an acceptor m.from, where m.bal is the ballot number of the 1a message that m is a reply of, and m.voted is the value of a Voted[m.from] when m was sent. The first conjunct says that m.bal is no higher than the highest ballot number seen by m.from. The second conjunct states that m.from has voted every triple in the set m.voted. The last conjunct asserts that for each slot s and ballot b higher than the highest ballot that m.from has voted in for slot s, and lower than the ballot in m, m.from has not voted any value v for slot s in ballot b.

MsgInv1b(m)  \( \triangleq \) 

 \( \wedge m.bal \leq aBal[m.from] \) 

 \( \wedge \forall t \in m.voted : VotedForIn(m.from, t.bal, t.slot, t.val) \) 

 \( \wedge \forall b \in B, s \in S, v \in V : b \in MaxBalInSlot(m.voted, s) + 1..m.bal - 1 \) 

 \( \Rightarrow \neg VotedForIn(m.from, b, s, v) \)

13

---

The following invariant is for a 2a message m, where m.bal is the ballot for which a quorum of 1b replies were received by m.from, and m.propSV is the set of proposals that m.from proposes for ballot m.bal based on the set of voted triples in these 1b replies, as shown in Phase2a (Fig. 4). The first conjunct establishes safety for each proposal d in m. The second conjunct says that two proposals in m with the same slot must be the same proposal. That is, for each slot, there is at most one proposal in m. The third conjunct says that there is at most one 2a message for each ballot.

\[
M s g I n v 2 a (m) \triangleq
\]

\[
\wedge \forall d \in m. p r o p S V: S a f e A t (m. b a l, d. s l o t, d. v a l)
\]

\[
\wedge \forall d 1, d 2 \in m. p r o p S V: d 1. s l o t = d 2. s l o t \Rightarrow d 1 = d 2 \tag {19}
\]

\[
\wedge \forall m 2 \in m s g s: (m 2. t y p e = " 2 a") \wedge (m 2. b a l = m. b a l) \Rightarrow m 2 = m
\]

The following invariant is for a 2b message m sent by acceptor m.from, where m.bal is the ballot number of the 2a message m2 that m is a reply of, and m.propSV is the same set of proposals as in m2. The first conjunct states that there exists a 2a message with the same ballot and same set of proposals as m. The second conjunct asserts that the ballot of m is no higher than the highest ballot seen by m.from.

\[
M s g I n v 2 b (m) \triangleq
\]

\[
\wedge \exists m 2 \in m s g s: m 2. t y p e = " 2 a" \wedge m 2. b a l = m. b a l \wedge m 2. p r o p S V = m. p r o p S V \tag {20}
\]

\[
\wedge m. b a l \leq a B a l [ m. f r o m ]
\]

The complete message invariant is the conjunction of  \( MsgInv1b \) ,  \( MsgInv2a \) , and  \( MsgInv2b \) :

\[
M s g I n v \triangleq \forall m \in m s g s: \wedge (m. t y p e = " 1 b") \Rightarrow M s g I n v 1 b (m)
\]

\[
\wedge (m. t y p e = " 2 a") \Rightarrow M s g I n v 2 a (m) \tag {21}
\]

\[
\wedge (m. t y p e = " 2 b") \Rightarrow M s g I n v 2 b (m)
\]

The complete invariants, auxiliary operators, and the safety property to be proved can be found in Appendix B.

### 4.5 Overall proof strategy

The main theorem to prove is Safety as defined in (22). For this, we define Inv and first prove  \( Inv \Rightarrow Safe \) . Then, we prove  \( Spec \Rightarrow \Box Inv \)  which by temporal logic, concludes  \( Spec \Rightarrow \Box Safe \) . Note that property Safety is called Consistent, and invariant Safe is called Consistency by Lamport et al. [41].

\[
I n v \triangleq T y p e O K \wedge A c c I n v \wedge M s g I n v
\]

\[
\text { Safe } \triangleq \forall v 1, v 2 \in \mathcal {V}, s \in \mathcal {S}: \text { Chosen } (s, v 1) \wedge \text { Chosen } (s, v 2) \Rightarrow v 1 = v 2 \tag {22}
\]

\[
\text { THEOREM   Safety } \triangleq S p e c \Rightarrow \square S a f e
\]

The proof is developed following a standard hierarchical structure and uses proof by induction and contradiction. To prove  \( Spec \Rightarrow \Box Inv \) , we employ a systematic proof strategy that works well for algorithms described in an event-driven style [15]. Distributed algorithms are prime examples of this style as they can be specified in blocks of code, each triggered upon the event of receiving a certain set of messages, and ending with sending a set of messages. We demonstrate the strategy with a few invariants in Inv as examples.

First, consider invariant TypeOK. The goal to prove is  \( Spec \Rightarrow \Box TypeOK \) . Recall  \( Spec \triangleq Init \land \Box [Next]_{vars} \) :

14

---

- The induction basis, $Init \Rightarrow TypeOK$, is trivial in this case because the set of sent messages is empty. To prove this step, we need to instruct TLAPS to unfold the definitions of $Init$ and $TypeOK$ and of all other predicates in $TypeOK$ till we have only statements about the variables. In most cases, TLAPS would then prove this step without a manually written proof.
- The induction step is to prove $TypeOK \wedge [Next]_{vars} \Rightarrow TypeOK'$, where the left side is the induction hypothesis, and right side is the goal to be proved. $[Next]_{vars}$ is a disjunction of phases, as for any distributed algorithm, and $TypeOK'$ is a conjunction of smaller invariants, as for many invariants.

The hypothesis of the induction step can be stripped down to each disjunct of $Next$ separately, and each smaller goal needs to be proved for each disjunct. This process is mechanical, and TLAPS provides a feature for precisely this expansion into smaller proof obligations. This breakdown is the first step in our proof strategy. For $TypeOK$, this expands to 4 smaller assertions; with 4 phases in $Next$, we obtain 16 even smaller proof obligations.

### 4.6 Proving complex invariants using invariance lemmas and increments

$AccInv$ and $MsgInv$ are more involved. We proceed as we did for $TypeOK$ and create a proof tree, each branch of which aims to prove an invariant for a disjunct in $Next$. To explain the rest of our strategy, we show a complex combination: $MsgInv$ and $Phase1b$. (23) shows the skeleton of the proof.

$\langle 4 \rangle 2. \text{ASSUME NEW } a \in \mathcal{A}, \text{Phase1b}(a) \text{ PROVE } MsgInv'$

$\langle 5 \rangle 1. \text{PICK } m \in msgs : \text{Phase1b}(a)!(m) \dots \quad \backslash * m$ is a witness of $\text{Phase1b}(a)$

$\langle 5 \rangle . \text{DEFINE } m1 \triangleq [type \mapsto "1b", from \mapsto a, bal \mapsto m.bal, voted \mapsto a\text{Voted}[a]]$

$\langle 5 \rangle 2.(m1.bal \leq aBal[m1.from])' \dots$

$\langle 5 \rangle 4. (\forall r \in m1.voted : \text{VotedForIn}(m1.from, r.bal, r.slot, r.val))' \dots$

$\langle 5 \rangle 12. (\forall b \in \mathcal{B}, s \in \mathcal{S}, v \in \mathcal{V} : b \in \text{MaxBalInSlot}(m1.voted, s) + 1..m1.bal - 1 \Rightarrow \neg \text{VotedForIn}(m1.from, b, s, v))' \dots$

$\langle 5 \rangle 13. \text{CASE } m.bal > aBal[a]$

$\langle 6 \rangle \text{ASSUME NEW } m2 \in msgs' \text{ PROVE } MsgInv!(m2)' \quad \backslash * m2$ is the instantiation in $MsgInv'$

$\langle 6 \rangle 1.(m2.type = "1b" \Rightarrow MsgInv1b(m2))'$

$\langle 7 \rangle 1. \text{CASE } m2 \in msgs \dots$

$\langle 7 \rangle 2. \text{CASE } m2 \in msgs' \setminus msgs \dots$

$\langle 6 \rangle 2. ((m2.type = "2a" \Rightarrow MsgInv2a(m2)) \wedge$

$(m2.type = "2b" \Rightarrow MsgInv2b(m2)))' \dots$

(23)

The goal is to prove $\langle 4 \rangle 2$, which states that $MsgInv'$ holds if acceptor $a$ executes $Phase1b$. $\langle 6 \rangle 1$ proves $MsgInv1b'$ and $\langle 6 \rangle 2$ proves $MsgInv2a'$ and $MsgInv2b'$.

We describe the use of invariance lemmas to prove $\langle 6 \rangle 2$ and the additional use of increments to prove $\langle 6 \rangle 1$.

**Invariance lemmas.** $\langle 6 \rangle 2$ seems easy, because $Phase1b$ sends a $1b$ message, and $MsgInv2a$ and $MsgInv2b$ are not invariants about $1b$ messages. However, this is not the case because $MsgInv2a$

15

---

uses predicate SafeAt, which is more complicated. To succeed, we use what we call invariance lemmas.

An invariance lemma is a lemma that asserts that a predicate continues to hold as the system goes from one state to the next in a single step. This happens when a step does not affect the part of system state asserted by the predicates, and requires more tedious proofs otherwise. For example, the invariance lemma for SafeAt states that SafeAt continues to hold for any disjunct in Next, which includes Phase1b. The characteristic property of such lemmas is their reuse. In our proof of Multi-Paxos, we defined 5 invariance lemmas which are used in 27 places.

Increments.  \( \langle6\rangle1 \)  is more complicated, because MsgInv1b is about 1b messages, and Phase1b generates a 1b message. The prover needs more manual intervention. To proceed, we split the set of messages in the new state into two sets:  \( \langle7\rangle1 \)  for the old messages, and  \( \langle7\rangle2 \)  for the increment, the new message sent in this step. For the old messages, we use invariance lemmas. The most challenging case is for the increment.

An increment is a new message sent in a phase or generally a new element in a set. In the example in (23), the increment is \( m1 \), and we focus on the cause of the increment—here from the definition of Phase1b—and prove each conjunct of the goal MsgInv1b for \( m1 \) separately, in \( \langle 5\rangle 2,4,12 \). For \( \langle 5\rangle 2 \), the prover just needs the definition of Phase1b. For \( \langle 5\rangle 4 \), the prover needs in addition invariance lemma for VotedForIn for Phase1b. For \( \langle 5\rangle 12 \), the prover further needs case-specific manual intervention. Specifically, we helped the prover understand the change in limits of the set MaxBalInSlot(\( m1.voted,s \)) + 1..\( m1.bal-1 \). This proves MsgInv1b for \( m1 \). The only thing left to prove now is that \( m1 \) is in fact the increment due to Phase1b. This is proved in \( \langle 7\rangle 2 \) by adding the assertion \( m2 = m1 \). \( \langle 7\rangle 1 \) uses invariance lemmas to prove that the invariant continues to hold for the old messages. This shows an example of proof with set increment.

### 4.7 Induction for properties over sets, and ways of accessing components of tuples

With the strategy discussed so far, we were still faced with certain assertions that were difficult to prove. One of the main difficulties lay in proving properties about tuples and sets of tuples for each of a set of processes in Multi-Paxos, as opposed to one or two values for each of a set of processes in Basic Paxos. It may appear that, in many places, this requires simply adding an extra parameter for the slot, but the proof became significantly more difficult: even in places where an explicit inductive proof is not needed, auxiliary facts had to be added to help TLAPS succeed or proceed sufficiently fast.

For example, to prove theorem Safety by adding a slot component to the proof of theorem Safety for Basic Paxos, the prover took about 90 seconds to check the proof. To aid the proof, we added \(\exists a\in \mathcal{A}:VotedForIn(a,b1,s,v1)\wedge VotedForIn(a,b1,s,v2)\) as an intermediate fact derivable from \(b1 = b2\wedge ChosenIn(b1,s,v1)\wedge ChosenIn(b2,s,v2)\), under the case \(b1 = b2\) in the proof of theorem Safety. Following this, the prover asserted the conclusion \(v1 = v2\) in a few milliseconds, a \(10,000\times\) speedup.

Tuples have only a fixed number of components and therefore do not require separate inductive proofs. However, they often turn out to be tricky and require special care in choosing the ways to access and test their components, to sufficiently reduce TLAPS's proof-checking time and observe progress.

For example, consider the definition of VotedForIn in (12). Originally,  \( [slot \mapsto s, val \mapsto v] \in m.propSV \)  was written for the last conjunct with existential quantification,  \( \exists d \in m.propSV : d.slot = s \land d.val = v \) , because it was natural, but it had to be changed to the latter for the prover to make more observable progress. With the original version, the proof did not carry through after

16

---

1 or 2 minutes. After the change, the proof proceeded quickly. One minute of waiting for such simple, small tests felt very long, making it uncertain whether the proof would carry through.

### 4.8 Note on validity condition

We discuss the validity condition C1 in Section 2.1. In our specification, value v is proposed for slot s if the following predicate holds:

\[
\text { Proposed } (s, v) \triangleq \exists m \in \text { sent }: m. \text { type } = \text {"2a"} \land \exists d \in m. \text { propSV }: d. \text { slot } = s \land d. \text { val } = v \tag {24}
\]

Then, C1 can be formally stated as

\[
\forall s \in \mathcal {S}, v \in \mathcal {V}: \text { Chosen } (s, v) \Rightarrow \text { Proposed } (s, v) \tag {25}
\]

From the definitions of Chosen, ChosenIn, and VotedForIn, one can easily conclude the following:

\[
\begin{array}{l} \forall s \in \mathcal {S}, v \in \mathcal {V}: \text { Chosen } (s, v) \Rightarrow \\ \exists m \in \text { sent }: m. \text { type } = \text {"2b"} \land \exists d \in m. \text { propSV }: d. \text { slot } = s \land d. \text { val } = v \tag {26} \\ \end{array}
\]

Then, from the first conjunct of \(MsgInv2b\) for 2b messages, one can directly deduce that C1 holds.

## 5 MULTI-PAXOS WITH PREEMPTION, AND OVERALL PROOF IMPROVEMENTS

Preemption is described informally in Lamport's description of Basic Paxos in Fig. 1, in the paragraph after the two phases. It is about when and how to let a proposer abandon its current ballot. Specifically, preemption makes an acceptor reply to a proposer, instead of doing nothing as in the specifications discussed so far, in both Phases 1b and 2b, if the proposer's ballot is preempted, i.e., the acceptor has seen a higher ballot than the one just received from the proposer.

This reply informs the proposer of the highest ballot the acceptor has seen, and the proposer can abandon its lower ballot number and later choose a higher ballot. This is an important performance optimization because otherwise, as in the specifications discussed so far, it can happen that proposers arbitrarily choose new ballots. In particular, if a proposer chooses a ballot lower than previously proposed ballots, its messages with a lower ballot are wasteful, contribute to network traffic, and cause unnecessary work for the acceptors.

To specify preemption, we (1) define predicate Preempt that specifies when and how proposers update pBal upon receiving a preempt message, (2) modify Phase1a to send 1a message with pBal, and (3) add a new case to Phases1b and Phase2b for when the acceptor a receives a ballot lower than aBal[a]. In this new case, the acceptor replies with a newly added preempt message. Preempt specifies that proposer p abandons its current ballot only after receiving a preempt message with a ballot higher than pBal[p] and that it updates pBal[p] to be a ballot higher than the ballot of the preempt message

Fig. 6 shows Preempt, and Phase1a and Phase1b with and without the modifications to add preemption. Modifications to Phase2b are similar and are in Appendix A. We also extend the definition of Messages in (16) to include the new preempt message. Adding preemption increases the width of the proof tree. Except for TypeOK, this new branch of the proof was proved by asserting invariance lemmas established earlier. The entire task of adding the new parts of specification and proof, except for the proof of TypeOK, took less than an hour.

17

---

|  NewBal(b2 ∈ B) ≜ CHOOSE b ∈ B : b > b2Preempt(p ∈ P) ≜ ∃ m ∈ msgs :∧ m.type = “preempt”∧ m.to = p∧ m.bal > pBal[p]∧ pBal' = [pBal EXCEPT ![p] = NewBal(m.bal)]∧ UNCHANGED ⟨ msgs, aBal, aVoted⟩  |   |
| --- | --- |
|  Without preemption | With preemption  |
|  Phase1a(p ∈ P) ≜ ∃ b ∈ B :∧ ≠ m ∈ msgs : ∧m.type = “1a”∧m.bal = b∧Send([type ↦ “1a”, from ↦ p, bal ↦ b])∧pBal' = [pBal EXCEPT ![p] = b]∧UNCHANGED ⟨ aBal, aVoted⟩ | Phase1a(p ∈ P) ≜∧Send([type ↦ “1a”, from ↦ p, bal ↦ pBal[p]])∧UNCHANGED ⟨ pBal, aBal, aVoted⟩  |
|  Phase1b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “1a”∧m.bal > aBal[a]∧Send([type ↦ “1b”, from ↦ a, bal ↦ m.bal, voted ↦ aVoted[a]])∧aBal' = [aBal EXCEPT ![a] = m.bal]∧UNCHANGED ⟨ pBal, aVoted⟩ | Phase1b(a ∈ A) ≜∃ m ∈ msgs :∧m.type = “1a”∧IF m.bal > aBal[a] THEN∧Send([type ↦ “1b”, from ↦ a, bal ↦ m.bal, voted ↦ aVoted[a]])∧aBal' = [aBal EXCEPT ![a] = m.bal]∧UNCHANGED ⟨ pBal, aVoted⟩ELSE∧Send([type ↦ “preempt”, to ↦ m.from, bal ↦ aBal[a]])∧UNCHANGED ⟨ pBal, aBal, aVoted⟩  |

Fig. 6. Extension of Multi-Paxos to Multi-Paxos with Preemption. We show changes in Phase1a and Phase1b. Changes to Phase2b are similar to Phase1b and omitted for brevity.

### 5.1 Remedial proof for TypeOK

The invariance of TypeOK, i.e., \(\square\) TypeOK, was automatically checked previously [7] by instructing TLAPS to use its PTL (Propositional Temporal Logic) backend prover. However, we later discovered that due to an undocumented bug in TLAPS [49], using PTL in this way causes incorrect obligations to be marked correct. This bug lets one prove that a primed formula is true if its unprimed form is an assumption. A primed formula is one with only primed variables and constants. For example, \(x' = 0\) is primed but \(x' = x + 1\) and \(x = 0\) are not. The unprimed form of \(x' = 0\) is \(x = 0\). With this bug, TLAPS incorrectly verifies TypeOK' under the assumption TypeOK \(\wedge\) [Next]\(_{vars}\).

To remedy this, we provide here the missing proof of invariance of TypeOK. Using our proof strategy, writing this proof of invariance took a matter of minutes for Multi-Paxos. The remedial proof of TypeOK is longer by 39 lines, a \(72\%\) increase from 54 for the previous incomplete proof [7] to 93, and took 3 seconds more for TLAPS to check, a \(30\%\) increase from 10 to 13 seconds.

For Multi-Paxos with Preemption, our strategy failed at first because the invariants were not strong enough. We added the conjunct \( pBal \in [\mathcal{P} \to \mathcal{B}] \) to TypeOK. Upon preemption, proposer

18

---

p changes  \( pBal[p] \)  to ballot b such that (i) no 1a message has been sent yet with b as ballot, and (ii) b is higher than the ballot of the preempting message. To prove that  \( \Box pBal \in [P \to B] \) , we need to establish that such a b indeed exists.

To this end, we strengthen our invariants by adding the fact that msgs is always a finite set, i.e., IsFiniteSet(msgs). We add this as a conjunct in TypeOK. Now, it can be proved that only a finite number of 1a messages exist. Thus, only a finite number of ballots can ever be used. Because B is infinite, there will always be some ballot available to be chosen. We prove this constructively by providing the prover with a witness: a ballot that is 1 greater than the highest ballot in 1a messages. This remedial proof of TypeOK is 63 lines longer, an 84% increase from 75 for the previous incomplete proof [7] to 138, and took 29 seconds more for TLAPS to check, a 242% increase from 12 to 41 seconds.

Table 1 summarizes the results, in the first two columns under Multi-Paxos and under Multi-Paxos with Preemption; the third columns under those are from overall proof improvement described in Section 5.2.

|  Metric | Multi-Paxos |   |   | Multi-Paxos with Preemption  |   |   |
| --- | --- | --- | --- | --- | --- | --- |
|   |  Old [7] | Remedial Proof | Improved Remedial Proof | Old [7] | Remedial Proof | Improved Remedial Proof  |
|  Proof size | 54^^ | 93 | 54 | 75^^ | 138 | 77  |
|  CPU check time (s) | 10^^ | 13 | 4 | 12^^ | 41 | 12  |
|  Elapsed check time (s) | 20^^ | 22 | 8 | 21^^ | 35 | 25  |

Table 1. Proof statistics for remedial proof of TypeOK and improvements on it. Proof size is measured as non-empty lines excluding comments.

^^ indicates an incorrect number from the incomplete proof due to the TLAPS bug [49].

### 5.2 Overall proof improvement

In Section 4 we discussed the proof of Multi-Paxos from [7] and outlined its differences with the proof for Basic Paxos. Then, in Section 5.1 we explained (1) why that proof was incorrect due to a bug in TLAPS and (2) the remedial proof. In this section, we describe three main kinds of improvements that we made to the complete proof (including the remedial proof) to make it easier to read and understand. These improvements apply to the proofs for both Multi-Paxos and Multi-Paxos with Preemption; the numbers described are for the proof of Multi-Paxos with Preemption.

(1) Refined invariants. MsgInv is now defined as a conjunction of three new predicates MsgInv1b, MsgInv2a, and MsgInv2b, in (18)-(20), which were not separate predicates in the proof for [7], even though the names were used in the text for ease of presentation. This introduction of intermediate predicates reduced the proof size by 75 lines, because, at 12 places, the new names are used in place of their expanded definitions. This also reduced the proof-checking time by about 2 seconds, because now only the portions of MsgInv needed for examination in the proof are instructed to be expanded.
(2) Merged proof cases. Cases with similar proofs are merged into a single proof. For example, to prove that AccInv continues to hold when acceptor \(a\) executes Phase1b with preemption, there are three cases to consider for each acceptor \(a2\) in \(\mathcal{A}\): (1) if \(a2\) is not \(a\), (2) if \(a2\) is \(a\), and it sends a preempt message in this action, and (3) if \(a2\) is \(a\), and it sends a 1b

19

---

message in this action. The proofs for cases (1) and (2) are similar because in both cases, a2's state does not change. We found 5 instances in the proof described in [7] where such cases were handled separately, creating unnecessarily longer proof. Merging them resulted in an overall reduction of 27 lines of proof and slightly reduced (by less than 2 seconds) proof-checking time.

(3) **Removed unnecessary obligations.** When TLAPS fails to prove an assertion, a proof must be manually written to be checked automatically by TLAPS. We discovered that parts of these manually written proofs described in [7] are unnecessary because they can be found automatically by TLAPS. About 25 instances of these were removed causing 110 lines of proof to be removed and the proof-checking times to reduce by about 20 seconds. An additional 22 instances were removed from the remedial proof for *TypeOK*, reducing the proof size by 61 lines, from 138 to 77, and the proof-checking time by 29 seconds, from 41 to 12.

### 5.3 Overall proof summary

Overall, the proof for Multi-Paxos with Preemption (about 8.5 pages total) consists of the following:

(1) Helper lemmas and their proofs (1.25 pages). These are used to prove helpful properties of operators *MaxBalInSlot*, *NewSV*, and *MaxSV*. This also includes proofs for lemmas *VotedInv* and *VotedOnce*.
(2) The invariance lemmas and their proofs (0.75 page), as explained in Section 4.6.
(3) The proof of type invariant *TypeOK* (1 page). It uses only a 1-level proof for each action.
(4) The proof of acceptor invariant *AccInv* (1.5 pages). This uses a 5-level proof for action *Phase2b* because only *Phase2b* changes variable *aVoted* which affects *AccInv*. The proofs for other actions are only 1-level because of their independence from *AccInv*.
(5) The proof of message invariant *MsgInv* (3.75 pages). This uses 4-level proofs for actions *Phase1b* and *Phase2b* taking about 1 page each, and a 6-level proof for *Phase2a* taking 2 pages, because *MsgInv* is over messages sent in these actions. The proof is long and complex in particular because each message is a tuple and its contents may contain sets of tuples, as explained in Section 4.7.
(6) The proof of theorem *Safety* (0.25 page) using *Spec* $\Rightarrow$ $\square$ *Inv* and *Inv* $\Rightarrow$ *Safe*, which together, by temporal logic, prove *Safety*.

The complete TLAPS-checked proof for Multi-Paxos with Preemption is given in Appendix C.

## 6 RESULTS OF TLAPS-CHECKED PROOFS

Table 2 summarizes the results from our specification and proof. Some mistakes in the counting logic of the result generating script caused incorrect numbers to be reported in [7] for specification and proof sizes and number of proofs by contradiction. These errors have been corrected here.

- The specification size grew by only 3 lines (6%), from 52 lines for Basic Paxos to 55 lines for Multi-Paxos; another 19 lines (35%) are added for preemption.
- Overall, the proof size increased significantly by 477 lines (154%), from 310 for Basic Paxos to 787 for Multi-Paxos; only 44 more lines (6%) were added for preemption, thanks to the reuse of all lemmas, especially invariance lemmas. As mentioned in Section 5.1, adding the remedial proof to the incomplete proof reported in [7] initially increased the proof size by 39 lines (4%) from 1010 to 1049 for Multi-Paxos and by 63 lines (6%) from 1054 to 1117 for Multi-Paxos with Preemption. However, the proof improvements decreased these numbers by 262 lines (25%) from 1049 to 787 for Multi-Paxos and by 286 lines (26%) from 1117 to 831 for Multi-Paxos with Preemption.

20

---

|  Metric | Basic Paxos | Multi-Paxos |   | Multi- with Preemption  |   |
| --- | --- | --- | --- | --- | --- |
|   |   |  Old [7] | New | Old [7] | New  |
|  Spec size (lines, excl. comments) | -, 52 | -, 56 | 55 | -, 75 | 74, 52*  |
|  Proof size (lines, excl. comments) | -, 310 | -, 1010^^ | 787 | -, 1054^^ | 831, 528*  |
|  Spec size incl. comments (lines) | 115^, 106 | 133^, 123 | 115 | 158^, 151 | 144, 76*  |
|  Proof size incl. comments (lines) | 423^, 432 | 1106^, 1096^^ | 868 | 1136^, 1143^^ | 915, 603*  |
|  Max level of proof tree nodes | 7 | 11 | 10 | 11 | 10  |
|  Max degree of proof tree nodes | 3 | 17 | 17 | 17 | 17  |
|  # lemmas | 4 | 11 | 11 | 12 | 12  |
|  # invariance lemmas | 1 | 5 | 5 | 6 | 6  |
|  # uses of invariance lemmas | 8 | 27 | 32 | 29 | 35  |
|  # proofs with set increment | 0 | 4 | 3 | 4 | 3  |
|  # proofs by contradiction | 1^, 2 | 1^, 3 | 3 | 1^, 3 | 3  |
|  # obligations in TLAPS | 239 | 918^^ | 779 | 959^^ | 825  |
|  TypeOK CPU check time (s) | -, 1 | -, 10^^ | 4 | -, 12 | 12  |
|  Total CPU check time (s) | -, 40 | -, 228^^ | 175 | -, 222^^ | 180  |
|  TypeOK elapsed check time (s) | -, 1 | -, 20^^ | 7 | -, 22^^ | 24  |
|  Total elapsed check time (s) | 24**, 14 | 128**, 63^^ | 51 | 94**, 69^^ | 90  |

Table 2. Summary of results. The check time is on a 4-core Intel i7-4720HQ 2.6 GHz CPU with 16 GB of memory, running 64-bit Ubuntu 17.10 and TLAPS 1.5.6.

- denotes an entry not in [7], followed by the now added measurement.

^ indicates a number with a count oversight in [7], followed by the now correct count.

^^ indicates an incorrect number from the incomplete proof due to the TLAPS bug [49].

** indicates a number that used TLAPS 1.5.3 in [7], followed by the number using the new version 1.5.6.

* indicates a number for the specification or proof in Appendix C, after removing unnecessary line breaks from default latex generated by TLA+ Tools.

- The maximum level of proof tree nodes increased from 7 to 10 going from Basic Paxos to Multi-Paxos but remained 10 after adding preemption; this contrast is even stronger for the maximum degree of proof tree nodes, consistent with the challenge of going to Multi-Paxos. The proof improvements reduced the maximum level of proof tree nodes by 1 for both Multi-Paxos and Multi-Paxos with Preemption compared with [7].
- The increase in number of lemmas is due to the change from Max in Basic Paxos to MaxBalInSlot in Multi-Paxos, defined in (15). Five lemmas were needed for this predicate alone to aid the prover, as we moved from two numbers to a set of 3-tuples for each acceptor.
- No proof with set increment is used for Basic Paxos. Three such proofs are used for Multi-Paxos and for Multi-Paxos with Preemption.
- Proof by contradiction is used twice in the proof of Basic Paxos, and we extended them with slots in the proofs of Multi-Paxos and Multi-Paxos with Preemption. We use proof by contradiction one more time in our proofs in lieu of longer proof by induction.
- The number of proof obligations for the provers increased most significantly, by 540 (226%), from 239 for Basic Paxos to 779 for Multi-Paxos. Only another 46 (6%) proof obligations were generated for Multi-Paxos with Preemption. Also, with our proof improvements, the number of obligations decreased by 139 (15%) from 918 in [7] to 779 for Multi-Paxos and by 134 (14%), from 959 in [7] to 825 for Multi-Paxos with Preemption.

21

---

- The checking time of invariance proof of TypeOK increased by 3 seconds (300%) from 1 for Basic Paxos to 4 for Multi-Paxos due to the more complicated structures involved in Multi-Paxos. It further increased by 8 seconds (200%) from 4 to 12 when preemption was added. This is due to new aspects in the remedial proof of TypeOK as explained in Section 5.1.
- The checking time of the total proof increased by 135 seconds (338%), from 40 for Basic Paxos to 175 for Multi-Paxos, despite our continuous efforts to help the prover reduce it. This is because of the greatly increased size and complexity of inductions, leading to significantly more obligations for the prover. A small increase of 5 seconds (3%) is observed when preemption is added. As a result of the proof improvements, we obtain a 53 second decrease (23%, from 228 to 175) and a 42 second decrease (19%, from 222 to 180) in the proof-checking time for Multi-Paxos and Multi-Paxos with Preemption.

## 7 RELATED WORK AND CONCLUSION

We discuss closest related results on verification of Paxos and related methods using TLA⁺ and TLA tools, including works using any model checking or theorem proving techniques.

Model checking. Lamport has written TLA⁺ specifications for Basic Paxos and its variants, e.g., Fast Paxos [38], and checked them using the TLA⁺ model checker TLC [29, 32, 63], but not for Multi-Paxos or its variants; a number of M.S. students at our university have also done this in course projects, including for Multi-Paxos. TLC is part of the TLA Tools [30, 32] that also includes TLAPS, and has been used in finding bugs in other models [31, 53]. Delzanno et al. [17] modeled Basic Paxos in Promela and checked it using the Spin model checker [22]. To reduce the state space, they use counting guards to track majority, reset local variables after state operations, and use sorted send instead of FIFO send (with random receive, to model non-FIFO channels). They checked Basic Paxos for pairs of numbers of proposers and acceptors up to (2,8), (3,5), (4,4), (5,3), and (8,2).

Yabandeh et al. [61] checked a C++ implementation of Basic Paxos using CrystalBall, a tool built on Mace [27], which includes a model checker. Yang et al. [62] used their model checker MoDist to check a Multi-Paxos-based service system developed by a Microsoft product team [44]. With dynamic partial-order reduction [19], they found 13 bugs including 2 bugs in the Paxos implementation, with as few as 3 replicas and a few slots.

In all cases, existing work in model checking either does not check Multi-Paxos or can check it for only a very small number of processes and slots.

Deductive verification. Kellomäki [26] formally specified and verified Basic Paxos using PVS [55], and Charron-Bost and Merz [9] verified a version of Basic Paxos using Isabelle/HOL [13]. Drăgoi et al. [18] specified and verified a version of Basic Paxos in PSync, which is based on the Heard-Of model, so the specification and proof are similar to [9, 10]. Lamport et al. [41] give a formal specification of Basic Paxos in TLA⁺ and a TLAPS-checked proof of its correctness. Lamport [39] wrote a TLA⁺ specification of Byzantine Paxos, a variant of Basic Paxos that tolerates arbitrary failures, and a TLAPS-checked proof that it refines Basic Paxos. Küfner et al. [28] exhibit a methodology to develop machine-checkable parameterized proofs of correctness of fault-tolerant round-based distributed algorithms with Basic Paxos as a case study. Their proof is approximately 10,000 lines in Isabelle/HOL. Merz, Lu, and Weidenbach [50], and Azmy, Merz, and Weidenbach [2] develop several versions of TLAPS proofs for Pastry, a distributed hash table algorithm, but do not discuss general proof strategies.

In the IronFleet project, Hawblitzel et al. [21] verified a state machine replication system that uses Multi-Paxos at its core. Their specification mimics TLA⁺ models but is written in Dafny [42],

22

---

which has no direct concurrency support but has more automated proof support than TLAPS. This work is superior to its peers by proving not only safety but also liveness properties. However, it is a complex system, with 3 levels and many components of specifications, over 1000 lines, and proofs, over 30,000 lines. Schiper et al. [58] used EventML [3] to specify Multi-Paxos and used NuPRL [14] to verify safety. Using the Verdi framework, Wilcox et al. [60] expressed Raft [54], an algorithm similar to Multi-Paxos, in OCAML and verified it using Coq [12]. The proof is over 50,000 lines and takes almost 30 minutes to verify. Padon et al. [56] specify variants of Paxos including Basic and Multi-Paxos in first-order logic. They present a methodology aiming at automatic verification based on Effectively Propositional Logic (EPR).

All these works either do not handle Multi-Paxos or handle it using more restricted or less direct language models than TLA+, some with reformulated algorithms and some mixed in large systems, making the exact algorithm indirect to see and the essence of the proof harder to find and understand.

Conclusion. This work specifies the exact phases of Multi-Paxos in a most direct way in a general language, TLA+, and develops an automatically checked proof of its safety property using TLAPS. As a general method for verifying variants of Paxos, we show precisely how to extend Lamport et al.'s specification and proof for Basic Paxos to Multi-Paxos and then to Multi-Paxos with Preemption. We further present our general strategies for proving complex properties and improving the proofs.

We have also used our method and general proof strategies in specifying and verifying other variants of Paxos, including simpler variants of Basic Paxos and Multi-Paxos specified using history variables [6], and complete executable programs for Multi-Paxos with Preemption as well as optimized programs after state reduction and failure detection [45]. The latter also allowed us to discover a safety violation, not discovered by extensive testing and model checking, in an earlier program.

Future work may provide more automated proof by induction and support the verification of more variants that improve and extend Multi-Paxos.

# ACKNOWLEDGMENTS

We thank Stephan Merz for his helpful comments on the proofs and explanations of TLAPS. We thank anonymous reviewers for their helpful comments on this work. We thank Leslie Lamport for his encouragement for this work. This work was supported in part by National Science Foundation grants CCF-1248184, CCF-1414078, CNS-1421893, and IIS-1447549, and Office of Naval Research grant N000141512208. Any opinions, findings, and conclusions or recommendations expressed in this material are those of the authors and do not necessarily reflect the views of these agencies.

# REFERENCES

[1] D. Agrawal and A. El Abbadi. 1992. The Generalized Tree Quorum Protocol: An Efficient Approach for Managing Replicated Data. ACM Trans. Database Syst. 17, 4 (Dec. 1992), 689–717. https://doi.org/10.1145/146931.146935

[2] Noran Azmy, Stephan Merz, and Christoph Weidenbach. 2018. A machine-checked correctness proof for Pastry. Science of Computer Programming 158 (June 2018), 64–80. https://doi.org/10.1016/j.scico.2017.08.003 Abstract State Machines, Alloy, B, TLA, VDM and Z (ABZ 2016).

[3] Mark Bickford, Robert L. Constable, Richard Eaton, David Guaspari, and Vincent Rahli. 2012. Introduction to EventML. Retrieved November 6, 2019 from http://www.nuprl.org/software/eventml/IntroductionToEventML.pdf

[4] Mike Burrows. 2006. The Chubby Lock Service for Loosely-coupled Distributed Systems. In Proceedings of the 7th Symposium on Operating Systems Design and Implementation (OSDI '06). USENIX Association, Berkeley, CA, USA, 335–350. http://dl.acm.org/citation.cfm?id=1298455.1298487

[5] Microsoft Research-Inria Joint Center. 2017. TLA+ Proof System (TLAPS). Retrieved September 9, 2019 from http://tla.msr-inria.inria.fr/tlaps

23

---

[6] Saksham Chand and Yanhong A. Liu. 2018. Simpler Specifications and Easier Proofs of Distributed Algorithms Using History Variables. In NASA Formal Methods (NFM '18). Springer International Publishing, Cham, Switzerland, 70–86. https://doi.org/10.1007/978-3-319-77935-5_5
[7] Saksham Chand, Yanhong A. Liu, and Scott D. Stoller. 2016. Formal Verification of Multi-Paxos for Distributed Consensus. In FM 2016: Formal Methods (FM '16). Springer International Publishing, Cham, Switzerland, 119–136. https://doi.org/10.1007/978-3-319-48989-6_8
[8] Tushar D. Chandra, Robert Griesemer, and Joshua Redstone. 2007. Paxos Made Live: An Engineering Perspective. In Proceedings of the Twenty-sixth Annual ACM Symposium on Principles of Distributed Computing (PODC '07). ACM, New York, NY, USA, 398–407. https://doi.org/10.1145/1281100.1281103
[9] Bernadette Charron-Bost and Stephan Merz. 2009. Formal Verification of a Consensus Algorithm in the Heard-Of Model. International Journal of Software and Informatics (IJSI) 3, 2-3 (2009), 273–303. https://hal.inria.fr/inria-00426388
[10] Bernadette Charron-Bost and André Schiper. 2009. The Heard-Of Model: Computing in Distributed Systems with Benign Faults. Distrib. Comput. 22, 1 (April 2009), 49–71. https://doi.org/10.1007/s00446-009-0084-6
[11] Kaustuv Chaudhuri, Damien Doligez, Leslie Lamport, and Stephan Merz. 2008. A TLA+ Proof System. In Proceedings of the LPAR Workshops, CEUR Workshop, Vol. 418. 17–37. http://ceur-ws.org/Vol-418/paper2.pdf
[12] Coq Community. 2019. The Coq Proof Assistant. Retrieved November 4, 2019 from http://coq.inria.fr/
[13] Isabelle Community. 2019. Isabelle (a generic proof assistant). Retrieved November 1, 2019 from http://isabelle.in.tum.de
[14] R. L. Constable, S. F. Allen, H. M. Bromley, W. R. Cleaveland, J. F. Cremer, R. W. Harper, D. J. Howe, T. B. Knoblock, N. P. Mendler, P. Panangaden, J. T. Sasaki, and S. F. Smith. 1986. Implementing Mathematics with the Nuprl Proof Development System. Prentice-Hall, Inc., Upper Saddle River, NJ, USA.
[15] Wikipedia contributors. 2019. Event-driven programming — Wikipedia, The Free Encyclopedia. Retrieved September 9, 2019 from https://en.wikipedia.org/w/index.php?title=Event-driven_programming&oldid=914440210
[16] Denis Cousineau, Damien Doligez, Leslie Lamport, Stephan Merz, Daniel Ricketts, and Hernán Vanzetto. 2012. TLA+ Proofs. In FM 2012: Formal Methods. Springer Berlin Heidelberg, Berlin, Heidelberg, 147–154. https://doi.org/10.1007/978-3-642-32759-9_14
[17] Giorgio Delzanno, Michele Tatarek, and Riccardo Traverso. 2014. Model Checking Paxos in Spin. In Proceedings of the 5th International Symposium on Games, Automata, Logics and Formal Verification (EPTCS). 131–146. https://doi.org/10.4204/EPTCS.161.13
[18] Cezara Drăgoi, Thomas A. Henzinger, and Damien Zufferey. 2016. PSync: A Partially Synchronous Language for Fault-tolerant Distributed Algorithms. SIGPLAN Not. 51, 1 (Jan. 2016), 400–415. https://doi.org/10.1145/2914770.2837650
[19] Cormac Flanagan and Patrice Godefroid. 2005. Dynamic Partial-order Reduction for Model Checking Software. SIGPLAN Not. 40, 1 (Jan. 2005), 110–121. https://doi.org/10.1145/1047659.1040315
[20] Sanjay Ghemawat, Howard Gobioff, and Shun-Tak Leung. 2003. The Google File System. SIGOPS Oper. Syst. Rev. 37, 5 (Oct. 2003), 29–43. https://doi.org/10.1145/1165389.945450
[21] Chris Hawblitzel, Jon Howell, Manos Kapritsos, Jacob R. Lorch, Bryan Parno, Michael L. Roberts, Srinath Setty, and Brian Zill. 2015. IronFleet: Proving Practical Distributed Systems Correct. In Proceedings of the 25th Symposium on Operating Systems Principles (SOSP '15). ACM, New York, NY, USA, 1–17. https://doi.org/10.1145/2815400.2815428
[22] Gerard Holzmann. 2011. The SPIN Model Checker: Primer and Reference Manual (1st ed.). Addison-Wesley Professional.
[23] Patrick Hunt, Mahadev Konar, Flavio P. Junqueira, and Benjamin Reed. 2010. ZooKeeper: Wait-free Coordination for Internet-scale Systems. In Proceedings of the 2010 USENIX Conference on USENIX Annual Technical Conference (USENIXATC '10). USENIX Association, Berkeley, CA, USA, 11–11. http://dl.acm.org/citation.cfm?id=1855840.1855851
[24] Amazon Web Services Inc. 2019. Amazon DynamoDB Developer Guide. Retrieved November 4, 2019 from https://docs.aws.amazon.com/en_pv/amazondynamodb/latest/developerguide/Introduction.html
[25] Michael Isard. 2007. Autopilot: Automatic Data Center Management. SIGOPS Oper. Syst. Rev. 41, 2 (April 2007), 60–67. https://doi.org/10.1145/1243418.1243426
[26] Pertti Kellomäki. 2004. An annotated specification of the consensus protocol of Paxos using superposition in PVS. Technical Report. Tampere, Finland. http://www.cs.tut.fi/ohj/laitosraportit/report36-paxos.pdf
[27] Charles Edwin Killian, James W. Anderson, Ryan Braud, Ranjit Jhala, and Amin M. Vahdat. 2007. Mace: Language Support for Building Distributed Systems. SIGPLAN Not. 42, 6 (June 2007), 179–188. https://doi.org/10.1145/1273442.1250755
[28] Philipp Küfner, Uwe Nestmann, and Christina Rickmann. 2012. Formal Verification of Distributed Algorithms: From Pseudo Code to Checked Proofs. In Proceedings of the 7th IFIP TC 1/WG 202 International Conference on Theoretical Computer Science (TCS '12). Springer-Verlag, Berlin, Heidelberg, 209–224. https://doi.org/10.1007/978-3-642-33475-7_15
[29] Markus A. Kuppe. 2017. A Verified and Scalable Hash Table for the TLC Model Checker: Towards an Order of Magnitude Speedup. Master's thesis. University of Hamburg. http://www.lemmster.de/talks/MSc_MarkusAKuppe_1497363471.pdf

24

---

[30] Markus A. Kuppe. 2018. Let TLA+ RiSE. RiSE group all-hands meeting. Retrieved November 8, 2019 from http://www.lemmster.de/talks/Let_TLA_RiSE_-_Markus_Alexander_Kuppe.pdf

[31] Markus A. Kuppe. 2019. Debuggable Design with TLA+. Hyperscale Verification. Retrieved November 8, 2019 from http://www.lemmster.de/talks/Hyperscale_Verification_-_Markus_Alexander_Kuppe.pdf

[32] Markus A. Kuppe et al. 2019. TLC Github. Retrieved November 8, 2019 from https://github.com/tlaplus/tlaplus

[33] Leslie Lamport. 1978. Time, Clocks, and the Ordering of Events in a Distributed System. Commun. ACM 21, 7 (July 1978), 558–565. https://doi.org/10.1145/359545.359563

[34] Leslie Lamport. 1994. The Temporal Logic of Actions. ACM Trans. Program. Lang. Syst. 16, 3 (May 1994), 872–923. https://doi.org/10.1145/177492.177726

[35] Leslie Lamport. 1998. The Part-time Parliament. ACM Trans. Comput. Syst. 16, 2 (May 1998), 133–169. https://doi.org/10.1145/279227.279229

[36] Leslie Lamport. 2001. Paxos made simple. ACM SIGACT News 32, 4 (Dec. 2001), 51–58. https://doi.org/10.1145/568425.568433

[37] Leslie Lamport. 2002. Specifying Systems: The TLA+ Language and Tools for Hardware and Software Engineers. Addison-Wesley Longman Publishing Co., Inc., Boston, MA, USA.

[38] Leslie Lamport. 2006. Fast Paxos. Distributed Computing 19, 2 (Oct. 2006), 79–103. https://doi.org/10.1007/s00446-006-0005-x

[39] Leslie Lamport. 2011. Byzantizing Paxos by Refinement. In Proceedings of the 25th International Conference on Distributed Computing (DISC'11). Springer-Verlag, Berlin, Heidelberg, 211–224. http://dl.acm.org/citation.cfm?id=2075029.2075058

[40] Leslie Lamport. 2012. How to write a 21st century proof. Journal of Fixed Point Theory and Applications 11, 1 (March 2012), 43–63. https://doi.org/10.1007/s11784-012-0071-6

[41] Leslie Lamport, Stephan Merz, and Damien Doligez. 2014. Paxos.tla. Retrieved February 6, 2018 from https://github.com/tlaplus/v1-tlapm/blob/master/examples/paxos/Paxos.tla

[42] K. Rustan M. Leino. 2010. Dafny: An Automatic Program Verifier for Functional Correctness. In Proceedings of the 16th International Conference on Logic for Programming, Artificial Intelligence, and Reasoning (LPAR'10). Springer-Verlag, Berlin, Heidelberg, 348–370. http://dl.acm.org/citation.cfm?id=1939141.1939161

[43] Barbara Liskov. 2010. Replication. Springer-Verlag, Berlin, Heidelberg, Chapter From Viewstamped Replication to Byzantine Fault Tolerance, 121–149. http://dl.acm.org/citation.cfm?id=2172338.2172345

[44] Xuezheng Liu, Zhenyu Guo, Xi Wang, Feibo Chen, Xiaochen Lian, Jian Tang, Ming Wu, M. Frans Kaashoek, and Zheng Zhang. 2008. D3S: Debugging Deployed Distributed Systems. In Proceedings of the 5th USENIX Symposium on Networked Systems Design and Implementation (NSDI'08). USENIX Association, Berkeley, CA, USA, 423–437. http://dl.acm.org/citation.cfm?id=1387589.1387619

[45] Yanhong A. Liu, Saksham Chand, and Scott D. Stoller. 2019. Moderately Complex Paxos Made Simple: High-Level Executable Specification of Distributed Algorithms. In Proceedings of the 21st International Symposium on Principles and Practice of Programming Languages 2019 (PPDP '19). ACM, New York, NY, USA, Article 15, 15 pages. https://doi.org/10.1145/3354166.3354180

[46] Mamoru Maekawa. 1985. A √N Algorithm for Mutual Exclusion in Decentralized Systems. ACM Trans. Comput. Syst. 3, 2 (May 1985), 145–159. https://doi.org/10.1145/214438.214445

[47] Stephan Merz. 2003. On the Logic of TLA+. Computing and Informatics 22, 3-4 (2003), 351–379. http://www.cai.sk/ojs/index.php/cai/article/view/460/367

[48] Stephan Merz. 2008. The Specification Language TLA+. Springer Berlin Heidelberg, Berlin, Heidelberg, 401–451. https://doi.org/10.1007/978-3-540-74107-7_8

[49] Stephan Merz. 2017. Coalescing bug in TLAPS. Retrieved March 16, 2018 from https://github.com/tlaplus/tlaplus/issues/40

[50] Stephan Merz, Tianxiang Lu, and Christoph Weidenbach. 2011. Towards Verification of the Pastry Protocol using TLA+. In 31st IFIP International Conference on Formal Techniques for Networked and Distributed Systems (FMOODS/FORTE 2011), Vol. 6722. 244–258. https://hal.inria.fr/inria-00593523

[51] Stephan Merz and Hernán Vanzetto. 2012. Automatic Verification of TLA+ Proof Obligations with SMT Solvers. In Proceedings of the 18th International Conference on Logic for Programming, Artificial Intelligence, and Reasoning (LPAR'12). Springer-Verlag, Berlin, Heidelberg, 289–303. https://doi.org/10.1007/978-3-642-28717-6_23

[52] Stephan Merz and Hernán Vanzetto. 2012. Harnessing SMT Solvers for TLA+ Proofs. In Proceedings of the 12th International Workshop on Automated Verification of Critical Systems (AVoCS '12), Vol. 53. European Association of Software Science and Technology, 1–15. https://doi.org/10.14279/tuj.eceasst.53.766

[53] Chris Newcombe. 2014. Why Amazon Chose TLA+. In Proceedings of the 4th International Conference on Abstract State Machines, Alloy, B, TLA, VDM, and Z - Volume 8477 (ABZ 2014). Springer-Verlag New York, Inc., New York, NY, USA, 25–39. https://doi.org/10.1007/978-3-662-43652-3_3

25

---

[54] Diego Ongaro and John Ousterhout. 2014. In Search of an Understandable Consensus Algorithm. In Proceedings of the 2014 USENIX Conference on USENIX Annual Technical Conference (USENIX ATC '14). USENIX Association, Berkeley, CA, USA, 305–320. http://dl.acm.org/citation.cfm?id=2643634.2643666
[55] Sam Owre, John M. Rushby, and Natarajan Shankar. 1992. PVS: A Prototype Verification System. In Proceedings of the 11th International Conference on Automated Deduction: Automated Deduction (CADE-11). Springer-Verlag, London, UK, UK, 748–752. http://dl.acm.org/citation.cfm?id=648230.752639
[56] Oded Padon, Giuliano Losa, Mooly Sagiv, and Sharon Shoham. 2017. Paxos Made EPR: Decidable Reasoning About Distributed Protocols. Proc. ACM Program. Lang. 1, OOPSLA, Article 108 (Oct. 2017), 31 pages. https://doi.org/10.1145/3140568
[57] Microsoft research INRIA Joint Centre. 2014. TLA+ Proof System, Tactics. Retrieved November 1, 2019 from https://tla.msr-inria.inria.fr/tlaps/content/Documentation/Tutorial/Tactics.html
[58] Nicolas Schiper, Vincent Rahli, Robbert Van Renesse, Marck Bickford, and Robert L. Constable. 2014. Developing Correctly Replicated Databases Using Formal Tools. In Proceedings of the 2014 44th Annual IEEE/IFIP International Conference on Dependable Systems and Networks (DSN '14). IEEE Computer Society, Washington, DC, USA, 395–406. https://doi.org/10.1109/DSN.2014.45
[59] Robbert Van Renesse and Deniz Altinbuken. 2015. Paxos Made Moderately Complex. ACM Comput. Surv. 47, 3, Article 42 (Feb. 2015), 36 pages. https://doi.org/10.1145/2673577
[60] James R. Wilcox, Doug Woos, Pavel Panchekha, Zachary Tatlock, Xi Wang, Michael D. Ernst, and Thomas Anderson. 2015. Verdi: A Framework for Implementing and Formally Verifying Distributed Systems. SIGPLAN Not. 50, 6 (June 2015), 357–368. https://doi.org/10.1145/2813885.2737958
[61] Maysam Yabandeh, Nikola Knežević, Dejan Kostić, and Viktor Kuncak. 2010. Predicting and Preventing Inconsistencies in Deployed Distributed Systems. ACM Trans. Comput. Syst. 28, 1, Article 2 (Aug. 2010), 49 pages. https://doi.org/10.1145/1731060.1731062
[62] Junfeng Yang, Tisheng Chen, Ming Wu, Zhilei Xu, Xuezheng Liu, Haoxiang Lin, Mao Yang, Fan Long, Lintao Zhang, and Lidong Zhou. 2009. MODIST: Transparent Model Checking of Unmodified Distributed Systems. In Proceedings of the 6th USENIX Symposium on Networked Systems Design and Implementation (NSDI'09). USENIX Association, Berkeley, CA, USA, 213–228. http://dl.acm.org/citation.cfm?id=1558977.1558992
[63] Yuan Yu, Panagiotis Manolios, and Leslie Lamport. 1999. Model Checking TLA+ Specifications. In Proceedings of the 10th IFIP WG 10.5 Advanced Research Working Conference on Correct Hardware Design and Verification Methods (CHARME '99). Springer-Verlag, London, UK, 54–66. http://dl.acm.org/citation.cfm?id=646704.702012

26

---

## A TLA* SPECIFICATION OF MULTI-PAXOS WITH PREEMPTION

MODULE MultiPaxosSpec

This is a specification in TLA+ and machine checked proof in TLAPS of Multi-Paxos with Preemption.

EXTENDS Integers, TLAPS, FiniteSets, FiniteSetTheorems

CONSTANTS P, A, Q, V Sets of proposers, acceptors, quorums of acceptors, and values to propose

|  VARIABLES msgs, | Set of sent messages  |
| --- | --- |
|  pBal, | For each proposer, the current ballot of the proposer  |
|  aBal, | For each acceptor, the highest ballot seen by the acceptor  |
|  aVoted | For each acceptor, a subset of \( \langle ballot, slot, value \rangle \) triples that the acceptor has voted  |

ASSUME QuorumAssumption \(\triangleq Q\subseteq\) SUBSET \(\mathcal{A}\wedge \forall Q1,Q2\in Q:Q1\cap Q2\neq \emptyset\)

|  \( \mathcal{B} \triangleq \mathbb{N} \) | Set of ballots  |
| --- | --- |
|  \( \mathcal{S} \triangleq \mathbb{N} \) | Set of slots  |
|  vars \( \triangleq \) | \( \langle msgs, pBal, aBal, aVoted \rangle \)  |
|  Send(m) \( \triangleq \) | \( msgs' = msgs \cup \{m\} \)  |

Phase 1a: For a proposer \( p \), this phase selects some ballot number \( pBal[p] \) with which a 1a message has not been sent, and sends it (to all processes).

|  Phase1a(p) ≜  |
| --- |
|  ∧ Send([type ↦ "1a", from ↦ p, bal ↦ pBal[p]])  |
|  ∧ UNCHANGED ⟨pBal, aBal, aVoted⟩  |

Phase 1b: For an acceptor \( a \), if there is a 1a message \( m \) with ballot \( m.bal \) that is higher than the highest it has seen, \( a \) sends a 1b message with \( m.bal \) and with the set of highest-numbered triples it has voted for each slot, and it updates the highest ballot it has seen to be \( m.bal \); otherwise it sends a preempt message back with the highest ballot it has seen.

|  Phase1b(a) ≜ ∃m ∈ msgs : m.type = "1a" ∧  |
| --- |
|  IF m.bal > aBal[a] THEN  |
|  ∧ Send([type ↦ "1b", from ↦ a, bal ↦ m.bal, voted ↦ aVoted[a]])  |
|  ∧ aBal' = [aBal EXCEPT ![a] = m.bal]  |
|  ∧ UNCHANGED ⟨pBal, aVoted⟩  |
|  ELSE  |
|  ∧ Send([type ↦ "preempt", to ↦ m.from, bal ↦ aBal[a]])  |
|  ∧ UNCHANGED ⟨pBal, aBal, aVoted⟩  |

Phase 2a: For a proposer \( p \), if there is no 2a message with current ballot \( pBal[p] \), and a quorum of acceptors has sent a set \( S \) of 1b messages with \( pBal[p] \), \( p \) sends a 2a message with \( pBal[p] \) and a set of proposals \( PropSV(T) \), where \( T \) is the union of all voted triples in messages in \( S \). \( PropSV(T) \) includes \( MaxSV(T) \), the set of slot-value pairs with the highest ballot for each slot in \( T \), and \( NewSV(T) \), a set of new slot-value pairs for slots not in \( T \).

|  MaxBSV(T) | \( \triangleq \{ t \in T : \forall t2 \in T : t2.slot = t.slot \Rightarrow t2.bal \leq t.bal \} \)  |
| --- | --- |
|  MaxSV(T) | \( \triangleq \{ [slot \mapsto t.slot, val \mapsto t.val] : t \in MaxBSV(T) \} \)  |
|  UnusedS(T) | \( \triangleq \{ s \in S : \nexists t \in T : t.slot = s \} \)  |
|  NewSV(T) | \( \triangleq \text{CHOOSE } D \subseteq [slot : UnusedS(T), val : V] : \forall d1, d2 \in D : d1.slot = d2.slot \Rightarrow d1 = d2 \)  |
|  PropSV(T) | \( \triangleq MaxSV(T) \cup NewSV(T) \)  |

|  Phase2a(p) ≜  |
| --- |
|  ∧ ∉ m ∈ msgs : (m.type = "2a") ∧ (m.bal = pBal[p])  |
|  ∧ ∃ Q ∈ Q, S ⊆ {m ∈ msgs : m.type = "1b" ∧ m.bal = pBal[p]} :  |
|  ∧ ∀ a ∈ Q : ∃ m ∈ S : m.from = a  |
|  ∧ Send([type ↦ "2a", from ↦ p, bal ↦ pBal[p], propSV ↦ PropSV(UNION {m.voted : m ∈ S})])  |
|  ∧ UNCHANGED ⟨pBal, aBal, aVoted⟩  |

Phase 2b: For an acceptor \( a \), if there is a 2a message \( m \) with ballot \( m.bal \) that is higher than or equal to the highest it has seen, \( a \) sends a 2b message with \( m.bal \) and \( m.propSV \), updates the highest ballot it has seen to \( m.bal \), and updates set of voted triples using \( m.propSV \); otherwise it sends a preempt message back with the highest ballot it has seen.

|  Phase2b(a) ≜ ∃m ∈ msgs : m.type = "2a" ∧  |
| --- |
|  IF m.bal ≥ aBal[a] THEN  |
|  ∧ Send([type ↦ "2b", from ↦ a, bal ↦ m.bal, propSV ↦ m.propSV])  |
|  ∧ aBal' = [aBal EXCEPT ![a] = m.bal]  |

27

---

\( \wedge aVoted' = [aVoted EXCEPT ![a] = \{[bal \mapsto m.bal, slot \mapsto d.slot, val \mapsto d.val] : d \in m.propSV\} \cup \{e \in aVoted[a] : \nexists r \in m.propSV : e.slot = r.slot\}] \)

\(\wedge\) UNCHANGED \(\langle pBal\rangle\)

ELSE

\( \wedge \) Send([type \( \mapsto \) "preempt", to \( \mapsto \) m.from, bal \( \mapsto \) aBal[a]])  
\( \wedge \) UNCHANGED (pBal, aBal, aVoted)

Preempt: For a proposer \( p \), if there is a preempt message \( m \) with ballot \( m.bal \) that is higher than \( p \)'s current ballot, \( p \) updates its current ballot to a new ballot that is higher than \( m.bal \) and with which no 1a message has been sent.

NewBal(b2) \(\triangleq\) CHOOSE \(b\in \mathcal{B}:b > b2\)

Preempt(p) \(\triangleq \exists m\in msgs:\)

\( \wedge m.type = "preempt" \wedge m.to = p \wedge m.bal > pBal[p] \)

\( \wedge pBal' = [pBal EXCEPT ![p] = NewBal(m.bal)] \)

\( \wedge \) UNCHANGED \( \langle msgs, aBal, aVoted \rangle \)

Init \(\triangleq msgs = \emptyset \land pBal = [p \in \mathcal{P} \mapsto 0] \land aBal = [a \in \mathcal{A} \mapsto -1] \land aVoted = [a \in \mathcal{A} \mapsto \emptyset]\)

Next \(\triangleq \vee \exists p\in \mathcal{P}:Phase1a(p)\vee Phase2a(p)\vee Preempt(p)\)

\(\vee \exists a\in \mathcal{A}:Phase1b(a)\vee Phase2b(a)\)

Spec \(\triangleq\) Init \(\wedge\) \(\square [Next]_{vars}\)

## B SAFETY PROPERTY TO PROVE FOR MULTI-PAXOS WITH PREEMPTION AND INVARIANTS USED IN PROOF

### MODULE MultiPaxosProp

VotedForIn(a, b, s, v) means that acceptor a has sent some 2b message m with m.bal equal to b and some proposal in m.propSV with slot equal to s and val equal to v. This specifies that acceptor a has voted the triple  \( \langle b, s, v \rangle \) .

VotedForIn(a, b, s, v) \(\triangleq \exists m \in msgs\):

m.type = "2b" ∧ m.from = a ∧ m.bal = b ∧ ∃ d ∈ m.propSV : d.slot = s ∧ d.val = v

ChosenIn(b, s, v) means that every acceptor in some quorum Q has voted the triple  \( \langle b, s, v \rangle \) .

ChosenIn(b, s, v) \(\triangleq \exists Q \in Q: \forall a \in Q: \text{VotedForIn}(a, b, s, v)\)

Chosen(s, v) means that for some ballot b, ChosenIn(b, s, v) holds.

Chosen(s, v) \(\triangleq \exists b\in \mathcal{B}:\) ChosenIn(b, s, v)

WontVoteIn(a, b, s) means that acceptor a has seen a higher ballot than b, and did not and will not vote any value with b for slot s.

WontVoteIn(a, b, s) \(\triangleq aBal[a] > b \land \forall v \in \mathcal{V} : \neg VotedForIn(a, b, s, v)\)

SafeAt(b, s, v) means that no value except perhaps v has been or will be chosen in any ballot lower than b for slot s.

SafeAt(b, s, v) \(\triangleq \forall b2\in 0\ldots (b - 1):\exists Q\in Q:\forall a\in Q:\text{VotedForIn}(a,b2,s,v)\lor \text{WontVoteIn}(a,b2,s)\)

Safe states that at most one value can be chosen for each slot.

Safe \(\triangleq \forall v1, v2 \in \mathcal{V}, s \in \mathcal{S}: Chosen(s, v1) \land Chosen(s, v2) \Rightarrow v1 = v2\)

Messages defines the set of valid messages. TypeOK defines invariants for the types of the variables.

Messages \(\triangleq\) [type : {"1a"}, from : \(\mathcal{P}\), bal : \(\mathcal{B}\)] ∪

[type : {"1b"}, from : A, bal : B, voted : SUBSET [bal : B, slot : S, val : V]] ∪

[type : {"2a"}, from : P, bal : B, propSV : SUBSET [slot : S, val : V]] ∪

[type : {"2b"}, from : A, bal : B, propSV : SUBSET [slot : S, val : V]] ∪

[type : {"preempt"}, to : P, bal : B]

TypeOK \(\triangleq\) \(\wedge\) msgs \(\subseteq\) Messages \(\wedge\) IsFiniteSet(msgs) \(\wedge\) pBal \(\in [\mathcal{P} \to \mathcal{B}]\)

\( \wedge aBal \in [\mathcal{A} \to \mathcal{B} \cup \{-1\}] \wedge aVoted \in [\mathcal{A} \to \text{SUBSET} [bal : \mathcal{B}, slot : \mathcal{S}, val : \mathcal{V}]] \)

Max(T) selects the largest element in nonempty set T.

Max(T) \(\triangleq\) CHOOSE \(e\in T:\forall f\in T:e\geq f\)

MaxBalInSlot(T, s) selects, among set of elements in T with slot s, the highest ballot, or -1 if no element has slot s.

28

---

MaxBalInSlot(T, s) ≜ LET E ≜ {e ∈ T : e.slot = s} IN IF E = ∅ THEN -1 ELSE Max({e.bal : e ∈ E})

MsgInv defines properties satisfied by the contents of messages, for 1b, 2a, and 2b messages.

MsgInv1b(m) ≜ ∧ m.bal ≤ aBal[m.from]
    ∧ ∀ r ∈ m.voted : VotedForIn(m.from, r.bal, r.slot, r.val)
    ∧ ∀ b ∈ B, s ∈ S, v ∈ V : b ∈ MaxBalInSlot(m.voted, s) + 1 .. m.bal - 1 ⇒
    ¬VotedForIn(m.from, b, s, v)

MsgInv2a(m) ≜ ∧ ∀ d ∈ m.propSV : SafeAt(m.bal, d.slot, d.val)
    ∧ ∀ d1, d2 ∈ m.propSV : d1.slot = d2.slot ⇒ d1 = d2
    ∧ ∀ m2 ∈ msgs : (m2.type = "2a" ∧ m2.bal = m.bal) ⇒ m2 = m

MsgInv2b(m) ≜ ∧ ∃ m2 ∈ msgs : m2.type = "2a" ∧ m2.bal = m.bal ∧ m2.propSV = m.propSV
    ∧ m.bal ≤ aBal[m.from]

MsgInv ≜ ∀ m ∈ msgs : ∧ (m.type = "1b") ⇒ MsgInv1b(m)
    ∧ (m.type = "2a") ⇒ MsgInv2a(m)
    ∧ (m.type = "2b") ⇒ MsgInv2b(m)

AccInv defines properties satisfied by the data maintained by the acceptors.

AccInv  \( \triangleq \forall a \in A: \) 

 \( \wedge aBal[a] = -1 \Rightarrow aVoted[a] = \emptyset \) 

 \( \wedge \forall r \in aVoted[a]: aBal[a] \geq r.bal \wedge VotedForIn(a, r.bal, r.slot, r.val) \) 

 \( \wedge \forall b \in B, s \in S, v \in V: VotedForIn(a, b, s, v) \Rightarrow \exists r \in aVoted[a]: r.bal \geq b \wedge r.slot = s \) 

 \( \wedge \forall b \in B, s \in S, v \in V: b > MaxBalInSlot(aVoted[a], s) \Rightarrow \neg VotedForIn(a, b, s, v) \)

Inv is the complete inductive invariant.

Inv ≜ TypeOK ∧ AccInv ∧ MsgInv

## C TLAPS CHECKED PROOF OF MULTI-PAXOS WITH PREEMPTION

MODULE MultiPaxosProof

The following 2 axioms and 10 lemmas are straightforward consequences of the predicates defined above.

AXIOM MaxInSet  \( \triangleq \forall S \in \text{SUBSET} \mathbb{N}: Max(S) \in S \) 
AXIOM MaxOnNat  \( \triangleq \forall S \in \text{SUBSET} \mathbb{N}: \nexists s \in S: Max(S) < s \)

LEMMA MaxOnNatS ≜ ∀ S1, S2 ∈ SUBSET N : S1 ⊆ S2 ⇒ Max(S1) ≤ Max(S2) BY MaxInSet

LEMMA MaxBinSType \(\triangleq \forall S\in \mathrm{SUBSET}\) [bal: \(\mathcal{B}\), slot: \(S\), val: \(\mathcal{V}\)], \(s\in S\) : MaxBalInSlot(S, s) \(\in \mathcal{B}\cup \{-1\}\) BY MaxInSet DEF MaxBalInSlot

LEMMA MaxBinSSubsets \(\triangleq \forall S1, S2 \in \text{SUBSET} [bal: \mathcal{B}, slot: S, val: \mathcal{V}], s \in S: S1 \subseteq S2 \Rightarrow\) MaxBalInSlot(S1, s) \(\leq\) MaxBalInSlot(S2, s)

<1> SUFFICES ASSUME NEW S1 ∈ SUBSET [bal : B, slot : S, val : V], NEW s ∈ S,
NEW S2 ∈ SUBSET [bal : B, slot : S, val : V], S1 ⊆ S2
PROVE MaxBalInSlot(S1, s) ≤ MaxBalInSlot(S2, s) OBVIOUS
<1>1. CASE ∉ d ∈ S1 : d.slot = s
<2>1. MaxBalInSlot(S1, s) = -1 BY <1>1 DEF MaxBalInSlot
<2>QED BY <2>1, MaxBinSType DEF B
<1>2. CASE ∃ d ∈ S1 : d.slot = s
<2>1. CASE ∉ d ∈ S2 \ S1 : d.slot = s
<3>1. MaxBalInSlot(S1, s) = MaxBalInSlot(S2, s) BY <2>1, <1>2 DEF MaxBalInSlot
<3>QED BY <3>1, MaxBinSType DEF B
<2>2. CASE ∃ d ∈ S2 \ S1 : d.slot = s BY <2>2, <1>2, MaxBinSType, MaxOnNatS DEF B, MaxBalInSlot
<2>QED BY <2>1, <2>2
<1>QED BY <1>1, <1>2

LEMMA MaxBinSNoSlot  \( \triangleq \forall S \in \text{SUBSET} [bal : B, slot : S, val : V], s \in S : (\nexists d \in S : d.slot = s) \equiv MaxBalInSlot(S, s) = -1 \) 
BY MaxInSet DEF MaxBalInSlot, B

LEMMA MaxBinSExists \(\triangleq \forall S\in \mathrm{SUBSET}\) [bal: \(\mathcal{B}\), slot: \(S\), val: \(\mathcal{V}\)], \(s\in S\) : MaxBalInSlot(S, s) \(\in \mathcal{B}\Rightarrow\) \(\exists d\in S:d.slot = s\land d.bal = MaxBalInSlot(S,s)\)   
\(\langle 1\rangle\) SUFFICES ASSUME NEW \(S\subseteq [bal:\mathcal{B},slot:S,val:\mathcal{V}],\mathrm{NEW}s\in S,\mathrm{MaxBalInSlot}(S,s)\in \mathcal{B}\) PROVE \(\exists d\in S:d.bal = MaxBalInSlot(S,s)\land d.slot = s\) OBVIOUS   
\(\langle 1\rangle 1.\exists d\in S:d.slot = s\) BY DEF MaxBalInSlot, \(\mathcal{B}\)   
\(\langle 1\rangle 2.\mathrm{MaxBalInSlot}(S,s) = \mathrm{Max}(\{d.bal:d\in \{d\in S:d.slot = s\} \})\) BY \(\langle 1\rangle 1\) DEF MaxBalInSlot

29

---

(1) QED BY  \( \langle1\rangle1,\langle1\rangle2,MaxInSet \)

LEMMA MaxBinSNoMore \(\triangleq \forall S\in \mathrm{SUBSET}\) [bal: \(\mathcal{B}\), slot: \(S\), val: \(\mathcal{V}\)], \(s\in S\) : \(\nexists d\in S:d.bal > MaxBalInSlot(S,s)\wedge d.slot = s\)

(1) SUFFICES ASSUME NEW \(S \in\) SUBSET [bal: \(\mathcal{B}\), slot: \(S\), val: \(\mathcal{V}\)], NEW \(s \in S\) PROVE \(\nexists d \in S: d.bal > MaxBalInSlot(S, s) \land d.slot = s\) OBVIOUS

(1)1. CASE \(\nexists d\in S:d.slot = s\) BY (1)1

(1)2. CASE \(\exists d\in S:d.slot = s\)

(2)1. \(\nexists b\in \{d.bal:d\in \{d\in S:d.slot = s\} \} :b > MaxBalInSlot(S,s)\) BY (1)2, MaxOnNat DEF MaxBalInSlot, \(\mathcal{B},\mathcal{S}\)

(2)2. \(\nexists d\in S:(d.slot = s\land -(d.bal\leq MaxBalInSlot(S,s)))\) BY (2)1

(2) QED BY (2)2

(1) QED BY  \( \langle1\rangle1,\langle1\rangle2 \)

LEMMA Misc \(\triangleq \forall S: \land NewSV(S) \in (\text{SUBSET} [slot : UnusedS(S), val : \mathcal{V}])\)  
\(\land \forall t1, t2 \in NewSV(S): t1.slot = t2.slot \Rightarrow t1 = t2\)  
\(\land \nexists t1 \in MaxSV(S), t2 \in NewSV(S): t1.slot = t2.slot\)

(1) SUFFICES ASSUME NEW S

PROVE \(\wedge\) NewSV(S) \(\in\) (SUBSET [slot : UnusedS(S), val : V])

\(\wedge \forall t1, t2 \in NewSV(S): t1.slot = t2.slot \Rightarrow t1 = t2\)

\(\wedge \nexists t1 \in MaxSV(S), t2 \in NewSV(S): t1.slot = t2.slot\) OBVIOUS

(1)1. \(\exists T\in\) SUBSET [slot : UnusedS(S), val : V]: \(\forall t1,t2\in T:t1.slot = t2.slot\Rightarrow t1 = t2\) BY DEF UnusedS

(1)2. NewSV(S) ∈ (SUBSET [slot : UnusedS(S), val : V]) BY (1)1 DEF NewSV

(1)3. \(\forall t1, t2 \in NewSV(S): t1.slot = t2.slot \Rightarrow t1 = t2\) BY (1)1 DEF NewSV

(1)4. \(\nexists t1\in MaxSV(S),t2\in [slot:UnusedS(S),val:\mathcal{V}]:t1.slot = t2.slot\) BY DEF MaxSV, MaxBSV, UnusedS

(1) QED BY (1)2, (1)3, (1)4

VotedInv asserts that if any acceptor \(a\) voted any triple \(\langle b, s, v \rangle\), then that triple is safe.

LEMMA VotedInv \(\triangleq\) MsgInv \(\wedge\) TypeOK \(\Rightarrow \forall a\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}\):

VotedForIn(a, b, s, v) ⇒ SafeAt(b, s, v) ∧ b ≤ aBal[a]

BY DEF VotedForIn, MsgInv, Messages, TypeOK, MsgInv2a, MsgInv1b, MsgInv2b

VotedOnce asserts that if any acceptor a1 voted triple \(\langle b, s, v1 \rangle\) and acceptor a2 voted triple \(\langle b, s, v2 \rangle\), then \(v1 = v2\).

LEMMA VotedOnce \(\triangleq\) MsgInv \(\Rightarrow \forall a1, a2 \in \mathcal{A}, b \in \mathcal{B}, s \in \mathcal{S}, v1, v2 \in \mathcal{V}\):

VotedForIn(a1, b, s, v1) ∧ VotedForIn(a2, b, s, v2) ⇒ (v1 = v2)

BY DEF MsgInv, VotedForIn, MsgInv2a, MsgInv1b, MsgInv2b

VotedUnion asserts that, in any two 1b messages' voted field, triples with the same ballot and slot have the same value.

LEMMA VotedUnion \(\triangleq\) MsgInv \(\wedge\) TypeOK \(\Rightarrow \forall m1, m2 \in msgs : m1.type = "1b" \wedge m2.type = "1b" \Rightarrow\) \(\forall d1 \in m1.voted, d2 \in m2.voted : (d1.bal = d2.bal \wedge d1.slot = d2.slot) \Rightarrow\) \(d1.val = d2.val\)

(1) SUFFICES ASSUME MsgInv, TypeOK, NEW m1 ∈ msgs, NEW m2 ∈ msgs, m1.type = "1b", m2.type = "1b", NEW d1 ∈ m1.voted, NEW d2 ∈ m2.voted, d1.bal = d2.bal, d1.slot = d2.slot
PROVE d1.val = d2.val OBVIOUS

(1)1. VotedForIn(m1.from, d1.bal, d1.slot, d1.val) BY DEF MsgInv, MsgInv1b

(1)2. VotedForIn(m2.from, d2.bal, d2.slot, d2.val) BY DEF MsgInv, MsgInv1b

(1) QED BY  \( \langle1\rangle1,\langle1\rangle2 \) , VotedOnce DEF TypeOK, Messages

The following 5 invariance lemmas assert that, for acceptor \( a \), ballot \( b \), slot \( s \), and value \( v \), and for all phases and preempt, if \( \text{VotedForIn}(a, b, s, v) \) holds then \( \text{VotedForIn}(a, b, s, v)' \) holds; for all except \( \text{Phase2b} \), the inverse also holds.

LEMMA Phase1a VotedForInv \(\triangleq\) TypeOK \(\Rightarrow \forall p\in \mathcal{P}:Phase1a(p)\Rightarrow \forall a\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:\) VotedForIn(a,b,s,v) \(\equiv\) VotedForIn(a,b,s,v)'

BY DEF VotedForIn, Send, TypeOK, Messages, Phase1a

LEMMA Phase1b VotedForInv \(\triangleq\) TypeOK \(\Rightarrow \forall a\in \mathcal{A}:Phase1b(a)\Rightarrow \forall a2\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:\) VotedForIn(a2,b,s,v) \(\equiv\) VotedForIn(a2,b,s,v)'

BY DEF VotedForIn, Send, TypeOK, Messages, Phase1b

LEMMA Phase2a VotedForInv \(\triangleq\) TypeOK \(\Rightarrow \forall p\in \mathcal{P}:Phase2a(p)\Rightarrow \forall a\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:\) VotedForIn(a,b,s,v) \(\equiv\) VotedForIn(a,b,s,v)'

BY \(\forall p\in \mathcal{P}:Phase2a(p)\Rightarrow \forall m\in msgs'\backslash msgs:m.type = "2a"\) DEF VotedForIn, Send, TypeOK, Messages, Phase2a

LEMMA Phase2b VotedForInv \(\triangleq\) TypeOK \(\Rightarrow \forall a\in \mathcal{A}:Phase2b(a)\Rightarrow \forall a2\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:\) VotedForIn(a2,b,s,v) \(\Rightarrow\) VotedForIn(a2,b,s,v)'

BY DEF VotedForIn, Send, TypeOK, Messages, Phase2b

LEMMA Preempt VotedForInv \(\triangleq\) TypeOK \(\Rightarrow \forall p\in \mathcal{P}:\) Preempt(p) \(\Rightarrow \forall a\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}\):

30

---

VotedForIn(a, b, s, v) ≡ VotedForIn(a, b, s, v)'

BY DEF VotedForIn, Send, TypeOK, Messages, Preempt

Invariance lemma SafeAtStable asserts that if SafeAt(b, s, v) holds then SafeAt(b, s, v)' holds in the next state.

LEMMA SafeAtStable \(\triangleq\) Inv \(\wedge\) Next \(\wedge\) TypeOK' \(\Rightarrow \forall b\in \mathcal{B}\), \(s\in S\), \(v\in \mathcal{V}:SafeAt(b,s,v)\Rightarrow SafeAt(b,s,v)'\)

(1) SUFFICES ASSUME Inv, Next, TypeOK', NEW b ∈ B, NEW s ∈ S, NEW v ∈ V, SafeAt(b, s, v) PROVE SafeAt(b, s, v)' OBVIOUS

(1) USE DEF Send, Inv, B

(1)1. CASE \(\exists p\in \mathcal{P}:Phase1a(p)\) BY (1)1 DEF SafeAt, Phase1a, VotedForIn, WontVoteIn

(1)2. CASE \(\exists a\in \mathcal{A}:Phase1b(a)\)

BY (1)2, QuorumAssumption DEF TypeOK, SafeAt, WontVoteIn, VotedForIn, Phase1b

(1)3. ASSUME NEW \(p \in \mathcal{P}\), Phase2a(p) PROVE SafeAt(b, s, v)'

(2)1. \(\forall a\in \mathcal{A}\), \(b2\in \mathcal{B}\), \(s2\in S\): WontVoteIn(a, b2, s2) \(\equiv\) WontVoteIn(a, b2, s2)' BY (1)3, Phase2aVotedForInv DEF WontVoteIn, Send, Phase2a

(2) QED BY (2)1, QuorumAssumption, Phase2aVotedForInv, (1)3 DEF SafeAt

(1)4. ASSUME NEW \(a \in \mathcal{A}\), Phase2b(a) PROVE SafeAt(b, s, v)'

(2)1. PICK \(m \in msgs: Phase2b(a)!(m)\) BY (1)4 DEF Phase2b

(2)2. \(\forall a2\in \mathcal{A},b2\in \mathcal{B}:aBal[a2] > b2\Rightarrow aBal'[a2] > b2\) BY (2)1 DEF TypeOK

(2)3. ASSUME NEW \(a2 \in \mathcal{A}\), NEW \(b2 \in \mathcal{B}\), NEW \(s2 \in \mathcal{S}\), NEW \(v2 \in \mathcal{V}\), WontVoteIn(a2, b2, s2), VotedForIn(a2, b2, s2, v2)', NEW \(S \subseteq [slot : S \setminus \{s2\}, val : V]\) PROVE FALSE

(3)1.  \( \exists m1 \in msgs' \setminus msgs : \land m1.type = "2b" \land m1.bal = b2 \land m1.from = a2 \land \exists d \in m1.propSV : d.slot = s2 \land d.val = v2 \) 
BY (2)3 DEF VotedForIn, WontVoteIn

(3)2. \(a2 = a \wedge m.bal = b2\) BY (2)1, (3)1 DEF TypeOK

(3) QED BY (2)1, (2)3, (3)2, (3)1 DEF Phase2b, WontVoteIn, TypeOK

(2)4. \(\forall a2\in \mathcal{A},b2\in \mathcal{B},s2\in \mathcal{S}:WontVoteIn(a2,b2,s2)\Rightarrow WontVoteIn(a2,b2,s2)'\) BY (2)2, (2)3 DEF WontVoteIn

(2) QED BY Phase2bVotedForInv, (2)4, QuorumAssumption, (1)4 DEF SafeAt

(1)5. CASE \(\exists p\in \mathcal{P}:Preempt(p)\) BY (1)5 DEF SafeAt, Preempt, VotedForIn, WontVoteIn

(1) QED BY (1)1, (1)2, (1)3, (1)4, (1)5 DEF Next

Invariant asserts the temporal formula that if Spec holds then Inv always holds.

THEOREM Invariant \(\triangleq\) Spec \(\Rightarrow \square\) Inv

(1) USE DEF B, S

(1)1. Init \(\Rightarrow\) Inv BY FS_EmptySet DEF Init, Inv, TypeOK, AccInv, MsgInv, VotedForIn

(1)2. Inv ∧ [Next] \( _{vars} \)  ⇒ Inv'

(2) SUFFICES ASSUME Inv, [Next] \( _{vars} \) PROVE Inv' OBVIOUS

(2) USE DEF Inv

(2)1. CASE Next

(3)1 proves TypeOK' for Next. Each of (4)1-4 assumes the action of a phase and proves TypeOK' for that case.

(3)1. TypeOK'

(4)1. ASSUME NEW \( p \in \mathcal{P} \), Phase1a(p) PROVE TypeOK' BY (4)1, msgs' \ msgs ⊆ Messages, FS_AddElement DEF Phase1a, TypeOK, Send, Messages

(4)2. ASSUME NEW \(p \in \mathcal{P}\), Phase2a(p) PROVE TypeOK'

(5)1. PICK \(Q \in Q\), \(S \in \text{SUBSET}\) \(\{m \in msgs : (m.type = "1b") \land (m.bal = pBal[p])\} : \land \forall a \in Q : \exists m \in S : m.from = a \land Send([type \mapsto "2a", from \mapsto p, bal \mapsto pBal[p], propSV \mapsto PropSV(UNION\{m.voted : m \in S\})])\) BY (4)2 DEF Phase2a

(5)2. UnusedS(UNION {m.voted : m ∈ S}) ⊆ S BY (5)1 DEF UnusedS, TypeOK, Messages

(5)3. MaxBSV(UNION {m.voted : m ∈ S}) ⊆ [bal : B, slot : S, val : V] BY DEF MaxBSV, TypeOK, Messages

(5)4. \(\wedge\) NewSV(UNION \(\{m.voted : m \in S\}\)) \(\subseteq\) [slot : \(S\), val : \(\mathcal{V}\)] \(\wedge\) MaxSV(UNION \(\{m.voted : m \in S\}\)) \(\subseteq\) [slot : \(S\), val : \(\mathcal{V}\)] BY (5)3, (5)2, Misc DEF MaxSV

(5)5. PropSV(UNION {m.voted : m ∈ S}) ⊆ [slot : S, val : V] BY (5)4 DEF PropSV

(5)6. \(\forall m2\in msgs'\setminus msgs:\wedge m2.type = "2a"\wedge m2.from = p\wedge m2.bal = pBal[p]\) \(\wedge m2.propSV = PropSV(UNION\{m.voted:m\in S\})\) BY (5)1, (5)5 DEF Send, TypeOK, Messages

(5)7. (msgs ⊆ Messages)' BY (4)2, (5)6, (5)1, (5)5, msgs' \ msgs ⊆ Messages DEF Phase2a, TypeOK, Send, Messages

(5) QED BY (5)7, (4)2, FS_AddElement DEF Phase2a, TypeOK, Send

(4)3. ASSUME NEW \(a \in \mathcal{A}\), Phase1b(a) PROVE TypeOK'

(5)1. PICK m ∈ msgs : Phase1b(a)!(m) BY (4)3 DEF Phase1b

(5)2. \(\vee\) msgs' = msgs \(\cup\) {type \(\mapsto\) "1b", from \(\mapsto\) a, bal \(\mapsto\) m.bal, voted \(\mapsto\) aVoted[a]]} \(\vee\) msgs' = msgs \(\cup\) {type \(\mapsto\) "preempt", to \(\mapsto\) m.from, bal \(\mapsto\) aBal[a]]} BY (5)1 DEF Send

(5)3. \(\forall m2\in msgs'\setminus msgs:\lor m2.type = "1b"\land m2.from = a\land m2.bal = m.bal\land m2.voted = aVoted[a]\)

31

---

\[
\vee m 2. t y p e = \text { "preempt" } \wedge m 2. t o = m. f r o m \wedge m 2. b a l = a B a l [ a ]
\]

BY (5)1 DEF Send

(5)4. (msgs ⊆ Messages)' BY (5)1, (5)3 DEF TypeOK, Messages, Send

(5)5. IsFiniteSet(msgs)' BY (5)2, FS_AddElement DEF TypeOK

(5) QED BY (5)4, (5)5, (4)3 DEF Phase1b, TypeOK

(4)4. ASSUME NEW \(a \in \mathcal{A}\), Phase2b(a) PROVE TypeOK'

(5)1. PICK \(m \in msgs: Phase2b(a)!(m)\) BY (4)4 DEF Phase2b

(5)2. \(\vee\) msgs' = msgs \(\cup\) {type \(\mapsto\) "2b", bal \(\mapsto\) m.bal, from \(\mapsto\) a, propSV \(\mapsto\) m.propSV}

\[
\vee m s g s ^ {\prime} = m s g s \cup \{[ t y p e \mapsto " p r e m p t", t o \mapsto m. f r o m, b a l \mapsto a B a l [ a ] ] \} B Y (5) 1 D E F S e n d
\]

(5)3. IsFiniteSet(msgs)' BY (5)2, FS_AddElement DEF TypeOK

(5)4. \(\forall m2\in msgs'\setminus msgs:\lor m2.type = "2b"\land m2.from = a\land m2.bal = m.bal\land m2.propSV = m.propSV\) \(\lor m2.type = "preempt"\land m2.to = m.from\land m2.bal = aBal[a]\)

BY (5)1 DEF Send

(5)5. (msgs ⊆ Messages)' BY (5)1, (5)4 DEF TypeOK, Send, Messages

(5)6. \(\vee a\) Voted \(= a\) Voted'

\[
\vee \wedge \text { DOMAIN } a V o t e d = \text { DOMAIN } a V o t e d ^ {\prime}
\]

\[
\wedge a V o t e d ^ {\prime} [ a ] = \{d \in a V o t e d [ a ]: \nexists d 2 \in m. p r o p S V: d. s l o t = d 2. s l o t \} \cup
\]

\[
\{[ b a l \mapsto m. b a l, s l o t \mapsto d. s l o t, v a l \mapsto d. v a l ]: d \in m. p r o p S V \}
\]

\[
\wedge \forall a 2 \in \mathcal {A} \backslash \{a \}: a V o t e d [ a 2 ] = a V o t e d ^ {\prime} [ a 2 ] B Y (5) 1 D E F T y p e O K
\]

(5)7. (a Voted ∈ [A → SUBSET [bal : B, slot : S, val : V]) BY (5)1, (5)6,

\[
\{[ b a l \mapsto m. b a l, s l o t \mapsto d. s l o t, v a l \mapsto d. v a l ]: d \in m. p r o p S V \} \subseteq [ b a l: \mathcal {B}, s l o t: \mathcal {S}, v a l: \mathcal {V} ],
\]

\[
\{d \in a V o t e d [ a ]: \nexists d 2 \in m. p r o p S V: d. s l o t = d 2. s l o t \} \subseteq [ b a l: \mathcal {B}, s l o t: \mathcal {S}, v a l: \mathcal {V} ],
\]

\[
a V o t e d ^ {\prime} [ a ] \subseteq [ b a l: \mathcal {B}, s l o t: \mathcal {S}, v a l: \mathcal {V} ] D E F T y p e O K, M e s s a g e s
\]

(5) QED BY (5)5, (5)7, (4)4, (5)3 DEF Phase2b, TypeOK

(4)5 proves TypeOK' for Preempt. This is the new part of the remedial proof described in Section 5.1.

(4)5. ASSUME NEW \(p \in \mathcal{P}\), Preempt(p) PROVE TypeOK'

(5) DEFINE \(S \triangleq \{m1 \in msgs : m1.type = "1a"\} T \triangleq \{s.bal : s \in S\} f \triangleq [s \in S \mapsto s.bal]\)

(5) HIDE DEF S, T, f

(5)1. PICK \(m \in msgs: Preempt(p)!(m)\) BY (4)5 DEF Preempt

(5)2. \(T \subseteq \mathcal{B}\) BY DEF \(T\), \(S\), TypeOK, Messages

(5)3. \(\exists b\in \mathcal{B}:b > m.bal\land b\notin T\)

\[
\text { BY } (5) 2, \text { MaxInSet }, \text { Max } (T \cup \{m. b a l \}) + 1 > m. b a l, (5) 1 \text { DEF } \text { Max }, \text { TypeOK }, \text { Messages }
\]

(5)4. NewBal(m.bal) ∈ B BY (5)3 DEF NewBal, TypeOK, Messages, T, S

(5)5. \((pBal\in [\mathcal{P}\to \mathcal{B}])'\) BY (5)1, (5)4 DEF TypeOK, Messages

(5) QED BY (4)5, (5)5 DEF Preempt, TypeOK

(4) QED BY (2)1, (4)1, (4)2, (4)3, (4)4, (4)5 DEF Next

(3)2 proves AccInv' for Next. Each of (4)1-4 assumes the action of a phase and proves AccInv' for that case.

Only Phase2b is challenging because only it updates acceptor variables.

(3)2. AccInv'

(4)1. CASE \(\exists p\in \mathcal{P}:Phase1a(p)\) BY (4)1, (3)1, Phase1aVotedForInv DEF AccInv, TypeOK, Phase1a, Send

(4)2. ASSUME NEW \(p \in \mathcal{P}\), Phase2a(p) PROVE AccInv'

(5)1. \(\forall a\in \mathcal{A},b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:VotedForIn(a,b,s,v)\equiv VotedForIn(a,b,s,v)'\)

\[
\text { BY } (4) 2, \text { Phase2aVotedForInv }
\]

(5) QED BY (3)1, (4)2, (5)1 DEF AccInv, TypeOK, Phase2a, Send, Messages

(4)3. CASE \(\exists a\in \mathcal{A}:Phase1b(a)\) BY (4)3, (3)1, Phase1bVotedForInv DEF AccInv, TypeOK, Phase1b, Send

(4)4. ASSUME NEW \(a \in \mathcal{A}\), Phase2b(a) PROVE AccInv'

(5) SUFFICES ASSUME NEW \(a2 \in \mathcal{A}'\)

\[
\text { PROVE } \quad (\wedge a B a l [ a 2 ] = - 1 \Rightarrow a V o t e d [ a 2 ] = \emptyset
\]

\[
\wedge \forall r \in a V o t e d [ a 2 ]: V o t e d F o r I n (a 2, r. b a l, r. s l o t, r. v a l) \wedge r. b a l \leq a B a l [ a 2 ]
\]

\[
\wedge \forall b \in \mathcal {B}, s \in \mathcal {S}, v \in \mathcal {V}:
\]

\[
\wedge \text { VotedForIn } (a 2, b, s, v) \Rightarrow \exists r \in a \text { Voted } [ a 2 ]: r. b a l \geq b \wedge r. s l o t = s
\]

\[
\wedge b > \text { MaxBalInSlot } (a \text { Voted } [ a 2 ], s) \Rightarrow \neg \text { VotedForIn } (a 2, b, s, v) ^ {\prime}
\]

BY DEF AccInv

(5)1. PICK \(m \in msgs: Phase2b(a)!(m)\) BY (4)4 DEF Phase2b

(5)2 assumes that \( a \) received a 2a message with a ballot lower than the highest it has seen, thus triggering preemption. This case is simple as acceptor variables are unchanged.

(5)2. CASE \((a2 = a\wedge \neg (m.bal\geq aBal[a]))\vee a2\neq a\)

(6)1. \(\forall b\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:VotedForIn(a2,b,s,v)\equiv VotedForIn(a2,b,s,v)'\) BY (3)1, (5)1, (5)2 DEF Phase2b, TypeOK, \(\mathcal{B}\), Messages, VotedForIn, Send

(6) QED BY (6)1, (5)1, (5)2, (4)4, (3)1 DEF Phase2b, Send, AccInv, TypeOK, Messages

(5)3 assumes that \( a \) received a 2a message with a ballot higher than or equal to the highest it has seen. Thus, it responds with a 2b message and updates its variables as specified. Each of (6)1-4 proves a conjunct of AccInv.

(5)3. CASE \(a2 = a \wedge (m.bal \geq aBal[a])\)

32

---

\(\langle 6\rangle 1.\) \((aBal[a2] = -1\Rightarrow aVoted[a2] = \emptyset)'\) BY \(\langle 5\rangle 3,\langle 4\rangle 4,\langle 3\rangle 1\) DEF AccInv, Phase2b, Send, TypeOK, Messages

\(\langle 6\rangle 2.\) \((\forall r\in a\text{Voted}[a2]:\text{VotedForIn}(a2,r.\text{bal},r.\text{slot},r.\text{val})\land r.\text{bal}\leq a\text{Bal}[a2])'\)

(7) SUFFICES ASSUME NEW \(r \in (a\text{Voted}[a2])'\)

PROVE (VotedForIn(a2, r.bal, r.slot, r.val) ∧ r.bal ≤ aBal[a2])' OBVIOUS

(7)1 uses two cases. (8)1 is for \( r \in a \) Voted[a2] and uses invariance lemma for Phase2b. (8)2 is for the increment \( r \in a \) Voted'[a2] \ a Voted[a2] and uses definition of Phase2b.

(7)1. VotedForIn(a2, r.bal, r.slot, r.val)'

(8)1. CASE \(r \in a\) Voted[a2]

(9)1. VotedForIn(a2, r.bal, r.slot, r.val) BY (5)3, (4)4, (8)1 DEF AccInv

(9) QED BY (9)1, Phase2bVotedForInv, (3)1, (4)4 DEF TypeOK, Messages

(8)2. CASE \(r \in a\) Voted'[a2] \ a Voted[a2]

(9)1. \(\exists m2\in msgs':m2.type = "2b"\land m2.from = a2\land m2.bal = m.bal\land m2.propSV = m.propSV\) BY (3)1, (8)2, (5)1, (5)3 DEF Send

(9)2. \(r.bal = m.bal \wedge \exists d \in m.propSV: r.slot = d.slot \wedge r.val = d.val\) BY (5)1, (5)3, (3)1, (8)2

(9) QED BY (9)1, (9)2 DEF Send, TypeOK, Messages, VotedForIn

(8) QED BY (8)1, (8)2

(7)2. \((r.bal \leq aBal[a2])'\) BY (5)1, (5)3, (3)1, \(aBal[a] \leq aBal'[a]\), \(r \in aVoted[a] \Rightarrow r.bal \leq aBal'[a]\), \(aVoted'[a] = \{d \in aVoted[a] : \nexists d2 \in m.propSV : d.slot = d2.slot\} \cup\)

\(\{[bal \mapsto m.bal, slot \mapsto d.slot, val \mapsto d.val] : d \in m.propSV\}\),

\(r\in a\text{Voted}'[a2]\backslash a\text{Voted}[a2]\Rightarrow r.bal = m.bal\) DEF AccInv, Send, TypeOK, Messages

(7) QED BY (7)1, (7)2

(6)3. (¥ b ∈ B, s ∈ S, v ∈ V : VotedForIn(a2, b, s, v) ⇒ ∃ r ∈ aVoted[a2] : r.bal ≥ b ∧ r.slot = s)'

(7) SUFFICES ASSUME NEW \(b \in \mathcal{B}'\), NEW \(s \in \mathcal{S}'\), NEW \(v \in \mathcal{V}'\), VotedForIn(a2, b, s, v)' PROVE (\(\exists r \in a\) Voted[a2]: r.bal \(\geq b \land r.\) slot = s)' OBVIOUS

(7)1. CASE VotedForIn(a2, b, s, v)

(8)1. PICK \( r \in a \) Voted[a2]: r.bal ≥ b ∧ r.slot = s BY (7)1 DEF AccInv, TypeOK, Messages

(8)2. m.bal ≥ b BY (8)1, (5)3 DEF TypeOK, Messages, AccInv

(8)3. CASE \(\exists d\in m.propSV:d.slot = s\)

(9)1. \(\exists r2\in a\text{Voted}'[a]:r2.bal = m.bal\land r2.slot = s\) BY (5)1, (5)3, \(a\text{Voted}'[a] = \{d\in a\text{Voted}[a]:\nexists d2\in m.propSV:d.slot = d2.slot\} \cup\) \(\{[bal\mapsto m.bal, slot\mapsto d.slot, val\mapsto d.val]:d\in m.propSV\}\), (8)3 DEF TypeOK

(9) QED BY (9)1, (5)3, (8)2

(8)4. CASE \(\nexists d\in m.propSV:d.slot = s\) BY (8)4, (8)1, (5)1, (5)3, \(r\in a\) Voted'[a2]

(8) QED BY (8)3, (8)4

(7)2. CASE \(\neg\) VotedForIn(a2, b, s, v)

(8)1. \(\exists d\in m.propSV:d.slot = s\) BY (5)1, (7)2 DEF Send, TypeOK, Messages, VotedForIn

(8)2. \(\exists r\in a\text{Voted}'[a2]:r.bal = m.bal\land r.slot = s\) BY (5)1, (8)1, (2)1, (5)3 DEF Send, TypeOK, Messages

(8)3. \(b = m.bal\) BY (5)1, (7)2, (5)3 DEF VotedForIn, Send

(8) QED BY (8)2, (8)3

(7) QED BY (7)1, (7)2

(6)4. (¥ b ∈ B, s ∈ S, v ∈ V : b > MaxBalInSlot(aVoted[a2], s) ⇒ ¬VotedForIn(a2, b, s, v))'

(7) SUFFICES ASSUME NEW \(b \in \mathcal{B}'\), NEW \(s \in \mathcal{S}'\), NEW \(v \in \mathcal{V}'\), (VotedForIn(a2, b, s, v)), \(b > MaxBalInSlot(aVoted'[a2], s)\) PROVE FALSE OBVIOUS

(7)1. \(\nexists d\in a\text{Voted}'[a2]:d.slot = s\land d.bal > MaxBalInSlot(a\text{Voted}[a2],s)'\) BY MaxBinSNoMore, (3)1 DEF TypeOK

(7) QED BY (7)1, (6)3, \(\exists r\in a\text{Voted}'[a2]:r.bal\geq b\land r.slot = s\), MaxBinSType, (3)1 DEF Send, TypeOK, Messages

(6) QED BY (6)1, (6)2, (6)3, (6)4

(5) QED BY (5)2, (5)3

(4)5. CASE \(\exists p\in \mathcal{P}:\) Preempt(p)

(5)1. \(\forall a\in \mathcal{A},s\in \mathcal{S}:MaxBalInSlot(aVoted[a],s) = MaxBalInSlot(aVoted[a],s)'\) BY (3)1, (4)5 DEF Preempt, MaxBalInSlot

(5) QED BY (4)5, (3)1, PreemptVotedForInv, (5)1 DEF AccInv, TypeOK, Preempt, Send

(4).QED BY (4)1, (4)2, (4)3, (4)4, (4)5, (2)1 DEF Next

(3)3 proves \(MsgInv'\) for Next. Each of \(\langle 4\rangle 1 - 4\) assumes the action of a phase and proves \(MsgInv'\) for that case.

(3)3. MsgInv'

(4)1. CASE \(\exists p\in \mathcal{P}:Phase1a(p)\) BY (4)1, Phase1aVotedForInv, (3)1, SafeAtStable, (2)1 DEF Phase1a, MsgInv, Send, TypeOK, Messages, MsgInv1b, MsgInv2a, MsgInv2b

(4)2 proves \(MsgInv'\) for \(Phase1b\). (5)13,14 conclude the proof while (5)1-12 prove intermediate facts. Each of (5)2,4,12 proves a conjunct of \(MsgInv1b\) for the increment \(m1\)-the new \(1b\) message sent in \(Phase1b(a)\).

(4)2. ASSUME NEW \(a \in \mathcal{A}\), Phase1b(a) PROVE MsgInv'

(5)1. PICK \(m \in msgs: Phase1b(a)!(m)\) BY (4)2 DEF Phase1b

33

---

(5) DEFINE m1 ≜ [type ↦ "1b", from ↦ a, bal ↦ m.bal, voted ↦ aVoted[a]]
(5)2. (m1.bal ≤ aBal[m1.from]) BY (5)1, (3)1 DEF Phase1b, Send, TypeOK, MsgInv, Messages
(5)3. m1.voted = aVoted[m1.from] BY (5)1, (3)1 DEF Phase1b, Send, TypeOK, Messages
(5)4. (∀ r ∈ m1.voted : VotedForIn(m1.from, r.bal, r.slot, r.val))'
BY (5)1, (4)2, Phase1bVotedForInv, (3)1, (5)3 DEF TypeOK, Messages, AccInv
(5)5. (∀ b ∈ B, s ∈ S, v ∈ V : b > MaxBalInSlot(m1.voted, s) ⇒ ¬VotedForIn(m1.from, b, s, v))'
BY Phase1bVotedForInv, (4)2, (5)3, (5)1, (3)2, (3)1 DEF AccInv, MsgInv, Send, TypeOK, Messages
(5)6. ∀ s ∈ S : MaxBalInSlot(m1.voted, s) ∈ B ∪ {-1} BY (3)1, MaxBinSType DEF TypeOK, Messages
(5)7. ∀ s ∈ S : ∧ MaxBalInSlot(m1.voted, s) + 1 ∈ B
∧ MaxBalInSlot(m1.voted, s) + 1 > MaxBalInSlot(m1.voted, s)
(6) SUFFICES ASSUME NEW s ∈ S
PROVE ∧ MaxBalInSlot(m1.voted, s) + 1 > MaxBalInSlot(m1.voted, s)
∧ MaxBalInSlot(m1.voted, s) + 1 ∈ B OBVIOUS
(6)1. CASE MaxBalInSlot(m1.voted, s) = -1 BY (6)1
(6)2. CASE MaxBalInSlot(m1.voted, s) ∈ B BY ∀ x ∈ B : x + 1 > x, (6)2
(6) QED BY (6)1, (6)2, (5)6
(5)9. m1.bal ∈ B BY (3)1 DEF TypeOK, Messages
(5)10. ∀ b ∈ B, s ∈ S : b ∈ MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1 ⇒ b > MaxBalInSlot(m1.voted, s)
(6) SUFFICES ASSUME NEW b ∈ B, NEW s ∈ S, b ∈ MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1
PROVE b > MaxBalInSlot(m1.voted, s) OBVIOUS
(6) HIDE DEF m1
(6) DEFINE x ≜ MaxBalInSlot(m1.voted, s) y ≜ m1.bal - 1
(6)1. x ∈ B ∪ {-1} BY (5)6
(6)2. y ∈ B ∪ {-1} BY (5)9
(6) HIDE DEF x, y
(6)3. CASE x + 1 > y
(7)1. ∀ e ∈ B : e ≠ x + 1 .. y BY (6)3, (6)1, (6)2
(7) QED BY (6)3, (7)1 DEF x, y
(6)4. CASE x + 1 = y BY (6)4, (5)7, (3)1, (5)9 DEF x, y
(6)5. CASE x + 1 < y
(7)1. ∀ e ∈ B : e ≠ x + 1 .. y ⇒ e > x BY (6)5, (6)1, (6)2
(7)2. b ∈ x + 1 .. y BY DEF x, y
(7)3. b > x BY (7)2, (7)1
(7) QED BY (7)3 DEF x
(6) QED BY (6)3, (6)4, (6)5, (6)1, (6)2
(5)11. (∀ b ∈ B, s ∈ S : b ∈ MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1 ⇒
b > MaxBalInSlot(m1.voted, s))' BY (5)10, (5)1
(5)12. (∀ b ∈ B, s ∈ S, v ∈ V : b ∈ MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1 ⇒
¬VotedForIn(m1.from, b, s, v))'
(6) SUFFICES ASSUME NEW b ∈ B', NEW s ∈ S', NEW v ∈ V'
PROVE (b ∈ MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1 ⇒
¬VotedForIn(m1.from, b, s, v))' OBVIOUS
(6)1. CASE ≜ x ∈ B : x ∈ (MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1)' BY (6)1
(6)2. CASE ≜ x ∈ B : x ∈ (MaxBalInSlot(m1.voted, s) + 1 .. m1.bal - 1)' BY (6)2, (5)5, (5)11
(6) QED BY (6)1, (6)2
(5)13 is for 1a message m having a higher ballot than the highest seen, thus generating a 1b message.
(5)13. CASE m.bal > aBal[a]
(6) SUFFICES ASSUME NEW m2 ∈ msgs'
PROVE (∧ (m2.type = "1b") ⇒ MsgInv1b(m2) ∧ (m2.type = "2a") ⇒ MsgInv2a(m2)
∧ (m2.type = "2b") ⇒ MsgInv2b(m2))' BY DEF MsgInv
Proves MsgInv1b using two cases. (7)1 is for m2 ∈ msgs. (7)2 is for the increment m2 ∈ msgs' \ msgs.
(6)1. (m2.type = "1b" ⇒ MsgInv1b(m2))'
(7)1. CASE m2 ∈ msgs BY (7)1, (5)13, (5)1, Phase1bVotedForInv, (4)2 DEF MsgInv, MsgInv1b,
TypeOK, Messages
(7)2. CASE m2 ∈ msgs' \ msgs
(8)1. m2 = m1 BY (5)1, (7)2, (5)13 DEF Send
(8) QED BY (7)2, (5)13, Phase1bVotedForInv, (4)2, (5)2, (5)4, (5)12, (5)1, (3)1, (2)1,
(8)1 DEF Send, TypeOK, MsgInv, Messages, MsgInv1b
(7) QED BY (7)1, (7)2
Proves MsgInv2a and MsgInv2b using invariance lemma for Phase1b because it does not send 2a or 2b messages.
(6)2. ((m2.type = "2a" ⇒ MsgInv2a(m2)) ∧ (m2.type = "2b" ⇒ MsgInv2b(m2)))'
BY (5)13, Phase1bVotedForInv, (5)1, (4)2, (3)1, SafeAtStable, (2)1 DEF Send, TypeOK,
MsgInv, Messages, MsgInv2a, MsgInv2b
(6) QED BY (6)1, (6)2

34

---

\(\langle 5\rangle 14\) is the simple case of preemption and uses the invariance lemmas for Phase1b and SafeAt.

(5)14. CASE \(\neg (m.bal > aBal[a])\) BY (5)14, Phase1bVotedForInv, (4)2, (5)2, (5)4, (5)12, (5)1, SafeAtStable, (3)1, (2)1 DEF Send, TypeOK, MsgInv, Messages, MsgInv1b, MsgInv2a, MsgInv2b
(5) QED BY (5)13, (5)14

(4)3 proves \(MsgInv'\) for \(Phase2a\). Each of (5)4-6 proves a conjunct of \(MsgInv\).

(4)3. ASSUME NEW \(p \in \mathcal{P}\), Phase2a(p) PROVE MsgInv'

(5) SUFFICES ASSUME NEW  \( m \in msgs' \) 

PROVE ( \( \wedge(m.type = "1b" \Rightarrow MsgInv1b(m)) \wedge (m.type = "2a" \Rightarrow MsgInv2a(m)) \) 

 \( \wedge(m.type = "2b" \Rightarrow MsgInv2b(m))' \)  BY DEF MsgInv

(5) DEFINE \(b \triangleq pBal[p]\)

(5)1. PICK \(Q \in Q\), \(S \in \text{SUBSET}\) \(\{m2 \in msgs : (m2.type = "1b") \wedge (m2.bal = b)\}\):  
\(\wedge \forall a \in Q : \exists m2 \in S : m2.from = a\)  
\(\wedge Send([type \mapsto "2a", bal \mapsto b, from \mapsto p, propSV \mapsto PropSV(UNION\{m2.voted : m2 \in S\})])\)  
BY (4)3 DEF Phase2a

(5)2. \(b = pBal'[p] \wedge b \in \mathcal{B}\) BY (4)3 DEF Phase2a, TypeOK

(5)3. \(\forall m2\in msgs'\setminus msgs:m2.type = "2a"\land m2.bal = b\) BY (5)1 DEF Send

(5)4 proves \(MsgInv1b'\). It uses invariance lemma for \(Phase2a\), because \(Phase2a\) does not send 1b messages.

(5)4. (m.type = "1b" ⇒ MsgInv1b(m))'

(6) SUFFICES ASSUME (m.type = "1b")' PROVE MsgInv1b(m)' OBVIOUS

(6)1. (m.bal ≤ aBal[m.from])' BY (4)3, (5)3, (3)1, Phase2aVotedForInv DEF TypeOK, Messages, MsgInv, Phase2a, MsgInv1b

(6)2. ( \( \forall r \in m.voted : VotedForIn(m.from, r.bal, r.slot, r.val) \) ) BY (4)3, (5)3, (3)1, Phase2aVotedForInv DEF TypeOK, Messages, MsgInv, MsgInv1b

(6)3. \(\forall s\in S:MaxBalInSlot(m.voted,s) = MaxBalInSlot(m.voted,s)'\) BY DEF MaxBalInSlot

(6)4. ( \( \forall b2 \in B, s \in S, v \in V : b2 \in MaxBalInSlot(m.voted, s) + 1 \ldots m.bal - 1 \Rightarrow \neg VotedForIn(m.from, b2, s, v) \) )

BY (4)3, (5)3, (3)1, Phase2aVotedForInv, (6)3 DEF TypeOK, Messages, MsgInv, MsgInv1b

(6) QED BY (6)1, (6)2, (6)4 DEF MsgInv1b

(5)5 proves \(MsgInv2a'\). Each of (6)2-4 proves a conjunct of \(MsgInv2a\) using the increment approach. The increment is \(m2\) in (8)2 of (7)9.

(5)5. (m.type = "2a" ⇒ MsgInv2a(m))'

(6) SUFFICES ASSUME (m.type = "2a")' PROVE MsgInv2a(m)' OBVIOUS

(6) DEFINE VS ≜ UNION {m2.voted : m2 ∈ S}

(6)1. \(\forall a\in Q:aBal[a]\geq b\) BY (5)1, (3)2, (3)1 DEF MsgInv, TypeOK, Messages, MsgInv1b

(6)2. ( \( \forall d \in m.propSV : SafeAt(m.bal, d.slot, d.val) \) )'

(7)1. \(\forall d\in [slot:UnusedS(VS),val:\mathcal{V}]:SafeAt(b,d.slot,d.val)\)

(8) SUFFICES ASSUME NEW \(d \in [slot: UnusedS(VS), val: \mathcal{V}]\) PROVE SafeAt(b, d.slot, d.val) OBVIOUS

(8)1. \(\forall m2\in S:\nexists d2\in m2.voted:d.slot = d2.slot\) BY DEF UnusedS

(8)2. \(\forall m2\in S:MaxBalInSlot(m2.voted,d.slot) + 1 = 0\) BY (8)1 DEF MaxBalInSlot

(8)3. \(\forall m2\in S,b2\in \mathcal{B},s\in \mathcal{S},v\in \mathcal{V}:b2\in MaxBalInSlot(m2.voted,s) + 1\dots m2.bal - 1\Rightarrow\) \(\neg\) VotedForIn(m2.from,b2,s,v) BY DEF MsgInv,MsgInv1b

(8)4. \(\forall v\in \mathcal{V},b2\in \mathcal{B},a\in Q:b2\in 0\dots b - 1\Rightarrow \neg\) VotedForIn(a,b2,d.slot,v) BY (5)1, (8)2, (8)3 DEF UnusedS, TypeOK, Messages

(8) QED BY (8)4, (3)1, (6)1 DEF SafeAt, NewSV, UnusedS, WontVoteIn, TypeOK, Messages

(7)2. \(\forall d\in [slot:UnusedS(VS),val:\mathcal{V}]:SafeAt(b,d.slot,d.val)'\) BY (7)1, SafeAtStable, (3)1, (2)1, (5)2 DEF NewSV, UnusedS, TypeOK, Messages

(7)3. \(\forall d\in NewSV(VS):SafeAt(b,d.slot,d.val)'\) BY (7)2, Misc DEF NewSV

(7)4. \(\forall d\in MaxBSV(VS):SafeAt(b,d.slot,d.val)\)

(8) SUFFICES ASSUME NEW \(d \in MaxBSV(VS)\), NEW \(b2 \in \mathcal{B}\), \(b2 \in 0 \ldots (b - 1)\) PROVE \(\exists Q2 \in Q: \forall a \in Q2: \lor VotedForIn(a, b2, d.slot, d.val) \lor WontVoteIn(a, b2, d.slot)\) BY DEF SafeAt

(8) DEFINE max ≜ MaxBalInSlot(VS, d.slot)

(8) USE DEF MaxBSV

(8)1. max ∈ B BY MaxBinSType, MaxBinSNoSlot DEF TypeOK, Messages

(8)2. \(\forall m2\in S:MaxBalInSlot(m2.voted,d.slot)\leq max\) BY \(\forall m2\in S:m2.voted\subseteq VS,MaxBinSSubsets\) DEF MaxBSV, TypeOK, Messages

(8)3. \(\nexists d2\in VS:(d2.bal > d.bal\land d2.slot = d.slot)\) BY \(\forall d2\in VS:\neg (\neg (d2.bal\leq d.bal)\land d2.slot = d.slot)\) DEF MaxBSV, TypeOK, Messages

(8)4. \(VS \subseteq [bal : \mathcal{B}, slot : S, val : \mathcal{V}]\) BY (3)1 DEF TypeOK, Messages

(8)5. max = d.bal

(9) SUFFICES ASSUME max ≠ d.bal PROVE FALSE OBVIOUS

(9)1. CASE max > d.bal

(10) HIDE DEF VS

35

---

(10)1. \(\exists d2\in VS:d2.bal = max\land d2.slot = d.slot\) BY (8)4, (8)1, MaxBinSExists

(10) QED BY (10)1, (8)3, (9)1

(9)2. CASE max < d.bal BY MaxBinSNoMore, (9)2 DEF MaxBSV, TypeOK, Messages

(9) QED BY (9)1, (9)2, (8)1 DEF B, TypeOK, Messages

(8)6. CASE \( b2 \in max + 1 \ldots b - 1 \)

(9) HIDE DEF max

(9)1. \(\forall m2\in S,b3\in \mathcal{B},v\in \mathcal{V}:b3\in MaxBalInSlot(m2.voted,d.slot) + 1\dots b - 1\Rightarrow\) \(\neg VotedForIn(m2.from,b3,d.slot,v)\) BY DEF MsgInv, TypeOK, Messages, MsgInv1b

(9)2. \(\forall m2\in S,v\in \mathcal{V}:\neg VotedForIn(m2.from,b2,d.slot,v)\) BY (8)6, (9)1, (8)2, (8)1, MaxBinSType DEF TypeOK, Messages, Send

(9) QED BY (5)1, (8)6, (6)1, (9)2 DEF MsgInv, MsgInv1b, TypeOK, Messages, WontVoteIn, MaxBSV

(8)7. CASE b2 = max

(9)1. \(\exists a\in \mathcal{A},m2\in S:m2.from = a\land \exists d2\in m2.voted:\) \(d2.bal = d.bal\land d2.slot = d.slot\land d2.val = d.val\) BY DEF MaxBalInSlot, TypeOK, Messages

(9)2. \(\exists a\in \mathcal{A}\) : VotedForIn(a, b2, d.slot, d.val) BY (8)7, (9)1, (8)5 DEF MsgInv, TypeOK, Messages, MsgInv1b

(9)3. \(\forall q\in Q,v2\in \mathcal{V}:VotedForIn(q,b2,d.slot,v2)\Rightarrow v2 = d.val\) BY (9)2, VotedOnce, QuorumAssumption DEF TypeOK, Messages

(9)4. \(\forall q\in Q:aBal[q] > b2\) BY (5)1 DEF MsgInv, TypeOK, Messages, MsgInv1b

(9) QED BY (8)7, (9)3, (9)4 DEF WontVoteIn

(8)8. CASE b2 ∈ 0 . . max - 1

(9)1. \(\exists a\in \mathcal{A}\) : VotedForIn(a, d.bal, d.slot, d.val) BY (8)8, (8)2 DEF MsgInv, TypeOK, Messages, MsgInv1b

(9)2. SafeAt(d.bal, d.slot, d.val) BY (9)1, VotedInv DEF TypeOK, Messages

(9) QED BY (8)8, (9)2, (8)5 DEF SafeAt, MsgInv, TypeOK, Messages, MaxBalInSlot

(8) QED BY (8)6, (8)7, (8)8, (8)1

(7)5. MaxBSV(VS) ⊆ [bal : B, slot : S, val : V] BY DEF MaxBSV, TypeOK, Messages

(7)6. \(\forall d\in MaxBSV(VS):SafeAt(b,d.slot,d.val)'\) BY (7)4, SafeAtStable, (3)1, (7)5, (2)1, (5)2

(7)7. \(\forall d\in MaxSV(VS):SafeAt(b,d.slot,d.val)'\) BY (7)6, (7)5 DEF MaxSV

(7)8. \(\forall d\in MaxSV(VS)\cup NewSV(VS):SafeAt(b,d.slot,d.val)'\) BY (7)7, (7)3

(7)9. ( \( \forall m2 \in msgs : m2.type = "2a" \Rightarrow \forall d \in m2.propSV : SafeAt(m2.bal, d.slot, d.val) \) )

(8) SUFFICES ASSUME NEW \(m2 \in msgs'\), \((m2.type = "2a")'\), NEW \(d \in m2.propSV\) PROVE (SafeAt(m2.bal, d.slot, d.val))' OBVIOUS

(8)1. CASE \(m2 \in msgs\) BY (3)1, SafeAtStable, (8)1, (2)1 DEF MsgInv, MsgInv2a, Messages, TypeOK

(8)2. CASE \(m2 \in msgs' \backslash msgs\)

(9)1. SafeAt(m2.bal, d.slot, d.val)' BY (7)8, (8)2, (5)1, (5)2 DEF Send, PropSV

(9) QED BY (3)1, (9)1 DEF Send, TypeOK, Messages

(8) QED BY (8)1, (8)2

(7) QED BY (7)9

The increment is \(m2\) in (7)1.

(6)3. ( \( \forall d1, d2 \in m.propSV : d1.slot = d2.slot \Rightarrow d1 = d2 \) )

(7)1. \(\forall m2\in msgs'\backslash msgs:\forall d1,d2\in m2.propSV:d1.slot = d2.slot\Rightarrow d1 = d2\)

(8)1. VS ∈ SUBSET [bal : B, slot : S, val : V] BY DEF Messages, TypeOK

(8)2. \(\forall r1, r2 \in MaxBSV(VS): r1.slot = r2.slot \Rightarrow r1.bal = r2.bal\) BY (8)1 DEF MaxBSV

(8)3. MaxBSV(VS) ⊆ VS BY (8)1 DEF MaxBSV

(8)4. \(\forall r1, r2 \in MaxBSV(VS): r1.bal = r2.bal \land r1.slot = r2.slot \Rightarrow r1.val = r2.val\) BY (8)3, VotedUnion

(8)5. \(\forall r1, r2 \in MaxBSV(VS): r1.slot = r2.slot \Rightarrow r1.bal = r2.bal \land r1.val = r2.val\) BY (8)4, (8)2, (8)3, (8)1

(8)6. \(\forall r1, r2 \in MaxSV(VS): r1.slot = r2.slot \Rightarrow r1 = r2\) BY (8)5 DEF MaxSV

(8) QED BY (8)6, Misc, (5)1 DEF PropSV, Send

(7) QED BY (7)1 DEF MsgInv, MsgInv2a

The increment is \(m1, m2\) in (7)2.

(6)4. ( \( \forall m2 \in msgs : (m2.type = "2a" \land m2.bal = m.bal) \Rightarrow (m2 = m) \) )'

(7)1. \(\forall m2\in msgs:(m2.type = "2a")\Rightarrow (m2.bal\neq b)\) BY (4)3 DEF Phase2a

(7)2. \(\forall m1, m2 \in msgs' \backslash msgs: m1 = m2\) BY (4)3 DEF Phase2a, Send

(7) QED BY (7)1, (7)2, (5)3, (2)1, (3)1 DEF MsgInv, MsgInv2a

(6) QED BY (6)2, (6)3, (6)4 DEF MsgInv2a

(5)6. ((m.type = "2b") ⇒ MsgInv2b(m))' BY (5)3, (5)1, m.type = "2b" ⇒ m ∈ msgs, (3)1, (4)3 DEF TypeOK, Messages, MsgInv, Phase2a, Send, MsgInv2b

(5) QED BY (5)4, (5)5, (5)6

(4)4 proves MsgInv' for Phase2b. Each of (5)2-4 proves a conjunct of MsgInv.

36

---

\(\langle 4\rangle 4\) .ASSUME NEW \(a\in \mathcal{A}\) Phase2b(a)PROVE MsgInv'

(5) SUFFICES ASSUME NEW m ∈ msgs'

PROVE ( \( \wedge(m.type = "1b") \Rightarrow MsgInv1b(m) \wedge (m.type = "2a") \Rightarrow MsgInv2a(m) \) )

\( \wedge(m.type = "2b") \Rightarrow MsgInv2b(m)' BY DEF MsgInv \)

(5)1. PICK \(m1 \in msgs: Phase2b(a)!(m1)\) BY (4)4 DEF Phase2b

(5)2 proves \(MsgInv1b'\) for Phase2b. Invariance lemmas do not apply because the 3rd conjunct in \(MsgInv1b\) quantifies over \(2b\) messages negatively - VotedForIn(a, b, s, v) means acceptor a has sent a 2b message voting \(\langle b, s, v \rangle\).

(5)2. ((m.type = "1b") ⇒ MsgInv1b(m))'

(6) SUFFICES ASSUME (m.type = "1b") PROVE MsgInv1b(m)' OBVIOUS

(6)1. (m.bal ≤ aBal[m.from] ∧ ∀ r ∈ m.voted : VotedForIn(m.from, r.bal, r.slot, r.val)')
BY (5)1, (3)1, (4)4, Phase2bVotedForInv DEF MsgInv, MsgInv1b, TypeOK, Messages, Send

(6)2. ( \( \forall b \in B, s \in S, v \in V : b \in MaxBalInSlot(m.voted, s) + 1 \ldots m.bal - 1 \Rightarrow \neg VotedForIn(m.from, b, s, v) \) )'

(7) SUFFICES ASSUME NEW \( b \in \mathcal{B}' \), NEW \( s \in \mathcal{S}' \), NEW \( v \in \mathcal{V}' \), (b ∈ MaxBalInSlot(m.voted, s) + 1 .. m.bal - 1)' PROVE (¬VotedForIn(m.from, b, s, v))' OBVIOUS

(7)1. ¬VotedForIn(m.from, b, s, v) BY (5)1 DEF Send, MsgInv, TypeOK, Messages, MsgInv1b

(7)2. CASE m.from ≠ a ∨ ¬(m1.bal ≥ aBal[a]) BY (5)1, (3)1, (7)2, (7)1 DEF VotedForIn, TypeOK, Messages, Send

(7)3. CASE m.from = a ∧ (m1.bal ≥ aBal[a])

(8)1. ∀ m2 ∈ msgs' \ msgs : m2.bal = m1.bal BY (5)1, (7)3, (6)1, (7)3 DEF Send, TypeOK

(8)2. ∀ m2 ∈ msgs' \ msgs : m2.bal ≠ b BY (5)1, (7)3, (6)1, (7)3, (8)1 DEF TypeOK, Messages

(8) QED BY (7)3, (7)1, (8)2 DEF VotedForIn, TypeOK, Messages

(7) QED BY (7)2, (7)3 DEF TypeOK, Messages

(6) QED BY (6)1, (6)2 DEF MsgInv1b

(5)3. ((m.type = "2a") ⇒ MsgInv2a(m))' BY SafeAtStable, (3)1, (4)4, (2)1 DEF MsgInv, MsgInv2a, TypeOK, Messages, Phase2b, Send

(5)4 proves \(MsgInv2b'\). It uses two cases: the second case, (6)2, is for the increment \(m\).

(5)4. ((m.type = "2b") ⇒ MsgInv2b(m))'

(6)1. CASE \(\neg(m1.bal \geq aBal[a]) \lor m \in msgs\) BY (5)1, (3)1, (6)1 DEF TypeOK, Messages, Send, MsgInv, MsgInv2b

(6)2. CASE m1.bal ≥ aBal[a] ∧ m ∈ msgs' \ msgs BY (5)1, (3)1, (6)2 DEF TypeOK, Send, MsgInv2b

(6) QED BY (6)1, (6)2

(5) QED BY (5)2, (5)3, (5)4 DEF MsgInv2b

(4)5 proves \(MsgInv'\) for Preempt. It uses the invariance lemma for Preempt, since Preempt does not send messages.

(4)5. CASE \(\exists p\in \mathcal{P}:\) Preempt(p) BY (4)5, PreemptVotedForInv, (3)1, SafeAtStable, (2)1 DEF Preempt, MsgInv, TypeOK, Messages, MsgInv1b, MsgInv2a, MsgInv2b

(4) QED BY (4)1, (4)2, (4)3, (4)4, (4)5, (2)1 DEF Next

(3) QED BY (3)1, (3)2, (3)3 DEF Inv, vars, Next

(2)2. CASE UNCHANGED vars BY (2)2 DEF vars, Inv, TypeOK, AccInv, MsgInv, VotedForIn, SafeAt, WontVoteIn, MaxBalInSlot, MsgInv1b, MsgInv2a, MsgInv2b

(2) QED BY (2)1, (2)2

(1) QED BY (1)1, (1)2, PTL DEF Spec

Safety asserts that Spec implies that Safe always holds.

THEOREM Safety \(\triangleq\) Spec \(\Rightarrow\) \(\square\) Safe

(1) USE DEF B

(1)1. Inv \(\Rightarrow\) Safe

(2) SUFFICES ASSUME Inv, NEW v1 ∈ V, NEW v2 ∈ V, NEW s ∈ S, NEW b1 ∈ B, NEW b2 ∈ B, ChosenIn(b1, s, v1), ChosenIn(b2, s, v2), b1 ≤ b2
PROVE v1 = v2 BY DEF Safe, Chosen

(2)1. CASE b1 = b2

(3)1. \(\exists a\in \mathcal{A}:VotedForIn(a,b1,s,v1)\wedge VotedForIn(a,b1,s,v2)\) BY (2)1, QuorumAssumption DEF ChosenIn

(3) QED BY (3)1, VotedOnce DEF Inv

(2)2. CASE b1 < b2

(3)1. SafeAt(b2, s, v2) BY VotedInv, QuorumAssumption DEF ChosenIn, Inv

(3)2. PICK \(Q1 \in Q: \forall a \in Q1: VotedForIn(a, b1, s, v1)\) BY DEF ChosenIn

(3)3. PICK \(Q2 \in Q: \forall a \in Q2: VotedForIn(a, b1, s, v2) \lor WontVoteIn(a, b1, s)\) BY (3)1, (2)2 DEF SafeAt

(3) QED BY (3)2, (3)3, QuorumAssumption, VotedOnce DEF WontVoteIn, Inv

(2) QED BY (2)1, (2)2

(1) QED BY Invariant, (1)1, PTL

37