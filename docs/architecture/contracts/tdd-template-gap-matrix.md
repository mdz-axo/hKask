# TDD Template Audit — Gap Matrix (v0.28.0)

Audit date: 2026-06-18. Target: `TESTING_DISCIPLINE.md` §1.2 (extended syntax) and §2.6 (Goal-Principle Contract Model).

## Gap Matrix by Template

| Field | Required by TDD Spec? | tdd-plan | tdd-tracer | tdd-refactor | tdd-verify | tdd-gap-check | Gap Severity |
|-------|----------------------|----------|------------|--------------|------------|---------------|--------------|
| `pre:` condition | Yes (C1) | Yes | Yes | Yes | Yes | Yes | None |
| `post:` condition | Yes (C1) | Yes | Yes | Yes | Yes | Yes | None |
| `inv:` invariant | Yes for types (C2) | Yes | Yes | Yes | Yes | Yes | None |
| `expect:` user expectation | Yes (v0.28.0 §1.2) | Yes (as `user_expectation`) | Yes (as `user_expectation`) | Yes (as `refined_user_expectation`) | Yes (as `user_expectation:`) | Yes (as `user_expectation_correct`) | None |
| `[P{N}]` goal principle tag on `expect:` | Yes (PRINCIPLES.md §1.6) | Yes (as `goal_principle`) | Yes (as `goal_principle`) | Partially (Rule 6 mentions it but no output schema field) | Yes (as `[P{N}] Goal:` check) | Yes (as `goal_principle_match`) | Medium (refactor) |
| `[P{N}] Constraining:` annotations | Yes (PRINCIPLES.md §1.5) | Yes (as `constraining_principles`) | Yes (as `constraining_principles`) | Partially (Rule 6 mentions constraints, output schema has `added_constraint`) | Yes (as `[P{N}] Constraining:` check) | Yes (as `constraining_principles_complete`) | Medium (refactor) |
| `prob:` for non-deterministic functions | Yes (§7.6) | No | Yes | No | No | Yes (as `is_probabilistic`/`has_prob_contract`) | Low |
| `spec_id` traceability to specification | Yes (C5, Q4) | Yes | Yes | Yes | Yes | Yes | None |
| Bidirectional verification of links 2 and 3 | Yes (§6.1 v0.28.0) | No | No | No | Partially (Link 1 only) | Partially (Links 2,3 via contract_quality) | Critical |

## Detailed Findings

### tdd-plan.j2 — High Maturity
- **`expect:` field**: Present as `user_expectation` in functional_requirements output. ✅
- **`[P{N}]` goal principle**: Present as `goal_principle` object with `principle`, `name`, `rationale`. ✅
- **`[P{N}] Constraining:`**: Present as `constraining_principles` array. ✅
- **Gaps**: Missing `prob:` for non-deterministic functions. Missing bidirectional verification (Links 2 and 3). MDS Category Test Strategies table lacks default goal principle column.
- **Lexicon**: Missing `anchor`, `principal` terms despite being core concepts.

### tdd-tracer.j2 — High Maturity (Already Partially Updated)
- **`expect:` field**: Present as `user_expectation` in both contract code format AND output JSON. ✅
- **`[P{N}]` goal principle**: Present as `goal_principle` object. ✅
- **`[P{N}] Constraining:`**: Present as `constraining_principles` array. ✅
- **Gaps**: Missing bidirectional verification enforcement. Missing explicit selection guideline for choosing goal principles. The template appears to have been already partially updated for v0.28.0 — the contract code format includes all four layers. However, the `expect:` field uses the label `user_expectation:` but the spec says `expect:` — naming inconsistency.
- **Lexicon**: Already includes `constrain`. Missing `expect` and `expectation`.

### tdd-refactor.j2 — Medium Maturity
- **`expect:` field**: Rule 6 mentions preserving `user_expectation:` as one of four layers. ✅
- **`[P{N}]` goal principle**: Rule 6 mentions preserving `[P{N}] Goal:` but output schema `contract_changes` has no `goal_principle_changed` field. ⚠️
- **`[P{N}] Constraining:`**: Rule 6 mentions preserving `[P{N}] Constraining:` but no output schema field for tracking constraining changes. ⚠️
- **Gaps**: 
  - Output schema `contract_changes` lacks `expectation_changed`, `goal_principle_changed`, `constraining_changes` fields.
  - No post-refactor grep verification for extended contract fields (Rule 8bis).
  - No contract evolution consent gate (P2) when `expect:` or goal principle changes.
  - No refactor-safe contract evolution rule (expect: as ground truth).
- **Lexicon**: Missing `constrain` term.

### tdd-verify.j2 — High Maturity
- **`expect:` field**: "Contract Quality" checklist mentions `user_expectation:` and `[P{N}] Goal:` checks. ✅
- **`[P{N}] Constraining:`**: "Contract Quality" checklist mentions constraint check. ✅
- **Gaps**:
  - Violation type enum lacks `missing-user-expectation`, `missing-goal-principle`, `missing-constraining-annotation`, `expectation-postcondition-mismatch`.
  - No systematic `Contract Structure` verification section with gates.
  - No `contract_expectations_valid` output field.
  - No CNS span emission for violations (missing integration with `contract_discipline`).
  - Output schema lacks `cns_spans_emitted`.
- **Lexicon**: Missing `expectation` term.

### tdd-gap-check.j2 — High Maturity
- **`expect:` field**: "Contract Quality Analysis" already checks `user_expectation` quality and `goal_principle_match`. ✅
- **`[P{N}]` goal principle**: Comprehensive check (`goal_principle_match`, `user_expectation_correct`). ✅
- **`[P{N}] Constraining:`**: `constraining_principles_complete` check. ✅
- **Gaps**:
  - No `goal_principle_alignment` field checking contract's [P{N}] against MDS category default.
  - No `constraining_completeness` output field listing missing Magna Carta constraints.
  - No `expectation_quality` numeric scoring (0-3 scale).
  - No explicit RDF triple graph reference for gap detection model.
  - Gap types lack `missing-goal-principle`, `missing-constraining-annotation`.

## Lexicon Gap Analysis (manifest.yaml)

| Lexicon Term | Should Be In | Currently In |
|-------------|-------------|-------------|
| `expect` | tdd-tracer | ❌ Missing |
| `anchor` | tdd-plan | ❌ Missing |
| `principal` | tdd-plan | ❌ Missing |
| `constrain` | tdd-tracer, tdd-refactor | ✅ tdd-tracer, ❌ tdd-refactor |
| `expectation` | tdd-verify | ❌ Missing |

## Summary

| Template | Maturity | Critical Gaps | High Gaps | Medium Gaps |
|----------|----------|---------------|-----------|-------------|
| tdd-plan | High | 0 | 0 | 3 |
| tdd-tracer | High | 0 | 0 | 1 |
| tdd-refactor | Medium | 0 | 3 | 2 |
| tdd-verify | High | 0 | 4 | 1 |
| tdd-gap-check | High | 0 | 2 | 2 |
