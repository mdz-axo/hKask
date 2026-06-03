---
title: "hKask Distillation ERD — Post-Distillation Entity Relationships"
audience: [architects, developers, agents]
last_updated: 2026-06-02
version: "0.22.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, curation]
---

# hKask Distillation ERD

**Version:** v0.22.0 (post-distillation)
**Status:** Active — reflects the codebase after semantic distillation

## Authority DAG

```mermaid
graph TD
    CUR[Curation Loop 5] --> CYB[Cybernetics Loop 6]
    CYB --> INF[Inference Loop 1]
    CYB --> EPI[Episodic Loop 2a]
    CYB --> SEM[Semantic Loop 2b]
    CYB --> COM[Communication Loop 4]
    EPI -->|consolidation bridge| SEM

    style CUR fill:#e1f5fe
    style CYB fill:#fff3e0
    style INF fill:#e8f5e9
    style EPI fill:#f3e5f5
    style SEM fill:#fce4ec
    style COM fill:#e0f2f1
```

No sideways edges. Authority flows downward. The consolidation bridge (EPI→SEM) is one-way, gated by `ConsolidationToken`.

## Core Entity Relationships

```mermaid
erDiagram
    CURATION ||--o{ CURATOR_DIRECTIVE : issues
    CURATION ||--o{ CURATION_DECISION : evaluates
    CURATION ||--o{ CURATOR_HANDLE : owns
    CYPBERNETICS ||--o{ ENERGY_BUDGET : governs
    CYPBERNETICS ||--o{ CIRCUIT_BREAKER : governs
    CYPBERNETICS ||--o{ DAMPENER : governs
    CYPBERNETICS ||--o{ ALGEDONIC_MANAGER : governs
    CYPBERNETICS ||--o{ VARIETY_TRACKER : governs
    CYPBERNETICS ||--o{ CYBERNETICS_TOKEN : issues
    CYPBERNETICS ||--o{ CONSOLIDATION_TOKEN : issues
    INFERENCE ||--o{ LLM_PARAMETERS : uses
    INFERENCE ||--o{ TEMPLATE_INVOCATION : uses
    INFERENCE ||--o{ GOVERNED_INFERENCE : "membrane"
    EPISODIC ||--o{ EPISODIC_READ_HANDLE : owns
    EPISODIC ||--o{ EPISODIC_WRITE_HANDLE : owns
    SEMANTIC ||--o{ CONSOLIDATION_OUTCOME : produces
    COMMUNICATION ||--o{ LOOP_MESSAGE : routes
    CYPBERNETICS }o--|| CURATION : "authority DAG"
    INFERENCE }o--|| CYPBERNETICS : "authority DAG"
    EPISODIC }o--|| CYPBERNETICS : "authority DAG"
    SEMANTIC }o--|| CYPBERNETICS : "authority DAG"
    COMMUNICATION }o--|| CYPBERNETICS : "authority DAG"
    EPISODIC }o--|| SEMANTIC : "one-way bridge"

    CAPABILITY_TOKEN ||--o{ CAVEAT : constrained_by
    DATA_CATEGORY ||--o{ VISIBILITY : "default_visibility"
    OCAP_BOUNDARY ||--|| CAPABILITY : bounds
    KILL_ZONE_CONFIG ||--o{ CNS : "sensed by"
    NUEVENT ||--|| SPAN_NAMESPACE : categorized_by
    PHASE ||--|| LOOP : "4-stage cycle"
```

## Capability Token Authority Flow

```mermaid
graph LR
    CUR[CuratorHandle] -->|issues| CT[CurationToken]
    CUR -->|issues| CONS[ConsolidationToken]
    CYB[CyberneticsLoop] -->|issues| CYT[CyberneticsToken]
    CYB -->|issues| CONS

    CT -->|authorizes| OVERRIDE[Cybernetics Override]
    CYT -->|authorizes| BUDGET[Energy Budget Change]
    CYT -->|authorizes| CIRCUIT[Circuit Break Toggle]
    CYT -->|authorizes| DAMP[Dampening]
    CONS -->|authorizes| BRIDGE[Episodic → Semantic]

    style CUR fill:#e1f5fe
    style CYB fill:#fff3e0
    style CT fill:#bbdefb
    style CYT fill:#ffe0b2
    style CONS fill:#c8e6c9
```

## Distillation Changes Summary

| # | Change | Root Cause | Loop |
|---|--------|-----------|------|
| 1 | Remove `LoopId::External` | Dead variant, zero consumers | Cross-cutting |
| 2 | Replace `SpanCategory` enum with `SpanNamespace` newtype | Structural duplication, OCP violation | Cybernetics |
| 3 | Add `Phase::Compare`, rename `Observe→Sense`, `Regulate→Compute`, `Outcome→Act` | Phase didn't match 4-stage cycle | Cross-cutting |
| 4 | Move `SoapInferenceConfig` I/O to CLI, keep `InferenceConfig` in types | Types crate had I/O and config | Inference |
| 5 | Remove `AuthorityLevel` enum | OCAP anti-pattern (Implicit authority) | Curation |
| 6 | Add ZST capability tokens (`CyberneticsToken`, `CurationToken`, `ConsolidationToken`) | String-based capability forgery | Cybernetics |
| 9 | Enforce AUTHORITY_ORDER in LoopSystem tick | Loops ticked in registration order, not authority order | Cross-cutting |
| 15 | Remove speculative `AgentDefinition` fields | P6: delete stubs, don't publish | Curation |
| 16 | Move `KillZoneDetector` logic to CNS, keep `KillZoneConfig` in types | Regulation logic in type crate | Cybernetics |
| 17 | Add `DataCategory::default_visibility()` | Scattered visibility mapping | Cybernetics |
| 18 | Wire `ConsolidationToken` into `ConsolidationPort` | One-way bridge had no capability gate | Cybernetics |