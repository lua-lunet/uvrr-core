# uvrr-audit — historical audit record (marked historical, item24)

The files in this directory are timestamped audit history from the classic
crash-recover (CR) research attempt: the delayed-fence counterexample logs
(`delayed-fence-*`, `recovery-*`), the recovery-literature survey, and the
claim-witness proofs. uVRR eliminates the CR amnesia class by construction via
Crash-Stop-Self-Evict reincarnation (a dirty node reopens under a new
incarnation, the leader evicts the old identity through the forced weight
sequence, and the new identity rejoins as a weight-0 learner). The witnesses
below therefore describe the classic crash-recover attempt that was removed,
not an open bug in uVRR; they are retained as dated evidence of how the claim
arose and how it was resolved, not as statements about the current core.
