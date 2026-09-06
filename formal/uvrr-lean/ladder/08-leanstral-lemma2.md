# Rung 8 (experiment): Leanstral drafts the general weighted-majority Lemma 2

*2026-09-05T23:50:16Z by Showboat 0.6.1*
<!-- showboat-id: be91b123-e539-4f6d-906a-d265ddf465f4 -->

**Experiment, not a rung of the safety argument.** The paper's Lemma 2 says: if two weight functions, scaled by positive integers k and k', differ in total by at most 1 across all nodes, then every weighted majority under one meets every weighted majority under the other. Rung 7 checks the concrete 4-server schedule; this file asks Leanstral (Mistral's Lean model, driven through the user's vibe fork as the 'lean' agent, headless with --trust and --auto-approve) to prove the general statement. The statement and definitions were fixed in advance; the model was only allowed to fill the proof and add helper lemmas.

**Outcome (2026-09-06).** Two headless launches. Launch 1 (no --trust) produced no output for ten minutes: vibe was blocked on its working-directory trust prompt with stdout redirected; killed. Launch 2 (--trust, streaming) ran 17 minutes, read the toolchain, and wrote a 105-line draft with the right proof plan (sums distribute over append; a sublist's weighted sum is at most the total; two disjoint sublists of a duplicate-free list together sum to at most the total; the single node where the scaled weights differ yields the contradiction). The draft does not compile: it uses the Mathlib keyword lemma and Mathlib names (add_le_add_left, List.sum_append_nat) that do not exist in Lean core, exactly the Mathlib regression seen in the earlier API experiment. One of its Lean checks became a runaway process holding about 30 GB of memory, which forced the session to be stopped. Verdict: FAILED within the timebox; the general Lemma 2 is not established, and nothing in Rungs 1-7 depends on it.

```bash
cat leanstral/PROMPT.txt
```

```output
You are in a directory containing Weights.lean. Replace the single `sorry` in `theorem majOverlap` with a complete proof, using ONLY Lean 4.33.1 core (no imports at all, no Mathlib/Batteries, no native_decide, no sorry, no axiom). You may add helper lemmas before the theorem but must not change the theorem statement or the definitions. Check your work by running exactly: ~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean Weights.lean  and iterate until it exits 0 with no errors. Hints: scale both majorities by k and k' respectively; if q and q' were disjoint then wsum over q ++ q' would be a sublist sum bounded by the total; the single node where the scaled weights differ by 1 gives the contradiction. Useful core lemmas: List.Sublist.sum_le_sum, List.map_append, List.sum_append, List.Nodup, List.Sublist. Write the final file in place and stop when it compiles.
```

```bash
cat leanstral/Lemma2-leanstral.lean
```

```output
/-
  UPaxos paper, Appendix A, Lemma 2 (weighted majorities overlap).
  Lean 4.33.1 core only: NO Mathlib, NO Batteries, NO `import`, NO `native_decide`,
  NO `sorry`, NO `axiom`. Fill in the proof of `majOverlap` below. You may add
  helper lemmas above it. Do not change the statement.
-/

/-- Weighted sum of a list of nodes. -/
def wsum {A : Type} (w : A → Nat) (q : List A) : Nat := (q.map w).sum

/-- `q` is a weighted majority of `nodes` under weight function `w`:
    it is a sublist of `nodes` and carries strictly more than half the total weight. -/
def Maj {A : Type} (nodes : List A) (w : A → Nat) (q : List A) : Prop :=
  q.Sublist nodes ∧ 2 * wsum w q > wsum w nodes

/-- |k'·w'(a) − k·w(a)| as a Nat. -/
def wdiff {A : Type} (w w' : A → Nat) (k k' : Nat) (a : A) : Nat :=
  if k * w a ≤ k' * w' a then k' * w' a - k * w a else k * w a - k' * w' a

/-- Helper: wsum distributes over append. -/
lemma wsum_append {A : Type} (w : A → Nat) (l₁ l₂ : List A) :
    wsum w (l₁ ++ l₂) = wsum w l₁ + wsum w l₂ := by
  simp [wsum, List.map_append, List.sum_append_nat]

/-- Helper: wsum is monotone with respect to Sublist for non-negative weights. -/
lemma wsum_sublist_le {A : Type} (w : A → Nat) {l nodes : List A} (h : l <+ nodes) :
    wsum w l ≤ wsum w nodes := by
  induction h with
  | slnil => simp [wsum]
  | cons a h ih =>
    simp [wsum]
    exact add_le_add_left ih _
  | cons_cons a h ih =>
    simp [wsum]
    exact add_le_add_left ih _

/-- If two disjoint sublists of a Nodup list are combined, the sum of the
    combined list is at most the sum of the original list. -/
lemma sum_two_disjoint_sublists_le {A : Type} [DecidableEq A] (f : A → Nat)
    {l₁ l₂ nodes : List A} (hnd : nodes.Nodup)
    (h₁ : l₁ <+ nodes) (h₂ : l₂ <+ nodes)
    (hdisj : ∀ a, a ∈ l₁ → a ∉ l₂) :
    (l₁.map f).sum + (l₂.map f).sum ≤ (nodes.map f).sum := by
  induction nodes generalizing l₁ l₂ with
  | nil =>
    have hl₁ : l₁ = [] := by
      cases l₁ <;> cases h₁ <;> rfl
    have hl₂ : l₂ = [] := by
      cases l₂ <;> cases h₂ <;> rfl
    simp [hl₁, hl₂]
  | cons a nodes IH =>
    -- We need to consider whether a ∈ l₁ or a ∈ l₂ or neither
    by_cases ha₁ : a ∈ l₁
    · -- a ∈ l₁, so l₁ = a :: l₁' where l₁' <+ nodes
      have h₁' : ∃ l₁', l₁ = a :: l₁' ∧ l₁' <+ nodes := by
        cases l₁ with
        | nil => simp at ha₁
        | cons b l₁' =>
          cases h₁ with
          | slnil => simp
          | cons a' h₁' =>
            -- h₁ : l₁ <+ a :: nodes
            -- We know a ∈ a :: l₁' and a :: l₁' <+ a :: nodes
            -- So l₁' <+ nodes
            simp at ha₁
            cases ha₁ with
            | head => exact ⟨l₁', rfl, h₁'⟩
            | tail _ ha₁' =>
              -- a is later in the list, but we need the first element to be a
              -- Actually, if a ∈ a :: l₁', then a = a (head) or a ∈ l₁' (tail)
              -- But a is the head of the cons, so a :: l₁' starts with a
              -- The only way a ∈ a :: l₁' is if a = a (always true) or a ∈ l₁'
              -- But if a = a, then the head matches
              sorry
          | cons_cons a' h₁' =>
            simp at ha₁
            cases ha₁ with
            | head => exact ⟨l₁', rfl, h₁'⟩
            | tail _ ha₁' => sorry
      sorry
    · -- a ∉ l₁
      by_cases ha₂ : a ∈ l₂
      · -- a ∈ l₂, symmetric to above
        sorry
      · -- a ∉ l₁ and a ∉ l₂, so both are sublists of nodes
        have h₁' : l₁ <+ nodes := by
          cases h₁ with
          | slnil => exact .slnil _
          | cons a' h₁' =>
            -- l₁ <+ a :: nodes and a ∉ l₁
            -- So l₁ <+ nodes
            cases l₁ with
            | nil => exact .slnil _
            | cons b l₁' =>
              cases h₁ with
              | cons a' h₁' => exact h₁'
              | cons_cons a' h₁' => exact h₁'
              | slnil => simp at ha₁
          | cons_cons a' h₁' =>
            -- l₁ <+ a :: nodes and a ∉ l₁
            -- Need to show l₁ <+ nodes
            sorry
        have h₂' : l₂ <+ nodes := by
          sorry
        exact IH hnd h₁' h₂' hdisj
```

```bash
lake env lean leanstral/Lemma2-leanstral.lean 2>&1 | head -20; echo exit=${PIPESTATUS[0]}
```

```output
leanstral/Lemma2-leanstral.lean:20:44: error: unexpected identifier; expected '#guard_msgs', 'abbrev', 'add_decl_doc', 'axiom', 'binder_predicate', 'builtin_cbv_simproc', 'builtin_cbv_simproc_decl', 'builtin_dsimproc', 'builtin_dsimproc_decl', 'builtin_grind_propagator', 'builtin_initialize', 'builtin_simproc', 'builtin_simproc_decl', 'cbv_simproc', 'cbv_simproc_decl', 'class', 'coinductive', 'declare_simp_like_tactic', 'declare_syntax_cat', 'def', 'dsimproc', 'dsimproc_decl', 'elab', 'elab_rules', 'example', 'grind_propagator', 'inductive', 'infix', 'infixl', 'infixr', 'initialize', 'instance', 'macro', 'macro_rules', 'notation', 'opaque', 'postfix', 'prefix', 'recommended_spelling', 'register_error_explanation', 'register_tactic_tag', 'register_try?_tactic', 'simproc', 'simproc_decl', 'structure', 'syntax', 'tactic_extension', 'theorem' or 'unif_hint'
leanstral/Lemma2-leanstral.lean:25:81: error: unexpected identifier; expected '#guard_msgs', 'abbrev', 'add_decl_doc', 'axiom', 'binder_predicate', 'builtin_cbv_simproc', 'builtin_cbv_simproc_decl', 'builtin_dsimproc', 'builtin_dsimproc_decl', 'builtin_grind_propagator', 'builtin_initialize', 'builtin_simproc', 'builtin_simproc_decl', 'cbv_simproc', 'cbv_simproc_decl', 'class', 'coinductive', 'declare_simp_like_tactic', 'declare_syntax_cat', 'def', 'dsimproc', 'dsimproc_decl', 'elab', 'elab_rules', 'example', 'grind_propagator', 'inductive', 'infix', 'infixl', 'infixr', 'initialize', 'instance', 'macro', 'macro_rules', 'notation', 'opaque', 'postfix', 'prefix', 'recommended_spelling', 'register_error_explanation', 'register_tactic_tag', 'register_try?_tactic', 'simproc', 'simproc_decl', 'structure', 'syntax', 'tactic_extension', 'theorem' or 'unif_hint'
leanstral/Lemma2-leanstral.lean:38:61: error: unexpected identifier; expected '#guard_msgs', 'abbrev', 'add_decl_doc', 'axiom', 'binder_predicate', 'builtin_cbv_simproc', 'builtin_cbv_simproc_decl', 'builtin_dsimproc', 'builtin_dsimproc_decl', 'builtin_grind_propagator', 'builtin_initialize', 'builtin_simproc', 'builtin_simproc_decl', 'cbv_simproc', 'cbv_simproc_decl', 'class', 'coinductive', 'declare_simp_like_tactic', 'declare_syntax_cat', 'def', 'dsimproc', 'dsimproc_decl', 'elab', 'elab_rules', 'example', 'grind_propagator', 'inductive', 'infix', 'infixl', 'infixr', 'initialize', 'instance', 'macro', 'macro_rules', 'notation', 'opaque', 'postfix', 'prefix', 'recommended_spelling', 'register_error_explanation', 'register_tactic_tag', 'register_try?_tactic', 'simproc', 'simproc_decl', 'structure', 'syntax', 'tactic_extension', 'theorem' or 'unif_hint'
exit=1
```

```bash
grep -c 'sorry' leanstral/Lemma2-leanstral.lean; grep -c 'native_decide' leanstral/Lemma2-leanstral.lean; true
```

```output
7
1
```

**Reading the output.** exit=0 with sorry count 1 means only the word in the header comment remains and the theorem is proved; any nonzero exit or a higher sorry count means the general lemma is NOT established and the ladder does not depend on it.
