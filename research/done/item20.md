# item20 — Stale-docs sweep (owner-directed, immediate)

**Newly discovered work** (owner directive 2026-08-14): tracked documentation may
be out of date with the target design, and parallel agents writing a TLA+
specification are reading it NOW. Run a deleting-dead-code pass over the tracked
docs. Excision of stale claims is preferred over expansion; corrections are minimal
and in the document's existing register.

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## In scope (tracked docs only)

- `docs/architecture.md` (482 lines)
- `docs/vrr-durability-model.md` (916 lines)

## Explicitly OUT of scope — do not open, edit, or stage

- `README.md` — the parallel TLA+ agents have it modified in the working tree.
- `AGENTS.md` — rules, not design documentation.
- Anything untracked: `formal/`, `Dockerfile.tla` (the TLA+ work itself).
- `.dockerignore`, `Makefile` — carry the parallel agents' unstaged changes.
- `.tmp/` — never staged, never committed.
- Any `src/` or `tests/` file — docs only in this item.

## The design deltas the docs must reflect (committed design, HEAD 93a425f)

Verify every claim against the code (`src/replica/recovery.rs`, `src/replica/mod.rs`
headers) — the code is ground truth where they conflict:

1. **Recovery nonce is a bounded SET, not a single value.** A recovery attempt
   retains up to `MAX_RECOVERY_NONCES` (8) nonces, one per host re-drive; the
   re-drive inserts the fresh tick and preserves collected responses. A
   `RecoveryResponse` counts iff its nonce is in the set; responses across
   different in-set nonces combine into one `R_g` quorum; an evicted nonce's
   response is stale exactly like one to an attempt that never ran. The set is
   volatile — a crash discards it.
2. **Known-stale hunks** (verify, then correct or excise):
   - `docs/vrr-durability-model.md:109` — volatility table row: "Prevents mixing
     distinct recovery attempts".
   - `docs/vrr-durability-model.md:209` — "prevents a delayed response from an
     earlier recovery attempt being counted in the current attempt": false under
     the set semantics; the set admits delayed responses to in-set re-drives of
     the SAME episode. What the set excludes: responses to nonces the attempt
     never minted or already evicted.
   - `docs/vrr-durability-model.md:211-226` — the §6.1 invariant and host-clock
     table. Host-side freshness (a fresh tick per re-drive) remains true; any
     claim that a delayed response to an earlier in-life attempt is discarded is
     false. If the invariant text needs a design ruling rather than a doc edit,
     STOP and report that hunk instead of rewriting it.
   - `docs/architecture.md` S4 (lines ~287-307) and the line-50 mention: the
     "tick is the nonce" decision stands; the rationale citing single-nonce
     discard semantics does not.
3. **Committed fast-forward while `Recovering`.** An accepted `RecoveryResponse`
   whose `committed` exceeds the local frontier advances it: takeWhile over
   sequentially-adjacent locally journal-present slots, ordered `Apply` upcalls,
   idempotent to duplicates, stops at the first gap. `committed`/`applied` are
   monotone across fast-forward and completion.
4. **Within-life duplicate `Apply` suppression (owner ruling 2026-08-14).** A
   completion's replay walk does not re-emit slots already emitted this life by
   the fast-forward. The §11.1 crash boundary is unchanged: after a crash the
   volatile emission memory is gone and replay re-emits from the durable
   `applied`. Sweep the docs for any at-least-once claim that contradicts the
   within-life suppression and correct it. (item18b is implementing the code
   change in parallel; write the docs to the RULED design, not the code's current
   intermediate state.)
5. §6.1's primary-only suffix installation is UNCHANGED — preserve every claim
   to that effect.

## Method

- Sweep both files end-to-end, not only the named hunks: any claim that
  contradicts the committed design is dead text — excise or minimally correct it.
- Keep spec-section citations (§6.1, §8.3, §10, §11.1) and decision ids (S4 etc.)
  consistent with the surviving text; if a section number is cited by surviving
  text, do not renumber sections.
- No new files. No ticket identifiers. Never the word 'paxos'.

## Verification

- `git add docs/architecture.md docs/vrr-durability-model.md` (only those paths).
  Never commit. Never stage anything else. No stash/reset/checkout.
- `rg -in 'paxos' docs/` — empty.
- Do NOT run `cargo test` as a gate for this item: the tree intentionally carries
  one preserved Red reproduction (`completion_before_acknowledgement_does_not_
  reemit_fast_forwarded_upcalls`) while item18b is in flight. Docs edits do not
  affect the suite.
- `git status --porcelain` before and after: confirm only the two docs paths are
  staged by you, and the parallel agents' paths (`README.md`, `Makefile`,
  `.dockerignore`, untracked `formal/`, `Dockerfile.tla`) are untouched.

## Report

Per-hunk verdict (excised / corrected / verified-still-true) with the reasoning,
any hunk that needs an owner ruling (quoted, not rewritten), gate output verbatim,
`git status --porcelain`, follow-ons not done.
