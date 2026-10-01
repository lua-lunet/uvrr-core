<!-- source: https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/ -->

# [slash dev slash null](https://simbo1905.wordpress.com/ "Home")

### The network is faster than the disk

#### by simbo1905

In distributed systems, contrary to popular belief, the local disk may not always be faster than the network. In a future post we will seen why this fact makes Viewstamped Replication Revisited (VRR/[VRR-2012](https://dspace.mit.edu/entities/publication/80846d94-fcd3-40e6-87fb-8d91fe99a5d1)) the queen of the hill of all strong consistency algorithms. I recently came across two articles that tested latencies on AWS that demonstrate the often-overlooked opportunities of modern data centre networks.

It is important to carefully test the performance characteristics of both local disk and network resources when designing and deploying distributed systems. With that disclaimer in place article [Log sync latency explained](https://dev.to/yugabyte/log-sync-latency-explained-4m94) checked on an EC2 vm instance the latency of fsync. They measured 3ms for 256 kb. It took 20ms to sync a 1.6M WAL log. That is to my mind shocking slow. Yet it demonstrates that commodity instances are meant to run caching web servers or stateless applications servers. When you want to deploy a reliable database on cloud that the cloud vendor supports the costs shoot up. This fact is completely unnoticed by the average developer yet it’s a very real cost barrier to running a “bet your business on it” solution. Just as you cannot escape the speed of light you cannot escape the profit margins that cloud provider as to have paying enterprise clients subsidize tens of millions of free tier cloud users.

The article [Measuring Latencies Between AWS Availability Zones](https://www.bitsand.cloud/posts/cross-az-latencies/?t) tested across availability zone ping times across all regions. It found sub-ms latency was normal. Only some remote and slowest regions has 2.5ms latency.

This suggests that we really do not want to use consensus algorithms that force the disk like Raft or Paxos if we can use a consensus algorithm like Viewstamped Replication (VSP).

There is an amazing paper review [Paper #74. Viewstamped Replication Revisited](https://youtu.be/Wii1LX_ltIs?si=0_ng8FOv8qJ4cGtq) by one of the TigerBeetle team that explains VSP for a high-performance database.

Given that the modern paper on VRRwas published in 2012, and referenced in the Raft paper, it would seem that it was the state-of-the-art for more than a decade now. It was great to learn that TigerBeetle DB is putting it into production. What I find surprising is why everyone else seems to have overlooked that [VRR-2012](https://cf003.cdn.4science.cloud/4s-assetstore-prod--mit--dspace/137208522401284443530908491331331971826?response-content-type=application%2Fpdf&response-content-disposition=attachment%3B+filename%3D%22MIT-CSAIL-TR-2012-021.pdf%22&Expires=1787386568&Signature=OaIG7RCHZ-mCqGePU4ggbQ7Rlcloomgj-WE-7PKW~J6fyaPYGoKt0PFM6MkeVCNAsO86SH3tmkHoj8TH9BvyyOuIfv4Z7uZevf-Jmlq0bqt340BHNYkHS9G3CzDe9uY9xH~QEjgLvZuSVLhTQtXYvRytoyDIv09M8WwqnY0OvESK6ZHLpeoG4-bws9Bu77zJOJ9GA-Cb6gwKFuJSiuF6oIqSCR~7b~XO38MGk40NlJp0T5Nvq87zN2fUtd8JcTtTBSb8rcdKK0au9RoLOlYwbCAddmkrEwHYNa6-3zNP~THyN9MZax3p50drP~oe3nZVI4kWTYtq1~-SEpjk3hFuzQ__&Key-Pair-Id=K1M0LPFHJMTXJX) is has a very significant competitive advantage.

### Share this:

### Leave a comment [Cancel reply](/2024/04/12/the-network-is-faster-than-the-disk/#respond)

Δ

#### Recent Posts

#### Recent Comments

|  |  |
| --- | --- |
| [simbo1905's avatar](https://simbo1905.wordpress.com) | [simbo1905](https://simbo1905.wordpress.com) on [Blockchain for Finance is Boll…](https://simbo1905.wordpress.com/2020/05/13/blockchain-for-finance-is-bollocks/comment-page-1/#comment-1077) |
| Anthony's avatar | Anthony on [The Trial Of Paxos Algorithm](https://simbo1905.wordpress.com/2014/10/05/in-defence-of-the-paxos-consensus-algorithm/comment-page-1/#comment-1075) |
| Anthony's avatar | Anthony on [The Trial Of Paxos Algorithm](https://simbo1905.wordpress.com/2014/10/05/in-defence-of-the-paxos-consensus-algorithm/comment-page-1/#comment-1074) |
|  | [Cluster Replication…](https://simbo1905.wordpress.com/2014/10/28/transaction-log-replication-with-paxos/) on [Pre-voting in distributed cons…](https://simbo1905.wordpress.com/2017/08/22/pre-voting-in-distributed-consensus/comment-page-1/#comment-1073) |
| [allenling (@Allen_Ling3)'s avatar](http://twitter.com/Allen_Ling3) | [allenling (@Allen\_Li…](http://twitter.com/Allen_Ling3) on [Cluster Replication With Paxos](https://simbo1905.wordpress.com/2014/10/28/transaction-log-replication-with-paxos/comment-page-1/#comment-1000) |

![simbo1905's avatar](https://1.gravatar.com/avatar/72a08f4ac662dd802f976862cc5b01c043a593e7c29183f5ac287afe9eb6aa0b?s=48&d=identicon&r=G)
![Anthony's avatar](https://1.gravatar.com/avatar/19029705ec4f254c5ae2f496f95ab3ed7ac14dcd50644788919665119a093974?s=48&d=identicon&r=G)
![Anthony's avatar](https://1.gravatar.com/avatar/19029705ec4f254c5ae2f496f95ab3ed7ac14dcd50644788919665119a093974?s=48&d=identicon&r=G)
![allenling (@Allen_Ling3)'s avatar](https://i0.wp.com/pbs.twimg.com/profile_images/480696220260659200/1vZ-PswH_normal.jpeg?resize=48%2C48&ssl=1)

#### Archives

#### Categories

#### Meta

[Blog at WordPress.com.](https://wordpress.com/?ref=footer_blog)

![](https://s-ssl.wordpress.com/i/logo/wpcom-gray-white.png?m=1479929237i)

Have a WordPress.com account? [Log in now.](https://wordpress.com/log-in?redirect_to=https%3A%2F%2Fsimbo1905.wordpress.com%2F2024%2F04%2F12%2Fthe-network-is-faster-than-the-disk%2F&signup_flow=account)

![](https://s-ssl.wordpress.com/i/logo/wpcom-gray-white.png?m=1479929237i)
![](https://pixel.wp.com/b.gif?v=noscript)
