# FAST '18 best paper — reference manifest

Source: Ramnatthan Alagappan, Aishwarya Ganesan, Eric Lee, Aws Albarghouthi,
Vijay Chidambaram, Andrea C. Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau,
"Protocol-Aware Recovery for Consensus-Based Storage," Proc. 16th USENIX
Conference on File and Storage Technologies (FAST '18), Oakland, CA, February
2018 (Best Paper Award), printed pp. 15–31.

## Acquisition and pinning

- `fast18-alagappan.pdf` — the official proceedings copy, 18 pages (USENIX
  cover sheet + body pp. 15–28 + references pp. 29–31), US Letter
  612×792 pt, iText-produced, created 30 January 2018, modified 6 February
  2018. sha256
  `b18cc6fc2d6605e54a153697b608ac3d60bc363bacd6a8395860430286af5dc5`.
  Fetched from the paper's proceedings page
  https://www.usenix.org/conference/fast18/presentation/alagappan, which
  links the paper as
  https://www.usenix.org/system/files/conference/fast18/fast18-alagappan.pdf.
  Markers verified: full title, all seven authors with affiliations
  (UW–Madison; UT Austin), the FAST '18 proceedings cover sheet
  (ISBN 978-1-931971-42-3), and running footers
  "16th USENIX Conference on File and Storage Technologies". A second
  public revision exists — the authors' mirror
  https://www.cs.utexas.edu/~vijay/papers/fast18-par.pdf, a 17-page
  camera-ready without the USENIX cover sheet (body text verified
  identical on a compared page); the official proceedings copy is
  pinned here. The official slides supplement
  (fast18_slides_alagappan.pdf, fetched to scratch only) carries no
  appendix material.
- No appendix exists in this paper (the Director's phrase "Appendix X 3.1
  amnesia recovery flaw" has no referent in the proceedings copy; §3.1 of
  the paper is the Fault Model). The amnesia recovery flaw is stated in
  the body — see `summary.md`, which pins it to §2.3 (printed pp. 17–18,
  Figure 2 walkthrough on p. 18) with the verbatim statement quoted, and
  notes the empirical confirmation in §5.1.1 (printed p. 25, Table 4(a)).
  Note also that the paper never uses the word "amnesia"; its own
  vocabulary is "silent data loss".

## Text extraction

- `text.txt` — sha256
  `839012a70e839dc15462e61cbc9e51d3b15535e477e7649001ca6850c3b96b95`.
  Produced with `pdftotext -layout` (poppler). Faithful verbatim text;
  two-column layout interleaves columns on some lines (headings sit
  mid-line against the other column's text).
- Cross-check: PDF pages 2, 4, 8 and 12 (printed 15, 17, 21, 25) rendered
  at 150 dpi (`pdftoppm`) and re-read with `tesseract --psm 1`.
  Agreements: title/authors/abstract, the Truncate/DeleteRebuild
  failure-mode prose ("it truncates its log, losing the data locally",
  "data is silently lost, violating safety", "silent data"), the
  "stale node, overwriting" sentence of the introduction, and the §5.1.1
  percentages all match verbatim. Disagreements, all of the expected
  OCR-of-markup class, none affecting `text.txt`: tesseract mangles
  cross-reference superscripts and citation brackets ("Table 1" as
  "Table i}", "Figure 1's" as "Figure {ls", "Figure 1(i)" as
  "Figure fo. S)", "S1" as "S|"), which pdftotext preserves.

## Summary

- `summary.md` — sha256
  `988dbbb94fa051656c44c0945515627e0aa99b07d573a51932c107a70d2caa6d`.
  Section-by-section claims with printed-page pointers, the full recovery
  taxonomy, the amnesia (silent-data-loss) class pinned with quotes, the
  CTRL design, and evaluation numbers. Drafted with Mistral
  `mistral-large-latest` over four page-chunks, then curated line-by-line
  against `text.txt`: all section/page pointers re-derived from the
  running footers, and evaluation numbers that only appear in later
  chunks (which early draft passes had hallucinated, e.g. nonexistent
  Figures 6–8 and invented case counts) were replaced with verified
  values.

## Figures

Five figures, extracted as full-page vector SVGs with `pdftocairo -svg`
(one per figure's page), cropped by rewriting the root
`width`/`height`/`viewBox` to the figure region (top-left origin, points;
page content unmodified vector). Caption text is excluded from the crops;
Figure 5's (a)–(d) sub-labels are included. Filenames use the paper's
printed page numbers.

| Figure | Printed page (PDF page) | Caption (excluded from crop) | Crop box (x, y, w, h) |
|--------|-------------------------|------------------------------|------------------------|
| 1 | 17 (4) | Figure 1: Sample Scenarios. | 70, 60, 232, 95 |
| 2 | 18 (5) | Figure 2: Safety Violation Example. | 54, 64, 246, 88 |
| 3 | 19 (6) | Figure 3: Log Format. | 306, 60, 258, 112 |
| 4 | 22 (9) | Figure 4: Distributed Log Recovery. | 52, 54, 512, 126 |
| 5 | 26 (13) | Figure 5: Common-Case Write Performance. | 52, 54, 512, 109 |

Tables 1–5 are not banked as figures (Table 1 and Table 2 carry the
taxonomy and fault model and are quoted in `summary.md` and `text.txt`).

PNG renders (`fig-<n>-page-<p>.png`): headless Google Chrome
(`--headless=new --force-device-scale-factor=4 --window-size=<intrinsic
96-dpi px> --hide-scrollbars --default-background-color=FFFFFFFF`,
screenshot of the SVG file), i.e. 96 dpi natural size at 4× device scale
≈ 5.33 px/pt. All PNGs are well under 2 MB (largest ≈ 140 KB). Crop
fidelity was verified by eye against 72-dpi `pdftoppm` renders of the
same page regions (caption sliver / clipped legend iterations are
resolved in the final boxes above).

`fig-<n>.description.md`: standalone descriptions produced with Mistral
`mistral-medium-latest` vision (chat completions, image passed as a base64
`data:` URI `image_url`), prompted to describe only what is visible —
rows, cells, arrows, panels, verbatim labels, shading/hatching, ordering —
with no source context. These are the reference set for comparison against
identically produced descriptions of any original redrawings. Known vision
quirks on these reads: hatch placement is approximate (Figure 1's per-cell
hatching is partially misattributed; Figure 4's epoch superscripts
transcribe informally), and bar heights/axis values in Figure 5 are
estimates, not readings — the printed normalization ratios above the bars
are transcribed verbatim.

## sha256 inventory

```
350566c92202c64010e2e085f5770dd07b4f6bc6db04d1f4b147b7b39cace06e  figures/fig-1.description.md
ef14098d47594de9d07a3ca5e7af1deb2209392c089c919dc8693ed4972da312  figures/fig-1-page-17.png
216347d741e3962e8a13dd930269d5d3a56e71361bf955cb935239ab707c729f  figures/fig-1-page-17.svg
f93063497ddc3c52e0ebfd771463afc5bde9b34e538a192c64c3c5dbe4c4b736  figures/fig-2.description.md
e13628b5b67987094565d612eca7c3b72ec6cc03a7beecba9ee6bf08f74a4985  figures/fig-2-page-18.png
f29f7f2209d718bd0447e333e163814e7870323d8499e184afb50c3116d4ec94  figures/fig-2-page-18.svg
f8c7d050cb1aebe0cd6bc5e12cdc01b66ee5a65687fc88392ef3b4665932d3a3  figures/fig-3.description.md
74b25f42ed62756b264ba52c6bd5c1e2a818a530ac0d31b509af5e6b373b2fd6  figures/fig-3-page-19.png
8fa03510dfb9c1f240c582d6e0db9800a2ba74087d00a92e9ca7369ade2da79f  figures/fig-3-page-19.svg
62b353b47d4b7ffeaca459b5775550306711239a03f57961d4828f2ffac0cbd5  figures/fig-4.description.md
dead13b2df73779180761d0fe62cda714888a4373c5a56766ff6ca9e9588362c  figures/fig-4-page-22.png
1ac558843f3e8b6260d46ee8ba31a4cd6854302a6ee5d6dbddcc846eb5838853  figures/fig-4-page-22.svg
188ac01974b53c90d6f32f853d4b0f6258786aaad51013ceb9372b06bcccf430  figures/fig-5.description.md
319f365261654056d7775d0a6d3035bb8ca44a2f582b8b008d8e321c29bcacbc  figures/fig-5-page-26.png
5960619730041e061c8045de95fc12c7f384ef542a6eb07a43d9fb1d67e4c1f1  figures/fig-5-page-26.svg
b18cc6fc2d6605e54a153697b608ac3d60bc363bacd6a8395860430286af5dc5  fast18-alagappan.pdf
988dbbb94fa051656c44c0945515627e0aa99b07d573a51932c107a70d2caa6d  summary.md
839012a70e839dc15462e61cbc9e51d3b15535e477e7649001ca6850c3b96b95  text.txt
```

## License

© 2018 The Authors / USENIX Association; the proceedings are open access
under a CC BY-like USENIX distribution policy. Extraction, banking, and
redescription here is reference use with attribution.
