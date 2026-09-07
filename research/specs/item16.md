# item16 — UPaxos/VRR sources: fetch blog posts + OCR diagrams, macOS

Repo root: /Users/Shared/lua-lunet/vrr-core. Platform: darwin. TIMEBOX ~30 min.

Goal: gather the user's source material for the uVRR proof effort: three blog
posts plus any UPaxos paper PDFs already on disk, with diagrams transcribed.

Posts:
1. https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/
2. https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/
3. https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/

Steps:
1. Fetch each post's text (tavily extract preferred, webfetch fallback). Save to
   `.tmp/blogs/<slug>.md`.
2. Diagrams are IMAGES on the pages: for each post, use the chrome-devtools MCP
   to open the URL, wait for load, and take targeted screenshots of each figure
   (scroll to it, screenshot; full-page screenshot as fallback). Save PNGs to
   `.tmp/blogs/img/`.
3. OCR the screenshots with Mistral OCR via the existing script
   `.tmp/ocr_run.py` (reads MISTRAL_API_KEY SILENTLY from the repo-root `.env`
   via awk — never print, copy, or grep the key; never copy .env). Save OCR text
   next to the images as `.tmp/blogs/img/<name>.ocr.md`. Where the OCR is poor
   for a sequence diagram, ALSO transcribe the diagram by eye from the
   screenshot yourself and write your own message-flow table, marked as
   "hand-transcribed".
4. Search the repo checkout for the UPaxos paper PDFs the user says are on disk:
   glob `**/*.pdf` (repo + `.tmp/papers/`). If an UPaxos paper PDF is found, OCR
   it the same way to `.tmp/blogs/ocr-upaxos-paper.md`; note its path.
5. Write `.tmp/blogs/SOURCES.md`: per source — URL/path, what it contains, list
   of diagrams with figure numbers and what each shows.

Report back (<6 lines, no content dumps): posts fetched y/n, images captured
count, OCR done count, UPaxos paper found y/n + path, output root. Never echo
the key.
