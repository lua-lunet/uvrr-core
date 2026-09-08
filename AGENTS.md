# Repository agent rules

## Stance

- Formalism first. The lingua franca is mathematics and computer science, not convention,
  analogy, or taste. A design is stated as invariants and a transition function, and the
  code is judged against that statement.
- Prefer functions and components that compose. Composition is the unit of reuse; a
  hierarchy is not.
- Prefer compile-time certainty over runtime discovery. Where a property can be made
  unrepresentable, make it unrepresentable rather than validating it later.
- No frameworks. Write the low-level code this crate needs, or use the standard library.
- Dependencies are a lifelong support tax and a liability, never an asset. The
  non-optional dependency set stays empty and is gated by a test.
- YAGNI is the removal of future bloat and future bugs. A type, a knob, or an abstraction
  kept alive against a hypothetical consumer is debt that is already accruing.
- Code that exposes no useful service has no value. Code that can only be tested in
  production is legacy at the moment it is written.

## Documentation

- Documentation must be the timeless target end state. We practice
  markdown-driven development (MDD): the markdown states what the system IS.
- No project plans, task identifiers, orchestration chatter, or historic
  narrative in documentation or code. Such material is written only when
  explicitly requested by the User or added manually by the User. Dated
  evidence artifacts (lab book, audit logs, rung transcripts) are the
  established exceptions; do not add new narrative classes to documentation.

## Perimeters

- Enforce shapes at module perimeters: IO, network, storage, and boundaries between logic
  layers. Data crossing a perimeter is validated there, once.
- Do not mix maturities in one unit of work. Scaffolding for a spike and the core of a
  platform have different obligations and do not belong in the same change.
- The dependency graph is acyclic by ruling. A proposed edge that would close a cycle is a
  signal that a responsibility is in the wrong module.

## Test placement

- Do not put `#[cfg(test)]` modules, `mod tests`, or test functions in Rust
  implementation files under `src/`.
- Put unit, contract, and regression tests in `tests/`. Put shared test support in
  `tests/harness/` or `tests/support/`.
- Exercise behavior through the public interface. Do not add a public or test-only
  production API merely to expose private state to a test.
- If an old private-state test describes an unreachable condition and duplicates an
  invariant already owned by a public type, delete the unreachable test and keep the
  public invariant coverage.
- Before completing work, this search must find no inline test modules under `src/`:
  `rg -n '#\[cfg\(test\)\]|mod tests' src`.

### Why tests are not in `src/`

This is an inner-loop performance rule, not a style preference. Inline test modules
inflated implementation files until reading and editing them consumed disproportionate
context, and edit accuracy fell measurably as a result. Implementation files stay small
enough to hold in view and to edit whole. Keeping the test corpus in `tests/` also forces
every assertion through the public interface, which is where the contract actually lives.

## Test shape

- A narrow pyramid for a strongly typed language: just-enough-test. Too many tests is
  overfitting, and an overfitted suite obstructs the refactoring it was meant to protect.
- Test whole subsystems as black boxes through their public interface.
- Prefer exhaustive or property-based coverage where the domain is small and closed, over
  a list of hand-picked examples that happens to pass.
- A test that cannot fail for a stated reason is not a test.

## Claimed bugs and review findings

- A code-review observation is a suspected, unconfirmed behavior until a reproduction
  test fails on the unchanged code.
- A plan, ticket, or read-only review may describe the observed path and specify a
  reproduction test. It must not prescribe, predict, or name a fix or root cause.
- Create and run the smallest public-path reproduction before changing production code.
- If the reproduction passes without a production-code change, record the claim as
  `resolved unreproducible`, make no fix, and remove any speculative implementation.
- If the reproduction fails, preserve the Red result. Only then diagnose and make the
  minimum production change needed for that test.
- A model may call something the cause or the fix only after it has run both the failing
  test and the code change that makes that same test pass.
- Re-run the directly affected suite and the repository verification gates after Green.
  Never weaken an invariant merely to satisfy an unconfirmed claim.

## Inner loop and observability

- Lint, typecheck, and test locally. CI is the slowest feedback available; it is a gate,
  not a loop.
- Logging, tracing, and diagnostics are first-class and are added with the code, not
  after it. A refusal that cannot show an operator why is a refusal nobody can act on.

## Commit discipline

- Commit when the full suite is green. Do not accumulate a large uncommitted tree: a
  long-lived staged diff is unreviewable and destroys the bisect point that made it safe.
- A commit message describes the change as delivered. It does not enumerate pending
  chores, releases, or review steps, and it carries no internal tracking identifiers.

## Reproduction

- `formal/uvrr-lean/make-reproduce.sh --full` regenerates and replays the entire
  evidence chain (Lean build, axiom audit, rung transcripts, mutation controls, TLC,
  Rust gates, published-paper verification). It is run independently before a major
  release. It is never run for minor edits such as paper rewordings; without `--full`
  the script is a no-op that prints how to run it.

## Scratch and concurrent work

- `.tmp/` is scratch space. Never stage or commit anything under `.tmp/`.
- Preserve user and concurrent-agent changes. Do not reset, restore, or overwrite broad
  paths to remove a narrow change; edit only the proved hunk after the owner is finished.

## Subagent delegation

- Agents SHOULD delegate major todo items to subagents per the
  opencode-subagent-delegation skill, wherever doing so does not overwrite any
  other instruction in this AGENTS.md or the user's prior statements of
  preference.

## Tool inventory and submodule policy

The paper (`formal/uvrr-lean/paper/paper.tex`) and the Lean formalization
(`formal/uvrr-lean/`) use zero external Lean dependencies. The `lake-manifest.json`
has `"packages": []`. All Lean imports are internal (`UVRR.*` only). This is by
design, not accident.

### Tools that contributed to the paper

- **Lean 4.33.1** — kernel-checked proofs. The formalization's sole verification
  tool. No Mathlib, no external tactics.
- **TLC 2.19** — model-checked the era model and its mutations. Ran from a
  standalone jar, not a submodule. Evidence in `formal/uvrr-lean/evidence/tlc/`;
  the delayed-fence counterexample evidence was removed with the classic
  crash-recover attempt it witnessed (historical copies remain under
  `research/uvrr-audit/`).
- **Leanstral** (Mistral) — LLM Lean proof generator. Reported in the paper as a
  failed-attempt tool. No accepted Leanstral-generated proof is attributed.
- **Showboat** — executable evidence packaging. Cited in the paper bibliography.
- **Tectonic** — LaTeX build tool for `paper.tex` (via `paper/build.sh`).

### Tools surveyed and abandoned (not submodules)

The following were cloned during the research tool survey (items 12, 14a, 14b)
but did not contribute to the paper or formalization. Their experiment records
are committed under `research/`. Do not re-add them as submodules.

- **Veil** — never tire-kicked. Pins v4.32.0 + pulls Mathlib. The formalization
  uses plain Lean by design. Record: `research/tool-kick-tires.md`,
  `research/outcomes-paper.md`.
- **LeanLTL** — never exercised beyond checkout. Past-time operators remain
  future work. Record: `research/outcomes-paper.md`.
- **lean-auto / Duper** — exercised (2/3 theorems solved, T2 commutativity
  timed out at 500s). Not a dependency of the formalization. Record:
  `research/lean-solvers-cli-lean-auto.md`.
- **Aesop** — exercised (4/4 propositional theorems solved). Not a dependency.
  Record: `research/lean-solvers-cli-omega-aesop.md`.
- **omega** — built into Lean, zero install. Not a dependency. Record: same as
  Aesop.

### Submodules

- `maelstrom/` — Rust Maelstrom test harness. Retained. Unrelated to the paper.
- No `tools/` submodules. The four exploratory tool submodules (tla2tools, veil,
  LeanLTL, lean-auto) were removed after the survey concluded.
