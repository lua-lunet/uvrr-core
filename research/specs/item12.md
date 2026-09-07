# item12 — Kick the tires: Lean 4 tools (Veil / LeanLTL / lean-auto) — TIMEBOXED

Timebox: ~15 minutes total. If the Lean toolchain is missing and elan install is
not trivial, SKIP all and record why. If one tool is awkward, skip that tool only.
Do not touch `formal/` or existing tracked files beyond the submodule additions.

Repo-level work EXPLICITLY approved by the user (git submodule; `git add` only —
NEVER commit).

Steps:
1. `command -v lean elan lake`. If absent: ONE timeboxed attempt
   `curl -sSfL https://elan.lean-lang.org/elan-init.sh | sh -s -- -y` then
   `elan default stable`. If slow/fails → skip with reason.
2. From repo root `/Users/Shared/lua-lunet/vrr-core` add submodules (reference
   copies; git add only, NEVER commit):
   `tools/veil` ← https://github.com/verse-lab/veil
   `tools/LeanLTL` ← https://github.com/UCSCFormalMethods/LeanLTL
   `tools/lean-auto` ← https://github.com/leanprover-community/lean-auto
3. Hello world in `.tmp/lean-hello/`: minimal `lakefile.toml` (or .lean) +
   `Hello.lean` proving `theorem hello : 1 + 1 = 2 := by rfl`; build with
   `lake build` (or `lake env lean Hello.lean`). Optionally, if Veil's own
   examples resolve quickly, try building ONE Veil example — else skip.
4. Append results to `.tmp/research/tool-kick-tires.md` under `## Lean 4 tools`
   (create if absent; the TLA+ agent may have created it).

Report back (<6 lines, no dumps): toolchain y/n, submodules added, hello proof
compiled y/n, skip reasons.
