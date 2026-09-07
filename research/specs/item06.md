# item06 — Sync AGENTS.md with subagent-delegation preference (APPROVED — runs last)

1. Append to `/Users/Shared/lua-lunet/vrr-core/AGENTS.md` a short section:

## Subagent delegation

- Agents SHOULD delegate major todo items to subagents per the
  opencode-subagent-delegation skill, wherever doing so does not overwrite any
  other instruction in this AGENTS.md or the user's prior statements of
  preference.

2. Do not alter any other content of AGENTS.md.
3. `git add AGENTS.md` but NEVER commit (the user handles commits).
4. Verify: the section appears exactly once; `git diff --cached AGENTS.md` shows
   only the appended hunk.

Report back (<4 lines, no dumps): added y/n, diff stat.
