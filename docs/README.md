---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-10
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged
by [MDS](architecture/MDS.md) category. This portal is the navigation
surface; canonical content lives in the linked documents, never duplicated here.

ℏKask - "A Minimal Viable Container for Agents" - binary `kask`,
crate prefix `hkask-`, workspace v0.27.0.

> **Lifecycle note.** Retired documents are removed from the active tree via
> `git rm` (git history is the archive of record) and the on-disk `docs/archive/`
> snapshot is gitignored. See [`specifications/DOCUMENTATION_STANDARDS.md`](specifications/DOCUMENTATION_STANDARDS.md) §3.

---

## Start Here

| Document | What It Is |
|----------|------------|
| [`architecture/hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative index to the four MDS specification documents and reference artifacts |
| [`architecture/MDS.md`](architecture/MDS.md) | The 5-category specification framework and MDS methodology |
| [`specifications/MDS_SCAFFOLD.md`](specifications/MDS_SCAFFOLD.md) | MDS category → directory mapping and lifecycle policy |
| [`plans/TODO.md`](plans/TODO.md) | Open work only |

---

## Architecture (`architecture/`)

The architecture is specified in four MDS-aligned documents, each authoritative
for its category cluster.

| Document | MDS Categories | Description |
|----------|-------------------|-------------|
| [`MDS.md`](architecture/MDS.md) | domain, composition, trust, lifecycle, curation | Minimal Domain Specification — 5 categories, 6 tools, completeness predicate |
| [`PRINCIPLES.md`](architecture/PRINCIPLES.md) | domain, composition, trust, lifecycle, curation | Architecture principles (P1-P9), 5 anchors, anti-patterns |
| [`magna-carta.md`](architecture/magna-carta.md) | domain, composition, trust, lifecycle, curation | User sovereignty charter — 4 inviolable principles |
| [`loop-architecture.md`](architecture/loop-architecture.md) | domain, composition, lifecycle, curation | 4-loop architecture — RateLimiting→EnergyBudget, crate↔loop mapping |

### Architecture Decision Records

| ADR | MDS | Decision | Status |
|-----|--------|----------|--------|
| [ADR-024](architecture/ADR-024-unified-registry.md) | composition | Unified registry decision | Active |
| [ADR-025](architecture/ADR-025-attenuation-depth-limit.md) | trust | 7-level attenuation depth limit | Active |
| [ADR-026](architecture/ADR-026-bitemporal-triple-schema.md) | persistence | Bitemporal triple schema | Active |
| [ADR-027](architecture/ADR-027-argon2-hkdf-master-key.md) | trust | Argon2id + HKDF-SHA256 master key derivation | Active |
| [ADR-030](architecture/ADR-030-skill-bundler.md) | curation | Skill Bundler — meta-skill composition | Proposed |
| [ADR-031](architecture/ADR-031-consolidation-authorization.md) | trust | Consolidation authorization via master passphrase derivation | Active |
| [ADR-032](architecture/ADR-032-mcp-gateway-membrane.md) | composition, trust | MCP gateway membrane policy — Tier 1 (governed) vs Tier 2 (passthrough) | Draft |
| [ADR-033](architecture/ADR-033-dampener-override-cooldown.md) | trust, observability | Dampener override cooldown — per-issuer vs global | Draft |

### Reference Artifacts (`architecture/reference/`)

| Document | MDS | Description |
|----------|--------|-------------|

| [`hKask-hLexicon.md`](architecture/reference/hKask-hLexicon.md) | domain | Minimal composition vocabulary |
| [`ports-inventory.md`](architecture/reference/ports-inventory.md) | interface | Hexagonal port inventory |
| [`okapi-integration.md`](architecture/reference/okapi-integration.md) | domain | Okapi LLM API contract |
| [`utoipa-implementation.md`](architecture/reference/utoipa-implementation.md) | interface | API and CLI documentation generation |
| [`template-header-standard.md`](architecture/reference/template-header-standard.md) | composition | hLexicon functional role headers |
| [`hKask-Curator-persona.md`](architecture/reference/hKask-Curator-persona.md) | domain | Canonical human-facing replicant |


---

## Specifications (`specifications/`)

| Document | MDS | Description |
|----------|--------|-------------|
| [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md) | all | Implemented requirements as goal specs |
| [`TRACEABILITY_MATRIX.md`](specifications/TRACEABILITY_MATRIX.md) | all | Goal spec → code → test traceability |
| [`DOCUMENTATION_STANDARDS.md`](specifications/DOCUMENTATION_STANDARDS.md) | all | Metadata, citation, diagram, and lifecycle mandates |
| [`MDS_SCAFFOLD.md`](specifications/MDS_SCAFFOLD.md) | all | Category → directory mapping; lifecycle enforcement |
| [`WRITING_EXCELLENCE.md`](specifications/WRITING_EXCELLENCE.md) | curation | The 4-perspective writing test |
| [`ADR_TEMPLATE.md`](specifications/ADR_TEMPLATE.md) | curation | Starting point for new ADRs |
| [`DEPENDENCY_POLICY.md`](specifications/DEPENDENCY_POLICY.md) | lifecycle | Dependency governance |
| [`DEPLOYMENT.md`](specifications/DEPLOYMENT.md) | lifecycle | Deployment guide |
| [`CI-CD-GUIDE.md`](specifications/CI-CD-GUIDE.md) | lifecycle | CI/CD and installation |
| [`TESTING_STANDARDS.md`](specifications/TESTING_STANDARDS.md) | all | Testing protocol and classification |
| [`test-program.md`](specifications/test-program.md) | all | Test program specification |

---

## Status (`status/`)

| Document | MDS | Description |
|----------|--------|-------------|
| *(planned)* | all | *Status files are planned but not yet populated. Work items tracked in [`plans/TODO.md`](plans/TODO.md).* |

---

## Plans (`plans/`)

Open work and design drafts. Drafts (`Status: Draft`) are exploratory and not authoritative.

| Document | MDS | Description |
|----------|--------|-------------|
| [`TODO.md`](plans/TODO.md) | all | Open work items only |


---

## User Guides (`user-guides/`)

| Document | MDS | Description |
|----------|--------|-------------|
| [`AGENT-POD-CREATION-GUIDE.md`](user-guides/AGENT-POD-CREATION-GUIDE.md) | domain | Creating agent pods |
| [`COMMON-AGENT-PATTERNS.md`](user-guides/COMMON-AGENT-PATTERNS.md) | domain | Common agent patterns and templates |

---

## Cross-Cutting Indexes

| Document | MDS | Description |
|----------|--------|-------------|
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | all | Mermaid diagram verification registry |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | interface, composition, lifecycle, curation | Underspecified aspects (4 of 7 resolved) |

| [`generated/cli-reference.md`](generated/cli-reference.md) | interface | Auto-generated CLI reference |

---

## Research (`specifications/`)

| Document | MDS | Description |
|----------|--------|-------------|
---

## Verification

Documentation quality gates (run from the repository root):

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/check-metadata.sh   # mandatory metadata headers on every active doc
```

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
