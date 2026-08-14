# Repository agent rules

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

## Scratch and concurrent work

- `.tmp/` is scratch space. Never stage or commit anything under `.tmp/`.
- Preserve user and concurrent-agent changes. Do not reset, restore, or overwrite broad
  paths to remove a narrow change; edit only the proved hunk after the owner is finished.
