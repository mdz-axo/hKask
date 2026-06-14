---
title: "Document Corpus Roadmap — Prioritized"
audience: [project maintainers, documentation stewards, agents]
last_updated: 2026-06-14
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Document Corpus Roadmap — Prioritized

**Source:** [`docs/OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) — Document Futures section (2026-06-14 hygiene sweep)  
**Governing Principles:** P5 (Pared Surface), P6 (No Dead Docs), P8 (Semantic Grounding)

---

## Priority Tiers

| Tier | Definition | Target |
|------|-----------|--------|
| **P0 — Blocking** | Referenced by ≥2 authoritative documents; absent causes CI or correctness failures | This sprint |
| **P1 — High** | Unblocks verification workflow; removes false "broken" signals; fixes metadata drift | This release |
| **P2 — Medium** | Improves corpus health; enables new workflows; addresses known gaps | Next release |
| **P3 — Low** | Nice-to-have; user-facing growth; exploratory | Backlog |

---

## P0 — Blocking (This Sprint)

### P0-1: Fix `corpus_inventory.yaml` — CI Scripts Incorrectly Listed as Missing

**Status:** `docs/ci/check-links.sh` and `docs/ci/check-metadata.sh` both exist and are fully implemented, but `corpus_inventory.yaml` `missing_referenced` section lists them as "Missing." Regenerate the inventory or manually correct.

**Verification:** Re-run `check-links.sh` (251 links, 0 broken) and `check-metadata.sh` (67 docs checked, 5 fail metadata, 1 warning) — both scripts PASS their core functions.

### P0-2: Create `docs/status/spec-code-drift.yaml`

**Referenced by:** README portal (L116), MDS_SCAFFOLD, architecture master  
**Purpose:** Set-difference of named entities from spec documents against `pub` API surfaces. Core infrastructure for P8 enforcement.  
**Approach:** Start as manual inventory; automate via `cargo doc` or `cargo public-api` later.  
**Verification:** README portal line 116 currently says "not yet created" — this reference must be resolved.

### P0-3: Create `docs/status/curation-decisions.yaml`

**Referenced by:** README portal (L116), MDS_SCAFFOLD  
**Purpose:** Merge/Revise decisions per MDS curation protocol for each drift item.  
**Approach:** Start as manual log; link to spec-code-drift.yaml entries.  
**Verification:** README portal reference resolved.

---

## P1 — High (This Release)

### P1-1: Fix Metadata on 5 Audit Documents

**Current status:** 5 files in `docs/audit/` lack YAML frontmatter entirely (confirmed by `check-metadata.sh`).  
**Action:** Add minimal frontmatter with `status: "Deprecated"` and `mds_categories: [curation]` since these are completed audit bundles with historical value only.  
**Files:** `crate-audit-task1-cartography.md`, `crate-audit-task2-cybernetics.md`, `crate-audit-task3-rust-audit.md`, `crate-audit-task4-implementation.md`, `crate-audit-task5-open-questions.md`

### P1-2: Fix `mds_categories` on `docs/status/skill-inventory.md`

**Current:** `mds_categories: [composition, status]` — `status` is not a valid MDS category.  
**Fix:** Change to `mds_categories: [composition, curation]`.  
**Verification:** `check-metadata.sh` currently flags this as a warning.

### P1-3: Fix 18 Version Anomalies

Documents with `version` diverging from workspace `0.27.0`:

| Files tracking their own version (document divergence) | Files needing bump to 0.27.0 |
|---|---|
| `MDS_SCAFFOLD.md` (2.5.0) | `DOCUMENTATION_STANDARDS.md` (0.8.0) |
| `TRACEABILITY_MATRIX.md` (1.3.0) | `WRITING_EXCELLENCE.md` (0.3.0) |
| `test-inventory.md` (2.4.0) | `ADR_TEMPLATE.md` (1.0.0) |
| `mcp-tools-inventory.md` (2.1.0) | `DEPENDENCY_POLICY.md` (1.0.0) |
| `reference/template-header-standard.md` (0.23.0) | `CI-CD-GUIDE.md` (1.0.0) |
| `reference/hKask-Curator-persona.md` (1.1.0) | `TESTING_STANDARDS.md` (1.0.0) |
| `reference/utoipa-implementation.md` (1.0.0) | `test-program.md` (0.1.0) |
| `reference/okapi-integration.md` (0.28.0) | `MDS-agent-service.md` (0.27.2) |
| `plans/TODO.md` (1.9.0) | `crate-audit.md` (1.0.0) |
| | `gentle-lovelace-specification.md` (0.1.0) |
| | `plans/bundler-completion.md` (0.1.0) |
| | `plans/mcp-server-roadmap.md` (1.0.0) |
| | `plans/mcp-media-server-design.md` (1.0.0) |
| | `adversarial-simplification-inventory.md` (0.1.0) |

**Approach:** Left column — document in `corpus_inventory.yaml` that version is intentional. Right column — bump to `0.27.0`.

### P1-4: Create `docs/ci/sync-versions.sh`

Automates version field bumping across all YAML-frontmatter documents when workspace version changes.  
**Input:** New version string (from `Cargo.toml`).  
**Output:** Updated `version:` fields in all `.md` files with frontmatter. Skips files on an explicit exclusion list.

### P1-5: Update `MDS_SCAFFOLD.md` Document Structure

**Current:** References files that no longer exist (`ci/check-links.sh` and `ci/check-metadata.sh` — these now DO exist but weren't listed) and omits 10 active documents.  
**Action:** Re-sync MDS_SCAFFOLD's document structure tree with current corpus state.

---

## P2 — Medium (Next Release)

### P2-1: Generate `docs/generated/openapi.json`

**Referenced by:** MDS_SCAFFOLD  
**Approach:** Add `cargo doc` or utoipa build step to CI pipeline.  
**Dependency:** Requires `hkask-api` crate to have utoipa annotations for all endpoints.

### P2-2: Missing ADRs

Three architectural domains currently encoded only in handoffs and code:
- **OCR pipeline ADR** — sealed type hierarchy, deterministic routing, pluggable backends in `hkask-mcp-markitdown`
- **Wallet payments ADR** — payment mechanism architecture
- **Media server ADR** — 28-tool media MCP server architecture

### P2-3: Codify Archive Policy as Living Document

Extract archive policy from `docs/archive/MANIFEST.md` and `HANDOFF_LIFECYCLE.md` into a consolidated section in `DOCUMENTATION_STANDARDS.md` or a standalone `ARCHIVE_POLICY.md`.

### P2-4: Runbook / Operational Guide

No operational documentation exists for hKask deployments. Minimum viable: cloud server daemon setup, troubleshooting, log locations, key rotation procedure.

### P2-5: Migrate `docs/audit/` to Archive

Completed audit bundles (5 files) are static historical records with no forward-looking value. After adding frontmatter (P1-1), consider archiving the entire `docs/audit/` directory.

### P2-6: Standardize Plan Document Versioning

Plan documents (in `docs/plans/`) currently use inconsistent versions (1.9.0, 0.1.0, 1.0.0, 0.27.0). Standardize: plans should track project version (`0.27.0`), not their own numbering. Exceptions documented in corpus inventory.

---

## P3 — Low (Backlog)

### P3-1: Crate-Specific Onboarding Guides

Currently only 3 user guides exist (agent pod creation, common patterns, companies). 18 crates — especially new ones (`hkask-mcp-media`, `hkask-mcp-companies`) — would benefit from onboarding guides.

### P3-2: Replicant Onboarding Walkthrough

End-to-end guide from `kask onboard` through first `kask chat` session with a named replicant. Currently no single document covers the full onboarding flow.

### P3-3: Essentialist Auto-Culling

Automated deletion test: script that checks if a document is referenced from any index (portal, architecture master, AGENTS.md). Unreferenced documents are candidates for archival.

### P3-4: Corpus Inventory Regeneration Automation

Currently manual (agent runs classification sweep). Automate via script that re-runs the classification logic.

### P3-5: Pre-Commit Hook for Version Anomalies

Hook that flags documents where `version` ≠ workspace `Cargo.toml` version, with an exclusion list for intentionally divergent documents.

---

## Quick Wins (Under 30 Minutes)

| # | Task | Effort |
|---|------|--------|
| QW-1 | Fix `skill-inventory.md` mds_categories: s/status/curation/ | 1 min |
| QW-2 | Add frontmatter to 5 audit documents (all same template) | 15 min |
| QW-3 | Fix `corpus_inventory.yaml` missing_referenced: remove check-links.sh + check-metadata.sh | 5 min |
| QW-4 | Bump 14 document versions from various → 0.27.0 | 15 min |
| QW-5 | Create `spec-code-drift.yaml` stub with section headers | 15 min |
| QW-6 | Create `curation-decisions.yaml` stub with section headers | 15 min |

---

## Dependency Graph

```
P1-1 (audit frontmatter)
  ↓
P2-5 (archive audit/)

P0-2 (spec-code-drift.yaml)
  ↓
P0-3 (curation-decisions.yaml)

P1-3 (fix version anomalies)
  ↓
P1-4 (create sync-versions.sh)

P1-5 + P1-2 + QW-1 (metadata fixes)
  ↓
Re-run check-metadata.sh → 0 errors
```

---

## Verification Gates

After each tier is complete:

| Tier | Gate |
|------|------|
| P0 | `check-links.sh` passes; README portal has no "not yet created" references; spec-code-drift.yaml and curation-decisions.yaml exist |
| P1 | `check-metadata.sh` passes with 0 errors; all version anomalies resolved or documented; sync-versions.sh functional |
| P2 | `openapi.json` generated; missing ADRs exist; audit docs archived |
| P3 | Backlog grooming complete |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
