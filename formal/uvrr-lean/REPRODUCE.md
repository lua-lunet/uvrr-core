# Reproducing the uVRR safety-ladder evidence

*2026-09-07T00:08:40Z by Showboat 0.6.1*
<!-- showboat-id: 2275b99b-64db-405b-8460-fcc5de127870 -->

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
three standard Lean axioms, that all 20 rung transcripts replay, that the fault-injection
mutations are rejected by the compiler, that the two TLC model checks reproduce, that
the Rust implementation passes its gates, and that the paper builds to a byte-identical
PDF. It does **not** establish an end-to-end uVRR or VRR-2012 safety proof: the ladder
proves exact component theorems, and whole-log composition across reconfigurations,
client-visible linearizability and progress are open, as the paper and `LAB-BOOK.md`
state.

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

```bash
sw_vers -productVersion; uname -m
lean --version
lake --version
elan --version
python3 --version
echo "showboat $(showboat --version)"
tectonic --version
java -version 2>&1 | head -1
cargo --version
rg --version | head -1
pdfinfo -v 2>&1 | head -1
```

```output
26.6.2
arm64
Lean (version 4.33.1, arm64-apple-darwin24.6.0, commit 819816b2e0a3bf405af45ae5c7af2491d8f5bee6, Release)
Lake version 5.0.0-src+819816b (Lean version 4.33.1)
elan 4.2.4 (227caca13 2026-08-25)
Python 3.14.6
showboat 0.6.1
Tectonic 0.17.0
openjdk version "26.0.2" 2026-07-21
cargo 1.96.0 (30a34c682 2026-05-25)
ripgrep 15.2.0
pdfinfo version 26.07.0
```

## 2. The Lean library

`lake build` compiles all 19 modules of the `UVRR` library under the toolchain pinned in
`lean-toolchain`. On a fresh clone the compiler prints one line per module; only the final
summary line is compared here. The digests that follow tie this transcript to the exact
sources that were checked.

```bash
cat lean-toolchain; lake build 2>&1 | tail -1
```

```output
leanprover/lean4:v4.33.1
Build completed successfully (22 jobs).
```

```bash
shasum -a 256 lakefile.toml lake-manifest.json lean-toolchain UVRR.lean UVRR/*.lean
```

```output
3278e7feb5801b189f9bf9861fcf942a0f6741a40c5c19d6ea6501e257fce718  lakefile.toml
6e47b49409f143d341c4c6e804639227c01096f1628f765a9e75c9cc8b91e767  lake-manifest.json
3aac669c7a910ec2389f4e4f921b605adf6ebf2d1e0c9b9cd0be4d33f3f5db71  lean-toolchain
ae004fa4171ea36a02acabf8dbfe84be3bfb4b6afdfa77dc55faff3ad599862e  UVRR.lean
447b859643b2b93cfba5504912cb3a6eea63b3b5d618aa3daeb1c8f9f50997e0  UVRR/Acceptor.lean
4c57bf2b9ac24872ebfa4d748c2640f6ead82f1901ffb3eda46b1b1dc45e75c6  UVRR/AcquisitionOrder.lean
eb7499f88feea53a0a972634acf5c8d30aaae3f6a75c3f6cde7cd13596ccbc9d  UVRR/CastingVote.lean
36126e1a2160a491d124f40d1f91872cae43c9af888f7545a70dc898ae198d53  UVRR/Counterexample.lean
da655d993fccb7074d35e78f53df04d6dbaceaae5d3357f4169718f985c1f44b  UVRR/CrashVector.lean
93ba001b17ad908d03de8319cb0868a863573e680107c3099e28bc6c259a471d  UVRR/Eras.lean
6aeceecc95874a5560abd291d1235c12fb0f2fee58467f790e3650eb40b3a48a  UVRR/LexBallot.lean
07d99a25dc6c1a2f775a4673d996dafc34e982870c11ac3a34312301fed221d3  UVRR/LogProvenance.lean
51b2987e04a4ebed3baa3fdf5c28619cf6eac03e45a779fff0a8af22afccf6ae  UVRR/MultiPromise.lean
88f777a188475d320a8b58f841d7d6401d3ebf74fc0f81d6a52beacedb4f630d  UVRR/NegativeControls.lean
3e7340de62ef42558df1e9a687876b1f85d99189f6c977e605f0c9293963a394  UVRR/NormalLog.lean
8fd7ba97afdacdccca71fc30875fd72da385fea6670e2dd779ab3cf9b1a5e323  UVRR/RecoveryAcquire.lean
3c6d33205b535dd20427ec65b166694c6527001aaa96648dba59fd25887dc5fe  UVRR/RecoveryFence.lean
931d0ba52acbe700050a2444e1cab4f0f530c7c2517edfe962c57c694be33375  UVRR/Structure.lean
d7c8b1934cde6236b24d8bf64f00f7611d171a194e2486287433cc57391003b9  UVRR/Synod.lean
0bc8565547700d036d19909bd9a18aecd2bd7f8a785002bfe5c3a5f1a4e941d9  UVRR/ViewFence.lean
4495ef46dbcf7ee427ff71efe57607d4b62c465a5cf5e415aa6a256f58b716f3  UVRR/ViewSelection.lean
0a242d8197f592b752757d188ddffd4c874d1a82ce93f4f0506931d1ba5608b4  UVRR/WeightedGeneral.lean
2585d9dc65fd74918ba58b633724f09c5195beb19f3b526a2d02a944a4be6cb5  UVRR/Weights.lean
```

### 2.1 Axiom audit

`check_axioms.py` queries `#print axioms` for every named theorem, definition and
abbreviation in the compiled sources and fails closed if any declaration cannot be
queried. Only `propext`, `Quot.sound` and `Classical.choice` are allowed; `sorryAx` or
any custom axiom fails the audit. This checks what each proof depends on, not whether a
theorem statement is the intended protocol specification. The second block confirms that
no `sorry` and no `native_decide` occur in the sources (the single match is a comment
saying so).

```bash
python3 check_axioms.py
```

```output
PASS 345 declarations: only standard Lean axioms
```

```bash
grep -rnw sorry UVRR UVRR.lean || echo "no sorry in sources"
grep -rn native_decide UVRR UVRR.lean || echo "no native_decide in sources"
```

```output
no sorry in sources
UVRR/Structure.lean:8:  no `native_decide`.
```

## 3. The rung ladder and its fault controls

Each rung is its own Showboat transcript in `ladder/`. A positive rung prints the entire
module, builds it and prints the axiom report of its stated theorems; rung 8 records a
historical Leanstral draft that fails to compile. Replaying every rung below reruns those
builds and diffs every captured output.

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
| 18 | `CrashVector.lean` | Published crash-vector collector: reachable reply sets are incarnation-consistent |
| 19 | `AcquisitionOrder.lean` | Temporal acquisition induction under explicit recovery provenance; retention discharged by the durable superblock identity |
| 20 | `RecoveryAcquire.lean` | Operational acquisition transitions; finished certificates are crash-consistent quorums for exactly their request |

```bash
for f in ladder/[0-9][0-9]-*.md; do
  if showboat verify "$f" >/dev/null 2>&1; then echo "ok   $f"; else echo "FAIL $f"; exit 1; fi
done
```

```output
ok   ladder/01-structure.md
ok   ladder/02-lex-ballots.md
ok   ladder/03-synod-agreement.md
ok   ladder/04-eras-theorem10.md
ok   ladder/05-counterexample.md
ok   ladder/06-casting-vote.md
ok   ladder/07-weights.md
ok   ladder/08-leanstral-lemma2.md
ok   ladder/09-weighted-general.md
ok   ladder/10-acceptor-transitions.md
ok   ladder/11-multi-promises.md
ok   ladder/12-negative-controls.md
ok   ladder/13-view-selection.md
ok   ladder/14-normal-log.md
ok   ladder/15-view-fence.md
ok   ladder/16-log-provenance.md
ok   ladder/17-recovery-fence.md
ok   ladder/18-crash-vector.md
ok   ladder/19-acquisition-order.md
ok   ladder/20-recovery-acquisition.md
```

### 3.1 Compiler-rejected mutations

`check_mutations.py` copies a module into a temporary file, applies one named edit, and
requires Lean to reject the result after the unchanged copy compiles. The edits remove a
guard or premise that the proof relies on: an existential instead of universal quorum
check, an unsafe four-node family, an omitted crash-vector filter, an omitted response
ordering, a stale acquisition request, and a `sorry` that
must be exposed by the axiom audit despite a zero compiler exit. Rejection measures proof
sensitivity; it is not a claim that every conceivable algorithm needs the guard.

```bash
python3 check_mutations.py
```

```output
PASS unchanged control compiles
PASS Lean rejects existential quorum checker
PASS Lean rejects unsafe four-node decision family
PASS Lean rejects omitted next-slot guard
PASS Lean rejects append after view-change fence
PASS Lean rejects cross-view prepare receipt
PASS Lean rejects stale recovery episode evidence
PASS Lean rejects omitted crash-vector filtering
PASS Lean rejects omitted acquisition response ordering
PASS Lean rejects recovering sender response
PASS Lean rejects stale acquisition request
PASS axiom audit detects sorryAx despite compiler exit 0
```

## 4. TLC model checks

The TLA+ models live in `formal/`. TLC 2.19 is obtained from the `v1.7.4` release of `tla2tools.jar` on GitHub and
checked against the SHA-256 recorded when the original runs were made; the download stops
the document if the digest differs. The jar is stored under the gitignored `.tmp/` at the
repository root, and TLC's scratch state goes there too, so nothing is written into the
tracked tree.

```bash
mkdir -p ../../.tmp
curl -sSL -o ../../.tmp/tla2tools.jar https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
echo "936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88  ../../.tmp/tla2tools.jar" | shasum -a 256 -c
```

```output
../../.tmp/tla2tools.jar: OK
```

### 4.1 The transfer mutation (expected violation)

`VrrCoreErasM1.cfg` is a mutated configuration of the era model that must violate
`FrontiersOrdered`. It is run with one worker because a multi-worker search stops at a
nondeterministic point after the first violation, which makes the state counts vary; only
the violated invariant is compared.

```bash
cd .. && java -Xmx2g -jar ../.tmp/tla2tools.jar -workers 1 -metadir ../.tmp/tlc-m1 -config VrrCoreErasM1.cfg VrrCoreEras.tla 2>&1 | grep -E "Error: Invariant|Model checking completed"
```

```output
Error: Invariant FrontiersOrdered is violated.
```

### 4.2 The era model (expected pass)

`VrrCoreEras.cfg` is the three-node `inc3` scenario with one command value, maximum log
length three, two eras and view index zero; `MaxEpoch=0` disables crashes, so this run
gives no crash-recovery coverage. Breadth-first search with two workers and a 2 GiB heap
takes about two and a half minutes on the recording machine and must complete with no
error and an empty queue. The generated and distinct state totals are deterministic for a
completed search; the progress lines with timings are not compared.

```bash
cd .. && java -Xmx2g -XX:+UseParallelGC -jar ../.tmp/tla2tools.jar -workers 2 -metadir ../.tmp/tlc-eras -config VrrCoreEras.cfg VrrCoreEras.tla 2>&1 | grep -E "Error: Invariant|^[0-9]+ states generated|Model checking completed"
```

```output
Model checking completed. No error has been found.
2743933 states generated, 837204 distinct states found, 0 states left on queue.
```

```bash
cd .. && shasum -a 256 VrrCoreEras.tla VrrCoreEras.cfg VrrCoreErasM1.cfg
```

```output
2d51449d707f104a506971a8e0a915b69255e1f99b6bbc5a6594384d49311ef2  VrrCoreEras.tla
1f91a858af7cb8b0972535eb674665cde2637bd6c1824d4740b3770883d6fc3d  VrrCoreEras.cfg
d4fca78fafbaadfb7f77db760028410b68ab0128392b700e836f13c8ef3fcf6c  VrrCoreErasM1.cfg
```

## 5. The Rust implementation gates

The crate at the repository root is the implementation the models describe. Its gates
are format, Clippy on all targets and features with warnings denied, the all-feature test
suite, the doctests, and two source searches: no inline test modules under `src/`, and no
Paxos terminology under `src/`. Passing them does not establish a formal refinement to the
Lean models. Test timings are stripped from the output; the per-binary result lines are
summarised as a sorted count so that binary ordering cannot cause a spurious diff.

```bash
cd ../.. && cargo fmt --check && echo "fmt clean"
cargo clippy --all-targets --all-features -- -D warnings >/dev/null 2>&1 && echo "clippy clean with -D warnings"
cargo test --all-features --all-targets 2>&1 | grep -E "^test result" | sed "s/; finished in.*//" | sort | uniq -c
echo "doctests:"; cargo test --doc 2>&1 | grep -E "^test result" | sed "s/; finished in.*//"
if rg -q "#\[cfg\(test\)\]|mod tests" src; then echo "FAIL inline tests under src/"; exit 1; else echo "no inline tests under src/"; fi
if rg -qi paxos src; then echo "FAIL Paxos reference under src/"; exit 1; else echo "no Paxos references under src/"; fi
```

```output
fmt clean
clippy clean with -D warnings
      3 test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      2 test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      2 test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      2 test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      2 test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      2 test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      1 test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
doctests:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
no inline tests under src/
no Paxos references under src/
```

## 6. The paper

`paper/build.sh` runs Tectonic on `paper/paper.tex` with a fixed `SOURCE_DATE_EPOCH`, so
the PDF is byte-identical across builds with the same Tectonic release and the same
package bundle. Tectonic downloads and caches the standard packages on its first run; a
verifier without that cache needs network access once. The digest of the PDF, its page
count and its metadata are recorded, followed by the digest of the source.

```bash
sh paper/build.sh >/dev/null 2>&1 && shasum -a 256 paper/paper.pdf
pdfinfo paper/paper.pdf | grep -E "^(Title|Author|Pages)"
printf "overfull boxes: %s\n" "$(grep -c Overfull paper/paper.log || true)"
```

```output
d989ce84b65e1b9bd72edc31735d6220c8240020e8abca801c111adf79ab09ae  paper/paper.pdf
Title:           An Executable Safety Ladder Toward Unbounded Viewstamped Replication
Author:          Simon Massey
Pages:           8
overfull boxes: 0
```

```bash
shasum -a 256 paper/paper.tex paper/build.sh
```

```output
244b21337f090a9703a205ecf1de4798dd22f57b9bfdd7ad1122a64aa9d8534b  paper/paper.tex
e3f5d3b629938c30d42840648a9fc6331c78cbbebfb1606c5042705fb2ca5212  paper/build.sh
```

## 7. Digests of the transcripts and check scripts

These digests bind this document to the exact rung transcripts and check scripts that
were replayed above.

```bash
shasum -a 256 ladder/*.md check_axioms.py check_mutations.py
```

```output
46b0c50675f7172d80332b0d9d04c997a75f162cfcf6e2a9daabe0ef11dc7042  ladder/01-structure.md
d9d7081101073de0a349d8394ccc84bccc7e8777ad04e4e1296dc870857542a9  ladder/02-lex-ballots.md
1e3d3ed80f213ff7876eb32d2756bf422de7e41b13cf2d2bf8c53a1e09cde63c  ladder/03-synod-agreement.md
f95c9dc7a14809a24177cedef7ed82e0eae4453774a7bf38b2c85445c541880c  ladder/04-eras-theorem10.md
2bfc5a8b82f5d45fe355bdf46282f3a251883cd3afd36e988bd6703de72c8918  ladder/05-counterexample.md
bf1a958801503eb1f82d8ad9dcb7ad647646d397f5dac3ed2f10fba7c74a9921  ladder/06-casting-vote.md
0e5dbe8674dd6601b0e2ad16c454b6a4dd0004d7c8c5698641d0bf3bb389eccf  ladder/07-weights.md
f7cbc44fe61a1511e5761da1c8bc10e1e4ef3183eae6fadac3f37c3ff771e758  ladder/08-leanstral-lemma2.md
c4449a0806fe9afd04bb5337c4301c19a486af0694f5f2a0d14229696ec578ef  ladder/09-weighted-general.md
c010a7e1157616c60554d86ef971a41ade8f1728cae9ff248b0da7aabf21c541  ladder/10-acceptor-transitions.md
cb9f5e2becc3d7499c017f0c01ef6786ea27a6534649d47f1f3f04efe348c650  ladder/11-multi-promises.md
04906536a4f507676ea8f612bda6d8b2a347ea2212f3b7bcce322403d6f671b1  ladder/12-negative-controls.md
8d01d27d25e2a1581bcc07192b815709ca632a565a1033d763827300cb5e1650  ladder/13-view-selection.md
4ba1b60110417289a9045d9931d3fb1854d17aacbe51e217010290134414645f  ladder/14-normal-log.md
f49557de254c33eb55317d1907e83abc84f50c378d3e7571c1acdaa7edbe955b  ladder/15-view-fence.md
57b9623d0dc25838188314dfe077a97a9a7a6d26c390f09a9514fbeb39888c13  ladder/16-log-provenance.md
371ce4eeb2ce5ec33538cf76ea989dbf957fbda318da486d4ba1ab8ea7db16e1  ladder/17-recovery-fence.md
d08b444567a5b071a30578efde0d160b5ead8b6079bcbdec44fea762efa5e071  ladder/18-crash-vector.md
2643b179950d597fe9ea2cc98067a2235a9bb83d94abe173a3e97b6422022c18  ladder/19-acquisition-order.md
cee41077b7268279ad8061261f3d7fdf315d8d2e24bfc5eba5788d4d402316fc  ladder/20-recovery-acquisition.md
a4add3a8c2c0ee28c1f3d75d3e0e2a4f87132f3bd38c48fafaae68357a96c6e8  check_axioms.py
2617b524ded9fff554c7418054a44dd8095a51212d21284801fc665a54f6ca6f  check_mutations.py
```

## 8. What remains open

Everything above is either a kernel-checked component theorem, a finite model check, or an
implementation test. The end-to-end uVRR claim is not yet proved. The obligations recorded
in `LAB-BOOK.md` and in the paper's closing section are: the four-superblock durable
identity contract, the reincarnation wire message and forced weight sequence
(Crash-Stop-Self-Evict), quorum-knowledge persistence and value reconstruction for the
operational acquisition of rung 20, whole-log preservation across an arbitrary sequence of
reconfigurations, client-visible linearizability, conditional progress, and an explicit
refinement between the Rust implementation and the checked models.

