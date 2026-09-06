# One More Frown Please? (UPaxos Quorum Overlaps)

Source: https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/
Author: simbo1905 (slash dev slash null)
Fetched: 2026-09-05 via tavily extract (markdown), gaps filled from the live DOM
via chrome-devtools (the original had inline math rendered as HTML text, not
images — zero <img> elements exist in .entry-content). Site boilerplate removed.

---

There was some discussion around UPaxos safety on a [gist](https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1) where Dave Turner was kind enough to clarify a confusion of mine. I had said that we needed an overlap between prepare quorums to avoid a split-brain. This was incorrect and I am greatful for Dave for correcting my misunderstanding. Yet there was something about not having that overlap that was bugging me… This morning I had an "Aha!" moment: if it exists then Trex will perform an optimisation. Yet in Trex this overlap is not enforced.

First lets do a recap of the safety requirement of UPaxos which is:

QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1

Lets break that down by first discussing the "frown operator":

Qx ⌢ Qy

Each Q is some set of set of quorums. The ⌢ symbol is stating an invariant that a safe combination of quorums taken from the two available sets of quorums must overlap. An actual instance of a quorum set is written q ∈ Q. That states that qx is a quorum that is a member of the set of legal quorums Qx. When we are talking about overlaps of q we are talking about regular set intersections qx ∩ qy which you might compute in scala with "x.intersect(y)". When we are talking about majority overlaps in a three node cluster there are permutations of sets that would be valid. For example:

{1,3} ∩ {1,2} = {1}
{2,3} ∩ {2,1} = {2}
{3,1} ∩ {3,2} = {3}

We can talk about the legal quorums as satisfying:

qx ∩ qy **≠** ∅

Where ∅ is the empty set.

Qx ⌢ Qy is simply generalising over qx ∩ qy to assert an invariant that the intersection of all legal combinarions of qx ∈ Qx and qy ∈ Qy are not empty.

A real leader will act as soon as it sees some actual set of votes that convinces it that it is safe to send out the next message. So in practice the actual q's used to make decisions at the leader will depend on the arrival order of response messages at the leader. Additional messages may arrive after a leader is satisfied that it is safe to move forward. Trex simply ignores any late messages.

We write QI to mean the set of valid "phase one" quorums that are seen in response to prepare messages. QII is the set of valid "phase two" quorums seen in response to accept messages. QI ⌢ QII is basically saying that a leader must wait until it sees enough messages that confirm qI∩qII**≠**∅

The invariant QI ⌢ QII is simply the [FPaxos](https://simbo1905.blog/2016/09/30/the-fpaxos-even-nodes-optimisation/) result. The intuitive explaination of Paxos is that the new leader collaborates with the old leader by choosing the same value. That can only happen if the accept quorum of the dead leader overlaps the prepare quorum of the new leader. The new leader will then see the value chosen by the dead leader and finish the work. This is why Paxos such a beautifully simple algorithm. Once you understand that is a simple collaboration algorithm it is very easy to impliment. An implimenation of a concurrent list in the core Java libraries has an analogous trick of racing threads "choosing to collaborate" and complete each others work. With a concurrent distributed algorithm such as Paxos the state spaces is far more complex. This makes writing a bug free implimenation more challenging. That is because distributed systems are inherently more complex and is not because Paxos is in any way mechanically complicated.

Now that we understand the frown operator is we can unpick the UPaxos safety equation. Here it is again:

QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1

Dave clarifies that Q ⌢ Q is not transitive. This means that as we only see three ⌢ that express three invariants. So we we need only three quorum overlaps. Qe is some set of quorums before a reconfiguration. Qe+1 is some set of quorums after the reconfiguration. Two overlaps are simply saying that FPaxos rules apply both before and after the reconfiguration:

QIIe ⌢ QIe
QIIe+1 ⌢ QIe+1

The third one is:

QIe ⌢ QIIe+1

That one asserts that FPaxos rules apply across the reconfiguration.

Where I made a mistake on the gist at the link above was in asserting that we needed one more frown to avoid a split-brain:

QIe ⌢ QIe+1

That frown is ensuring that leadership elections between eras have an overlap. Dave corrected my misunderstanding. That overlap of prepare quorums is **not** required to maintain safety. The overlaps between prepare and accepts alone maintain safety. Yet you **can** have that overlap with some configurations such as simple majority quorums in a three node cluster.

Something was bugging me about that unnecessary overlap. The next morning I realised why. Trex impliments FPaxos and will optionally use this overlap in an optimisation to try to reduce the occurance of unnecessary leadership shootouts!

David highlighted a simple two node case where not having this overlap would be okay:

> This actually isn't needed for safety, similarly to FPaxos. The following (no-op) configuration change is fine and does not satisfy QIe ⌢ QIe+1:
>
> QIe = {{1},{2}}
>
> QIIe = {{1,2}}
>
> QIe+1 = {{1},{2}}
>
> QIIe+1 = {{1,2}}
>
> — David C. Turner

We can see that there are the three necessary safety overlaps. The missing QIe ⌢ QIe+1 overlap is because picking from the above permutations of the sets of valid quorums we can encouter:

{1} ∩ {2} **=** ∅

That is saying we have two possible leaders in each era who don't even need to exchange any messages to get a phase 1 quorum to believe that they are a leader. That isn't a safety issue. With only two nodes there isn't much of a problem. The problem is when you have three nodes and a network parition between one node and the leader.

With Trex if we had three nodes and a partition between one node and a leader then the node will timeout. It would then issue a prepare message. This could be accepted by the third node. If the ballot number of the prepare message was high enough to assume the leadership it would unnecessarily interrupt an otherwise stable leader. There could be a flip-flop effect where the leadership swaps between the two nodes that cannot exchange messages. This would harm the throughput of the system.

To help reduce such leadership shootouts Trex has an optimisation called the "[low-ball prepare](https://simbo1905.blog/2017/08/22/pre-voting-in-distributed-consensus/)". The optimsation is as follows:

In addition to the mandatory messages Trex uses negative acknowledgments (aka "nacks").
Nack includes a heartbeat from any stable leader, the highest slot number that has been accepted, and the highest promise number that has been made.
A potential leader first sends a prepare with a very low ballot number. This will never interrupt a stable leader.
Only if the a quorum of nack to the "low-ball" has no suggestion that there is a stable leader will the node issue a fresh prepare with a high ballot number.

This is an entirely optional performance optimisation intented to reduce the potential of cluster instability due to leadership shootouts. Once again there is no requirement for such messages to arrive. If all nack messages are lost that is fine. It is only the standard messages that must get through for saftey.

It appeared to me that this optimisation wouldn't be possible during UPaxos reconfiguration if there was no overlap between the phase 1 quorums of the two eras. Yet would have risked a leadership shootout between eras? Probably not as in the Trex sketch of UPaxos the era was encoded into the most significant bits of the ballot number. A leader issuing prepare messages in the new era e+1 would never be disrupted by an attempt to become the leader issued in any prior era. Any node that had promised to a leader in the latest era would ignore a prepare message from a leader operating in any prior era. So there really would be no practical loss of throughput with not having the QIe ⌢ QIe+1 overlap.

In steady state FPaxos many typical configurations would have this "additional frown". If there is no such overlap that is okay in terms of safety. Trex uses exponentially increasing randomised back-offs to try to resolve any leadership shootouts that occur. As long as you have the necessary frowns then safety will be assured. One more overlap is helpful for Trex to avoid leadership shootouts leading to increasing randomised back-offs. Yet this isn't in anyway enforced so it isn't an invariant of either the algorithm nor the implimentation.

Once again the "low-ball ballot" optimisation is simply an attempt to propagate additional information around the cluster to avoid leadership shootouts. This is entirely optional information and safety is ensured as long as the required messages get through.

In the comments Dave helpfully points out that FPaxos allows for grid quorums. Imaging nine nodes arranged three by three. You can define a legal Phase I quorum is any completed row in the grid and a legal Phase II quorum as any column. All messages can be sent to all nine servers. The leader can then play a game of bingo where they "win" if they get a Phase I quorum that complete any one column and a Phase II quorum if they complete any one row. With that setup Phase I quorums wont overlap as different rows are equality valid. As it stands Trex currently only supplies one default quorum strategy that is based on simple majorities. That strategy includes the FPaxos even nodes optimisation discussed [here](https://simbo1905.blog/2016/09/30/the-fpaxos-even-nodes-optimisation/).

---

## Comments

**davecturner:** If Trex relies on Q(I,e) ⌢ Q(I,e+1) then I think it also requires Q(I,e) ⌢ Q(I,e) which is the same constraint when there's no configuration change in progress. This is quite a tough constraint since non-intersecting quorums can be powerful (e.g. grid quorums in the FPaxos paper). However, I don't think this is actually the right constraint for you.

It's certainly tricky to make sure you're advancing the ballot the right amount: if it doesn't advance enough then you risk deadlock, but if it advances too much then you risk livelock. Trex's "lowball prepare" messages are very much related to prevoting (covered in your previous post) which waits for responses from a quorum of nodes before starting a new ballot, but this very much raises the question of which quorum we mean: phase I or phase II, and in which era? Turns out I didn't specify that in my post on prevoting.

I think in practice you want prevoting to wait for responses that forms _both_ a phase I and a phase II quorum, in the era corresponding with the first uncommitted instance, since this is the set of nodes you need to cooperate in order to commit the next instance so there's no point in continuing unless you can contact such a set of nodes. There's no need to worry about quorums from other eras in prevoting: cross-era quorums are helpful for avoiding a pipeline stall, but if we're prevoting then we've already hit a timeout so the pipeline probably already stalled.

I also think it's better to implement this in the prevoting phase itself rather than expressing it as a constraint on the quorums themselves.

**simbo1905:** It isn't actually a constraint in that it isn't actually enforced anywhere. So my language is being imprecise. If there is no such overlap then no additional information is seen and the optimisation of not entering a leadership shootout. A shootout then happens which is resolved via the typical process of increasing randomised backoffs until we stable leader emerges. I will update the blog to be more exact.

**simbo1905:** I think that in the past we discussed that prevoting was equivalent to the approach that Trex takes which is exponential backoff plus eventual consistency of nacks propagating information about the cluster in an attempt to avoid shootouts. So yes indeed there is no enforce invariant rather some optimisations in the eventual consistency and exponential backoff approach of Trex A prevoting phase is probably superior in terms of making a fast recovery from failures. So prevoting is probably the way to go in low latency scenarios. In sloppy timeouts and exponential backoffs are okay then Trex's approach will work.
