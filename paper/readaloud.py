#!/usr/bin/env python3
"""Render a paper .tex as read-aloud markdown: one column, figures as their
captions, tables as markdown tables, citations as [n] in bibliography order.
The converter is deliberately strict: an unsupported command or environment
is an error with context, never silently dropped text.

Usage: readaloud.py IN.tex OUT.md
"""
import re
import sys


def fail(msg, s=None, i=None):
    if s is not None and i is not None:
        ctx = s[max(0, i - 60):i + 60].replace("\n", " ")
        msg = f"{msg} near: ...{ctx}..."
    raise SystemExit(f"readaloud: {msg}")


def strip_comments(s):
    out, i, n = [], 0, len(s)
    while i < n:
        if s[i] == "\\" and i + 1 < n:
            out.append(s[i:i + 2])
            i += 2
            continue
        if s[i] == "%":
            while i < n and s[i] != "\n":
                i += 1
            continue
        out.append(s[i])
        i += 1
    return "".join(out)


def skip_ws(s, i):
    while i < len(s) and s[i] in " \t\n":
        i += 1
    return i


def read_group(s, i):
    i = skip_ws(s, i)
    if i >= len(s) or s[i] != "{":
        fail("expected '{'", s, i)
    depth, j = 0, i
    while j < len(s):
        c = s[j]
        if c == "\\":
            j += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return s[i + 1:j], j + 1
        j += 1
    fail("unclosed '{'", s, i)


def read_optarg(s, i):
    i = skip_ws(s, i)
    if i < len(s) and s[i] == "[":
        depth, j = 0, i
        while j < len(s):
            if s[j] == "\\":
                j += 2
                continue
            if s[j] == "[":
                depth += 1
            elif s[j] == "]":
                depth -= 1
                if depth == 0:
                    return s[i + 1:j], j + 1
            j += 1
        fail("unclosed '['", s, i)
    return None, i


def env_body(s, i, name):
    pat = re.compile(r"\\(begin|end)\{" + re.escape(name) + r"\}")
    depth = 1
    for m in pat.finditer(s, i):
        if m.group(1) == "begin":
            depth += 1
        else:
            depth -= 1
            if depth == 0:
                return s[i:m.start()], m.end()
    fail("unclosed environment " + name, s, i)


def split_top(s, sep):
    parts, cur, depth, i, n = [], [], 0, 0, len(s)
    while i < n:
        c = s[i]
        if c == "\\":
            if sep == "\\\\" and s.startswith("\\\\", i) and depth == 0:
                parts.append("".join(cur))
                cur = []
                i += 2
                continue
            if sep == "\\item" and depth == 0 and s.startswith("\\item", i):
                parts.append("".join(cur))
                cur = []
                i += 5
                continue
            cur.append(s[i:i + 2])
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        if sep == "&" and c == "&" and depth == 0:
            parts.append("".join(cur))
            cur = []
            i += 1
            continue
        cur.append(c)
        i += 1
    parts.append("".join(cur))
    return parts


def roman(n):
    vals = [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
    out = ""
    for v, r in vals:
        while n >= v:
            out += r
            n -= v
    return out


def normalize(t):
    return re.sub(r"\s+", " ", t).strip()


def textify(t):
    t = t.replace("---", "—").replace("--", "–")
    t = t.replace("``", "“").replace("''", "”")
    return t


MATH_GROUPS = ("mathcal", "mathbb", "mathrm", "mathit", "mathbf", "mathsf",
               "mathtt", "text", "textrm", "textbf", "textit")
MATH_SYMBOLS = [
    ("textemdash", "—"), ("textendash", "–"),
    ("varnothing", "∅"), ("emptyset", "∅"), ("setminus", "∖"),
    ("longrightarrow", "⟶"), ("rightarrow", "→"), ("Rightarrow", "⇒"),
    ("leftarrow", "←"), ("mapsto", "↦"), ("notin", "∉"),
    ("subseteq", "⊆"), ("subset", "⊂"), ("supseteq", "⊇"), ("supset", "⊃"),
    ("times", "×"), ("cdot", "·"), ("bullet", "•"), ("circ", "∘"),
    ("infty", "∞"), ("varphi", "φ"), ("alpha", "α"), ("beta", "β"),
    ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("lambda", "λ"),
    ("sigma", "σ"), ("omega", "ω"), ("forall", "∀"), ("exists", "∃"),
    ("approx", "≈"), ("simeq", "≃"), ("propto", "∝"), ("equiv", "≡"),
    ("preceq", "⪯"), ("succeq", "⪰"), ("prec", "≺"), ("succ", "≻"),
    ("langle", "⟨"), ("rangle", "⟩"), ("ldots", "…"), ("cdots", "⋯"),
    ("dotsc", "…"), ("perp", "⊥"), ("land", "∧"), ("lor", "∨"),
    ("sum", "∑"), ("prod", "∏"), ("int", "∫"), ("ell", "ℓ"),
    ("phi", "φ"), ("prime", "′"), ("ast", "∗"), ("div", "÷"),
    ("cap", "∩"), ("cup", "∪"), ("neq", "≠"), ("geq", "≥"), ("leq", "≤"),
    ("mid", "|"), ("neg", "¬"), ("top", "⊤"), ("bot", "⊥"),
    ("sim", "∼"), ("mu", "μ"), ("tau", "τ"), ("pi", "π"), ("pm", "±"),
    ("ne", "≠"), ("ge", "≥"), ("le", "≤"), ("in", "∈"), ("to", "→"),
]
MATH_DROP = ("left", "right", "big", "Big", "bigg", "Bigg", "displaystyle",
             "limits", "quad", "qquad")


class Conv:
    def __init__(self, bibmap, mathmacros):
        self.bibmap = bibmap
        self.mathmacros = mathmacros
        self.labels = {}
        self.final = False
        self.buf, self.blocks, self.inline = [], [], False
        self.anchor = None
        self._reset_counters()

    def _reset_counters(self):
        self.sec = self.subsec = self.fig = self.tab = self.eqn = 0
        self.thms = {}

    def mathlike(self, t):
        t = t.replace("\\{", "\x01").replace("\\}", "\x02")
        for name, body in self.mathmacros.items():
            t = re.sub(r"\\" + name + r"(?![A-Za-z])", lambda m: body, t)
        for name, sym in MATH_SYMBOLS:
            t = re.sub(r"\\" + name + r"(?![A-Za-z])", sym, t)
        for name in MATH_DROP:
            t = re.sub(r"\\" + name + r"(?![A-Za-z])\s*", "", t)
        group_re = re.compile(
            r"\\(?:" + "|".join(MATH_GROUPS) + r")\s*\{([^{}]*)\}")
        prev = None
        while prev != t:
            prev = t
            t = group_re.sub(r"\1", t)
        t = re.sub(r"\\(?:" + "|".join(MATH_GROUPS) + r")\s+([A-Za-z])",
                   r"\1", t)
        t = re.sub(r"\\[,;:! ]", " ", t)
        prev = None
        while prev != t:
            prev = t
            t = re.sub(r"_\{([^{}]*)\}", r"\1", t)
            t = re.sub(r"\^\{([^{}]*)\}", r"^\1", t)
        t = re.sub(r"_(\w)", r"\1", t)
        t = re.sub(r"\{([^{}]*)\}", r"\1", t)
        t = t.replace("{", "").replace("}", "")
        t = t.replace("\x01", "{").replace("\x02", "}")
        for c in "%$&#_":
            t = t.replace("\\" + c, c)
        m = re.search(r"\\([A-Za-z]+)", t)
        if m:
            fail("unsupported math macro \\" + m.group(1) + " in: " + t)
        return normalize(t)

    def render_inline(self, s):
        saved = (self.buf, self.blocks, self.inline)
        self.buf, self.blocks, self.inline = [], None, True
        self._walk(s)
        out = normalize("".join(self.buf))
        self.buf, self.blocks, self.inline = saved
        return out

    def render_blocks(self, s):
        saved = (self.buf, self.blocks, self.inline)
        self.buf, self.blocks, self.inline = [], [], False
        self._walk(s)
        self._flush()
        out = self.blocks
        self.buf, self.blocks, self.inline = saved
        return out

    def _flush(self):
        if self.inline:
            self.buf.append(" ")
            return
        text = normalize("".join(self.buf))
        if text:
            self.blocks.append(text)
        self.buf = []

    def _need_block(self, what, s, i):
        if self.inline:
            fail(what + " is not inline content", s, i)

    def _walk(self, s):
        i, n = 0, len(s)
        while i < n:
            c = s[i]
            if c == "\\":
                i = self._command(s, i)
            elif c == "$":
                j = s.find("$", i + 1)
                if j < 0:
                    fail("unclosed '$'", s, i)
                self.buf.append(self.mathlike(s[i + 1:j]))
                i = j + 1
            elif c == "\n":
                if self.inline:
                    self.buf.append(" ")
                    i += 1
                else:
                    m = re.match(r"\n[ \t\n]*\n", s[i:])
                    if m:
                        self._flush()
                        i += m.end()
                    else:
                        self.buf.append(" ")
                        i += 1
            elif c in "{}":
                i += 1
            elif c == "~":
                self.buf.append(" ")
                i += 1
            elif c == "&":
                fail("stray '&'", s, i)
            else:
                j = i
                while j < n and s[j] not in "\\$\n{}&~":
                    j += 1
                self.buf.append(textify(s[i:j]))
                i = j

    def _command(self, s, i):
        j = i + 1
        if j >= len(s):
            fail("lone backslash", s, i)
        if not s[j].isalpha():
            sym = s[j]
            table = {"&": "&", "%": "%", "$": "$", "#": "#", "_": "_",
                     "{": "{", "}": "}", " ": " ", ",": "", ";": " ",
                     "!": "", "-": "", "\\": " ", "'": "'"}
            if sym not in table:
                fail("unsupported control symbol \\" + sym, s, i)
            self.buf.append(table[sym])
            return j + 1
        k = j
        while k < len(s) and s[k].isalpha():
            k += 1
        name = s[j:k]
        handler = getattr(self, "cmd_" + name, None)
        if handler is None:
            fail("unsupported command \\" + name, s, i)
        return handler(s, k, i)

    # --- inline commands -------------------------------------------------
    def _wrap(self, s, k, pre, post, raw=False):
        g, j = read_group(s, k)
        self.buf.append(pre + (g if raw else self.render_inline(g)) + post)
        return j

    def cmd_emph(self, s, k, i):
        return self._wrap(s, k, "*", "*")

    def cmd_textit(self, s, k, i):
        return self._wrap(s, k, "*", "*")

    def cmd_textbf(self, s, k, i):
        return self._wrap(s, k, "**", "**")

    def cmd_textsc(self, s, k, i):
        g, j = read_group(s, k)
        self.buf.append(self.render_inline(g))
        return j

    def cmd_code(self, s, k, i):
        return self._wrap(s, k, "`", "`", raw=True)

    def cmd_texttt(self, s, k, i):
        return self._wrap(s, k, "`", "`", raw=True)

    def cmd_url(self, s, k, i):
        g, j = read_group(s, k)
        self.buf.append(g.strip())
        return j

    def cmd_href(self, s, k, i):
        _url, j = read_group(s, k)
        g, j = read_group(s, j)
        self.buf.append(self.render_inline(g))
        return j

    def cmd_cite(self, s, k, i):
        opt, j = read_optarg(s, k)
        g, j = read_group(s, j)
        if not self.final:
            return j
        nums = []
        for key in g.split(","):
            key = key.strip()
            if key not in self.bibmap:
                fail("unknown citation key " + key, s, i)
            nums.append(self.bibmap[key])
        if opt is not None:
            note = self.render_inline(opt)
            self.buf.append("[" + ", ".join(str(x) for x in nums)
                            + ", " + note + "]")
        else:
            self.buf.append(", ".join("[" + str(x) + "]" for x in nums))
        return j

    def cmd_ref(self, s, k, i):
        g, j = read_group(s, k)
        if self.final:
            if g not in self.labels:
                fail("unresolved \\ref{" + g + "}", s, i)
            self.buf.append(self.labels[g])
        return j

    def cmd_eqref(self, s, k, i):
        g, j = read_group(s, k)
        if self.final:
            if g not in self.labels:
                fail("unresolved \\eqref{" + g + "}", s, i)
            self.buf.append("(" + self.labels[g] + ")")
        return j

    def cmd_label(self, s, k, i):
        g, j = read_group(s, k)
        if self.anchor is not None:
            self.labels[g] = self.anchor
        return j

    def cmd_includegraphics(self, s, k, i):
        _opt, j = read_optarg(s, k)
        _g, j = read_group(s, j)
        return j

    def cmd_thanks(self, s, k, i):
        _g, j = read_group(s, k)
        return j

    def cmd_textendash(self, s, k, i):
        self.buf.append("–")
        return k

    def cmd_textemdash(self, s, k, i):
        self.buf.append("—")
        return k

    def cmd_ldots(self, s, k, i):
        self.buf.append("…")
        return k

    def cmd_dots(self, s, k, i):
        self.buf.append("…")
        return k

    def cmd_S(self, s, k, i):
        self.buf.append("§")
        return k

    def cmd_expwarning(self, s, k, i):
        self._need_block("expwarning", s, i)
        self._flush()
        self.blocks.append(
            "> **WARNING: THIS WORK HAS NOT BEEN DONE.** The experiments "
            "below are designs; all numbers and charts are placeholders "
            "(marked “dummy”).")
        return k

    # --- no-op commands ---------------------------------------------------
    def _noop(self, s, k, i):
        return k

    cmd_maketitle = _noop
    cmd_noindent = _noop
    cmd_centering = _noop
    cmd_small = _noop
    cmd_footnotesize = _noop
    cmd_scriptsize = _noop
    cmd_tiny = _noop
    cmd_bfseries = _noop
    cmd_itshape = _noop
    cmd_em = _noop
    cmd_normalfont = _noop
    cmd_raggedbottom = _noop

    def cmd_thispagestyle(self, s, k, i):
        _g, j = read_group(s, k)
        return j

    def cmd_vspace(self, s, k, i):
        j = skip_ws(s, k)
        if j < len(s) and s[j] == "*":
            j += 1
        _g, j = read_group(s, j)
        return j

    cmd_hspace = cmd_vspace

    def cmd_par(self, s, k, i):
        self._flush()
        return k

    # --- headings -----------------------------------------------------------
    def cmd_section(self, s, k, i):
        self._need_block("section", s, i)
        g, j = read_group(s, k)
        self._flush()
        self.sec += 1
        self.subsec = 0
        num = roman(self.sec)
        self.anchor = num
        self.blocks.append("## " + num + ". " + self.render_inline(g))
        return j

    def cmd_subsection(self, s, k, i):
        self._need_block("subsection", s, i)
        g, j = read_group(s, k)
        self._flush()
        self.subsec += 1
        letter = chr(ord("A") + self.subsec - 1)
        self.anchor = letter
        self.blocks.append("### " + letter + ". " + self.render_inline(g))
        return j

    def cmd_paragraph(self, s, k, i):
        self._need_block("paragraph", s, i)
        g, j = read_group(s, k)
        self._flush()
        self.buf.append("**" + self.render_inline(g) + ".** ")
        return j

    # --- environments ---------------------------------------------------------
    def cmd_begin(self, s, k, i):
        name, j = read_group(s, k)
        opt, j = read_optarg(s, j)
        body, j = env_body(s, j, name)
        method = getattr(self, "env_" + name.replace("*", "_star"), None)
        if method is None:
            fail("unsupported environment " + name, s, i)
        method(name, opt, body, s, i)
        return j

    def cmd_end(self, s, k, i):
        fail("stray \\end", s, i)

    def env_document(self, name, opt, body, s, i):
        self.blocks.extend(self.render_blocks(body))

    def env_abstract(self, name, opt, body, s, i):
        self._flush()
        self.blocks.append("## Abstract")
        self.blocks.extend(self.render_blocks(body))

    def env_IEEEkeywords(self, name, opt, body, s, i):
        self._flush()
        text = self.render_inline(body).rstrip(".")
        self.blocks.append("*Keywords: " + text + ".*")

    def _figure(self, name, opt, body, s, i):
        self._need_block("figure", s, i)
        self._flush()
        self.fig += 1
        num = str(self.fig)
        self.anchor = num
        cap = self._caption_of(body, s, i)
        self._labels_of(body)
        self.blocks.append("> **Figure " + num + ".** " + cap)

    env_figure = _figure
    env_figure_star = _figure

    def _table(self, name, opt, body, s, i):
        self._need_block("table", s, i)
        self._flush()
        self.tab += 1
        num = str(self.tab)
        self.anchor = num
        cap = self._caption_of(body, s, i)
        self._labels_of(body)
        self.blocks.append("**Table " + num + ".** " + cap)
        idx = body.find("\\begin{tabular}")
        if idx < 0:
            fail("table without tabular", s, i)
        _spec, j = read_group(body, idx + len("\\begin{tabular}"))
        rows_src, _j = env_body(body, j, "tabular")
        self.blocks.append(self._tabular(rows_src, s, i))

    env_table = _table
    env_table_star = _table

    def env_tikzpicture(self, name, opt, body, s, i):
        pass

    def env_center(self, name, opt, body, s, i):
        self.blocks.extend(self.render_blocks(body))

    def env_itemize(self, name, opt, body, s, i):
        self._list(body, s, i, "- ")

    def env_enumerate(self, name, opt, body, s, i):
        self._list(body, s, i, None)

    def _list(self, body, s, i, marker):
        self._need_block("list", s, i)
        self._flush()
        items = [p for p in split_top(body, "\\item") if p.strip()]
        lines = []
        for n, item in enumerate(items, 1):
            lead = marker if marker else f"{n}. "
            lines.append(lead + self.render_inline(item))
        self.blocks.append("\n".join(lines))

    def _numbered_math(self, name, opt, body, s, i):
        self._need_block("math display", s, i)
        self._flush()
        numbered = not name.endswith("*")
        for row in split_top(body, "\\\\"):
            row = row.strip()
            if not row:
                continue
            for m in re.finditer(r"\\label\{([^}]*)\}", row):
                pass
            labels = re.findall(r"\\label\{([^}]*)\}", row)
            row = re.sub(r"\\label\{[^}]*\}", "", row)
            cells = [c for c in split_top(row, "&") if c.strip()]
            text = self.mathlike(" ".join(cells))
            if numbered:
                self.eqn += 1
                num = str(self.eqn)
                self.anchor = num
                for key in labels:
                    self.labels[key] = num
                self.blocks.append(text + "  (" + num + ")")
            else:
                self.blocks.append(text)

    env_align = _numbered_math
    env_align_star = _numbered_math
    env_equation = _numbered_math
    env_equation_star = _numbered_math

    def _theorem(self, name, opt, body, s, i):
        self._need_block("theorem", s, i)
        self._flush()
        base = name.rstrip("*")
        self.thms[base] = self.thms.get(base, 0) + 1
        num = str(self.thms[base])
        self.anchor = num
        title = base.capitalize() + " " + num
        if opt is not None:
            title += " (" + self.render_inline(opt) + ")"
        self.blocks.append("**" + title + ".** " + self.render_inline(body))

    env_theorem = _theorem
    env_lemma = _theorem
    env_proposition = _theorem
    env_corollary = _theorem
    env_definition = _theorem

    def env_proof(self, name, opt, body, s, i):
        self._need_block("proof", s, i)
        self._flush()
        self.blocks.append("**Proof.** " + self.render_inline(body))

    def env_thebibliography(self, name, opt, body, s, i):
        self._need_block("bibliography", s, i)
        self._flush()
        self.blocks.append("## References")
        entries = []
        for n, entry in enumerate(self.bib_entries, 1):
            entries.append(str(n) + ". " + self.render_inline(entry))
        self.blocks.append("\n".join(entries))

    # --- helpers ------------------------------------------------------------
    def _caption_of(self, body, s, i):
        m = re.search(r"\\caption\b", body)
        if not m:
            fail("float without caption", s, i)
        _opt, j = read_optarg(body, m.end())
        g, _j = read_group(body, j)
        return self.render_inline(g)

    def _labels_of(self, body):
        for m in re.finditer(r"\\label\{([^}]*)\}", body):
            if self.anchor is not None:
                self.labels[m.group(1)] = self.anchor

    def _tabular(self, rows_src, s, i):
        grid = []
        for row in split_top(rows_src, "\\\\"):
            row = re.sub(r"\\(?:top|mid|bottom)rule\b", "", row).strip()
            if not row:
                continue
            cells = [self.render_inline(c.strip())
                     for c in split_top(row, "&")]
            grid.append(cells)
        if not grid:
            fail("empty tabular", s, i)
        ncol = max(len(r) for r in grid)
        for r in grid:
            r.extend([""] * (ncol - len(r)))
        lines = ["| " + " | ".join(grid[0]) + " |",
                 "|" + "---|" * ncol]
        lines += ["| " + " | ".join(r) + " |" for r in grid[1:]]
        return "\n".join(lines)


def parse_preamble(pre):
    macros = {}
    for m in re.finditer(r"\\newcommand\{(\\[A-Za-z]+)\}(?!\[)", pre):
        body, _j = read_group(pre, m.end())
        macros[m.group(1)[1:]] = body
    title = ""
    m = re.search(r"\\title\s*\{", pre)
    if m:
        title, _j = read_group(pre, m.end() - 1)
    author, thanks = "", ""
    m = re.search(r"\\author\s*\{", pre)
    if m:
        raw, _j = read_group(pre, m.end() - 1)
        t = re.search(r"\\thanks\s*\{", raw)
        if t:
            thanks, _j = read_group(raw, t.end() - 1)
            author = raw[:t.start()] + raw[_j:]
        else:
            author = raw
    return macros, title, author, thanks


def parse_bibliography(body):
    m = re.search(r"\\begin\{thebibliography\}", body)
    if not m:
        return {}, []
    bib_body, _j = env_body(body, m.end(), "thebibliography")
    opt, j = read_optarg(bib_body, 0)
    if opt is None:
        _g, j = read_group(bib_body, 0)
    bib_body = bib_body[j:]
    keys, entries = [], []
    for part in re.split(r"\\bibitem\b", bib_body)[1:]:
        part = part.lstrip()
        key, j = read_group(part, 0)
        keys.append(key)
        entries.append(part[j:].strip())
    return {k: n + 1 for n, k in enumerate(keys)}, entries


def expand_weight_table(s):
    marker = "\\uwrweightstable"
    idx = s.find(marker)
    if idx < 0:
        return s
    body, j = read_group(s, idx + len(marker))
    rows = []
    pos = 0
    while True:
        r = body.find("\\uwrweightrow", pos)
        if r < 0:
            break
        cells = []
        p = r + len("\\uwrweightrow")
        for _ in range(5):
            g, p = read_group(body, p)
            cells.append("\\textendash{}" if g.strip() == "-" else g)
        rows.append(" & ".join(cells) + " \\\\")
        pos = p
    tabular = ("\\begin{tabular}{lcccc}\n"
               "\\toprule Era & A & B & C & D \\\\\n\\midrule\n"
               + "\n".join(rows) + "\n\\bottomrule\n\\end{tabular}")
    return s[:idx] + tabular + expand_weight_table(s[j:])


def convert(tex_path, out_path):
    src = strip_comments(open(tex_path).read())
    doc = src.find("\\begin{document}")
    if doc < 0:
        fail("no document environment in " + tex_path)
    end = src.find("\\end{document}")
    macros, title, author, thanks = parse_preamble(src[:doc])
    body = src[doc + len("\\begin{document}"):end]
    body = expand_weight_table(body)
    bibmap, bib_entries = parse_bibliography(body)

    conv = Conv(bibmap, macros)
    conv.bib_entries = bib_entries
    conv.render_blocks(body)  # numbering pass
    conv._reset_counters()
    conv.blocks = []
    conv.final = True
    blocks = conv.render_blocks(body)

    stem = re.sub(r"\.md$", "", out_path.rsplit("/", 1)[-1])
    m = re.search(r"(\d{8}-[0-9a-f]{6,})$", stem)
    note = ("> Read-aloud render of the two-column PDF `" + stem + ".pdf`"
            + (" (version of record `" + m.group(1) + "`)" if m else "")
            + ": the same content reflowed to a single column; figures are "
              "represented by their captions and tables as markdown tables, "
              "suitable for listening and for later rendering as HTML.")

    head = ["# " + conv.render_inline(title)]
    byline = conv.render_inline(author)
    if thanks:
        byline += " — " + conv.render_inline(thanks)
    head += [byline, note]

    out = "\n\n".join(head + blocks) + "\n"
    with open(out_path, "w") as f:
        f.write(out)
    print(f"wrote {out_path} ({len(blocks)} blocks)")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: readaloud.py IN.tex OUT.md")
    convert(sys.argv[1], sys.argv[2])
