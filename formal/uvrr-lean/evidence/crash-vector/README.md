# Crash-vector collection: a component, not a recovery repair

Primary sources, independently read on 6 September 2026:

- Ellis Michael, Dan R. K. Ports, Naveen Kr. Sharma, Adriana Szekeres,
  *Providing Stable Storage for the Diskless Crash-Recovery Failure Model*,
  UW-CSE-16-08-02, 25 August 2016.
  https://homes.cs.washington.edu/~drkp/papers/diskless-tr16.pdf
- The same authors, *Recovering Shared Objects Without Stable Storage
  [Extended Version]*, UW-CSE-17-08-01, 5 August 2017; extended version of
  their DISC 2017 paper, updating the earlier report.
  https://drkp.net/papers/recovery-tr17.pdf

Downloaded PDF hashes are in sources.json. Appendix B.1 and Figure 1 of the
2017 report already describe the delayed view-change/recovery failure pattern.
Our public Rust witness additionally drives both primaries to different
committed operations. It supplies repository-specific evidence, not a new
literature discovery of this failure class.

The new Lean module implements the pointwise vector join and reply pruning
from Algorithm 1 and proves Definition 6's pairwise incarnation condition.
It counts identities as a set, so duplicates cannot manufacture quorum size.
A two-of-three example accepts an inconsistent pair when pruning is omitted,
rejects the stale reply with pruning, and accepts a replacement current reply.
The mutation harness also removes pruning in a temporary source copy and
requires Lean rejection after the unchanged copy compiles.

This does not mechanize the paper's quorum-knowledge persistence argument.
Recovery must acquire and propagate its own new incarnation through a
crash-consistent quorum, not merely filter ordinary VRR recovery responses.
The publication's acquisition protocol also handles per-request freshness,
resending after pruning, value reconstruction, and operational exclusion.
Those layers, the temporal quorum-knowledge induction, and composition with
uVRR are unproved here. The supplied replacement reply demonstrates collector
progress only. It is not evidence of eventual network delivery or full service
availability. Published liveness assumes a suitable stable quorum over the
acquisition interval; at most one recovering node at a time is insufficient
as a blanket termination premise under arbitrary churn.

No production change or repair is claimed. The existing public-path
counterexample remains the required regression target. No model-service call
was made for this component.
