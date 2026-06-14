---
title: "MDS Documentation Scaffold"
audience: [architects, documentation maintainers, agents]
last_updated: 2026-06-14
version: "2.5.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS Documentation Scaffold

**Purpose:** Maps the MDS 5-category taxonomy to directory locations and enforces the lifecycle policy. Two jobs, nothing else.

**Role:** This document tells you what documentation to produce and where to put it. Verification that what you produced is correct is governed by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md). Current inventory lives in [`docs/README.md`](../README.md) (portal) and [`docs/status/corpus_inventory.yaml`](../status/corpus_inventory.yaml) (lifecycle classification) — this scaffold does not duplicate them.

**MDS Reference:** [`architecture/MDS.md`](../architecture/MDS.md)

---

## 1. Category → Directory Mapping

Where each MDS category's authoritative documents live:

| # | MDS Category | Authoritative Document | Primary Directory | Supporting References |
|---|--------------|----------------------|-------------------|----------------------|
| 1 | **Domain** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`reference/hKask-hLexicon.md`](../architecture/reference/hKask-hLexicon.md), [`reference/hKask-Curator-persona.md`](../architecture/reference/hKask-Curator-persona.md) |
| 2 | **Composition** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`reference/template-header-standard.md`](../architecture/reference/template-header-standard.md) |
| 3 | **Trust** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`magna-carta.md`](../architecture/magna-carta.md) |
| 4 | **Lifecycle** | [`MDS.md`](../architecture/MDS.md) | `architecture/` | [`CI-CD-GUIDE.md`](CI-CD-GUIDE.md), [`DEPLOYMENT.md`](DEPLOYMENT.md) |
| 5 | **Curation** | [`MDS.md`](../architecture/MDS.md) + [`WRITING_EXCELLENCE.md`](WRITING_EXCELLENCE.md) | `architecture/` + `specifications/` | — |

**Rule:** New documents go in the directory of their primary MDS category. Cross-cutting documents (multiple categories) go in the directory of their dominant category. The portal provides category-based navigation; this scaffold provides the placement rule.

---

## 2. Lifecycle Enforcement

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §3:

```
Draft → Active → Deprecated → Superseded → Removed
```

| State | Rule |
|-------|------|
| **Active** | Must map to ≥1 MDS category via `mds_categories` frontmatter |
| **Deprecated** | Moved to `docs/archive/YYYY-MM-DD-<label>/` |
| **Superseded** | Moved to `docs/archive/YYYY-MM-DD-<label>/`; successor must reference it |
| **Removed** | `git rm` from working tree; git history is archive of record |
| **Archive** | `docs/archive/` is gitignored per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §3 |

[^nygard-adr]: Nygard, M. (2011). *Documenting Architecture Decisions.* http://thinkrelevance.com/blog/2011/11/15/documenting-architecture-decisions — ADR lifecycle states that MDS_SCAFFOLD extends to all document types.

---

## 3. Spec-Code Drift

Spec-code alignment is tracked in dedicated files, not duplicated here:

| Tracking File | Purpose |
|---------------|---------|
| [`spec-code-drift.yaml`](../status/spec-code-drift.yaml) | Set-difference of named entities from spec docs against `pub` API surfaces |
| [`curation-decisions.yaml`](../status/curation-decisions.yaml) | Merge/Revise/Defer/Discard decisions per drift item |

**Rule:** When spec and code diverge, record the drift item in `spec-code-drift.yaml` and the curation decision in `curation-decisions.yaml`. This scaffold does not maintain a duplicate completeness table.

---

## 4. Metadata Requirements

Per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §2. Every active document must have YAML frontmatter with `title`, `audience`, `last_updated`, `version`, `status`, `domain`, `mds_categories`.

---

## 5. Verification

```bash
bash docs/ci/check-links.sh      # Zero broken cross-references
bash docs/ci/check-metadata.sh   # All active docs have required frontmatter
bash docs/ci/sync-versions.sh --dry-run  # Version fields match workspace Cargo.toml
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
