#!/bin/sh
# Build beside this script, independently of Codex, Lean, or the test harness.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
if ! command -v tectonic >/dev/null 2>&1; then
    printf '%s\n' 'Tectonic is required. On macOS: brew install tectonic' >&2
    exit 127
fi
exec tectonic --keep-logs "$@" paper.tex
