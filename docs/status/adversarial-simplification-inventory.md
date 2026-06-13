---
title: "Adversarial Simplification Inventory"
audience: [architects, developers]
last_updated: 2026-06-08
version: "0.1.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, domain]
---

# Adversarial Simplification Inventory

Catalog of dead code, unwired seams, and simplification opportunities across the hKask codebase. Each entry is testable — removal should not break `cargo check --workspace`.

## Summary

| Category | Count |
|----------|-------|
| `#[allow(dead_code)]` annotations | 12 |
| Unwired seams (stubs/traits with 0 callers) | 4 |
| Simplification candidates | 3 |
| **Total** | **19** |

---

## Dead Code: `#[allow(dead_code)]` Inventory

Items annotated `#[allow(dead_code)]` — code that compiles but has no runtime caller. Each is either reserved for future use or awaiting integration.

### DC-001: `GitCasAdapter::new(base_path)`

- **File:** `crates/hkask-mcp/src/git_cas/mod.rs:26`
- **Annotation:** `#[allow(dead_code)]` on `pub fn new`
- **Status:** ⚠️ Reserved — alternatives: `from_path`, `new_with_checker`. The public `new(base_path)` constructor may be used by external crates but has no internal callers.
- **Action:** Keep. Public API exposed for external consumption.

### DC-002: `GitCasAdapter::load_template_crate_or_synthesize`

- **File:** `crates/hkask-mcp/src/git_cas/mod.rs:80`
- **Annotation:** `#[allow(dead_code)] pub(crate)`
- **Status:** ⚠️ Reserved — synthesizes `.j2`/`.yaml` files into template crates. No current caller but bridges the gap between filesystem templates and Git CAS expectations.
- **Action:** Keep. Will be needed when template loading moves from filesystem to Git CAS (deferred P1 task).

### DC-003: `RepoManager` (entire struct + impl)

- **File:** `crates/hkask-mcp/src/git_cas/repo_manager.rs:16-26`
- **Annotation:** `#[allow(dead_code)]` on both struct and impl block
- **Status:** ⚠️ Reserved — wraps `Arc<dyn GitCASPort>` for testability. No runtime caller; no tests exercise it.
- **Action:** Defer removal. The `RepoManager` is the testable abstraction layer for Git CAS operations (ADR-019). Write tests or remove if unused by v0.24.

### DC-004: `SnapshotWriter` (entire struct + impl)

- **File:** `crates/hkask-mcp/src/git_cas/snapshot_writer.rs:25-34`
- **Annotation:** `#[allow(dead_code)]` on both struct and impl block
- **Status:** ⚠️ Reserved — writes registry entries through `Arc<dyn GitCASPort>`. No runtime caller; no tests.
- **Action:** Defer removal. Will be needed when `kask skill publish` pushes skills to Git CAS (P2 deferred task). Same assessment as DC-003.

### DC-005: `McpToolOutput::with_metadata`

- **File:** `crates/hkask-mcp/src/server.rs:208`
- **Annotation:** `#[allow(dead_code)] pub(crate)`
- **Status:** ⚠️ Reserved — constructor overload for tool output with metadata. `McpToolOutput::new(content)` is the primary constructor.
- **Action:** Keep. Adding metadata to tool output is a natural extension path.

### DC-006: `VerificationService::Assertion`

- **File:** `crates/hkask-services/src/verification.rs:22-24`
- **Annotation:** `#[allow(dead_code)] pub struct Assertion`
- **Status:** ⚠️ Reserved — Magna Carta verification assertion type. No callers (`VerificationService` is a planned but unwired service).
- **Action:** Defer. The Magna Carta verifier skill and `kask sovereignty verify` command exist; this service layer struct may become the canonical type.

### DC-007: `bundle::cns_spans` module

- **File:** `crates/hkask-types/src/bundle.rs:512-516`
- **Annotation:** `#[allow(dead_code)] pub(crate) mod cns_spans`
- **Status:** ⚠️ Reserved — CNS span namespace constants for bundle composition operations. No CNS observer reads these span names yet.
- **Action:** Keep. Will be consumed when CNS span integration for bundle operations is implemented.

### DC-008: `WebID::full_display()`

- **File:** `crates/hkask-types/src/id.rs:211-215`
- **Annotation:** `#[allow(dead_code)] pub(crate) fn full_display`
- **Status:** ⚠️ Reserved — guarded behind `HKASK_TRACE_WEBIDS=1` env var for trace-level diagnostics.
- **Action:** Keep. Security-sensitive full-display is an intentional "break glass" diagnostic tool.

### DC-009: `Confidence::zero()` and `Confidence::into_inner()`

- **File:** `crates/hkask-types/src/visibility.rs:202-212`
- **Annotation:** `#[allow(dead_code)] pub(crate)`
- **Status:** ⚠️ Reserved — convenience constructors for the `Confidence` newtype.
- **Action:** Keep. `zero()` is the identity value for confidence arithmetic; `into_inner()` is a standard newtype unwrap pattern.

### DC-010: `hkask-mcp-spec` types module-wide `allow(dead_code)`

- **File:** `mcp-servers/hkask-mcp-spec/src/types.rs:1` — `#![allow(dead_code)]` at module level
- **Status:** ⚠️ Reserved — entire types module suppressed. Contains `TestClassification`, `TestPriority`, `TestCoverage` enums for MDS testing protocol (TP-1 through TP-5).
- **Action:** Defer. Types are referenced in the spec server's OpenAPI schema but have no runtime callers. Remove when spec server is self-applied (DA-4).

### DC-011: `A2A` compile-time guard + `RouteFields` 

- **File:** `crates/hkask-agents/src/acp/mod.rs:168-181`
- **Annotation:** `#[allow(dead_code)]` on compile-time trait object safety assertion and `pub(super) RouteFields`
- **Status:** ⚠️ Reserved — `RouteFields` used only in test module. Compile-time guard ensures `A2AMessageVisitor` stays object-safe.
- **Action:** Keep. Compile-time invariant guard is a deliberate architectural choice.

### DC-012: Bundle test helpers (`make_skill`, `make_step`, `valid_manifest`)

- **File:** `crates/hkask-types/src/bundle.rs:522-562`
- **Annotation:** `#[allow(dead_code)] fn` in test module
- **Status:** ✅ Test-only — test helper functions used only within `mod tests`. Standard Rust pattern.
- **Action:** Keep. Test fixtures.

---

## Unwired Seams

Traits, structs, or functions that exist in the codebase but are not called from any runtime path.

### US-001: `CapabilityAwareValidator`

- **File:** `crates/hkask-templates/src/capability_validator.rs`
- **Status:** ⚠️ Stub — returns `Ok(())` unconditionally. Not wired into `register()`.
- **Action:** Wire when OCAP-aware template registration is needed (FA-T3). Current enforcement handled by `GovernedTool` at runtime.

### US-002: `SqliteSpecStore::expire()`

- **File:** `crates/hkask-storage/src/spec_store.rs:268-279`
- **Status:** ✅ Wired (2026-06-08) — now used in `CurationLoop` for spec lifecycling via `SpecStore::expire()`.
- **Note:** Method exists and is tested (6 tests pass) but integration caller not yet active in curation loop.

### US-003: `SqliteCurationRecordStore` — `save_curation_record` caller

- **File:** `crates/hkask-storage/src/spec_store.rs:104-119`
- **Status:** ⚠️ Unwired — `save_curation_record` exists but `DefaultSpecCurator::evaluate()` returns `SpecCurationRecord` without persisting it through the store.
- **Action:** Wire `save_curation_record` into `evaluate()`. Requires passing `&SqliteCurationRecordStore` to `SpecCurator::evaluate()` (breaking trait change) or using an observer pattern. Tracked as FUT-012.

### US-004: `DefaultSpecCurator::calibrate_from_history()`

- **File:** `crates/hkask-agents/src/curator_agent/spec_curator.rs:51-96`
- **Status:** ⚠️ New (2026-06-08) — added but no caller yet. Designed for CLI `kask cns calibrate` or automatic calibration in the curation cycle.
- **Action:** Wire into CLI command or CurationLoop startup. Low priority — threshold is operational data-dependent.

---

## Simplification Candidates

Code that is wired and functional but could be simpler.

### SC-001: `SpecStore::list_all()` vs `SpecStore::list_by_category(any)`

- **Rationale:** `list_all()` is a special case of `list_by_category` with no filter. Both exist in the trait; `list_all()` has no caller that could not be served by `list_by_category` with iteration.
- **Status:** ⚠️ Keep — `list_all()` is a convenience method used in tests and by `SpecService::list()`. Removing it would require callers to iterate across all 9 categories.
- **Recommendation:** Retain. The convenience is earned by actual consumers.

### SC-002: `load_curation_records` and `load_all_curation_records` overlap

- **Rationale:** `load_curation_records(spec_id)` returns records for a single spec. `load_all_curation_records()` returns all records. The former is a filtered subset of the latter.
- **Status:** ⚠️ Keep — `load_curation_records` is used by `SpecService::validate()` for per-spec curation audit; `load_all_curation_records` is used by `calibrate_from_history()` for threshold calibration. Both have distinct callers.
- **Recommendation:** Retain. P1: no trait without two consumers — here each method has its own consumer.

### SC-003: `DomainAnchor` enum with only 2 variants

- **File:** `crates/hkask-storage/src/spec_types.rs:96-102`
- **Rationale:** 2-variant enum (`Okapi`, `Hkask`) could be collapsed into a boolean or flag. But enum ensures extensibility for future domain anchors.
- **Status:** ⚠️ Keep — the MDS spec explicitly describes `DomainAnchor` as an open set. Adding a third variant (e.g., `Federation`) would require refactoring a boolean.
- **Recommendation:** Retain. Enum is correct for an extensible taxonomy.

---

## Removal Candidates

Items that have no callers and no planned future use.

**None identified.** All `#[allow(dead_code)]` items have documented rationale for retention. This is consistent with hKask's design constraints — P6 ("Delete stubs, don't publish them") and P7 ("todo!() macros only with FocusingAssumptions") have kept the codebase clean of orphaned stubs.

---

## Verification

```bash
# Count dead_code annotations (excluding test fixtures)
grep -r "#\[allow(dead_code)\]" crates/ mcp-servers/ | grep -v "mod tests" | wc -l

# Check for leftover todo!()/unimplemented!()
grep -r "todo!\|unimplemented!" crates/ mcp-servers/ --include="*.rs" | grep -v "//. "
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
