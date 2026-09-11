#!/usr/bin/env -S uv -S --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "httpx",
#   "jinja2",
#   "pyyaml",
# ]
# ///
"""
Multi-model panel review of a LaTeX academic paper.

Usage:
    ./scripts/panel.py --paper formal/uvrr-lean/paper/paper.tex
    ./scripts/panel.py --paper formal/uvrr-lean/paper/paper.tex --models glm-5.3-flash
    ./scripts/panel.py --paper formal/uvrr-lean/paper/paper.tex --models all
    ./scripts/panel.py --paper formal/uvrr-lean/paper/paper.tex --stage blind
    ./scripts/panel.py --paper formal/uvrr-lean/paper/paper.tex --stage merge

Stages (run in order, each builds on the previous):
    blind      — "not totally blind" test: can the model read LaTeX/tables?
    notes      — margin notes: section summaries in JSON
    glossary   — glossary pass: all figures, tables, concepts
    conceptmap — concept map: forward/backward references
    errors     — error detection: deleted-reference test
    grades     — section/figure grading
    recommend  — sweeping recommendations for tone, ordering, impact
    merge      — GLM-5.3 deduplicates all panel results into one set

Models (OpenCode Go endpoints):
    glm-5.3-flash    — OpenAI Chat Completions  (/zen/go/v1/chat/completions)
    deepseek-v4-flash — OpenAI Chat Completions  (/zen/go/v1/chat/completions)
    qwen3.8-flash    — Anthropic Messages       (/zen/go/v1/messages)

The merge model is glm-5.3 (not K3 — K3 is expensive).

Environment:
    OPENCODE_API_KEY — required for all OpenCode Zen/Go calls
    MISTRAL_API_KEY   — optional, for Mistral models

Scratch space:
    .tmp/panel_{unix_epoch}/{model}/{stage}.json
    .tmp/panel_{unix_epoch}/merge/{stage}.json
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

# --- Configuration -----------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent
RUBRIC_PATH = REPO_ROOT / "scripts" / "panel-rubric.md"
PROMPTFOO_DIR = REPO_ROOT / ".promptfoo"

MODELS = {
    # OpenCode Go models (require OPENCODE_API_KEY + x-opencode-session header)
    "glm-5.3-flash": {
        "endpoint": "https://opencode.ai/zen/go/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "GLM-5.3-Flash (OpenCode Go)",
        "provider": "opencode_go",
    },
    "deepseek-v4-flash": {
        "endpoint": "https://opencode.ai/zen/go/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "DeepSeek V4 Flash (OpenCode Go)",
        "provider": "opencode_go",
        "note": "Requires China region opt-in",
    },
    "mimo-v2.5": {
        "endpoint": "https://opencode.ai/zen/go/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "MiMo V2.5 (OpenCode Go)",
        "provider": "opencode_go",
    },
    "qwen3.8-flash": {
        "endpoint": "https://opencode.ai/zen/go/v1/messages",
        "api_shape": "anthropic_messages",
        "display": "Qwen 3.8 Flash (OpenCode Go)",
        "provider": "opencode_go",
    },
    # GLM 5.2 on Mistral — same model as OpenCode Go's glm-5.3-flash but
    # hosted by Mistral without the reasoning-token issue. 1M context, 128k output.
    "zai-glm-5-2": {
        "endpoint": "https://api.mistral.ai/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "GLM 5.2 (Mistral)",
        "provider": "mistral",
    },
    # Merge model: GLM-5.3 on OpenCode Go (falls back to zai-glm-5-2 on Mistral)
    "glm-5.3": {
        "endpoint": "https://opencode.ai/zen/go/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "GLM-5.3 (OpenCode Go)",
        "provider": "opencode_go",
    },
    # Mistral models
    "mistral-small-latest": {
        "endpoint": "https://api.mistral.ai/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "Mistral Small",
        "provider": "mistral",
    },
    "ministral-8b-latest": {
        "endpoint": "https://api.mistral.ai/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "Ministral 8B",
        "provider": "mistral",
    },
    "mistral-medium-latest": {
        "endpoint": "https://api.mistral.ai/v1/chat/completions",
        "api_shape": "openai_chat",
        "display": "Mistral Medium",
        "provider": "mistral",
    },
}

STAGES = [
    "blind",
    "notes",
    "glossary",
    "conceptmap",
    "errors",
    "grades",
    "recommend",
]

# --- Utilities ---------------------------------------------------------------


def die(msg: str, code: int = 1) -> None:
    print(f"panel: {msg}", file=sys.stderr)
    sys.exit(code)


def load_env() -> None:
    env_file = REPO_ROOT / ".env"
    if not env_file.exists():
        die(".env not found at repo root")
    for line in env_file.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            key, _, val = line.partition("=")
            os.environ.setdefault(key.strip(), val.strip())


def check_gitignore() -> None:
    gitignore = REPO_ROOT / ".gitignore"
    if not gitignore.exists():
        die(".gitignore not found")
    content = gitignore.read_text()
    if ".tmp/" not in content:
        die(".tmp/ is not in .gitignore — refusing to write scratch there")
    if ".promptfoo/" not in content:
        die(".promptfoo/ is not in .gitignore — refusing to write configs there")


def make_panel_dir() -> Path:
    epoch = int(time.time())
    panel_dir = REPO_ROOT / ".tmp" / f"panel_{epoch}"
    panel_dir.mkdir(parents=True, exist_ok=True)
    return panel_dir


def read_paper(paper_path: Path) -> str:
    if not paper_path.exists():
        die(f"paper not found: {paper_path}")
    return paper_path.read_text()


def read_rubric() -> str:
    if not RUBRIC_PATH.exists():
        die(f"rubric not found: {RUBRIC_PATH}")
    return RUBRIC_PATH.read_text()


def extract_sections(latex: str) -> list[dict[str, str]]:
    """Extract sections from LaTeX source."""
    import re

    sections = []
    pattern = r"\\section\{([^}]*)\}(?:\\label\{([^}]*)\})?"
    for m in re.finditer(pattern, latex):
        sections.append(
            {
                "title": m.group(1),
                "label": m.group(2) or "",
                "line": latex[: m.start()].count("\n") + 1,
            }
        )
    return sections


def extract_floats(latex: str) -> list[dict[str, str]]:
    """Extract figure and table labels/captions from LaTeX."""
    import re

    floats = []
    # Figures
    for m in re.finditer(
        r"\\begin\{figure\}.*?\\caption\{([^}]*)\}.*?\\label\{(fig:[^}]*)\}",
        latex,
        re.DOTALL,
    ):
        floats.append(
            {"type": "figure", "caption": m.group(1), "label": m.group(2)}
        )
    # Tables
    for m in re.finditer(
        r"\\begin\{table\}.*?\\caption\{([^}]*)\}.*?\\label\{(tab:[^}]*)\}",
        latex,
        re.DOTALL,
    ):
        floats.append(
            {"type": "table", "caption": m.group(1), "label": m.group(2)}
        )
    return floats


# --- API calls ---------------------------------------------------------------


def call_openai_chat(
    endpoint: str,
    api_key: str,
    model: str,
    messages: list[dict[str, str]],
    session_id: str = "panel-review",
    **kwargs,
) -> dict[str, Any]:
    """Call an OpenAI-compatible chat completions endpoint."""
    import httpx

    body = {"model": model, "messages": messages, **kwargs}
    try:
        resp = httpx.post(
            endpoint,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
                "x-opencode-session": session_id,
                "User-Agent": "panel-review/1.0",
            },
            json=body,
            timeout=180,
        )
        resp.raise_for_status()
        return resp.json()
    except httpx.HTTPStatusError as e:
        return {"error": f"HTTP {e.response.status_code}: {e.response.text[:500]}"}
    except Exception as e:
        return {"error": str(e)}


def call_anthropic_messages(
    endpoint: str,
    api_key: str,
    model: str,
    messages: list[dict[str, str]],
    session_id: str = "panel-review",
    **kwargs,
) -> dict[str, Any]:
    """Call an Anthropic Messages API endpoint."""
    import httpx

    body = {"model": model, "messages": messages, **kwargs}
    try:
        resp = httpx.post(
            endpoint,
            headers={
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
                "Content-Type": "application/json",
                "x-opencode-session": session_id,
                "User-Agent": "panel-review/1.0",
            },
            json=body,
            timeout=180,
        )
        resp.raise_for_status()
        return resp.json()
    except httpx.HTTPStatusError as e:
        return {"error": f"HTTP {e.response.status_code}: {e.response.text[:500]}"}
    except Exception as e:
        return {"error": str(e)}


def call_model(model_id: str, system: str, user: str, max_tokens: int = 16384) -> str:
    """Call a model and return the text response."""
    system = "Output JSON only."
    cfg = MODELS[model_id]

    # Pick the right API key based on provider
    if cfg.get("provider") == "mistral":
        api_key = os.environ.get("MISTRAL_API_KEY", "")
        if not api_key:
            die("MISTRAL_API_KEY not set")
    else:
        api_key = os.environ.get("OPENCODE_API_KEY", "")
        if not api_key:
            die("OPENCODE_API_KEY not set")

    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]

    if cfg["api_shape"] == "openai_chat":
        result = call_openai_chat(
            cfg["endpoint"], api_key, model_id, messages, max_tokens=max_tokens
        )
        if "error" in result:
            return f"ERROR: {result['error']}"
        return result.get("choices", [{}])[0].get("message", {}).get("content", "")
    elif cfg["api_shape"] == "anthropic_messages":
        result = call_anthropic_messages(
            cfg["endpoint"], api_key, model_id, messages, max_tokens=max_tokens
        )
        if "error" in result:
            return f"ERROR: {result['error']}"
        content = result.get("content", [])
        if isinstance(content, list):
            return "".join(
                block.get("text", "") for block in content if block.get("type") == "text"
            )
        return str(content)
    else:
        die(f"unknown api_shape: {cfg['api_shape']}")
        return ""


# --- Promptfoo integration ---------------------------------------------------


def ensure_promptfoo_dir() -> None:
    PROMPTFOO_DIR.mkdir(parents=True, exist_ok=True)


def run_promptfoo(config_path: Path, output_path: Path | None = None) -> dict:
    """Run promptfoo eval and return results."""
    cmd = ["promptfoo", "eval", "--config", str(config_path)]
    if output_path:
        cmd += ["--output", str(output_path)]

    try:
        result = subprocess.run(
            cmd,
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode != 0:
            return {"error": result.stderr, "stdout": result.stdout}
        return {"stdout": result.stdout, "stderr": result.stderr}
    except subprocess.TimeoutExpired:
        return {"error": "promptfoo eval timed out after 300s"}
    except FileNotFoundError:
        return {"error": "promptfoo not found — install with: npm i -g promptfoo"}


# --- Stage implementations ----------------------------------------------------


def stage_blind(paper: str, model_id: str, panel_dir: Path) -> dict:
    """
    Stage 1: 'Not totally blind' test.
    Send the model the LaTeX source and ask a question that can only be
    answered if it can read the math or tables.
    """
    system = (
        "You are a paper reviewer. You are given the LaTeX source of an "
        "academic paper. Answer questions about it precisely."
    )
    user = f"""Here is the LaTeX source of a paper:

```latex
{paper[:8000]}
```

Question: What is the title of this paper? What is the main theorem or
result stated? List every figure and table label you can find (e.g. fig:synod,
tab:correspondence). What is the abstract about?

Answer in JSON:
{{
  "title": "...",
  "main_result": "...",
  "floats": ["fig:...", "tab:..."],
  "abstract_summary": "..."
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "blind", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "blind", result)
    return result


def stage_notes(paper: str, model_id: str, panel_dir: Path) -> dict:
    """
    Stage 2: Margin notes — section summaries in JSON.
    The model reads the paper and writes out key points per section.
    """
    system = (
        "You are a paper reviewer performing a light margin-note methodology. "
        "Read the paper and write margin notes: ideas, terms, what would be "
        "underlined, questions the paper leaves open. Output JSON only."
    )
    user = f"""Here is the LaTeX source of a paper:

```latex
{paper}
```

Read through the paper and write margin notes for each section. For each
section, record:
- The section title
- Key points (2-5 bullet points)
- New terms or concepts introduced
- Questions or open issues

Output as JSON:
{{
  "sections": [
    {{
      "title": "...",
      "key_points": ["...", "..."],
      "new_terms": ["...", "..."],
      "questions": ["...", "..."]
    }}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "notes", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "notes", result)
    return result


def stage_glossary(paper: str, model_id: str, panel_dir: Path, prev_notes: str = "") -> dict:
    """
    Stage 3: Glossary pass. Build a complete glossary from the margin notes
    and the paper. Every figure and table must appear.
    """
    system = (
        "You are a paper reviewer building a glossary. Consolidate all "
        "concepts, terms, figures, and tables into a single glossary. "
        "Every figure and table MUST appear."
    )
    context = ""
    if prev_notes:
        context = f"\n\nHere are the margin notes from the previous pass:\n{prev_notes}\n"

    # Truncate paper for flash models to avoid timeout
    paper_trunc = paper

    user = f"""Here is the LaTeX source of a paper:
```latex
{paper_trunc}
```
{context}
Build a complete glossary. For each entry:
- preferred_term: one unambiguous name
- aliases: other names used in the paper
- definition: concise definition
- first_seen: section or line where first encountered
- type: concept | figure | table | theorem | definition

Every figure (fig:*) and table (tab:*) MUST appear as a glossary entry.

Output as JSON:
{{
  "glossary": [
    {{
      "preferred_term": "...",
      "aliases": ["..."],
      "definition": "...",
      "first_seen": "...",
      "type": "..."
    }}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "glossary", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "glossary", result)
    return result


def stage_conceptmap(
    paper: str, model_id: str, panel_dir: Path, glossary: str = ""
) -> dict:
    """
    Stage 4: Concept map. Go through the paper with the glossary and build
    forward/backward references and concept dependencies.
    """
    system = (
        "You are a paper reviewer building a concept map. Go through the "
        "paper and record where each concept appears, classify references "
        "as forward or backward, and record concept dependencies. "
        "Output JSON only."
    )
    context = ""
    if glossary:
        context = f"\n\nHere is the glossary from the previous pass:\n{glossary}\n"

    paper_trunc = paper

    user = f"""List concepts and their dependencies from this paper as JSON.

Paper:
```latex
{paper_trunc}
```
{context}

Output as JSON:
{{
  "concepts": [
    {{
      "name": "...",
      "first_appearance": "...",
      "depends_on": ["..."],
      "referenced_by": ["..."]
    }}
  ],
  "unreferenced_floats": ["fig:...", "tab:..."],
  "dangling_references": [
        {{"reference": "...", "issue": "..."}}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "conceptmap", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "conceptmap", result)
    return result


def stage_errors(paper: str, model_id: str, panel_dir: Path, conceptmap: str = "") -> dict:
    """
    Stage 5: Error detection. Give the model a copy of the paper with a
    deleted reference and check if it finds the error.
    """
    system = (
        "You are a paper reviewer checking for errors. List all omissions, "
        "errors, broken items, and dangling references. Output JSON only."
    )
    context = ""
    if conceptmap:
        context = f"\n\nHere is the concept map from the previous pass:\n{conceptmap}\n"

    # Create a deliberately broken version: remove one \\ref{fig:synod}
    broken_paper = paper.replace("\\ref{fig:synod}", "\\ref{fig:DELETED}", 1)
    paper_trunc = broken_paper

    user = f"""Here is the LaTeX source of a paper that may contain errors:
```latex
{paper_trunc}
```
{context}
Check this paper for:
1. Omissions: material referenced but not present.
2. Errors: wrong citations, broken references, mislabeled figures.
3. Broken items: LaTeX errors, undefined references, missing captions.
4. Dangling references: "see Section X" where Section X does not exist.
5. Unreferenced figures or tables: a float never cited in the text.

Pay special attention to any \\ref{{...}} that points to a label that does
not exist in the paper.

Output as JSON:
{{
  "errors": [
    {{
      "type": "omission|error|broken|dangling|unreferenced",
      "location": "...",
      "description": "...",
      "severity": "high|medium|low"
    }}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "errors", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "errors", result)
    return result


def stage_grades(
    paper: str, model_id: str, panel_dir: Path, rubric: str, conceptmap: str = ""
) -> dict:
    """
    Stage 6: Grade each section, figure, and table F through A (or U for
    placeholders).
    """
    system = (
        "You are a paper reviewer grading each section, figure, and table. "
        "Use grades A through F, and U for placeholder sections (lorem ipsum, "
        "TODO, or explicit draft warnings). Output JSON only."
    )
    context = ""
    if conceptmap:
        context = f"\n\nHere is the concept map:\n{conceptmap}\n"

    paper_trunc = paper

    user = f"""Here is the LaTeX source of a paper:
```latex
{paper_trunc}
```

Here is the grading rubric:
{rubric}
{context}
Grade every section, every figure, and every table. For each:
- name: section title or float label
- grade: A | B | C | D | F | U
- comments: what would improve it
- location: where it is in the paper

Remember:
- U for placeholder sections (lorem ipsum, TODO, or explicit warnings).
- C for warned gaps (the paper carries an explicit warning).
- D for silent gaps (missing content with no acknowledgment).
- F for broken or wrong content.

Output as JSON:
{{
  "grades": [
    {{
      "name": "...",
      "type": "section|figure|table",
      "grade": "A|B|C|D|F|U",
      "comments": "...",
      "location": "..."
    }}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "grades", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "grades", result)
    return result


def stage_recommend(
    paper: str, model_id: str, panel_dir: Path, rubric: str, grades: str = ""
) -> dict:
    """
    Stage 7: Sweeping recommendations for tone, ordering, and impact.
    The model is told the paper has been linted for errors and is asked to
    make recommendations to improve impact without hyperbole.
    """
    system = (
        "You are a paper reviewer making sweeping recommendations. The paper "
        "has already been linted for errors. Focus on tone, ordering, and "
        "impact. Be persuasive without shallow pathos or ethos. Output JSON only."
    )
    context = ""
    if grades:
        context = f"\n\nHere are the grades from the panel:\n{grades}\n"

    paper_trunc = paper

    user = f"""Here is the LaTeX source of a paper that has been linted for errors:
```latex
{paper_trunc}
```

Here is the rubric:
{rubric}
{context}
Make sweeping recommendations to improve the impact of the findings:

1. Title: Should it change? Be more specific, more punchy, or more accurate?
2. Abstract: Should results be pulled forward? Is the contribution clear in
   the first three sentences?
3. Ordering: Should sections be reordered for better flow?
4. Tone: Is it well-calibrated — confident without being cocky, precise
   without being pedantic?
5. Presentation: Are key results foregrounded or hidden? Are concepts easy
   to follow?
6. Impact: How can the paper be more persuasive about impact without shallow
   pathos or ethos techniques? How can it make concepts easy to follow while
   having the necessary details to back up claims?

Output as JSON:
{{
  "recommendations": [
    {{
      "category": "title|abstract|ordering|tone|presentation|impact",
      "recommendation": "...",
      "priority": "high|medium|low",
      "rationale": "..."
    }}
  ]
}}
"""

    response = call_model(model_id, system, user)
    result = {"stage": "recommend", "model": model_id, "response": response}
    save_result(panel_dir, model_id, "recommend", result)
    return result


def stage_merge(
    panel_dir: Path,
    paper: str,
    rubric: str,
    all_results: dict[str, dict],
    merge_model: str = "glm-5.3",
) -> dict:
    """
    Merge stage: deduplicates all panel results into one set of
    grades, issues, and recommendations.
    """
    model_id = merge_model

    # Collect all model results, truncating responses to keep prompt small
    truncated_results: dict[str, dict] = {}
    for model_id, model_results in all_results.items():
        if model_id == "glm-5.3":
            continue
        truncated_results[model_id] = {}
        for stage, result in model_results.items():
            resp = result.get("response", "")
            # Strip markdown fences
            if resp.startswith("```"):
                resp = resp.replace("```json\n", "", 1).replace("```\n", "", 1)
                if "```" in resp:
                    resp = resp.rsplit("```", 1)[0]
            truncated_results[model_id][stage] = resp[:4000]

    panel_input = json.dumps(truncated_results, indent=2)

    system = (
        "You are a senior paper reviewer orchestrating a panel of reviewers. "
        "Multiple models have reviewed the same paper independently. "
        "Deduplicate their findings into one consolidated set of grades, "
        "issues, and recommendations. Output JSON only."
    )

    user = f"""Here is the LaTeX source of the paper (truncated):
```latex
{paper[:5000]}
```

Here is the rubric (truncated):
{rubric[:2000]}

Here are the results from the panel of reviewers (each reviewed independently):
{panel_input}

Deduplicate and consolidate into one set of:
1. Grades: one grade per section/figure/table, with the consensus grade and
   any dissent noted.
2. Issues: deduplicated list of all errors, omissions, and broken items found
   by any reviewer.
3. Recommendations: deduplicated and prioritized list of all recommendations.

Output as JSON:
{{
  "consolidated_grades": [
    {{
      "name": "...",
      "type": "section|figure|table",
      "consensus_grade": "A|B|C|D|F|U",
      "dissent": "...",
      "comments": "..."
    }}
  ],
  "consolidated_issues": [
    {{
      "type": "omission|error|broken|dangling|unreferenced",
      "description": "...",
      "severity": "high|medium|low",
      "found_by": ["model1", "model2"]
    }}
  ],
  "consolidated_recommendations": [
    {{
      "category": "title|abstract|ordering|tone|presentation|impact",
      "recommendation": "...",
      "priority": "high|medium|low",
      "rationale": "..."
    }}
  ]
}}
"""

    response = call_model(model_id, system, user, max_tokens=8192)
    result = {"stage": "merge", "model": model_id, "response": response}
    save_result(panel_dir, "merge", "merge", result)
    return result


# --- Result persistence ------------------------------------------------------


def save_result(panel_dir: Path, model_id: str, stage: str, result: dict) -> None:
    model_dir = panel_dir / model_id
    model_dir.mkdir(parents=True, exist_ok=True)
    out_path = model_dir / f"{stage}.json"
    out_path.write_text(json.dumps(result, indent=2))
    print(f"  saved: {out_path.relative_to(REPO_ROOT)}")


def load_result(panel_dir: Path, model_id: str, stage: str) -> dict | None:
    path = panel_dir / model_id / f"{stage}.json"
    if path.exists():
        return json.loads(path.read_text())
    return None


# --- Promptfoo config generation ---------------------------------------------


def gen_promptfoo_config(
    stage: str, model_id: str, paper: str, panel_dir: Path
) -> Path:
    """Generate a promptfoo YAML config for a given stage and model."""
    import yaml  # type: ignore

    cfg = {
        "description": f"Panel review stage: {stage} — model: {model_id}",
        "prompts": [],
        "providers": [],
        "tests": [],
    }

    # Build the prompt based on stage
    if stage == "blind":
        cfg["prompts"] = [
            f"Given this LaTeX paper source, what is the title, main result, "
            f"and list all figure/table labels?\n\n{paper[:8000]}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "Unbounded Viewstamped Replication"},
                    {"type": "icontains", "value": "fig:"},
                    {"type": "icontains", "value": "tab:"},
                ],
            }
        ]
    elif stage == "notes":
        cfg["prompts"] = [
            f"Read this LaTeX paper and write margin notes as JSON with "
            f"sections, key_points, new_terms, and questions.\n\n{paper}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "sections"},
                    {"type": "icontains", "value": "key_points"},
                ],
            }
        ]
    elif stage == "glossary":
        cfg["prompts"] = [
            f"Build a complete glossary from this paper as JSON. Every figure "
            f"and table MUST appear.\n\n{paper}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "glossary"},
                    {"type": "icontains", "value": "fig:synod"},
                    {"type": "icontains", "value": "tab:correspondence"},
                ],
            }
        ]
    elif stage == "conceptmap":
        cfg["prompts"] = [
            f"Build a concept map from this paper as JSON with forward/backward "
            f"references and dependencies.\n\n{paper}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "concepts"},
                    {"type": "icontains", "value": "depends_on"},
                ],
            }
        ]
    elif stage == "errors":
        broken = paper.replace("\\ref{fig:synod}", "\\ref{fig:DELETED}", 1)
        cfg["prompts"] = [
            f"Check this LaTeX paper for errors, broken references, and "
            f"dangling references. Output JSON.\n\n{broken}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "errors"},
                    {"type": "icontains", "value": "fig:DELETED"},
                ],
            }
        ]
    elif stage == "grades":
        rubric = read_rubric()
        cfg["prompts"] = [
            f"Grade each section, figure, and table in this paper as JSON. "
            f"Use A-F or U for placeholders.\n\nRubric:\n{rubric}\n\nPaper:\n{paper}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "grades"},
                    {"type": "icontains", "value": "grade"},
                ],
            }
        ]
    elif stage == "recommend":
        rubric = read_rubric()
        cfg["prompts"] = [
            f"Make sweeping recommendations for this paper's tone, ordering, "
            f"and impact as JSON.\n\nRubric:\n{rubric}\n\nPaper:\n{paper}"
        ]
        cfg["tests"] = [
            {
                "assert": [
                    {"type": "icontains", "value": "recommendations"},
                    {"type": "icontains", "value": "priority"},
                ],
            }
        ]

    # Provider config
    model_cfg = MODELS[model_id]
    if model_cfg.get("provider") == "mistral":
        cfg["providers"] = [
            {
                "id": "openai-compatible",
                "apiBaseUrl": "https://api.mistral.ai/v1",
                "apiKey": "${MISTRAL_API_KEY}",
                "model": model_id,
            }
        ]
    elif model_cfg["api_shape"] == "openai_chat":
        cfg["providers"] = [
            {
                "id": "opencode",
                "type": "openai",
                "apiBaseUrl": "https://opencode.ai/zen/go/v1",
                "apiKey": "${OPENCODE_API_KEY}",
                "model": model_id,
            }
        ]
    elif model_cfg["api_shape"] == "anthropic_messages":
        cfg["providers"] = [
            {
                "id": "opencode",
                "type": "anthropic",
                "apiBaseUrl": "https://opencode.ai/zen/go/v1",
                "apiKey": "${OPENCODE_API_KEY}",
                "model": model_id,
            }
        ]

    # Write config
    ensure_promptfoo_dir()
    cfg_path = PROMPTFOO_DIR / f"{stage}_{model_id}.yaml"
    cfg_path.write_text(yaml.dump(cfg, default_flow_style=False))
    return cfg_path


# --- Main flow ---------------------------------------------------------------


def run_ladder(
    paper: str,
    rubric: str,
    model_id: str,
    panel_dir: Path,
    stages: list[str] | None = None,
) -> dict[str, dict]:
    """Run the full ladder of stages for one model."""
    if stages is None:
        stages = STAGES

    results: dict[str, dict] = {}
    prev_output = ""

    def clean_response(text: str, max_len: int = 4000) -> str:
        """Strip markdown fences and truncate. For JSON, try to extract just key fields."""
        if text.startswith("```"):
            text = text.replace("```json\n", "", 1).replace("```\n", "", 1)
            if "```" in text:
                text = text.rsplit("```", 1)[0]
        # Try to extract just term names from glossary JSON to save context
        try:
            parsed = json.loads(text)
            if "glossary" in parsed:
                terms = [entry.get("preferred_term", "") for entry in parsed["glossary"] if entry.get("preferred_term")]
                return ", ".join(terms)[:max_len]
            if "concepts" in parsed:
                names = [c.get("name", "") for c in parsed["concepts"] if c.get("name")]
                return ", ".join(names)[:max_len]
            if "sections" in parsed:
                titles = [s.get("title", "") for s in parsed["sections"] if s.get("title")]
                return ", ".join(titles)[:max_len]
        except (json.JSONDecodeError, TypeError):
            pass
        return text[:max_len]

    for stage in stages:
        print(f"\n  [{model_id}] stage: {stage}")

        # If running a single stage, try to load the previous stage's output from disk
        if len(stages) == 1 and not prev_output:
            stage_idx = STAGES.index(stage) if stage in STAGES else -1
            if stage_idx > 0:
                prev_stage = STAGES[stage_idx - 1]
                prev_result = load_result(panel_dir, model_id, prev_stage)
                if prev_result:
                    prev_output = clean_response(prev_result.get("response", ""))
                    print(f"    loaded previous stage '{prev_stage}' from disk ({len(prev_output)} chars)")

        if stage == "blind":
            r = stage_blind(paper, model_id, panel_dir)
        elif stage == "notes":
            r = stage_notes(paper, model_id, panel_dir)
        elif stage == "glossary":
            r = stage_glossary(paper, model_id, panel_dir, prev_notes=prev_output)
        elif stage == "conceptmap":
            r = stage_conceptmap(paper, model_id, panel_dir, glossary=prev_output)
        elif stage == "errors":
            r = stage_errors(paper, model_id, panel_dir, conceptmap=prev_output)
        elif stage == "grades":
            r = stage_grades(paper, model_id, panel_dir, rubric, conceptmap=prev_output)
        elif stage == "recommend":
            r = stage_recommend(paper, model_id, panel_dir, rubric, grades=prev_output)
        else:
            print(f"    unknown stage: {stage}, skipping")
            continue

        results[stage] = r
        prev_output = clean_response(r.get("response", ""))

        # Check for errors
        if r["response"].startswith("ERROR:"):
            print(f"    FAILED: {r['response'][:200]}")
            break

    return results


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Multi-model panel review of a LaTeX paper"
    )
    parser.add_argument(
        "--paper",
        type=Path,
        default=Path("formal/uvrr-lean/paper/paper.tex"),
        help="Path to the LaTeX paper source",
    )
    parser.add_argument(
        "--models",
        type=str,
        default="all",
        help="Comma-separated model IDs, or 'all' for the three flash models",
    )
    parser.add_argument(
        "--stage",
        type=str,
        default=None,
        help="Run only one stage (blind, notes, glossary, conceptmap, errors, grades, recommend, merge)",
    )
    parser.add_argument(
        "--promptfoo",
        action="store_true",
        help="Generate promptfoo configs and run them instead of direct API calls",
    )
    parser.add_argument(
        "--panel-dir",
        type=Path,
        default=None,
        help="Reuse an existing panel directory instead of creating a new one",
    )

    args = parser.parse_args()

    # Setup
    load_env()
    check_gitignore()

    paper_path = (REPO_ROOT / args.paper).resolve() if not args.paper.is_absolute() else args.paper
    paper = read_paper(paper_path)
    rubric = read_rubric()

    # Determine models
    if args.models == "all":
        # Use GLM 5.2 on Mistral (same model as OpenCode Go's glm-5.3-flash
        # but without the reasoning-token issue) plus two OpenCode Go models.
        opencode_key = os.environ.get("OPENCODE_API_KEY", "")
        mistral_key = os.environ.get("MISTRAL_API_KEY", "")
        if mistral_key and opencode_key:
            model_ids = ["zai-glm-5-2", "mimo-v2.5", "qwen3.8-flash"]
        elif mistral_key:
            model_ids = ["zai-glm-5-2", "mistral-small-latest", "mistral-medium-latest"]
        else:
            model_ids = ["glm-5.3-flash", "mimo-v2.5", "qwen3.8-flash"]
    elif args.models == "mistral":
        model_ids = ["mistral-small-latest", "ministral-8b-latest", "mistral-medium-latest"]
    elif args.models == "merge":
        model_ids = ["zai-glm-5-2"]
    elif args.models == "merge-mistral":
        model_ids = ["mistral-medium-latest"]
    else:
        model_ids = [m.strip() for m in args.models.split(",")]

    # Determine stages
    if args.stage:
        stages = [args.stage]
    else:
        stages = STAGES

    # Create or reuse panel directory
    if args.panel_dir:
        panel_dir = args.panel_dir if args.panel_dir.is_absolute() else REPO_ROOT / args.panel_dir
    else:
        panel_dir = make_panel_dir()

    print(f"Panel directory: {panel_dir.relative_to(REPO_ROOT)}")
    print(f"Paper: {paper_path.relative_to(REPO_ROOT)}")
    print(f"Models: {model_ids}")
    print(f"Stages: {stages}")

    # Run promptfoo mode
    if args.promptfoo:
        print("\n=== Promptfoo mode ===")
        for model_id in model_ids:
            for stage in stages:
                if stage == "merge":
                    continue
                print(f"\n  [{model_id}] promptfoo stage: {stage}")
                cfg_path = gen_promptfoo_config(stage, model_id, paper, panel_dir)
                output_path = panel_dir / model_id / f"{stage}_promptfoo.json"
                output_path.parent.mkdir(parents=True, exist_ok=True)
                result = run_promptfoo(cfg_path, output_path)
                if "error" in result:
                    print(f"    FAILED: {result['error'][:200]}")
                else:
                    print(f"    done (see {output_path.relative_to(REPO_ROOT)})")
        return

    # Run direct API mode
    all_results: dict[str, dict[str, dict]] = {}

    for model_id in model_ids:
        if model_id not in MODELS:
            die(f"unknown model: {model_id}. Available: {', '.join(MODELS.keys())}")

        print(f"\n=== Running panel: {model_id} ===")
        model_stages = [s for s in stages if s != "merge"]
        results = run_ladder(paper, rubric, model_id, panel_dir, model_stages)
        all_results[model_id] = results

    # Merge stage
    if "merge" in stages or (args.stage == "merge"):
        print("\n=== Merging panel results (GLM-5.3) ===")
        # Collect all non-merge results
        merge_input: dict[str, dict[str, dict]] = {}
        for model_id, model_results in all_results.items():
            if model_id == "glm-5.3":
                continue
            merge_input[model_id] = model_results

        if not merge_input:
            # Load from disk if we didn't just run
            for model_dir in panel_dir.iterdir():
                if model_dir.name == "merge":
                    continue
                model_results: dict[str, dict] = {}
                for stage_file in model_dir.glob("*.json"):
                    stage = stage_file.stem
                    model_results[stage] = json.loads(stage_file.read_text())
                merge_input[model_dir.name] = model_results

        if merge_input:
            # Pick merge model: zai-glm-5-2 on Mistral (no reasoning-token issue)
            merge_model = "zai-glm-5-2"
            if all(m in MODELS for m in model_ids) and all(
                MODELS[m].get("provider") == "mistral" for m in model_ids
            ):
                merge_model = "mistral-medium-latest"

            merge_result = stage_merge(panel_dir, paper, rubric, merge_input, merge_model)
            print(f"\n  Merge complete: {panel_dir / 'merge' / 'merge.json'}")
        else:
            print("  No panel results found to merge")

    print(f"\nDone. Panel directory: {panel_dir.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
