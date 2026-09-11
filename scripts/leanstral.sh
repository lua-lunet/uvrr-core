#!/usr/bin/env bash
# leanstral.sh — drive Leanstral (Mistral) + the local Lean toolchain as a
# full code/compile/debug/proof loop.
#
# Three channels exist (mistral.ai/news/leanstral and /news/leanstral-1-5):
#   1. THIS SCRIPT — the Labs API loop: generate -> compile -> feed the exact
#      compiler errors back -> refine, until the file compiles with no `sorry`
#      (the multiturn environment Leanstral 1.5 was trained in).
#   2. Vibe agent mode — `vibe -p "<prompt>" --agent lean --auto-approve
#      --max-turns 12` (the model edits files and runs bash itself; trained
#      for lean-lsp-mcp). Record: research/leanstral-vibe-smoke.md.
#   3. Local weights — mistralai/Leanstral-2603 on Hugging Face, Apache-2.0,
#      on your own metal.
#
# Known quirks, all learned the hard way (research/mistral-lean/VERDICT.md,
# research/leanstral-vibe-smoke.md, research/claude-plus-leanstral/):
#   - The Labs endpoint rejects `temperature: 0` alone with HTTP 400 code
#     3054 ("top_p must be 1 when using greedy sampling") — always send
#     top_p: 1 alongside.
#   - Preserve project imports and forbid new dependencies; imported local
#     lemmas are part of the proof target's environment.
#   - The model reaches for `native_decide` (compiled evaluation, not
#     kernel-checked); this script rejects it — a proof here means the
#     kernel checked it.
#   - The model over-proves: it induction-bombs goals the standard library
#     already settles (a+b=b+a is Nat.add_comm). The prompt tells it to
#     reach for the library first.
#   - It writes core lemma names UNQUALIFIED (`add_comm`, `add_succ`) which
#     fail as unknown identifiers — the feedback loop usually repairs this;
#     when it stalls, the human lands the qualified name (the recorded
#     "Claude + Leanstral" pattern: the orchestrator lands the last step).
#   - A rejection the compiler cannot see (native_decide compiles clean)
#     needs the VERDICT in the feedback, not just compiler output — an
#     empty feedback file makes the model repeat itself forever.
#   - The free Labs endpoint is listed for retirement on 2026-09-30; set
#     LEANSTRAL_MODEL to move (labs-leanstral-1-5-1 today).
#
# Usage:
#   scripts/leanstral.sh check <file.lean>          # compile + verdict only
#   scripts/leanstral.sh prove <file.lean> [rounds] # the full loop (default 4 rounds)
#
# The Lean toolchain must be on PATH (elan). The key is read from
# MISTRAL_API_KEY or the repo's .env — never printed, never stored. The API
# prompt is assembled inside python (never through shell expansion: a raw
# backtick pair in a heredoc once executed ImageMagick's `import`).

set -euo pipefail

MODEL="${LEANSTRAL_MODEL:-labs-leanstral-1-5-1}"
API="https://api.mistral.ai/v1/chat/completions"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEAN="${LEAN_BIN:-lean}"

die() { echo "leanstral.sh: $*" >&2; exit 1; }

get_key() {
    if [ -n "${MISTRAL_API_KEY:-}" ]; then
        printf '%s' "$MISTRAL_API_KEY"
        return
    fi
    [ -f "$REPO_ROOT/.env" ] || die "no MISTRAL_API_KEY in env and no .env at $REPO_ROOT"
    awk -F= '/^MISTRAL_API_KEY=/{print $2}' "$REPO_ROOT/.env" \
        | tr -d '"' | tr -d "'" | grep -v '^$' \
        || die "MISTRAL_API_KEY is empty in $REPO_ROOT/.env"
}

# check <file> — compile and verdict. PASS requires: lean exit 0, no `sorry`
# warning, no `native_decide`. Prints the compiler output on failure.
check() {
    local file="$1"
    [ -f "$file" ] || die "no such file: $file"
    local out
    if ! out=$("$LEAN" "$file" 2>&1); then
        printf '%s\nFAIL: does not compile\n' "$out"
        return 1
    fi
    if printf '%s' "$out" | grep -q 'declaration uses `sorry`'; then
        printf '%s\nINCOMPLETE: still contains `sorry`\n' "$out"
        return 1
    fi
    if grep -q 'native_decide' "$file"; then
        printf '%s\nREJECTED: uses native_decide (not kernel-checked). Replace every native_decide with a kernel-checked tactic: rfl, decide, simp, omega, or the matching core lemma applied directly (for example a + b = b + a is Nat.add_comm).\n' "$out"
        return 1
    fi
    printf 'PASS: compiles clean, kernel-checked, no sorry\n'
    return 0
}

# ask — one API round. Reads the proof target from $LEANSTRAL_FILE and the
# previous round's compiler feedback from $LEANSTRAL_FEEDBACK (empty file =
# first round). Writes candidate.lean (the single fenced block).
ask() {
    local key
    key=$(get_key)
    KEY="$key" MODEL="$MODEL" API="$API" \
    LEANSTRAL_FILE="$LEANSTRAL_FILE" LEANSTRAL_FEEDBACK="$LEANSTRAL_FEEDBACK" \
    LEANSTRAL_CANDIDATE="$LEANSTRAL_CANDIDATE" \
    python3 - <<'PYEOF'
import json, os, re, sys, urllib.request, urllib.error

with open(os.environ["LEANSTRAL_FILE"]) as f:
    current = f.read()
feedback = ""
fb_path = os.environ["LEANSTRAL_FEEDBACK"]
if os.path.getsize(fb_path) > 0:
    with open(fb_path) as f:
        feedback = f.read()

prompt = (
    "You are completing a Lean 4 file. Rules, all mandatory:\n"
    "- Output the FULL file, exactly one fenced lean code block, no prose outside it.\n"
    "- Preserve the existing imports exactly. Do not add imports or dependencies.\n"
    "- Replace every `sorry` with a kernel-checked tactic proof. Do NOT use\n"
    "  `native_decide` — the kernel must check the proof.\n"
    "- REACH FOR THE STANDARD LIBRARY FIRST: if the goal is an instance of a\n"
    "  core lemma (for example a + b = b + a is `Nat.add_comm`), apply or simp\n"
    "  with that lemma; do not re-prove library facts by induction from first\n"
    "  principles. `omega` settles linear arithmetic goals outright.\n"
    "- Do not weaken, restate, or remove any theorem.\n"
    "- Keep every theorem statement byte-identical.\n"
    "\n"
    "The current file:\n"
    "```lean\n" + current + "\n```\n"
)
if feedback:
    prompt += (
        "\nThe previous attempt did NOT pass the Lean compiler. The exact\n"
        "compiler output follows; fix precisely what it names:\n\n"
        + feedback
    )

payload = {
    "model": os.environ["MODEL"],
    "temperature": 0,
    "top_p": 1,  # the Labs endpoint rejects temperature 0 alone (code 3054)
    "messages": [{"role": "user", "content": prompt}],
}
req = urllib.request.Request(
    os.environ["API"],
    data=json.dumps(payload).encode(),
    headers={
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": "Bearer " + os.environ["KEY"],
    },
)
try:
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.loads(resp.read().decode())
except urllib.error.HTTPError as e:
    sys.exit(f"HTTP {e.code}: {e.read().decode()[:400]}")

content = data["choices"][0]["message"]["content"]
blocks = re.findall(r"```(?:lean|Lean)?\s*\n(.*?)```", content, re.DOTALL)
if len(blocks) != 1:
    sys.exit(f"expected exactly one fenced lean block, got {len(blocks)}")
with open(os.environ["LEANSTRAL_CANDIDATE"], "w") as f:
    f.write(blocks[0].strip() + "\n")
PYEOF
}

# prove <file> [rounds] — the full loop: the file's `sorry`s are the proof
# obligations; each round sends the current file (plus the previous round's
# exact compiler output when a round failed) and replaces the file with the
# returned block, then checks.
prove() (
    local file="$1"
    local rounds="${2:-4}"
    [ -f "$file" ] || die "no such file: $file"
    local round=1
    LEANSTRAL_FILE="$(cd "$(dirname "$file")" && pwd)/$(basename "$file")"
    LEANSTRAL_WORK="$(mktemp -d -t leanstral.XXXXXX)"
    trap 'rm -rf "$LEANSTRAL_WORK"' EXIT
    LEANSTRAL_FEEDBACK="$LEANSTRAL_WORK/feedback.txt"
    LEANSTRAL_CANDIDATE="$LEANSTRAL_WORK/candidate.lean"
    : > "$LEANSTRAL_FEEDBACK"
    while [ "$round" -le "$rounds" ]; do
        echo "== round $round/$rounds ==" >&2
        if ! ask; then
            echo "API round failed; retrying with the same feedback next round" >&2
            round=$((round + 1))
            continue
        fi

        cp "$LEANSTRAL_CANDIDATE" "$file"
        local verdict
        if verdict=$(check "$file"); then
            echo "$verdict"
            echo "PROVED in $round round(s): $file" >&2
            return 0
        fi
        echo "$verdict" >&2
        # The feedback must carry the VERDICT reason (the model cannot read
        # our minds: a native_decide rejection compiles clean, so compiler
        # output alone would be empty and the model would repeat itself).
        printf '%s\n' "$verdict" > "$LEANSTRAL_FEEDBACK"
        "$LEAN" "$file" >> "$LEANSTRAL_FEEDBACK" 2>&1 || true
        round=$((round + 1))
    done
    die "budget exhausted after $rounds rounds: $file still not green (last feedback: $(head -3 "$LEANSTRAL_FEEDBACK"))"
)

case "${1:-}" in
    check) shift; [ $# -eq 1 ] || die "usage: check <file.lean>"; check "$1" ;;
    prove) shift; [ $# -ge 1 ] && [ $# -le 2 ] || die "usage: prove <file.lean> [rounds]"; prove "$@" ;;
    *) die "usage: $0 {check <file.lean> | prove <file.lean> [rounds]}" ;;
esac
