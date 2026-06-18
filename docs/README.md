---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged
by [MDS](architecture/core/MDS.md) category. 59 active documents. ℏKask v0.27.0.

> **Lifecycle:** Retired documents are removed via `git rm`. The gitignored
> `docs/archive/` holds date-stamped snapshots for reference.

---

## Start Here

| Document | What It Is |
|----------|------------|
| [`architecture/hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative architecture index — 4 patterns, kata, kanban, LoRA, deployment |
| [`architecture/core/MDS.md`](architecture/core/MDS.md) | 5-category specification framework and MDS methodology |
| [`architecture/core/PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | Architecture principles (P1-P12) |
| [`architecture/core/TESTING_NARRATIVE.md`](architecture/core/TESTING_NARRATIVE.md) | Contract-anchored testing — narrative walkthrough (start here) |
| [`plans/TODO.md`](plans/TODO.md) | Open work |
| [`plans/deployment-and-backup.md`](plans/deployment-and-backup.md) | Deployment & Multi-User Plan |

---

## Architecture (`architecture/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | all | Authoritative index — patterns, kata, kanban, LoRA, daemon, ACP, deployment |
| [`loop-architecture.md`](architecture/loop-architecture.md) | domain, composition, lifecycle | 4-loop authority model |
| [`energy-gas-payments-api-keys.md`](architecture/energy-gas-payments-api-keys.md) | domain, trust, lifecycle | Energy, gas, payments & API key architecture |
| [`matrix-integration-architecture.md`](architecture/matrix-integration-architecture.md) | composition, trust | Matrix transport, Conduit sidecar |
| [`PUBLIC_SURFACE_JUSTIFICATIONS.md`](architecture/PUBLIC_SURFACE_JUSTIFICATIONS.md) | composition, curation | Deep-module audit — 16-crate surface justifications |

### Core (`architecture/core/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`magna-carta.md`](architecture/core/magna-carta.md) | all | User sovereignty charter — 4 inviolable principles |
| [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | all | Architecture principles (P1-P12) |
| [`MDS.md`](architecture/core/MDS.md) | all | Minimal Domain Specification — 5 categories, 5 tools |
| [`TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | all | Contract-anchored testing — DbC + PBT specification |
| [`TESTING_NARRATIVE.md`](architecture/core/TESTING_NARRATIVE.md) | lifecycle, curation | Narrative companion — getting-started walkthrough |
| [`CNS-DOMAIN-SPECIFICATION.md`](architecture/core/CNS-DOMAIN-SPECIFICATION.md) | domain, composition, lifecycle | CNS specification — 6 sub-domains, 44 contracts |
| [`FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | domain, composition | AgentService functional specification |
| [`RSOLIDITY_VOCABULARY.md`](architecture/core/RSOLIDITY_VOCABULARY.md) | domain, trust | rSolidity macro vocabulary |

### Mandates

| Document | MDS | Description |
|----------|-----|-------------|
| [`P12-replicant-host-mandate.md`](architecture/mandates/P12-replicant-host-mandate.md) | domain, trust, composition | Replicant host mandate — P12 elaboration |

### ADRs (Active only)

| ADR | MDS | Decision |
|-----|-----|----------|
| [ADR-031](architecture/ADRs/ADR-031-consolidation-authorization.md) | trust | Consolidation authorization via master passphrase derivation |
| [ADR-035](architecture/ADRs/ADR-035-replicant-server-mode.md) | composition, trust, lifecycle | Replicant server mode — AgentMode, daemon transport, dual memory |

**Archived (2026-06-17):** ADR-030, ADR-032–034, ADR-036–038 (7 Draft ADRs, never adopted). Recoverable via git history.

### Reference

| Document | Description |
|----------|-------------|
| [`hKask-hLexicon.md`](architecture/reference/hKask-hLexicon.md) | 87-term vocabulary catalog |
| [`ports-inventory.md`](architecture/reference/ports-inventory.md) | Hexagonal port inventory |
| [`okapi-integration.md`](architecture/reference/okapi-integration.md) | Inference Router API contract |
| [`utoipa-implementation.md`](architecture/reference/utoipa-implementation.md) | OpenAPI generation guide |
| [`template-header-standard.md`](architecture/reference/template-header-standard.md) | Template metadata format |
| [`hKask-Curator-persona.md`](architecture/reference/hKask-Curator-persona.md) | Curator persona specification |

---

## Specifications (`specifications/`)

### Standards — HOW we work

| Document | MDS | Description |
|----------|-----|-------------|
| [`DOCUMENTATION_STANDARDS.md`](specifications/standards/DOCUMENTATION_STANDARDS.md) | all | Metadata, citation, diagram, and lifecycle mandates |
| [`WRITING_EXCELLENCE.md`](specifications/standards/WRITING_EXCELLENCE.md) | curation | The 4-perspective writing test |
| [`DEPENDENCY_POLICY.md`](specifications/standards/DEPENDENCY_POLICY.md) | lifecycle | Dependency governance |

### Policies — WHAT we must do

| Document | MDS | Description |
|----------|-----|-------------|
| [`DOCUMENT_OWNERSHIP.md`](specifications/policies/DOCUMENT_OWNERSHIP.md) | lifecycle, curation | Document category ownership, version sync policy |
| [`HANDOFF_LIFECYCLE.md`](specifications/policies/HANDOFF_LIFECYCLE.md) | lifecycle, curation | Handoff lifecycle — states, 30-day staleness, archive |

### System Specifications — WHAT the system does

| Document | MDS | Description |
|----------|-----|-------------|
| [`MDS_SCAFFOLD.md`](specifications/specs/MDS_SCAFFOLD.md) | all | Category → directory mapping; lifecycle enforcement |
| [`REQUIREMENTS.md`](specifications/specs/REQUIREMENTS.md) | all | Implemented requirements as goal specs |
| [`TRACEABILITY_MATRIX.md`](specifications/specs/TRACEABILITY_MATRIX.md) | all | Goal spec → code → test traceability |
| [`REPL-specification.md`](specifications/specs/REPL-specification.md) | domain, composition, lifecycle | REPL specification — `kask chat` |
| [`MDS-agent-service.md`](specifications/specs/MDS-agent-service.md) | domain, composition, trust, lifecycle | AgentService specification |
| [`wallet-specification.md`](specifications/specs/wallet-specification.md) | domain, composition, trust, lifecycle | Wallet crate specification |
| [`salience-specification.md`](specifications/specs/salience-specification.md) | domain, composition | Passage salience algorithm |
| [`gentle-lovelace-specification.md`](specifications/specs/gentle-lovelace-specification.md) | domain, composition, curation | Gentle Lovelace replica specification |

**Archived (2026-06-17):** crate-audit.md, HANDOFF_FUNCTIONAL_SPEC.md.

---

## Status

| Document | Description |
|----------|-------------|
| [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | Build, test, and CI health |
| [`corpus_inventory.yaml`](status/corpus_inventory.yaml) | Document corpus lifecycle classification |
| [`public-seam-inventory.md`](status/public-seam-inventory.md) | Public seam inventory — 2,841 items across 46 crates |
| [`public-seam-priority.md`](status/public-seam-priority.md) | Public seam priority ranking |

---

## Guides

| Document | Description |
|----------|-------------|
| [`kata-user-guide.md`](guides/kata-user-guide.md) | Toyota Kata — research, technical build, kanban integration, user how-to |
| [`admin-install-guide.md`](guides/admin-install-guide.md) | Admin install — cloud server setup, OAuth, domain, sidecar deployment |
| [`DEPLOYMENT.md`](guides/DEPLOYMENT.md) | Deployment — production server, systemd, health checks, security hardening |
| [`OPERATIONS_RUNBOOK.md`](guides/OPERATIONS_RUNBOOK.md) | Operations — health checks, troubleshooting, backup/recovery |
| [`lora-training-guide.md`](guides/lora-training-guide.md) | LoRA training — dataset prep to CNS-verified deployment, hardening, troubleshooting |
| [`skill-designer-guide.md`](guides/skill-designer-guide.md) | Skill design — creating, packaging, registering |

---

## User Guides

| Document | Description |
|----------|-------------|
| [`REPLICANT-ONBOARDING-WALKTHROUGH.md`](user-guides/REPLICANT-ONBOARDING-WALKTHROUGH.md) | End-to-end onboarding — install through first chat |
| [`AGENT-POD-CREATION-GUIDE.md`](user-guides/AGENT-POD-CREATION-GUIDE.md) | Creating and managing agent pods |
| [`kanban-user-guide.md`](user-guides/kanban-user-guide.md) | Kanban task coordination — boards, tasks, WIP, kata, error recovery |
| [`skill-user-guide.md`](user-guides/skill-user-guide.md) | Skill usage — installing, activating, composing |
| [`lora-adapter-store-guide.md`](user-guides/lora-adapter-store-guide.md) | LoRA adapter store — lifecycle, routing, deployment |
| [`COMPANIES-GUIDE.md`](user-guides/COMPANIES-GUIDE.md) | Company research and portfolio management |
| [`ACP-ZED-CONFIGURATION.md`](user-guides/ACP-ZED-CONFIGURATION.md) | ACP IDE agent configuration |

---

## Cross-Cutting

| Document | Description |
|----------|-------------|
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects |
| [`generated/cli-reference.md`](generated/cli-reference.md) | Auto-generated CLI reference |
| [`generated/openapi.json`](generated/openapi.json) | OpenAPI 3.1.0 specification (60 endpoints, all documented) |
| [`research/lazy-universe-research.md`](research/lazy-universe-research.md) | Least-action principle — research grounding |

---

## Verification

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/check-metadata.sh   # mandatory metadata headers
bash scripts/ci/contract-audit.sh   # contract coverage (1,804 pub fns, 2,343 REQ tags, 129.8%)
```

*ℏKask — A Minimal Viable Container for Agents — v0.27.0 — 59 active documents*
