---
title: "hKask Documentation Portal"
audience: [project maintainers, contributors, architects, agents]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Documentation Portal

**Purpose:** Single entry point indexing every active document in `docs/`, tagged by [MDS](architecture/core/MDS.md) category. hKask v0.31.0.

### Diataxis Structure

Documentation is organized by [Diataxis](https://diataxis.fr/) quadrants — tutorials, how-to guides, reference, and explanation — supplemented by architecture, specifications, and the diagram verification registry.

> **Lifecycle:** Retired documents are moved to `docs/archive/`. Git history preserves all versions.
>
> **Diagram policy:** Per `DOCUMENTATION_STANDARDS.md` §1, all Mermaid diagrams are inline in the documents they describe. The former `docs/diagrams/` directory has been consolidated — see [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) for the verification registry.

---

## Tutorial

| Document | Description |
|----------|-------------|
| [`how-to/getting-started.md`](how-to/getting-started.md) | End-to-end walkthrough — install, configure, first chat |

---

## How-To Guides (`how-to/`)

| Guide | What You'll Do |
|-------|----------------|
| [`install-and-configure.md`](how-to/install-and-configure.md) | Install hKask, configure, and run your first session (includes bootstrap diagram) |
| [`agents-and-pods.md`](how-to/agents-and-pods.md) | Create and manage agent pods (includes pod lifecycle state diagram) |
| [`deployment-and-transport.md`](how-to/deployment-and-transport.md) | Deploy on Kubernetes, configure Matrix transport, OAuth/invite flow (includes 6 deployment diagrams) |
| [`skills-and-composition.md`](how-to/skills-and-composition.md) | Install, invoke, compose skills; run Kata cycles (includes 2 kata/skills diagrams) |
| [`training-and-adapters.md`](how-to/training-and-adapters.md) | Train Qwen3.6-27B on RunPod with Unsloth; LoRA adapter training (includes 6 training diagrams + Qwen3.6 hyperparameter reference) |
| [`sovereignty-and-observability.md`](how-to/sovereignty-and-observability.md) | Audit sovereignty compliance, read CNS alerts |

---

## Reference (`reference/`)

| Document | Description |
|----------|-------------|
| [`api-reference.md`](reference/api-reference.md) | API reference for hKask crates (includes 9 inlined CodeGraph + TUI diagrams) |
| [`cns-spans.md`](reference/cns-spans.md) | CNS span catalog and namespaces |
| [`magna-carta.md`](reference/magna-carta.md) | Magna Carta — 4 inviolable sovereignty principles |
| [`mcp-servers/README.md`](reference/mcp-servers/README.md) | MCP server reference — 15 built-in servers (includes Companies MCP server) |
| [`skills/README.md`](reference/skills/README.md) | Skills registry — manifests and metadata |

---

## Explanation (`explanation/`)

| Document | Topic |
|----------|-------|
| [`architecture-patterns.md`](explanation/architecture-patterns.md) | Hexagonal ports, service layer, template cascade, MCP dispatch patterns (includes 7 inlined diagrams + template authorship) |
| [`cns-and-loops.md`](explanation/cns-and-loops.md) | CNS homeostatic regulation, algedonic escalation, loop action lifecycle (includes 8 inlined diagrams) |
| [`cognition-and-replica.md`](explanation/cognition-and-replica.md) | Memory pipeline, classification, embedding architecture (includes 4 inlined diagrams) |
| [`sovereignty-and-ocap.md`](explanation/sovereignty-and-ocap.md) | OCAP attenuation, consent flow, guard pipeline (includes 4 inlined diagrams) |
| [`federation-and-transport.md`](explanation/federation-and-transport.md) | Federation dispatch model, adapter lifecycle (includes 1 inlined diagram) |
| [`energy-and-economy.md`](explanation/energy-and-economy.md) | Energy and gas payment system |

---

## Architecture (`architecture/`)

| Document | Description |
|----------|-------------|
| [`core/hKask-architecture-master.md`](architecture/core/hKask-architecture-master.md) | Authoritative architecture index — 4 patterns, four-loop decomposition, economic layer, pod architecture, Curator persona. Includes merged sections: Database Providers, Matrix Integration, Well Wallet, Scenarios–Companies bridge, Federation V2, and 8 inlined storage/database/scenarios diagrams. |
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry — all diagrams now inline |

### Core (`architecture/core/`)

| Document | Description |
|----------|-------------|
| [`FUNCTIONAL_SPECIFICATION.md`](architecture/core/FUNCTIONAL_SPECIFICATION.md) | Functional specification — 26 domains, 14 inline ERD/flowchart diagrams. Includes merged Bloom QA Pipeline and Cross-Reference QA sections. |
| [`magna-carta.md`](architecture/core/magna-carta.md) | User sovereignty charter — 4 inviolable principles |
| [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md) | Architecture principles (P1-P12), dual-axis framework |
| [`MDS.md`](architecture/core/MDS.md) | Minimal Domain Specification — 5 categories |
| [`TESTING_DISCIPLINE.md`](architecture/core/TESTING_DISCIPLINE.md) | Contract-anchored testing — DbC, PBT, fuzz, mutation |

### ADRs (`architecture/ADRs/`)

17 ADRs covering consolidation authorization, replicant server mode, gix migration, BLAKE3 content addressing, dynamic model discovery, port promotion, database driver, nested runtime panics, ledger-wallet separation, CLI bootstrap, REPL extraction, storage modularization, CNS type decomposition, ledger database driver compliance, Qwen3.6 chat template and training gap analysis. See directory for full index.

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

## Other Documents

| Document | Description |
|----------|-------------|
| [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) | Underspecified aspects — open crossroads and future design decisions |
| [`DIAGRAMS_INDEX.md`](DIAGRAMS_INDEX.md) | Mermaid diagram verification registry — all diagrams now inline in parent documents |
| [`diagrams/flowchart-documentation-structure.md`](diagrams/flowchart-documentation-structure.md) | Diataxis navigation map — quadrant structure of the `docs/` directory |

---

## Verification

```bash
bash docs/ci/check-links.sh      # link integrity — zero broken links
bash docs/ci/verify-docs.sh      # Tier 1 code-anchored claim verification
```

ℏKask — A Minimal Viable Container for Replicants — v0.31.0 — Diataxis-structured documentation portal