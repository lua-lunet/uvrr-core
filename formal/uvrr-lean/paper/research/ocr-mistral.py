"""Mistral OCR pass over the paper/research corpus.

Follows the repository's established OCR pattern (paper/build.sh --ocr,
research/ocr_run.py): the key is read silently from the repository .env, never
printed, echoed, or written anywhere. Model is mistral-ocr-latest, the same
model the paper's own build gate uses.

Usage: python3 ocr-mistral.py          (from formal/uvrr-lean/paper/research)
Writes ocr-mistral/<pdf-stem>.md, one "---"-separated markdown page stream.
Prints one JSON line per paper: {"id", "mode", "pages", "chars", "seconds"}.
Skips PDFs whose output already exists and is non-trivial.
"""
import base64
import json
import pathlib
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent.parent  # repo root
HERE = pathlib.Path(__file__).resolve().parent
PAPERS = HERE / "pdf"
OUT = HERE / "ocr-mistral"

API = "https://api.mistral.ai/v1"
MODEL = "mistral-ocr-latest"


def load_key() -> str:
    env = {}
    for line in (ROOT / ".env").read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            env[k.strip()] = v.strip().strip('"').strip("'")
    key = env.get("MISTRAL_API_KEY", "").strip()
    if not key:
        raise SystemExit("MISTRAL_API_KEY not found in .env")
    return key


def http_json(url: str, key: str, body: dict, timeout: int = 600) -> dict:
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())


def ocr_via_upload(key: str, pdf: pathlib.Path) -> dict:
    boundary = "----ocredge7f3a"
    parts = [
        f'--{boundary}\r\nContent-Disposition: form-data; name="purpose"\r\n\r\nocr\r\n'.encode(),
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="file"; '
            f'filename="{pdf.name}"\r\nContent-Type: application/pdf\r\n\r\n'
        ).encode()
        + pdf.read_bytes()
        + b"\r\n",
        f"--{boundary}--\r\n".encode(),
    ]
    req = urllib.request.Request(
        f"{API}/files",
        data=b"".join(parts),
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Authorization": f"Bearer {key}",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=600) as r:
        fid = json.loads(r.read())["id"]
    req = urllib.request.Request(
        f"{API}/files/{fid}/url", headers={"Authorization": f"Bearer {key}"}
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        url = json.loads(r.read())["url"]
    return http_json(
        f"{API}/ocr",
        key,
        {"model": MODEL, "document": {"type": "document_url", "document_url": url}},
    )


def ocr_via_data_uri(key: str, pdf: pathlib.Path) -> dict:
    b64 = base64.b64encode(pdf.read_bytes()).decode()
    return http_json(
        f"{API}/ocr",
        key,
        {
            "model": MODEL,
            "document": {
                "type": "document_url",
                "document_url": f"data:application/pdf;base64,{b64}",
            },
        },
    )


def main() -> None:
    key = load_key()
    OUT.mkdir(parents=True, exist_ok=True)
    pdfs = sorted(PAPERS.glob("*.pdf"))
    if not pdfs:
        raise SystemExit(f"no PDFs in {PAPERS}")
    for pdf in pdfs:
        out = OUT / (pdf.stem + ".md")
        if out.exists() and out.stat().st_size > 100:
            print(json.dumps({"id": pdf.stem, "mode": "skip", "reason": "exists"}))
            continue
        t0 = time.time()
        try:
            try:
                resp = ocr_via_data_uri(key, pdf)
                mode = "data-uri"
            except urllib.error.HTTPError as e:
                if e.code in (400, 413, 422):
                    resp = ocr_via_upload(key, pdf)
                    mode = "upload"
                else:
                    raise
            pages = resp.get("pages", [])
            md = "\n\n---\n\n".join(p.get("markdown", "") for p in pages)
            out.write_text(md, encoding="utf-8")
            print(
                json.dumps(
                    {
                        "id": pdf.stem,
                        "mode": mode,
                        "pages": len(pages),
                        "chars": len(md),
                        "seconds": round(time.time() - t0),
                    }
                ),
                flush=True,
            )
        except Exception as e:
            print(
                json.dumps(
                    {
                        "id": pdf.stem,
                        "error": f"{type(e).__name__}: {e}",
                        "seconds": round(time.time() - t0),
                    }
                ),
                flush=True,
            )


if __name__ == "__main__":
    sys.exit(main())
