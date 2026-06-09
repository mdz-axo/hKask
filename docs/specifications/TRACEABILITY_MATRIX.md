---
title: "hKask Traceability Matrix"
audience: [architects, developers, auditors]
last_updated: 2026-06-07
version: "1.3.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Traceability Matrix

**Purpose:** Bidirectional traceability from MDS goal specs → code → tests, organized by MDS category.

**Related:** [`REQUIREMENTS.md`](REQUIREMENTS.md), [`MDS.md §7.1-7.2`](../architecture/MDS.md §7.1-7.2), [`MDS.md §7.2`](../architecture/MDS.md §7.2), [`MDS.md §7.3`](../architecture/MDS.md §7.3), [`MDS.md §7.4`](../architecture/MDS.md §7.4)

**Scope-exempt from Sourced-Ideas Mandate** — this is a cross-reference table, not a design document.

---

## Domain

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-DOM-001 | Bounded context identity | `hkask-types` | `id`, `event`, `agent_def` | `WebID`, `NuEvent`, `AgentDefinition` | — | ✅ Implemented |
| REQ-DOM-002 | ν-event observability primitive | `hkask-types` | `event` | `NuEvent`, `Span`, `NuEventSink` | — | ✅ Implemented |
| REQ-DOM-003 | hLexicon vocabulary grounding | `hkask-types`, `hkask-templates` | `lexicon` | `HLexicon`, `load_hlexicon_from_yaml` | `hkask-templates::lexicon` module tests | \u2705 Implemented |
| REQ-DOM-004 | hLexicon single-source derivation (markdown \u2192 YAML, explicit regen, drift test) | `hkask-templates` | `lexicon` | `load_hlexicon_from_yaml`, `load_hlexicon_default` | `hkask-templates::lexicon` module tests | \u2705 Implemented |

## Capability

| Goal ID | Requirement | Crate | Module | Type/Function | Tests | Status |
|---------|------------|-------|--------|---------------|-------|--------|
| REQ-CAP-001 | OCAP access control | `hkask-types` | `capability`, `visibility` | `DelegationToken`, `AccessControl` | \u2014 | \u2705 Implemented |
| REQ-CAP-002 | Capability attenuation | `hkask-types` | `capability` | `DelegationToken` (attenuation_level) | \u2014 | \u2705 Implemented |
| REQ-CAP-003 | MCP tool surface | `hkask-mcp`, `hkask-cns` | `runtime`, `governed_tool` | `McpRuntime`, `GovernedTool` | \u2014 | \u2705 Implemented |
| REQ-CAP-004 (ADR-029, P0-03) | Goal capability — owner-scoped authority via `&WebID`; owner/visibility checks co-located with every write; legal-transition enforcement | `hkask-types`, `hkask-storage` | `goal`, `goals` | `WebID`, `GoalState::can_transition_to`, `SqliteGoalRepository` | `goal::tests` (transitions), `goals::tests` (transition, owner-only delete) | ✅ Implemented |

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
| REQ-OBS-003 (ADR-029, P0-03) | Goal operations use WebID-based owner scoping — access is determined by `&WebID` identity rather than capability tokens; no capability denials to observe (ADR-029 archived) | `hkask-types`, `hkask-storage`, `hkask-api`, `hkask-mcp-goal` | `goal`, `goals`, `routes/goal`, `main` | `WebID`, `SqliteGoalRepository`, `ApiState.goal_repo`, `GoalServer` | `goals::tests` (owner-scoped access), `hkask-mcp-goal` tests | ✅ Implemented |

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
| REQ-CUR-001 | MDS specification tools | `hkask-mcp-spec`, `hkask-storage`, `hkask-agents` | `lib`, `spec_types`, `spec_store`, `curator_agent/spec_curator` | 8 MCP tools, `SpecStore`, `SqliteSpecStore`, `DefaultSpecCurator` | \u2014 | \u2705 Implemented |
| REQ-CUR-002 | Curation decision gradient | `hkask-storage`, `hkask-types` | `spec_types`, `curation` | `SpecCurationRecord`, `CurationDecision` | \u2014 | \u2705 Implemented |

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

**MDS completeness:** 25/25 implemented requirements satisfied. 5 deferred with documented rationale (see [`REQUIREMENTS.md`](REQUIREMENTS.md) §11). `curated?` holds — every requirement has a curation decision.

**Test coverage note (updated 2026-06-04):** The goal-capability hardening (originally ADR-029, P0-03; ADR-029 archived — `GoalCapabilityToken` removed) now uses WebID-based owner scoping. `GoalCapabilityToken` and associated forgery/expiry/attenuation tests were removed in v0.23.0. Remaining dedicated `#[test]` coverage: transition tests in `hkask-types` (`goal`), owner-only-delete tests in `hkask-storage` (`goals`). `cargo test --workspace` is green. Other MDS requirements remain primarily doctest- or inspection-verified pending broader test expansion (P0-02).
