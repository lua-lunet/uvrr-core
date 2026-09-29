# uVRR citation graph

This page is a read-only view of the graph data in `data.js`. To compare it
with the paper's Zotero/Better BibTeX bibliography, choose the existing
`paper/papers/references.bib` file with **Read a BibTeX export**. The page
reads entry keys in the browser and never edits or uploads the selected file.
It does not connect to Zotero's database or read Zotero notes, abstracts,
attachments, or other item fields.

After loading the file, teal outlines mark graph nodes whose explicit
Better BibTeX key appears in the export. Click a node to see its matched key
or the key the graph expects. **Show only works in this bibliography** hides
unmatched nodes and edges; the `uvrrpaper` node stays as context so its
outgoing references remain visible. The checkbox starts unchecked so the
full graph remains available.

The key crosswalk is explicit in `data.js`. If a citekey changes in Zotero,
update that mapping to match the next export. The chart does not infer identity
from titles or abstracts and does not treat an unkeyed graph node as a match.
The selected file is a one-time browser read: reselect it after Better BibTeX
updates the export to refresh the filter.

Edges are directed from source to target; edge labels describe the recorded
citation or relationship. The bibliography filter checks node membership in
the selected `.bib` export. It does not parse the LaTeX source to decide
whether an entry is actually cited in the paper body.

## Keeping Zotero projects separate

Use one Zotero library with project collections and topic tags. A reference
can belong to more than one collection without creating duplicate items.
For the uVRR paper, keep its Better BibTeX auto-export targeted at the current
paper bibliography and preserve that path because the TeX source uses it.
For the software-security research, use a separate collection and, if needed,
a separate export file outside this paper's bibliography path. Do not point
the graph at a combined library export when you want a paper-specific view.

For your WordPress posts, save the post as a Web Page item through the Zotero
Connector and keep the snapshot attachment if you want a local captured copy.
Snapshots preserve the page as it looked when saved; they do not track later
edits. Save a new snapshot/version when a post changes materially. Zotero's
stored-file syncing is separate from metadata syncing, so confirm attachment
file sync is enabled if you need snapshots on other devices. Keep private
drafts or notes out of any shared group or published graph.
