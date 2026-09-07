#!/bin/sh
# Build beside this script, independently of Lean, Rust, Showboat or any model service.
# SOURCE_DATE_EPOCH is fixed unless the caller overrides it, so the PDF is byte-reproducible
# for a given Tectonic release and package bundle (2026-09-06T00:00:00Z).
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
if ! command -v tectonic >/dev/null 2>&1; then
    printf '%s\n' 'Tectonic is required. On macOS: brew install tectonic' >&2
    exit 127
fi
: "${SOURCE_DATE_EPOCH:=1757116800}"
export SOURCE_DATE_EPOCH
exec tectonic --keep-logs "$@" paper.tex
