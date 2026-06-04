---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-03
version: "1.2.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged
by [DDMVSS](architecture/DDMVSS.md) category. This portal is the navigation
surface; canonical content lives in the linked documents, never duplicated here.

ℏKask - "A Minimal Viable Container for Agents" - binary `kask`,
crate prefix `hkask-`, workspace v0.22.0.

> **Lifecycle note.** Retired documents are removed from the active tree via
> `git rm` (git history is the archive of record) and the on-disk `docs/archive/`
> snapshot is gitignored. See [`specifications/DOCUMENTATION_STANDARDS.md`](specifications/DOCUMENTATION_STANDARDS.md) §3.

---

## Start Here

| Document | What It Is |
|----------|------------|
| [`architecture/hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative index to the four DDMVSS specification documents and reference artifacts |
| [`architecture/DDMVSS.md`](architecture/DDMVSS.md) | The 9-category goal-group taxonomy and MVSDD methodology |
| [`specifications/DDMVSS_SCAFFOLD.md`](specifications/DDMVSS_SCAFFOLD.md) | DDMVSS category → directory mapping and lifecycle policy |
| [`status/PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | Single source of truth for build/test/metrics status |
| [`plans/TODO.md`](plans/TODO.md) | Open work only |

---

## Architecture (`architecture/`)

The architecture is specified in four DDMVSS-aligned documents, each authoritative
for its category cluster.

| Document | DDMVSS Categories | Description |
|----------|-------------------|-------------|
| [`domain-and-capability.md`](architecture/domain-and-capability.md) | domain, capability | Bounded context, agent taxonomy, capability/OCAP model, MCP tool surface, hLexicon |
| [`interface-and-composition.md`](architecture/interface-and-composition.md) | interface, composition | MCP/CLI/API equivalence, hexagonal ports, unified registry, template cascade |
| [`trust-security-observability.md`](architecture/trust-security-observability.md) | trust, observability | OCAP enforcement, key derivation, encryption stack, CNS spans, algedonic alerts |
| [`persistence-and-lifecycle.md`](architecture/persistence-and-lifecycle.md) | persistence, lifecycle | SQLite + SQLCipher, bitemporal triples, embeddings, bootstrap, evolution rules |
| [`PRINCIPLES.md`](architecture/PRINCIPLES.md) | domain, capability, curation | Architecture principles (P1-P7, C1-C7), five anchors, anti-patterns |
| [`magna-carta.md`](architecture/magna-carta.md) | trust | User sovereignty charter — catch-and-release, kill-zone detection |

### Architecture Decision Records

| ADR | DDMVSS | Decision |
|-----|--------|----------|
| [ADR-022](architecture/ADR-022-comprehensive-security-hardening.md) | trust | Comprehensive security hardening |
| [ADR-023](architecture/ADR-023-master-key-derivation.md) | trust | Master key derivation via HKDF-SHA256 |
| [ADR-024](architecture/ADR-024-unified-registry.md) | composition | Unified registry decision |
| [ADR-025](architecture/ADR-025-attenuation-depth-limit.md) | trust | 7-level attenuation depth limit |
| [ADR-026](architecture/ADR-026-bitemporal-triple-schema.md) | persistence | Bitemporal triple schema |
| [ADR-027](architecture/ADR-027-argon2-hkdf-master-key.md) | trust | Argon2id + HKDF-SHA256 master key derivation |
| [ADR-028](architecture/ADR-028-acp-protocol-design.md) | interface, composition | ACP protocol design (JSON-RPC 2.0 over stdio) |
| [ADR-029](architecture/ADR-029-goal-capability-primitive.md) | capability, trust | Goal capability primitive — distinct typed token (not collapsed into generic capability) |

### Reference Artifacts (`architecture/reference/`)

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`hKask-erd.md`](architecture/reference/hKask-erd.md) | persistence | Core entity relationship diagrams |
| [`subsystem-erds.md`](architecture/reference/subsystem-erds.md) | persistence | Per-crate ERDs grounded in Rust source |
| [`registry-erd.md`](architecture/reference/registry-erd.md) | persistence | Template registry ERD |
| [`hKask-hLexicon.md`](architecture/reference/hKask-hLexicon.md) | domain | Minimal composition vocabulary |
| [`ports-inventory.md`](architecture/reference/ports-inventory.md) | interface | Hexagonal port inventory |
| [`okapi-integration.md`](architecture/reference/okapi-integration.md) | domain | Okapi LLM API contract |
| [`utoipa-implementation.md`](architecture/reference/utoipa-implementation.md) | interface | API and CLI documentation generation |
| [`template-header-standard.md`](architecture/reference/template-header-standard.md) | composition | hLexicon functional role headers |
| [`hKask-Curator-persona.md`](architecture/reference/hKask-Curator-persona.md) | domain | Canonical human-facing replicant |

---

## Specifications (`specifications/`)

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md) | all | Implemented requirements as goal specs |
| [`TRACEABILITY_MATRIX.md`](specifications/TRACEABILITY_MATRIX.md) | all | Goal spec → code → test traceability |
| [`DOCUMENTATION_STANDARDS.md`](specifications/DOCUMENTATION_STANDARDS.md) | all | Metadata, citation, diagram, and lifecycle mandates |
| [`DDMVSS_SCAFFOLD.md`](specifications/DDMVSS_SCAFFOLD.md) | all | Category → directory mapping; lifecycle enforcement |
| [`WRITING_EXCELLENCE.md`](specifications/WRITING_EXCELLENCE.md) | curation | The 4-perspective writing test |
| [`ADR_TEMPLATE.md`](specifications/ADR_TEMPLATE.md) | curation | Starting point for new ADRs |
| [`DEPENDENCY_POLICY.md`](specifications/DEPENDENCY_POLICY.md) | lifecycle | Dependency governance |
| [`DEPLOYMENT.md`](specifications/DEPLOYMENT.md) | lifecycle | Deployment guide |
| [`CI-CD-GUIDE.md`](specifications/CI-CD-GUIDE.md) | lifecycle | CI/CD and installation |

---

## Status (`status/`)

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | all | **Single source of truth** for metrics, build, test, DDMVSS completeness |
| [`mcp-server-audit.md`](status/mcp-server-audit.md) | capability, observability | MCP server completeness audit |

---

## Plans (`plans/`)

Open work and design drafts. Drafts (`Status: Draft`) are exploratory and not authoritative.

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`TODO.md`](plans/TODO.md) | all | Open work items only |
| [`high-temp-templates.md`](plans/high-temp-templates.md) | composition, curation | High-temperature template design (draft) |

---

## User Guides (`user-guides/`)

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`AGENT-POD-CREATION-GUIDE.md`](user-guides/AGENT-POD-CREATION-GUIDE.md) | domain | Creating agent pods |
| [`COMMON-AGENT-PATTERNS.md`](user-guides/COMMON-AGENT-PATTERNS.md) | domain | Common agent patterns and templates |

---

## Cross-Cutting Indexes

| Document | DDMVSS | Description |
|----------|--------|-------------|
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | all | Mermaid diagram verification registry |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | interface, composition, capability, observability, curation, lifecycle | Underspecified aspects awaiting decisions |
| [`gml/gml-allosteric-thinking-v2.md`](gml/gml-allosteric-thinking-v2.md) | domain | GML Allosteric Thinking tool |
| [`gml/gml-user-guide.md`](gml/gml-user-guide.md) | domain | GML user guide |
| [`generated/cli-reference.md`](generated/cli-reference.md) | interface | Auto-generated CLI reference |

---

## Verification

Documentation quality gates (run from the repository root):

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/check-metadata.sh   # mandatory metadata headers on every active doc
```

*ℏKask - A Minimal Viable Container for Agents — v0.22.0*
