# item11 — Kick the tires: TLA+ tooling (submodule + TLC hello world) — TIMEBOXED

Timebox: ~10 minutes. If anything is not easy to get/use, SKIP it and record why.
Do not fight the tooling. Do not touch `formal/` or any existing tracked files
beyond the submodule additions.

This is repo-level work EXPLICITLY approved by the user (git submodule; `git add`
only — NEVER commit; the user handles commits).

Steps:
1. `command -v java` — TLC needs Java 11+. If no Java, try `brew install --quiet
   temurin` once; if that is slow/fails, skip with reason.
2. From repo root `/Users/Shared/lua-lunet/vrr-core`:
   `git submodule add https://github.com/tlaplus/tla2tools tools/tla2tools`
   then `git add .gitmodules tools/tla2tools` — NEVER commit.
3. Hello world in `.tmp/tla-hello/`: `Hello.tla` (tiny Init/Next spec, e.g. a
   one-bit register toggling) + `Hello.cfg` (INIT/NEXT/INVARIANT). Get a TLC jar
   the easy way: `curl -L -o .tmp/tla-hello/tla2tools.jar
   https://github.com/tlaplus/tla2tools/releases/latest/download/tla2tools.jar`.
   Run: `java -cp .tmp/tla-hello/tla2tools.jar tlc2.TLC .tmp/tla-hello/Hello.tla`.
   (If the release download fails, trying a build is out of timebox — skip.)
4. Append results to `.tmp/research/tool-kick-tires.md` under a `## TLA+` heading
   (create the file). Note: repo already has real TLA+ specs under `formal/`
   (VrrCore.tla, VrrCoreEras.tla) — mention as context, do not run them.

Report back (<6 lines, no dumps): submodule added y/n + path, TLC ran y/n +
model-checking verdict, any skip reasons.
