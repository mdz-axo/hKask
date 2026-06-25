---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-22
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged by [MDS](architecture/core/MDS.md) category. hKask v0.30.0.

### Two-Tier Document Model

Documents are classified by **verification regime**, reflecting different audiences and drift risks:

| Tier | Audience | Count | Verification | Drift Risk |
|------|----------|-------|-------------|------------|
| **Tier 1 — Spec & Development** | Architects, developers, agents | **73** | `verify-docs.sh` enforces code-anchored claims; `check-links.sh` enforces cross-references | HIGH — stale claims cause agent hallucination |
| **Tier 2 — User & Operator Guides** | Human operators, users, replicants | **18** | Link integrity only; content accuracy verified during onboarding/testing | LOW — guides serve human readers; drift degrades experience but not agent behavior |

**Tier 1 consolidation target:** ≤40 spec/dev documents. Tier 2 guides are maintained separately; their count is driven by user needs, not architectural minimalism.

> **Lifecycle:** Retired documents are removed via `git rm`. The gitignored
> `docs/archive/` holds date-stamped snapshots for reference.

---

## Tier 1 — Specification & Development Documents

These documents are anchored against code. Every factual claim (crate names, counts, versions) must be verifiable. Stale Tier 1 documents produce incorrect agent behavior.

### Start Here

| Document | What It Is |
|----------|------------|
| [`architecture/hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative architecture index — 4 patterns, three-tier pods, kata, kanban, LoRA, deployment |
| [`architecture/core/SOLID_POD_ISOMORPHISM.md`](architecture/core/SOLID_POD_ISOMORPHISM.md) | AgentPod↔Solid Pod isomorphism — drift analysis and resolution |
| [`architecture/core/MULTI_POD_ARCHITECTURE.md`](architecture/core/MULTI_POD_ARCHITECTURE.md) | Three-tier pod architecture (CuratorPod, TeamPod, ReplicantPod) |
| [`architecture/core/MDS.md`](architecture/core/MDS.md) | 5-category specification framework and MDS methodology |
| [`architecture/core/PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | Architecture principles (P1-P12, includes P12 replicant host mandate) |
| [`architecture/core/TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | Contract-anchored testing — DbC + PBT specification + QA triage |
| [`plans/TODO.md`](plans/TODO.md) | Open work |
| [`plans/deployment-and-backup.md`](plans/deployment-and-backup.md) | Deployment & Multi-User Plan |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |

---

## Architecture (`architecture/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | all | Authoritative index — patterns, kata, kanban, LoRA, daemon, ACP, deployment. Includes deep-module public surface audit. |
| [`loop-architecture.md`](architecture/loop-architecture.md) | domain, composition | Four-loop authority model — semantic root-cause analysis |
| [`energy-gas-payments-api-keys.md`](architecture/energy-gas-payments-api-keys.md) | domain, lifecycle | Energy budget, gas, payments, and API key architecture |
| [`self-healing.md`](architecture/self-healing.md) | domain, lifecycle | Self-healing architecture patterns |
| [`matrix-integration-architecture.md`](architecture/matrix-integration-architecture.md) | domain, composition, lifecycle | Matrix transport, Conduit sidecar, integration architecture |

### Specs (`architecture/specs/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`hkask-ledger.md`](architecture/specs/hkask-ledger.md) | domain, trust | Triple-entry accounting ledger specification |
| [`provider-intelligence.md`](architecture/specs/provider-intelligence.md) | domain, composition | Provider intelligence architecture |
| [`rjoule-cost-system.md`](architecture/specs/rjoule-cost-system.md) | domain, lifecycle | rJoule cost tracking system |

### Core (`architecture/core/`)

| Document | MDS | Description |
|----------|-----|-------------|
| [`magna-carta.md`](architecture/core/magna-carta.md) | all | User sovereignty charter — 4 inviolable principles |
| [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | all | Architecture principles (P1-P12) |
| [`MDS.md`](architecture/core/MDS.md) | all | Minimal Domain Specification — 5 categories, 5 tools |
| [`TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | all | Contract-anchored testing — DbC, PBT, fuzz, mutation, LLM triage |
| [`CNS-DOMAIN-SPECIFICATION.md`](architecture/core/CNS-DOMAIN-SPECIFICATION.md) | domain, composition, lifecycle | CNS specification — 6 sub-domains, 44 contracts |
| [`SOLID_POD_ISOMORPHISM.md`](architecture/core/SOLID_POD_ISOMORPHISM.md) | domain, composition | AgentPod↔Solid Pod isomorphism — drift analysis, deployment types, semantic map |
| [`MULTI_POD_ARCHITECTURE.md`](architecture/core/MULTI_POD_ARCHITECTURE.md) | domain, composition, lifecycle | Multi-pod tiers — CuratorPod + TeamPod + ReplicantPod |
| [`FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | domain, composition | AgentService functional specification |

### ADRs (Active only)

| ADR | MDS | Decision |
|-----|-----|----------|
| [ADR-031](architecture/ADRs/ADR-031-consolidation-authorization.md) | trust | Consolidation authorization via master passphrase derivation |
| [ADR-035](architecture/ADRs/ADR-035-replicant-server-mode.md) | composition, trust, lifecycle | Replicant server mode — AgentMode, daemon transport, dual memory |

**Archived (2026-06-17):** ADR-030, ADR-032–034, ADR-036–038 (7 Draft ADRs, never adopted).

**Archived (2026-06-22):** `qa/QA_PLAN.md` (merged into TESTING_DISCIPLINE.md), `mandates/P12-replicant-host-mandate.md` (merged into PRINCIPLES.md), `core/OPEN_QUESTIONS_POD.md` (merged into OPEN_QUESTIONS.md), `handoffs/` (2 historical handoffs).

### Reference

| Document | Description |
|----------|-------------|
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
| [`HANDOFF_LIFECYCLE.md`](specifications/policies/HANDOFF_LIFECYCLE.md) | lifecycle, curation | Handoff lifecycle — states, 30-day staleness, archive |

### System Specifications — WHAT the system does

| Document | MDS | Description |
|----------|-----|-------------|
| [`MDS.md`](architecture/core/MDS.md) | all | MDS specification framework + documentation structure (§9) |
| [`REQUIREMENTS.md`](specifications/specs/REQUIREMENTS.md) | all | Implemented requirements as goal specs |
| [`REPL-specification.md`](specifications/specs/REPL-specification.md) | domain, composition, lifecycle | REPL specification — `kask chat` |
| [`MDS-agent-service.md`](specifications/specs/MDS-agent-service.md) | domain, composition, trust, lifecycle | AgentService specification |
| [`wallet-specification.md`](specifications/specs/wallet-specification.md) | domain, composition, trust, lifecycle | Wallet crate specification |
| [`salience-specification.md`](specifications/specs/salience-specification.md) | domain, composition | Passage salience algorithm |
| [`gentle-lovelace-specification.md`](specifications/specs/gentle-lovelace-specification.md) | domain, composition, curation | Gentle Lovelace replica specification |

**Archived (2026-06-17):** crate-audit.md, HANDOFF_FUNCTIONAL_SPEC.md.

---

### Plans

| Document | Description |
|----------|-------------|
| [`deployment-and-backup.md`](plans/deployment-and-backup.md) | Deployment & Multi-User Plan (includes past research §14) |
| [`TODO.md`](plans/TODO.md) | Open work |

### Research

| Document | Description |
|----------|-------------|
| [`lazy-universe-research.md`](research/lazy-universe-research.md) | Least-action principle — research grounding |
| [`bug-hunting-skill-synthesis.md`](research/bug-hunting-skill-synthesis.md) | Bug hunting — design, theory, synthesis |
| [`bug-hunting-as-autopoietic-skill-unified.md`](research/bug-hunting-as-autopoietic-skill-unified.md) | Autopoietic bug hunting — unified theory |
| [`bug-hunting-skill-corrected-design.md`](research/bug-hunting-skill-corrected-design.md) | Bug hunting — corrected design |
| [`bug-hunting-skill-implementation-plan.md`](research/bug-hunting-skill-implementation-plan.md) | Bug hunting — implementation plan |

### Status

| Document | Description |
|----------|-------------|
| [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | Build, test, and CI health |
| [`public-seam-priority.md`](status/public-seam-priority.md) | Public seam priority ranking |

### Cross-Cutting

| Document | Description |
|----------|-------------|
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry (30 diagrams) |
| [`generated/cli-reference.md`](generated/cli-reference.md) | Auto-generated CLI reference |
| [`generated/openapi.json`](generated/openapi.json) | OpenAPI 3.1.0 specification |

---

## Tier 2 — User & Operator Guides

These documents serve human operators and users. They are verified for link integrity but exempt from the code-anchored claim verification that governs Tier 1. Drift here degrades user experience but does not cause agent hallucination.

### Operator Guides

| Document | Description |
|----------|-------------|
| [`admin-setup-guide.md`](guides/admin-setup-guide.md) | System administrator setup |
| [`DEPLOYMENT.md`](guides/DEPLOYMENT.md) | Deployment — production server, systemd, health checks, security hardening |
| [`kubernetes-primer.md`](guides/kubernetes-primer.md) | Kubernetes primer for hKask deployment |
| [`OPERATIONS_RUNBOOK.md`](guides/OPERATIONS_RUNBOOK.md) | Operations — health checks, troubleshooting, backup/recovery |
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

*ℏKask — A Minimal Viable Container for Agents — v0.30.0 — 55 Tier 1 + 18 Tier 2 = 73 active documents*
