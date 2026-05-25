---
title: "hKask Traceability Matrix"
audience: [architects, developers, auditors]
last_updated: 2026-05-25
version: "1.0.0"
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
| REQ-DOM-001 | Bounded context identity | `hkask-types` | `id`, `event`, `agent_def` | `WebID`, `NuEvent`, `AgentDefinition` | 6 test files | ✅ Implemented |
| REQ-DOM-002 | ν-event observability primitive | `hkask-types` | `event` | `NuEvent`, `Span`, `NuEventSink` | event.rs tests | ✅ Implemented |
| REQ-DOM-003 | hLexicon vocabulary grounding | `hkask-types` | lexicon | Bootstrap terms | — | ✅ Implemented |

## Capability

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-CAP-001 | OCAP access control | `hkask-types` | `visibility` | `Capability`, `AccessEvaluator` | visibility.rs tests | ✅ Implemented |
| REQ-CAP-002 | Capability attenuation | `hkask-types` | `visibility` | `Delegation`, `DelegationStore`, `RevocationList` | visibility.rs tests | ✅ Implemented |
| REQ-CAP-003 | MCP tool surface | `hkask-mcp` | `runtime`, `security`, `transport` | `McpRuntime`, `SecurityGateway`, `McpTransport` | 1 test file | ✅ Implemented |

## Interface

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-IFC-001 | MCP ≡ CLI ≡ API equivalence | `hkask-cli`, `hkask-api`, `hkask-mcp` | `main`, `lib`, `runtime` | `Commands`, `create_router`, `McpRuntime` | — | ✅ Implemented |
| REQ-IFC-002 | OpenAPI documentation | `hkask-api` | `openapi` | `ApiDoc` | — | ✅ Implemented |

## Composition

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-COM-001 | Unified template registry | `hkask-templates` | root | `SqliteRegistry`, `ContractValidator` | 6 test files | ✅ Implemented |
| REQ-COM-002 | Template cascade depth limit | `hkask-templates` | `dependency`, `resolver` | `DependencyGraph`, `TemplateResolver` | — | ✅ Implemented |
| REQ-COM-003 | Agent pod composition | `hkask-agents` | `pod`, `consent` | `AgentPod`, `PodManager`, `ConsentManager` | 7 test files | ✅ Implemented |

## Trust & Security

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-TRU-001 | Zero-trust defaults | `hkask-mcp` | `security` | `SecurityPolicy`, `SecurityGateway` | — | ✅ Implemented |
| REQ-TRU-002 | Encrypted storage at rest | `hkask-storage` | `database` | `Database` (SQLCipher) | 8 test files | ✅ Implemented |
| REQ-TRU-003 | Deterministic identity | `hkask-types`, `hkask-agents` | `id`, `pod` | `WebID`, `AgentIdentity` | — | ✅ Implemented |

## Observability

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-OBS-001 | CNS span emission | `hkask-types`, `hkask-cns` | `event`, `runtime` | `Span`, `CnsRuntime` | — | ✅ Implemented |
| REQ-OBS-002 | Algedonic alerting | `hkask-cns`, `hkask-types` | `algedonic`, `cns` | `AlgedonicManager`, `AlgedonicAlert` | — | ✅ Implemented |

## Persistence

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-PER-001 | Bitemporal triple storage | `hkask-storage` | `triples` | `TripleStore`, `Triple` | triples.rs tests | ✅ Implemented |
| REQ-PER-002 | Embedding vector search | `hkask-storage` | `embeddings` | `EmbeddingStore`, `KnnResult` | embeddings.rs tests | ✅ Implemented |

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
| Capability | 3 | 0 | 3 | 6 |
| Interface | 2 | 0 | 1 | 3 |
| Composition | 3 | 0 | 1 | 4 |
| Trust & Security | 3 | 0 | 0 | 3 |
| Observability | 2 | 0 | 0 | 2 |
| Persistence | 2 | 0 | 1 | 3 |
| Lifecycle | 2 | 0 | 0 | 2 |
| Curation | 2 | 0 | 0 | 2 |
| **Total** | **22** | **0** | **6** | **28** |

**DDMVSS completeness:** 22/22 implemented requirements satisfied. 6 deferred with documented rationale. `curated?` holds — every requirement has a curation decision.
