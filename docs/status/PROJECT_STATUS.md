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

**Current session:** Onboarding overhaul + P8 test gap closure (2026-06-11).

---

## Build

All 18 workspace members. `hkask-cli` and `hkask-services` have pre-existing errors **in tests only** (not from this session).

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo check --workspace`) | ✅ Pass | 2026-06-11 |
| Core crates (types, storage, memory, cns, templates, agents, keystore, mcp, services, cli, api) | ✅ Pass | 2026-06-11 |
| MCP servers (condenser, research, spec, fmp, communication, fal) | ✅ Pass | 2026-06-11 |
| `hkask-cli` (production) | ✅ Pass | 2026-06-11 |
| `hkask-cli` (tests) | ✅ Pass — 25 tests | 2026-06-11 |
| `hkask-services` (production) | ✅ Pass | 2026-06-11 |
| `hkask-services` (tests) | ✅ Pass — 29 tests | 2026-06-11 |

---

## Test

`cargo test --workspace` result: ✅ Pass — 246 tests, 0 failures (7 doc-tests ignored in hkask-storage).

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

## This Session (2026-06-11)

- Onboarding overhaul: model selection, passphrase strength UX, First Steps guide, `is_first_run` flag
- New `kask onboard` CLI subcommand for adding replicants to existing installations
- New `/start` guided tour (9 steps) and `/feedback` REPL-only ledger command
- 3 code bugs fixed: `run_add_replicant` dangerous fallback, `/start` cursor reset, stale comment
- 6 P8 tests added: `append_feedback` (3) + `passphrase_strength` (3) — total: 19→25
- Docs updated: AGENTS.md, cli-reference.md, REPL-specification.md, test-inventory.md
- Pre-existing build errors in `hkask-cli` and `hkask-services` tests confirmed resolved (prior session)

## Session (2026-06-11)

- Handoff continuation: verified build (246 tests, 0 failures), fixed 3 unicode escape errors in `hkask-services/src/bundle.rs` (Rust 2024 `\u{XXXX}` format)
- HIGH #1: Transient AgentService accessor errors — confirmed resolved (no old accessor names in codebase, build clean)
- HIGH #2: Architecture master sovereignty claim — updated AgentService section to current named-accessor pattern, noted sovereignty distribution across `hkask-types`/`hkask-agents`/`hkask-services`
- LOW #3: Architecture master allosteric/RBarThreshold update — confirmed already resolved (no references in arch master; remaining occurrences are historical docs, GML templates, or deletion-acknowledging code comments)
- LOW #4: Citation compliance audit (PS-07) — completed; 23 files with footnote citation gaps identified and catalogued
- MEDIUM: AgentService adapters refactoring — completed; 5 stale comments in `hkask-api/src/routes/` (acp.rs, mcp.rs, templates.rs) updated from old grouped-tuple references to current named accessors. Zero old accessor patterns remain in codebase.
- PROJECT_STATUS.md updated: What Remains pruned, citation audit table added

## Session (2026-06-10)

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
| LOW | Citation compliance: 23 files have fewer footnote citations than `##` sections (PS-07 gap). Audit complete 2026-06-11 — see §Citation Audit below. |
| NOT YET DONE | End-to-end onboarding smoke test (needs live Okapi) |

### Citation Audit (2026-06-11)

PS-07 ("Sourced Ideas") requires every `##` section to have at least one `[^...]` footnote citation. Audit found 23 files with gaps:

| Gap | Files |
|-----|-------|
| 3 | `TESTING_STANDARDS.md` |
| 4 | `ADR-024`, `ADR-026`, `MDS.md` |
| 5 | `ADR-031`, `ADR-032`, `ADR-033`, `ADR-034` |
| 6 | `AGENTSERVICE-IMPLEMENTATION.md`, `MDS_SCAFFOLD.md` |
| 7 | `hKask-architecture-master.md`, `ADR_TEMPLATE.md`, `MDS-agent-service.md` |
| 8 | `refactoring-plan-services-2026-06-09.md` |
| 9 | `agatha-eliot-moe-plan.md`, `semantic-condensation-analysis.md` |
| 10 | `REQUIREMENTS.md`, `TRACEABILITY_MATRIX.md` |
| 11 | `CI-CD-GUIDE.md` |
| 12 | `test-program.md` |
| 13 | `DEPLOYMENT.md` |
| 23 | `REPL-specification.md` |

Fixing these requires domain knowledge to assign appropriate external citations per section — not mechanically resolvable.

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
