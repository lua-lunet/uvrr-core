#!/bin/sh
# Check paper prose with Aspell's British-English dictionary in TeX mode.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
WORDS="$(pwd)/british-words.txt"
PAPER="${1:-paper.tex}"

if [ ! -r "$PAPER" ]; then
    printf 'Cannot read TeX source: %s\n' "$PAPER" >&2
    exit 2
fi

if ! command -v aspell >/dev/null 2>&1; then
    printf '%s\n' 'British-English spell check requires Aspell. On macOS: brew install aspell' >&2
    exit 127
fi

if ! aspell dump dicts | grep -qx 'en_GB'; then
    printf '%s\n' 'Aspell does not provide the en_GB dictionary.' >&2
    exit 127
fi

# TikZ's `align=center` is a presentation key, not prose. Replacing just this
# key avoids whitelisting the American spelling `center` in the project list.
MISSPELLINGS=$(sed 's/align=center/align=left/g' "$PAPER" |
    aspell --lang=en_GB --mode=tex --add-wordlists="$WORDS" --ignore=2 list |
    LC_ALL=C sort -fu)

if [ -n "$MISSPELLINGS" ]; then
    printf '%s\n' 'British-English spell check failed:' >&2
    printf '%s\n' "$MISSPELLINGS" >&2
    printf '%s\n' 'Correct the prose or add an intentional technical name to british-words.txt.' >&2
    exit 1
fi

printf '%s\n' 'PASS British-English spelling (en_GB)'
