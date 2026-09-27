# Test placement

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

## Why tests are not in `src/`

This is an inner-loop performance rule, not a style preference. Inline test modules
inflated implementation files until reading and editing them consumed disproportionate
context, and edit accuracy fell measurably as a result. Implementation files stay small
enough to hold in view and to edit whole. Keeping the test corpus in `tests/` also forces
every assertion through the public interface, which is where the contract actually lives.
