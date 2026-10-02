#!/usr/bin/env python3
"""Render paper.tex as a one-column markdown reading copy.

The paper is paper.tex; this is a reading copy, not a second source of truth.
Pandoc does the structural conversion, then a normalisation pass makes the
prose readable aloud: display maths is flattened to plain text, section
references become plain numbers, and cross-reference labels are dropped.

Usage: to_markdown.py <input.tex> <output.md>

Requires pandoc on PATH. Exits non-zero with a diagnostic when it is absent, so
the caller can copy the PDF and skip the markdown without failing the build.
"""
import re
import subprocess
import sys

# LaTeX escapes and symbols that carry their meaning when spoken.
SYMBOLS = {
    r"\to": " → ", r"\gets": " ← ", r"\leftarrow": " ← ",
    r"\le": " ≤ ", r"\leq": " ≤ ", r"\ge": " ≥ ", r"\geq": " ≥ ",
    r"\neq": " ≠ ", r"\ne": " ≠ ", r"\equiv": " ≡ ",
    r"\neg": " ¬ ", r"\lnot": " ¬ ", r"\wedge": " ∧ ", r"\land": " ∧ ",
    r"\vee": " ∨ ", r"\lor": " ∨ ", r"\times": " × ", r"\cdot": " · ",
    r"\ell": "ℓ", r"\mu": "μ", r"\in": " ∈ ", r"\mid": "|",
    r"\forall": "∀", r"\exists": "∃", r"\vdash": "⊢", r"\circ": "∘",
    r"\ldots": "…", r"\dots": "…", r"\prime": "′",
    r"\langle": "⟨", r"\rangle": "⟩", r"\sum": "∑",
    r"\{": "{", r"\}": "}", r"\%": "%", r"\&": "&", r"\#": "#",
}


def pandoc(src: str) -> str:
    done = subprocess.run(
        ["pandoc", src, "-f", "latex", "-t", "gfm", "--wrap=none"],
        capture_output=True, text=True,
    )
    if done.returncode != 0:
        sys.exit("to_markdown: pandoc failed:\n" + done.stderr)
    return done.stdout


def flatten_math(text: str) -> str:
    """Turn LaTeX source into spoken prose: drop markup, keep the symbols."""
    text = re.sub(r"\\label\{[^}]*\}", "", text)
    text = re.sub(r"\\(?:begin|end)\{[^}]*\}", "", text)
    text = re.sub(r"\\(?:text|mathit|mathrm|mathbf|mathsf|operatorname)\{([^}]*)\}", r"\1", text)
    text = re.sub(r"\\cite(?:\[[^\]]*\])?\{[^}]*\}", "", text)
    text = re.sub(r"\\label\{[^}]*\}", "", text)
    text = text.replace("\\\\", "\n")
    text = re.sub(r"\\qquad|\\quad|\\,|\\;|\\ ", "  ", text)
    text = re.sub(r"\\ref\{[^}]*\}", "", text)
    for tex, sym in SYMBOLS.items():
        text = text.replace(tex, sym)
    # Any surviving command is markup, not content.
    text = re.sub(r"\\[a-zA-Z]+\s*", " ", text)
    return re.sub(r"[ \t]+", " ", text).strip()


def clean(markdown: str) -> str:
    # Pandoc resolves \ref to an HTML anchor; keep the label's text only.
    markdown = re.sub(r'<a href="[^"]*"[^>]*>([^<]*)</a>', r"\1", markdown)
    markdown = re.sub(r"\[(?:eq|tab|fig|sec):[A-Za-z0-9_-]+\]", "", markdown)
    markdown = re.sub(r"``` math\n(.*?)```",
                      lambda m: "```\n" + flatten_math(m.group(1)) + "\n```",
                      markdown, flags=re.S)
    markdown = re.sub(r"\$`(.+?)`\$", lambda m: flatten_math(m.group(1)), markdown)
    markdown = re.sub(r"\\(?:emph|textbf|textit)\{([^}]*)\}", r"*\1*", markdown)
    markdown = re.sub(r"\\texttt\{([^}]*)\}", r"\1", markdown)
    return markdown


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: to_markdown.py <input.tex> <output.md>")
    src, dst = sys.argv[1], sys.argv[2]
    open(dst, "w", encoding="utf-8").write(clean(pandoc(src)))


if __name__ == "__main__":
    main()