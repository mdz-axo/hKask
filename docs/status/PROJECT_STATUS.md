---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-07-20
last-verified-against: "8d3cf671"
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

**Current session:** v0.31.0 — Ratatui architecture hardening. Request-owned inference routing, MCP completion delivery, UTF-8-safe input, fallible PTY startup, enforced window lifecycle policy, single-pass fallback bootstrap, and explicit Backup/Wallet/pod unavailability are implemented. The synchronous timed-cache experiment was reverted because `tick` shares the event-loop thread. `hkask-tui`: 127 tests pass (32 unit + 95 integration); strict clippy passes for TUI and REPL-with-TUI.

**This session (2026-07-20):**
- Added the source-aligned [Terminal UI Architecture](../explanation/tui-architecture.md) explanation and `DIAG-TUI-005` sequence diagram.
- Implemented request-owned inference and scoped MCP completion delivery without a global event bus.
- Added UTF-8-safe cursor operations shared by Chat, Curator, Terminal, and Editor.
- Made PTY startup fallible, enforced singleton/closeability metadata, and reused initialized state during line-REPL fallback.
- Reverted timed Kanban/Media cache refreshes after determining that `tick` runs on the same event-loop thread and therefore does not provide asynchronous isolation.
- Replaced fabricated Backup, Wallet, and pod telemetry with explicit unavailable/ready/failed semantics.
- Corrected active TUI launch, keybinding, persistence, bridge-count, and dependency documentation.
- `cargo test -p hkask-tui`: 127 passed; strict clippy passed for `hkask-tui` and `hkask-repl --features tui`.

**Previous session (2026-07-17):**
- Documentation consolidation (diataxis-diagram + grill-me + kata-improvement skills).
- Root `README.md` rewritten with actual codebase counts: 54 crates, 46 PDCA skills (49 capabilities total: 46 skills + 2 templates + 1 bundle), 83 manifests, 367 templates, ~2,166 tests, 37 CLI subcommands.
- Stale root `OPEN_QUESTIONS.md` removed (self-declared superseded by `docs/OPEN_QUESTIONS.md`; per DOCUMENTATION_STANDARDS.md §3 lifecycle policy).
- Fixed 80 broken intra-doc hyperlinks caused by `hKask-architecture-master.md` move from `docs/architecture/` to `docs/architecture/core/`.
- `docs/README.md` portal: fixed phantom `tutorial/getting-started.md` path → `how-to/getting-started.md`.
- `docs/status/PROJECT_STATUS.md` metrics updated: 59→69 workspace members, 44→54 crates, 39→46 PDCA skills, 72→83 manifests, 294→367 templates, LOC counts refreshed.
- Build: clean (0 warnings). Docs: clean (0 errors, 6 advisory warnings — all forward-looking PLANNED references in plans/status docs).

**Previous session (2026-07-01):**
- Gas rename: Energy* types → Gas* across 25 files. Curation concepts preserved.
- Budget persistence: JSON save/load with Well state, async I/O, version envelope.
- Escalation: exhaustion alerts via algedonic pathway (CurationInput::Alert).
- Stale reservation auto-release: 5-minute timeout prevents hold-settle leaks.
- Consumption velocity: per-agent gas burned tracked across ticks.
- Well system: WellManager with auto-create, replenish, exhaustion alert dampening.
- Wallet system: SQLite-backed WalletStore + WalletManager, integrated into spend path.
- Auto-draw: synchronous Well→Wallet draw on low balance during spend.
- 10 new Regulation spans for Well/Wallet/Curator lifecycle.
- Build: clean. Tests: 109/109 pass.

**Note:** `hkask-cli` build is clean (pre-existing compile errors resolved).

**Previous (2026-06-25):** v0.31.0 — Training provider architecture refactor + MCP media tool split.

---

## Build

All 69 workspace members (54 crates + 15 MCP servers, excluding fuzz targets).

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo build --workspace`) | ✅ Pass (0 errors, 0 warnings) | 2026-07-17 |
| Workspace (54 crates + 15 MCP servers) | ✅ Pass | 2026-07-17 |
| Warnings | 0 | 2026-07-17 |

---

## Test

`cargo test --workspace` result: ✅ Pass — ~1,500 tests across workspace, 0 failures. Contracts use `expect:` + `[P{N}]` annotations with Regulation span observation for runtime enforcement. (Test count updated from 1,460 to reflect v0.31.0 additions.)

### Test Distribution

| Crate | Tests |
|-------|-------|
| hkask-types | 85 |
| hkask-inference | 23 |
| hkask-storage | 59 |
| hkask-memory | 16 |
| hkask-regulation | 42 |
| hkask-pods | 31 |
| hkask-keystore | 13 |
| hkask-services-core | 19 |
| hkask-services-chat | 9 |
| hkask-services-compose | — |
| hkask-services-context | — |
| hkask-services-corpus | 16 |

| hkask-services-kata-kanban | 39 |
| hkask-services-onboarding | — |
| hkask-services-runtime | 7 |
| hkask-services-skill | 2 |
| hkask-services-wallet | 6 |
| _[Note: as of v0.31.0, the old monolithic service crate decomposed into 11 subcrates above]_ | |
| hkask-templates | 22 |
| hkask-condenser | 34 |
| hkask-wallet | 13 |
| hkask-communication | 25 |
| hkask-mcp | 38 |
| hkask-cli | 43 |
| hkask-tui | 127 |
| hkask-api | 12 |
| hkask-acp | 4 |
| hkask-adapter | 51 |
| hkask-codegraph | 22 |
| MCP servers (16) | ~770 |
| **Workspace total** | **~1,460** |

---

## Clippy (Lint)

| Target | Result | Date |
|--------|--------|------|
| Workspace (`-D warnings`) | ✅ Pass — 0 warnings | 2026-07-01 |

---

## Constraint Verification

| Check | Result | Date |
|-------|--------|------|
| `todo!()`, `unimplemented!()`, `#[deprecated]` | 0 violations | 2026-07-01 |
| Multi-user access control | ✅ Implemented: Role enum, admin middleware, invite CRUD, Regulation spans | 2026-06-18 |
| OAuth providers | ✅ GitHub + Google (Google OAuth implemented 2026-06-18) | 2026-06-18 |
| Contract annotations | ✅ Regulation 100%, Wallet 100%, Memory 100% — `expect:` + `[P{N}]` format | 2026-06-21 |
| Unsafe blocks | ✅ All documented with SAFETY: comments | 2026-07-01 |
| Rc<RefCell> patterns | ✅ Zero across all crates | 2026-07-01 |

---

## Codebase Metrics

| Metric | Value |
|--------|-------|
| Source files (total) | ~780 |
| **Core LOC (crates/src/)** | ~182,500 |
| **MCP Server LOC (src/)** | ~49,600 |
| **Total LOC** | ~232,100 |
| Workspace members | 69 (54 crates + 15 MCP servers, excluding fuzz targets) |
| Core crates | 54 (14 foundation + 16 infra + 14 services + 3 wallet/identity/ledger + 2 ontology/interface + 5 bridges) |
| Skills | 46 PDCA (49 capabilities total: 46 skills + 2 templates + 1 bundle; 83 registry manifests, 367 Jinja2 templates) |
| MCP servers | 15 |
| Tests | ~2,166 (`#[test]` + `#[tokio::test]` annotations) |
| CLI subcommands | 37 |
| API route groups | 21 |
| ACP userpod | 1 (`hkask-acp`) — IDE agent presence via Agent Client Protocol |
| Regulation spans | 100+ |

---

## CI Quality Gates

Verification gates in `.github/workflows/ci.yml` on every PR and push to main:

| Check | Method | Result | Date |
|-------|--------|--------|------|
| Format | `cargo fmt --check` | ✅ Pass | 2026-07-01 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ Pass | 2026-07-01 |
| Build | `cargo build --workspace` | ✅ Pass | 2026-07-01 |
| Tests | `cargo test --workspace` | ✅ Pass | 2026-07-01 |
| Security baseline | No stubs, deprecated, secrets, visual UI; unsafe blocks documented | ✅ Pass | 2026-07-01 |
| Dependencies | `cargo deny check` | ✅ Pass | 2026-07-01 |

---

## Documentation CI

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Link checker | `docs/ci/check-links.sh` | ✅ Pass (requires `cargo doc`) | 2026-07-01 |

**All CI gates pass.**

---

## Code Drift

See [`docs/status/corpus_inventory.yaml`](corpus_inventory.yaml).

**All 14 drift items resolved (2026-06-12).** Zero remaining spec_ahead, code_ahead, or divergent items.

---

## Sovereignty

| Check | Result |
|-------|--------|
| Magna Carta P1 (User Sovereignty) | Sovereignty distributed across `hkask-types::sovereignty`, `hkask-pods::sovereignty`, `hkask-services-core::verification`. No single SovereigntyService — this is correct, not a gap. |
| Magna Carta P2 (Affirmative Consent) | Regulation consent denial events emitted. Prohibition gate — denial is terminal. |
| Magna Carta P3 (Generative Space) | 16 MCP servers + multi-provider inference. No feature flags, no gated surfaces. |
| Magna Carta P4 (Clear Boundaries) | OCAP capability membrane. Dual-gate enforcement (require_capability + require_sovereignty) with Ed25519 cryptographic tokens. DenyAllConsent default. Verified across all capability-granting paths. |

---

## Session (2026-06-15) — Skills Training Expansion

**Training MCP Server — 8→15 tools, 1→5 providers, 3→14 tests:**

- Tools added: `training_evaluate` (exact/contains/semantic), `training_register_adapter` (persistent registry), `training_recommend_model` (base model guidance), `training_record_invocation` (episodic recording), `training_curate_feedback` (LLM-as-judge curation), `training_retrain` (merge+dedup+retrain with versioning), `training_ingest_dataset` (standalone dataset normalization).
- Tools enhanced: `training_generate_traces` (model override + chunking for large docs), `training_assemble_dataset` (system prompt support), `training_submit` (token-length validation), `training_status` (auto-register on completion + blob storage), `training_cancel` (PID-tracked SIGTERM for local providers), `training_list_adapters` (skill_name + version fields).
- Providers added: **Baseten** (managed infra + generated TRL/LoRA train.py, HF-native model loading, multi-LoRA serving), **Runpod** (GPU pod dispatch via GraphQL API). Total: 5 providers (Together AI, Baseten, Runpod, Axolotl, Unsloth).
- Infrastructure: Canonical `hkask_adapter::AdapterStore` wired into production (replaces deleted `SqliteAdapterStore`/`InMemoryAdapterStore`), `JobStore` with `training_jobs` table for persistent job tracking, `CompletionMetadata` trait for provider-agnostic training metrics, `adapter_weight_path` for local blob storage, `skill_name` + `version` fields on `TrainedLoRAAdapter`.
- Multi-LoRA inference: `LLMParameters.adapter` field added to `hkask-types`. `InferenceRouter::generate` + `generate_with_model` append `#adapter` to model name for Baseten multi-LoRA serving.
- Tests: 3→14 (7 JobStore tests, 4 chunking tests).
- Docs: `docs/architecture/PUBLIC_SURFACE-hkask-mcp-training.md` created, `docs/research/training-decomposition-traces.md` updated (completed items, provider table, Baseten/Runpod design decisions, deferred items).
- Infrastructure: Unsloth BF16 LoRA training pipeline for Qwen3.6-27B on RunPod community pods (bare pod + curl, single-command launch). Scripts hosted on HuggingFace (`Axolotl-Partners/rust-adapter-scripts`), not in the hKask repo (Python is not an acceptable hKask dependency). See `docs/how-to/training-and-adapters.md`.
- Deferred: `training_monitor_health` (needs active usage data), `training_ab_test` (needs multiple active versions). Fireworks AI provider removed (billing inefficiency). OpenRouter added as replacement inference provider.
- Build: ✅ All 18 workspace members compile. 14/14 training tests pass.

**R7.3 Public Seam Watcher — P8 Runtime Enforcement:**

- Plan: `docs/plans/r7.3-public-seam-watcher-v0.30.0.md` — 5-wave implementation plan with adversarial pragmatics+grill-me review. 5 gaps found and resolved (afferent signal, deployment path, surface count, temporal mismatch, asymmetric observability).
- JSON inventory: `scripts/public-seam-inventory.sh` extended with `build_json_inventory()` — generates machine-readable `docs/status/public-seam-inventory.json` alongside markdown. Both CI-enforced for drift.
- Types: `SeamCoverage`, `SeamInventory` in `hkask-types::regulation`. `SignalMetric::SeamCoverage` + `ActionType::Notify` in `hkask-types::loops`. 2 new canonical Regulation spans: `reg.architecture.seam.coverage`, `reg.architecture.seam.drift` (30→32 total).
- Core module: `hkask-regulation/src/seam_watcher.rs` — `SeamWatcher` (load, register_domains, check_drift, refresh, summary), `SeamDrift`, `SeamSummary`. Embedded JSON via `include_str!()` for deployment safety. File path override via `HKASK_SEAM_INVENTORY_PATH` for development. 9 REQ-tagged tests.
- Algedonic integration: `CyberneticsLoop::compute()` handles `SeamCoverage` — `BelowSetPoint`→`Escalate(Curation)` with severity grading (>5pp critical, 1–5pp warning), `AboveSetPoint`→`Notify(Curation)` for improvements. `seam_coverage_min` set-point (default: 0.0 = alert on any regression).
- Bootstrap: `AgentService::build()` loads seam watcher, registers 25 per-crate variety domains (`seam:{crate_name}`), spawns periodic background task (30-min interval, configurable via `HKASK_SEAM_CHECK_INTERVAL_SECS`). Watcher stored as `Arc<RwLock<Option<SeamWatcher>>>`.
- Curator surface: `/status` command displays R7.3 seam coverage — color-coded bar (green ≥60%, yellow 30–60%, red <30%), crate count, covered/total items, coverage %, REQ test count.
- R7.3 identity: domains updated to `["regulation", "seam"]`, description updated.
- Build: all 18 workspace members compile. 35/35 Regulation tests pass (9 new + 26 existing). CI inventory gate passes (markdown + JSON).
- Docs updated: `hKask-architecture-master.md` (Pattern C table, key properties, crates, identified gaps, Regulation span count, mermaid), `PROJECT_STATUS.md` (this update), `docs/plans/r7.3-public-seam-watcher-v0.30.0.md` (implementation summary).

**Pragmatics Codebase Audit + Test Coverage + MCP Server Tool Audit + Communication Tests:**

- Pragmatics audit: 7-task principle-grounded review across all 16 crates. All 7 tasks converge at δ=0. Zero P1–P12 violations.
- Key findings: Regulation feedback loop fully closed (sense→compute→act with live-channel + persistence fallback), OCAP tokens cryptographically unforgeable (HMAC-SHA256, constant-time verification), zero unsafe blocks, zero Rc<RefCell>, all domain concepts have strong types (WebID, SpanNamespace, DelegationToken, AttenuationLevel, DataCategory, etc.), condenser complete (7/7 tools), services extraction ~70%+ with no premature deletions.
- Build: 15/16 crates check clean (`hkask-mcp` has pre-existing tracing macro issue). All tests pass.

## Session (2026-06-14)

**Matrix Integration — Architecture, Specification, and Implementation:**

- Architecture research report: `docs/architecture/matrix-integration-architecture.md` (~1,800 lines). Full deployment model, client orchestration, identity binding, agent interaction patterns, essentialist/grill-me/pragmatic-semantics/pragmatic-cybernetics reviews, gap analysis, verification spec, Regulation span specification.
- Spec resolved 4 Blocking gaps (B1–B4: Caddy TLS automation, MXID format, `.well-known` delegation, Conduit config defaults), 6 Important gaps (I1–I6: recovery keys, device names, message format, room encryption, error taxonomy, gas accounting), 4 Prohibitions (P1–P4), 10 Guardrails (G1–G10).
- Implementation: `matrix.rs` — 303 lines of stubs replaced with ~380 lines of real `MatrixTransport` using `matrix-sdk` 0.16. Login, send_message, get_messages (on-demand polling), create_room, invite_user, list_rooms. Regulation tracing on all operations.
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
- 2 pre-existing bugs fixed: `identity.rs` missing `passphrase_set_at`, `docproc/tools.rs` broken `LedgerObserver` impl
- Docs created: `docs/how-to/skills-and-composition.md` (kata content inlined), `docs/status/skill-inventory.md` (117 lines)
- Docs updated: 4 frontmatter dates, `docs/README.md` portal, `hKask-architecture-master.md`, `DIAGRAMS_INDEX.md`, `PROJECT_STATUS.md`
- 18 files updated with corrected path references across YAML, Rust, markdown, and shell scripts

## Session (2026-06-11)

- Onboarding overhaul: model selection, passphrase strength UX, First Steps guide, `is_first_run` flag
- New `kask onboard` CLI subcommand for adding userpods to existing installations
- New `/start` guided tour (9 steps) and `/feedback` REPL-only ledger command
- 3 code bugs fixed: `run_add_userpod` dangerous fallback, `/start` cursor reset, stale comment
- 6 P8 tests added: `append_feedback` (3) + `passphrase_strength` (3) — total: 19→25
- Docs updated: AGENTS.md, cli-reference.md, REPL-specification.md, test-inventory.md
- Pre-existing build errors in `hkask-cli` and the old monolithic service crate tests confirmed resolved (prior session) [Note: as of v0.31.0, the old monolithic service crate has been decomposed into 11 subcrates]

## Session (2026-06-11)

- Handoff continuation: verified build (246 tests, 0 failures), fixed 3 unicode escape errors in `crates/hkask-services-core/src/bundle.rs` (Rust 2024 `\u{XXXX}` format) [Note: as of v0.31.0, the old monolithic service crate has been decomposed into 11 subcrates; `bundle.rs` was in the monolithic crate]
- HIGH #1: Transient AgentService accessor errors — confirmed resolved (no old accessor names in codebase, build clean)
- HIGH #2: Architecture master sovereignty claim — updated AgentService section to current named-accessor pattern, noted sovereignty distribution across `hkask-types`/`hkask-pods`/`hkask-services-core` [Note: as of v0.31.0, the old monolithic service crate has been decomposed into 11 subcrates]
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

## Session (2026-06-23) — TUI Enhancement

| Feature | Status |
|---------|--------|
| Command palette (Ctrl+P, fuzzy search 19 window kinds) | ✅ Complete |
| MCP Two-Tab pattern (Chat + Data tabs for 6 MCP windows) | ✅ Complete |
| MCP Chat scoping (start_scoped_inference, per-window tool filtering) | ✅ Complete |
| Layout persistence (save/restore per-agent) | ✅ Complete |
| Keybinding convention fix (Tab to focus next, Ctrl+N to new Chat) | Complete |
| Companies live bridge (MCP dispatch to hkask-mcp-companies) | ✅ Complete |
| Matrix file attachments (upload_file, send_file via MCP tools) | ✅ Complete |
| Media server default models (FA/qwen-3-tts, FA/wizper, KC/qwen/qwen3-vl-235b-a22b-instruct, FA/flux-2) | ✅ Complete |
| TUI tests | 96 (8 unit + 88 integration) |
| Kanban TUI (interactive multi-column board, task moving) | ✅ Complete |

## What Remains

| Priority | Task |
|----------|------|
| MEDIUM | Integration tests for multi-pod sync: Regulation span emission, CuratorSync polling |
| LOW | Citation compliance: 23 files have fewer footnotes than `##` sections. Methodology: `docs/status/citation-audit-methodology.md`. Script: `docs/ci/check-citations.sh`. |
| LOW | End-to-end onboarding smoke test (needs live Okapi) |
| DEFERRED | Pod container export (`kask pod export-container`) |
| DEFERRED | `acquire_budget()` dead code removal |
| DEFERRED | CyberneticsLoop pass-through method cleanup (9 methods of pure indirection) |

### Communication Server — Remaining Items

| Priority | Task | Status |
|----------|------|--------|
| MEDIUM | `kask matrix register --agent` credential verification against stored keystore hash | TODO — currently accepts any credential with format warning |
| MEDIUM | SAS QR code generation for device verification | Deferred to v2 (requires matrix-sdk-crypto, blocked by SQLCipher/SQLite conflict) |
| LOW | Daemon periodic sidecar health task (every 60s: poll containers, emit Regulation spans) | Deferred — `kask matrix status-sidecar` provides on-demand checks |
| LOW | Regulation span formal registration in Regulation registry | Deferred — spans emit via tracing, functional but not registered |
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

## Known Gaps

| Gap | Severity | Status | Description |
|-----|----------|--------|-------------|
| **Real `provision_endpoint` API integration** | Medium | ✅ Complete (P1-12) | Runpod: GraphQL `saveEndpoint` mutation. Baseten: REST `POST /v1/models`. Both use real HTTP calls with API keys. Verified 2026-06-15. |
| **Manual contract review** | High | ⬜ TODO | Contracts use `expect:` + `[P{N}]` annotations. Ongoing curation: verify annotations match functional requirements in FUNCTIONAL_SPECIFICATION.md. Run contract-generator for any gaps. |
| **`expect:` field coverage** | Medium | ⬜ Pattern Only | `expect:` syntax demonstrated in Regulation crate and wallet. Remaining crates need annotation. Run contract-generator per domain. |
| **Deployment domain ER diagram ↔ code sync** | Low | ⬜ TODO | ER diagram in `FUNCTIONAL_SPECIFICATION.md` §3.18 aligned with deployment plan but not verified against actual type definitions in `hkask-api` and `kask` CLI. |
| **Domain ER diagrams for non-Regulation domains** | Low | ⬜ Partial | ER diagrams added for 8 Regulation domains (§2) and deployment (§3.18). Remaining 18 non-Regulation domains (§3) have entity models described in tables but not yet diagrammed. |

---

*ℏKask v0.31.0 — A Sovereign Chat Client for Human Users with AI Skills*
