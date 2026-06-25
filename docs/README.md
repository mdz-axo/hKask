---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-24
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged by [MDS](architecture/core/MDS.md) category. hKask v0.31.0.

### Two-Tier Document Model

Documents are classified by **verification regime**, reflecting different audiences and drift risks:

| Tier | Audience | Count | Verification | Drift Risk |
|------|----------|-------|-------------|------------|
| **Tier 1 — Spec & Development** | Architects, developers, agents | **26** | `verify-docs.sh` enforces code-anchored claims; `check-links.sh` enforces cross-references | HIGH — stale claims cause agent hallucination |
| **Tier 2 — User & Operator Guides** | Human operators, users, replicants | **14** | Link integrity only; content accuracy verified during onboarding/testing | LOW — guides serve human readers; drift degrades experience but not agent behavior |

**Tier 1 consolidation target:** ≤40 spec/dev documents. ✓ Achieved (26). Merged 2026-06-24.

> **Lifecycle:** Retired documents are moved to `docs/archive/`. Git history preserves all versions.

---

## Tier 1 — Specification & Development Documents

### Start Here

| Document | What It Is |
|----------|------------|
| [`architecture/hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative architecture index — 4 patterns, four-loop decomposition, economic layer, self-healing, pod architecture, Curator persona |
| [`architecture/core/MDS.md`](architecture/core/MDS.md) | 5-category specification framework, MDS methodology, AgentService spec (absorbed) |
| [`architecture/core/PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | Architecture principles (P1-P12), dual-axis ontological framework (PKO + DC+BIBO) |
| [`architecture/core/TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | Contract-anchored testing — DbC + PBT specification + QA triage |
| [`architecture/core/FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | Functional specification — 26 domains, CNS sub-domains (absorbed), contract anchoring |
| [`plans/deployment-and-backup.md`](plans/deployment-and-backup.md) | Deployment & Multi-User Plan — includes admin install, K8s, operations, QA pipeline |
| [`plans/TODO.md`](plans/TODO.md) | Open work |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |

---

## Architecture (`architecture/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | all | Authoritative index — 4 patterns, four-loop, energy, self-healing, pod, Curator persona |
| [`matrix-integration-architecture.md`](architecture/matrix-integration-architecture.md) | domain, composition, lifecycle | Matrix transport, Conduit sidecar, integration architecture |

### Core (`architecture/core/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`magna-carta.md`](architecture/core/magna-carta.md) | all | User sovereignty charter — 4 inviolable principles |
| [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | all | Architecture principles (P1-P12), dual-axis framework, 5W1H core |
| [`MDS.md`](architecture/core/MDS.md) | all | Minimal Domain Specification — 5 categories, 5 tools, AgentService spec |
| [`TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | all | Contract-anchored testing — DbC, PBT, fuzz, mutation, LLM triage |
| [`FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | domain, composition | AgentService functional spec + CNS domain specification |

### ADRs (Active only)

| ADR | MDS | Decision |
|-----|-----|----------|
| [ADR-031](architecture/ADRs/ADR-031-consolidation-authorization.md) | trust | Consolidation authorization via master passphrase derivation |
| [ADR-035](architecture/ADRs/ADR-035-replicant-server-mode.md) | composition, trust, lifecycle | Replicant server mode — AgentMode, daemon transport, dual memory |

**Archived (2026-06-17):** ADR-030, ADR-032–034, ADR-036–038 (7 Draft ADRs, never adopted).

**Archived (2026-06-24):** 33 documents merged or archived in consolidation sweep. loop-architecture, energy-gas-payments, self-healing, provider-intelligence, rjoule-cost-system, hkask-ledger, MULTI_POD_ARCHITECTURE, SOLID_POD_ISOMORPHISM, CNS-DOMAIN-SPECIFICATION, hKask-Curator-persona, MDS-agent-service, TUI_SPECIFICATION → content absorbed into targets. Federation v1 + all addenda, bug-hunting research/designs, stale status docs → archive.

---

## Specifications (`specifications/`)

### Standards — HOW we work

| Document | MDS | Description |
|----------|-----|-------------|
| [`DOCUMENTATION_STANDARDS.md`](specifications/standards/DOCUMENTATION_STANDARDS.md) | all | Metadata, citation, diagram, lifecycle mandates. Includes Writing Excellence (Appendix A), Handoff Lifecycle (Appendix B), Dependency Policy (Appendix C). |

### System Specifications — WHAT the system does

| Document | MDS | Description |
|----------|-----|-------------|
| [`REQUIREMENTS.md`](specifications/specs/REQUIREMENTS.md) | all | Implemented requirements as goal specs |
| [`REPL-specification.md`](specifications/specs/REPL-specification.md) | domain, composition, lifecycle | REPL specification — `kask chat` |
| [`wallet-specification.md`](specifications/specs/wallet-specification.md) | domain, composition, trust, lifecycle | Wallet crate specification |
| [`salience-specification.md`](specifications/specs/salience-specification.md) | domain, composition | Passage salience algorithm |
| [`gentle-lovelace-specification.md`](specifications/specs/gentle-lovelace-specification.md) | domain, composition, curation | Gentle Lovelace replica specification |

### Plans

| Document | Description |
|----------|-------------|
| [`deployment-and-backup.md`](plans/deployment-and-backup.md) | Deployment & Multi-User Plan — includes admin install, K8s, operations, Cloud Gateway, QA pipeline |
| [`TODO.md`](plans/TODO.md) | Open work |

### Research

| Document | Description |
|----------|-------------|
| [`lazy-universe-research.md`](research/lazy-universe-research.md) | Least-action principle — research grounding |

### Status

| Document | Description |
|----------|-------------|
| [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | Build, test, and CI health |

### Cross-Cutting

| Document | Description |
|----------|-------------|
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry |
| [`generated/cli-reference.md`](generated/cli-reference.md) | Auto-generated CLI reference |
| [`generated/openapi.json`](generated/openapi.json) | OpenAPI 3.1.0 specification |

---

## Tier 2 — User & Operator Guides

### Operator Guides

| Document | Description |
|----------|-------------|
| [`QA_GUIDE.md`](guides/QA_GUIDE.md) | QA system operations — fuzz triage, mutation analysis, autonomous scripts |
| [`lora-training-guide.md`](guides/lora-training-guide.md) | LoRA training — dataset prep to CNS-verified deployment |
| [`kata-user-guide.md`](guides/kata-user-guide.md) | Toyota Kata — research, technical build, kanban integration |
| [`bug-hunter-guide.md`](guides/bug-hunter-guide.md) | Bug hunting methodology and expedition execution |
| [`skill-designer-guide.md`](guides/skill-designer-guide.md) | Skill design — creating, packaging, registering |

### User Guides

| Document | Description |
|----------|-------------|
| [`REPLICANT-ONBOARDING-WALKTHROUGH.md`](user-guides/REPLICANT-ONBOARDING-WALKTHROUGH.md) | End-to-end onboarding — install through first chat |
| [`AGENT-POD-CREATION-GUIDE.md`](user-guides/AGENT-POD-CREATION-GUIDE.md) | Creating and managing agent pods |
| [`kanban-user-guide.md`](user-guides/kanban-user-guide.md) | Kanban task coordination — boards, tasks, WIP |
| [`skill-user-guide.md`](user-guides/skill-user-guide.md) | Skill usage — installing, activating, composing |
| [`skill-composition-guide.md`](user-guides/skill-composition-guide.md) | Skill composition — bundling and cascade ordering |
| [`lora-adapter-store-guide.md`](user-guides/lora-adapter-store-guide.md) | LoRA adapter store — lifecycle, routing, deployment |
| [`COMPANIES-GUIDE.md`](user-guides/COMPANIES-GUIDE.md) | Company research and portfolio management |
| [`dokkodo-user-guide.md`](user-guides/dokkodo-user-guide.md) | Dokkodo mindset — perceptual filter application |
| [`ACP-ZED-CONFIGURATION.md`](user-guides/ACP-ZED-CONFIGURATION.md) | ACP IDE agent configuration |

---

## Verification

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/verify-docs.sh      # Tier 1 code-anchored claim verification
```

*ℏKask — A Minimal Viable Container for Agents — v0.31.0 — 26 Tier 1 + 14 Tier 2 = 40 active documents*
