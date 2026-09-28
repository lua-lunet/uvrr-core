#!/bin/zsh
# agent366 text extraction — one .md per source in ../md/.
# Born-digital PDFs: pdftotext -layout. Scanned PDFs: pdftoppm + tesseract.
# paper.pdf gets BOTH (user asked for tesseract on it; pdftotext kept for quality).
set -u
P="$(dirname "$0")/../papers"
M="$(dirname "$0")"
mkdir -p "$M"

txt() { # $1 = pdf name (no ext)
  [ -s "$M/$1.md" ] && { echo "SKIP $1.md"; return 0; }
  [ -s "$P/$1.pdf" ] || { echo "MISS $1.pdf"; return 1; }
  pdftotext -layout "$P/$1.pdf" "$M/$1.md" && echo "TXT  $1.md ($(wc -c < "$M/$1.md" | tr -d ' ') bytes)"
}

ocr() { # $1 = pdf name (no ext), $2 = output suffix (default .md)
  local out="$M/$1${2:-.md}"
  [ -s "$out" ] && { echo "SKIP $1${2:-.md}"; return 0; }
  [ -s "$P/$1.pdf" ] || { echo "MISS $1.pdf"; return 1; }
  local d="$P/.ocr-$1"
  rm -rf "$d"; mkdir -p "$d"
  pdftoppm -r 300 -gray -png "$P/$1.pdf" "$d/pg" || { echo "FAIL-render $1"; return 1; }
  : > "$out"
  local f
  for f in "$d"/pg-*.png; do
    tesseract "$f" stdout -l eng 2>/dev/null >> "$out"
    printf '\n\n--- page break ---\n\n' >> "$out"
  done
  rm -rf "$d"
  echo "OCR  $1${2:-.md} ($(wc -c < "$out" | tr -d ' ') bytes)"
}

# born-digital
for n in vr-revisited-2012 paxos-made-simple-2001 vertical-paxos-2009 \
         vertical-paxos-disc-2017 reconfiguring-a-state-machine-2010 \
         osdi14-pillai-allfs sosp13-optimistic-crash fast18-alagappan-par \
         diskless-tr16 disk-paxos-2003 corfu-tocs2013 zookeeper-atc10-hunt \
         fqi-2016-howard raft-atc14-ongaro lean4-cade28 \
         paper_ladder paper_exceeding paper_results; do
  txt "$n"
done

# scans
for n in lampson-sturgis-1979 vr-1988-oki-liskov oki-dissertation-tr423; do
  ocr "$n"
done

# paper.pdf: both routes — pdftotext for fidelity, tesseract as requested
txt paper
ocr paper .ocr.md
echo DONE
