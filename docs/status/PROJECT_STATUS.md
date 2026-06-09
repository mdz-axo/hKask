---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-06-08
version: "0.1.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [observability, lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

## Build

| Target | Command | Result | Date |
|--------|---------|--------|------|
| Workspace | `cargo check --workspace` | ✅ **Pass** (0 errors) | 2026-06-08 |
| `hkask-types` | `cargo check -p hkask-types` | ✅ Pass | 2026-06-08 |
| `hkask-templates` | `cargo check -p hkask-templates` | ✅ Pass | 2026-06-08 |
| `hkask-cns` | `cargo check -p hkask-cns` | ✅ Pass | 2026-06-08 |
| `hkask-mcp-condenser` | `cargo check -p hkask-mcp-condenser` | ✅ Pass | 2026-06-08 |

**Note (2026-06-08):** Restored `GasError` enum to `hkask-cns/src/energy.rs` (stripped in commit `a4cb05de` "cleaning") and `ContextCategory`, `CondenserStats`, `CompressedOutput`, `ThreadSummaryRequest`, `ThreadSummaryOutput` to `hkask-mcp-condenser/src/types.rs` (stripped in commit `ad961c14` "ouch").

## Test

| Target | Command | Result | Date |
|--------|---------|--------|------|
| `hkask-types` | `cargo test -p hkask-types` | ✅ **Pass** (0 tests) | 2026-06-08 |
| `hkask-templates` | `cargo test -p hkask-templates` | ✅ **Pass** (1 doc-test) | 2026-06-08 |
| Full workspace | `cargo test --workspace` | ⚠️ Not run this session | 2026-06-08 |

## Clippy (Lint)

| Target | Command | Result | Date |
|--------|---------|--------|------|
| Workspace | `cargo clippy --workspace -- -D warnings` | ✅ **Pass** (0 warnings) | 2026-06-08 |

## Documentation CI

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Link checker | `docs/ci/check-links.sh` | ✅ **Pass** (259 links, 0 broken) | 2026-06-08 |
| Metadata checker | `docs/ci/check-metadata.sh` | ⚠️ 2 pre-existing issues (not from this session) | 2026-06-08 |

## Code Drift

See [`docs/status/spec-code-drift.yaml`](spec-code-drift.yaml) for the exhaustive 14-item drift set and [`docs/status/curation-decisions.yaml`](curation-decisions.yaml) for curation decisions.

| Classification | Count |
|---|---|
| spec_ahead | 5 |
| code_ahead | 2 |
| divergent | 5 |
| duplicate | 2 |

**All P2-06 drift items (D1–D9) resolved.** 4 additional drift items (DRIFT-001–004) also resolved.

## TODO Completion (This Session)

| ID | Task | Status |
|----|------|--------|
| P2-09 | TemplateType vocabulary mapping (Prompt↔WordAct, Process↔FlowDef, Cognition↔KnowAct) in `interface-and-composition.md` | ✅ Done |
| P2-10 | DDMVSS §11 R3 deferred items (all 10) added to `OPEN_QUESTIONS.md` | ✅ Done |
| P2-11 | `PROJECT_STATUS.md` populated with build/test/clippy results | ✅ Done |
| **#4** | Wire `ContractValidator::validate_terms()` into template registration path | ✅ Done |
| **#5** | Implement `parse_markdown_catalog` / `render_workspace_yaml` / `regenerate_workspace_yaml` | ✅ Done |

## Remaining (from HANDOFF.md)

| # | Task | Priority |
|---|------|----------|
| 6 | Add bitemporal query methods to `SqliteSpecStore` (FUT-011) | Medium |
| 7 | Calibrate coherence threshold (FUT-013) | Medium |
| 8 | Populate Fowler audit and adversarial simplification inventories (P2-14/P2-15) | Low |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
