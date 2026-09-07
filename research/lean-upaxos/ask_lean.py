import json
import os
import re
import sys
import urllib.error
import urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))
ENV_PATH = "/Users/Shared/lua-lunet/vrr-core/.env"


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


PROMPT = r"""Write a complete self-contained Lean 4 (v4.33.1) file using ONLY the Lean core library that compiles with plain `lean` (no Mathlib, no imports beyond none needed; do NOT use `Finset` — it is not in this core).

Scope: UPaxos era/quorum structure, baby step (structure only, no liveness). Model quorums as `List Node` and overlap as decidable List computation so `decide` can close concrete facts.

Give exactly these definitions and theorems; output exactly one fenced ```lean code block with the full file and no prose outside it:

1.
```lean
abbrev Era := Nat
def eOf (b : Era × Nat) : Era := b.1
abbrev Node := Nat
structure Config where
  QI : List Node
  QII : List Node
def Overlap (Q1 Q2 : List Node) : Prop := Q1.any (fun x => x ∈ Q2) = true
```

2. Structural-invariant plumbing (trivial application lemmas):
```lean
theorem overlap_same (C : Era → Config) (e : Era) (h : ∀ e, Overlap (C e).QI (C e).QII) :
    Overlap (C e).QI (C e).QII := h e
theorem overlap_cross (C : Era → Config) (e : Era)
    (h : ∀ e, Overlap (C (e+1)).QI (C e).QII) :
    Overlap (C (e+1)).QI (C e).QII := h e
```

3. Concrete 3-zone upgrade instance (zones A,B,C are nodes 0,1,2; node 3 is the temporarily added Z):
```lean
def cfgE : Era → Config
  | 0 => ⟨[0, 1, 2], [0, 1, 2]⟩
  | 1 => ⟨[0, 1, 3], [0, 2, 3]⟩
  | _ => ⟨[0, 1, 3], [0, 1, 3]⟩
```
Prove each of the following with `by decide` (they must all be closed by `decide` alone or `decide` after unfolding; if `decide` alone fails, use `by simp [cfgE]` then `decide`, or explicit witnesses):
- same-era: Overlap (cfgE 0).QI (cfgE 0).QII, same for 1, 2
- adjacent cross-era (both directions): (cfgE 0).QI ∩ (cfgE 1).QII, (cfgE 1).QI ∩ (cfgE 0).QII, (cfgE 1).QI ∩ (cfgE 2).QII, (cfgE 2).QI ∩ (cfgE 1).QII
- also (cfgE 0).QI with (cfgE 2).QII

4. Leader-casting-vote micro-fact:
```lean
def FixableIn (b : Era × Nat) (s : Era) : Prop := s = b.1 ∨ s = b.1 + 1
theorem fixable_le (b : Era × Nat) (s : Era) (h : FixableIn b s) : b.1 ≤ s := by
  rcases h with h | h
  · exact h ▸ Nat.le_refl b.1
  · exact h ▸ Nat.le_succ b.1
```

5. Negative control: a config with disjoint quorums where Overlap FAILS:
```lean
def bad : Config := ⟨[0], [1]⟩
theorem bad_no_overlap : ¬ Overlap bad.QI bad.QII := by decide
```

Everything must compile with Lean core only. Prefer `by decide` for the concrete instances. If `decide` cannot reduce `List.any` membership, restructure `Overlap` so it can (e.g. keep the ∃-form as a separate non-decidable statement). No ` sorry`, no `axiom`, no `native_decide`."""


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
    with urllib.request.urlopen(req, timeout=240) as resp:
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

with open(os.path.join(ROOT, "candidate.lean"), "w") as f:
    f.write(blocks[0].strip() + "\n")
print("ok: %d block(s), saved first" % len(blocks))
