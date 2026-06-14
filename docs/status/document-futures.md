# hKask — Open Questions & Under-Specification (Document Futures)

**Created:** 2026-06-14  
**Purpose:** Records open questions surfaced during the 2026-06-14 document corpus hygiene sweep.  
**Governing Principles:** P3 (Generative Space), P5 (Pared Surface), P6 (No Dead Docs)  

---

## 1. Anticipated but Not Yet Written

| Document | Purpose | Priority | Notes |
|----------|---------|----------|-------|
| `docs/ci/check-links.sh` | Link integrity checker — verifies all hyperlinks between documents are live | High | Referenced by README, DOCUMENTATION_STANDARDS.md, MDS_SCAFFOLD.md |
| `docs/ci/check-metadata.sh` | Metadata compliance checker — enforces mandatory YAML frontmatter on all active documents | High | Referenced by README, DOCUMENTATION_STANDARDS.md, MDS_SCAFFOLD.md |
| `docs/status/spec-code-drift.yaml` | Spec-code drift tracking — set-difference of named entities from spec docs against `pub` API surfaces | High | Referenced by README, MDS_SCAFFOLD, and architecture master. Core infrastructure for P8 (Semantic Grounding) enforcement. |
| `docs/status/curation-decisions.yaml` | Curation decisions per drift item — records Merge/Revise decisions per the MDS curation protocol | High | Referenced by README and MDS_SCAFFOLD. Needed for spec-code alignment workflow. |
| `docs/generated/openapi.json` | OpenAPI specification — auto-generated from utoipa annotations | Medium | Referenced by MDS_SCAFFOLD. Should be generated from `cargo doc` or utoipa build step. |
| Crate-specific onboarding guides | Onboarding guide for each of the 18 crates (especially new ones: hkask-mcp-media, hkask-mcp-companies) | Low | User-facing growth. Currently only 3 user guides exist (agent pod creation, common patterns, companies). |
| Architecture decision records for missing topics | OCR pipeline ADR, wallet payments ADR, media server ADR | Medium | Essential architectural knowledge currently encoded only in handoffs and code |

## 2. Tools & Automation Opportunities

| Need | Approach | Priority |
|------|----------|----------|
| CI hook for spec-code coherence checking | Hook into `cargo test` or `cargo clippy` to run drift detection. Could be a `#[test]` that compares `pub` API symbols against spec documents. | High |
| Automated frontmatter validation | `check-metadata.sh` could be a simple grep-based script that verifies every `.md` in active directories has YAML frontmatter with `title`, `audience`, `status`, `last_updated`, `version`, `mds_categories`. | High |
| Automated link checking | `check-links.sh` could parse markdown links and verify target files exist. Could use `lychee` or similar tool. | High |
| Version synchronization | Script to bump `version` in all document frontmatter when `Cargo.toml` workspace version changes. | Medium |
| Corpus inventory regeneration | Re-run inventory classification after major structural changes (new crates, renames, removals). | Medium |
| Essentialist auto-culling | Automated deletion test for documents — check if document is referenced from any index (portal, master, AGENTS.md). | Low |

## 3. Document Categories Lacking Clear Ownership

| Category | Current State | Recommendation |
|----------|--------------|----------------|
| `docs/handoffs/` | Transient session handoffs. Accumulate rapidly (10 files in ~2 weeks). No clear lifecycle: when do they become stale? When are they superseded? | Define a policy: handoffs older than 30 days without successor are archived. Handoffs superseded by newer handoffs are deleted. |
| `docs/audit/` | 5-task audit bundle. Completed work, no forward-looking value beyond historical record. | Consider archiving or moving to `docs/archive/` after implementation is complete. |
| `docs/plans/` | Mix of plans (TODO, roadmap) and handoffs (date-prefixed). Version numbers are inconsistent (0.1.0, 1.0.0, 1.9.0 vs project 0.27.0). | Standardize: plans should track project version (0.27.0), not their own numbering. |
| `docs/status/` inventory files | test-inventory.md, mcp-tools-inventory.md, corpus_inventory.yaml are auto-generated. skill-inventory.md is maintained manually. | Define which are auto-generated vs. maintained. Auto-generated files should have metadata noting their generation source. |

## 4. Document Types Without a Home

| Document Type | Current Location | Issue |
|--------------|-----------------|-------|
| Specification documents for new MCP servers | Mixed: some in `specifications/`, some inline in `plans/`, some only in handoffs | Should consolidate to `specifications/` with consistent naming and metadata |
| Runbooks / operational guides | None exist | Missing entirely. hKask deployments (especially cloud server with daemon) lack operational documentation |
| Replicant onboarding guide | None exist beyond `kask onboard` command docs | `user-guides/` has pod creation and agent patterns, but no "new replicant onboarding from scratch" walkthrough |
| API reference | `generated/cli-reference.md` (auto-generated) | Would benefit from OpenAPI spec generation (`generated/openapi.json`) |

## 5. Archive Policy Considerations

| Question | Status |
|----------|--------|
| Should archive policy be codified as a living document? | Yes — `docs/archive/MANIFEST.md` was created as the initial archive manifest during this sweep. This should be maintained as documents are archived. |
| Should archive policy itself be governed by its own principles? | Yes — P5 (Pared Surface: archive only what's truly superseded) and P6 (No Dead Docs: active tree must stay clean) are the governing principles. Archive policy should reference these. |
| When should handoffs be transitioned to archived? | Policy needed: after successor handoff supersedes them, or after 30 days without activity. |
| Should version anomalies be automatically flagged? | Yes — a CI script or pre-commit hook could flag documents where `version` != workspace `Cargo.toml` version. |

## 6. Resolved During This Sweep (for completeness)

- ✅ 5 documents archived (see `docs/archive/MANIFEST.md`)
- ✅ Architecture master updated: 21 → 25 framework documents
- ✅ README portal updated: architecture, plans, handoffs sections added
- ✅ corpus_inventory.yaml generated
- ✅ MDS category tags corrected (interface→composition, status→curation)
- ✅ Deleted documents verified not referenced from portal (no cross-reference cleanup needed)

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
