# Contract Framework — Open Questions and Underspecified Aspects

Document generated 2026-06-18 during the TDD Contract Framework v0.28.0 refactoring.

---

## Q1: Semantic Matching of `expect:` to `pre:`/`post:` (Link 2 Automation)

**Status:** Deferred (manual verification only).

**Reference:** `TESTING_DISCIPLINE.md` §6.1, Link 2 (Contract → UserExpectation).

The `tdd-verify.j2` Task 5 gate flags semantic drift between `expect:` natural language and `pre:`/`post:` formal specification, but currently this is a manual assessment. The gate marks contracts with `expectation-postcondition-mismatch` violation type based on human judgment.

**Future work:** Define a `semantic_match(expect_text, pre_post_text) → float` scoring function with a threshold for `expectation-postcondition-mismatch` auto-detection. Candidates:
- Embedding similarity (cosine distance between `expect:` and concatenated `pre: + post:` text)
- Entailment checking ("does `pre:`/`post:` entail `expect:` in the user's sense?")
- Contract-level mutation testing (mutate `expect:`, re-run proptests, measure semantic distance)

**Constraint force:** Hypothesis. No existing implementation. Threshold not yet defined.

---

## Q2: Principle Conflict Resolution as Decision Procedure

**Status:** Deferred (implicit Optimality Theory ranking only).

**Reference:** `PRINCIPLES.md` §1.6, "Principle conflict resolution (implicit)."

Conflict resolution is currently implicit: Magna Carta (P1-P4) outranks Operational (P5-P7) outranks Regulatory (P8-P9) outranks Agent (P10-P12). This is a default constraint ranking heuristic, not a formalized decision procedure.

**Future work:** Formalize as a computable function `resolve_conflict(P_a, P_b) → P_a | P_b` with Optimality Theory constraint ranking. Until then, template agents must flag conflicts for human resolution (P2 consent). 

**Constraint force:** Hypothesis. No executable implementation. Human-in-the-loop is the interim gate.

---

## Q3: rSolidity Smart-Contract Expression

**Status:** Underspecified. No target language defined.

**Reference:** User prompt request: "implementation or expression in rSolidity so that this whole framework is logically composed of smart contracts between the code and the context-specific user expectations with the principles as the terms."

rSolidity is not yet a defined target language in the hKask codebase. Options under evaluation:

| Option | Description | P5 Essentialism Assessment |
|--------|-------------|---------------------------|
| (a) Solidity-like DSL for on-chain contract verification | Compile `/// REQ:` contracts to EVM-compatible bytecode for on-chain verification | High complexity. Requires blockchain integration. Fails P5 deletion test. |
| (b) Rust proc-macro eDSL (`#[contract(expect = "...", goal = P9)]`) | Replace `/// REQ:` doc-comments with attribute macros that enforce contract structure at compile time | Moderate complexity. Existing test harness already extracts contract metadata from doc-comments. Would duplicate infrastructure. |
| (c) WASM-hosted contract verifier loaded into runtime | Compile contract checker to WASM, load at runtime | High complexity. Runtime loading adds deployment surface (P5 violation). |

**Current assessment:** The existing `/// REQ:` doc-comment format already serves as a lightweight embedded DSL. The `inventory_contracts()` function in `crates/hkask-test-harness/src/test_runner.rs` extracts all contract metadata. Contract compliance verification is enforced by the CI pipeline via `contract-audit.sh`. Strengthening `/// REQ:` to a proc-macro would duplicate this infrastructure and violate P5 (essentialism) — the deletion test says "if you delete the `/// REQ:` format, complexity vanishes; if you add a proc-macro, complexity increases."

**Recommendation:** Defer rSolidity exploration until the existing `/// REQ:` format shows a concrete gap that a proc-macro would fill. The format is already Turing-complete for contract specification purposes.

---

## Q4: Probabilistic Contract `expect:` Mapping for Non-Deterministic Functions

**Status:** Underspecified. `prob:` field exists but `expect:` semantics not defined.

**Reference:** `TESTING_DISCIPLINE.md` §7.6, "Probabilistic Contracts for LLM Agents."

For non-deterministic LLM agent functions using `(p, δ, k)`-satisfaction, the `expect:` field's natural language may not have a deterministic `pre:`/`post:` equivalent. The `prob:` field already exists in the contract syntax but `expect:` for probabilistic contracts is undefined.

**Open questions:**
- How should `expect:` express non-deterministic guarantees? (e.g., "The agent should typically build on my input" → `prob: p=0.90`)
- Should `expect:` for probabilistic contracts carry the `p` threshold inline? (e.g., `expect: "The agent should build on my input at least 90% of the time" [P6]`)
- What is the verifiability of probabilistic `expect:` fields? Can a test assert "the user expectation was met in ≥90% of trials"?

**Constraint force:** Evidence. `prob:` field is defined but `expect:` mapping is not.

---

## Q5: Contract Evolution Requiring User Consent (P2 Gate Threshold)

**Status:** Underspecified. Boundary between "refactor contract" and "evolve contract" not formally defined.

**Reference:** Task 7 Rule 3, Task 7 refactor-safe contract evolution rule.

When strengthening a contract, the change may alter user-visible guarantees. The refactoring template now flags `expectation_changed: true` or `goal_principle_changed: true` as requiring P2 consent, but the threshold at which contract evolution requires new affirmative consent is not formally defined.

**Open questions:**
- Widening a precondition is backward-compatible (old callers still satisfy it) — should this always be P2-free?
- Narrowing a postcondition is NOT backward-compatible (callers may rely on the stronger old guarantee) — should this always require P2 consent?
- Changing the `expect:` field's semantics without changing `pre:`/`post:` — is this a contract evolution or a documentation fix?
- Changing the `[P{N}]` goal principle without changing `pre:`/`post:` — is this a reclassification or a contract evolution?

**Constraint force:** Hypothesis. Current rule is conservative: any `expect:` or `goal_principle` change must be flagged for human review.

---

## Q6: RDF Graph Persistence in TripleStore

**Status:** Underspecified. Graph exists as documentation only.

**Reference:** Task 0 RDF graph (`docs/architecture/contracts/contract-traceability.ttl`).

The contract triple graph is currently a documentation artifact. It is not persisted in the `TripleStore` as queryable metadata alongside the code contracts.

**Future work:** If persisted, CNS could execute queries like:
- `SELECT ?contract WHERE { ?contract hkask:hasGoalPrinciple hkask:P9 . ?contract hkask:hasConstrainingPrinciple ?cp . FILTER NOT EXISTS { ?contract hkask:hasConstrainingPrinciple hkask:P4 } }` → "find all P9 contracts with missing P4 constraints"
- Gap detection would be queryable rather than grep-based

**Constraint force:** Hypothesis. No implementation. Preservation of the `.ttl` file in `docs/architecture/contracts/` is the current mechanism.

---

## Q7: `emit_contract_coverage()` Extension for `expectation_completeness_pct`

**Status:** Underspecified (API signature gap).

**Reference:** Task 9.6, `crates/hkask-cns/src/contract_discipline.rs::emit_contract_coverage()`.

The existing `emit_contract_coverage()` function accepts `(total_pub_fns, contracted_fns, coverage_pct)` but does not accept `expectation_completeness_pct`. When `--expect` mode finds missing `expect:` fields, the span payload should include `expectation_completeness_pct` for CNS observability.

**Future work:** Add an `expectation_completeness_pct: f64` parameter to `emit_contract_coverage()` or create a separate `emit_expectation_coverage()` function. Update all call sites. This is a backward-compatible additive change.

**Constraint force:** Evidence. Existing API exists but lacks the field. Backward-compatible change.

---

## Q8: SKILL.md Regeneration from Registry Crate

**Status:** Pending (per P5.1 single source of truth).

**Reference:** Task 10.7, `PRINCIPLES.md` P5 (Essentialism), P5.1 (single source of truth).

After updating registry templates, the `.agents/skills/tdd/SKILL.md` companion must be regenerated from the registry crate. This is a mechanical transformation (`.j2` + `manifest.yaml` → `SKILL.md`) but requires a tooling step. Until regenerated, the SKILL.md may drift from the registry templates.

**Future work:** Run the skill-manager or skill-translator to regenerate all SKILL.md files from their registry crates. This should be part of the CI pipeline or a post-template-update hook.

**Constraint force:** Guideline. P5.1 requires single source of truth. The `.j2` templates are the source; SKILL.md is a generated companion. Drift between them violates P5.1.
