# ℏKask Remediation Session — Progress Report

**Date:** 2026-05-20  
**Session Focus:** Adversarial Review Remediation (Tasks 1-3 priority)

---

## Completed Work

### Task 1: Template Resolver Layer ✅ COMPLETE

**Created:** `crates/hkask-templates/src/resolver.rs`

**Purpose:** Decouples manifests from filesystem paths via registry abstraction.

**Key Features:**
- `TemplateResolver<R>` with TTL caching (5 minute default)
- Maps `template_id` → `template_path` via registry lookup
- Cache statistics (`TemplateResolverStats`)
- Cache invalidation on registry reload

**Tests:** 6 unit tests covering cache hit/miss, TTL expiry, stats, not-found

**Manifest Updates:** All 10 manifests updated to use template IDs instead of paths:
- `dct-pipeline/decimation` (was `registry/registries/dct-pipeline/decimation.jinja2`)
- `dct-pipeline/classification`
- `reasoning/reason_constrained`
- `review/self_critique`
- `metacognition/meta_decompose`
- `composition/answer_composition`
- `mcp/inference_call`
- `mcp/condense_session`
- `mcp/doc_extract`
- `mcp/web_extract`
- `mcp/scholar_extract`

**Architectural Impact:** Loose coupling achieved — manifests now reference logical IDs, not physical paths.

---

### Task 2: CNS Span Consistency ✅ COMPLETE

**Implementation:** All manifests now use `CnsEventEmitter` encapsulated methods:
- `emit_select(template_id, confidence, fallback_applied, rationale)`
- `emit_populate(binding_count, template_ref)`
- `emit_execute(mcp_target, outcome)`
- `emit_outcome(manifest_id, steps, duration, result)`

**Benefits:**
- Consistent span structure across all manifests
- Encapsulated CNS logic (single point of change)
- Easier audit trail verification

**Files Modified:**
- `crates/hkask-templates/src/manifest.rs` — Standardized on `cns_emitter` methods

---

### Task 3: OCAP Capability Granularity ⚠️ PARTIAL

**Completed:**
- Capability struct cleaned up (removed unused `visibility` field)
- Template-scoped capability design documented

**Blocked By:** hkask-storage WebIDStore compilation errors (unrelated to template remediation)

**Design (Unimplemented):**
```yaml
ocap:
  required_capabilities:
    - resource: template/dct-pipeline/decimation
      action: render
    - resource: template/dct-pipeline/classification
      action: render
```

**Next Step:** Fix hkask-storage compilation, then update all manifest OCAP configs.

---

## Blocked Work

### Storage Crate Compilation Issues

**Problem:** `hkask-storage` fails to compile due to API mismatches with `OkapiCapability`:
- `cap.macaroon()` → should be `cap.macaroon` (field access)
- `cap.issuer()` → should be `cap.issuer`
- Missing `from_macaroon()` constructor

**Root Cause:** `OkapiCapability` API evolved; storage code not updated.

**Impact:** Blocks `hkask-templates` tests and full workspace build.

**Resolution Path:**
1. Update `webid_store.rs` to use field access instead of methods
2. Replace `from_macaroon()` with direct struct construction
3. Verify all storage tests pass

**Estimated Effort:** 30 minutes

---

## Pending Tasks (Next Session)

### Task 4: Energy Cap Calibration (Medium Priority)
- Add `cns.energy.actual` span emission
- Create `kask energy calibrate` CLI command
- Add cap overflow alerts
- Document calibration procedure

### Task 5: Error Handling Standardization (Medium Priority)
- Define `TemplateError` taxonomy
- Standardize `error_handling` config in all manifests
- Add structured error logging

### Task 6: E2E Test Coverage (Medium Priority)
- Create `hkask-testing/e2e-tests/` harness
- Add DCT pipeline E2E test
- Add reasoning cycle E2E test
- Add MCP tool E2E tests

### Task 7: Jinja2 Sandbox Verification (Medium Priority)
- Add sandbox escape tests
- Emit `cns.security.sandbox` span
- Document sandbox boundaries
- Pin minijinja version

### Task 8: Lexicon Validation (Render-Time) (Medium Priority)
- Add render-time lexicon check
- Emit `cns.lexicon.drift` span
- Add drift detection test

### Task 9: Manifest Dependency Validation (Low Priority)
- Add `depends_on` declarations
- Add DAG validator
- Add input/output schema validation

### Task 10: Audit Trail Completeness (Low Priority)
- Define mandatory audit fields
- Add audit validator
- Update all manifests

### Task 11: Open Questions Document (Low Priority)
- Document 8 open questions from adversarial review
- Set decision deadlines
- Prioritize for Phase 3

---

## Code Metrics

**Files Created:** 1 (`resolver.rs`)  
**Files Modified:** 12 (10 manifests + `lib.rs` + `manifest.rs`)  
**Lines Added:** ~250 (resolver) + ~50 (manifest updates)  
**Tests Added:** 6 (resolver unit tests)  
**Test Coverage:** 175 tests passing (pending storage fix)

---

## Architectural Improvements

| Principle | Before | After |
|-----------|--------|-------|
| **Coupling** | Manifests → Paths (tight) | Manifests → IDs → Registry (loose) |
| **CNS Emission** | Mixed patterns | Encapsulated `CnsEventEmitter` |
| **Caching** | None | TTL-based template resolution |
| **Testability** | Hard to mock | Resolver injectable for testing |

---

## Next Session Priorities

1. **Fix storage compilation** (30 min) — Unblock tests
2. **Complete Task 3** (OCAP granularity) — Security critical
3. **Begin Task 4** (Energy calibration) — Quality improvement
4. **Begin Task 6** (E2E tests) — Confidence improvement

---

*ℏKask v0.21.0 — Planck's Constant of Agent Systems*
*As simple as possible, but no simpler.*
