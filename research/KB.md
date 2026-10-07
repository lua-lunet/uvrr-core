# The knowledge base, and how it stays sane while moving fast

This is the position on where the paper's knowledge lives, what is master of
what, and how the whole thing survives speed and mistakes. It is the contract
the tooling already enforces, stated so the next person does not have to
rediscover it.

## One master per class of state

There is exactly one master for each class of state, and everything else is a
derived index or a copy brought up to date by an explicit act.

| state | master | everything else |
|---|---|---|
| the paper's prose | `papers/fragments/` in git, one file per paragraph, rendered in order by `paper/paper.md.j2` | the graph index under `.kg/` is derived; the render is checked byte for byte against the files |
| the bibliography | the Zotero library | one Better BibTeX auto-export snapshot, `paper/papers/references.bib`, replaced wholesale from the export, never hand-edited |
| the corpus PDFs | git, under `research/agent366/papers/` | the reading folder holds byte-identical copies, verified by sha256 |
| the cold copy | `~/icloud/2026/UVRR/kb`, append-only, content addressed | derived from git; a rebuild is always available |

One master means one place to fix a mistake. A copy that disagrees with its
master is stale, not authoritative, and the fix is to bring the copy up to
date, never to edit the copy into a second truth.

## Worktrees are replicas, not masters

`main` is the release line and the one place Zotero syncs to, forever. Any
other worktree updates the bibliography by copying the file out of `main`
with `cp`, referencing that copy, and committing it. The same eventual
consistency applies to everything: a worktree builds, tests, commits and
pushes, and the state lands back on `main` through a merge. No worktree is a
second export location.

## The rules that keep it sane

- **Files are the content; the database is the index.** The golden source of
  the paper is files in git. The graphlite index under `.kg/` is derived and
  gitignored, so losing it costs nothing: it is rebuilt from the fragments
  and the export.
- **The index is never hand-edited and the database is never written
  directly.** Zotero is driven only through its HTTP API; its sqlite file is
  opened `mode=ro`, and only to read the local server id.
- **Credentials never enter git.** The API key lives in `.env` as
  `ZOTERO_API_KEY`, which `.gitignore` excludes, and is never printed.
- **A new revision is a new row.** The cold mirror is append-only: rows are
  never deleted or overwritten, and an object is written once under its own
  content hash. A mistaken revision can only add to the mirror, never damage
  it, which is what makes maintaining it safe.
- **Determinism is the proof.** The extractions re-run byte-identical, the
  renderer is byte-identical in file and graph modes, and the check commands
  exit non-zero on any fault, so they can gate.

## Recovery, when it is needed

Recovery is a read, not an archaeology project.

- Lost prose: the fragment files in git are the content; the mirror holds the
  same bytes under their hashes and is greppable without any tooling.
- Lost index: rebuild it from the fragments and the export; the loading
  tooling is recorded in the repository's history.
- Lost bibliography: re-export from Zotero and replace the snapshot wholesale.
- Lost reading folder: copy the corpus out of git; the copies are verified
  byte-identical by construction and by check.

Nothing master-only lives in scratch space, in a worktree, or only inside the
database. If a thing exists in exactly one place, that place is git or the
Zotero library.

## Deliberately deferred

- **Fork, deprecate and the other revision semantics.** The mirror's encoding
  carries the idea of a set of revisions and no more; the version column is a
  unix epoch standing in for a revision number, and two revisions may share
  one.
- **The version model in the database.** Revisions will become an encoding in
  graphlite when the formalism is refactored; until then the mirror is the
  record and the mirror is honest about being scaffolding.
- **The automatic hook.** The mirror is maintained by its own tool rather
  than by the loader's write path; wiring it in is deferred until the
  refactor, because the loader's write path is what will change.
