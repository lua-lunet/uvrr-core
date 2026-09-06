import json
import os
import re
import sys
import urllib.error
import urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))


def get_key():
    key = os.environ.get("MISTRAL_API_KEY")
    if key:
        return key.strip()
    sys.exit("MISTRAL_API_KEY not in environment")


PROMPT = (
    "Write a complete Lean 4 file (Lean 4.33.1, core library only, no Mathlib, "
    "no native_decide) proving these theorems:\n"
    "theorem three_mul_four : (3 : Nat) * 4 = 12\n"
    "theorem add_comm_small (a b : Nat) : a + b = b + a  (prove for a=6, b=7 by decide, or generally with omega)\n"
    "theorem seven_lt_nine : (7 : Nat) < 9\n"
    "Use tactics (rfl, decide, omega). The file must be self-contained and compile "
    "as-is with plain `lean`. Output exactly one fenced lean code block containing "
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
    with urllib.request.urlopen(req, timeout=180) as resp:
        body = resp.read().decode()
except urllib.error.HTTPError as e:
    sys.exit(f"HTTP {e.code}")
except Exception:
    sys.exit("request failed")

data = json.loads(body)
content = data["choices"][0]["message"]["content"]

blocks = re.findall(r"```(?:lean|Lean)?\s*\n(.*?)```", content, re.DOTALL)
if not blocks:
    sys.exit("no fenced code block in response")

with open(os.path.join(ROOT, "leanstral_api.lean"), "w") as f:
    f.write(blocks[0].strip() + "\n")
print("ok")
