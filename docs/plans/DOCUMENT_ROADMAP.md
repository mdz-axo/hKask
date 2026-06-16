---
title: "Document Corpus Roadmap — Prioritized"
audience: [project maintainers, documentation stewards, agents]
last_updated: 2026-06-15-sweep
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

### P0-4: Consolidate Testing Documentation ✅ DONE (2026-06-15)

**Status:** `docs/specifications/specs/test-program.md` and `docs/specifications/standards/TESTING_STANDARDS.md` archived to `docs/archive/2026-06-15-testing-consolidation/`. Single authoritative document: `docs/architecture/core/TESTING_DISCIPLINE.md` (565 lines, 10 sections). Anchored on Design by Contract (Meyer, 1986) + Property-Based Testing (QuickCheck, 2000). TDD skill and coding-guidelines skill updated to reference the new discipline. CNS spans `cns.contract.violated` and `cns.contract.coverage` registered in canonical CNS span registry (`crates/hkask-types/src/cns.rs`, `CnsSpan`) and `CANONICAL_NAMESPACES`.

### P0-1: Fix `corpus_inventory.yaml` — CI Scripts Incorrectly Listed as Missing

**Status:** `docs/ci/check-links.sh` and `docs/ci/check-metadata.sh` both exist and are fully implemented, but `corpus_inventory.yaml` `missing_referenced` section lists them as "Missing." Regenerate the inventory or manually correct.

**Verification:** Re-run `check-links.sh` (251 links, 0 broken) and `check-metadata.sh` (67 docs checked, 5 fail metadata, 1 warning) — both scripts PASS their core functions.

### P0-2: Create `do../status/corpus_inventory.yaml` ✅ DONE

**Status:** File exists at `docs/status/corpus_inventory.yaml`. Content may need periodic regeneration but the stub is in place. README portal reference resolved.

### P0-3: Create `docs/status/curation_decisions.yaml` — Obsolete

**Status:** Archived 2026-06-15 per README portal. `curation-decisions.yaml` served its purpose and was removed from the working tree. Recoverable via git history. No action needed.

---

## P1 — High (This Release)

### P1-1: Fix Metadata on 5 Audit Documents ✅ DONE

**Status:** Archived 2026-06-14. All 5 audit documents had YAML frontmatter (were not actually missing). Archived per essentialist Gate 1 — no forward-looking value.

### P1-2: Fix `mds_categories` on `docs/status/skill-inventory.md` — Obsolete

**Status:** Archived 2026-06-15. `skill-inventory.md` was removed from the working tree per README portal. Recoverable via git history. No action needed.

### P1-3: Fix 18 Version Anomalies ✅ DONE

**Status:** Resolved 2026-06-15 by document-update sweep. `corpus_inventory.yaml` reports `version_anomalies: 0`. All documents now at `0.27.0` or documented as intentionally divergent.

<details>
<summary>Original anomaly list (resolved)</summary>

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
| | `plans/bundler-completion.md` — archived 2026-06-15
| | `plans/mcp-server-roadmap.md` (1.0.0) |
| `plans/mcp-media-server-design.md` — archived 2026-06-15
| `adversarial-simplification-inventory.md` — archived 2026-06-15

**Approach:** Left column — document in `corpus_inventory.yaml` that version is intentional. Right column — bump to `0.27.0`.

</details>

### P1-4: Create `docs/ci/sync-versions.sh` ✅ DONE

**Status:** Script exists and is functional. Supports `--dry-run` and `--new-version` flags. Exclusion list cleaned up (2026-06-15-sweep) — removed 4 archived entries, fixed 2 path errors. Dry-run confirms 0 files need updating (all at 0.27.0).

### P1-5: Update `MDS_SCAFFOLD.md` Document Structure ✅ DONE

**Status:** Fixed 2026-06-15-sweep. §3 updated — removed references to archived `spec-code-drift.yaml` and `curation-decisions.yaml`, consolidated into `corpus_inventory.yaml`. All referenced files verified to exist. §5 verification commands all functional.

---

## P2 — Medium (Next Release)

### P2-1: Generate `docs/generated/openapi.json` ✅ DONE

**Status:** File exists at `docs/generated/openapi.json` (3,855 lines). Generated from `hkask-api` utoipa annotations via `create_openapi()`. Full OpenAPI 3.1.0 spec with all route paths, schemas, and tags.

### P2-2: Missing ADRs ✅ DONE

**Status:** All three ADRs exist with proper YAML frontmatter and substantive content:
- **ADR-036** — OCR pipeline architecture (68 lines, created 2026-06-14)
- **ADR-037** — Wallet payment mechanism architecture (70 lines, created 2026-06-14)
- **ADR-038** — Media MCP server architecture (66 lines, created 2026-06-14)

### P2-3: Codify Archive Policy as Living Document ✅ DONE

**Status:** Archive policy already codified in `DOCUMENTATION_STANDARDS.md` §3 (Lifecycle) and `HANDOFF_LIFECYCLE.md`. Git history is archive of record; `docs/archive/` is gitignored personal reference. No standalone ARCHIVE_POLICY.md needed — policy is consolidated in existing authoritative documents.

### P2-4: Runbook / Operational Guide ✅ DONE

**Status:** `docs/guides/OPERATIONS_RUNBOOK.md` exists (created 2026-06-14). Covers all minimum viable items: deployment architecture (§1), prerequisites (§2), startup (§3), health checks (§4), key rotation (§5), troubleshooting (§6), log locations (§7), backup/recovery (§8), shutdown (§9). Status: Draft.

### P2-5: Migrate `docs/audit/` to Archive ✅ DONE

**Status:** Archived 2026-06-14. All 5 audit documents moved to `docs/archive/2026-06-14-crate-audit/`. Failed essentialist Gate 1 (Exist). `docs/audit/` directory is now empty.

### P2-6: Standardize Plan Document Versioning ✅ DONE

**Status:** All 5 plan documents now at `0.27.0` with YAML frontmatter. Added missing frontmatter to `pragmatic-audit-implementation-plan-v0.27.0.md` and `test-harness-maturation-plan-v0.27.0.md`. `TODO.md` tracks its own version (excluded in sync-versions.sh).

---

## P3 — Low (Backlog)

### P3-1: Crate-Specific Onboarding Guides — Rejected

**Rationale:** "Crate onboarding" is a category error — crates don't onboard, replicants and users do. The underlying need (developer reference docs for complex crates) is real but not well-scoped as a P3 item. Defer to per-crate workstreams when specific crate documentation gaps are identified.

### P3-2: Replicant Onboarding Walkthrough ✅ DONE

**Status:** Guide created at `docs/user-guides/REPLICANT-ONBOARDING-WALKTHROUGH.md` (239 lines). Covers: prerequisites, build/install, onboarding flow, verification, first chat session, next steps (consent, MCP servers, additional replicants), troubleshooting, reference.

### P3-3: Essentialist Auto-Culling — Rejected (marginal)

**Rationale:** Script created then deleted per essentialist review. Signal-to-noise too poor: 21 "unreferenced" but 12 are intentional PUBLIC_SURFACE stubs. Only ~3 genuinely actionable findings. Not worth maintaining a dedicated script for.

### P3-4: Corpus Inventory Regeneration Automation — Rejected (marginal)

**Rationale:** Script created then deleted per essentialist review. Solves only the mechanical 20% (path/category/status extraction from frontmatter). The hard 80% (staleness_signal, governing_principles, disposition, notes) remains TODO. Mechanical extraction is trivial grep work — doesn't earn a dedicated script.

### P3-5: Pre-Commit Hook for Version Anomalies — Rejected (marginal)

**Rationale:** Script created then deleted per essentialist review. `sync-versions.sh` already handles version synchronization. A pre-commit hook adds friction to every commit for a check that would fire on ~0% of commits (version changes only on release). Near-zero benefit, non-zero cost.

---

## Quick Wins (Under 30 Minutes)

| # | Task | Effort |
|---|------|--------|
| QW-1 | Fix `skill-inventory.md` mds_categories: s/status/curation/ | 1 min | Obsolete (archived 2026-06-15) |
| QW-2 | Add frontmatter to 5 audit documents (all same template) | 15 min | ✅ Done (P1-1) — docs archived 2026-06-14 |
| QW-3 | Fix `corpus_inventory.yaml` missing_referenced: remove check-links.sh + check-metadata.sh | 5 min | ✅ Done (document-update sweep 2026-06-15) |
| QW-4 | Bump 14 document versions from various → 0.27.0 | 15 min | ✅ Done (document-update sweep 2026-06-15) |
| QW-5 | Create `spec-code-drift.yaml` stub with section headers | 15 min | Obsolete (archived 2026-06-15 per README portal) |
| QW-6 | Create `curation-decisions.yaml` stub with section headers | 15 min | Obsolete (archived 2026-06-15 per README portal) |

---

## Dependency Graph

```
P1-3 (fix version anomalies) ✅ DONE
  ↓
P1-4 (create sync-versions.sh) ✅ DONE

P1-5 (update MDS_SCAFFOLD.md) ✅ DONE
  ↓
Re-run check-metadata.sh → 0 errors

All P0/P1 items resolved. Remaining: P0-1 (verify), P2, P3.
```

---

## Verification Gates

After each tier is complete:

| Tier | Gate |
|------|------|
| P0 | `check-links.sh` passes; README portal has no "not yet created" references; P0-1 (CI scripts in missing_referenced) resolved ✅ |
| P1 | `check-metadata.sh` passes with 0 errors; all version anomalies resolved ✅; sync-versions.sh functional ✅; MDS_SCAFFOLD.md updated ✅ |
| P2 | `openapi.json` generated ✅; missing ADRs exist ✅; archive policy codified ✅; runbook exists ✅; plan versioning standardized ✅ |
| P3 | Backlog grooming complete; onboarding walkthrough created ✅; P3-1 rejected (category error); P3-3/4/5 rejected (marginal — scripts created then deleted per essentialist review) |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
