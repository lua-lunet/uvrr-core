# item09 — Install a proper version of the user's skill gist

Source gist: https://gist.github.com/simbo1905/c2a0ac48eee089dd176e7f2b7432dac3
(file `skill.md`). It belongs to the user's own GitHub account (gh is
authenticated as simbo1905).

Steps:
1. Fetch the gist content: `gh gist view c2a0ac48eee089dd176e7f2b7432dac3` (or the
   raw gistusercontent URL). If it fails, report and stop.
2. Turn it into a proper opencode skill: create
   `~/.config/opencode/skills/<name>/SKILL.md` where `<name>` is a
   lowercase-hyphen slug derived from the gist's own name/intent. Frontmatter:
   ---
   name: <name>   (matches folder name, ≤64 chars, lowercase-hyphen)
   description: <one sentence, third person, what it does + when to trigger, front-loaded keywords>
   ---
   Body: the gist's instructions cleaned into proper markdown. Preserve the
   author's intent; fix formatting only. NOTE: this install location is OUTSIDE
   `.tmp/` — explicitly approved by the user for this item ONLY. Do not touch
   anything else outside .tmp.
3. Verify: file exists, frontmatter name matches folder name, body non-empty.

Report back (<5 lines, no content dumps): skill name, install path, the
description line, and a note that opencode must be quit and restarted to load it.
