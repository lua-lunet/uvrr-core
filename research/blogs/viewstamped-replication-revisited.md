# Unbounded Viewstamped Replication Revisited?

Source: https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/
Author: simbo1905 (slash dev slash null)
Fetched: 2026-09-05 via tavily extract (markdown). Site boilerplate removed.

---

Asks yourself whey is there no open-source proofs that it is practical to so strong consistency with single digit millisecond primary fail over while supporting none stop cluster reconfigurations such as adding or removing cluster nodes?

Just because you can do something, doesn't mean you should do something, yet if no-one else has proven they have done it, surely you just gotta do it? (simbo1905, August 2026)

Timing is everything. It is 2012 and you had just published a way to do strong consensus without flushing the disk. You were two years ahead of the RAFT paper. So surely your work is going to be the defacto standard for strong consistency, right? Srsly?! Humf!

Two years before the RAFT paper, Barbara Liskov and James Cowling published a technically superior algorithm, Viewstamped Replication Revisited (VRR-2012). This improved on the earlier version, Viewstamped Replication: A New Primary Copy Method to Support Highly-Available Distributed Systems by Barbara H. Liskov and Brian M. Oki (VSR-1988), which predates [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf) by Leslie Lamport (2001). The blinder on VRR-2012 that makes it technically superior is that you don't have to force the disk for crash safety, as long as you accept an additional network round trip. Now obviously there is no free lunch. We might guess that by not eagerly flushing the disk, we require a crashed node to contact a majority of nodes in the cluster to rejoin correctly. If multiple nodes crash and try to rejoin, things may get messy due to failed state transfers during repeated leader timeouts.

What surprises I is that only the total legends at TigerBeetle have tried to take the VRR high road. As at August 2026, Tigerbeetle as a finance-focused database, is a great product without supporting dynamically changing the cluster size. Banking systems typically use a 24×6 model. We patched and restarted database clusters on a day 7 on a bi-weekly or monthly cycle. As long as enough of six production servers remained up we would wait to do unscheduled maintenance at the next marker close.

As of mid-2026, I can not find anyone attempting nonstop cluster reconfigurations with VRR-2012. That's been on my bucket list for a while. Some people even have skiing off the top of Mount Everest on their bucket list. It's not something you casually fit into your hobby time. A lot of people had to support the successful ski descent off Everest, which I had to watch in its entirety, because it's simply a stunning achievement.

Similarly, the challenge this post discusses requires a team effort. Proving utility beyond a reasonable doubt will require a commercial-grade amount of effort. Yet I didn't have an extremely low-latency, nano-state, strong-consistency problem with a commercial tie-in to solve until now.

That's enough for now.

TBC

---

## Comments

(none — comments section was empty; "Leave a comment" form only)
