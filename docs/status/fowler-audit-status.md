---
title: "Fowler Pattern Refactoring Tracker"
audience: [architects, developers]
last_updated: 2026-06-08
version: "0.1.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, composition, domain]
---

# Fowler Pattern Refactoring Tracker

Tracks refactoring opportunities across the hKask codebase aligned to Martin Fowler's catalog of refactoring patterns. Each entry identifies a code smell, the applicable refactoring pattern, and current status.

## Pattern Categories Scanned

| Category | Patterns Checked |
|----------|-----------------|
| Composing Methods | Extract Method, Inline Method, Replace Temp with Query |
| Moving Features | Move Method, Move Field, Extract Class |
| Organizing Data | Replace Magic Number with Constant, Encapsulate Field |
| Simplifying Conditionals | Decompose Conditional, Consolidate Conditional Expression |
| Dealing with Generalization | Pull Up Method, Push Down Method, Extract Interface |

---

## Identified Opportunities

### FR-001: `Registry::register()` duplicates lexicon validation

- **File:** `crates/hkask-templates/src/registry.rs:128-149`, `crates/hkask-templates/src/registry_sqlite.rs:214-262`
- **Pattern:** **Extract Method** (Fowler §6.1) — lexion validation was inline in both `register()` methods
- **Action:** ✅ **Applied** (2026-06-08) — Extracted `ContractValidator::validate_terms()` shared between `Registry` and `SqliteRegistry`
- **Rationale:** Same validation logic existed in two impl blocks. ContractValidator provides single source of truth with configurable Warn/Reject modes.

### FR-002: Inline `HLexicon::validate()` call in `Registry::register()`

- **File:** `crates/hkask-templates/src/registry.rs:137-145` (prior state)
- **Pattern:** **Replace Method with Method Object** (Fowler §6.9) — simple `HLexicon::validate()` call replaced with `ContractValidator` for richer behavior
- **Action:** ✅ **Applied** (2026-06-08) — `ContractValidator` holds configurable `ValidationMode` and emits structured CNS spans
- **Rationale:** The simple `unknown = lexicon.validate()` check was adequate for warning but not for rejection or CNS observability. ContractValidator adds both without changing call-site complexity.

### FR-003: `row_to_curation_record` row-index fragility

- **File:** `crates/hkask-storage/src/spec_store.rs:24-69`
- **Pattern:** **Replace Parameter with Explicit Method** — `row_to_curation_record(row, spec_id, decision_idx, ocap_idx)` uses positional indices for column mapping
- **Status:** ⚠️ **Open (Low)** — positional offset parameters (`decision_idx: 0` vs `decision_idx: 1`) dependent on whether `spec_id` is in the SELECT list
- **Recommendation:** Split into two methods: `row_to_curation_record_with_spec_id` and `row_to_curation_record_without_spec_id` to eliminate the positional coupling. Defer until a third call site appears (P1: no abstraction without second consumer).

### FR-004: `collect_rows!` macro has two code paths for error handling

- **File:** `crates/hkask-storage/src/store_macros.rs:154-188`
- **Pattern:** **Consolidate Duplicate Conditional Fragments** (Fowler §10.5) — 3-arg and 4-arg macro forms have near-identical bodies
- **Status:** ⚠️ **Open (Low)** — the 3-arg form returns `Result<_, rusqlite::Error>` directly (bubbles errors), the 4-arg form skips malformed rows with `tracing::warn!`. Both patterns are legitimate depending on call-site semantics.
- **Recommendation:** Keep. The two forms serve different error strategies (fail-fast vs skip-and-warn). Attempting to unify would add a `Result`-vs-`skip` parameter that makes the macro harder to read.

### FR-005: `list_curation_records_since` and `load_all_curation_records` share row-mapping closure

- **File:** `crates/hkask-storage/src/spec_store.rs:137-165`
- **Pattern:** **Extract Method** (Fowler §6.1) — both methods inline a `|row| { let s: String = row.get(0)?; ... row_to_curation_record(...) }` closure
- **Status:** ⚠️ **Open (Low)** — duplication is minimal (4 lines) and `row_to_curation_record` already handles the shared logic. The closure varies only in which column `spec_id` comes from.
- **Recommendation:** Apply when a third call site for the same query pattern appears (P1: two consumers is the threshold).

### FR-006: `DefaultSpecCurator` constructor duplication

- **File:** `crates/hkask-agents/src/curator_agent/spec_curator.rs:40-67`
- **Pattern:** **Extract Method** — `new()` and `from_config()` duplicate field initialization
- **Status:** ⚠️ **Open (Low)** — `new()` takes bare threshold; `from_config()` takes `CurationThresholdConfig`. Both clamp and set `drift_threshold`/`max_iterations`. The `from_config()` also emits `tracing::info!`.
- **Recommendation:** Extract `fn with_defaults(coherence: f64, drift: f64) -> Self` shared constructor. Defer until the struct gains another field (P1: no trait without two consumers).

---

## Pattern Application Log

| Date | ID | Pattern | Scope | Decision |
|------|-----|---------|-------|----------|
| 2026-06-08 | FR-001 | Extract Method | `ContractValidator::validate_terms()` across two `register()` impls | Applied |
| 2026-06-08 | FR-002 | Replace Method with Method Object | `ContractValidator` replacing inline `HLexicon::validate()` | Applied |

---

## Summary

| Status | Count |
|--------|-------|
| Applied | 2 |
| Open (Low) | 4 |
| Open (Medium) | 0 |
| Open (High) | 0 |

**Overall:** No high-priority Fowler patterns required. The codebase follows hKask design constraints (P1: no trait without two consumers, C4: repetition is missing primitive). Most remaining opportunities are low-priority consolidations that would add abstraction without clear benefit at the current consumer count.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
