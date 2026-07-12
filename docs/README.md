---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged by [MDS](architecture/core/MDS.md) category. hKask v0.31.0.

### Diataxis Structure

Documentation is organized by [Diataxis](https://diataxis.fr/) quadrants — tutorials, how-to guides, reference, and explanation — supplemented by architecture, diagrams, specifications, and legacy user guides.

> **Lifecycle:** Retired documents are moved to `docs/archive/`. Git history preserves all versions.

---

## Tutorial

| Document | Description |
|----------|-------------|
| [`tutorial/getting-started.md`](tutorial/getting-started.md) | End-to-end walkthrough — install, configure, first chat |

---

## How-To Guides (`how-to/`)

Task-oriented guides (20):

| Guide | What You'll Do |
|-------|----------------|
| [`install-and-run.md`](how-to/install-and-run.md) | Install hKask and run your first session |
| [`configure-database-backend.md`](how-to/configure-database-backend.md) | Set up SQLite, SQLCipher, or PostgreSQL |
| [`configure-feature-gates.md`](how-to/configure-feature-gates.md) | Enable/disable compilation features |
| [`configure-guard.md`](how-to/configure-guard.md) | Configure the OCAP guard membrane |
| [`bootstrap-mcp-server.md`](how-to/bootstrap-mcp-server.md) | Create a new MCP server |
| [`create-agent-pod.md`](how-to/create-agent-pod.md) | Create and manage agent pods |
| [`deploy-k8s.md`](how-to/deploy-k8s.md) | Deploy on Kubernetes |
| [`train-qwen36-unsloth-runpod.md`](how-to/train-qwen36-unsloth-runpod.md) | Train Qwen3.6-27B on RunPod with Unsloth — single-command deployment |
| [`backup-and-restore.md`](how-to/backup-and-restore.md) | Backup and restore hKask data |
| [`setup-matrix-transport.md`](how-to/setup-matrix-transport.md) | Configure Matrix transport |
| [`use-repl.md`](how-to/use-repl.md) | Use the `kask chat` REPL |
| [`use-tui.md`](how-to/use-tui.md) | Use the TUI interface |
| [`use-kanban.md`](how-to/use-kanban.md) | Coordinate tasks with Kanban boards |
| [`invoke-a-skill.md`](how-to/invoke-a-skill.md) | Install, activate, and invoke skills |
| [`compose-skills.md`](how-to/compose-skills.md) | Compose skills into bundles |
| [`design-a-skill.md`](how-to/design-a-skill.md) | Design, package, and register a new skill |
| [`run-kata-cycle.md`](how-to/run-kata-cycle.md) | Run a Toyota Kata improvement cycle |
| [`train-lora-adapter.md`](how-to/train-lora-adapter.md) | Current LoRA training availability and blocked MCP workflow |
| [`read-cns-alerts.md`](how-to/read-cns-alerts.md) | Read and interpret CNS alerts |
| [`audit-sovereignty.md`](how-to/audit-sovereignty.md) | Audit sovereignty compliance |

---

## Reference (`reference/`)

### API Reference (23 crates)

| Crate | Crate | Crate |
|-------|-------|-------|
| [`hkask-acp`](reference/api/hkask-acp.md) | [`hkask-agents`](reference/api/hkask-agents.md) | [`hkask-api`](reference/api/hkask-api.md) |
| [`hkask-capability`](reference/api/hkask-capability.md) | [`hkask-cli`](reference/api/hkask-cli.md) | [`hkask-cns`](reference/api/hkask-cns.md) |
| [`hkask-codegraph`](reference/api/hkask-codegraph.md) | [`hkask-communication`](reference/api/hkask-communication.md) | [`hkask-condenser`](reference/api/hkask-condenser.md) |
| [`hkask-database`](reference/api/hkask-database.md) | [`hkask-federation`](reference/api/hkask-federation.md) | [`hkask-guard`](reference/api/hkask-guard.md) |
| [`hkask-improv`](reference/api/hkask-improv.md) | [`hkask-inference`](reference/api/hkask-inference.md) | [`hkask-keystore`](reference/api/hkask-keystore.md) |
| [`hkask-ledger`](reference/api/hkask-ledger.md) | [`hkask-mcp`](reference/api/hkask-mcp.md) | [`hkask-memory`](reference/api/hkask-memory.md) |
| [`hkask-ports`](reference/api/hkask-ports.md) | [`hkask-storage`](reference/api/hkask-storage.md) | [`hkask-templates`](reference/api/hkask-templates.md) |
| [`hkask-types`](reference/api/hkask-types.md) | [`hkask-wallet`](reference/api/hkask-wallet.md) | |

### Other Reference

| Document | Description |
|----------|-------------|
| [`reference/skills/`](reference/skills/) | Skills registry — manifests and metadata |
| [`reference/cns-spans.md`](reference/cns-spans.md) | CNS span catalog and namespaces |
| [`reference/magna-carta.md`](reference/magna-carta.md) | Magna Carta — 4 inviolable sovereignty principles |
| [`qwen36-training-hyperparameters.md`](reference/qwen36-training-hyperparameters.md) | Qwen3.6-27B training hyperparameters — provenance and rationale |
| [`reference/mcp-servers/`](reference/mcp-servers/) | MCP server reference (15 built-in servers) |

---

## Explanation (`explanation/`)

| Document | Topic |
|----------|-------|
| [`dual-axis-ontology.md`](explanation/dual-axis-ontology.md) | PKO + DC/BIBO dual-axis ontological framework |
| [`hexagonal-ports.md`](explanation/hexagonal-ports.md) | Hexagonal architecture — port/adaptor contracts |
| [`loom-and-thread.md`](explanation/loom-and-thread.md) | Loom and thread concurrency model |
| [`cns-homeostatic-loop.md`](explanation/cns-homeostatic-loop.md) | CNS homeostatic regulation loop |
| [`curator-metacognition.md`](explanation/curator-metacognition.md) | Curator metacognition and self-reflection |
| [`database-driver-abstraction.md`](explanation/database-driver-abstraction.md) | Database driver abstraction layer |
| [`energy-gas-system.md`](explanation/energy-gas-system.md) | Energy and gas payment system |
| [`federation-model.md`](explanation/federation-model.md) | Federation dispatch model |
| [`good-regulator.md`](explanation/good-regulator.md) | Conant-Ashby Good Regulator theorem application |
| [`nu-event-semantics.md`](explanation/nu-event-semantics.md) | ν-event semantic model and observability |
| [`ocap-mcp-dispatch.md`](explanation/ocap-mcp-dispatch.md) | OCAP-attenuated MCP tool dispatch |
| [`skill-pdca-model.md`](explanation/skill-pdca-model.md) | Skill PDCA loop model |
| [`vsm-mapping.md`](explanation/vsm-mapping.md) | Viable System Model mapping to hKask |

---

## Architecture (`architecture/`)

| Document | Description |
|----------|-------------|
| [`hKask-architecture-master.md`](architecture/hKask-architecture-master.md) | Authoritative architecture index — 4 patterns, four-loop decomposition, economic layer, self-healing, pod architecture, Curator persona |
| [`matrix-integration-architecture.md`](architecture/matrix-integration-architecture.md) | Matrix transport, Conduit sidecar, integration architecture |
| [`well-wallet-architecture.md`](architecture/well-wallet-architecture.md) | Wallet architecture |
| [`database-providers.md`](architecture/database-providers.md) | Database provider architecture |
| [`ADR-043-database-driver.md`](architecture/ADR-043-database-driver.md) | ADR-043 — database driver abstraction |

### Core (`architecture/core/`)

| Document | Description |
|----------|-------------|
| [`magna-carta.md`](architecture/core/magna-carta.md) | User sovereignty charter — 4 inviolable principles |
| [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | Architecture principles (P1-P12), dual-axis framework |
| [`MDS.md`](architecture/core/MDS.md) | Minimal Domain Specification — 5 categories |
| [`TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | Contract-anchored testing — DbC, PBT, fuzz, mutation |
| [`FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | Functional specification — 26 domains |

### ADRs (`architecture/ADRs/`)

Active ADRs: ADR-031 (consolidation authorization), ADR-035 (replicant server mode). Archived (2026-06-17): ADR-030, ADR-032–034, ADR-036–037. See directory for full index.

---

## Diagrams (`diagrams/`)

45 standalone Mermaid diagrams across 5 types — flowchart, sequence, state, class, ERD — plus 14 inline functional-specification diagrams. See [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) for the curated verification registry.

---

## Specifications (`specifications/`)

| Document | Description |
|----------|-------------|
| [`DOCUMENTATION_STANDARDS.md`](specifications/DOCUMENTATION_STANDARDS.md) | Metadata, citation, diagram, lifecycle mandates |
| [`REQUIREMENTS.md`](specifications/REQUIREMENTS.md) | Implemented requirements as goal specs |
| [`REPL-specification.md`](specifications/REPL-specification.md) | REPL specification — `kask chat` |
| [`wallet-specification.md`](specifications/wallet-specification.md) | Wallet crate specification |
| [`salience-specification.md`](specifications/salience-specification.md) | Passage salience algorithm |

---

## Legacy User Guides (`user-guides/`)

These user guides lack how-to equivalents and remain as legacy references:

| Document | Description |
|----------|-------------|
| [`API_GUIDE.md`](user-guides/API_GUIDE.md) | REST API usage guide |
| [`COMPANIES-GUIDE.md`](user-guides/COMPANIES-GUIDE.md) | Company research and portfolio management |
| [`bug-hunter-guide.md`](user-guides/bug-hunter-guide.md) | Bug hunting methodology and expedition execution |
| [`lora-adapter-store-guide.md`](user-guides/lora-adapter-store-guide.md) | LoRA adapter store — lifecycle, routing, deployment |
| [`QA_GUIDE.md`](user-guides/QA_GUIDE.md) | QA system operations — fuzz triage, mutation analysis, autonomous scripts |

---

## Other Documents

| Document | Description |
|----------|-------------|
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry |
| [`generated/cli-reference.md`](generated/cli-reference.md) | Auto-generated CLI reference |
| [`generated/openapi.json`](generated/openapi.json) | OpenAPI 3.1.0 specification |
| [`plans/k8s-admin-guide.md`](plans/k8s-admin-guide.md) | Kubernetes deployment and backup guide |
| [`plans/wss-chat-endpoint.md`](plans/wss-chat-endpoint.md) | WSS Chat Endpoint — design space |
| [`plans/wss-chat-endpoint-implementation.md`](plans/wss-chat-endpoint-implementation.md) | WSS Chat Endpoint — implementation guide |
| [`plans/TODO.md`](plans/TODO.md) | Open work |
| [`research/lazy-universe-research.md`](research/lazy-universe-research.md) | Least-action principle — research grounding |
| [`status/PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | Build, test, and CI health |
| [`status/replica-corpus-training-readiness.md`](status/replica-corpus-training-readiness.md) | Verified readiness of the replica, corpus, and RunPod/Unsloth workflow |

---

## Verification

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/verify-docs.sh      # Tier 1 code-anchored claim verification
```

ℏKask — A Minimal Viable Container for Replicants — v0.31.0 — Diataxis-structured documentation portal
