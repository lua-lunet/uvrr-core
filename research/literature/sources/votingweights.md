<!-- source: https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/ -->

# [slash dev slash null](https://simbo1905.wordpress.com/ "Home")

simbo1905’s ramblings about computers

### Paxos Voting Weights

#### by simbo1905

The [last UPaxos post](https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/) took a run through the reconfiguration stall avoiding aspects of the [UPaxos paper](http://tessanddave.com/paxos-reconf-902f8b7.pdf). In this post, we will take a quick look at how the paper describes hot swapping of nodes using voting weights.

So what’s a voting weight? It’s an integer multiplier that a given node has within a Paxos quorum. Traditional Paxos has all nodes have a weight of “1” such that every node is equal. So what can we achieve by varying the weights?

Well, straight away you get “learners” by configuring acceptors with a zero weight. The original Paxos papers decomposed Paxos into proposers, acceptors and learners roles. Learners are nodes that track the outcome of the consensus algorithm without participating in the consensus quorums. They are replicas which can be used to scale read load. Simply applying a zero voting weight makes an acceptor a learner. Better yet if an acceptor dies you can promote a learner by assigning it a voting weight of “1”. Happy days.

What else? Well, it allows us to set up leader casting vote scenarios that we need for [UPaxos cluster reconfigurations](https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/). The paper gives a very practical scenario which is upgrading a server. It points out that cloud providers today typically have three rather than four availability zones in each region. If we just replace a node, by adding the new one then deleting the old one, we pass through the following three states:

| Zone A | Zone B | Zone C |
| --- | --- | --- |
| Server W | Server X | Server Y | Server Z |
| 1 | 1 | 1 | – |
| 1 | 1 | 1 | 1 |
| 1 | 1 | – | 1 |

The problem is that we temporarily have four nodes with two of them in the same availability zone. If we loose Zone C in that state then we only have half the cluster. With the [FPaxos even nodes optimisation](https://simbo1905.wordpress.com/2016/09/30/the-fpaxos-even-nodes-optimisation/), a leader in Zone A or B will continue to process client commands. The problem is that you cannot elect a new lead if it dies.

To get around this we can use voting weights. In the following table if we lose Zone C at any point a leader in Zone A or B can still get a majority:

| Zone A | Zone B | Zone C |
| --- | --- | --- |
| Server W | Server X | Server Y | Server Z |
| 1 | 1 | 1 | – |
| 2 | 2 | 2 | – |
| 2 | 2 | 2 | 1 |
| 2 | 2 | 1 | 1 |
| 2 | 2 | – | 1 |
| 2 | 2 | – | 2 |
| 1 | 1 | – | 1 |

At each step in that process, we either doubled all weights, halved all weights, or changed one node’s voting weight by one. Why? To ensure the majorities between consecutive reconfigurations overlap.

It is fairly obvious that if you double or halve all weights between consecutive configurations a simple majority in each will overlap. Any integer scaling factor has that property. Adjusting only one node by 1 is a little more subtle. This is equivalent to adding or removing one node in a cluster where every node has a voting weight of 1.  If we consider the sequence of integers `{3, 4, 5, 6, 7, ...}` then we can see that consecutive majorities do indeed overlap. If we were to skip a number such as changing from 5 to 7 then majorities wouldn’t overlap and we would have violated the UPaxos safety constraints.

### Share this:

* [Share on X (Opens in new window) X](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?share=twitter&nb=1)
* [Share on Facebook (Opens in new window) Facebook](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?share=facebook&nb=1)

Like Loading...

Published: [March 16, 2017](https://simbo1905.wordpress.com/2017/03/16/)

Filed Under: [Paxos](https://simbo1905.wordpress.com/category/paxos/), [Trex](https://simbo1905.wordpress.com/category/trex/)

### 16 Comments to “Paxos Voting Weights”

1. ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

   [March 29, 2017 at 11:12 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-186)

   This algorithm and paper sounds almost too good to be true !  
    I wonder if anyone is working on a TLA+ implementation of this. Is this what you plan to implement for TREX ?

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=186#respond)

   * ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

     [March 30, 2017 at 7:13 am](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-187)

     @coboler yes I intend to implement dynamic cluster membership as UPaxos. Having talked it though with the author of UPaxos the changes needed to the “library” project of TRex are straight-forward:

     1. Add an “era” int into the BallotNumber case class which defaults to zero. That won’t break any existing tests nor break any existing users of the library.

     2. Change the logic that issues high prepare messages. Currently, that logic updates the current promise prior to broadcasting the message. That logic needs to broadcast without updating the promise. Then on the handler to the responses to the message make the promise only when it is calculated that this will give a majority.

     With those minimal changes, I believe that the UPaxos leader overlap mode can be coded as a message filter and addition state tracking in the “core” project above the library. If I am correct that I can implement UPaxos in that manner then it will be a stunning result. This is because those two changes to the core library are simple enough to easily see that they don’t violate the Paxos Made Simple paper. Anyone else can use the library within their own server and implement cluster membership changes in the old fashioned way.

     Implementing UPaxos like that in the higher layers of the software stack means all it can do is hide messages from the library code. That is equivalent to messages being dropped by the network. As we know any working Paxos library is safe to dropped messages. That will give me high confidence that implementing UPaxos in that manner is not introducing a large surface area for new bugs.

     **Update:** These ideas are now sketched out in working code on a branch of TRex as documented at [UPaxos-Sketch-(April-2017).](https://github.com/trex-paxos/trex/wiki/UPaxos-Sketch-(April-2017)) Step 1 above was a core library change but step 2 is implemented as a message filter at the leader which matches only responses to the final message of the UPaxos reconfiguration.

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=187#respond)
   * ![David Turner (@DaveCTurner)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/3425917658/1764355f5dcfe0621f0469488dee6b72_normal.jpeg?resize=48%2C48&ssl=1) [David Turner (@DaveCTurner)](http://twitter.com/DaveCTurner) says:

     [March 30, 2017 at 10:29 am](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-188)

     Hi coboler,

     Paper author here! Thanks for the enthusiastic comment, and thanks to simbo1905 for his accessible write-ups too.

     There’s one wrinkle to curb your enthusiasm which simbo spotted: abdication to a new leader may still cause a pipeline stall. I can’t see a way around this. This means that replacing a single node is stall-free (if the leader doesn’t change) but replacing your whole cluster requires an abdication at some point as the leader necessarily changes.

     The safety proofs in the paper are formalised in Isabelle/HOL (a theorem prover rather than a model checker) so I’ve minimal doubt that they’re true. More strongly, in fact, Isabelle was instrumental in this work: I don’t think I would have discovered this had I not been trying to formalise something simpler first. I don’t know of anyone trying to translate this to TLA+ but I’m sure it’d be an interesting adventure.

     The liveness proof remains informal for now; in fact I’m working on formalising a stronger liveness property that doesn’t​ rely on an external leader election protocol, again following Raft’s approach. This is in its very early stages at the moment and it may turn out that TLA+ is an appropriate tool for me to use for this job.

     Thanks again,

     David

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=188#respond)

     + ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

       [March 30, 2017 at 5:46 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-190)

       @David

       Given that there is no issue on expanding a cluster without changing the leader I think it’s a great achievement.

       I suspect most systems can find a low point in the calendar to do such routine maintenance for something as big as moving clusters between data centres or replacing all the nodes and the like. Systems that cannot afford any disruptions whatsoever are going to have a spend a disproportionate amount of money to get the extra 9s of reliability.
2. ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

   [March 30, 2017 at 3:30 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-189)

   Ahh thank you both for the clarifications! I look forward to both your next steps.  
    As an exercise, I would like to implement Paxos in C and/or Rust. So seeing an actual implementation of UPaxos a a guide would be very helpful.  
    As an aside, I came across an interesting of Paxos implementation from Tencent. They mention battle-tested but no formal proof.  
    <https://github.com/tencent-wechat/phxpaxos>  
    The interesting part is that they  
    “- Implementing Master election as a state-machine embedded in PhxPaxos  
    – Implementing reconfiguration as a state-machine embedded in PhxPaxos”  
    I have never seen it implemented/mentioned like that before and if given time would love to prove if this idea is sound.

   One again, thanks for your replies ! Much appreciated.

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=189#respond)

   * ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

     [March 30, 2017 at 6:58 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-191)

     @coboler There are some hooks for this already in TRex.

     Commands come in two flavors. ClientCommandValues and ClusterCommandValues. Cluster commands will be sent by an administrator tool. The algorithm simply treats them equally as the next possible value the leader can choose.

     The “deliver” method is called when a command is known to be fixed. Normal Client commands can be passed to the host application to do whatever it does. Administrator Cluster commands can be used to change the cluster membership both in-memory and on disk for crash resilience. The in-memory model of the cluster membership can then be used to compute what is a quorum for the next slots.

     The approach is described in the last paragraph of the paper Paxos Made Simple by the inventor of Paxos. The fact that it says that “an arbitrarily sophisticated reconfiguration algorithm” may be used when values are chosen which change the configuration indicates that yes indeed something as sophisticated as UPaxos is possible.

     The original approach of that paper is described in this post <https://simbo1905.wordpress.com/2015/07/30/paxos-dynamic-cluster-membership-2/>

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=191#respond)
3. ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

   [March 30, 2017 at 9:16 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-192)

   @coboler here is the change to ballot numbers as a first step in supporting UPaxos <https://travis-ci.org/trex-paxos/trex/builds/216660776>

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=192#respond)

   * ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

     [March 31, 2017 at 4:44 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-193)

     @simbo1905

     I am wondering about a few implementation details regarding your game plan and I apologize if I am not clear.  
      I take it that all we need to track during reconfiguration is which peers were acknowledged during prepare from era (e) and to make sure that in the propose phase of (e+1) at least one of the responders is present in both sets. So in essence we are not explicitly partitioning the acceptors into prepare and propose sets but rather tracking their sizes and who responded during each phase.

     If it makes sense, is this correct ?  
      Is the proposal acceptance conditional actually modeled as  
      ((accepted\_count >= Size of Qprop) && count\_of\_intersected\_peers > 0)

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=193#respond)

     + ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

       [March 31, 2017 at 8:36 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-194)

       I am intending for the leader to actually partition the cluster but only at the last step in the reconfiguration which is described at:

       > [UPaxos: Unbounded Paxos Reconfigurations](https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/)

       Where it says:

       > [the leader] introduces an asymmetry by entering into a “leader overlap mode”

       That is describing the leader partitioning the cluster. The leader needs to broadcast the “prepare(b’)” to a minority of nodes to avoid risking a stall if the responses are lost. The leader itself will only promise to it’s own “prepare(b’)” last when it knows this gives it a majority. The leader shouldn’t bother to send any further “accept(\_,b,\_)” to the any nodes it has sent the new “prepare(b’)” as they will reject the messages having promised to the new ballot number in the higher era.

       My gameplan is to just to have the leader order the other nodes in the cluster by their unique number and partition them into two equal sets. It can send the “prepare(b’)” to one half and any further “accept(s,b,v)” to the other half. The leader itself will be the overlap node which gives itself a majority in both quorums.

       All of the above is only required during that last step of the UPaxos reconfiguration. At any other point we can use classic paxos simple majorities or some of the other quorum strategies named in the FPaxos paper such as “grid quorums”. TRex has an interface QuorumStrategy so that how the actual quorum is calculated can be plugged in.
4. ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

   [April 3, 2017 at 1:14 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-196)

   @simbo1905

   Thank you for your replies ! It is a lot clearer now and I am quite eager to read the final implementation.

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=196#respond)

   * ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

     [April 4, 2017 at 8:09 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-197)

     @ coboler

     I have committed some experimental code to a branch on github which does UPaxos in TRex as discussed in the comments on this post. A quick write up is on the TRex wiki at <https://github.com/trex-paxos/trex/wiki/UPaxos-Sketch-(April-2017)>

     Thanks for your interest!

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=197#respond)

     + ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

       [April 5, 2017 at 5:09 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-203)

       That is actually quite helpful !

       I am wondering about two things :  
        – How do we change the voting weight for the peers ? In the UPaxos branch it seems that the leader has data structures for it but how do the peers know ? During the reconfiguration phase(via messages) , do we assign the peers new voting weights and in the event that the current leader actually crashes, can a new leader election can go on ?  
        – from the paper section on voting weights , it is mentioned that the new voting weights will guard against stalling when a leader dies. In the comments here, @DaveCTurner mentions that there may be a stall when that happens. I guess I am unclear on if the leader dying during reconfiguration is the currently unaddressed problem mentioned here in the comments.

       Truth be told, I am a bit embarrassed by the barrage of questions I am sending both of you, but this is very instructive and Paxos seems a bit less daunting now.

       So thank you again @simbo1905 and @DaveCTurner!
5. ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

   [April 5, 2017 at 9:28 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-204)

   @coboler

   > – How do we change the voting weight for the peers? In the UPaxos branch it seems that the leader has data structures for it but how do the peers know?

   The “Membership” data structure defines all of the cluster configuration including the current voting weights and the quorums for prepares and accepts. An administration tool would encode it as json and transmit it to the leader in a “ClusterCommandValue”. The leader is free to choose that value, or a vanilla “ClientCommandValue” from a regular client, or a no-op value, to use in the next accept message for the next slot.

   It seems reasonable that the leader should prioritize system administrator commands over regular client commands. The classic Paxos algorithm fixes it into the next slot and ensures that all nodes are consistent. That new cluster configuration is then in effect for all subsequent slots. In this manner we simply use the classic Paxos algorithm itself to transmit it’s own configuration to all nodes in a strongly consistent manner as described in the last paragraph of the paper Paxos Made Simple.

   You are correct that in the sketch only the leader uses the cluster membership. That is because it is the active party. The other nodes are followers and only respond to the leader in a passive manner. In the final code all nodes will journal the latest membership to disk at the point they learn it is fixed. If a follower times out and manages to become the leader it will use the latest information that it knows.

   > During the reconfiguration phase (via messages), do we assign the peers new voting weights and in the event that the current leader actually crashes, can a new leader election can go on ?

   If the leader dies during the reconfiguration it is no different from a leader dying at any other point. One surviving node will timeout and run the leader takeover protocol prior to leading. It is guaranteed to find the latest fixed cluster configuration value. I am not anticipating doing anything different in such a scenario whether its UPaxos or classic Paxos. Only when the new leader gets a fresh reconfiguration command will it use the casting vote technique of UPaxos. This means that UPaxos will be elegant cluster reconfiguration mechanism logically above the core library which implements classic Paxos as sketched out in Paxos Made Simple.

   I don’t really like the phrase “leader election”. Any node can attempt to lead by running the leader takeover protocol. If it can get back positive responses from a majority of nodes it will lead. One of my first Paxos blog posts “Cluster Replication With Paxos” describes the leader takeover protocol in detail. I don’t really think of it as an election. Rather I think of the nodes voting on the values in each slot. I think of it as the next leader syncing its state with the other nodes in the cluster by asking for a vote on the value at each slot. At the same time it collaborates with the dead leader by completing any outstanding work by choosing all the values it learns as having had being chosen by the last leader. It is less like an election than one node just unilaterally moving from passive to active mode. If two nodes simultaneously attempt to move from passive to active mode a fight breaks out. That doesn’t really sound like a leader election to me.

   > – from the paper section on voting weights, it is mentioned that the new voting weights will guard against stalling when a leader dies.

   The problem it is avoiding is in going from 3 to 4 nodes by adding a new node which is in the same cloud resilience zone as an existing node. Lose that zone and you lose half the cluster. The two surviving nodes cannot get a majority to lead. The fix is to double all voting weights of the original 3 node cluster then add the new node with a weight of 1. I had to carefully check each row in both tables thinking about permutations of crashes to fully understand this point 🙂

   > In the comments here, @DaveCTurner mentions that there may be a stall when that happens. I guess I am unclear on if the leader dying during reconfiguration is the currently unaddressed problem mentioned here in the comments.

   If the leader dies the next leader has to complete the takeover protocol. That is a short stall of several message roundtrips. If a fight breaks out between two nodes attempting to lead the resolution may take many additional message round trips.

   If you are wanting to replace all nodes in the cluster for a hardware upgrade you might upgrade the leader last. That requires that you have the leader running on the old hardware exit the cluster as the final step. So the remaining challenge is whether you can have the old leader abdicate to a new leader without the risk of stalls.

   > Truth be told, I am a bit embarrassed by the barrage of questions I am sending both of you, but this is very instructive and Paxos seems a bit less daunting now.

   Thanks for asking questions. It’s very encouraging to know that someone is interested!

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=204#respond)

   * ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

     [April 6, 2017 at 2:25 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-205)

     Ahh much clearer. Each weight state is an era and you set them using ClusterCommandValues. I can follow along better but don’t seem to grok the last two steps. Why must the newly added server increase it’s voting weight to 2 only for all the servers to go back to 1 at the last step ?

     Is this part of the algorithm only for cases you are replacing a node ? As in if you just wanted to add a node or remove one in one transaction, you wouldn’t have to set weights. The paper does mention that this is for cases where you can lose half your cluster due to correlated failures.

     As for interest , there is definitely​ interest from the students I know . It’s just that even in distributed class they mention Paxos in passing and emphasize how difficult it is. Then they move on to Raft and make us implement it. I was actually attracted to Paxos because the outline seemed very straightforward …and because people kept saying it’s hard 🙂  
      Reading your blog posts are certainly helping and I think I will start implementing it soon based on your writing and Trex. Raft’s strict Leader-Follower is not elegant to me either. I would like to suggest to the professor to make Unbounded Paxos an extra-credit assignment.

     [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=205#respond)

     + ![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G) [simbo1905](https://simbo1905.wordpress.com) says:

       [April 6, 2017 at 9:04 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-206)

       > Why must the newly added server increase it’s voting weight to 2 only for all the servers to go back to 1 at the last step ?

       Because of the properties of integer numbers. If you take a pencil and paper and draw out the table and figure out all the permutations of crashes and quorums for each row you will see the truth of it. I have to confess I missed “why” myself until I looked hard at this and thought through the combinations. As we are dealing with integers the pattern which emerges is easily extrapolated to all integer numbers.

       > Is this part of the algorithm only for cases you are replacing a node ?

       Changing a single node is just one desired outcome of altering the cluster membership. Swapping one node can be repeated multiple times to swap all nodes so is a powerful real world example. Once we understand how it works we can extrapolate it to other “routine” scenarios such as expanding it contracting a cluster.

       > they mention Paxos in passing and emphasize how difficult it is. […] I was actually attracted to Paxos because the outline seemed very straightforward …and because people kept saying it’s hard

       Exactly! How can something so mechanically simple not be something of elegance? When people say its hard how can we not be curious?

       Distributing computing is hard because of the permutations of things that can go wrong. A mechanically simple algorithm is a big bonus. The effort to work out the how and why it works is time very well spent.

       > I think I will start implementing it soon based on your writing and Trex.

       You mention C or Rust. My thought would be to use Rust as it focuses on safety which fits well with Paxos. I am sure that you would spot many improvements to TRex if you tried to transcribe it to another language.

       > Raft’s strict Leader-Follower is not elegant to me either.

       🙂

       > I would like to suggest to the professor to make Unbounded Paxos an extra-credit assignment.

       An excellent idea! I am happy to speak to your professor and to present the topic to a class over skype. Drop me an email at my id at 60hertz.com
6. ![coboler's avatar](https://2.gravatar.com/avatar/ba378ee2204a13fb0ff80faf48e201505a9e9d1727f27e06716f96e8416771dc?s=48&d=identicon&r=G) coboler says:

   [April 6, 2017 at 10:04 pm](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comment-207)

   @simbo1905

   Oh I ended my Distributed Class almost 2 years ago 🙂  
    I meant it more in a general way that Professors should look into this along with teaching TLA+. Apologies for the confusion.

   I now work in computational geometry space but always wanted to get a good handle on distributed systems with a particular fascination with fault tolerance so I am quite grateful for blogs like these.

   Rust is a solid option even though GoLang is the instructional language of choice for dist systems courses.

   Thanks again!

   [Reply](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/?replytocom=207#respond)

### Leave a comment [Cancel reply](/2017/03/16/paxos-voting-weights/#respond)

[« Previous Post](https://simbo1905.wordpress.com/2017/02/24/cassandra-for-shared-media-libraries/)

[Next Post »](https://simbo1905.wordpress.com/2017/04/04/trex-upaxos-experimental-code/)

[Blog at WordPress.com.](https://wordpress.com/?ref=footer_blog)

* [Comment](https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/#comments)
* Reblog
* Subscribe Subscribed

  + [![](https://s2.wp.com/i/logo/wpcom-gray-white.png?m=1479929237i) slash dev slash null](https://simbo1905.wordpress.com)* Already have a WordPress.com account? [Log in now.](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2017%2F03%2F16%2Fpaxos-voting-weights%2F&signup_flow=account)
* + [![](https://s2.wp.com/i/logo/wpcom-gray-white.png?m=1479929237i) slash dev slash null](https://simbo1905.wordpress.com)
  + Subscribe Subscribed
  + [Sign up](https://wordpress.com/start/)
  + [Log in](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2017%2F03%2F16%2Fpaxos-voting-weights%2F&signup_flow=account)
  + [Copy shortlink](https://wp.me/p2EIyU-1pz)
  + [Report this content](https://wordpress.com/abuse/?report_url=https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/)
  + [View post in Reader](https://wordpress.com/reader/blogs/39257092/posts/5429)
  + [Manage subscriptions](https://subscribe.wordpress.com/)
  + Collapse this bar
