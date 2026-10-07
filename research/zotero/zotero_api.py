#!/usr/bin/env python3
"""Command-line access to the Zotero local API.

Zotero is the database of record for the bibliography. This module is the one
place that talks to it, so that no shell reaches for a hand-written curl again.

Two rules govern everything here.

The database is never edited. Zotero is driven only through its HTTP API.
The sqlite file is opened read-only, and only to read the local server id,
which the API does not expose over HTTP.

Credentials are never committed and never printed. The API key comes from
the `ZOTERO_API_KEY` entry in `.env` at the repository root, or from the
environment if it is already set there. `.gitignore` excludes `.env`.

Reads need no credential. Writes need `Zotero-Server-ID` and `Zotero-API-Key`
alongside `X-Zotero-API-Version: 3`, and the server id is read from the
database because no endpoint reports it.

Examples:
  zotero_api.py ping
  zotero_api.py count
  zotero_api.py search turner --field citationKey
  zotero_api.py show 8ABCD234 --format bibtex
  zotero_api.py create-item --json-file new-item.json --dry-run
  zotero_api.py attach-parent 8ABCD234 ./paper.pdf --dry-run

Run `zotero_api.py --help` for the command list, or
`zotero_api.py <command> --help` for one command.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import sqlite3
import sys
import urllib.error
import urllib.parse
import urllib.request

BASE = "http://localhost:23119/api/"
SERVER_ID_SQL = "select value from settings where setting='localAPI' and key='serverID'"
DEFAULT_DB = pathlib.Path.home() / "Zotero" / "zotero.sqlite"
REPO = pathlib.Path(__file__).resolve().parent.parent.parent
VERSION_HEADER = "X-Zotero-API-Version"


class ZoteroError(RuntimeError):
    pass


def load_key(env_path: pathlib.Path | None = None) -> str | None:
    """The API key from the environment, else from `.env`. Never echoed."""
    key = os.environ.get("ZOTERO_API_KEY")
    if key:
        return key.strip()
    path = env_path or REPO / ".env"
    if not path.exists():
        return None
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        name, _, value = line.partition("=")
        if name.strip() == "ZOTERO_API_KEY":
            return value.strip().strip("'\"")
    return None


def server_id(db: pathlib.Path = DEFAULT_DB) -> str:
    """The local server id, read-only. The API has no endpoint for this."""
    if not db.exists():
        raise ZoteroError(f"no Zotero database at {db}")
    uri = f"file:{urllib.parse.quote(str(db))}?mode=ro"
    con = sqlite3.connect(uri, uri=True)
    try:
        row = con.execute(SERVER_ID_SQL).fetchone()
    except sqlite3.Error as exc:
        raise ZoteroError(f"reading the server id failed: {exc}") from exc
    finally:
        con.close()
    if not row:
        raise ZoteroError("no localAPI serverID in the database settings")
    return row[0]


class Zotero:
    def __init__(self, base: str = BASE, key: str | None = None, sid: str | None = None,
                 db: pathlib.Path = DEFAULT_DB, timeout: float = 30.0) -> None:
        self.base = base.rstrip("/") + "/"
        self.key = key
        self.sid = sid
        self.db = db
        self.timeout = timeout

    def _url(self, path: str, params: dict | None = None) -> str:
        url = urllib.parse.urljoin(self.base, path.lstrip("/"))
        if params:
            clean = {k: v for k, v in params.items() if v is not None}
            if clean:
                url = f"{url}?{urllib.parse.urlencode(clean)}"
        return url

    def url_for(self, path: str, params: dict | None = None) -> str:
        return self._url(path, params)

    def request(self, method: str, path: str, *, params: dict | None = None, body: bytes | None = None,
                headers: dict | None = None, write: bool = False, follow: bool = True) -> tuple[int, bytes, dict]:
        hdrs = {VERSION_HEADER: "3"}
        if write:
            if not self.key:
                raise ZoteroError("a write needs ZOTERO_API_KEY in .env or the environment")
            if not self.sid:
                self.sid = server_id(self.db)
            hdrs["Zotero-API-Key"] = self.key
            hdrs["Zotero-Server-ID"] = self.sid
        hdrs.update(headers or {})

        url = self._url(path, params)
        req = urllib.request.Request(url, data=body, method=method, headers=hdrs)
        opener = urllib.request.build_opener() if follow else urllib.request.build_opener(NoRedirect)
        try:
            with opener.open(req, timeout=self.timeout) as resp:
                return resp.status, resp.read(), dict(resp.headers)
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace").strip()
            raise ZoteroError(f"{method} {url} -> {exc.code} {exc.reason}: {detail[:400]}") from exc
        except urllib.error.URLError as exc:
            raise ZoteroError(f"{method} {url} -> {exc.reason}. Is Zotero running?") from exc

    def json_request(self, method: str, path: str, *, params: dict | None = None, payload=None,
                     write: bool = False, headers: dict | None = None):
        body = None
        hdrs = dict(headers or {})
        if payload is not None:
            body = json.dumps(payload).encode("utf-8")
            hdrs["Content-Type"] = "application/json"
        _, raw, _ = self.request(method, path, params=params, body=body, headers=hdrs, write=write)
        if not raw:
            return None
        try:
            return json.loads(raw)
        except json.JSONDecodeError as exc:
            raise ZoteroError(f"response was not JSON: {raw[:200]!r}") from exc

    def get(self, path: str, **kw):
        return self.json_request("GET", path, **kw)

    def paged(self, path: str, limit: int = 100, **params) -> list:
        out, start = [], 0
        while True:
            batch = self.get(path, params={"limit": limit, "start": start, **params}) or []
            out.extend(batch)
            if len(batch) < limit:
                return out
            start += limit


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """`GET /file` answers 302 to a file:// URL. Following it is never wanted."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def emit(value, as_json: bool) -> None:
    if as_json:
        print(json.dumps(value, indent=2, sort_keys=True))
    else:
        print(value)


def require(value, what: str):
    if value is None:
        raise ZoteroError(f"{what} returned an empty body")
    return value


def redact(text: str, key: str | None) -> str:
    return text.replace(key, "<ZOTERO_API_KEY>") if key else text


def cmd_ping(z: Zotero, args) -> int:
    status, _, _ = z.request("GET", "")
    print(f"Zotero local API is up: {z.base} -> {status}")
    return 0


def cmd_count(z: Zotero, args) -> int:
    _, _, headers = z.request("GET", "users/0/items", params={"limit": 1})
    print(f"items in library: {headers.get('Total-Results', 'unknown')}")
    return 0


def cmd_attachments(z: Zotero, args) -> int:
    items = z.paged("users/0/items", itemType="attachment")
    if args.json:
        emit(items, True)
        return 0
    for item in items:
        data = item["data"]
        print(f"{item['key']}  {data.get('title', '(no title)')}")
    print(f"{len(items)} attachment(s)")
    return 0


def cmd_search(z: Zotero, args) -> int:
    items = z.paged("users/0/items", q=args.query)
    if args.field == "citationKey":
        items = [i for i in items if i["data"].get("citationKey") == args.query]
    if args.json:
        emit(items, True)
        return 0
    for item in items:
        data = item["data"]
        label = data.get("citationKey") or item["key"]
        print(f"{label}  {data.get('itemType', '?'):<12} {data.get('title', data.get('shortTitle', ''))[:80]}")
    print(f"{len(items)} match(es)")
    return 0


def cmd_show(z: Zotero, args) -> int:
    if args.format == "bibtex":
        _, raw, _ = z.request("GET", f"users/0/items/{args.key}", params={"format": "bibtex"})
        print(raw.decode("utf-8", "replace"))
        return 0
    item = z.get(f"users/0/items/{args.key}")
    emit(item, True)
    return 0


def cmd_children(z: Zotero, args) -> int:
    emit(z.paged(f"users/0/items/{args.key}/children"), True)
    return 0


def cmd_collections(z: Zotero, args) -> int:
    emit(z.paged("users/0/collections"), True)
    return 0


def cmd_tags(z: Zotero, args) -> int:
    emit(z.paged("users/0/tags"), True)
    return 0


def cmd_create_item(z: Zotero, args) -> int:
    payload = json.loads(args.json_file.read_text(encoding="utf-8"))
    items = payload if isinstance(payload, list) else [payload]
    for item in items:
        if "itemType" not in item:
            raise ZoteroError("every item needs an itemType; a partial item is rejected with 400")
    if args.dry_run:
        print(json.dumps(items, indent=2))
        print(f"dry run: would POST {len(items)} item(s) to users/0/items")
        return 0
    result = z.json_request("POST", "users/0/items", payload=items, write=True)
    emit(result, True)
    return 0


def cmd_patch_item(z: Zotero, args) -> int:
    item = require(z.get(f"users/0/items/{args.key}"), f"item {args.key}")
    version = item["version"]
    for pair in args.set:
        field, _, value = pair.partition("=")
        item["data"][field] = value
    payload = {"data": item["data"], "version": version}
    if args.dry_run:
        print(json.dumps(payload, indent=2))
        print(f"dry run: would PUT users/0/items/{args.key} at version {version}")
        return 0
    emit(z.json_request("PUT", f"users/0/items/{args.key}", payload=payload, write=True,
                        headers={"If-Unmodified-Since-Version": str(version)}), True)
    return 0


def cmd_attach_parent(z: Zotero, args) -> int:
    payload = {
        "itemType": "attachment",
        "linkMode": "imported_file",
        "title": pathlib.Path(args.pdf).name,
        "parentItem": args.parent,
        "contentType": "application/pdf",
    }
    if args.dry_run:
        print(json.dumps([payload], indent=2))
        print(f"dry run: would create the attachment item, then upload {args.pdf} to its /file")
        return 0
    created = require(z.json_request("POST", "users/0/items", payload=[payload], write=True), "item creation")
    successful = created.get("successful") if isinstance(created, dict) else None
    key = successful["0"]["key"] if successful else created[0]["key"]
    print(f"attachment item {key}")
    return cmd_upload(z, argparse.Namespace(key=key, pdf=args.pdf, dry_run=args.dry_run))


def cmd_upload(z: Zotero, args) -> int:
    import hashlib

    path = pathlib.Path(args.pdf)
    if not path.exists():
        raise ZoteroError(f"no such file: {path}")
    data = path.read_bytes()
    digest = hashlib.md5(data).hexdigest()
    mtime = str(int(path.stat().st_mtime * 1000))
    url = z.url_for(f"users/0/items/{args.key}/file")

    def phase(fields: dict, body: bytes | None, headers: dict) -> tuple:
        hdrs = {"Zotero-API-Key": z.key or "", VERSION_HEADER: "3", **headers}
        req = urllib.request.Request(url, data=body, method="POST", headers=hdrs)
        opener = urllib.request.build_opener()
        try:
            with opener.open(req, timeout=z.timeout) as resp:
                return resp.status, resp.headers
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace").strip()
            raise ZoteroError(f"upload phase -> {exc.code}: {detail[:300]}") from exc

    if not z.sid:
        z.sid = server_id(z.db)
    # phase 1 and 3 are form-encoded with a precondition; phase 2 is the bytes alone
    phase({"md5": digest, "mtime": mtime, "filename": path.name, "filesize": str(len(data)), "params": "1"},
          urllib.parse.urlencode({"md5": digest, "mtime": mtime, "filename": path.name,
                                  "filesize": str(len(data)), "params": "1"}).encode(),
          {"Content-Type": "application/x-www-form-urlencoded", "If-None-Match": "*"})
    boundary = "----zoteroapi"
    multipart = (
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{path.name}\"\r\n"
        f"Content-Type: application/pdf\r\n\r\n"
    ).encode() + data + f"\r\n--{boundary}--\r\n".encode()
    status, _ = phase({}, multipart, {"Content-Type": f"multipart/form-data; boundary={boundary}"})
    phase({"upload": z.key or ""}, urllib.parse.urlencode({"upload": z.key or ""}).encode(),
          {"Content-Type": "application/x-www-form-urlencoded", "If-None-Match": "*"})
    print(f"uploaded {path.name} ({len(data)} bytes, md5 {digest[:12]}) to {args.key}: {status}")
    return 0


def cmd_trash(z: Zotero, args) -> int:
    if args.dry_run:
        print(f"dry run: would DELETE users/0/items/{args.key} (Zotero moves it to the trash)")
        return 0
    z.request("DELETE", f"users/0/items/{args.key}", write=True)
    print(f"trashed {args.key}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="zotero_api.py",
        description="Command-line access to the Zotero local API. Zotero is the database of record.",
        epilog=(
            "The sqlite database is never written: it is opened read-only to read the local\n"
            "server id, which the API does not expose. The API key comes from ZOTERO_API_KEY\n"
            "in .env at the repository root and is never printed.\n\n"
            "Writes are guarded by --dry-run where a mistake would be visible in the library.\n\n"
            "examples:\n"
            "  zotero_api.py ping\n"
            "  zotero_api.py count\n"
            "  zotero_api.py search turner --field citationKey\n"
            "  zotero_api.py show 8ABCD234 --format bibtex\n"
            "  zotero_api.py children 8ABCD234\n"
            "  zotero_api.py create-item --json-file item.json --dry-run\n"
            "  zotero_api.py attach-parent 8ABCD234 ./paper.pdf --dry-run\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--base", default=BASE, help=f"local API base (default: {BASE})")
    parser.add_argument("--db", type=pathlib.Path, default=DEFAULT_DB,
                        help=f"Zotero database, opened read-only (default: {DEFAULT_DB})")
    sub = parser.add_subparsers(dest="command", metavar="<command>")

    def add(name, func, help_text):
        p = sub.add_parser(name, help=help_text, description=help_text)
        p.set_defaults(func=func)
        return p

    add("ping", cmd_ping, "Check the local API is reachable.")

    p = add("count", cmd_count, "Report the number of items in the library.")
    p.set_defaults(json=False)

    p = add("attachments", cmd_attachments, "List attachment items.")
    p.add_argument("--json", action="store_true", help="emit the raw item objects")

    p = add("search", cmd_search, "Search the library.")
    p.add_argument("query", help="text to search for")
    p.add_argument("--field", choices=["any", "citationKey"], default="any",
                   help="match any field, or exactly one citation key")
    p.add_argument("--json", action="store_true", help="emit the raw item objects")

    p = add("show", cmd_show, "Show one item, optionally as BibTeX.")
    p.add_argument("key", help="item key")
    p.add_argument("--format", choices=["json", "bibtex"], default="json")

    p = add("children", cmd_children, "List an item's child items, such as its attachments.")
    p.add_argument("key", help="parent item key")

    add("collections", cmd_collections, "List collections.")
    add("tags", cmd_tags, "List tags.")

    p = add("create-item", cmd_create_item, "Create one or more items. WRITE.")
    p.add_argument("--json-file", type=pathlib.Path, required=True,
                   help="a JSON object, or an array of them, each with an itemType")
    p.add_argument("--dry-run", action="store_true", help="print the request without sending it")

    p = add("patch-item", cmd_patch_item, "Set fields on an item. WRITE. Sends the whole object.")
    p.add_argument("key", help="item key")
    p.add_argument("--set", action="append", metavar="FIELD=VALUE", required=True,
                   help="a field to set; repeatable")
    p.add_argument("--dry-run", action="store_true", help="print the request without sending it")

    p = add("attach-parent", cmd_attach_parent, "Attach a PDF to a parent item. WRITE.")
    p.add_argument("parent", help="parent item key")
    p.add_argument("pdf", help="path to the PDF")
    p.add_argument("--dry-run", action="store_true", help="print the request without sending it")

    p = add("upload", cmd_upload, "Upload the bytes for an existing attachment item. WRITE.")
    p.add_argument("key", help="attachment item key")
    p.add_argument("pdf", help="path to the PDF")

    p = add("trash", cmd_trash, "Move an item to the Zotero trash. WRITE.")
    p.add_argument("key", help="item key")
    p.add_argument("--dry-run", action="store_true", help="print the request without sending it")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if not args.command:
        parser.print_help()
        return 2
    z = Zotero(base=args.base, key=load_key(), db=args.db)
    try:
        return args.func(z, args)
    except ZoteroError as exc:
        print(f"error: {redact(str(exc), z.key)}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
