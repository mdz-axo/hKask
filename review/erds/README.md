# Seam ERDs — hKask Adversarial Review

> Eight Mermaid diagrams, one per deep seam. Each is the contract a reviewer
> reads in 30 seconds and reasons about in 5. Every ERD references a node in
> `../graphs/joined.ttl` so the diagrams stay in sync with the semantic graph.

## Index

| # | Seam | Class | Mermaid | Realization (per `joined.ttl`) |
|---|------|-------|---------|--------------------------------|
| 1 | `AgentPod` ↔ `MemoryStoragePort` | `erDiagram` | [pod_lifecycle.mmd](pod_lifecycle.mmd) | `concept:agent_pod` → `tech:hkask_agents` |
| 2 | `OCAPBoundary` ↔ `OcapCapability` ↔ `DelegationToken` | `erDiagram` | [ocap_boundary.mmd](ocap_boundary.mmd) | `concept:ocap` → `tech:hkask_types` |
| 3 | `Spec` ↔ `Goal` ↔ `Criterion` ↔ `Artifact` | `erDiagram` | [spec_goal.mmd](spec_goal.mmd) | `concept:goal` → `tech:hkask_storage` |
| 4 | `NuEvent` ↔ SpanCategory ↔ `DecayConfig` | `erDiagram` | [nu_event_decay.mmd](nu_event_decay.mmd) | `concept:nu_event` → `tech:hkask_storage` |
| 5 | Bitemporal triple ↔ `Visibility` ↔ `Snapshot` | `erDiagram` | [bitemporal_triple.mmd](bitemporal_triple.mmd) | `concept:bitemporal_triple` → `tech:hkask_storage` |
| 6 | CNS loop wiring | `classDiagram` | [cns_loops.mmd](cns_loops.mmd) | `concept:cns` → `tech:hkask_cns` |
| 7 | HHH pipeline | `classDiagram` | [hhh_pipeline.mmd](hhh_pipeline.mmd) | `concept:hhh_pipeline` → `tech:hkask_agents` |
| 8 | MCP server ↔ Tool ↔ Capability gate | `classDiagram` | [mcp_membrane.mmd](mcp_membrane.mmd) | `concept:mcp` → `tech:hkask_mcp` |

## Conventions

- **Star zero-or-many**, **one dash one-or-one**, **pipe-or one-or-one**.
- Numeric limits go in `comment` lines under the relation, not in free text.
- A diagram that says "exactly 1" must be a hard structural invariant;
  if it's only sometimes, use `1..*` and a `// optional` note.
- All entities are `tech:<crate>__<Type>` if defined in code, or just the
  canonical name if from the functional graph.

## Drift guard

The numeric limits asserted in these ERDs (attenuation ≤ 7, override cooldown
120s, backpressure threshold 100, algedonic warn at R-bar 0.3 / critical 0.8,
decay half-lives per category) are referenced from
`crates/hkask-types/src/capability/mod.rs:7`,
`crates/hkask-cns/src/dampener.rs:58`,
`crates/hkask-types/src/cns.rs:63`,
`crates/hkask-cns/src/algedonic.rs:35,40`,
`crates/hkask-storage/src/nu_event_store.rs:571` (the test that asserts the
half-lives). If any of those numbers drift, the ERD is wrong and the
synthesis agent (Task 3) must surface it as a finding.
