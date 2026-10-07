# Citation graph remote search

The graph currently reads selected Better BibTeX exports locally. A remote
search should add candidates for reading without changing Zotero, the export,
or the graph's static relationship data.

1. Add a search box whose request is sent only after an explicit search action.
2. Query a configured research endpoint with the search text and a bounded
   result count.
3. Render title, authors, year, venue, abstract, DOI, and canonical URL as
   untrusted result data using DOM text nodes.
4. Open a result in the existing detail panel. Show that it is a remote result
   rather than a reference loaded from the selected export.
5. Keep importing into Zotero as an explicit, separate action outside the
   graph. The graph must never write to Zotero or mutate a BibTeX export.
6. Cache only a user-selected result set in browser storage, with a visible
   clear control. Do not send bibliography contents, Zotero notes, or local
   attachment paths to the endpoint.

Acceptance checks: an empty query makes no request; a request failure leaves
loaded collection data usable; HTML in a remote title or abstract is rendered
as text; selecting a remote result does not alter the currently selected
collection or graph filter.
