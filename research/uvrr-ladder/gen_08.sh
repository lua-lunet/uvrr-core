#!/usr/bin/env bash
# usage: gen_08.sh "<verdict paragraph>"  — copies Leanstral's Weights.lean into the project and builds ladder/08
set -u
cd /Users/Shared/lua-lunet/vrr-core/formal/uvrr-lean || exit 1
export PATH=$HOME/.elan/bin:$HOME/.local/bin:$PATH
mkdir -p leanstral
cp /Users/Shared/lua-lunet/vrr-core/.tmp/uvrr-ladder/leanstral/Weights.lean leanstral/Lemma2-leanstral.lean
cp /Users/Shared/lua-lunet/vrr-core/.tmp/uvrr-ladder/leanstral/PROMPT.txt leanstral/PROMPT.txt
doc=ladder/08-leanstral-lemma2.md; rm -f "$doc"
showboat init "$doc" "Rung 8 (experiment): Leanstral drafts the general weighted-majority Lemma 2" >/dev/null
showboat note "$doc" "**Experiment, not a rung of the safety argument.** The paper's Lemma 2 says: if two weight functions, scaled by positive integers k and k', differ in total by at most 1 across all nodes, then every weighted majority under one meets every weighted majority under the other. Rung 7 checks the concrete 4-server schedule; this file asks Leanstral (Mistral's Lean model, driven through the user's vibe fork as the 'lean' agent, headless with --trust and --auto-approve) to prove the general statement. The statement and definitions were fixed in advance; the model was only allowed to fill the proof and add helper lemmas.

$1" >/dev/null
showboat exec "$doc" bash "cat leanstral/PROMPT.txt" >/dev/null
showboat exec "$doc" bash "cat leanstral/Lemma2-leanstral.lean" >/dev/null
showboat exec "$doc" bash "lake env lean leanstral/Lemma2-leanstral.lean 2>&1 | head -20; echo exit=\${PIPESTATUS[0]}" >/dev/null
showboat exec "$doc" bash "grep -c 'sorry' leanstral/Lemma2-leanstral.lean; grep -c 'native_decide' leanstral/Lemma2-leanstral.lean; true" >/dev/null
showboat note "$doc" "**Reading the output.** exit=0 with sorry count 1 means only the word in the header comment remains and the theorem is proved; any nonzero exit or a higher sorry count means the general lemma is NOT established and the ladder does not depend on it." >/dev/null
showboat verify "$doc" >/dev/null 2>&1 && echo VERIFY_OK || echo VERIFY_FAIL
