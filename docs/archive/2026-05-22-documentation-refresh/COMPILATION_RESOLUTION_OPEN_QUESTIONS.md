# Compilation Resolution Open Questions

**Date:** 2026-05-21  
**Status:** All questions resolved  
**Related:** hKask Architecture Master Spec v0.21.0

---

## Summary

This document captures design decisions from the hKask compilation resolution effort (2026-05-21). All questions have been resolved with implementations or documented policies.

---

## Question 1: Route Handler Ownership ✅ RESOLVED

**Decision:** Service layer pattern

**Implementation:**
- Service modules created in `crates/hkask-api/src/services/`
- Services: `TemplateService`, `CnsService`, `PodService`, `SovereigntyService`
- Routes delegate to services (thin HTTP adapters)
- Full refactoring deferred — pattern established, incremental migration recommended

**GitHub Issue:** #[TBD]

---

## Question 2: Capability Propagation ✅ RESOLVED

**Decision:** Hybrid approach (pod-scoped + per-call verification)

**Implementation:**
- `McpPort` trait enables capability token injection
- Pod receives attenuated capability at creation
- Foundation laid for per-call OCAP verification
- Requires ACP runtime integration spec for full implementation

**GitHub Issue:** #[TBD]

---

## Question 3: Error Surface Unification ✅ RESOLVED

**Decision:** Error trait with CNS conversion (`ToCnsAlert`)

**Implementation:**
- `ApiError` enum with `ToCnsAlert` trait
- Converts errors to `AlgedonicAlert` with severity levels
- Enables automatic CNS algedonic escalation
- Location: `crates/hkask-api/src/error.rs`

**GitHub Issue:** #[TBD]

---

## Question 4: Testing Boundary ✅ RESOLVED

**Decision:** Inline unit, integration in hkask-testing

**Implementation:**
- `hkask-testing/unit-tests/` — stub test files created
- `hkask-testing/integration-tests/` — integration tests
- `hkask-testing/test-harnesses/` — shared test utilities
- Policy: Keep simple unit tests inline (<50 lines), move complex tests to `hkask-testing`

**GitHub Issue:** #[TBD]

---

## Question 5: MCP Dispatch Coupling ✅ RESOLVED

**Decision:** Trait-based injection

**Implementation:**
- `crates/hkask-api/src/ports/mcp_port.rs` — `McpPort` trait
- `ApiState` uses `Arc<dyn McpPort>` for dependency injection
- Enables testing with mock implementations
- Added `async-trait` crate dependency

**Benefits Achieved:**
- Decoupled `hkask-api` from concrete `hkask-mcp` implementation
- Test mocking enabled without compiling `hkask-mcp`
- Runtime injection of alternative MCP implementations

**GitHub Issue:** #[TBD] — Mark as resolved

---

## Question 6: Sovereignty Service Architecture ✅ RESOLVED

**Decision:** Stateless by default, TTL cache optional

**Implementation:**
- `SovereigntyService` stub in `crates/hkask-api/src/services/sovereignty.rs`
- Default: stateless capability + boundary check
- Future: session-backed cache with 5-minute TTL (deferred)
- Cache key: `(user_webid, category_hash)`
- Invalidation: explicit consent grant/revoke clears cache

**Requires:** `hkask-storage::SovereigntyRepository` for persistent state

**GitHub Issue:** #[TBD]

---

## Resolution Status

| Question | Status | Implementation |
|----------|--------|----------------|
| 1. Route Handler Ownership | ✅ Resolved | Service layer pattern established |
| 2. Capability Propagation | ✅ Resolved | `McpPort` trait foundation |
| 3. Error Surface Unification | ✅ Resolved | `ToCnsAlert` trait |
| 4. Testing Boundary | ✅ Resolved | Policy documented |
| 5. MCP Dispatch Coupling | ✅ Resolved | Trait-based injection |
| 6. Sovereignty Service | ✅ Resolved | Stub service in place |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Compilation Resolution Open Questions — 2026-05-21 — ALL RESOLVED*
