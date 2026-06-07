---
title: "hKask Architecture Master"
audience: [architects, developers, agents]
last_updated: 2026-06-07
version: "2.2.3"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Architecture Master

**Purpose:** Index to the four authoritative DDMVSS specification documents and supporting reference artifacts.

**Project:** hKask (ℏKask - "A Minimal Viable Container for Agents") v0.23.0
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## DDMVSS Specification Documents

The architecture is specified in four DDMVSS-aligned documents, each authoritative for its category cluster:

| Document | DDMVSS Categories | Scope |
|----------|-------------------|-------|
| [`domain-and-capability.md`](domain-and-capability.md) | Domain, Capability | Bounded context, entities, agent taxonomy, capability model, MCP tool surface, hLexicon |
| [`interface-and-composition.md`](interface-and-composition.md) | Interface, Composition | MCP/CLI/API equivalence, hexagonal ports, unified registry, template cascade, rendering pipeline |
| [`trust-security-observability.md`](trust-security-observability.md) | Trust, Observability | Zero-trust model, OCAP enforcement, master key derivation, encryption stack, CNS spans, algedonic alerts, threat model |
| [`persistence-and-lifecycle.md`](persistence-and-lifecycle.md) | Persistence, Lifecycle | SQLite + SQLCipher, bitemporal triples, embeddings, bootstrap sequence, evolution rules |

---

## Framework Documents

| Document | Purpose |
|----------|---------|
| [`DDMVSS.md`](DDMVSS.md) | Domain-Driven Minimum Viable Specification Set — 9-category taxonomy and MVSDD methodology |
| [`PRINCIPLES.md`](PRINCIPLES.md) | Architecture principles (P1-P7, C1-C7), five anchors, anti-patterns |
| [`loop-architecture.md`](loop-architecture.md) | 6-loop architecture — authority DAG, crate↔loop mapping, capability membranes |
| [`magna-carta.md`](magna-carta.md) | User sovereignty charter — catch-and-release, kill-zone detection |

---

## Reference Artifacts

Detailed lookup tables and diagrams in `reference/`:

| Artifact | Purpose |
|----------|---------|
| [`reference/hKask-erd.md`](reference/hKask-erd.md) | Core entity relationship diagrams |
| [`reference/registry-erd.md`](reference/registry-erd.md) | Registry schema diagrams |
| [`reference/subsystem-erds.md`](reference/subsystem-erds.md) | Per-crate ERDs |
| [`reference/hKask-hLexicon.md`](reference/hKask-hLexicon.md) | Full 87-term vocabulary catalog |
| [`reference/ports-inventory.md`](reference/ports-inventory.md) | Hexagonal port trait signatures |
| [`reference/utoipa-implementation.md`](reference/utoipa-implementation.md) | OpenAPI generation guide |
| [`reference/template-header-standard.md`](reference/template-header-standard.md) | Template metadata format |
| [`reference/hKask-Curator-persona.md`](reference/hKask-Curator-persona.md) | Curator persona specification |
| ~~`reference/distillation-erd.md`~~ | ~~Post-distillation authority DAG and ERD~~ — **Archived**; canonical ERDs remain active |
| [`reference/okapi-integration.md`](reference/okapi-integration.md) | Okapi LLM API contract |


---

## Decision Records

| ADR | Topic |
|-----|-------|
| [`ADR-022-comprehensive-security-hardening.md`](ADR-022-comprehensive-security-hardening.md) | ADV-REVIEW-F2 security hardening (T01-T22) |
| ~~[`ADR-023-master-key-derivation.md`](ADR-023-master-key-derivation.md)~~ | ~~Master key derivation via HKDF-SHA256~~ — **Archived**; superseded by [ADR-027](ADR-027-argon2-hkdf-master-key.md) |
| [`ADR-024-unified-registry.md`](ADR-024-unified-registry.md) | Unified registry with `template_type` discriminator (retroactive) |
| [`ADR-025-attenuation-depth-limit.md`](ADR-025-attenuation-depth-limit.md) | 7-level attenuation depth limit (retroactive) |
| [`ADR-026-bitemporal-triple-schema.md`](ADR-026-bitemporal-triple-schema.md) | Bitemporal triple schema with valid-time × transaction-time (retroactive) |
| [`ADR-027-argon2-hkdf-master-key.md`](ADR-027-argon2-hkdf-master-key.md) | Argon2id + HKDF-SHA256 master key derivation (retroactive) |
| ~~[`ADR-028-acp-protocol-design.md`](ADR-028-acp-protocol-design.md)~~ | ~~ACP protocol design — JSON-RPC 2.0 over stdio~~ — **Archived**; deferred (ACP transport layer removed) |
| ~~[`ADR-029-goal-capability-primitive.md`](ADR-029-goal-capability-primitive.md)~~ | ~~Goal capability primitive — distinct typed token~~ — **Archived**; superseded (`GoalCapabilityToken` removed; goals use `&WebID` owner scoping) |
| [`ADR-030-skill-bundler.md`](ADR-030-skill-bundler.md) | Skill bundler — meta-skill composition |
| [`ADR-031-consolidation-authorization.md`](ADR-031-consolidation-authorization.md) | Consolidation authorization via master passphrase derivation |

---

## Specifications

| Document | Purpose |
|----------|---------|
| [`../specifications/REQUIREMENTS.md`](../specifications/REQUIREMENTS.md) | 22 implemented + 5 deferred goal specs |
| [`../specifications/TRACEABILITY_MATRIX.md`](../specifications/TRACEABILITY_MATRIX.md) | Bidirectional code→test traceability |


---

*Verification commands:* `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`. See [`DDMVSS_SCAFFOLD.md`](../specifications/DDMVSS_SCAFFOLD.md) §6 for the full verification gate table.

---

## Document Structure

```
docs/architecture/
├── hKask-architecture-master.md           # THIS FILE (index)
├── DDMVSS.md                              # Framework
├── PRINCIPLES.md                          # Framework
├── loop-architecture.md                   # Framework (6-loop authority model)
├── magna-carta.md                         # Framework
├── domain-and-capability.md               # SPEC (Domain + Capability)
├── interface-and-composition.md           # SPEC (Interface + Composition)
├── trust-security-observability.md        # SPEC (Trust + Observability)
├── persistence-and-lifecycle.md           # SPEC (Persistence + Lifecycle)
├── ADR-022-comprehensive-security-hardening.md  # Decision record
├── ~~ADR-023-master-key-derivation.md~~   # ARCHIVED (superseded by ADR-027)
├── ADR-024-unified-registry.md            # Decision record
├── ADR-025-attenuation-depth-limit.md     # Decision record
├── ADR-026-bitemporal-triple-schema.md    # Decision record
├── ADR-027-argon2-hkdf-master-key.md      # Decision record
├── ~~ADR-028-acp-protocol-design.md~~        # ARCHIVED (deferred)
├── ~~ADR-029-goal-capability-primitive.md~~   # ARCHIVED (superseded)
├── ADR-030-skill-bundler.md                # Decision record
├── ADR-031-consolidation-authorization.md  # Decision record
└── reference/
    ├── hKask-erd.md                       # Diagram artifact
    ├── registry-erd.md                    # Diagram artifact
    ├── subsystem-erds.md                  # Diagram artifact
    ├── hKask-hLexicon.md                  # Vocabulary catalog
    ├── ports-inventory.md                 # Port reference
    ├── utoipa-implementation.md           # API guide
    ├── template-header-standard.md        # Format reference
    ├── hKask-Curator-persona.md           # Persona spec
    └── okapi-integration.md               # Okapi API contract
```

**Total:** 22 active architecture documents (4 specs + 4 framework + 1 index + 7 active ADRs + 6 active reference artifacts). Archived: ADR-023 (superseded by ADR-027), ADR-028 (deferred), ADR-029 (superseded), distillation-erd.md (changes applied to codebase), IMPLEMENTATION-PLAN-simplification.md.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
