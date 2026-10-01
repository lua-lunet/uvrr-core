<!-- source: https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/ -->

# [slash dev slash null](https://simbo1905.wordpress.com/ "Home")

### UPaxos: Unbounded Paxos Reconfigurations

#### by simbo1905

The year 2016 turned out to be a bumper year for pragmatic Paxos discoveries. Hot on the heels of the [FPaxos](https://simbo1905.wordpress.com/2016/09/30/the-fpaxos-even-nodes-optimisation/) discovery of more flexible quorums comes [Unbounded Pipelining in Dynamically Reconfigurable Paxos Clusters](https://davecturner.github.io/2016/06/09/unbounded-pipelining-paxos.html) or “UPaxos”. This uses overlapping quorums between consecutive cluster configurations, and a leader “casting vote”, to enable cluster reconfigurations in a non-stop manner even when reconfiguration messages are lost.

The UPaxos paper describes a state-of-the-art general-purpose Paxos implementation packed with solid techniques. In this post we will look at the key finding of eliminating the [pipeline stall](https://simbo1905.wordpress.com/2016/12/15/paxos-reconfiguration-stalls/) problem.

To understand the safety of UPaxos it is helpful to recap on overlapping quorums as generalised by [FPaxos](https://simbo1905.wordpress.com/2016/09/30/the-fpaxos-even-nodes-optimisation/). We know from FPaxos that we can use separate quorums for `Prepare` and `Accept` messages as long as they overlap. Why? If a leader transmits an `Accept,` and then fails, the next leader will issue a `Prepare,` which needs to see at least one response containing the highest accepted value of the old leader. Why? Because Paxos is a collaborative algorithm. Any new leader always “chooses” the highest numbered value of the old leader. Intuitively we can say that the new leader “completes” any consensus work started by the last leader. UPaxos takes this to the next level by requiring that quorums overlap between consecutive reconfigurations. The stroke of genius in the UPaxos paper is exactly how we can use this additional safety property to avoid pipeline stalls.

`Prepare`
`Accept`
`Accept,`
`Prepare,`

With Paxos a cluster configuration is a pair of quorum sets ![\langle Q^I,Q^{II} \rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q%5EI%2CQ%5E%7BII%7D+%5Crangle&bg=fff&fg=444444&s=0&c=20201002) where ![Q^I](https://s0.wp.com/latex.php?latex=Q%5EI&bg=fff&fg=444444&s=0&c=20201002) is the set of nodes used for “Phase I” `Prepare` messages and ![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) is the set of nodes used for “Phase II” `Accept` messages. The general safety requirement is that the two sets overlap which is written ![Q^I \frown Q^{II}](https://s0.wp.com/latex.php?latex=Q%5EI+%5Cfrown+Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002). The [original Paxos papers](http://research.microsoft.com/en-us/um/people/lamport/pubs/paxos-simple.pdf) used a simple majority of nodes for both phases which clearly overlap. Using distinct but overlapping sets for the two phases [is a new idea in 2016](http://ssougou.blogspot.co.uk/2016/08/a-more-flexible-paxos.html) which [the FPaxos paper](https://arxiv.org/pdf/1608.06696v1.pdf) made famous.

![\langle Q^I,Q^{II} \rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q%5EI%2CQ%5E%7BII%7D+%5Crangle&bg=fff&fg=444444&s=0&c=20201002)
![Q^I](https://s0.wp.com/latex.php?latex=Q%5EI&bg=fff&fg=444444&s=0&c=20201002)
`Prepare`
![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)
`Accept`
![Q^I \frown Q^{II}](https://s0.wp.com/latex.php?latex=Q%5EI+%5Cfrown+Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)

Changes to the cluster configuration are modelled by a sequence of configurations ![\langle Q_0^I,Q_0^{II}\rangle,\langle Q_1^I,Q_1^{II}\rangle \ldots](https://s0.wp.com/latex.php?latex=%5Clangle+Q_0%5EI%2CQ_0%5E%7BII%7D%5Crangle%2C%5Clangle+Q_1%5EI%2CQ_1%5E%7BII%7D%5Crangle+%5Cldots&bg=fff&fg=444444&s=0&c=20201002). The integer subscript is called the era and it is the index of a particular configuration in the sequence of configurations. As with the original Paxos paper we change the cluster configuration by treating it as a command value that we run through the algorithm. When the cluster reconfiguration command value is known to be fixed it is appended to the local copy of the sequence of configurations at each node. Another way to say this is that we use the Paxos engine itself as the atomic broadcast mechanism for new cluster configurations. As stalls are not a problem for UPaxos the cluster configuration comes into effect immediately. As we saw in [the previous post](https://simbo1905.wordpress.com/2016/12/15/paxos-reconfiguration-stalls/) with traditional Paxos that would harm concurrency when messages are lost.

![\langle Q_0^I,Q_0^{II}\rangle,\langle Q_1^I,Q_1^{II}\rangle \ldots](https://s0.wp.com/latex.php?latex=%5Clangle+Q_0%5EI%2CQ_0%5E%7BII%7D%5Crangle%2C%5Clangle+Q_1%5EI%2CQ_1%5E%7BII%7D%5Crangle+%5Cldots&bg=fff&fg=444444&s=0&c=20201002)

The UPaxos paper has a logical proof of the additional safety requirement used to eliminate the possibility of stalls. The additional requirement is an overlap between the phase I and phase II quorums of successive configurations. This is written as ![Q_e^I \frown Q_{e+1}^{II}](https://s0.wp.com/latex.php?latex=Q_e%5EI+%5Cfrown+Q_%7Be%2B1%7D%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) for all ![e](https://s0.wp.com/latex.php?latex=e&bg=fff&fg=444444&s=0&c=20201002). It’s worth noting that if we use straight-forward majorities, and we either add or remove a single node, then majorities in consecutive configurations will overlap.

![Q_e^I \frown Q_{e+1}^{II}](https://s0.wp.com/latex.php?latex=Q_e%5EI+%5Cfrown+Q_%7Be%2B1%7D%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)
![e](https://s0.wp.com/latex.php?latex=e&bg=fff&fg=444444&s=0&c=20201002)

We can talk about the era of a slot as ![e \langle s \rangle ](https://s0.wp.com/latex.php?latex=e+%5Clangle+s+%5Crangle+&bg=fff&fg=444444&s=0&c=20201002). This indicates the phase II quorum to fix a slot. Ballot numbers also have an era ![e \langle b \rangle ](https://s0.wp.com/latex.php?latex=e+%5Clangle+b+%5Crangle+&bg=fff&fg=444444&s=0&c=20201002). How? We encode the era number into the most significant bits of the ballot number. We can refer to the prepare quorum of a ballot number as ![Q_{e \langle b \rangle}^{I}](https://s0.wp.com/latex.php?latex=Q_%7Be+%5Clangle+b+%5Crangle%7D%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002).We say that the cluster is “in era e” while instances are being chosen using ![\langle Q_{e}^{I},Q_{e}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%7D%5E%7BI%7D%2CQ_%7Be%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002). The paper states that whilst a change from era e to e + 1 is in progress some instances may use the interim configuration ![\langle Q_{e}^{I},Q_{e+1}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%7D%5E%7BI%7D%2CQ_%7Be%2B1%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002), and once the change from era e to e+1 is complete all instances will use ![\langle Q_{e+1}^{I},Q_{e+1}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%2B1%7D%5E%7BI%7D%2CQ_%7Be%2B1%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002). The paper distills the safety requirement down to ![e \langle s \rangle \geq e \langle b \rangle + 1](https://s0.wp.com/latex.php?latex=e+%5Clangle+s+%5Crangle+%5Cgeq+e+%5Clangle+b+%5Crangle+%2B+1&bg=fff&fg=444444&s=0&c=20201002). Which basically says that if a leader has received sufficient promises for a ballot number in a given era it can use it fix a slot in both the same or next era.

![e \langle s \rangle ](https://s0.wp.com/latex.php?latex=e+%5Clangle+s+%5Crangle+&bg=fff&fg=444444&s=0&c=20201002)
![e \langle b \rangle ](https://s0.wp.com/latex.php?latex=e+%5Clangle+b+%5Crangle+&bg=fff&fg=444444&s=0&c=20201002)
![Q_{e \langle b \rangle}^{I}](https://s0.wp.com/latex.php?latex=Q_%7Be+%5Clangle+b+%5Crangle%7D%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002)
![\langle Q_{e}^{I},Q_{e}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%7D%5E%7BI%7D%2CQ_%7Be%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002)
![\langle Q_{e}^{I},Q_{e+1}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%7D%5E%7BI%7D%2CQ_%7Be%2B1%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002)
![\langle Q_{e+1}^{I},Q_{e+1}^{II}\rangle](https://s0.wp.com/latex.php?latex=%5Clangle+Q_%7Be%2B1%7D%5E%7BI%7D%2CQ_%7Be%2B1%7D%5E%7BII%7D%5Crangle&bg=fff&fg=444444&s=0&c=20201002)
![e \langle s \rangle \geq e \langle b \rangle + 1](https://s0.wp.com/latex.php?latex=e+%5Clangle+s+%5Crangle+%5Cgeq+e+%5Clangle+b+%5Crangle+%2B+1&bg=fff&fg=444444&s=0&c=20201002)

Okay, enough math already. How do we use this to do a reconfiguration without a stall? The main technique is the “leader casting vote”. Let’s walk through a simple reconfiguration. By way of a warm-up consider the following diagram where there is no reconfiguration to establish a baseline. In this figure a three node cluster comes up and gets to normal running using simple majority quorums:

![leader1](https://simbo1905.wordpress.com/wp-content/uploads/2016/11/leader12.png?w=420&h=376)

![leader1](https://simbo1905.wordpress.com/wp-content/uploads/2016/11/leader12.png?w=420&h=376)

That is the simplest case in vanilla Paxos. A node times out and becomes the leader. It issues a prepare using a suitably high ballot number b. When it hears back a quorum response it then streams client commands v1 and v2 with two consecutive accept messages. The messages target slots s and s+1. The diagram only shows two client commands but the leader can continue indefinitely.

Okay so what happens during a UPaxos reconfiguration? In the following diagram, we move between two eras without changing any physical nodes. Why? Well, it does help to make the diagram less messy. There is actually a very good reason to keep the same physical nodes but to change their voting weights which is covered in the UPaxos paper. I will cover that in a [later post](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/). So for the moment assume that we are just doing this to demonstrate the principle of how stalls are eliminated.

![leader2](https://simbo1905.wordpress.com/wp-content/uploads/2016/11/leader23.png?w=406&h=443)

![leader2](https://simbo1905.wordpress.com/wp-content/uploads/2016/11/leader23.png?w=406&h=443)

At the first message the leader is still in steady state mode (the bottom of the previous diagram). In this mode it happens to stream a client command labeled v1 under ballot number b in the original era e. Then at the upper dotted line the administrator commands a reconfiguration to move the cluster into era e+1. The leader dutifully streams that reconfiguration command, rather than a regular client command, into the next slot. When the leader hears a quorum response to the reconfiguration command at the second dotted line the new configuration is in effect. Then things get interesting.

Once the leader knows the new era is fixed at slot s+1 it needs to upgrade to a fresh ballot number in the new era. It does this by having a majority of nodes promise to a new ballot number in the new era. Only once it has done this will it have completed the reconfiguration. Until it has upgraded to a new ballot number in the new era it will not be able to process any further reconfiguration commands.

To upgrade to a fresh ballot number belonging to the new era the leader introduces an asymmetry by entering into a “leader overlap mode”. It splits the cluster into two quorums. A prepare quorum ![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002)  with Follower 1 and an accept quorum ![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) with Follower 2. It only sends the prepare message containing the new ballot number b’ to Follower 1. Meanwhile, it continues to stream client values to Follower 2 using the old ballot number? Why?

![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002)
![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)

The leader does this to create the circumstances where it has the “casting vote” as to whether the ![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002) quorum has made sufficient promises to the new ballot number. It does this by acting last within the prepare quorum. It defers from making a promise to its own new ballot number until it knows that sufficient other nodes have also made promises to give it the “casting vote”. The leader doesn’t risk disrupt a majority of nodes in the ![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) quorum by sending any b’ prepare messages until it has cast its own vote and has a majority. Rather it continues to stream client commands to the ![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) quorum. Why?

![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002)
![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)
![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)

This is because Paxos is designed to work even when messages are delayed or reordered. If it was to steam a prepare b’ to nodes in the ![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002) it may be reordered with respect to client commands it previously streamed using b. That will cause the client commands to be rejected forcing the leader to resend. By holding the “casting vote” in the  ![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002) quorum the leader knows exactly the point at where it instantaneously promises to b’ itself. That is the exact point where it will switch to using b’ for subsequent client commands. As those are local actions that won’t be reordered.

![Q^{II}](https://s0.wp.com/latex.php?latex=Q%5E%7BII%7D&bg=fff&fg=444444&s=0&c=20201002)
![Q^{I}](https://s0.wp.com/latex.php?latex=Q%5E%7BI%7D&bg=fff&fg=444444&s=0&c=20201002)

What happens if the leader crashes at any point during the reconfiguration? No special logic need to recover from a leader crash. The normal logic of the [leader takeover phase](https://simbo1905.wordpress.com/2014/10/28/transaction-log-replication-with-paxos/) will work as normal. Regardless of whether the next leader is in the old or new era it will “see” any values which it needs to complete.

I should note that some less reputable consensus algorithms dismiss reordering as a problem for strong consistency. They assume that TCP connections will keep messages in order. That gets tricky when you have dropped connections and reconnects so you likely still need to be tracking what’s going on above the network layer. Yet UDP has lower overhead without attempting to order messages. The new super fast “kernel bypass” [RDMA networking technologies](http://www.rdmamojo.com/2013/06/01/which-queue-pair-type-to-use/) that take networking down to microseconds have a UDP mode; exactly to avoid the overhead of ordering. When we consider that the network might send datagram packets between two data centre resilience zones via redundant routes then we see how reordering can occur. The whole point of Paxos is to generate consensus across of an unreliable and asynchronous network that doesn’t guarantee messaging ordering. That works with redundant routes with arbitrary delays where messaging ordering isn’t a guaranteed.

### Share this:

### 12 Comments to “UPaxos: Unbounded Paxos Reconfigurations”

[…] Turner the inventor of UPaxos has posted a great blog post about monitoring Paxos clusters. It’s a must read for consensus […]

![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

i got a little trouble in understanding upaxos progress. it will be really appreciated if you could please help me understanding some examples?

let`s write Q(I, e) and Q(II, e) for phase-I and phase-II quorums for ballot b respectively.

fig.3 in upaxos paper is not a general situation as setting {leader, a1} = Q(I, e) = Q(I, e+1) and {leader, a2} = Q(II, e) = Q(II, e+1).

lets split them and say N1=6, Q(I, e)={1, 2, 3, 4, 5}, Q(II, e+1)={5, 6}, L=5,

Q(I, e)  
1 2 3 4  
5  
6  
Q(II, e)

and then we move from e to e+1, say N2=3, Q(I, e+1)={5, 7}, Q(II, e+1)={5, 8}, we have Q(I, e) and Q(II, e+1) intersect.

Q(I, e)  
1 2 3 4 7 Q(I, e+1)  
5  
6 8 Q(II, e+1)  
Q(II, e)

what happen when cluster entering split mode? according to the upaxos paper:

“While a change from era e to e + 1 is in progress some instances may use the interim configuration ”

leaer 5 should write client commands into Q(II, e+1), which is {8}, even the e(b) of node 8 is not known yet.

after fix reconfiguration at slot s+1, leader 5 crash before it send a b` prepare to Q(I, e+1)

then {1, 2, 3, 4, 6} would elect a leader , lets say 6, then new leader 6 would join {7, 8}, and would find all client commands.

and about “casting vote”, what if nodes in Q(I, e+1) going to elect a leader when leader down?

lets say N2=4, Q(I, e+1)={5, 7, 8}, Q(II, e+1)={5, 9}

Q(I, e)  
1 2 3 4 7, 8 Q(I, e+1)  
5  
6 9 Q(II, e+1)  
Q(II, e)

after fix reconfiguration at slot s+1, leader 5 crash before it send a b` prepare to Q(I, e+1)

nodes {7, 8, 9} could elect a new leader because the amount of nodes in e+1 satisfy the required amount of Q(I, e+1), which is 3.

and in other side, {1, 2, 3, 4, 6} would elect 6 to be leader, and 6 would see the reconfiguration in slot s+1, and join {7, 8, 9}, finally we got two leaders

and you says:

“Once the leader knows the new era is fixed at slot s+1 it needs to upgrade to a fresh ballot number in the new era. It does this by having a majority of nodes promise to a new ballot number in the new era.”

and

“It does this by acting last within the prepare quorum. It defers from making a promise to its own new ballot number until it knows that sufficient other nodes have also made promises to give it the ‘casting vote.'”

in this situation, majority does not helps, and how to construct the “casting vote”?

![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

seems removing space mess the example 😦

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)

Perhaps creating a “gist” on GitHub of a particular type (eg question.md) might help?

![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

<https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1.js>

![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

here is the gist <https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1> 🙂

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)

thanks Allen, I am enjoying rereading the paper and will respond once I have digested your question.

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)

The UPaxos author has kindly responded on the gist. I think that as he suggests it is the correct tracking the identity of nodes within a quorum, rather than quantity of nodes in a quorum, that is key to implementing the algorithm safely. If I were implementing UPaxos in Scala is would use a Set[int] to define a quorum ‘q’ where the ints are the unique node identifiers. When I got back responses I would hold them in a Map[int, Response] where the int key is the node identifier. Duplicated messages would be deduplicated by inserting into the map. To compute “Do I see a quorum response?” I can take the set of keys from the map and compare that with the set of integers of the quorum ‘q’ and know that, yes, the exact nodes in the quorum have responded.

From a pragmatic point of view, I probably would not code an implementation that let someone perform drastic reconfiguration like your question shows in a single step under normal running. Rather if an operator wanted to move from a membership N1 to N2 then I would compute a set of intermediate reconfigurations that only added or removed one node at a time else only changed the voting weights. I would write a client admin tool that computes all the intermediate memberships to get from N1 to N2 in small steps. I would write exhaustive unit tests for that tool. I would then write exhaustive unit tests for adding or removing one node or changing the voting weights in a cluster. I would then be very confident that an operator could say “replace all 7 small VMs with 7 large VMs” and the admin tool would present them with a plan that replaced one node at a time that they could enter ‘yes’ to confirm the commands and the admin client tool would step through replacing one VM at a time.

![davecturner's avatar](https://0.gravatar.com/avatar/cc5223e5f1f5a1038eca9bd65988f8df3de7966759c39d3eb008d4a19fe13aff?s=48&d=identicon&r=G)

Thanks for the question @Allen\_Ling3, I made an initial reply over on Github: <https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1#gistcomment-3312461>.

![davecturner's avatar](https://0.gravatar.com/avatar/cc5223e5f1f5a1038eca9bd65988f8df3de7966759c39d3eb008d4a19fe13aff?s=48&d=identicon&r=G)

> Rather if an operator wanted to move from a membership N1 to N2 then I would compute a set of intermediate reconfigurations that only added or removed one node at a time else only changed the voting weights.

This is actually a lot harder to do reliably than making the change in a single step, because there’s so many more intermediate states and failure modes, and you need to worry about how to resume a failed migration and maybe take it in a different direction and so on.

FWIW Elasticsearch makes large config changes in a single step for exactly this reason.

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)

that’s very interesting. thanks for the insight.

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)

Hi Allen, I have written a new post about UPaxos over at <https://simbo1905.blog/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/> Thanks for asking questions so that we can all learn something new!

### Leave a comment [Cancel reply](/2016/12/16/upaxos-unbounded-paxos-reconfigurations/#respond)

Δ

#### Recent Posts

#### Recent Comments

|  |  |
| --- | --- |
| [simbo1905's avatar](https://simbo1905.wordpress.com) | [simbo1905](https://simbo1905.wordpress.com) on [Blockchain for Finance is…](https://simbo1905.wordpress.com/2020/05/13/blockchain-for-finance-is-bollocks/comment-page-1/#comment-1077) |
| Anthony's avatar | Anthony on [The Trial Of Paxos Algori…](https://simbo1905.wordpress.com/2014/10/05/in-defence-of-the-paxos-consensus-algorithm/comment-page-1/#comment-1075) |
| Anthony's avatar | Anthony on [The Trial Of Paxos Algori…](https://simbo1905.wordpress.com/2014/10/05/in-defence-of-the-paxos-consensus-algorithm/comment-page-1/#comment-1074) |
|  | [Cluster Replication…](https://simbo1905.wordpress.com/2014/10/28/transaction-log-replication-with-paxos/) on [Pre-voting in distributed…](https://simbo1905.wordpress.com/2017/08/22/pre-voting-in-distributed-consensus/comment-page-1/#comment-1073) |
| [allenling (@Allen_Ling3)'s avatar](http://twitter.com/Allen_Ling3) | [allenling (@Allen\_Li…](http://twitter.com/Allen_Ling3) on [Cluster Replication With …](https://simbo1905.wordpress.com/2014/10/28/transaction-log-replication-with-paxos/comment-page-1/#comment-1000) |

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)
![Anthony's avatar](https://1.gravatar.com/avatar/19029705ec4f254c5ae2f496f95ab3ed7ac14dcd50644788919665119a093974?s=48&d=identicon&r=G)
![Anthony's avatar](https://1.gravatar.com/avatar/19029705ec4f254c5ae2f496f95ab3ed7ac14dcd50644788919665119a093974?s=48&d=identicon&r=G)
![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

#### Archives

#### Categories

#### Meta

[Blog at WordPress.com.](https://wordpress.com/?ref=footer_blog)

![](https://s-ssl.wordpress.com/i/logo/wpcom-gray-white.png?m=1479929237i)

Have a WordPress.com account? [Log in now.](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2016%2F12%2F16%2Fupaxos-unbounded-paxos-reconfigurations%2F&signup_flow=account)

![](https://s-ssl.wordpress.com/i/logo/wpcom-gray-white.png?m=1479929237i)
![](https://pixel.wp.com/b.gif?v=noscript)
