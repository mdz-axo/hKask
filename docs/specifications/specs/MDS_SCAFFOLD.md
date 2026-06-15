---
title: "MDS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS Documentation Scaffold

**Purpose:** Maps the MDS 5-category taxonomy to directory locations and enforces the lifecycle policy. Two jobs, nothing else.

**Role:** This document tells you what documentation to produce and where to put it. Verification that what you produced is correct is governed by [`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md). Current inventory lives in [`docs/README.md`](../README.md) (portal) and [`docs/status/corpus_inventory.yaml`](../../status/corpus_inventory.yaml) (lifecycle classification) — this scaffold does not duplicate them.

**MDS Reference:** [`architecture/MDS.md`](../../architecture/core/MDS.md)

---

## 1. Category → Directory Mapping

Where each MDS category's authoritative documents live:

| # | MDS Category | Authoritative Document | Primary Directory | Supporting References |
|---|--------------|----------------------|-------------------|----------------------|
| 1 | **Domain** | [`MDS.md`](../../architecture/core/MDS.md) | `architecture/` | [`reference/hKask-hLexicon.md`](../../architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](../../architecture/reference/hKask-Curator-persona.md) |
| 2 | **Composition** | [`MDS.md`](../../architecture/core/MDS.md) | `architecture/` | [`reference/template-header-standard.md`](../../architecture/reference/template-header-standard.md) |
| 3 | **Trust** | [`MDS.md`](../../architecture/core/MDS.md) | `architecture/` | [`magna-carta.md`](../../architecture/core/magna-carta.md) |
| 4 | **Lifecycle** | [`MDS.md`](../../architecture/core/MDS.md) | `architecture/` | [`CI-CD-GUIDE.md`](../../guides/CI-CD-GUIDE.md), [`DEPLOYMENT.md`](../../guides/DEPLOYMENT.md) |
| 5 | **Curation** | [`MDS.md`](../../architecture/core/MDS.md) + [`WRITING_EXCELLENCE.md`](../standards/WRITING_EXCELLENCE.md) | `architecture/` + `specifications/` | — |

**Rule:** New documents go in the directory of their primary MDS category. Cross-cutting documents (multiple categories) go in the directory of their dominant category. The portal provides category-based navigation; this scaffold provides the placement rule.

---

## 2. Lifecycle Enforcement

Per [`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md) §3:

```
Draft → Active → Deprecated → Superseded → Removed
```

| State | Rule |
|-------|------|
| **Active** | Must map to ≥1 MDS category via `mds_categories` frontmatter |
| **Deprecated** | Moved to `docs/archive/YYYY-MM-DD-<label>/` |
| **Superseded** | Moved to `docs/archive/YYYY-MM-DD-<label>/`; successor must reference it |
| **Removed** | `git rm` from working tree; git history is archive of record |
| **Archive** | `docs/archive/` is gitignored per [`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md) §3 |

[^nygard-adr]: Nygard, M. (2011). *Documenting Architecture Decisions.* http://thinkrelevance.com/blog/2011/11/15/documenting-architecture-decisions — ADR lifecycle states that MDS_SCAFFOLD extends to all document types.

---

## 3. Spec-Code Drift

Spec-code alignment is tracked in the corpus inventory, not duplicated here:

| Tracking File | Purpose |
|---------------|---------|
| [`corpus_inventory.yaml`](../../status/corpus_inventory.yaml) | Lifecycle classification + drift tracking (spec-code-drift.yaml and curation-decisions.yaml archived 2026-06-15 — their function merged into corpus_inventory.yaml) |

**Rule:** When spec and code diverge, the drift is captured in `corpus_inventory.yaml` staleness signals. This scaffold does not maintain a duplicate completeness table.

---

## 4. Metadata Requirements

Per [`DOCUMENTATION_STANDARDS.md`](../standards/DOCUMENTATION_STANDARDS.md) §2. Every active document must have YAML frontmatter with `title`, `audience`, `last_updated`, `version`, `status`, `domain`, `mds_categories`.

---

## 5. Verification

```bash
bash docs/ci/check-links.sh      # Zero broken cross-references
bash docs/ci/check-metadata.sh   # All active docs have required frontmatter
bash docs/ci/sync-versions.sh --dry-run  # Version fields match workspace Cargo.toml
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
