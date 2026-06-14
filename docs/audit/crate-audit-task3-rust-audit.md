---
title: "Crate Audit — Task 3: Idiomatic Rust Audit (Graydon Hoare School)"
audience: [developers, auditors]
last_updated: 2026-06-12
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, curation]
---

# Task 3 — Idiomatic Rust Audit (Graydon Hoare School)

**Bundle:** `crate-audit` | **Phase:** Core (rust-expertise + deep-module + pragmatic-semantics)
**Date:** 2026-06-12 | **Provenance:** [Directly Stated, grep search] + [Implicit, pattern inference]

---

## Phase 1: Philosophy Assessment (Per-Crate Invariant Summary)

| Crate | Core Invariant | Invalid States Eliminated |
|-------|---------------|--------------------------|
| `hkask-types` | All ID types are validated at construction; OCAP tokens carry attenuation proofs | `WebID` validated via `Id::new()`; `DelegationToken` carries signature |
| `hkask-inference` | Provider routing is deterministic from model name prefix | Provider prefix parsing in `InferenceRouter::route()` |
| `hkask-storage` | All DB operations are encrypted via SQLCipher; ν-events are immutable | `Database::open_database()` requires passphrase |
| `hkask-memory` | Episodic → Semantic consolidation preserves provenance | `ConsolidationBridge` carries `TripleID` references |
| `hkask-cns` | Variety counters reflect actual system diversity; algedonic alerts trigger at thresholds | `SetPoints` config with validated ranges |
| `hkask-templates` | Template execution follows FlowDef contracts; registry entries are content-hashed | `ManifestExecutor` validates step contracts |
| `hkask-agents` | Pod lifecycle is linear (Populated→Registered→Activated→Deactivated); OCAP gates all operations | `PodLifecycleState::can_transition_to()` |
| `hkask-keystore` | All secret material is zeroized after use; keys derived via HKDF | `Zeroizing` wrappers on all key material |
| `hkask-mcp` | All tool dispatch passes through GovernedTool membrane | `McpDispatcher` wraps `RawMcpToolPort` with `GovernedTool` |
| `hkask-services` | CLI and API share identical service layer; no duplicated business logic | `AgentService` is single source of truth |
| `hkask-api` | All routes are authenticated; OpenAPI schema is complete | `auth_middleware` on all routes |
| `hkask-cli` | All commands delegate to `hkask-services`; no direct crate calls | Commands use `AgentService::build()` |
| `hkask-wallet` | Signing is isolated in `signing.rs`; keys are per-operation, zeroized immediately | `sign_capability()`, `sign_withdrawal()` in isolated module |

---

## Phase 2: Type Strength Audit

### Finding T1 — Error Stringification in `From` Impls

| # | Location | Anti-Pattern | Detail |
|---|----------|-------------|--------|
| T1a | `hkask-agents/src/error.rs:49` | **Error stringification** | `impl From<rusqlite::Error> for CoreError` uses `e.to_string()` → loses rusqlite error type |
| T1b | `hkask-agents/src/error.rs:79` | **Error stringification** | `impl From<hkask_storage::DatabaseError> for MemoryError` uses `e.to_string()` |
| T1c | `hkask-api/src/error.rs:230-236` | **Error stringification** | `impl From<InfrastructureError> for ApiError` uses `e.to_string()` |
| T1d | `hkask-api/src/error.rs:146-152` | **Error stringification** | `impl From<NuEventError> for ApiError` uses `e.to_string()` |
| T1e | `hkask-api/src/error.rs:160` | **Error stringification** | `impl From<ConsentStoreError> for ApiError` uses `e.to_string()` for Infra variant |

**Epistemic Mode:** [Declarative, IS] — directly observed in source code.
**Constraint Force:** **[Guideline, OUGHT-Probabilistic]** — Error stringification loses type information for upstream error handlers. `thiserror`'s `#[source]` attribute should be used instead.

### Finding T2 — `Box<dyn Error>` in Library Public APIs

| # | Location | Anti-Pattern | Detail |
|---|----------|-------------|--------|
| T2a | `hkask-agents/src/error.rs:21` | **Box<dyn Error> in library API** | `McpError::InvocationFailed(Box<dyn std::error::Error + Send + Sync>)` — should be typed |
| T2b | `hkask-agents/src/pod/mod.rs:161` | **Box<dyn Error> in library API** | `AgentPodError::ToolError(Box<dyn std::error::Error + Send + Sync>)` — should be typed |
| T2c | `hkask-templates/src/ports.rs:29` | **Box<dyn Error> in library API** | `TemplateError::Mcp(Box<dyn std::error::Error + Send + Sync>)` — should be typed |
| T2d | `hkask-agents/src/registry_loader.rs:16` | **Box<dyn Error> in library API** | `RegistryLoaderError::Io(Box<dyn std::error::Error + Send + Sync>)` — could use `std::io::Error` directly |

**Epistemic Mode:** [Declarative, IS] — directly observed in source code.
**Constraint Force:** **[Guideline, OUGHT-Probabilistic]** — `Box<dyn Error>` in library APIs prevents callers from matching on specific error types. Use typed error variants or `#[source]` with concrete types.

### Finding T3 — `.unwrap()` in Library Code

| # | Location | Anti-Pattern | Detail |
|---|----------|-------------|--------|
| T3a | `hkask-inference/src/embedding_router.rs:99` | **`.unwrap()` in library code** | `self.fireworks_client.as_ref().unwrap()` — panics if client not initialized |
| T3b | `hkask-inference/src/embedding_router.rs:109` | **`.unwrap()` in library code** | `self.deepinfra_client.as_ref().unwrap()` — panics if client not initialized |

**Epistemic Mode:** [Declarative, IS] — directly observed in source code.
**Constraint Force:** **[Guardrail, OUGHT-Declarative]** — Libraries must not panic. These `.unwrap()` calls on `Option` from `as_ref()` should return `Result` with a meaningful error. The embedding router should validate client availability at construction or return `Err` at call time.

### Finding T4 — Boolean Blindness (Minor)

| # | Location | Anti-Pattern | Detail |
|---|----------|-------------|--------|
| T4 | `hkask-cli/src/cli/helpers.rs:16` | **Boolean blindness** | `pub fn init_logging(verbose: bool)` — `bool` carries no meaning. Could be `enum LogLevel { Quiet, Verbose }`. |

**Epistemic Mode:** [Declarative, IS] — directly observed.
**Constraint Force:** **[Evidence, IS-Probabilistic]** — In application code (CLI), boolean parameters are acceptable. Only flagging for consistency awareness.

---

## Phase 3: Ownership Clarity Audit

### Finding O1 — Nested `Arc<RwLock<Arc<RwLock<...>>>>`

| # | Location | Anti-Pattern | Detail |
|---|----------|-------------|--------|
| O1 | `hkask-agents/src/ensemble/session.rs:22` | **Nested interior mutability** | `chats: Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>` — double locking. The inner `Arc<RwLock<EnsembleChat>>` suggests `EnsembleChat` itself needs shared mutable access. Could `EnsembleChat` own its state without interior mutability? **Deferred (2026-06-14):** Ensemble crate removed. Finding preserved for reference when future multi-agent mode is implemented. |

**Epistemic Mode:** [Declarative, IS] — directly observed.
**Constraint Force:** **[Evidence, IS-Probabilistic]** — Nested `Arc<RwLock<...>>` is a complexity signal. May be justified by concurrent access patterns, but warrants review.

### Finding O2 — `Arc<dyn Trait>` Proliferation

| # | Location | Detail |
|---|----------|--------|
| O2 | `hkask-agents/src/pod/manager.rs:20-30` | `PodManager` has 8 `Arc<dyn ...Port>` fields. This is the hexagonal ports pattern — each port is a trait object for testability. Justified by architecture, but 8 ports is a large dependency surface. |

**Epistemic Mode:** [Declarative, IS] — directly observed.
**Constraint Force:** **[Evidence, IS-Probabilistic]** — Hexagonal architecture justifies trait objects. Not an anti-pattern, but port count warrants periodic review.

---

## Phase 4: Error Handling Hygiene Audit

### Finding E1 — Discarded Results (`.ok()`)

| # | Location | Detail |
|---|----------|--------|
| E1 | `hkask-agents/src/pod/mod.rs:558` (test) | `std::fs::create_dir_all(&crate_dir).ok();` — explicit discard in test code. Acceptable. |

**No production discarded Results found.** All `.ok()` calls are in test code with explicit intent.

### Finding E2 — `.expect()` Usage

**All `.expect()` calls found are in test code only.** No production `.expect()` calls in library crates. ✓

---

## Phase 5: Trait Usage & Unsafe Audit

### Trait Usage

| Check | Result |
|-------|--------|
| `Box<dyn Trait>` where generics would work | **None found.** All trait objects are for runtime polymorphism (hexagonal ports, chain backends). |
| Missing `Display` impls where `ToString` is derived | **None found.** No `ToString` impls without `Display`. |
| `From` impls that should be `TryFrom` | **None found.** All `From` impls are infallible conversions (error wrapping, field mapping). |
| Derive-all-the-things without semantic justification | **Not audited exhaustively.** Spot-check shows derives are intentional (e.g., `Clone` on `ApiState` for axum state sharing). |

### Unsafe Audit

| # | Location | Has `# Safety`? | Context |
|---|----------|----------------|---------|
| U1 | `hkask-storage/src/database.rs:27` | **YES** — detailed SAFETY comment | `sqlite3_vec_init` FFI — transmute for sqlite extension loading. Standard pattern. |
| U2 | `hkask-agents/src/pod/mod.rs:554` (test) | **YES** | `std::env::set_var` in single-threaded test |
| U3 | `hkask-keystore/src/keychain.rs:373` (test) | **YES** | `std::env::set_var` in single-threaded test |
| U4 | `hkask-services/src/wallet.rs:218` (test) | **YES** | `std::env::set_var` in single-threaded test |
| U5 | `hkask-wallet/src/issuer.rs:175` (test) | **YES** | `std::env::set_var` in single-threaded test |
| U6 | `hkask-wallet/src/manager.rs:616` (test) | **YES** | `std::env::set_var` in single-threaded test |
| U7 | `hkask-wallet/src/signing.rs:107` (test) | **YES** | `std::env::set_var` in single-threaded test |

**Finding:** All `unsafe` blocks have `# SAFETY` documentation. ✓
**Finding:** Only 1 production `unsafe` block (U1) — properly documented and isolated. ✓
**Finding:** Test `unsafe` blocks all use the same `set_var` pattern — could be extracted into a `test_helpers::set_test_master_key()` function to consolidate. [Guideline, OUGHT-Probabilistic]

---

## Phase 6: Module Depth Audit

Full depth scoring per crate. Interface = count of `pub` items. Implementation = estimated non-pub LOC.

| Crate | Pub Items | Est. Impl LOC | Depth Score | Classification |
|-------|-----------|---------------|-------------|----------------|
| `hkask-types` | 275 | ~3000 | 10.9 | **Very Shallow** |
| `hkask-agents` | 204 | ~2500 | 12.3 | **Very Shallow** |
| `hkask-cli` | 196 | ~3000 | 15.3 | Shallow (app boundary) |
| `hkask-api` | 166 | ~2000 | 12.0 | **Very Shallow** |
| `hkask-services` | 144 | ~2500 | 17.4 | Shallow |
| `hkask-storage` | 86 | ~1500 | 17.4 | Shallow |
| `hkask-cns` | 53 | ~1200 | 22.6 | Shallow |
| `hkask-mcp` | 48 | ~1000 | 20.8 | Shallow |
| `hkask-templates` | 45 | ~800 | 17.8 | Shallow |
| `hkask-inference` | 44 | ~600 | 13.6 | **Very Shallow** |
| `hkask-memory` | 40 | ~600 | 15.0 | Shallow |
| `hkask-keystore` | 32 | ~400 | 12.5 | **Very Shallow** |
| `hkask-wallet` | 24 | ~500 | 20.8 | Shallow |

### Depth Findings

| # | Crate | Finding | Constraint Force |
|---|-------|---------|-----------------|
| D1 | `hkask-types` | 275 pub items. Foundation crate with interface explosion. 20 public modules, ~120 re-exports. | **[Evidence, IS-Declarative]** |
| D2 | `hkask-agents` | 204 pub items. 12 public modules. Many re-exports from `hkask-storage` (EscalationQueue types). | **[Evidence, IS-Declarative]** |
| D3 | `hkask-templates` | Re-exports 5 items from `hkask-inference` with zero added behavior — pass-through. | **[Guideline, OUGHT-Probabilistic]** |
| D4 | `hkask-keystore` | 15 public resolver functions in `keychain` module — interface explosion for a 400-line crate. | **[Guideline, OUGHT-Probabilistic]** |
| D5 | `hkask-storage` | 14 public modules, each with its own error type. Error type per module is an anti-pattern per deep-module P3. | **[Guideline, OUGHT-Probabilistic]** |

---

## 7. Consolidated Finding Registry

| ID | Phase | Crate | File | Anti-Pattern | Epistemic Mode | Constraint Force |
|----|-------|-------|------|-------------|----------------|-----------------|
| T1a | Type | agents | `error.rs:49` | Error stringification (`e.to_string()`) | [Declarative, IS] | **[Guideline]** |
| T1b | Type | agents | `error.rs:79` | Error stringification | [Declarative, IS] | **[Guideline]** |
| T1c | Type | api | `error.rs:230` | Error stringification | [Declarative, IS] | **[Guideline]** |
| T1d | Type | api | `error.rs:146` | Error stringification | [Declarative, IS] | **[Guideline]** |
| T1e | Type | api | `error.rs:160` | Error stringification | [Declarative, IS] | **[Guideline]** |
| T2a | Type | agents | `error.rs:21` | `Box<dyn Error>` in library API | [Declarative, IS] | **[Guideline]** |
| T2b | Type | agents | `pod/mod.rs:161` | `Box<dyn Error>` in library API | [Declarative, IS] | **[Guideline]** |
| T2c | Type | templates | `ports.rs:29` | `Box<dyn Error>` in library API | [Declarative, IS] | **[Guideline]** |
| T2d | Type | agents | `registry_loader.rs:16` | `Box<dyn Error>` in library API | [Declarative, IS] | **[Guideline]** |
| T3a | Type | inference | `embedding_router.rs:99` | `.unwrap()` in library code | [Declarative, IS] | **[Guardrail]** |
| T3b | Type | inference | `embedding_router.rs:109` | `.unwrap()` in library code | [Declarative, IS] | **[Guardrail]** |
| T4 | Type | cli | `cli/helpers.rs:16` | Boolean blindness | [Declarative, IS] | **[Evidence]** |
| O1 | Ownership | agents | `ensemble/session.rs:22` | Nested `Arc<RwLock<Arc<RwLock<...>>>>` — **Deferred (2026-06-14):** Ensemble crate removed. Finding preserved for reference when future multi-agent mode is implemented. | [Declarative, IS] | **[Evidence]** |
| O2 | Ownership | agents | `pod/manager.rs:20` | 8 `Arc<dyn Port>` fields | [Declarative, IS] | **[Evidence]** |
| D3 | Depth | templates | `lib.rs:37-41` | Pass-through re-exports from inference | [Declarative, IS] | **[Guideline]** |
| D4 | Depth | keystore | `lib.rs:11-15` | 15 public resolver functions | [Declarative, IS] | **[Guideline]** |
| D5 | Depth | storage | `lib.rs` | Per-module error types (anti-pattern) | [Declarative, IS] | **[Guideline]** |
| U-Cons | Unsafe | multi | test files | Repeated `set_var` unsafe pattern | [Declarative, IS] | **[Guideline]** |

---

## 8. Constraint-Force Summary

| Force | Count | Findings |
|-------|-------|----------|
| **Prohibition** | 0 | No inviolable violations |
| **Guardrail** | 2 | T3a, T3b: `.unwrap()` in library code (inference) |
| **Guideline** | 12 | T1a-e (error stringification), T2a-d (Box<dyn Error>), D3-D5 (depth), U-Cons (unsafe consolidation) |
| **Evidence** | 4 | T4 (boolean blindness), O1-O2 (ownership complexity), D1-D2 (depth scores) |
| **Hypothesis** | 0 | No subjunctive findings |

---

## 9. Verification Checklist

- [x] Every `pub` type in every core crate audited (via grep enumeration)
- [x] Every `unsafe` block reviewed (7 total, all documented)
- [x] No finding lacks a constraint-force classification
- [x] Epistemic mode stated for every finding
- [x] File:line references provided for actionable findings
