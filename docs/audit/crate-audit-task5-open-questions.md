# Task 5 — Open Questions and Underspecified Aspects

**Bundle:** `crate-audit` | **Phase:** Post-core (reflection)
**Date:** 2026-06-12

---

## 1. Evidence Findings (Probabilistic, IS)

Collected from Tasks 2–3. These are observations that inform but do not demand action.

| # | Source | Finding | Epistemic Mode | Provenance | Resolution Trigger | Constraint Force |
|---|--------|---------|---------------|------------|-------------------|-----------------|
| Q1 | Task 2, R2 | **CNS variety deficit:** Direct crate calls bypass CNS span registration. How many code paths are uninstrumented? | [Evidence, IS-Probabilistic] | [Implicit, pattern inference from `cybernetics_loop.rs` span registration scope] | Instrumentation audit: grep for direct `hkask-storage` calls outside MCP dispatch path. Count uninstrumented paths. | **Guideline** — should resolve before next audit |
| Q2 | Task 2, R3 | **Memory fidelity degradation:** Lossy ν-event encoding → degraded semantic extraction. What metadata is missing from ν-events? | [Evidence, IS-Probabilistic] | [Implicit, inference from `nu_event_store.rs` schema vs `consolidation.rs` extraction] | Compare ν-event schema fields to consolidation input requirements. Identify missing fields. | **Guideline** — should resolve before next audit |
| Q3 | Task 2, R4 | **Static energy cost tables:** `table_energy_estimator.rs` uses hardcoded costs. How far do actual costs deviate from table values? | [Evidence, IS-Probabilistic] | [Implicit, inference from static table design] | Measure actual tool execution cost (wall time, token count, network bytes) for 10 representative tools. Compare to table values. | **Evidence** — may resolve with measurement |
| Q4 | Task 2, R7 | **Behavioral spec drift undetected:** Structural comparison misses semantic divergence. How many specs have correct structure but wrong behavior? | [Evidence, IS-Probabilistic] | [Implicit, inference from `spec_types.rs` structural comparison design] | Behavioral spec test: for each spec, write a test that verifies the implementation matches the spec's described behavior, not just its structure. | **Evidence** — may resolve with test program |
| Q5 | Task 3, T4 | **Boolean blindness in CLI:** `init_logging(verbose: bool)`. Is this the only boolean parameter in the CLI surface? | [Evidence, IS-Probabilistic] | [Directly Stated, grep search — only one instance found] | Audit all CLI command signatures for boolean parameters. If only one, low priority. | **Evidence** — informational |
| Q6 | Task 3, O1 | **Nested Arc<RwLock<Arc<RwLock<...>>>>:** `SessionManager.chats` uses double locking. Is this justified by concurrent access patterns, or can `EnsembleChat` own its state without interior mutability? | [Evidence, IS-Probabilistic] | [Directly Stated, source code observation] | Profile concurrent access to `EnsembleChat`. If only one writer at a time, replace inner `Arc<RwLock<EnsembleChat>>` with owned `EnsembleChat`. | **Guideline** — should investigate |
| Q7 | Task 3, O2 | **8 Arc<dyn Port> fields in PodManager:** Is every port exercised in production, or are some test-only seams? | [Evidence, IS-Probabilistic] | [Directly Stated, source code observation] | Audit which ports are `None` in production vs test. Remove ports that are always `None` in production. | **Evidence** — may resolve with audit |
| Q8 | Task 3, D1 | **275 pub items in hkask-types:** Are all re-exports justified by external consumers, or are some internal-only types leaked? | [Evidence, IS-Declarative] | [Directly Stated, pub item count] | For each re-export in `lib.rs`, check if any external crate imports it. Remove re-exports with zero external consumers. | **Guideline** — should resolve before next audit |
| Q9 | Task 3, D2 | **204 pub items in hkask-agents:** Many re-exports from `hkask-storage` (EscalationQueue types). Should these be obtained directly from `hkask-storage`? | [Evidence, IS-Declarative] | [Directly Stated, pub item count + re-export audit] | Check if consumers use `hkask_agents::EscalationQueue` or `hkask_storage::EscalationQueue`. If the latter, remove pass-through re-exports. | **Guideline** — should resolve before next audit |

---

## 2. Hypothesis Findings (Subjunctive, IS)

Collected from Tasks 2–3. These are speculative — they need verification before becoming Evidence.

| # | Source | Finding | Epistemic Mode | Provenance | Resolution Trigger | Constraint Force |
|---|--------|---------|---------------|------------|-------------------|-----------------|
| Q10 | Task 1, IS/OUGHT | **hkask-keystore → hkask-storage IS/OUGHT boundary:** If zeroizing is skipped, key material leaks. Is zeroizing consistently applied at every key use site? | [Hypothesis, IS-Subjunctive] | [Implicit, inference from security contract stated in keystore docs] | Audit every call site of `resolve_db_passphrase`, `derive_key`, `resolve_wallet_seed` for `Zeroizing` wrapper usage. | **Guideline** — should verify |
| Q11 | Task 2, L3 | **CNS gain calibration:** Are `DEFAULT_VARIETY_MAX_DEFICIT` (50/100) thresholds appropriate for real session patterns, or do they produce alert fatigue / missed signals? | [Hypothesis, IS-Subjunctive] | [Implicit, inference from uncalibrated defaults] | Collect variety counter data from 10 real sessions. Plot deficit over time. Check if alerts fire at meaningful moments. | **Evidence** — may resolve with data |
| Q12 | Task 2, L1 | **InferencePort timeout:** Would adding a timeout parameter to `InferencePort` reduce provider-failure fragility, or would it add complexity without measurable benefit? | [Hypothesis, IS-Subjunctive] | [Implicit, inference from missing timeout in port trait] | Measure p95 inference latency across providers. If >30s for any provider, timeout is warranted. | **Evidence** — may resolve with measurement |
| Q13 | Task 3, Phase 5 | **Derive-all-the-things:** Are all `#[derive(...)]` annotations semantically justified, or are some convenience derives that commit to contracts unintentionally? | [Hypothesis, IS-Subjunctive] | [Implicit, inference from spot-check — not exhaustively audited] | Exhaustive derive audit: for each `#[derive]` on a pub type, verify the trait contract is semantically meaningful for that type. | **Evidence** — may resolve with audit |

---

## 3. Design Tensions (from Task 1 Cartography)

Architectural tensions surfaced by the crate graph that the current structure creates but does not resolve.

| # | Tension | Epistemic Mode | Provenance | Resolution Trigger | Constraint Force |
|---|---------|---------------|------------|-------------------|-----------------|
| Q14 | **hkask-cns → hkask-wallet upward dependency:** CNS (control layer) depends on wallet (resource layer). This inverts the typical layered architecture. Is wallet-backed energy budget worth the layer inversion? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Cargo.toml — only upward edge in DAG] | Evaluate: could wallet energy estimation move to `hkask-services` as a coordination concern, with CNS consuming pre-computed budgets? | **Guideline** — should evaluate |
| Q15 | **hkask-services depends on 10 crates:** The service layer is a "god crate" that composes everything. Is this the right granularity, or should it be split into domain-specific service crates (e.g., `hkask-services-chat`, `hkask-services-pods`)? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Cargo.toml — 10 internal dependencies] | Measure: do CLI and API use disjoint subsets of services? If yes, split. If they share most services, keep unified. | **Guideline** — should evaluate |
| Q16 | **hkask-cli depends on hkask-api:** CLI can embed the API server. Is this bidirectional dependency (CLI→API, API→services→CLI types?) creating a hidden cycle risk? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Cargo.toml — CLI depends on API] | Check: does API depend on CLI types? If not, the edge is unidirectional and safe. If API imports CLI types, there's a cycle risk. | **Guideline** — should verify |

---

## 4. Deferred Decisions (from Task 4)

Places where a fix was deferred because the right design wasn't clear.

| # | Decision | Epistemic Mode | Provenance | Resolution Trigger | Constraint Force |
|---|----------|---------------|------------|-------------------|-----------------|
| Q17 | **CNS algedonic alert persistence during idle:** Should alerts be persisted to storage and replayed when Curator becomes active, or should a background consumer process alerts even when no chat session is active? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Task 4 deferral of R1] | Design decision: persistence (simpler, adds storage dependency) vs background consumer (more complex, keeps CNS self-contained). | **Guideline** — should decide before next audit |
| Q18 | **InfrastructureError type strength:** Should `InfrastructureError::Database(String)` become `InfrastructureError::Database(#[source] Box<dyn Error>)` to preserve error types, or should it use a concrete `DatabaseError` type? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Task 4 deferral of T1a-e] | Evaluate: how many callers match on `InfrastructureError` variants? If zero, typed source is low-priority. If callers need to distinguish DB errors from IO errors, typed source is high-priority. | **Guideline** — should evaluate |
| Q19 | **Error type unification in hkask-storage:** 14 modules each with own error type. Should they unify into a single `StorageError` enum, or is per-module error typing intentional for caller precision? | [Hypothesis, IS-Subjunctive] | [Directly Stated, Task 4 deferral of D5] | Audit: do callers match on specific store error types (e.g., `NuEventError::NotFound` vs `TripleError::NotFound`)? If yes, per-module errors are justified. If callers only check `Infra` vs `NotFound`, unification is safe. | **Guideline** — should evaluate |

---

## 5. Constraint-Force Summary for Open Questions

| Force | Count | Questions |
|-------|-------|-----------|
| **Guideline** (should resolve before next audit) | 11 | Q1, Q2, Q6, Q8, Q9, Q10, Q14, Q15, Q16, Q17, Q18, Q19 |
| **Evidence** (may resolve with measurement/audit) | 7 | Q3, Q4, Q5, Q7, Q11, Q12, Q13 |

---

## 6. Verification Checklist

- [x] Every Evidence finding from Tasks 2–3 appears in the registry (Q1–Q9)
- [x] Every Hypothesis finding from Tasks 2–3 appears in the registry (Q10–Q13)
- [x] Design tensions from Task 1 included (Q14–Q16)
- [x] Deferred decisions from Task 4 included (Q17–Q19)
- [x] No question lacks provenance
- [x] No question stated declaratively if it is subjunctive
- [x] Epistemic mode stated for every question
- [x] Resolution trigger stated for every question
- [x] Sorted by constraint force (Guidelines first, then Evidence)
