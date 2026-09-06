import base64
import json
import os
import pathlib
import sys
import time
import urllib.request
import urllib.error

IN_DIR = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("./papers")
OUT_DIR = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else pathlib.Path("./ocr")

API = "https://api.mistral.ai/v1"
MODEL = "mistral-ocr-latest"


def load_key() -> str:
    key = os.environ.get("MISTRAL_API_KEY", "").strip()
    if not key:
        env_file = pathlib.Path.cwd() / ".env"
        if env_file.exists():
            env = {}
            for line in env_file.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, v = line.split("=", 1)
                    env[k.strip()] = v.strip().strip('"').strip("'")
            key = env.get("MISTRAL_API_KEY", "").strip()
    if not key:
        raise SystemExit("MISTRAL_API_KEY not set (env var or .env in current dir)")
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
    # 1) upload file with purpose=ocr (multipart, stdlib)
    boundary = "----ocredge7f3a"
    fname = pdf.name
    parts = []
    parts.append(
        f'--{boundary}\r\nContent-Disposition: form-data; name="purpose"\r\n\r\nocr\r\n'.encode()
    )
    parts.append(
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="file"; '
            f'filename="{fname}"\r\nContent-Type: application/pdf\r\n\r\n'
        ).encode()
        + pdf.read_bytes()
        + b"\r\n"
    )
    parts.append(f"--{boundary}--\r\n".encode())
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
    # 2) signed url
    req = urllib.request.Request(
        f"{API}/files/{fid}/url",
        headers={"Authorization": f"Bearer {key}"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        url = json.loads(r.read())["url"]
    # 3) ocr
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
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    pdfs = sorted(IN_DIR.glob("*.pdf"))
    if not pdfs:
        raise SystemExit(f"no PDFs in {IN_DIR}")
    for pdf in pdfs:
        out = OUT_DIR / (pdf.stem + ".md")
        if out.exists() and out.stat().st_size > 100:
            print(f"skip {pdf.name} (already OCR'd)")
            continue
        t0 = time.time()
        try:
            try:
                resp = ocr_via_data_uri(key, pdf)
                mode = "data-uri"
            except urllib.error.HTTPError as e:
                if e.code in (400, 422):
                    resp = ocr_via_upload(key, pdf)
                    mode = "upload"
                else:
                    raise
            pages = resp.get("pages", [])
            md = "\n\n---\n\n".join(p.get("markdown", "") for p in pages)
            out.write_text(md, encoding="utf-8")
            print(
                f"OK  {pdf.name}  [{mode}]  {len(pages)} pages  "
                f"{len(md)} chars  {time.time()-t0:.0f}s  -> {out}"
            )
        except Exception as e:
            print(f"ERR {pdf.name}: {type(e).__name__}: {e}  ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
