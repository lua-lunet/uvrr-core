## TLA+

- Submodule: added `tools/tla2tools`, staged (not committed). URL substitution:
  `https://github.com/tlaplus/tla2tools` is dead ("Repository not found" — repo was
  folded into the `tlaplus/tlaplus` monorepo). Added `https://github.com/tlaplus/tlaplus`
  (shallow, --depth 1) at the prescribed path instead.
- Java: already present (Homebrew OpenJDK 26.0.2) — no install needed.
- Jar: `https://github.com/tlaplus/tla2tools/releases/latest/download/tla2tools.jar`
  returns 9-byte "Not Found" (dead with the repo). Working source:
  `https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar`
  (2.27 MB, valid JAR, TLC2 Version 2.19 of 08 August 2024, rev 5a47802).
- Hello world: `.tmp/tla-hello/Hello.tla` (one-bit register toggling via `bit' = 1 - bit`,
  EXTENDS Naturals — note: `EXTENDS TLC` alone does not define `-`) + `Hello.cfg`
  (INIT/NEXT/INVARIANT).
- Result: `java -cp .tmp/tla-hello/tla2tools.jar tlc2.TLC .tmp/tla-hello/Hello.tla`
  — Model checking completed, no errors; 3 states generated, 2 distinct, depth 2,
  invariant Inv holds. Finished in <1s on aarch64.
- Context: repo has real specs under `formal/` (VrrCore.tla, VrrCoreEras.tla); not run
  per spec.

## Lean 4 tools

- Toolchain: was absent; installed via `elan-init.sh -y` (one attempt, ~2 min). Lean 4.33.1, Lake 5.0.0, arm64 darwin. PATH: `~/.elan/bin`.
- Submodules added (shallow, `git add` only, NOT committed): `tools/veil` (verse-lab/veil), `tools/LeanLTL` (UCSCFormalMethods/LeanLTL), `tools/lean-auto` (leanprover-community/lean-auto).
- Hello world: `.tmp/lean-hello/` (lakefile.toml + lean-toolchain v4.33.1 + Hello.lean). `theorem hello : 1 + 1 = 2 := by rfl` compiles clean via `lake env lean Hello.lean` (PROOF_OK). `lake build` alone warns "no targets specified" (no default target in bare toml lakefile); the `lake env lean` path in the spec suffices.
- Skipped: building a Veil example. Veil pins `leanprover/lean4:v4.32.0` (differs from installed 4.33.1; elan would fetch a second toolchain) and its lakefile pulls Mathlib — a multi-GB, many-minute resolve. Out of timebox.
- Untouched: `.gitignore`, `Cargo.toml`, `tools/tla2tools` — pre-existing changes from the concurrent TLA+ agent.
