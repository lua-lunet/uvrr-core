# item20a — Docs follow-ons from the item20 sweep

**RULINGS RECORDED 2026-08-15** (owner directive: "the docs item must be fixed
before the PR is merged"):

1. **§14 — amended in place.** The section is titled "Current implementation
   assessment", which declares it a living assessment, so correction in place is
   the right form. The falsified claims were rewritten against the verified
   code: FFI clone-and-stage / panic-poisoning removed (no FFI module exists);
   §14.2 now lists only the genuinely missing contract (the normative C ABI
   transition-ownership contract); the `Replica::new` amnesiac-voter paragraph
   now records that the constructor is gone and `provision`/`reopen` start
   fenced `Recovering`.
2. **§16 — no change.** The reference list is bibliographic; the
   forbidden-identifier gate covers `src/` and `tests/` only. Keep the URLs.
3. **§6.1 provenance — added.** One sentence: only the latest fenced view's
   primary's suffix is installation evidence; any other responder's suffix
   counts toward the quorum but never installs. Also folded in the within-life
   upcall-suppression sentence, whose ruling has since landed in code.

Done by the outer agent; `docs/vrr-durability-model.md` staged. This item is
complete; the text below is the original deferred scope for the record.

---

**Follow-on work to item20** (deferred by the item20 agent; not done). Three
loose ends in `docs/vrr-durability-model.md` / `docs/architecture.md`. None is
urgent for the TLA+ agents' current read (the recovery semantics are synced);
each needs a decision before editing.

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## 1. §14 sync against HEAD — BLOCKED on owner

`docs/vrr-durability-model.md` §14.1/§14.2 (lines ~825-849) is falsified by HEAD:
it still cites `Replica::new` (removed; `provision`/`reopen` start fenced
`Recovering`), the pre-S4 clock contract wording, and a "clone-and-stage FFI /
panic poisoning" claim the item20 agent could not verify from the recovery
ground truth. The owner must rule whether §14 is a LIVING assessment (sync it
now — requires a full audit of each listed contract against the code) or a DATED
record (amend in the §8.7.3/A1 pattern with a dated note). No edit before the
ruling.

## 2. §16 reference-list URL hygiene

Five pre-existing committed reference URLs in
`docs/vrr-durability-model.md:875-879` contain the substring the no-`'paxos'`
grep matches. The verification gate greps only `src/` and `tests/`, so nothing
fails; the question is whether the title-only citation convention extends to the
reference list (drop or rehost the URLs). Owner's call; the edit itself is
mechanical once ruled.

## 3. §6.1 primary-only suffix-installation provenance

The claim exists only in code comments (`src/replica/recovery.rs`); neither doc
states it. If the TLA+ specification needs it from the docs, adding it is a
small owner-approved doc addition. Record the decision either way.

## Constraints

- `docs/` only. Never `README.md`, `AGENTS.md`, `Makefile`, `.dockerignore`,
  `formal/`, `Dockerfile.tla`, `.tmp/`, or any `src/`/`tests/` file.
- No edit for any of the three before its ruling is recorded in this file.
- `git add` only the docs paths touched. Never commit. No stash/reset/checkout.

## Report

The rulings quoted, per-hunk edits, gate output verbatim
(`rg -in 'paxos' docs/` result stated honestly), `git status --porcelain`,
follow-ons not done.
