#!/usr/bin/env bash
# Regenerate REPRODUCE.md, the single executable reproduction document, with Showboat.
# Run from formal/uvrr-lean. Every command is executed for real while the document is
# built; `showboat verify REPRODUCE.md` later reruns all of them and diffs the output.
# Nothing here reads outside the repository except the pinned TLC jar download, which
# lands in the gitignored .tmp/ directory at the repository root.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.elan/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
D=REPRODUCE.md
rm -f "$D"
note() { showboat note "$D"; }          # body on stdin
run()  { showboat exec "$D" bash "$1" >/dev/null; }

showboat init "$D" "Reproducing the uVRR safety-ladder evidence"

note <<'EOF'
This is an executable laboratory document built with Simon Willison's Showboat. Every
fenced `bash` block below was run, and the block after it is the output that was captured
at that moment. Anyone with the same tools can rerun the whole chain and diff every output:

```sh
cd formal/uvrr-lean
showboat verify REPRODUCE.md
```

`showboat extract REPRODUCE.md` prints the shell commands that recreate this file. The
generator that produced it is `make-reproduce.sh` in the same directory. All paths are
relative to `formal/uvrr-lean` in the repository; nothing here depends on a home directory,
a credential, or a model service.

**What this document does and does not establish.** It shows that the Lean library
compiles under the pinned toolchain, that every named declaration depends only on the
three standard Lean axioms, that all 21 rung transcripts replay, that the fault-injection
mutations are rejected by the compiler, that the three TLC model checks reproduce, that
the Rust implementation passes its gates, that the recorded recovery counterexample still
reproduces as an expected Red result, and that the paper builds to a byte-identical PDF.
It does **not** establish an end-to-end uVRR or VRR-2012 safety proof: the ladder proves
exact component theorems, the recovery counterexample remains unresolved, and whole-log
composition across reconfigurations, client-visible linearizability and progress are
open, as the paper and `LAB-BOOK.md` state.
EOF

note <<'EOF'
## 1. Exact software versions

The transcript was recorded on macOS 26.6.2 on Apple Silicon (`arm64`). The tools and the
versions that produced the outputs in this file are:

| Tool | Version | How it was installed |
|---|---|---|
| Lean | 4.33.1 (commit `819816b2e0a3bf405af45ae5c7af2491d8f5bee6`) | `elan` reads `lean-toolchain` and installs `leanprover/lean4:v4.33.1` |
| Lake | 5.0.0-src+819816b | bundled with the Lean toolchain |
| elan | 4.2.4 | `curl https://elan.lean-lang.org/elan-init.sh -sSf \| sh` |
| Python | 3.14.6 | Homebrew; the check scripts use only the standard library |
| Showboat | 0.6.1 | `uv tool install showboat` (or `pip install showboat`) |
| Tectonic | 0.17.0 | `brew install tectonic` |
| OpenJDK | 26.0.2 | `brew install openjdk` |
| TLC | 2.19 (rev 5a47802), `tla2tools.jar` from the `v1.7.4` GitHub release | downloaded and hash-checked in section 4 |
| Rust | cargo 1.96.0, rustc 1.96.0 | `rustup` |
| ripgrep | 15.2.0 | `brew install ripgrep` |
| poppler `pdfinfo` | 26.07.0 | `brew install poppler` |

The Lean formalization has **zero** package dependencies: `lake-manifest.json` lists no
packages, there is no Mathlib, and every import is internal to `UVRR`. The next block prints
the versions so that a verifier on a different machine sees exactly which line differs.
EOF
run 'sw_vers -productVersion; uname -m
lean --version
lake --version
elan --version
python3 --version
echo "showboat $(showboat --version)"
tectonic --version
java -version 2>&1 | head -1
cargo --version
rg --version | head -1
pdfinfo -v 2>&1 | head -1'

note <<'EOF'
## 2. The Lean library

`lake build` compiles all 19 modules of the `UVRR` library under the toolchain pinned in
`lean-toolchain`. On a fresh clone the compiler prints one line per module; only the final
summary line is compared here. The digests that follow tie this transcript to the exact
sources that were checked.
EOF
run 'cat lean-toolchain; lake build 2>&1 | tail -1'
run 'shasum -a 256 lakefile.toml lake-manifest.json lean-toolchain UVRR.lean UVRR/*.lean'

note <<'EOF'
### 2.1 Axiom audit

`check_axioms.py` queries `#print axioms` for every named theorem, definition and
abbreviation in the compiled sources and fails closed if any declaration cannot be
queried. Only `propext`, `Quot.sound` and `Classical.choice` are allowed; `sorryAx` or
any custom axiom fails the audit. This checks what each proof depends on, not whether a
theorem statement is the intended protocol specification. The second block confirms that
no `sorry` and no `native_decide` occur in the sources (the single match is a comment
saying so).
EOF
run 'python3 check_axioms.py'
run 'grep -rnw sorry UVRR UVRR.lean || echo "no sorry in sources"
grep -rn native_decide UVRR UVRR.lean || echo "no native_decide in sources"'

note <<'EOF'
## 3. The rung ladder and its fault controls

Each rung is its own Showboat transcript in `ladder/`. A positive rung prints the entire
module, builds it and prints the axiom report of its stated theorems; rung 8 records a
historical Leanstral draft that fails to compile, and rung 18 records the expected-Red
recovery counterexample. Replaying every rung below reruns those builds and diffs every
captured output.

| Rung | Module | Verified result |
|---|---|---|
| 1 | `Structure.lean` | Finite quorum checker equivalence and a hot-swap schedule |
| 2 | `LexBallot.lean` | Well-founded lexicographic era/round order |
| 3 | `Synod.lean` | Agreement conditional on S1–S6, order and nonempty decision quorums (axiom-free) |
| 4 | `Eras.lean` | Era-indexed reduction to the conditional agreement theorem (Theorem 10) |
| 5 | `Counterexample.lean` | Removing cross-era overlap admits disagreement under the other encoded conditions |
| 6 | `CastingVote.lean` | Conditional guard noninterference and phase-I quorum completion |
| 7 | `Weights.lean` | Concrete weighted schedule and a failing weight-change example |
| 8 | historical Leanstral draft | Reproducible compiler failure; no proved target theorem |
| 9 | `WeightedGeneral.lean` | General finite-support scaled majority intersection; distance-two sharpness witness |
| 10 | `Acceptor.lean` | Reachable single-era promise/acceptance invariants and a guard-bypass counterexample |
| 11 | `MultiPromise.lean` | Derives the proposal-era bound from suffix/point promise bounds |
| 12 | `NegativeControls.lean` | Independence witnesses and the temporary source-mutation harness |
| 13 | `ViewSelection.lean` | Executable latest-normal-view/length selection and prefix preservation |
| 14 | `NormalLog.lean` | Fixed-view message induction: report comparability and replica prefix retention |
| 15 | `ViewFence.lean` | Voter-report bounds; committed prefixes preserved in later activated views under explicit provenance |
| 16 | `LogProvenance.lean` | Shared multi-view transition induction: committed-log compatibility, fixed configuration, no crashes |
| 17 | `RecoveryFence.lean` | A supporting quorum retains a known fence through arbitrary crash/recovery sequences |
| 18 | Rust path and directed TLC trace | Reproduced committed divergence after serial recoveries: an expected Red, not a safety theorem |
| 19 | `CrashVector.lean` | Published crash-vector collector: reachable reply sets are incarnation-consistent |
| 20 | `AcquisitionOrder.lean` | Temporal acquisition induction under explicit recovery provenance; retention premise open |
| 21 | `RecoveryAcquire.lean` | Operational acquisition transitions; finished certificates are crash-consistent quorums for exactly their request |
EOF
run 'for f in ladder/[0-9][0-9]-*.md; do
  if showboat verify "$f" >/dev/null 2>&1; then echo "ok   $f"; else echo "FAIL $f"; exit 1; fi
done'

note <<'EOF'
### 3.1 Compiler-rejected mutations

`check_mutations.py` copies a module into a temporary file, applies one named edit, and
requires Lean to reject the result after the unchanged copy compiles. The edits remove a
guard or premise that the proof relies on: an existential instead of universal quorum
check, an unsafe four-node family, an omitted crash-vector filter, an omitted response
ordering, a recovering sender answering, a stale acquisition request, and a `sorry` that
must be exposed by the axiom audit despite a zero compiler exit. Rejection measures proof
sensitivity; it is not a claim that every conceivable algorithm needs the guard.
EOF
run 'python3 check_mutations.py'

note <<'EOF'
## 4. TLC model checks

The TLA+ models live in `formal/`; the delayed-fence research copy lives beside its
evidence. TLC 2.19 is obtained from the `v1.7.4` release of `tla2tools.jar` on GitHub and
checked against the SHA-256 recorded when the original runs were made; the download stops
the document if the digest differs. The jar is stored under the gitignored `.tmp/` at the
repository root, and TLC's scratch state goes there too, so nothing is written into the
tracked tree.
EOF
run 'mkdir -p ../../.tmp
curl -sSL -o ../../.tmp/tla2tools.jar https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
echo "936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88  ../../.tmp/tla2tools.jar" | shasum -a 256 -c'

note <<'EOF'
### 4.1 The delayed-fence counterexample (expected violation)

`DelayedFence.tla` extends `VrrCoreEras.tla` to allow repeated serial crashes with the
incremented recovery epoch as nonce. TLC must find that `CommittedLogsAgree` is violated:
this is the finite model-checked twin of the Rust counterexample replayed in section 5.
A violation here is the expected result; it is evidence of an unresolved problem, not of
safety. The run uses one worker and a 512 MiB heap.
EOF
run 'cd evidence/delayed-fence/tla && java -Xmx512m -jar ../../../../../.tmp/tla2tools.jar -workers 1 -metadir ../../../../../.tmp/tlc-delayed-fence -config DelayedFence.cfg DelayedFence.tla 2>&1 | grep -E "^TLC2 Version|Error: Invariant|^[0-9]+ states generated|Model checking completed"'

note <<'EOF'
### 4.2 The transfer mutation (expected violation)

`VrrCoreErasM1.cfg` is a mutated configuration of the era model that must violate
`FrontiersOrdered`. It is run with one worker because a multi-worker search stops at a
nondeterministic point after the first violation, which makes the state counts vary; only
the violated invariant is compared.
EOF
run 'cd .. && java -Xmx2g -jar ../.tmp/tla2tools.jar -workers 1 -metadir ../.tmp/tlc-m1 -config VrrCoreErasM1.cfg VrrCoreEras.tla 2>&1 | grep -E "Error: Invariant|Model checking completed"'

note <<'EOF'
### 4.3 The era model (expected pass)

`VrrCoreEras.cfg` is the three-node `inc3` scenario with one command value, maximum log
length three, two eras and view index zero; `MaxEpoch=0` disables crashes, so this run
gives no crash-recovery coverage. Breadth-first search with two workers and a 2 GiB heap
takes about two and a half minutes on the recording machine and must complete with no
error and an empty queue. The generated and distinct state totals are deterministic for a
completed search; the progress lines with timings are not compared.
EOF
run 'cd .. && java -Xmx2g -XX:+UseParallelGC -jar ../.tmp/tla2tools.jar -workers 2 -metadir ../.tmp/tlc-eras -config VrrCoreEras.cfg VrrCoreEras.tla 2>&1 | grep -E "Error: Invariant|^[0-9]+ states generated|Model checking completed"'
run 'cd .. && shasum -a 256 VrrCoreEras.tla VrrCoreEras.cfg VrrCoreErasM1.cfg uvrr-lean/evidence/delayed-fence/tla/DelayedFence.tla uvrr-lean/evidence/delayed-fence/tla/DelayedFence.cfg uvrr-lean/evidence/delayed-fence/tla/VrrCoreEras.tla'

note <<'EOF'
## 5. The Rust implementation gates and the recovery counterexample

The crate at the repository root is the implementation the models describe. Its gates
are format, Clippy on all targets and features with warnings denied, the all-feature test
suite, the doctests, and two source searches: no inline test modules under `src/`, and no
Paxos terminology under `src/`. Passing them does not establish a formal refinement to the
Lean models. Test timings are stripped from the output; the per-binary result lines are
summarised as a sorted count so that binary ordering cannot cause a spurious diff.
EOF
run 'cd ../.. && cargo fmt --check && echo "fmt clean"
cargo clippy --all-targets --all-features -- -D warnings >/dev/null 2>&1 && echo "clippy clean with -D warnings"
cargo test --all-features --all-targets 2>&1 | grep -E "^test result" | sed "s/; finished in.*//" | sort | uniq -c
echo "doctests:"; cargo test --doc 2>&1 | grep -E "^test result" | sed "s/; finished in.*//"
if rg -q "#\[cfg\(test\)\]|mod tests" src; then echo "FAIL inline tests under src/"; exit 1; else echo "no inline tests under src/"; fi
if rg -qi paxos src; then echo "FAIL Paxos reference under src/"; exit 1; else echo "no Paxos references under src/"; fi'

note <<'EOF'
### 5.1 The expected-Red recovery replay

`check_recovery_counterexample.py` copies `evidence/delayed-fence/reproduction.rs` into
the crate's `tests/` directory under a temporary name, runs it, and requires the exact
committed-divergence diagnostic. Its zero exit means the counterexample **reproduced**:
serial amnesiac recoveries with authentic delayed messages and fresh recovery ticks make
the unchanged core commit different operations at slot 3. This is the open problem the
paper reports; a future repair must change this witness deliberately.
EOF
run 'python3 check_recovery_counterexample.py 2>&1 | tail -3'

note <<'EOF'
## 6. The paper

`paper/build.sh` runs Tectonic on `paper/paper.tex` with a fixed `SOURCE_DATE_EPOCH`, so
the PDF is byte-identical across builds with the same Tectonic release and the same
package bundle. Tectonic downloads and caches the standard packages on its first run; a
verifier without that cache needs network access once. The digest of the PDF, its page
count and its metadata are recorded, followed by the digest of the source.
EOF
run 'sh paper/build.sh >/dev/null 2>&1 && shasum -a 256 paper/paper.pdf
pdfinfo paper/paper.pdf | grep -E "^(Title|Author|Pages)"
printf "overfull boxes: %s\n" "$(grep -c Overfull paper/paper.log || true)"'
run 'shasum -a 256 paper/paper.tex paper/build.sh'

note <<'EOF'
## 7. Digests of the transcripts and check scripts

These digests bind this document to the exact rung transcripts, check scripts and Rust
witness that were replayed above.
EOF
run 'shasum -a 256 ladder/*.md check_axioms.py check_mutations.py check_recovery_counterexample.py evidence/delayed-fence/reproduction.rs'

note <<'EOF'
## 8. What remains open

Everything above is either a kernel-checked component theorem, a finite model check, or an
implementation test. The end-to-end uVRR claim is not yet proved. The obligations recorded
in `LAB-BOOK.md` and in the paper's closing section are: repairing the diskless
repeated-recovery counterexample and proving the repair, quorum-knowledge persistence and
value reconstruction for the operational acquisition of rung 21, composing recovery with
committed-log safety, whole-log preservation across an arbitrary sequence of
reconfigurations, client-visible linearizability, conditional progress, and an explicit
refinement between the Rust implementation and the checked models.
EOF
echo "generated $D"
