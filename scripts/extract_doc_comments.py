import os

roots = ["src", "tests", "examples"]
out = []
for root in roots:
    for dirpath, _, files in os.walk(root):
        for name in sorted(files):
            if not name.endswith(".rs"):
                continue
            path = os.path.join(dirpath, name)
            with open(path, encoding="utf-8") as fh:
                lines = fh.read().splitlines()
            block = []
            start = None
            for i, line in enumerate(lines, 1):
                t = line.lstrip()
                content = None
                if t.startswith("///"):
                    content = t[3:]
                elif t.startswith("//!"):
                    content = t[3:]
                if content is None:
                    if block:
                        out.append(f"=== {path}:{start} ===")
                        out.extend(block)
                        out.append("")
                        block = []
                        start = None
                    continue
                if content.startswith(" "):
                    content = content[1:]
                if start is None:
                    start = i
                block.append(content)
            if block:
                out.append(f"=== {path}:{start} ===")
                out.extend(block)
                out.append("")

with open(".tmp/doc_comments.txt", "w", encoding="utf-8") as fh:
    fh.write("\n".join(out))
print("blocks=%d lines=%d" % (sum(1 for l in out if l.startswith("===")), len(out)))
