---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-06-13
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

**Current session:** Matrix integration architecture, implementation, and documentation (2026-06-14).

---

## Build

All 18 workspace members.

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo check --workspace`) | ✅ Pass | 2026-06-14 |
| Core crates (types, storage, memory, cns, templates, agents, keystore, mcp, services, cli, api) | ✅ Pass | 2026-06-14 |
| MCP servers (condenser, research, spec, companies, communication, media, replica, docproc, training, memory) | ✅ Pass | 2026-06-14 |
| `hkask-cli` (production) | ✅ Pass | 2026-06-14 |
| `hkask-cli` (tests) | ✅ Pass — 25 tests | 2026-06-11 |
| `hkask-services` (production) | ✅ Pass | 2026-06-14 |
| `hkask-services` (tests) | ✅ Pass — 29 tests | 2026-06-11 |
| `hkask-api` (production) | ⚠️ 6 pre-existing errors (missing `From` trait impls for error types — unrelated to Matrix) | 2026-06-14 |

---

## Test

`cargo test --workspace` result: ✅ Pass — 246 tests, 0 failures (7 doc-tests ignored in hkask-storage).

---

## Clippy (Lint)

| Target | Result | Date |
|--------|--------|------|
| Workspace (`-D warnings`) | ✅ Pass (1 pre-existing warning: hkask-mcp-markitdown) | 2026-06-13 |

---

## Constraint Verification

| Check | Result | Date |
|-------|--------|------|
| `todo!()`, `unimplemented!()`, `#[deprecated]` | 0 violations | 2026-06-13 |
| Dead code (`#[allow(dead_code)]`) | 1 site: compile-time assertion in `acp/mod.rs:171` | 2026-06-10 |
| Headless constraint (no grafana/prometheus/dashboard/UI) | ✅ Clean | 2026-06-13 |

---

## Codebase Metrics

| Metric | Value |
|--------|-------|
| Source files (crates) | 252 |
| Source files (MCP servers) | 40 |
| Source files (total) | 292 |
| Workspace members | 18 |
| Active docs | 72 |
| Archived docs | 10 (2026-06-13: ADR-022, condensed-erd, high-temp-templates, 7 date-stamped archives) |
| Skills | 28 (4 kata: starter, improvement, coaching, bundle) |
| MCP servers | 10 |

---

## Documentation CI

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Link checker | `docs/ci/check-links.sh` | ✅ Pass (266 links, 0 broken) | 2026-06-14 |
| Metadata checker | `docs/ci/check-metadata.sh` | ✅ Pass (73 docs, 0 missing, 0 warnings) | 2026-06-14 |
| Version sync | `docs/ci/sync-versions.sh --dry-run` | ✅ Pass (0 pending updates, 11 excluded) | 2026-06-14 |

**All CI gates pass.** The previous `ddmvss_categories` check was migrated to `mds_categories` (5-category MDS taxonomy). No documents use the deprecated 9-category taxonomy.

---

## Code Drift

See [`docs/status/spec-code-drift.yaml`](spec-code-drift.yaml) and [`docs/status/curation-decisions.yaml`](curation-decisions.yaml).

**All 14 drift items resolved (2026-06-12).** Zero remaining spec_ahead, code_ahead, or divergent items.

---

## Sovereignty

| Check | Result |
|-------|--------|
| Magna Carta P1 (User Sovereignty) | Sovereignty distributed across `hkask-types::sovereignty`, `hkask-agents::sovereignty`, `hkask-services::verification`. No single SovereigntyService — this is correct, not a gap. |
| Magna Carta P2 (Affirmative Consent) | CNS consent denial events emitted. Prohibition gate — denial is terminal. |
| Magna Carta P3 (Generative Space) | 10 MCP servers + Okapi inference. No feature flags, no gated surfaces. |
| Magna Carta P4 (Clear Boundaries) | OCAP capability membrane. 1/10 MCP servers (`hkask-mcp-spec`) currently enforce via `GovernedTool` (see ADR-032). |

---

## This Session (2026-06-14)

**Matrix Integration — Architecture, Specification, and Implementation:**

- Architecture research report: `docs/architecture/matrix-integration-architecture.md` (~1,800 lines). Full deployment model, client orchestration, identity binding, agent interaction patterns, essentialist/grill-me/pragmatic-semantics/pragmatic-cybernetics reviews, gap analysis, verification spec, CNS span specification.
- Spec resolved 4 Blocking gaps (B1–B4: Caddy TLS automation, MXID format, `.well-known` delegation, Conduit config defaults), 6 Important gaps (I1–I6: recovery keys, device names, message format, room encryption, error taxonomy, gas accounting), 4 Prohibitions (P1–P4), 10 Guardrails (G1–G10).
- Implementation: `matrix.rs` — 303 lines of stubs replaced with ~380 lines of real `MatrixTransport` using `matrix-sdk` 0.16. Login, send_message, get_messages (on-demand polling), create_room, invite_user, list_rooms. CNS tracing on all operations.
- CLI: `kask matrix deploy-sidecar` (generates Caddy + Conduit + Hydrogen docker-compose), `kask matrix register --agent` (credential prompt, MXID derivation, Conduit admin API), `kask matrix register --user` (human account creation), `kask matrix status-sidecar` (Docker health check).
- `TurnRequest.source` field: `MessageSource` enum (Matrix, Daemon, Cli, Api) for P12 compliance.
- Overengineering removed: continuous sync loop, message inbox, `register_user` on MatrixTransport, `Encryption` error variant, `MatrixAction::Listen` CLI command, `AgentRegistry::register` (Matrix SDK registration). Net reduction: ~180 lines removed.
- All callers migrated: `main.rs`, `agent_registration.rs`, `moderation.rs`. Type renames: `RoomIdStr`→`RoomId`, `UserIdStr`→`UserId`, `MatrixClient`→`MatrixTransport`. `ConduitSidecar` and `EmbeddedHomeserver` deleted.
- E2EE deferred to v2 (SQLCipher/SQLite linking conflict between hkask-storage and matrix-sdk-sqlite). Continuous sync deferred until VOIP/real-time use case exists.
- Workspace build: ✅ Pass (all 18 members). `hkask-api` has 6 pre-existing errors (missing `From` trait impls — unrelated).

## Session (2026-06-13)

- Registry reorganization: deleted `registry/registries/` (26 misfiled YAMLs moved to correct locations), deleted `registry/corpora/` (moved to `registry/styles/gentle-lovelace/corpus-sources/`), deleted `registry/kata/` (replaced by 4-skill kata architecture)
- Root cleanup: 6 DB files → `data/`, 2 scripts → `scripts/`, `feedback.md` → `docs/`, `david-dunning/` → `registry/styles/david-dunning/`
- `DEFAULT_DB_PATH` changed from `"hkask.db"` to `"data/hkask.db"` in `config.rs`
- Kata system: deep research on Mike Rother's Toyota Kata methodology, full refactor from 1 skill with 3 artificial types → 4 skills (kata-starter, kata-improvement, kata-coaching, kata bundle) with 23 templates split across 4 directories, 5 manifests, 26 bootstrap entries
- 2 pre-existing bugs fixed: `identity.rs` missing `passphrase_set_at`, `markitdown/tools.rs` broken `CnsObserver` impl
- Docs created: `docs/guides/kata-user-guide.md` (361 lines), `docs/status/skill-inventory.md` (117 lines)
- Docs updated: 4 frontmatter dates, `docs/README.md` portal, `hKask-architecture-master.md`, `DIAGRAMS_INDEX.md`, `PROJECT_STATUS.md`, `kata-hlexicon.yaml` rewritten
- 18 files updated with corrected path references across YAML, Rust, markdown, and shell scripts

## Session (2026-06-11)

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

### Communication Server — Remaining Items

| Priority | Task | Status |
|----------|------|--------|
| MEDIUM | `kask matrix register --agent` credential verification against stored keystore hash | TODO — currently accepts any credential with format warning |
| MEDIUM | SAS QR code generation for device verification | Deferred to v2 (requires matrix-sdk-crypto, blocked by SQLCipher/SQLite conflict) |
| LOW | Daemon periodic sidecar health task (every 60s: poll containers, emit CNS spans) | Deferred — `kask matrix status-sidecar` provides on-demand checks |
| LOW | CNS span formal registration in CNS registry | Deferred — spans emit via tracing, functional but not registered |
| LOW | `kask matrix listen` (continuous sync for VOIP/real-time) | Deferred until use case exists |
| v2 | E2EE integration (Olm/Megolm, CryptoStore against hkask-keystore) | Blocked by SQLCipher/SQLite linking conflict |
| v2 | Cross-installation agent-to-agent communication via federation | Requires E2EE + continuous sync |

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
