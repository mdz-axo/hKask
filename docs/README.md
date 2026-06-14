---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-06-14
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

The architecture is specified in eight MDS-aligned documents, each authoritative for its category cluster.

| Document | MDS Categories | Description |
|----------|-------------------|-------------|
| [`MDS.md`](architecture/MDS.md) | domain, composition, trust, lifecycle, curation | Minimal Domain Specification — 5 categories, 5 tools, completeness predicate |
| [`PRINCIPLES.md`](architecture/PRINCIPLES.md) | domain, composition, trust, lifecycle, curation | Architecture principles (P1-P12), 5 anchors, anti-patterns |
| [`magna-carta.md`](architecture/magna-carta.md) | domain, composition, trust, lifecycle, curation | User sovereignty charter — 4 inviolable principles |
| [`loop-architecture.md`](architecture/loop-architecture.md) | domain, composition, lifecycle, curation | 4-loop architecture — RateLimiting→EnergyBudget, crate↔loop mapping |
| [`wallet-specification.md`](architecture/wallet-specification.md) | domain, composition, trust, lifecycle | Wallet crate — architectural specification |
| [`P12-replicant-host-mandate.md`](architecture/P12-replicant-host-mandate.md) | domain, trust, composition | Replicant host mandate — P12 elaboration |
| [`energy-gas-payments-api-keys.md`](architecture/energy-gas-payments-api-keys.md) | domain, trust, lifecycle, curation | Energy, gas, payments & API key architecture |
| [`lazy-universe-research.md`](architecture/lazy-universe-research.md) | domain, composition, curation | Least-action principle — research grounding |

### Architecture Decision Records

| ADR | MDS | Decision | Status |
|-----|--------|----------|--------|
| [ADR-024](architecture/ADR-024-unified-registry.md) | composition | Unified registry decision | Active |
| [ADR-025](architecture/ADR-025-attenuation-depth-limit.md) | trust | 7-level attenuation depth limit | Active |
| [ADR-026](architecture/ADR-026-bitemporal-triple-schema.md) | lifecycle | Bitemporal triple schema | Active |
| [ADR-027](architecture/ADR-027-argon2-hkdf-master-key.md) | trust | Argon2id + HKDF-SHA256 master key derivation | Active |
| [ADR-030](architecture/ADR-030-skill-bundler.md) | composition | Skill Bundler — meta-skill composition | Proposed |
| [ADR-031](architecture/ADR-031-consolidation-authorization.md) | trust | Consolidation authorization via master passphrase derivation | Active |
| [ADR-032](architecture/ADR-032-mcp-gateway-membrane.md) | composition, trust | MCP gateway membrane policy — Tier 1 (governed) vs Tier 2 (passthrough) | Draft |
| [ADR-033](architecture/ADR-033-dampener-override-cooldown.md) | trust, lifecycle | Dampener override cooldown — per-issuer vs global | Draft |
| [ADR-034](architecture/ADR-034-academic-author-pipeline.md) | composition, curation | Academic author pipeline architecture | Draft |
| [ADR-035](architecture/ADR-035-replicant-server-mode.md) | composition, trust, lifecycle | Replicant server mode — AgentMode, daemon transport, dual memory | Active |
| [ADR-036](architecture/ADR-036-ocr-pipeline.md) | composition, curation | OCR pipeline — sealed backend hierarchy, deterministic routing | Draft |
| [ADR-037](architecture/ADR-037-wallet-payments.md) | domain, trust, lifecycle | Wallet payment mechanism — rJoule currency, multi-chain bridges | Draft |
| [ADR-038](architecture/ADR-038-media-server.md) | composition, domain | Media MCP server — 36 tools, fal.ai backend, single-server architecture | Draft |

### Reference Artifacts (`architecture/reference/`)

| Document | MDS | Description |
|----------|--------|-------------|

| [`hKask-hLexicon.md`](architecture/reference/hKask-hLexicon.md) | domain | Minimal composition vocabulary |
| [`ports-inventory.md`](architecture/reference/ports-inventory.md) | interface | Hexagonal port inventory |
| [`okapi-integration.md`](architecture/reference/okapi-integration.md) | domain | Inference Router API contract |
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
| [`test-program.md`](specifications/test-program.md) | domain, composition, trust, lifecycle, curation | Test program specification |
| [`REPL-specification.md`](specifications/REPL-specification.md) | domain, composition, lifecycle, curation | REPL specification — `kask chat` |
| [`MDS-agent-service.md`](specifications/MDS-agent-service.md) | domain, composition, trust, lifecycle | AgentService specification |
| [`salience-specification.md`](specifications/salience-specification.md) | domain, composition | Passage salience algorithm specification |
| [`crate-audit.md`](specifications/crate-audit.md) | composition, curation | Crate audit bundle manifest |
| [`HANDOFF_LIFECYCLE.md`](specifications/HANDOFF_LIFECYCLE.md) | lifecycle, curation | Handoff lifecycle policy — states, 30-day staleness rule, archive procedure |
| [`DOCUMENT_OWNERSHIP.md`](specifications/DOCUMENT_OWNERSHIP.md) | lifecycle, curation | Document category ownership, version sync policy, review cadence |

---

## Status (`status/`)

| Document | MDS | Description |
|----------|--------|-------------|
| [`PROJECT_STATUS.md`](status/PROJECT_STATUS.md) | lifecycle | Build, test, and CI health |
| [`test-inventory.md`](status/test-inventory.md) | lifecycle, curation | Test inventory from `cargo test --list` |
| [`mcp-tools-inventory.md`](status/mcp-tools-inventory.md) | composition, lifecycle | MCP server tool catalog — 141 tools across 10 servers |
| [`skill-inventory.md`](status/skill-inventory.md) | composition, curation | Dual-layer skill registry — 33 skills cataloged |
| [`adversarial-simplification-inventory.md`](status/adversarial-simplification-inventory.md) | composition, domain | Dead code and simplification opportunities |
| [`spec-code-drift.yaml`](status/spec-code-drift.yaml) | domain, composition, trust, lifecycle, curation | Spec-code drift tracking — 14/14 items resolved (2026-06-12) |
| [`curation-decisions.yaml`](status/curation-decisions.yaml) | domain, composition, trust, lifecycle, curation | Curation decisions per drift item — 14 decisions recorded |
| [`corpus_inventory.yaml`](status/corpus_inventory.yaml) | lifecycle, curation | Document corpus lifecycle classification (generated 2026-06-14, updated 2026-06-14) |

---

## Plans (`plans/`)

Open work and design drafts. Drafts (`Status: Draft`) are exploratory and not authoritative.

| Document | MDS | Description |
|----------|--------|-------------|
| [`TODO.md`](plans/TODO.md) | domain, composition, trust, lifecycle, curation | Open work items |
| [`bundler-completion.md`](plans/bundler-completion.md) | domain, composition, lifecycle | Bundler completion — remaining tasks |
| [`mcp-server-roadmap.md`](plans/mcp-server-roadmap.md) | domain, composition, trust, lifecycle, curation | MCP server consolidation roadmap |
| [`2026-06-12-replicant-server-mode.md`](plans/2026-06-12-replicant-server-mode.md) | composition, trust, lifecycle | Replicant server mode handoff |
| [`2026-06-12-wallet-payment-mechanism.md`](plans/2026-06-12-wallet-payment-mechanism.md) | domain, composition, trust, lifecycle | Wallet payment integration plan |
| [`2026-06-12-wallet-rjoule-payments.md`](plans/2026-06-12-wallet-rjoule-payments.md) | domain, composition, trust, lifecycle | Wallet rJoule multi-chain plan |
| [`mcp-media-server-design.md`](plans/mcp-media-server-design.md) | domain, composition, lifecycle | MCP media server — design & implementation plan |
| [`DOCUMENT_ROADMAP.md`](plans/DOCUMENT_ROADMAP.md) | lifecycle, curation | Document corpus roadmap — P0→P3 prioritized (2026-06-14) |

---

## Handoffs (`handoffs/`)

Transient session handoffs recording implementation state. Handoffs are committed to git history and cleaned from the working tree when superseded or when their context is no longer needed. All handoffs are recoverable via git history.

*No active handoffs in working tree.* See git history for past handoffs (`git log -- docs/handoffs/`).

---

## Guides (`guides/`)

| Document | MDS | Description |
|----------|--------|-------------|
| [`kata-user-guide.md`](guides/kata-user-guide.md) | composition, lifecycle | Toyota Kata — research background, technical build, user how-to |
| [`OPERATIONS_RUNBOOK.md`](guides/OPERATIONS_RUNBOOK.md) | lifecycle, trust | Operations runbook — deployment, health checks, troubleshooting, backup/recovery |

---

## User Guides (`user-guides/`)

| Document | MDS | Description |
|----------|--------|-------------|
| [`AGENT-POD-CREATION-GUIDE.md`](user-guides/AGENT-POD-CREATION-GUIDE.md) | domain | Creating agent pods |
| [`COMMON-AGENT-PATTERNS.md`](user-guides/COMMON-AGENT-PATTERNS.md) | domain | Common agent patterns and templates |
| [`COMPANIES-GUIDE.md`](user-guides/COMPANIES-GUIDE.md) | domain, composition | Company research and portfolio management user guide |

---

## Cross-Cutting Indexes

| Document | MDS | Description |
|----------|--------|-------------|
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | domain, composition, trust, lifecycle, curation | Mermaid diagram verification registry |
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | domain, composition, lifecycle, curation | Underspecified aspects |

| [`generated/cli-reference.md`](generated/cli-reference.md) | composition | Auto-generated CLI reference |

---

## Verification

Documentation quality gates (run from the repository root):

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/check-metadata.sh   # mandatory metadata headers on every active doc
bash docs/ci/sync-versions.sh --dry-run  # version synchronization check
```

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
