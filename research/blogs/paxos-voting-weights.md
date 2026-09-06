# Paxos Voting Weights

Source: https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/
Author: simbo1905 (slash dev slash null)
Fetched: 2026-09-05 via tavily extract (markdown). Site boilerplate removed.

---

The [last UPaxos post](https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/) took a run through the reconfiguration stall avoiding aspects of the [UPaxos paper](http://tessanddave.com/paxos-reconf-902f8b7.pdf). In this post, we will take a quick look at how the paper describes hot swapping of nodes using voting weights.

So what's a voting weight? It's an integer multiplier that a given node has within a Paxos quorum. Traditional Paxos has all nodes have a weight of "1" such that every node is equal. So what can we achieve by varying the weights?

Well, straight away you get "learners" by configuring acceptors with a zero weight. The original Paxos papers decomposed Paxos into proposers, acceptors and learners roles. Learners are nodes that track the outcome of the consensus algorithm without participating in the consensus quorums. They are replicas which can be used to scale read load. Simply applying a zero voting weight makes an acceptor a learner. Better yet if an acceptor dies you can promote a learner by assigning it a voting weight of "1". Happy days.

What else? Well, it allows us to set up leader casting vote scenarios that we need for [UPaxos cluster reconfigurations](https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/). The paper gives a very practical scenario which is upgrading a server. It points out that cloud providers today typically have three rather than four availability zones in each region. If we just replace a node, by adding the new one then deleting the old one, we pass through the following three states:

| Zone A | Zone B | Zone C | |
| --- | --- | --- | --- |
| Server W | Server X | Server Y | Server Z |
| 1 | 1 | 1 | – |
| 1 | 1 | 1 | 1 |
| 1 | 1 | – | 1 |

The problem is that we temporarily have four nodes with two of them in the same availability zone. If we loose Zone C in that state then we only have half the cluster. With the [FPaxos even nodes optimisation](https://simbo1905.wordpress.com/2016/09/30/the-fpaxos-even-nodes-optimisation/), a leader in Zone A or B will continue to process client commands. The problem is that you cannot elect a new lead if it dies.

To get around this we can use voting weights. In the following table if we lose Zone C at any point a leader in Zone A or B can still get a majority:

| Zone A | Zone B | Zone C | |
| --- | --- | --- | --- |
| Server W | Server X | Server Y | Server Z |
| 1 | 1 | 1 | – |
| 2 | 2 | 2 | – |
| 2 | 2 | 2 | 1 |
| 2 | 2 | 1 | 1 |
| 2 | 2 | – | 1 |
| 2 | 2 | – | 2 |
| 1 | 1 | – | 1 |

At each step in that process, we either doubled all weights, halved all weights, or changed one node's voting weight by one. Why? To ensure the majorities between consecutive reconfigurations overlap.

It is fairly obvious that if you double or halve all weights between consecutive configurations a simple majority in each will overlap. Any integer scaling factor has that property. Adjusting only one node by 1 is a little more subtle. This is equivalent to adding or removing one node in a cluster where every node has a voting weight of 1. If we consider the sequence of integers `{3, 4, 5, 6, 7, ...}` then we can see that consecutive majorities do indeed overlap. If we were to skip a number such as changing from 5 to 7 then majorities wouldn't overlap and we would have violated the UPaxos safety constraints.

`{3, 4, 5, 6, 7, ...}`

---

## Comments (abridged to substantive exchanges)

**coboler:** This algorithm and paper sounds almost too good to be true! I wonder if anyone is working on a TLA+ implementation of this. Is this what you plan to implement for TREX?

**simbo1905:** @coboler yes I intend to implement dynamic cluster membership as UPaxos. Having talked it though with the author of UPaxos the changes needed to the "library" project of TRex are straight-forward:
1. Add an "era" int into the BallotNumber case class which defaults to zero. That won't break any existing tests nor break any existing users of the library.
2. Change the logic that issues high prepare messages. Currently, that logic updates the current promise prior to broadcasting the message. That logic needs to broadcast without updating the promise. Then on the handler to the responses to the message make the promise only when it is calculated that this will give a majority.

With those minimal changes, I believe that the UPaxos leader overlap mode can be coded as a message filter and addition state tracking in the "core" project above the library. [...]

**Update:** These ideas are now sketched out in working code on a branch of TRex as documented at [UPaxos-Sketch-(April-2017)](https://github.com/trex-paxos/trex/wiki/UPaxos-Sketch-(April-2017)). Step 1 above was a core library change but step 2 is implemented as a message filter at the leader which matches only responses to the final message of the UPaxos reconfiguration.

**David Turner (@DaveCTurner), paper author:** Hi coboler, Paper author here! [...] There's one wrinkle to curb your enthusiasm which simbo spotted: abdication to a new leader may still cause a pipeline stall. I can't see a way around this. This means that replacing a single node is stall-free (if the leader doesn't change) but replacing your whole cluster requires an abdication at some point as the leader necessarily changes. The safety proofs in the paper are formalised in Isabelle/HOL (a theorem prover rather than a model checker) so I've minimal doubt that they're true. More strongly, in fact, Isabelle was instrumental in this work: I don't think I would have discovered this had I not been trying to formalise something simpler first. [...] The liveness proof remains informal for now; in fact I'm working on formalising a stronger liveness property that doesn't rely on an external leader election protocol, again following Raft's approach. [...]

**coboler:** [...] How do we change the voting weight for the peers? [...] During the reconfiguration phase(via messages), do we assign the peers new voting weights and in the event that the current leader actually crashes, can a new leader election can go on? [...]

**simbo1905:** The "Membership" data structure defines all of the cluster configuration including the current voting weights and the quorums for prepares and accepts. An administration tool would encode it as json and transmit it to the leader in a "ClusterCommandValue". The leader is free to choose that value, or a vanilla "ClientCommandValue" from a regular client, or a no-op value, to use in the next accept message for the next slot. [...] If the leader dies during the reconfiguration it is no different from a leader dying at any other point. One surviving node will timeout and run the leader takeover protocol prior to leading. [...] > from the paper section on voting weights, it is mentioned that the new voting weights will guard against stalling when a leader dies. — The problem it is avoiding is in going from 3 to 4 nodes by adding a new node which is in the same cloud resilience zone as an existing node. Lose that zone and you lose half the cluster. The two surviving nodes cannot get a majority to lead. The fix is to double all voting weights of the original 3 node cluster then add the new node with a weight of 1. [...]

**coboler:** Why must the newly added server increase it's voting weight to 2 only for all the servers to go back to 1 at the last step?

**simbo1905:** Because of the properties of integer numbers. If you take a pencil and paper and draw out the table and figure out all the permutations of crashes and quorums for each row you will see the truth of it. [...]
