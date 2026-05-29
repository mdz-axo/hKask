---
title: "hKask Traceability Matrix"
audience: [architects, developers, auditors]
last_updated: 2026-05-29
version: "1.1.1"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Traceability Matrix

**Purpose:** Bidirectional traceability from DDMVSS goal specs → code → tests, organized by DDMVSS category.

**Related:** [`REQUIREMENTS.md`](REQUIREMENTS.md), [`domain-and-capability.md`](../architecture/domain-and-capability.md), [`interface-and-composition.md`](../architecture/interface-and-composition.md), [`trust-security-observability.md`](../architecture/trust-security-observability.md), [`persistence-and-lifecycle.md`](../architecture/persistence-and-lifecycle.md)

**Scope-exempt from Sourced-Ideas Mandate** — this is a cross-reference table, not a design document.

---

## Domain

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-DOM-001 | Bounded context identity | `hkask-types` | `id`, `event`, `agent_def` | `WebID`, `NuEvent`, `AgentDefinition` | — | ✅ Implemented |
| REQ-DOM-002 | ν-event observability primitive | `hkask-types` | `event` | `NuEvent`, `Span`, `NuEventSink` | — | ✅ Implemented |
| REQ-DOM-003 | hLexicon vocabulary grounding | `hkask-types`, `hkask-templates` | `lexicon`, `contract_validator` | `HLexicon::canonical`, `ContractValidator` | `lexicon::tests::canonical_lexicon_matches_catalog` | ✅ Implemented |
| REQ-DOM-004 | hLexicon single-source derivation (markdown → Rust, CI drift gate) | `hkask-types` | `lexicon`, `hlexicon_generated` | `HLexicon::canonical`, `generate-hlexicon.py`, `check-hlexicon.sh` | `lexicon::tests::*`, `docs/ci/check-hlexicon.sh` | ✅ Implemented |

## Capability

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-CAP-001 | OCAP access control | `hkask-types` | `visibility` | `Capability`, `AccessEvaluator` | — | ✅ Implemented |
| REQ-CAP-002 | Capability attenuation | `hkask-types` | `visibility` | `Delegation`, `DelegationStore`, `RevocationList` | — | ✅ Implemented |
| REQ-CAP-003 | MCP tool surface | `hkask-mcp` | `runtime`, `security`, `transport` | `McpRuntime`, `SecurityGateway`, `McpTransport` | — | ✅ Implemented |
| REQ-CAP-004 (ADR-029, P0-03) | Goal capability — unforgeable authority bound into HMAC; owner/visibility checks co-located with every write; legal-transition enforcement | `hkask-types`, `hkask-storage` | `goal_capability`, `goal`, `goals` | `GoalCapabilityToken`, `GoalState::can_transition_to`, `SqliteGoalRepository` | `goal_capability::tests` (forgery, expiry, attenuation, order-invariance), `goal::tests` (transitions), `goals::tests` (confused-deputy, transition, owner-only delete) | ✅ Implemented |

## Interface

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-IFC-001 | MCP ≡ CLI ≡ API equivalence | `hkask-cli`, `hkask-api`, `hkask-mcp` | `main`, `lib`, `runtime` | `Commands`, `create_router`, `McpRuntime` | — | ✅ Implemented |
| REQ-IFC-001a (P0-05) | Goal subsystem exposed on all three surfaces (parity exemplar) | `hkask-cli`, `hkask-api`, `hkask-mcp-goal` | `commands/goal`, `routes/goal`, `main` | `kask goal`, `goal_router`, `GoalServer` (`goal_create`/`goal_list`/`goal_set_state`) | `hkask-mcp-goal` tests (create/list round-trip, illegal transition, invalid visibility) | ✅ Implemented |
| REQ-IFC-002 | OpenAPI documentation | `hkask-api` | `openapi` | `ApiDoc` | — | ✅ Implemented |

## Composition

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-COM-001 | Unified template registry | `hkask-templates` | root | `SqliteRegistry`, `ContractValidator` | Doctests only (3 ok, 1 ignored) | ✅ Implemented |
| REQ-COM-002 | Template cascade depth limit | `hkask-templates` | `dependency`, `resolver` | `DependencyGraph`, `TemplateResolver` | — | ✅ Implemented |
| REQ-COM-003 | Agent pod composition | `hkask-agents` | `pod`, `consent` | `AgentPod`, `PodManager`, `ConsentManager` | — | ✅ Implemented |

## Trust & Security

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-TRU-001 | Zero-trust defaults | `hkask-mcp` | `security` | `SecurityPolicy`, `SecurityGateway` | — | ✅ Implemented |
| REQ-TRU-002 | Encrypted storage at rest | `hkask-storage` | `database` | `Database` (SQLCipher) | — | ✅ Implemented |
| REQ-TRU-003 | Deterministic identity | `hkask-types`, `hkask-agents` | `id`, `pod` | `WebID`, `AgentIdentity` | — | ✅ Implemented |

## Observability

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-OBS-001 | CNS span emission | `hkask-types`, `hkask-cns` | `event`, `runtime` | `Span`, `CnsRuntime` | — | ✅ Implemented |
| REQ-OBS-002 | Algedonic alerting | `hkask-cns`, `hkask-types` | `algedonic`, `cns` | `AlgedonicManager`, `AlgedonicAlert` | — | ✅ Implemented |
| REQ-OBS-003 (ADR-029, P0-03) | Goal capability denials are observable — emit `cns.tool.goal.capability.denied` ν-events via injected `NuEventSink` port (non-fatal) | `hkask-storage`, `hkask-types`, `hkask-api`, `hkask-mcp-goal` | `goals`, `event`, `routes/goal`, `main` | `SqliteGoalRepository::{with_telemetry, emit_denial}`, `NuEventSink`, `ApiState.goal_repo`, `GoalServer` | `goals::tests` (denial telemetry, non-fatal sink), `hkask-cns` `goal_capability_cybertests` (cyber_), `hkask-mcp-goal` tests | ✅ Implemented |

## Persistence

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-PER-001 | Bitemporal triple storage | `hkask-storage` | `triples` | `TripleStore`, `Triple` | — | ✅ Implemented |
| REQ-PER-002 | Embedding vector search | `hkask-storage` | `embeddings` | `EmbeddingStore`, `KnnResult` | — | ✅ Implemented |

## Lifecycle

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-LIF-001 | Bootstrap sequence | `hkask-cli` | `main` | `main()` | — | ✅ Implemented |
| REQ-LIF-002 | Forward-only evolution | — | Architecture invariant | Policy | — | ✅ Implemented |

## Curation

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-CUR-001 | DDMVSS specification tools | `hkask-mcp-spec`, `hkask-types`, `hkask-storage` | `lib`, `spec`, `spec_store` | 8 MCP tools, `SpecStore`, `SqliteSpecStore` | — | ✅ Implemented |
| REQ-CUR-002 | Curation decision gradient | `hkask-types` | `spec`, `curation` | `SpecCurationRecord`, `CurationDecision` | — | ✅ Implemented |

---

## Summary

| Category | Implemented | Partially | Deferred | Total |
|----------|------------|-----------|----------|-------|
| Domain | 3 | 0 | 0 | 3 |
| Capability | 4 | 0 | 2 | 6 |
| Interface | 3 | 0 | 1 | 4 |
| Composition | 3 | 0 | 1 | 4 |
| Trust & Security | 3 | 0 | 0 | 3 |
| Observability | 3 | 0 | 0 | 3 |
| Persistence | 2 | 0 | 1 | 3 |
| Lifecycle | 2 | 0 | 0 | 2 |
| Curation | 2 | 0 | 0 | 2 |
| **Total** | **25** | **0** | **5** | **30** |

**DDMVSS completeness:** 25/25 implemented requirements satisfied. 5 deferred with documented rationale (see [`REQUIREMENTS.md`](REQUIREMENTS.md) §11). `curated?` holds — every requirement has a curation decision.

**Test coverage note (updated 2026-05-29):** The goal-capability hardening (ADR-029, P0-03) added dedicated `#[test]` coverage: forgery/expiry/attenuation and transition tests in `hkask-types` (`goal_capability`, `goal`), confused-deputy/transition/owner-only-delete and denial-telemetry tests in `hkask-storage` (`goals`), and two `cyber_`-prefixed cybernetic tests in `hkask-cns` (`goal_capability_cybertests`). `cargo test --workspace` is green. Other DDMVSS requirements remain primarily doctest- or inspection-verified pending broader test expansion (P0-02).
