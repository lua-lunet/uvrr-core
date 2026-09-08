# Turner paper reference artifacts — manifest

Source: David C. Turner, "Unbounded Pipelining in Dynamically Reconfigurable Paxos
Clusters," revision 1A9DBA37, 14 August 2017 —
https://tessanddave.com/paxos-reconf-latest.pdf (10 pages, US Letter, 612×792 pt).

## Acquisition and pinning

- `paxos-reconf-latest.pdf` — sha256
  `3f5ffd4ecdf89f4dd8887a74b9f91d52914fcbe27183857cf68c6ff7cfd7f8ff`.
  The revision marker `1A9DBA37` appears in the running header of all 10 pages
  (`pdftotext` page stream), so the exact revision is pinned.
- Fetched over plain HTTP: the host's TLS certificate does not cover the requested
  hostname (it is served as `serval.mythic-beasts.com` only, on both `tessanddave.com`
  and `www.tessanddave.com`), so HTTPS transport could not be name-verified. Content
  integrity rests on the revision marker and the sha256 above.
- License: © 2016-7 Tracsis plc, CC BY-SA 4.0. Extraction and redescription here is
  reference use with attribution.

## Text extraction

- `text.txt` — sha256
  `b43ba62be656af15228677b273fd09eb119e85af485f1f0fe83a26c62d73cedb`.
  Produced with `pdftotext -layout` (poppler). This is the faithful verbatim text.
- Cross-check: pages 1, 3, 6 and 8 rendered at 150 dpi (`pdftoppm`) and re-read with
  `tesseract --psm 1`. Agreements: the revision marker reads identically on every
  sampled page; figure captions and distinctive prose phrases match verbatim.
  Disagreements, all of the expected OCR-of-math class, none affecting `text.txt`:
  - OCR reads the prime glyph ′ as an ASCII apostrophe, ℓ as ¢ or £, ∈ as €, and the
    "plc" ligature as "ple" in the small footer text; `pdftotext` preserves these.
  - `pdftotext` quirks of this dvips-produced PDF (known, systematic): the
    quorum-intersection symbol ⌢ (U+2322), ≺, ≻, ℓ, ⊆ and set symbols extract
    correctly, but `≠` extracts as the pair `6=`, the non-strict ballot-ordering
    relation in Lemmas 6–7 extracts as a single control byte 0x16 (the PDF uses a
    Type 3 glyph without a Unicode mapping; the relation is the reflexive closure of
    ≺), and subscripts are flattened (e.g. `QIe(b)` for Q^I_{e(b)}).

## Summary

- `summary.md` — sha256
  `968496c3a509d84a43f3305517a871d7a34ffc3167399156f7872f8948065da4`.
  Section-by-section claims, the full lemma/theorem list, and the figure contents,
  curated to timeless prose. Drafted with Mistral `mistral-large-latest` over four
  text chunks plus one consolidation round, then checked line-by-line against
  `text.txt` and corrected to the paper's own notation (⌢, ≺, ≻, era subscripts).

## Figures

Five figures, extracted as full-page vector SVGs with `pdftocairo -svg` (one per
figure's page) and cropped by rewriting the root `width`/`height`/`viewBox` to the
figure region (top-left origin, points; the page content itself is unmodified vector).
Caption text is excluded from the crops; captions are listed here.

| Figure | Page | Caption (excluded from crop) | Crop box (x, y, w, h) |
|--------|------|------------------------------|------------------------|
| 1 | 3 | Fig. 1. Invariants preserved by the Synod algorithm | 42, 71, 528, 123 |
| 2 | 5 | Fig. 2. Invariants preserved by the Paxos algorithm | 42, 71, 528, 166 |
| 3 | 6 | Fig. 3. Message flow during a reconfiguration. | 308, 48, 258, 365 |
| 4 | 7 | Fig. 4. Message flow for Dynamic Paxos with α = 2. | 308, 48, 258, 365 |
| 5 | 8 | Fig. 5. Message flow for Stoppable Paxos. | 44, 48, 260, 365 |

PNG renders (`fig-<n>.png`): headless Google Chrome (`--headless=new
--force-device-scale-factor=4 --window-size=<intrinsic px> --hide-scrollbars
--default-background-color=FFFFFFFF`, screenshot of the SVG file), i.e. 96 dpi natural
size at 4× device scale ≈ 5.33 px/pt. All PNGs are well under 2 MB. Render fidelity
was verified by matching ink-density band profiles against `pdftoppm` renders of the
same page regions.

`fig-<n>.description.md`: standalone descriptions produced with Mistral
`mistral-medium-latest` vision (chat completions, image passed as a base64 `data:`
URI `image_url`), prompted to describe only what is visible — nodes, lifelines,
arrows with direction and endpoints, regions/bands, verbatim labels, ordering — with
no source context. These are the reference set for comparison against identically
produced descriptions of original redrawings. Known vision quirks to expect on both
sides of any comparison: the quorum-intersection symbol ⌢ is misread as ⊂ or →, and
subscripts are transcribed informally.

## sha256 inventory

```
d8e34d32dfdd26343dcd8fb4aadc418155ac4796deb1e553fbd17e3336c8d834  figures/fig-1-page-3.svg
2e5eee1f2e11ae327f16f522db9cd1c87d0c978151bb81699c93cbdfd34e1058  figures/fig-1.png
c1936ca5e17bb0dc26cbe5e89503bcd5091f5b31922ff207a4e4b5f891d804f6  figures/fig-1.description.md
f8df5e670dd6f61d43bd9ab7e586a851fcf351f33aac5bed97960d517d2264c1  figures/fig-2-page-5.svg
87ac2f61ff2782735031ac004f7ec7fac3b79704b0b169e80c153806ea583f9b  figures/fig-2.png
d48efe916baf249cf6c0ecec7e1e165036d1465146548b14d4af987361d332ff  figures/fig-2.description.md
68f43f4f30438bb90345a74e4178a5d8b903283b0203ebf74e0584f33a96df08  figures/fig-3-page-6.svg
32cc592398d0f6788d2079a747091ba5da08e4ea7d3470a4319b6a6570c99680  figures/fig-3.png
c4440e8b08d9f88f0a7cf137a2dc4b2dabb3a26ef63ee2103b7be6125fbbc88e  figures/fig-3.description.md
06ff4146d249d1131344ca66cd5ea46c88a7097fd96c0bed8ea685ff021f281b  figures/fig-4-page-7.svg
3ab11397b07fbbf6d37b515801cf26586a01acfcd88d598a0c4c8c3343ae2bb7  figures/fig-4.png
e38d228c6e56415d60f92cb959ccca96dd54d527bea539ecce5e1601a73a1a9b  figures/fig-4.description.md
d6a75491ea137a9a522712bc1fb6d39d051d9cd6ea40677fc65521a667950b2a  figures/fig-5-page-8.svg
eab8118e6e47f0f69d11f7466fd5a5eededd8e18cc2763ce12c28e424f536467  figures/fig-5.png
3ccd8cf7f39bd1cf4b9a11252d81b58fd209254bbef968e98557ca09c53ed7a5  figures/fig-5.description.md
3f5ffd4ecdf89f4dd8887a74b9f91d52914fcbe27183857cf68c6ff7cfd7f8ff  paxos-reconf-latest.pdf
968496c3a509d84a43f3305517a871d7a34ffc3167399156f7872f8948065da4  summary.md
b43ba62be656af15228677b273fd09eb119e85af485f1f0fe83a26c62d73cedb  text.txt
```
