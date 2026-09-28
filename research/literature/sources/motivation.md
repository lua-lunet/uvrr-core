<!-- source: https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/ -->

# [slash dev slash null](https://simbo1905.wordpress.com/ "Home")

simbo1905’s ramblings about computers

### Unbounded Viewstamped Replication Revisited?

#### by simbo1905

Asks yourself whey is there no open-source proofs that it is practical to so strong consistency with single digit millisecond primary fail over while supporting none stop cluster reconfigurations such as adding or removing cluster nodes?

> Just because you can do something, doesn’t mean you should do something, yet if no-one else has proven they have done it, surely you just gotta do it? (simbo1905, August 2026)

Timing is everything. It is 2012 and you had just published a way to do strong consensus without flushing the disk. You were two years ahead of the RAFT paper. So surely your work is going to be the defacto standard for strong consistency, right? Srsly?! Humf!

Two years before the RAFT paper, Barbara Liskov and James Cowling published a technically superior algorithm, Viewstamped Replication Revisited (VRR-2012). This improved on the earlier version, Viewstamped Replication: A New Primary Copy Method to Support Highly-Available Distributed Systems by Barbara H. Liskov and Brian M. Oki (VSR-1988), which predates [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf) by Leslie Lamport (2001). The blinder on VRR-2012 that makes it technically superior is that you don’t have to force the disk for crash safety, as long as you accept an additional network round trip. Now obviously there is no free lunch. We might guess that by not eagerly flushing the disk, we require a crashed node to contact a majority of nodes in the cluster to rejoin correctly. If multiple nodes crash and try to rejoin, things may get messy due to failed state transfers during repeated leader timeouts.

What surprises I is that only the total legends at TigerBeetle have tried to take the VRR high road. As at August 2026, Tigerbeetle as a finance-focused database, is a great product without supporting dynamically changing the cluster size. Banking systems typically use a 24×6 model. We patched and restarted database clusters on a day 7 on a bi-weekly or monthly cycle. As long as enough of six production servers remained up we would wait to do unscheduled maintenance at the next marker close.

As of mid-2026, I can not find anyone attempting nonstop cluster reconfigurations with VRR-2012. That’s been on my bucket list for a while. Some people even have skiing off the top of Mount Everest on their bucket list. It’s not something you casually fit into your hobby time. A lot of people had to support the successful ski descent off Everest, which I had to watch in its entirety, because it’s simply a stunning achievement.

Similarly, the challenge this post discusses requires a team effort. Proving utility beyond a reasonable doubt will require a commercial-grade amount of effort. Yet I didn’t have an extremely low-latency, nano-state, strong-consistency problem with a commercial tie-in to solve until now.

That’s enough for now.

[Read more: Unbounded Viewstamped Replication Revisited?](https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/)

TBC

### Share this:

* [Share on X (Opens in new window) X](https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/?share=twitter&nb=1)
* [Share on Facebook (Opens in new window) Facebook](https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/?share=facebook&nb=1)

Like Loading...

Published: [August 12, 2026](https://simbo1905.wordpress.com/2026/08/12/)

Filed Under: [Uncategorized](https://simbo1905.wordpress.com/category/uncategorized/)

### Leave a comment [Cancel reply](/2026/08/12/viewstamped-replication-revisited/#respond)

[« Previous Post](https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/)

[Blog at WordPress.com.](https://wordpress.com/?ref=footer_blog)

* [Comment](https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/#respond)
* Reblog
* Subscribe Subscribed

  + [![](https://s2.wp.com/i/logo/wpcom-gray-white.png?m=1479929237i) slash dev slash null](https://simbo1905.wordpress.com)* Already have a WordPress.com account? [Log in now.](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2026%2F08%2F12%2Fviewstamped-replication-revisited%2F&signup_flow=account)
* + [![](https://s2.wp.com/i/logo/wpcom-gray-white.png?m=1479929237i) slash dev slash null](https://simbo1905.wordpress.com)
  + Subscribe Subscribed
  + [Sign up](https://wordpress.com/start/)
  + [Log in](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2026%2F08%2F12%2Fviewstamped-replication-revisited%2F&signup_flow=account)
  + [Copy shortlink](https://wp.me/p2EIyU-2gs)
  + [Report this content](https://wordpress.com/abuse/?report_url=https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/)
  + [View post in Reader](https://wordpress.com/reader/blogs/39257092/posts/8708)
  + [Manage subscriptions](https://subscribe.wordpress.com/)
  + Collapse this bar
