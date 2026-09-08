#!/bin/sh
# Publish a versioned paper beside this script, independently of Lean, Rust,
# Showboat or any model service. The id is YYYYMMDD-<short HEAD sha>; it is
# stamped into every page footer via the generated, gitignored version.tex
# and the PDF is published as papers/<id>.pdf, an immutable version of
# record. An already-published id is never rebuilt, and unstaged edits to
# tracked sources here are refused: the id names HEAD's commit. SOURCE_DATE_EPOCH
# stays fixed unless the caller overrides it, so the layout is stable for a
# given Tectonic release and package bundle (2026-09-06T00:00:00Z).
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

if ! command -v tectonic >/dev/null 2>&1; then
    printf '%s\n' 'Tectonic is required. On macOS: brew install tectonic' >&2
    exit 127
fi
if ! command -v pdftotext >/dev/null 2>&1; then
    printf '%s\n' 'pdftotext is required for the footer assertion. On macOS: brew install poppler' >&2
    exit 127
fi

OCR=0
TECTONIC_ARGS=''
for arg in "$@"; do
    case "$arg" in
        --ocr) OCR=1 ;;
        --only-cached) TECTONIC_ARGS="$TECTONIC_ARGS --only-cached" ;;
        *) printf 'unknown flag: %s\n' "$arg" >&2; exit 2 ;;
    esac
done

ROOT=$(git rev-parse --show-toplevel)
ID="$(date +%Y%m%d)-$(git rev-parse --short HEAD)"
OUT="papers/$ID.pdf"

if ! git diff --quiet -- .; then
    printf 'REFUSED: tracked sources under paper/ have unstaged modifications:\n' >&2
    git diff --name-only -- . >&2
    printf 'The id %s names HEAD; publish the edits under their own commit id. Stage or commit first.\n' "$ID" >&2
    exit 1
fi

if [ -e "$OUT" ]; then
    printf 'REFUSED: %s already exists. A published paper is an immutable version of record.\n' "$OUT" >&2
    printf 'To reattempt deliberately: rm "%s/%s"\n' "$(pwd)" "$OUT" >&2
    exit 1
fi

: "${SOURCE_DATE_EPOCH:=1757116800}"
export SOURCE_DATE_EPOCH
printf '\\renewcommand{\\paperversion}{%s}\n' "$ID" > version.tex
mkdir -p papers
# shellcheck disable=SC2086
tectonic --keep-logs $TECTONIC_ARGS paper.tex
cp paper.pdf "$OUT"
printf '%s\n' "$ID" > .published-id

if ! pdftotext "$OUT" - | grep -qF "$ID"; then
    printf 'ASSERTION FAILED: footer id %s is missing from the PDF text; the publication is withdrawn.\n' "$ID" >&2
    rm -f "$OUT" .published-id
    exit 1
fi
printf 'published %s (footer id asserted, %s pages)\n' "$OUT" "$(pdfinfo "$OUT" | awk '/^Pages:/{print $2}')"

if [ "$OCR" -eq 1 ]; then
    if [ ! -f "$ROOT/.env" ]; then
        printf 'ocr: %s/.env not found; MISTRAL_API_KEY is required for --ocr\n' "$ROOT" >&2
        exit 1
    fi
    KEY=$(sed -n 's/^MISTRAL_API_KEY=//p' "$ROOT/.env" | tail -n 1 | tr -d \")
    if [ -z "$KEY" ]; then
        printf 'ocr: MISTRAL_API_KEY is not set in %s/.env\n' "$ROOT" >&2
        exit 1
    fi
    SCRATCH="$ROOT/.tmp/paper-ocr"
    mkdir -p "$SCRATCH"
    printf 'ocr: uploading %s to Mistral OCR\n' "$OUT"
    UPLOAD="$SCRATCH/upload-$ID.json"
    curl -sS -X POST https://api.mistral.ai/v1/files \
        -H "Authorization: Bearer $KEY" \
        -F purpose=ocr \
        -F "file=@$OUT" \
        -o "$UPLOAD"
    FILE_ID=$(python3 - "$UPLOAD" <<'PY'
import json, sys
try:
    d = json.load(open(sys.argv[1]))
except Exception as e:
    sys.exit(f"ocr: upload response is not JSON: {e}")
if not isinstance(d, dict) or "id" not in d:
    sys.exit(f"ocr: upload refused: {d.get('message', d) if isinstance(d, dict) else d}")
print(d["id"])
PY
)
    printf 'ocr: uploaded as %s; reading with mistral-ocr-latest\n' "$FILE_ID"
    RESP="$SCRATCH/ocr-$ID.json"
    curl -sS -X POST https://api.mistral.ai/v1/ocr \
        -H "Authorization: Bearer $KEY" \
        -H 'Content-Type: application/json' \
        -d "{\"model\":\"mistral-ocr-latest\",\"document\":{\"type\":\"file\",\"file_id\":\"$FILE_ID\"}}" \
        -o "$RESP"
    python3 - "$RESP" "$SCRATCH/ocr-$ID.md" paper.tex <<'PY'
import json, re, sys
resp, mdout, texpath = sys.argv[1:4]
d = json.load(open(resp))
pages = d.get("pages") if isinstance(d, dict) else None
if not isinstance(pages, list) or not pages:
    sys.exit(f"ocr: no pages in the OCR response: {d.get('message', d) if isinstance(d, dict) else d}")
md = "\n\n".join(str(p.get("markdown") or "") for p in pages)
open(mdout, "w").write(md)

def norm(s):
    for a, b in (("``", '"'), ("''", '"'), ("\u201c", '"'), ("\u201d", '"'),
                 ("\u2018", "'"), ("\u2019", "'")):
        s = s.replace(a, b)
    s = re.sub(r"\\[A-Za-z]+\s*", " ", s)
    s = s.replace("{", "").replace("}", "")
    return re.sub(r"\s+", " ", s).strip().lower()

tex = open(texpath).read()
captions = []
for m in re.finditer(r"\\caption\{", tex):
    i, depth, j = m.end(), 1, m.end()
    while j < len(tex) and depth:
        if tex[j] == "{":
            depth += 1
        elif tex[j] == "}":
            depth -= 1
        j += 1
    captions.append(tex[i:j - 1])
ocr_text = norm(md)
missing = [c for c in captions if norm(c) not in ocr_text]
if missing:
    for c in missing:
        print(f"ocr: caption not found in the OCR text: {c[:70]}", file=sys.stderr)
    sys.exit(f"ocr: FAIL - {len(missing)}/{len(captions)} captions missing; markdown kept at {mdout}")
print(f"ocr: OK - {len(pages)} pages read, {len(captions)}/{len(captions)} captions present; markdown at {mdout}")
PY
fi
