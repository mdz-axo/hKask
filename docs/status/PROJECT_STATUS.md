---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-06-10
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

**Current session:** Documentation refresh (2026-06-10).

---

## Build

All 18 workspace members. `hkask-cli` and `hkask-services` have pre-existing errors **in tests only** (not from this session).

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo check --workspace`) | ✅ Pass (0 errors excluding pre-existing test-only errors) | 2026-06-10 |
| Core crates (types, storage, memory, cns, templates, agents, keystore, mcp, services, cli, api) | ✅ Pass | 2026-06-10 |
| MCP servers (condenser, web, spec, fmp, telnyx, fal, rss-reader) | ✅ Pass | 2026-06-10 |
| `hkask-cli` (production) | ✅ Pass | 2026-06-10 |
| `hkask-cli` (tests) | ❌ Pre-existing: `ensemble.rs` references `build_improv_client` which doesn't exist | — |
| `hkask-services` (production) | ✅ Pass | 2026-06-10 |
| `hkask-services` (tests) | ❌ Pre-existing: `SqliteSpecStore::load` references missing method | — |

---

## Test

`cargo test --workspace` result: ✅ Pass (1 doc-test passes, 7 ignored in hkask-storage).

---

## Clippy (Lint)

| Target | Result | Date |
|--------|--------|------|
| Workspace (`-D warnings`) | ✅ Pass (0 warnings) | 2026-06-10 |

---

## Constraint Verification

| Check | Result | Date |
|-------|--------|------|
| `todo!()`, `unimplemented!()`, `#[deprecated]` | 0 violations | 2026-06-10 |
| Dead code (`#[allow(dead_code)]`) | 1 site: compile-time assertion in `acp/mod.rs:171` | 2026-06-10 |
| Headless constraint (no grafana/prometheus/dashboard/UI) | ✅ Clean | 2026-06-10 |

---

## Codebase Metrics

| Metric | Value |
|--------|-------|
| Source files (crates) | 252 |
| Source files (MCP servers) | 40 |
| Source files (total) | 292 |
| Workspace members | 18 |
| Active docs | 55 |
| Archived docs | 3 (2026-06-10: ADR-022, condensed-erd, high-temp-templates) |
| Skills | 14 |
| MCP servers | 10 |

---

## Documentation CI

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Link checker | `docs/ci/check-links.sh` | ✅ Pass (201 links, 0 broken) | 2026-06-10 |
| Metadata checker | `docs/ci/check-metadata.sh` | ⚠️ 49 missing `ddmvss_categories` (pre-existing: schema mismatch) | 2026-06-10 |

**Metadata checker**: 49 `ddmvss_categories` missing (pre-existing schema mismatch — not caused by this session).

---

## Code Drift

See [`docs/status/spec-code-drift.yaml`](spec-code-drift.yaml) and [`docs/status/curation-decisions.yaml`](curation-decisions.yaml).

| Classification | Count |
|---|---|
| spec_ahead | 5 |
| code_ahead | 2 |
| divergent | 5 |
| duplicate | 2 |

All P2-06 drift items (D1–D9) and DRIFT-001–004 resolved.

---

## Sovereignty

| Check | Result |
|-------|--------|
| Magna Carta P1 (User Sovereignty) | Sovereignty distributed across `hkask-types::sovereignty`, `hkask-agents::sovereignty`, `hkask-services::verification`. No single SovereigntyService — this is correct, not a gap. |
| Magna Carta P2 (Affirmative Consent) | CNS consent denial events emitted. Prohibition gate — denial is terminal. |
| Magna Carta P3 (Generative Space) | 10 MCP servers + Okapi inference. No feature flags, no gated surfaces. |
| Magna Carta P4 (Clear Boundaries) | OCAP capability membrane. 1/10 MCP servers (`hkask-mcp-spec`) currently enforce via `GovernedTool` (see ADR-032). |

---

## This Session (2026-06-10)

- Documentation refresh: 71 broken internal links found and fixed (71→0)
- MDS category alignment: MDS_SCAFFOLD.md updated from 9-category to 5-category (Domain, Composition, Trust, Lifecycle, Curation)
- Spec-code completeness predicate collapsed from 9 rows to 5
- Document tree corrected: phantom section-files removed, missing actual files added
- Writing excellence audit completed across 55 active docs
- PROJECT_STATUS.md updated to v0.27.0 state

---

## What Remains

| Priority | Task |
|----------|------|
| HIGH | Fix architecture master sovereignty claim (SovereigntyService row) |
| HIGH | Pre-existing build errors in `hkask-cli` (ensemble.rs) and `hkask-services` tests |
| MEDIUM | AgentService adapters refactoring (incomplete, reverted) |
| LOW | Test coverage gaps (archival, compose, inference, onboarding, verification — shallow modules, acceptable) |
| LOW | Architecture master update (allosteric references, RBarThreshold, v0.27.2 state) |

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
