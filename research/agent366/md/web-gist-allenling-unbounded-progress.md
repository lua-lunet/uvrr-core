<!-- source: https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1 -->

Instantly share code, notes, and snippets.

![@allenling](https://avatars.githubusercontent.com/u/2723570?s=64&v=4)

# [allenling](/allenling)/**[upaxos\_progress.md](/allenling/99bf0e965fa7e0b208f461446fcc97e1)**

# Select an option

## No results found

# Select an option

## No results found

# writing commands into qIIe+1?

fig.3 in upaxos paper is not a general situation as setting {leader, a1} = qIe = qIe+1
and {leader, a2} = qIIe = qIIe+1.

lets split them up and say N1=6, qIe={1, 2, 3, 4, 5}, qIIe={5, 6}, Leader=5

qIe  
1 2 3 4  
              5 (L)  
         6  
         qIIe

and then we move from e to e+1, say N2=3, qIe+1={5, 7}, qIIe+1={5, 8}, we have qIe and qIIe+1 intersect.

qIe  
1 2 3 4              7  qIe+1  
               5  
         6              8  qIIe+1  
        qIIe

what happen when cluster entering split mode? according to the upaxos paper:

"While a change from era e to e + 1 is in progress some instances may use the interim configuration <QIe, QIIe+1>"

**leaer 5 should write client commands into qIIe+1 quorums, which would be {8}, even the e(b) of node 8 is not known yet.**

after fix reconfiguration at slot s+1, leader 5 crash before it send a b` prepare to qIe+1

then {1, 2, 3, 4, 6} would elect a leader , lets say 6, then new leader 6 would join {7, 8}, and would find all client commands.

# construct the "casting vote"

what if nodes in q(I, e+1) going to elect a leader when leader down?

lets say N2=4, qIe+1={5, 7, 8}, qIIe+1={5, 9}

qIe  
1 2 3 4        7,  8  qIe+1  
              5  
         6        9      qIIe+1  
        qIIe

after fix reconfiguration at slot s+1, leader 5 crash before it send a b` prepare request to qIe+1

**nodes {7, 8, 9} could elect a new leader because the amount of nodes in e+1 satisfy the required amount of q(I, e+1), which is 3.**

and in other side, {1, 2, 3, 4, 6} would elect 6 to be leader, and 6 would see the reconfiguration in slot s+1, and join {7, 8, 9}, finally we got two leaders

and you says:

"Once the leader knows the new era is fixed at slot s+1 it needs to upgrade to a fresh ballot number in the new era. It does this by having a majority of nodes promise to a new ballot number in the new era."

and

"It does this by acting last within the prepare quorum. It defers from making a promise to its own new ballot number until it knows that sufficient other nodes have also made promises to give it the 'casting vote.'"

**in this situation, majority does not helps, and how to construct the "casting vote"?**

![@DaveCTurner](https://avatars.githubusercontent.com/u/5058284?s=80&v=4)

### **[DaveCTurner](/DaveCTurner)** commented [May 21, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3312461#gistcomment-3312461) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

Hi there, thanks for taking an interest in UPaxos, and thanks so much for the improved formatting here. I'll see what I can do to help.

Your first question doesn't really typecheck:

QIe={1, 2, 3, 4, 5}, QIIe={5, 6}

These QIe things are *sets of sets of nodes* but you have declared them to be sets of nodes. The resilience of Paxos comes from having the freedom to use lots of different subsets of the nodes for each phase. I can think of a couple of few different interpretations but the meaning of your question depends a great deal on which fix is the one you intended. Could you adjust this to remove the ambiguity? For instance, you may mean

QIe=majorities({1, 2, 3, 4, 5}) = {{1,2,3},{1,2,4},{1,2,5},{1,3,4},{1,3,5},{1,4,5},{2,3,4},{2,3,5},{2,4,5},{3,4,5},{1,2,3,4},{1,2,3,5},{1,2,4,5},{1,3,4,5},{2,3,4,5},{1,2,3,4,5}}  
QIIe=majorities({5, 6}) = {{5,6}}

However this isn't valid because it does not satisfy QIe ⌢ QIIe since e.g. {1,2,3} ∩ {5,6} = ∅.

For your second question:

the amount of nodes in e+1 satisfy the required amount of Q(I, e+1), which is 3.

Again the same ambiguity, but there's another misunderstanding here too. It's not the *amount* of nodes that matters, the identities of the nodes are important. Some sets of 3 nodes may be a quorum while other sets of 3 nodes are not. I think I'll be able to answer more clearly if you adjust your question so that all the Qs are sets of sets of nodes.

how to construct the "casting vote"?

This is described in detail on page 6 of the paper. In particular note that a node with a casting vote does not always exist, especially if some nodes have failed or you have chosen the quorums badly. If there's no node with a casting vote then the algorithm still makes progress, but it will have to stop accepting proposals for the short period of time until the reconfiguration is complete.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@allenling](https://avatars.githubusercontent.com/u/2723570?s=80&v=4)

### **[allenling](/allenling)** commented [May 21, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3312690#gistcomment-3312690)

[@DaveCTurner](https://github.com/DaveCTurner) wow, thanks for reply!

yes we could freely choose any majority subsets for each phase, like N={1, 2, 3, 4, 5} and choose {1, 2, 3}, {2, 3, 4}, ... for phase-I, and {2, 4, 5}, {1, 2, 5}, ... for phase-II.

**and as flexible paxos says, it is only necessary for phase 1 quorums (Q1) and phase 2 quorums (Q2) to intersect. means (a) we could freely choose Q1 and Q2 just make sure Q1 ⌢ Q2 or len(Q1) + len(Q2) > len(N), and (b) we can continue to safely to select a new leader if we have enough nodes to form Q1 quorum.**

so we says N={1, 2, 3, 4, 5, 6}, len(N)=6, and leader 5 would send prepare request to {1, 2, 3, 4, 5}, which would be QIe, and send accept request to {5, 6}, which would be QIIe. we have QIe ⌢ QIIe = {5}, and len(QIe) = 5, len(QIIe) = 2.

we choose {5, 6} for phase-II because 6 is the fastest acceptor for leader 5, then we could have a good performance, or because acceptor 6 is equipped with a faster hardware, like a SSD.

or maybe leader 5 write current command into acceptor 6 coincidently. QIIe = {5, 6} does not means we can not choose other nodes to form QIIe.  
here we say QIIe = {5, 6}, and QIe = {1, 2, 3, 4, 5} for simplicity

QIe  
1 2 3 4  
              5 (L)  
         6  
         QIIe

and then we need a reconfiguration, lets say we want to change the cluster from {1, 2, 3, 4, 5, 6} to {5, 7, 8}, so we say N2 = {5, 7, 8}, len(N2) = 3, QIe+1={5, 7}, QIIe+1={5, 8}. we have:

QIe  
1 2 3 4        7,  QIe+1  
              5  
         6        8   QIIe+1  
        QIIe

**and the first question is when entering into the split mode <QIe, QIIe+1>, it means that we would write subsequent comands into QIIe+1, which is {8}, not QIIe, right? and how?**

i am confused about that progress as the set QIIe is set to be the set QIIe+1, which is {leader, a2}, in Fig. 3

when 5 get a reconfiguration command, it choose a slot c, and set e(c) = e(b) + 1.  
values for instance c and any future instances with the same era may be proposed and chosen even though phase I has not yet run for a ballot in this era, so this does not prevent any further instances from running concurrently. that means instance c + 1 would be chosen by QIIe+1? how? we stop the world, then change the QII of instance (c+1) to be  
QIIe+1, then when we run instance (c+1), we send accept request to QIIe+1?

**and second question is what would we do if leader crash before it finish sending b` prepare request to QIe+1?**

says we move cluster from N1={1, 2, 3, 4, 5, 6} to N2={5, 7, 8, 9}.

and we choose {5, 7, 8} to be Q1, and {5, 9} to be Q2, we have QIe+1={5, 7, 8}, QIIe+1={5, 9}

QIe  
1 2 3 4             7,  8  QIe+1  
              5 (X)  
         6             9      QIIe+1  
        QIIe

5 fix the reconfiguration command in slot s + 1, then 5 crash before finish sending b` prepare request to QIe+1, which {7, 8}. then 6 would be the new leader in N1.

and in N2, we have 3 nodes {7, 8 ,9} alive, so we could select a new leader, says 9 would be the new leader.

then 6 would see the reconfiguration command and join N2, and we have 2 leaders {6(leader), 7, 8, 9(leader)}.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@DaveCTurner](https://avatars.githubusercontent.com/u/5058284?s=80&v=4)

### **[DaveCTurner](/DaveCTurner)** commented [May 21, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3312864#gistcomment-3312864) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

I'm sorry, your questions are still confusing the notation and don't really make sense. As I said before, "QIe+1={5, 7}" simply isn't meaningful. Perhaps you mean "QIe+1={{5},{7},{5,7}}"? Note the nested {{}} brackets, this is a **set of sets of nodes** not simply a set of nodes as you are writing. Please can you clarify what you mean as I can't really help without understanding what you're asking.

Similarly "QIe ⌢ QIIe = {5}" makes no sense: "QIe ⌢ QIIe" is a statement about the two sets of quorums that doesn't equal anything (except maybe `true` or `false` I suppose). Perhaps you mean "QIe ∩ QIIe = {5}", noting the important difference between ∩ and ⌢? Even then that doesn't make sense, these are **sets of sets of nodes** so their intersection is also a set of sets of nodes, and it seems to be empty in all reasonable interpretations of your example.

`true`
`false`

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3313821#gistcomment-3313821) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

how to construct the "casting vote"?

There is a sketch of [UPaxos written in Scala](https://github.com/trex-paxos/trex/wiki/UPaxos-Sketch-(April-2017)) that constructs a [LeaderOverlap](https://github.com/trex-paxos/trex/blob/67d7e0b0fff8c77aa7b73558ac6ae445d3d5dafd/core/src/main/scala/com/github/trex_paxos/core/UPaxos.scala#L29) to be able to hold the leader casting vote. It first sorts by node unique identifier then arbitrarily assigns the lower numbered nodes to be the ones that it will send the new `prepare(b')` message. The higher numbered nodes to be the ones that it will continue to stream `accept(s,b,v)` messages. The idea of voting weights is introduced in the paper which is discussed at [paxos-voting-weights](https://simbo1905.blog/2017/03/16/paxos-voting-weights/) which is used to compute the two sets. I should make a disclaimer that the sketch is far from production-ready code. Rather it is throwaway code that established that no disruptive changes needed to be made to implement UPaxos in that codebase as discussed at the first link.

`prepare(b')`
`accept(s,b,v)`

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3313874#gistcomment-3313874)

In terms of notation, the paper uses lower case `q` to represent an instanced of a quorum set and capital `Q` to represent a set of possible sets. Likewise, the "frown notation" of QIe ⌢ QIIe is an abstract statement about some overlap between two sets of sets rather than a specific overlap between two specific sets which is written like {1,2,3} ∩ {5,6} = ∅ that says that the empty set is the overlap between two disjoint sets.

`q`
`Q`

When I read the question I substitute little `q` for big `Q` that seems to make more sense. Could the question please be reworded to use the papers notation such as `q` and avoid capital `Q` which is an abstract set of sets so cannot be used to accurately talk about any concrete scenarios?

`q`
`Q`
`q`
`Q`

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3313893#gistcomment-3313893) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

Let me have a go at answering the first question. Swapping to the lower case and adding a lot of `<sub>` and `<sup>` tags I believe the questions can be restated using as:

`<sub>`
`<sup>`

and then we need a reconfiguration, lets say we want to change the cluster from {1, 2, 3, 4, 5, 6} to {5, 7, 8}, so we say N2 = {5, 7, 8}, len(N2) = 3, qIe+1={5, 7}, qIIe+1={5, 8}. we have:

qIe  
1 2 3 4        7,  qIe+1  
              5  
         6        8   qIIe+1  
        qIIe

That looks fine to me I would say we are doing a reconfiguration from:

`{ { 1,2,3,4,5}, {5,6 } }` to `{ {5,7}, {5,8} }`

`{ { 1,2,3,4,5}, {5,6 } }`
`{ {5,7}, {5,8} }`

clearly we have overlaps between all of those sets which is {5} so we are safe.

**and the first question is when entering into the split mode <qIe, qIIe+1>, it means that we would write subsequent commands into qIIe+1, which is {8}, not qIIe, right? and how?**

Yes indeed I don't see anything complex in this scenario. The new configuration that is being moved into is a three-node cluster. At the point that the leader sees the new cluster configuration has been chosen it would stream accept messages to {8} and fire off a new prepare message to {7}. This is shown in this diagram:

[![overlap](https://camo.githubusercontent.com/2a84b95fe8e888397c2ed1d42fd7bb2d9e705201/68747470733a2f2f73696d626f313930352e66696c65732e776f726470726573732e636f6d2f323031362f31312f6c656164657232332e706e67)](https://camo.githubusercontent.com/2a84b95fe8e888397c2ed1d42fd7bb2d9e705201/68747470733a2f2f73696d626f313930352e66696c65732e776f726470726573732e636f6d2f323031362f31312f6c656164657232332e706e67)

![overlap](https://camo.githubusercontent.com/2a84b95fe8e888397c2ed1d42fd7bb2d9e705201/68747470733a2f2f73696d626f313930352e66696c65732e776f726470726573732e636f6d2f323031362f31312f6c656164657232332e706e67)

The only thing not shown in that diagram is that before the new cluster configuration is chosen it would be streaming accept messages to the old servers.

i am confused about that progess as the set qIIe is set to be the set qIIe+1, which is {leader, a2}, in Fig. 3

Initially, I didn't understand that last sentence. The set qIIe is `{5,6}` and the set qIIe+1 is `{5,8}` so I didn't understand what is meant by the statement which resolves to "`{5,6}` is set to `{5,8}`". Yet as we will see below there is a moment where "the leader updates its accept message routing table from {5,6}`to`{5,8}`" (but of course it doesn't need to actually send messages to itself). So yes, indeed, we swap qIIe to qIIe+1 at the leader at the specific point in time that the new cluster configuration is known to have been chosen via Paxos.

`{5,6}`
`{5,8}`
`{5,6}`
`{5,8}`
`to`

I think you are asking "how do you implement this in code" as it is hard to visualise what is going on. As discussed [here](https://github.com/trex-paxos/trex/wiki/UPaxos-Sketch-(April-2017)):

The big win found in this [UPaxos] sketch is that no overlap logic needs to be put into the core library. Rather when a new cluster configuration is known to be fixed the leader can put in place a message filter which runs the overlap mode and which reroutes any further client command Accept messages to the correct group of nodes. Since this is effectively hiding messages from the core library logic, which is equivalent to dropping messages, it is a low-risk enhancement in terms of safety.

Which is to say all of this only about changing routing tables at the leader as to which servers are sent which messages. When you are not in any overlap mode you broadcast to as many nodes as you can in case of message loss or node crashes. When you are in overlap mode you are careful to send messages to specific servers to avoid "a leader interrupting itself". This momentarily reduces resilience to messages loss but if you are unlucky enough that the network starts to fail when you are reconfiguring your cluster you are going to be having a bad day.

how? we stop the world, then change the QII of instance (c+1) to be  
QIIe+1, then when we run instance (c+1), we send accept request to QIIe+1?

The whole idea is to avoid stopping the world. If you take a step back we are just sending messages and updating the message routing tables within the leader as to which set of nodes is the quorum for which message type. In the example you give {5} is at the overlap of every quorum. So there is no safety violation. The leader is free to instantaneous switch from one routing table to another. It is safe to do that the moment it knows that the new cluster configuration has been chosen by the Paxos algorithm.

Most importantly the leader overlap idea means that the leader is streaming normal commands continuously throughout the process at full speed. This is a huge achievement over all the prior approaches that are discussed in the paper.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@allenling](https://avatars.githubusercontent.com/u/2723570?s=80&v=4)

### **[allenling](/allenling)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314105#gistcomment-3314105)

[@simbo1905](https://github.com/simbo1905) [@DaveCTurner](https://github.com/DaveCTurner) thinks for reply!!!

yes you are right, all Q should be replace with q.

i misuse the Q and q, quite embarrassing.

about first question, what makes me feel confused is Fig.3 in upaxos paper, it has:

The system starts in era e where {ℓ, a1} ∈ QIe and {ℓ, a2} ∈ QIIe, and moves to era e+ 1 where {ℓ, a1} ∈ QIe+1 and {ℓ, a2} ∈ QIIe+1 too

here we have qIIe = qIIe+1 = {ℓ, a2}.

but that is not a real question, i just want to make that example more clear by setting qIIe != qIIe+1.

and the second question, i think i can follow you about the key "leader casting vote" by reading the Trex code.

we can ensure reconfiguration safety if we can create the circumstance that the leader holds the "casting vote", we could do this by setting voting weight, or any other way we come up.

i think i got a more clear picture about upaxos, but still need rereading paper and blog for a few times.

thanks for your time to help, i am really really appreciate that.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@DaveCTurner](https://avatars.githubusercontent.com/u/5058284?s=80&v=4)

### **[DaveCTurner](/DaveCTurner)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314184#gistcomment-3314184)

Ah I see, thanks, that makes a bit more sense indeed. Thanks for the clarification [@allenling](https://github.com/allenling) and well done [@simbo1905](https://github.com/simbo1905) for guessing that, I don't think I would have got there alone.

[@allenling](https://github.com/allenling) I think it will help you a great deal to write out carefully all the sets of quorums Q rather than focussing on an individual quorum q in each step, and then verify that they do indeed satisfy QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1. This is quite a strong property and I think you will find it very illuminating to explore it with some examples. It's possible that the example you're considering doesn't satisfy this property, and this is the source of your confusion.

Assuming your example does satisfy this property, I think this implies it has a single point of failure and therefore progress cannot be guaranteed if a node fails. To be specific, if {5,8} is a phase-II quorum in the new configuration, i.e. q = {5,8} ∈ QIIe+1, then QIe ⌢ QIIe+1 means that every phase-I quorum in the original configuration must contain either node 5 or node 8. I think you intend that node 8 had no part in the original configuration, which means that node 5 must appear in every phase-I quorum, so we simply cannot make progress if node 5 crashes. If you want to guarantee progress in the presence of crashes then you must chose sets of quorums Q that don't have such a single point of failure.

when 5 get a reconfiguration command, it choose a slot c, and set e(c) = e(b) + 1.  
values for instance c and any future instances with the same era may be proposed and chosen even though phase I has not yet run for a ballot in this era, so this does not prevent any further instances from running concurrently. that means instance c + 1 would be chosen by QIIe+1? how?

I'm still not quite sure I'm understanding the question, but perhaps the key here is that QIe ⌢ QIIe+1 which tells us that every *new* phase-II quorum (in QIIe+1) intersects every *old* phase-I quorum (in QIe). This is the key to ensuring that it's safe for these new quorums to continue to choose values without first running another phase-I.

we can ensure reconfiguration safety if we can create the circumstance that the leader holds the "casting vote", we could do this by setting voting weight, or any other way we come up.

The "casting vote" property avoids needing to stall the pipeline to run another phase I but has absolutely no bearing on safety: safety follows directly from the invariants in fig.2 which are independent of the existence (or otherwise) of a casting vote.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314296#gistcomment-3314296) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

To address the second question converting to lower case q notation:

and second question is what would we do if leader crash before it finish sending prepare(b') request to qIe+1?

If we look again at the simpler scenario:

qIe  
1 2 3 4        7,  qIe+1  
              5  
         6        8   qIIe+1  
        qIIe

In that, we see that 5 is the only overlap between the quorums. It looks like a single point of failure. The paper addresses safety. In practical implementations, there can be a lot of details that are added to speed things up and to avoid a single point of failure. As discussed below as long as we don't violate the invariants of UPaxos we will have safety.

Apologies for the very long answer but I think that actually working through a realistic example might help. What might we actually be trying to achieve in the scenario above? To my mind, we are trying to get to a final state of three servers. Previously we had more than three servers. So let's imagine that we are migrating from five small hosts onto three bigger hosts. This could be a cost reduction exercise while at the same time refreshing onto faster hardware. Let's say host 5 is actually one of the new bigger hosts. So we must have first reconfigured a five node cluster to add that big host to get to the start of the scenario above. Then we would do a leadership swap to make the new big host 5 the leader. Then in the above scenario, we are swapping in two more new big servers while swapping out the original five smaller hosts.

Using a tuple notation as the cluster configuration "[prepare\_quorum, accept\_quorum]" the starting quorums could be a typical FPaxos five node setup:

["any four of five", "any two of five"]

Why? Because FPaxos shows that will be faster for steady-state than doing simply majority quorums of "three out of five" for both phases. Then we are going down to a three-node cluster where we will run simple majorities:

["any two of three", "any two of three"]

This is because with only three nodes there is not enough nodes to perform an FPaxos optimisation. The interim step used in the example is based on six nodes. Stretching my new notation the reconfiguration can be expressed as:

[prepare\_quorume -> prepare\_quorume+1, accept\_quorume -> accept\_quorume+1 ]

To go from five nodes to six can be expressed as something that matches the following transition while maintaining the necessary UPaxos invariants of QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1:

["any four of five" -> "any ? of six", "any two of five" -> "any ? of six"]

Note that we call those Q ⌢ Q a "frown" as it looks like an unhappy emoticon. **Edit:** clarified by David below these frowns are not transitive so we only need three overlaps to satisfy "the three frowns of UPaxos".

We need to respect the standard FPaxos invariant that qIe∩qIIe for all e. That covers two of the frowns. The third frown is the only one that runs across eras. The paper tells us that a prepare issued from the old era must see any accepted values chosen in the new era. So we need qIe∩qIIe+1. Why? Paxos is a collaborative algorithm where a newly elected leader needs to choose the values of the old crashed leader. UPaxos is doing "FPaxos across eras" so a new leader in the old era must see the accept messages of a crashed leader in the new era. The paper expresses this as:

While a change from era e to e + 1 is in progress some instances may use the interim configuration ⟨QIe , QIIe+1 ⟩

That isn't to say that the operator told it to run in that configuration. It is just the case that a possible leader in the old configuration must see and respect the accepted values of any crashed leader in the new configuration.

**Edit:** I had originally written:

Intuitively to avoid a split-brain we need an overlap overlap qIe∩qIe+1

As per the clarification from David below this simply isn't necessary.

We can go with this:

["any four of five"Ie -> "any four of six"Ie+1, "any two of five"IIe -> "any four of six"IIe+1]

Let's sanity-check that. In the two eras we have the necessary three UPaxos overlaps:

"any four of five"Ie ∩ "any two of five"IIe != ∅  
"any four of six"Ie+1 ∩ "any four of six"IIe+1 != ∅  
"any four of five"Ie ∩ "any four of six"IIe+1 != ∅

One place we don't have overlap is:

"any two of five"IIe ∩ "any four of six"IIe+1 == ∅

That doesn't violate any invariant that I understand needs to be preserved. An expert in the math would be able to prove or disprove that. Hopefully, David will correct my mistake if I have made one. I am relying upon my intuitive explanation that "Paxos works as the new leader collaborates with the old leader by seeing accepted values on prepare responses then choosing the same value". This means that not having an overlap in accept messages between the eras is okay.

That completes the first transition only. The example above is reducing from six nodes to three so we need:

["any four of six"Ie -> "{5} plus one another"Ie+1, "any four of six"IIe -> "{5} plus one another"IIe+1]

Why? Well, we can see that host 5 in the scenario above is the only node that overlaps! Rather than using "n of m" quorums, we must be more explicit. A quorum is some set of nodes that actually voted. If we only have one overlap node we need to be very explicit that the only allowable quorum is the set of positive votes that has the necessary overlaps between quorums sets across eras that are defined above.

The question is what happens if 5 crashes before your reconfiguration finishes? Then you learn that you shouldn't have done such a drastic reconfiguration in one step that left you exposed to a single point of failure. The initial 5 nodes configuration can survive any 2 nodes failing. The final 3 node configuration can survive any one node failing. The reconfiguration in question can survive zero nodes failing if it happens to be node 5.

In practice when we are changing clusters around we always go through a sequence of steps that are small changes and allow those to settle such as data rebalancing. As mentioned in the paper we know that if we use majorities and only either add or remove one node at a time we are safe. I would simply automate the process and swap in one big node at a time into the five node cluster as a single node addition reconfiguration followed by a single node removal reconfiguration. Repeat that three times then finally subtract the last two small nodes in two steps. That would be three adds and five subtracts so eight reconfigurations in total.

In the sketch of UPaxos in Trex there is no support for computing a quorum overlap other than using majority rules. So it would be able to add or remove one node at a time but it wouldn't be able to perform a transition that requires a specific node to be in a given quorum like the scenario above. I do not see that as a missing feature! More code is more surface for bugs! As in the real world, an operator can achieve a save refresh of a cluster one node at a time that is all the code I would write. All that needs to happen in Trex is that it checks "the three frowns" quorums overlap reusing its existing FPaxos logic when in the process of doing a reconfiguration. I didn't get around to writing that code in the sketch.

Thanks for the question as I really enjoyed rereading the paper as it is so well written. It really is my favourite thing that I have read about Paxos. Once again I must congratulate David on his beautiful and elegant invention. We Yorkshireman are supposed to be reserved but in this day and age, I think it is okay for me to make such complimentary statements!

(I should make a disclaimer that Trex is a personal educational project. The systems I write code for in production are usually proprietary software or platforms built on-top of established opensource. So when I talk about my experiences of expanding or contracting clusters they might not be running Paxos. All of them run at least Paxos, Raft or ZAB somewhere.)

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314337#gistcomment-3314337) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

[@DaveCTurner](https://github.com/DaveCTurner) is the frown operator transitive in the expression QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1 ? Is it saying all combinations of them must have one node in the overlap?

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314588#gistcomment-3314588) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

[@simbo1905](https://github.com/simbo1905) [@DaveCTurner](https://github.com/DaveCTurner) thinks for reply!!!  
i misuse the Q and q, quite embarrassing.

There are absolutely no embarrassing questions with Paxos. For example, I think I understand the notation but have a doubt so posted a question to ask the author above. If I have made a mistake in my analysis, and David corrects me, I won't feel shame. I will be happy that I have learnt something and corrected my misunderstanding.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@DaveCTurner](https://avatars.githubusercontent.com/u/5058284?s=80&v=4)

### **[DaveCTurner](/DaveCTurner)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314626#gistcomment-3314626)

No, that's not the intended meaning, QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1 means QIIe ⌢ QIe ∧ QIe ⌢ QIIe+1 ∧ QIIe+1 ⌢ QIe+1 and doesn't say anything about the other pairs of quorums.

Intuitively to avoid a split-brain we need an overlap qI∩qIe+1

This actually isn't needed for safety, similarly to FPaxos. The following (no-op) configuration change is fine and does not satisfy QIe ⌢ QIe+1:

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@DaveCTurner](https://avatars.githubusercontent.com/u/5058284?s=80&v=4)

### **[DaveCTurner](/DaveCTurner)** commented [May 22, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3314784#gistcomment-3314784) • edited Loading Uh oh! There was an error while loading. Please reload this page.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

For the example of migrating from 5 nodes to 3 nodes via 6 nodes the identities of the nodes are important, but the "three of five" notation omits this. Let's define P≥n(S) to be the at-least-size-n powerset of S, so we can say P≥3({1,2,3,4,5}) instead of "three of five" and be specific about which five nodes we're talking about. The goal (as I understand it) is to migrate from this configuration in era e ...

... via an intermediate configuration in era e+1 to this configuration in era e+2 ...

... noting that node 5 is in play at the end here too.

One possibility for an intermediate configuration of the form "four of six" is this:

This doesn't work because {1,2,3,4} ∈ QIe+1 doesn't intersect {6,7} ∈ QIIe+2.

Another possibility for "four of six" is this:

This also doesn't work, because {1,2,3,5} ∈ QIe+1 doesn't intersect {6,7} ∈ QIIe+2.

In fact I don't think there are any valid intermediate configurations of the form "four of six" that get you to the desired configuration in two steps. I'm pretty sure you can do it in 3 steps if you allow node 5 to become a single point of failure, but there are better answers too. For instance this configuration works as a single intermediate step and has no single points of failure:

As the paper mentions, you can in fact always do a reconfiguration safely in two steps by using Raft's notion of *joint configuration*, and I'm fairly sure that the joint configuration is always at least as resilient as the two configurations it's joining.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 23, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3315103#gistcomment-3315103)

Great, that clarifies things a lot. Thanks. I now feel that I am equipped to try to express things exactly and avoid safety violations with minimal constraints.

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=80&v=4)

### **[simbo1905](/simbo1905)** commented [May 23, 2020](/allenling/99bf0e965fa7e0b208f461446fcc97e1?permalink_comment_id=3315516#gistcomment-3315516)

I wrote up what I learnt in this discussion as a blog post at <https://simbo1905.blog/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/>

Sorry, something went wrong.

### Uh oh!

There was an error while loading. Please reload this page.

There was an error while loading. Please reload this page.

## Footer

### Footer navigation
