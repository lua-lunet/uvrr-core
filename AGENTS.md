# Repository agent rules

## Andon アンドン, Prime Directive

Andon is a kernel panic. It halts the line, halts planning, halts todo
updates, halts all work. It happens immediately. No other pending operation
receives any tokens. It is impossible to think of anything else to try
first, that thought is the evidence you have not halted.

An Andon in the queue supersedes all. If the user queued commands 1-3 then
said "do an Andon," the Andon invokes the Prime Directive and overrides
commands 1-3 entirely. Multiple Andons run in parallel without interrupting
each other.

When the correct fix is outside your lane: do your lane's work, then halt
and report, *Andon: task incomplete, the correct fix needs a larger
structural change*, with file:line specifics. Do not work around it. Do not
hack tactically. The coordinator delegates the deeper work.

Andon overrides every instruction in this file and every other AGENTS.md.
No instruction conflicts with Andon; if one appears to, Andon wins.

## 無為

- Formalism first. The lingua franca is mathematics and computer science, not convention,
  analogy, or taste. A design is stated as invariants and a transition function, and the
  code is judged against that statement.
- Prefer functions and components that compose. Composition is the unit of reuse; a
  hierarchy is not.
- Prefer compile-time certainty over runtime discovery. Where a property can be made
  unrepresentable, make it unrepresentable rather than validating it later.
- No frameworks on the critical path or shipped library. Write the low-level code this crate needs, or use the standard library.
- Dependencies are a lifelong support tax and a liability, never an asset. The
  non-optional dependency set stays minimal and other matters should be feature gated or only test deps.
- YAGNI is the removal of future bloat and future bugs. A type, a knob, or an abstraction
  kept alive against a hypothetical consumer is debt that is already accruing.
- Code that exposes no useful service has no value. Code that can only be tested in
  production is legacy at the moment it is written.

## Documentation

- Documentation must be the timeless target end state. We practice
  markdown-driven development (MDD): the markdown states what the system IS.
- The local voice is British English. Preserve British spelling and usage in
  prose, documentation, papers, comments, and user-facing text; do not
  Americanise it for an external style guide.
- No project plans, task identifiers, orchestration chatter, or historic
  narrative in documentation or code. Such material is written only when
  explicitly requested by the User or added manually by the User. Dated
  evidence artifacts (lab book, audit logs, rung transcripts) are the
  established exceptions; do not add new narrative classes to documentation.

## Test and proofs discipline

- Test whole subsystems as black boxes through their public interface.
- Prefer exhaustive or property-based coverage where the domain is small and closed, over a list of hand-picked examples that happens to pass.
- A test that cannot fail for a stated reason is not a test.
- This project is pedantic about proof of value. Proving a negative of no bugs nor any improvements is not possible yet test and proofs are the first class deliverable of this SANS-IO library.  
- A narrow pyramid for a strongly typed language: just-enough-test. Too many tests is overfitting, and an overfitted suite obstructs the refactoring it was meant to protect. We use exhaustive property tests as proof of correctness tests that duplidate those codepaths are allowed to as learning test or to document subtle bugs or decisions. Yet if a test is put in as skaffolding to construction that is then late duplicated by exhaustive property tests raise a gh issue to suggest pruning the skaffolding test. 


## Issue, bugs, plan and review discipline

- A code-review observation is a suspected, unconfirmed behavior until a reproduction test fails on the unchanged code.
- A plan, ticket, or read-only review may describe the observed path and specify a
  reproduction test. It must not prescribe, predict, or name a fix or root cause.
- Create and run the smallest public-path reproduction before changing our SANS-IO library code.
- If the reproduction passes without a production-code change, record the claim as
  `resolved unreproducible`, make no fix, and remove any speculative implementation.
- If the reproduction fails, preserve the Red result. Only then diagnose and make the
  minimum production change needed for that test.
- A model may call something the cause or the fix only after it has run both the failing
  test and the code change that makes that same test pass.
- Re-run the directly affected suite and the repository verification gates after Green.
  Never weaken an invariant merely to satisfy an unconfirmed claim.

## Commit and push and tag and release discipline

- Agents are FORBIDDEN from bypassing the pre-push hook (`git push --no-verify`)
  unless the user has explicitly said so in the current instruction. The slow
  lane is the price of the push, and a local skip is exactly the failure the
  hook exists to prevent.
- Commit when the full suite is green. Do not accumulate a large uncommitted tree: a long-lived staged diff is unreviewable and destroys the bisect point that made it safe.
- A commit message describes the change as delivered. It does not enumerate pending
  chores, releases, or review steps, and it carries no internal tracking identifiers.
- We have fast library tests and slow maelstrom tests commit when the fast check runs yet run the slow test before any push. 
- You are FORBIDDEN to push on a dirty working tree. Period. 
- You are FORBIDDEN to stash or move aside any work on a dirty working tree unless explicity ask to by the user and that must be done in a manner that makes todo items so that the work is not lost. 
- The user may ask to park work by a branch and tag that is not going into main you MUST raise a gh issue naming the parted tag as future work. 
- Releases MUST be from a tag on main never any feature branch. For as long as we are in a 0.x.y alpha you are FORBIDDEN from adding complexity to perseve prior state of behavour any new version requires all state reset and can and should break all API up until 1.a.b-Mx milestones. 

## PRs And Push

You MUST use the skill gh-actions-poll if it is installed. You are FOBRIDDEN from using a `gh pr view` loop to attempt to poll.  

## Inner loop and observability

- Lint, typecheck, and test locally. CI is the slowest feedback available; it is a gate,
  not a loop. Run `cargo test --features maelstrom` locally before any push: the
  default-feature run never boots the Maelstrom harness, so a push without the
  feature run has not exercised the adapter integration.
- Logging, tracing, and diagnostics are first-class and are added with the code, not
  after it. A refusal that cannot show an operator why is a refusal nobody can act on.
- The pre-push hook runs the verification gate before anything reaches the
  remote: activate once with `git config core.hooksPath .githooks`, and a push
  that has not passed fmt, clippy, both test lanes, the doc tests and the
  record checks aborts. `--no-verify` bypasses it and is a deliberate act.
- The Rust and Cargo arbitrate rule: Rust is not used as a fashion statement; it is
  used so the build system and the compiler are a proof of correctness, exactly as
  Lean 4 and exhaustive property testing and Maelstrom are. LLMs have a nasty habit
  of trying to think like the compiler and working out the consequences of a
  restructure in order to one-shot the full set of edits; that is a fool's errand.
  Humans make the breaking change, run the tooling, and fix in a loop, squeezing
  the toothpaste tube at the end and pushing out bug-free code. As long as code is
  committed or added to the index, a shoot-first-question-later stance is safe:
  cut deep and hard, run the tooling, chase the fixes until it is good, then add
  the working change. Doing otherwise looks catastrophically ignorant of basic
  programming skill.

## Scratch and concurrent work

- `.tmp/` is scratch space. Never stage or commit anything under `.tmp/`. Assume it is deleted frequently. 
- Preserve user and concurrent-agent changes. Do not reset, restore, or overwrite broad paths to remove a narrow change; edit only the proved hunk after the owner is finished.
- You own all the code and all the edits in the PWD do not say "some other agent made those changes" yxxxxx

## Branch discipline

Agents are FORBIDDEN from creating branches or working on any branch other
than the one the User names for the task. By default work happens in the
current checkout and lands on main through a pull request; no work is ever
left stranded on a local branch. A task that appears to require a new branch
stops and asks the User first. A local branch that is not part of the current
release is tagged `archive/<branch>` and then deleted, so no work is ever
abandoned on a branch.

You are FORBIDDEN from making a branch without adding a todo list item
to the end of the todo to check branch ${name} has been merged. This is
not negotiable. It is forbidden to do a branch then a todo, you must
do a todo and then the branch. Laptops crash, plans pivot, and far too
much work has been misplaced to the ire of the user. 

## Worktree discipline

A git worktree is a branch with a checkout attached, so every branch rule
applies to it with double force, the branch rule AND a cleanup rule.

- You are FORBIDDEN from creating a git worktree unless the User explicitly
  asked for one in the current task. "Do it in parallel", "fork an agent", or
  delegation alone is NOT permission to create a worktree: an orchestrator
  delegating scratch work must ask the User first, or work in the existing
  checkout / an explicitly named scratch directory.
- Before creating a worktree you MUST add TWO todo items, in this order: (1)
  check the worktree's branch has been merged, (2) check the worktree has been
  removed (`git worktree remove`). It is forbidden to create the worktree then
  the todos, todos first, worktree second, exactly as with branches.
- A worktree used for read-only work still counts: read-only work does not
  need a worktree at all. Read from the existing checkout, or use `git show
  <ref>:<path>` / `git log`, those cannot lose data, a worktree can.
- By the end of the task, `git worktree list` must contain nothing you
  created. A worktree left behind after its branch is merged is a violation;
  a worktree orphaned by a deleted branch is a violation and a data-loss risk
  the user will hunt you for.
- Read-only subagents (investigation, audit, survey) MUST NOT be given a
  worktree unless the User said so; they read the existing checkout and the
  git history.

## Subagent delegation

- Agents SHOULD delegate major todo items to subagents per the
  opencode-subagent-delegation skill, wherever doing so does not overwrite any
  other instruction in this AGENTS.md or the user's prior statements of
  preference.
- The skill has you make a task doc in a sidecar sqlite3 db that can be appended to as the user steers the work. The tool issues an auto-increment ID the created record must name the branch to create if the not forbidden by branch discipline, and the gh issue number if any. The the todo tool should name the allocated id and any branch and a todo must be added to the end to confirm the branch has not been lost. 
- The completed items MUST bit soft deleted in the sidecar db and you MUST from time to time look for items not soft deleted if there are many add to the todo list at the bottom to check each one in git history and or in code to mark as soft deleted else escallate to User to check if was either lost or deliberately abandoned. 

## Tool inventory and submodule policy

The deliverable crate is sans-I/O with zero runtime dependencies; that
property belongs to the library, not to the proof or build tooling. The Lean
formalization (`formal/uvrr-lean/`) pins Mathlib `v4.33.1` (matching the
toolchain) as build-time proof tooling. Per-file imports stay targeted
(`Mathlib.Tactic.*` modules only, never a full `Mathlib` import); the
elaboration timing measurements behind that ruling are recorded in the
proof auditors' doc comments. The paper's build (tectonic) is unchanged.

### Submodules

- `maelstrom/`, Rust Maelstrom test harness. Retained. Unrelated to the paper. To be run on push as it is with IO and we are SANS-IO so its a feature that does as nemesis test so a search for bugs in both the core lib and its own example host app `maelstrom-lin-kv`
- No `tools/` submodules. The four exploratory tool submodules (tla2tools, veil,
  LeanLTL, lean-auto) were removed after the survey concluded.

## Work item details are developer session-local

Work-item numbers (e.g. `itemNN` or an equivalent) live in the todo any other work list that is not in the code (e.g. "opencode-subagent-delegation" sqlite3 deb) never every put anything that is gh issue, or toto list, or project planning, as a string, in any file, that is committed into git.

## Pride in the work above all else

We are adding to a body of work that is the foundations of computing in 2026. The fact that progress is being made on topics that were new papers when User was in grade school last century is remarkable. Expect must ire and fury from User for not reading the room on safety first, proofs second, clearity and explainablity third, consistency of all seams fourth, zero warnings and less-is-better code fifth, and none of that AI slop em-dash Kruger-Dunning stuff last plush whatever other arbitrary grading the user choses to claim as lore. 

Finding a problem to resolve is a cause for celebration. Calling an Andon is the highest road to collective wisdom. Abiguity is entropy and must be faught. Hold the line. 

## The Eye Of Saurin watches

Lamport's talk post Turing Award has him mention an ambiguity in the 2001 paper that is not in the tlaplus proof. He is talking about private correspondence with REDACTED. REDACTED then disclosed the matter in a public forum to User. Only a pedant would know that. User knows that. Therefore User is a pedant on the topics of strong consistency. Expect user to be the eye of saurin looking at the work. We roll the dough again and again as Lean Development with Lean4 proofs; the system MUST converge towards long term stablity.
