#!/usr/bin/env python3
"""Extract every paragraph of paper/paper.tex into a Word-structure-view
markdown document: headings for the section tree, and each paragraph wrapped
as <p x-name="..." x-tex="paper.tex:START-END" x-labels=""> so it can be
sorted and graded independently of the LaTeX.

Usage: structure_extract.py <paper.tex> <output.md>
"""
import re
import sys

HEADINGS = [
    (r"\section{", "#"),
    (r"\subsection{", "##"),
    (r"\subsubsection{", "###"),
    (r"\paragraph{", "####"),
]
ENV_BLOCKS = ("figure", "figure*", "table", "table*", "align", "align*",
              "equation", "equation*", "thebibliography", "document",
              "abstract", "IEEEkeywords")


def title_of(line, pat):
    i = line.index(pat) + len(pat)
    depth, j = 1, i
    while j < len(line) and depth:
        if line[j] == "{":
            depth += 1
        elif line[j] == "}":
            depth -= 1
        j += 1
    return line[i:j - 1], line[j:]


def main() -> None:
    tex_path, out_path = sys.argv[1], sys.argv[2]
    lines = open(tex_path, encoding="utf-8").read().splitlines()

    out = ["<!-- Structure view of " + tex_path + ": every paragraph wrapped for "
           "sorting and grading. Regenerate with paper/structure_extract.py. -->", ""]
    stack = []           # [(level, name)]
    counter = {}         # name-prefix -> count
    para_buf, para_start = [], None
    in_doc = False
    env, env_start = None, None
    env_lines = []
    n = 0

    def sec_name():
        return ".".join(str(x[1]) for x in stack) or "0"

    def flush(end_line):
        nonlocal para_buf, para_start, n
        text = " ".join(t.strip() for t in para_buf).strip()
        text = re.sub(r"%.*$", "", text).strip()
        if text:
            n += 1
            name = f"p-{sec_name()}-{n}"
            out.append(f'<p x-name="{name}" x-tex="{tex_path}:{para_start}-{end_line}" x-labels="">')
            out.append("")
            out.append(text)
            out.append("")
            out.append("</p>")
            out.append("")
        para_buf, para_start = [], None

    for idx, raw in enumerate(lines, 1):
        line = raw.rstrip()
        if not in_doc:
            if line.startswith(r"\begin{document}"):
                in_doc = True
            continue
        m = re.match(r"\\begin\{(" + "|".join(ENV_BLOCKS) + r"\*?)\}", line)
        if env:
            if env in ("abstract", "IEEEkeywords"):
                if re.match(r"\\end\{" + re.escape(env) + r"\}", line):
                    text = re.sub(r"%.*$", "", " ".join(t.strip() for t in para_buf)).strip()
                    n += 1
                    name = f"p-{sec_name()}-{n}"
                    out.append(f'<p x-name="{name}" x-tex="{tex_path}:{env_start}-{idx}" x-labels="environment {env}">')
                    out.append("")
                    out.append(text)
                    out.append("")
                    out.append("</p>")
                    out.append("")
                    env, para_buf = None, []
                else:
                    para_buf.append(line)
                continue
            if re.match(r"\\end\{" + re.escape(env) + r"\}", line):
                # A figure or table carries prose in its caption; the caption is
                # the gradable paragraph, the TikZ/pgfplots body is the block.
                blob = "\n".join(env_lines)
                cm = re.search(r"\\caption\{", blob)
                if cm:
                    i, depth, j = cm.end(), 1, cm.end()
                    while j < len(blob) and depth:
                        if blob[j] == "{":
                            depth += 1
                        elif blob[j] == "}":
                            depth -= 1
                        j += 1
                    caption = re.sub(r"\s+", " ", blob[i:j - 1]).strip()
                    n += 1
                    name = f"p-{sec_name()}-{n}"
                    out.append(f'<p x-name="{name}" x-tex="{tex_path}:{env_start}-{idx}" x-labels="environment {env} caption">')
                    out.append("")
                    out.append(caption)
                    out.append("")
                    out.append("</p>")
                    out.append("")
                n += 1
                name = f"p-{sec_name()}-{n}"
                out.append(f'<p x-name="{name}" x-tex="{tex_path}:{env_start}-{idx}" x-labels="environment {env}">')
                out.append("")
                out.append(f"[{env} block" + (", caption above]" if cm else "]"))
                out.append("")
                out.append("</p>")
                out.append("")
                env, env_lines = None, []
            else:
                env_lines.append(line)
            continue
        if m:
            flush(idx - 1)
            env = m.group(1).rstrip("*")
            env_start = idx
            continue
        hit = None
        for pat, hashes in HEADINGS:
            if line.lstrip().startswith(pat):
                hit = (pat, hashes)
                break
        if hit:
            flush(idx - 1)
            title, _rest = title_of(line.lstrip(), hit[0])
            level = len(hit[1])
            while stack and stack[-1][0] >= level:
                stack.pop()
            parent = ".".join(str(x[1]) for x in stack)
            count_key = (parent, level)
            counter[count_key] = counter.get(count_key, 0) + 1
            stack.append((level, counter[count_key]))
            n = 0
            out.append(f'{hit[1]} {title}  <!-- sec-{sec_name()} -->')
            out.append("")
            continue
        if line.strip() == "":
            flush(idx - 1)
        else:
            if para_start is None:
                para_start = idx
            para_buf.append(line)
    flush(len(lines))

    open(out_path, "w", encoding="utf-8").write("\n".join(out) + "\n")
    print(f"{sum(1 for x in out if x.startswith('<p '))} paragraphs")


if __name__ == "__main__":
    main()