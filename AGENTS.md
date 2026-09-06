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

## Scratch and concurrent work

- `.tmp/` is scratch space. Never stage or commit anything under `.tmp/`.
- Preserve user and concurrent-agent changes. Do not reset, restore, or overwrite broad
  paths to remove a narrow change; edit only the proved hunk after the owner is finished.

## Subagent delegation

- Agents SHOULD delegate major todo items to subagents per the
  opencode-subagent-delegation skill, wherever doing so does not overwrite any
  other instruction in this AGENTS.md or the user's prior statements of
  preference.
