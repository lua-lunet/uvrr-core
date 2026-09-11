#!/bin/sh
# Tesseract OCR pass over the paper/research corpus.
#
# Rasterises each PDF at 200 dpi grayscale with pdftoppm, OCRs every page
# with tesseract (eng), and concatenates the pages into
# ocr-tesseract/<pdf-stem>.txt with "---" page separators. Skips PDFs whose
# output already exists and is non-trivial. Scratch images go to the
# repository .tmp/ and are removed afterwards.
#
# Usage: ./ocr-tesseract.sh   (from formal/uvrr-lean/paper/research)
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
PDFS="$HERE/pdf"
OUT="$HERE/ocr-tesseract"
SCRATCH="$HERE/../../../.tmp/research-ocr-tesseract"

mkdir -p "$OUT" "$SCRATCH"

for pdf in "$PDFS"/*.pdf; do
    stem=$(basename "$pdf" .pdf)
    txt="$OUT/$stem.txt"
    if [ -s "$txt" ] && [ "$(wc -c < "$txt")" -gt 100 ]; then
        echo "skip $stem (already OCR'd)"
        continue
    fi
    echo "ocr  $stem ..."
    rm -rf "$SCRATCH/$stem"
    mkdir -p "$SCRATCH/$stem"
    pdftoppm -r 200 -gray -png "$pdf" "$SCRATCH/$stem/page"
    : > "$txt"
    for img in $(ls "$SCRATCH/$stem"/page-*.png | sort -V); do
        tesseract "$img" stdout -l eng 2>/dev/null >> "$txt"
        printf '\n---\n' >> "$txt"
    done
    pages=$(ls "$SCRATCH/$stem"/page-*.png | wc -l | tr -d ' ')
    chars=$(wc -c < "$txt" | tr -d ' ')
    rm -rf "$SCRATCH/$stem"
    echo "done $stem  pages=$pages chars=$chars"
done
