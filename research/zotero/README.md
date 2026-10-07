# Zotero tooling

`zotero_api.py` is the one place in this repository that talks to Zotero. It
exists so that no shell reaches for a hand-written `curl` again: every call
the local API actually supports is a named command with a help string, and the
sharp edges are encoded once rather than rediscovered.

Zotero is the database of record for the bibliography. The repository holds one
Better BibTeX auto-export snapshot and nothing else, and the snapshot is
replaced wholesale from the export, never hand-edited.

## Two rules

**The database is never written.** Zotero is driven only through its HTTP API.
`~/Zotero/zotero.sqlite` is opened `mode=ro`, and only to read the local server
id, which the API does not expose over any endpoint.

**Credentials are never committed and never printed.** The API key comes from
the `ZOTERO_API_KEY` entry in `.env` at the repository root, or from the
environment when it is already set there. `.gitignore` excludes `.env`, so the
key cannot reach a commit. Error messages are redacted before printing.

## Where the snapshot comes from

Zotero syncs to the `main` checkout only, forever, because `main` is the single
place the export lands. Any other worktree updates the bibliography by copying
the snapshot out of `main` with `cp`, referencing that copy, and committing it.
Worktrees are transient; the export location is not. This is the eventual
consistency half of the model: one master, replicas brought up to date by an
explicit copy.

## Commands

    zotero_api.py ping                       is the local API up
    zotero_api.py count                      how many items the library holds
    zotero_api.py search QUERY               search, optionally by citation key
    zotero_api.py attachments                list attachment items
    zotero_api.py show KEY [--format bibtex] one item, as JSON or BibTeX
    zotero_api.py children KEY               an item's children, such as attachments
    zotero_api.py collections                list collections
    zotero_api.py tags                       list tags

Writes, each guarded by `--dry-run` where a mistake would show in the library:

    zotero_api.py create-item --json-file F  create items from JSON
    zotero_api.py patch-item KEY --set F=V  set fields on an item
    zotero_api.py attach-parent KEY PDF     attach a PDF to a parent item
    zotero_api.py upload KEY PDF            upload bytes for an attachment item
    zotero_api.py trash KEY                 move an item to the Zotero trash

Reads need no credential. Writes need `Zotero-Server-ID` and `Zotero-API-Key`
alongside `X-Zotero-API-Version: 3`, which the module adds for you.

## Sharp edges, encoded

- The base URL is `http://localhost:23119/api/`, but the library's items live
  under `users/0/`. Bare `api/items` is a 404.
- `POST /users/0/items` takes a **bare JSON array**, not `{"items": [...]}`, and
  answers `400 Uploaded data must be a JSON array` otherwise.
- `PUT /users/0/items/<key>` replaces the whole object and needs the current
  `version`. Sending a partial object answers `400 Unknown item type
  'undefined'`, which is why `patch-item` reads the item, edits the fields, and
  sends it whole with `If-Unmodified-Since-Version`.
- Attaching a PDF is two steps: create the `imported_file` attachment item with
  its `parentItem`, then `POST` the bytes to that item's `/file` endpoint. The
  upload is a three-phase protocol, and phases 1 and 3 carry `If-None-Match: *`
  or they are refused with `400 If-Match/If-None-Match header not provided`.
- `GET /users/0/items/<key>/file` answers `302` to a `file://` URL. Redirects are
  never followed.
- A write with no key is `401` with an empty body; with a wrong
  `Zotero-Server-ID` it is `412`, and with none it is `428`.

There is no endpoint that reports whether a key is valid: `/keys/current` is a
`404` on this build, under `/api/` and at the server root alike. A key is
verified by a write succeeding.

## Authorising the first time

The key is a per-application consent, so the repository records the flow rather
than the secret. Zotero Settings → Advanced → "Allow other applications on this
computer to communicate with Zotero", then a `POST /api/local/authorize` with
`{"appName": "<name>"}`, which prompts the person and answers
`{"key": ..., "remember": ...}`. Put the key in `.env` as `ZOTERO_API_KEY`.
Asking for `remember: true` means consenting once rather than per run.
