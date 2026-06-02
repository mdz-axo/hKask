---
title: "hKask Architecture Master"
audience: [architects, developers, agents]
last_updated: 2026-05-29
version: "2.2.1"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Architecture Master

**Purpose:** Index to the four authoritative DDMVSS specification documents and supporting reference artifacts.

**Project:** hKask (ℏKask — "A Minimal Viable Container for Agents") v0.21.0  
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
| [`reference/okapi-integration.md`](reference/okapi-integration.md) | Okapi LLM API contract |
| [`reference/loop-architecture.md`](reference/loop-architecture.md) | 6-loop architecture decomposition, crate mappings, capability membranes |

---

## Decision Records

| ADR | Topic |
|-----|-------|
| [`ADR-022-comprehensive-security-hardening.md`](ADR-022-comprehensive-security-hardening.md) | ADV-REVIEW-F2 security hardening (T01-T22) |
| [`ADR-023-master-key-derivation.md`](ADR-023-master-key-derivation.md) | Master key derivation via HKDF-SHA256, eliminate random secret generation, keystore persistence |
| [`ADR-024-unified-registry.md`](ADR-024-unified-registry.md) | Unified registry with `template_type` discriminator (retroactive) |
| [`ADR-025-attenuation-depth-limit.md`](ADR-025-attenuation-depth-limit.md) | 7-level attenuation depth limit (retroactive) |
| [`ADR-026-bitemporal-triple-schema.md`](ADR-026-bitemporal-triple-schema.md) | Bitemporal triple schema with valid-time × transaction-time (retroactive) |
| [`ADR-027-argon2-hkdf-master-key.md`](ADR-027-argon2-hkdf-master-key.md) | Argon2id + HKDF-SHA256 master key derivation (retroactive) |
| [`ADR-028-acp-protocol-design.md`](ADR-028-acp-protocol-design.md) | ACP protocol design — JSON-RPC 2.0 over stdio (retroactive) |

---

## Specifications

| Document | Purpose |
|----------|---------|
| [`../specifications/REQUIREMENTS.md`](../specifications/REQUIREMENTS.md) | 22 implemented + 5 deferred goal specs |
| [`../specifications/TRACEABILITY_MATRIX.md`](../specifications/TRACEABILITY_MATRIX.md) | Bidirectional code→test traceability |


---

## Verification

```bash
cargo check --workspace                    # Build
cargo test --workspace                     # Test
cargo clippy --workspace -- -D warnings    # Lint
cargo fmt --check                          # Format
```

---

## Document Structure

```
docs/architecture/
├── hKask-architecture-master.md           # THIS FILE (index)
├── DDMVSS.md                              # Framework
├── PRINCIPLES.md                          # Framework
├── magna-carta.md                         # Framework
├── domain-and-capability.md               # SPEC (Domain + Capability)
├── interface-and-composition.md           # SPEC (Interface + Composition)
├── trust-security-observability.md        # SPEC (Trust + Observability)
├── persistence-and-lifecycle.md           # SPEC (Persistence + Lifecycle)
├── ADR-022-comprehensive-security-hardening.md  # Decision record
├── ADR-023-master-key-derivation.md       # Decision record
├── ADR-024-unified-registry.md            # Decision record
├── ADR-025-attenuation-depth-limit.md     # Decision record
├── ADR-026-bitemporal-triple-schema.md    # Decision record
├── ADR-027-argon2-hkdf-master-key.md      # Decision record
├── ADR-028-acp-protocol-design.md         # Decision record
└── reference/
    ├── hKask-erd.md                       # Diagram artifact
    ├── registry-erd.md                    # Diagram artifact
    ├── subsystem-erds.md                  # Diagram artifact
    ├── hKask-hLexicon.md                  # Vocabulary catalog
    ├── ports-inventory.md                 # Port reference
    ├── utoipa-implementation.md           # API guide
    ├── template-header-standard.md        # Format reference
    ├── hKask-Curator-persona.md           # Persona spec
    ├── okapi-integration.md               # Okapi API contract
    └── loop-architecture.md              # 6-loop decomposition, crate↔loop map
```

**Total:** 24 active architecture documents (4 specs + 3 framework + 1 index + 7 ADRs + 9 reference artifacts).

---

*ℏKask — A Minimal Viable Container for Agents — v0.21.0*
