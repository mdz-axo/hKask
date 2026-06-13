---
title: "Skill Inventory — hKask Dual-Layer Skill Registry"
audience: [developers, curators]
last_updated: 2026-06-13
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, status]
---

# Skill Inventory — hKask Dual-Layer Skill Registry

hKask skills follow a dual-layer architecture: a **Zed agent layer** (`.agents/skills/<name>/SKILL.md`) that teaches the coding agent the methodology, and a **registry template layer** (`registry/templates/<name>/`) containing executable Jinja2 templates and a `manifest.yaml` crate definition. Bundle manifests in `registry/manifests/` define process orchestration.

## Skill Catalog (28 skills, 4 kata skills)

### Core Development Skills

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **coding-guidelines** | 3 (KnowAct) | `coding-guidelines.yaml` | Enforce Karpathy's four coding principles |
| **tdd** | 5 (FlowDef, KnowAct) | `tdd.yaml` | Test-driven development with red-green-refactor |
| **refactor-service-layer** | 3 (FlowDef) | — | Extract shared service layer via strangler fig |
| **improve-codebase-architecture** | 3 (KnowAct) | — | Find deepening opportunities in codebases |
| **deep-module** | 3 (KnowAct) | — | Module design discipline (Ousterhout) |
| **rust-expertise** | 7 (KnowAct, WordAct) | — | Idiomatic Rust design and implementation |
| **strangler-fig** | 3 (FlowDef) | — | Incremental architectural migration |

### Reasoning & Analysis Skills

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **diagnose** | 4 (KnowAct, FlowDef) | — | Disciplined diagnosis loop for hard bugs |
| **essentialist** | 1 (FlowDef) | — | Recursive eliminative interrogation (3-gate) |
| **constraint-forces** | 2 (KnowAct) | — | Classify constraints by force type |
| **pragmatic-semantics** | 3 (KnowAct) | — | Epistemic discipline for statement classification |
| **pragmatic-cybernetics** | 3 (KnowAct) | — | Cybernetic reasoning for CNS and feedback loops |
| **grill-me** | 3 (KnowAct) | — | Socratic interrogation for knowledge testing |
| **zoom-out** | 1 (KnowAct) | — | Broader context and higher-level perspective |

### Documentation & Specification Skills

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **document-update** | 7 (FlowDef, KnowAct, WordAct) | — | Systematic documentation corpus maintenance |
| **magna-carta-verifier** | 3 (KnowAct, FlowDef) | — | Verify Magna Carta P1-P4 implementation |

### Session & Workflow Skills

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **handoff** | 4 (KnowAct, WordAct) | `handoff.yaml` | Session context capture for agent handoffs |
| **condenser-continuation** | 4 (KnowAct, FlowDef) | — | Resume condenser implementation after context reset |

### Skill Management Skills (Meta)

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **skill-bundler** | 3 (FlowDef) | — | Compose multiple skills into cohesive bundles |
| **skill-discovery** | 2 (KnowAct) | — | Find, evaluate, and install dual-layer skills |
| **skill-maintenance** | 2 (KnowAct) | — | Audit skill architecture for staleness and gaps |
| **skill-manager** | 2 (FlowDef) | — | Dual-layer CRUD for the skill corpus |
| **skill-translator** | 2 (FlowDef) | — | Translate external skills into hKask dual-layer format |

### Kata Skills (Toyota Kata System)

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **kata-starter** | 5 (4 FlowDef, 1 KnowAct) | `starter-kata.yaml` | Build foundational scientific thinking habits |
| **kata-improvement** | 5 (1 FlowDef, 4 WordAct) | `improvement-kata.yaml` | 4-step scientific pattern for goal achievement |
| **kata-coaching** | 6 (1 FlowDef, 5 WordAct) | `coaching-kata.yaml` | 5-question dialogue for teaching scientific thinking |
| **kata** (bundle) | 7 (6 KnowAct, 1 WordAct) | `kata-pattern.yaml` | Full system orchestration composing the three kata skills |

### Other Skills

| Skill | Templates | Manifest | Purpose |
|-------|-----------|----------|---------|
| **caveman** | 1 (WordAct) | — | Minimalist communication pattern |

## Summary Statistics

| Metric | Count |
|--------|-------|
| Total skills (Zed layer) | 28 |
| Skills with registry templates | 28 |
| Total Jinja2 templates | 103 |
| Skills with bundle manifests | 10 |
| Kata-specific skills | 4 (3 individual + 1 bundle) |
| Kata templates | 23 |
| Kata manifests | 5 |

## Skill Location Map

| Layer | Location | Format |
|-------|----------|--------|
| Zed agent instructions | `.agents/skills/<name>/SKILL.md` | Markdown with YAML front matter |
| Template crate manifest | `registry/templates/<name>/manifest.yaml` | YAML with `crate:` and `templates:` sections |
| Jinja2 templates | `registry/templates/<name>/*.j2` | Jinja2 with `{# Template: ... #}` header |
| Bundle manifests | `registry/manifests/<name>.yaml` | YAML with `manifest:`, `steps:`, `gas:`, `cns:`, `ocap:` |
| Bootstrap registration | `registry/templates/bootstrap-registry.yaml` | YAML array of `RegistryEntry` structs |
| hLexicon domain registries | `registry/hlexicon/<domain>-hlexicon.yaml` | YAML with functional role categorization |
| Hexagonal ports | `registry/ports/<domain>-ports.yaml` | YAML with inbound/outbound port definitions |

## Skill Lifecycle

Skills follow the src→dist pattern with two zones:

| Zone | Directory | Visibility | Lifecycle |
|------|-----------|------------|-----------|
| **Private** (source of truth) | `.agents/skills/` | Author's working copies | Created, edited, reviewed |
| **Public** (export surface) | `skills/` | Generated by `kask skill publish` | Published, distributed |

The `SkillLoader` in `hkask-templates` scans both zones, parses YAML front matter, validates zone-vs-visibility consistency, and registers skills into the `SkillRegistryIndex`.

---

*ℏKask - A Minimal Viable Container for Agents — Skill Inventory — v0.27.0*
