r"""Citation checker for paper/paper.tex using Jev via OpenCode Zen.

Follows https://jevmanual.com/tutorials/citation-checking/ : for each
`\cite` use in the paper, retrieve verbatim passages from the scraped
primary source, then ask jev-1.13 whether the passage supports, contradicts,
or leaves the claim unresolved, and apply a confidence review gate.

Standard library plus curl. Fixture mode (default) exercises the pipeline
with deterministic sample answers and makes no network request; --live
makes one billed Jev call per citation use against
https://opencode.ai/zen/v1/systemone using OPENCODE_API_KEY.

Usage:
  python3 scripts/check-citations.py                # fixture plumbing check
  python3 scripts/check-citations.py --live         # OPENCODE_API_KEY, jev-1.13
  python3 scripts/check-citations.py --live --model jev-1.13-free
  python3 scripts/check-citations.py --live --only 65,73   # tex line numbers
  python3 scripts/check-citations.py --live --skip_cached  # resume

Each live verdict is appended to .tmp/citation-verdicts-v2.jsonl, the resume
cache; delete it to re-check every site from scratch. Verdicts are evidence
for review, not truth: the gate routes low confidence and insufficiency to a
human, and never contradicts automatically.
"""
import argparse
import json
import math
import os
import re
import socket
import subprocess
import sys
import urllib.error
from copy import deepcopy

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
PAPER = os.path.join(ROOT, "paper", "paper.tex")
VERDICTS = os.path.join(ROOT, ".tmp", "citation-verdicts-v2.jsonl")
MODEL = "jev-1.13"
HTTP_TIMEOUT = 30

# Scraped primary sources: key -> verbatim OCR or fetched text of the cited
# work. The `artifact` key cites this repository itself; the Lean proof
# artifact's README is the quoted source.
SOURCES = {
    "vr": "research/agent366/md/vr-1988-oki-liskov.md",
    "vrr": "research/agent366/md/vr-revisited-2012.md",
    "diskless": "research/agent366/md/diskless-tr16.md",
    "par": "research/agent366/md/fast18-alagappan-par.md",
    "pms": "research/agent366/md/paxos-made-simple-2001.md",
    "diskpaxos": "research/agent366/md/disk-paxos-2003.md",
    "allfs": "research/agent366/md/osdi14-pillai-allfs.md",
    "optimistic-crash": "research/agent366/md/sosp13-optimistic-crash.md",
    "turner": "research/literature/turner/text.txt",
    "motivation": "research/agent366/md/blog-uvrr-2026.md",
    "corfu": "research/agent366/md/corfu-tocs2013.md",
    "zookeeper": "research/agent366/md/zookeeper-atc10-hunt.md",
    "etcd-guarantees": "research/agent366/md/web-etcd-api-guarantees.md",
    "etcd-hardware": "research/agent366/md/web-etcd-hardware.md",
    "etcd-embed": "research/agent366/md/web-etcd-embed.md",
    "netdisk": "research/agent366/md/blog-netdisk-2024.md",
    "votingweights": "research/agent366/md/blog-vw-2017-paxos-voting-weights.md",
    "reconfig-sm": "research/agent366/md/reconfiguring-a-state-machine-2010.md",
    "vertical-paxos": "research/agent366/md/vertical-paxos-2009.md",
    "artifact": "formal/uvrr-lean/README.md",
    "lean4": "research/agent366/md/lean4-cade28.md",
}

STOPWORDS = set("""a an and are as at be been but by can could did do does for
from had has have how if in into is it its may might must no not of on or our
out own per said same shall should so some such than that the their them then
there these they this those through to too under until up use used using was
we were what when where which while who why will with within would you your
section appendix table figure fig ref eq cite""".split())

_CITE_RE = re.compile(r"\\cite(?:\[[^\]]*\])?\{([^}]+)\}")
BIB = {}


class HTTPErrorShaped(Exception):
    def __init__(self, code, body):
        self.code = code
        self.body = body
        super().__init__(f"HTTP {code}: {body}")


class Use:
    def __init__(self, line_no, keys, claim):
        self.line_no = line_no
        self.keys = keys
        self.claim = claim


def retrieve_passage(path, claim, top=3, budget=6000):
    """Deterministic keyword retrieval: rank paragraphs of the source by
    content-word overlap with the claim; return the top few joined."""
    full = os.path.join(ROOT, path)
    text = open(full, encoding="utf-8", errors="replace").read()
    terms = {w for w in re.findall(r"[a-zA-Z][a-zA-Z\-']{2,}", claim.lower()) if w not in STOPWORDS}
    paras = [p.strip() for p in re.split(r"\n\s*\n", text) if len(p.strip()) > 60]
    scored = []
    for p in paras:
        pl = p.lower()
        covered = sum(1 for t in terms if t in pl)
        scored.append((covered, len(p), p))
    scored.sort(key=lambda s: -s[0])
    picked, used = [], 0
    for covered, plen, p in scored:
        if covered == 0 and picked:
            break
        if used + plen > budget:
            continue
        picked.append(p)
        used += plen
        if len(picked) >= top:
            break
    return "\n\n---\n\n".join(picked), full


def _detex(s):
    s = re.sub(r"\\emph\{([^}]*)\}", r"\1", s)
    s = re.sub(r"\\href\{[^}]*\}\{([^}]*)\}", r"\1", s)
    s = re.sub(r"\\url\{([^}]*)\}", r"\1", s)
    s = re.sub(r"``|''", '"', s)
    s = re.sub(r"\\begin\{thebibliography\}\{[^}]*\}|\\end\{thebibliography\}", "", s)
    s = re.sub(r"[{}~]", " ", s)
    return re.sub(r"\s+", " ", s).strip()


def load_paper():
    src = open(PAPER).read()
    lines = src.splitlines()
    if "\\begin{thebibliography}" in src:
        blob = src[src.index("\\begin{thebibliography}"):]
        pieces = re.split(r"\\bibitem\{([^}]+)\}", blob)
        for i in range(1, len(pieces) - 1, 2):
            BIB[pieces[i]] = _detex(pieces[i + 1])

    uses = []
    for i, ln in enumerate(lines):
        for m in _CITE_RE.finditer(ln):
            keys = m.group(1).split(",")
            window = " ".join(lines[max(0, i - 3):min(len(lines), i + 4)])
            window = re.sub(r"\\emph\{([^}]*)\}", r"\1", window)
            window = re.sub(r"\\cite(?:\[[^\]]*\])?\{[^}]*\}", "[CITATION]", window)
            window = re.sub(r"\\(eqref|ref|label)\{[^}]*\}", "REF", window)
            window = re.sub(r"\s+", " ", window).strip()
            uses.append(Use(i + 1, keys, window[:1600]))
    return uses


QUESTIONS = {
    "verdict": {
        "type": "choice",
        "instructions": "You are checking citations in a research paper. The state gives the paper's claim, and quoted_passage: verbatim excerpts retrieved from the full text of the cited work. Does the quoted passage support the claim in its surrounding context? Judge only whether the quoted text of the cited work supports the claim as stated; do not judge whether the paper's own conclusions are true. If the quoted passage does not address the claim, the citation is insufficient.",
        "criteria": {
            "supports": "The quoted passage supports the claim as stated",
            "contradicts": "The quoted passage conflicts with the claim",
            "insufficient": "The quoted passage does not establish the claim",
        },
    },
}

FIXTURE = {
    "verdict": {
        "type": "choice",
        "choice": "supports",
        "confidence": 0.92,
        "probabilities": {"supports": 0.947, "contradicts": 0.027, "insufficient": 0.027},
    }
}


def validate(answers):
    if not isinstance(answers, dict):
        raise ValueError("Answer map missing")
    for name, question in QUESTIONS.items():
        answer = answers.get(name)
        if not isinstance(answer, dict) or answer.get("type") != question["type"]:
            raise ValueError("Missing or unexpected answer type: " + name)
        field = {"choice": "choice", "score": "score", "noul": "noul"}[question["type"]]
        value = answer.get(field)
        if field == "choice":
            if value not in question["criteria"]:
                raise ValueError("Unknown category: " + name)
        elif not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value):
            raise ValueError("Invalid number: " + name)
        elif not 0 <= value <= (1 if field == "noul" else len(question["criteria"]) - 1):
            raise ValueError("Out-of-range answer: " + name)
        if field != "noul":
            c = answer.get("confidence")
            if not isinstance(c, (int, float)) or isinstance(c, bool) or not math.isfinite(c) or not 0 <= c <= 1:
                raise ValueError("Invalid confidence: " + name)
    return answers


def decide(answers, use):
    verdict = answers["verdict"]
    if verdict["confidence"] < 0.9:
        route = "review_low_confidence"
    elif verdict["choice"] == "supports":
        route = "accept_evidence_match"
    elif verdict["choice"] == "contradicts":
        route = "reject"
    else:
        route = "seek_more_evidence"
    return {"route": route, "tex_line": use.line_no, "keys": use.keys}


def call_jev(state, key):
    """One bounded POST via curl. Python's urllib TLS signature is blocked by
    the endpoint's bot filter (Cloudflare error 1010); curl's is allowed."""
    payload = json.dumps({"model": MODEL, "state": state, "questions": QUESTIONS})
    proc = subprocess.run(
        [
            "curl", "-sS", "--max-time", str(HTTP_TIMEOUT), "-w", "\n%{http_code}",
            "https://opencode.ai/zen/v1/systemone",
            "-H", "Authorization: Bearer " + key,
            "-H", "Content-Type: application/json",
            "--data-binary", "@-",
        ],
        input=payload, capture_output=True, text=True,
    )
    body, _, tail = proc.stdout.rpartition("\n")
    if proc.returncode != 0 or not tail.isdigit():
        raise urllib.error.URLError("curl failed: " + proc.stderr.strip())
    if int(tail) != 200:
        raise HTTPErrorShaped(int(tail), body[:400])
    return json.loads(body)


def load_cache():
    prior = {}
    if os.path.exists(VERDICTS):
        for line in open(VERDICTS):
            try:
                rec = json.loads(line)
            except ValueError:
                continue
            prior.setdefault((rec["tex_line"], tuple(rec["keys"])), []).append(rec)
    return prior


def append_cache(rec):
    with open(VERDICTS, "a") as f:
        f.write(json.dumps(rec) + "\n")


def main():
    global MODEL
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--model", default="jev-1.13")
    parser.add_argument("--only", default="", help="comma-separated tex line numbers")
    parser.add_argument("--skip_cached", action="store_true")
    args = parser.parse_args()
    MODEL = args.model

    uses = load_paper()
    if args.only:
        wanted = {int(x) for x in args.only.split(",")}
        uses = [u for u in uses if u.line_no in wanted]

    cache = load_cache()
    results = []
    for u in uses:
        prior = cache.get((u.line_no, tuple(u.keys)))
        if args.skip_cached and prior:
            results.append({"tex_line": u.line_no, "keys": u.keys, "claim": u.claim, "cache": prior[-1]})
            continue
        if not args.live:
            answers = deepcopy(FIXTURE)
            model_out = "fixture-not-a-model-run"
        else:
            key = os.environ.get("OPENCODE_API_KEY", "").strip()
            if not key:
                sys.exit("Set OPENCODE_API_KEY in the environment for --live")
            passages = {}
            for k in u.keys:
                excerpt, fullpath = retrieve_passage(SOURCES[k], u.claim)
                passages[k] = {
                    "quoted_passage": excerpt,
                    "source_text": os.path.relpath(fullpath, ROOT),
                    "bibliography_entry": BIB.get(k, "MISSING BIBITEM"),
                }
            state = {
                "claim": u.claim,
                "quoted_passage": {k: v["quoted_passage"] for k, v in passages.items()},
                "surrounding_context": "The citation appears in a research paper draft on diskless viewstamped replication with crash-stop reincarnation, which repairs the diskless-recovery amnesia of Viewstamped Replication Revisited using Turner's reconfiguration-based quorum overlap.",
                "source_metadata": {
                    "tex_line": u.line_no,
                    "keys": u.keys,
                    "source_texts": {k: v["source_text"] for k, v in passages.items()},
                    "retrieved_at": "2026-09-28",
                },
            }
            try:
                result = call_jev(state, key)
            except HTTPErrorShaped as e:
                results.append({"tex_line": u.line_no, "keys": u.keys, "route": "review", "http_status": e.code, "error": "jev_call_failed: " + e.body, "skipped": True})
                continue
            except (urllib.error.URLError, socket.timeout, TimeoutError) as e:
                results.append({"tex_line": u.line_no, "keys": u.keys, "route": "review", "error": "jev_call_failed: %r" % e, "skipped": True})
                continue
            answers = validate(result["answers"])
            model_out = result.get("model", MODEL)
            append_cache({
                "tex_line": u.line_no,
                "keys": u.keys,
                "claim": u.claim,
                "evidence": passages,
                "answers": answers,
                "model": model_out,
            })

        decision = decide(answers, u)
        results.append({
            "tex_line": u.line_no,
            "keys": u.keys,
            "claim": u.claim[:220],
            "decision": decision,
            "answers": answers,
            "model": model_out,
        })

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
