# uVRR citation graph

This page is a read-only view of the graph data in `data.js`. To compare it
with the paper's Zotero/Better BibTeX bibliography, choose one or more
collection export files using **Read Zotero collection exports**. The filename
is used as the collection label in the dropdown; choose the paper's existing
`paper/papers/references.bib` for the current paper collection. The page reads
exported records in the browser and never edits or uploads the selected files.
It does not connect to Zotero's database or read Zotero notes, attachments, or
fields that Better BibTeX did not export. Use **Find a loaded reference** to
search citation keys, titles, authors, and abstracts; click a result to read
its exported abstract in the details panel.

After loading the files, teal outlines mark graph nodes whose explicit
Better BibTeX key appears in the selected export. Switch exports with the
collection dropdown; **Show only graph works in the selected collection**
hides unmatched nodes and edges. The `uvrrpaper` node stays as context so its
outgoing references remain visible. The checkbox starts unchecked so the
full graph remains available.

## Abstract refresh

Better BibTeX is the source of truth for the loaded abstracts. When an export
has a gap, prefer the primary paper's published abstract. For a self-published
post, derive a short abstract from the front-page snippet; if no snippet exists,
take a first-sentence summary from the opening text and keep it descriptive
rather than evaluative. Keep the derived text in Zotero's Abstract field and let
the export refresh, rather than editing abstracts inside the graph. The current
`references.bib` has an abstract for every exported record.

The key crosswalk is explicit in `data.js`. If a citekey changes in Zotero,
update that mapping to match the next export. The chart does not infer identity
from titles or abstracts and does not treat an unkeyed graph node as a match.
Each file is a one-time browser read: reselect the updated exports after Better
BibTeX refreshes them. This is an export-based sync, not a live Zotero
connection. Give each collection export a distinct filename so it is easy to
identify in the dropdown. For example, load `references.bib` alongside a
separate `software-security.bib`.

Edges are directed from source to target; edge labels describe the recorded
citation or relationship. The bibliography filter checks node membership in
the selected `.bib` export. It does not parse the LaTeX source to decide
whether an entry is actually cited in the paper body.

## Keeping Zotero projects separate

Use one Zotero library with project collections and topic tags. A reference
can belong to more than one collection without creating duplicate items.
For the uVRR paper, keep its Better BibTeX auto-export targeted at the current
paper bibliography and preserve that path because the TeX source uses it.
For the software-security research, use a separate collection and a distinct
export file outside this paper's bibliography path. Load both exports in the
graph and select a collection there. Do not point the paper's TeX command at a
combined library export when you want a paper-specific bibliography.

For your WordPress posts, save the post as a Web Page item through the Zotero
Connector and keep the snapshot attachment if you want a local captured copy.
Snapshots preserve the page as it looked when saved; they do not track later
edits. Save a new snapshot/version when a post changes materially. Zotero's
stored-file syncing is separate from metadata syncing, so confirm attachment
file sync is enabled if you need snapshots on other devices. Keep private
drafts or notes out of any shared group or published graph.
