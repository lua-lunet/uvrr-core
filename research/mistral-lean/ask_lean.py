import json
import os
import re
import sys
import urllib.error
import urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))
ENV_PATH = os.path.join(os.path.dirname(os.path.dirname(ROOT)), ".env")


def get_key():
    key = os.environ.get("MISTRAL_API_KEY")
    if key:
        return key.strip()
    if os.path.isfile(ENV_PATH):
        with open(ENV_PATH) as f:
            for line in f:
                m = re.match(r"\s*(?:export\s+)?MISTRAL_API_KEY\s*=\s*(.+)\s*$", line)
                if m:
                    return m.group(1).strip().strip('"').strip("'")
    sys.exit("MISTRAL_API_KEY not found")


PROMPT = (
    "Write a complete Lean 4 file that proves the following theorem using tactics:\n"
    "theorem two_plus_two : (2 : Nat) + 2 = 4\n"
    "The file must be self-contained, compile as-is with plain `lean`, and use only "
    "the Lean 4 core library. Output exactly one fenced lean code block containing "
    "the full file, with no prose outside the block."
)

payload = {
    "model": "labs-leanstral-1-5-1",
    "temperature": 0,
    "top_p": 1,
    "messages": [{"role": "user", "content": PROMPT}],
}

req = urllib.request.Request(
    "https://api.mistral.ai/v1/chat/completions",
    data=json.dumps(payload).encode(),
    headers={
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": "Bearer " + get_key(),
    },
)

try:
    with urllib.request.urlopen(req, timeout=120) as resp:
        body = resp.read().decode()
except urllib.error.HTTPError as e:
    sys.exit(f"HTTP {e.code}: {e.read().decode()[:500]}")
except Exception as e:
    sys.exit(f"request failed: {e}")

with open(os.path.join(ROOT, "response.json"), "w") as f:
    f.write(body)

data = json.loads(body)
content = data["choices"][0]["message"]["content"]
with open(os.path.join(ROOT, "raw_content.txt"), "w") as f:
    f.write(content)

blocks = re.findall(r"```(?:lean|Lean)?\s*\n(.*?)```", content, re.DOTALL)
if not blocks:
    sys.exit("no fenced code block in response")
if len(blocks) > 1:
    sys.exit(f"expected one code block, got {len(blocks)}")

with open(os.path.join(ROOT, "candidate.lean"), "w") as f:
    f.write(blocks[0].strip() + "\n")
print("ok")
