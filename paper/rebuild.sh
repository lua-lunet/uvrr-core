#!/usr/bin/env sh
# Rebuild the paper LaTeX document and open the resulting PDF.
# Safe to execute from any working directory on Darwin or Linux.
set -eu

# Ensure common tool locations (Homebrew on Apple Silicon/Intel, Cargo) are in PATH
for p in /opt/homebrew/bin /usr/local/bin "$HOME/.cargo/bin"; do
    case ":${PATH:-}:" in
        *":$p:"*) ;;
        *) [ -d "$p" ] && PATH="$p${PATH:+:$PATH}" ;;
    esac
done
export PATH

# Resolve script directory portably
SCRIPT_DIR="$(CDPATH="" cd -- "$(dirname -- "$0")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v tectonic >/dev/null 2>&1; then
    printf '%s\n' 'tectonic is required. On macOS: brew install tectonic' >&2
    exit 127
fi

TARGET="paper.tex"
DO_OPEN=1

for arg in "$@"; do
    case "$arg" in
        --no-open)
            DO_OPEN=0
            ;;
        --open)
            DO_OPEN=1
            ;;
        *.tex)
            TARGET="$arg"
            ;;
        *)
            if [ -f "${arg}.tex" ]; then
                TARGET="${arg}.tex"
            else
                printf 'unknown option or target: %s\n' "$arg" >&2
                exit 2
            fi
            ;;
    esac
done

if [ ! -f "$TARGET" ]; then
    printf 'target file not found: %s\n' "$TARGET" >&2
    exit 1
fi

BASE_NAME="${TARGET%.tex}"
PDF_OUT="${BASE_NAME}.pdf"

# 1. Generate version.tex with current timestamp and git revision
SHORT_SHA="$(git rev-parse --short HEAD 2>/dev/null || printf 'draft')"
if ! git diff --quiet HEAD -- . 2>/dev/null; then
    SHORT_SHA="${SHORT_SHA}-dirty"
fi
DATE_STR="$(date +%Y%m%d)"
printf '\\renewcommand{\\paperversion}{%s-%s}\n' "$DATE_STR" "$SHORT_SHA" > version.tex

# 2. Run spellcheck if available
if [ -x "./spellcheck.sh" ]; then
    ./spellcheck.sh
fi

# 3. Generate diagrams if missing
if [ ! -f "reincarnation.svg" ] && [ -f "reincarnation.trace" ] && command -v uv >/dev/null 2>&1; then
    uv run msgtrace.py reincarnation.trace
elif [ ! -f "reincarnation.svg" ] && [ -f "reincarnation.trace" ] && command -v python3 >/dev/null 2>&1; then
    python3 msgtrace.py reincarnation.trace reincarnation.svg
fi

if [ ! -f "turner-fig3.svg" ] && [ -f "turner-fig3.trace" ] && command -v uv >/dev/null 2>&1; then
    uv run msgtrace.py turner-fig3.trace
elif [ ! -f "turner-fig3.svg" ] && [ -f "turner-fig3.trace" ] && command -v python3 >/dev/null 2>&1; then
    python3 msgtrace.py turner-fig3.trace turner-fig3.svg
fi

# 4. Compile with tectonic
printf 'Compiling %s with tectonic...\n' "$TARGET"
tectonic --keep-logs "$TARGET"

if [ ! -f "$PDF_OUT" ]; then
    printf 'Error: expected output %s was not created.\n' "$PDF_OUT" >&2
    exit 1
fi

printf 'Built: %s/%s\n' "$SCRIPT_DIR" "$PDF_OUT"

# 5. Open output if requested and a desktop viewer is available
if [ "$DO_OPEN" -eq 1 ]; then
    if [ "$(uname -s)" = "Darwin" ] && command -v open >/dev/null 2>&1; then
        open "$PDF_OUT"
    elif command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$PDF_OUT" >/dev/null 2>&1 || true
    fi
fi
