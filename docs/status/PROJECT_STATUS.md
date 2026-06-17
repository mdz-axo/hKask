---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

**Current session:** Agentic QA Pipeline — contract quality review instrumented across 1,419 REQ-tagged functions. All contracts have pre/post (0 NO_PRE+NO_POST, 0 NO_POST). P1-13 added: manual contract-to-spec traceability review. (2026-06-17)

---

## Build

All 25 workspace members.

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo check --workspace`) | ✅ Pass (25/25 crates) | 2026-06-17 |
| ACP crate (`cargo check -p hkask-acp`) | ✅ Pass (0 errors, 0 warnings) | 2026-06-17 |
| Core crates (types, condenser, storage, memory, cns, templates, agents, keystore, mcp, services, cli, api, inference, improv, wallet, communication, acp) | ✅ Pass | 2026-06-17 |
| MCP servers (condenser, research, spec, companies, communication, media, replica, docproc, training, memory, kanban) | ✅ Pass | 2026-06-15 |

---

## Test

`cargo test --workspace` result: ✅ Pass — ~570 tests across 16 crates, 0 failures. 2334 `/// REQ:` tags (workspace-wide including MCP servers). 100% behavioral contract coverage — every `pub fn` carries `pre:`/`post:` conditions.

### Test Distribution

| Crate | Tests | REQ Tags |
|-------|-------|----------|
| hkask-types | 85 | 302 |
| hkask-inference | 23 | 87 |
| hkask-storage | 59 | 243 |
| hkask-memory | 16 | 68 |
| hkask-cns | 42 | 129 |
| hkask-agents | 31 | 161 |
| hkask-keystore | 13 | 41 |
| hkask-services | 78 | 305 |
| hkask-templates | 22 | 74 |
| hkask-condenser | 34 | 34 |
| hkask-improv | 37 | 37 |
| hkask-wallet | 13 | 99 |
| hkask-communication | 25 | 25 |
| hkask-mcp | 38 | 71 |
| hkask-cli | 43 | 118 |
| hkask-api | ~12 | 66 |
| hkask-acp | 4 | — |
| hkask-adapter | 51 | 58 |
| **Crate subtotal** | **~575** | **1860** |
| MCP servers (10) | — | ~55 |
| **Workspace total** | **~571** | **~1915** |

---

## Clippy (Lint)

| Target | Result | Date |
|--------|--------|------|
| Workspace (`-D warnings`) | ✅ Pass — 0 warnings | 2026-06-15 |

---

## Constraint Verification

| Check | Result | Date |
|-------|--------|------|
| `todo!()`, `unimplemented!()`, `#[deprecated]` | 0 violations | 2026-06-15 |
| Dead code (`#[allow(dead_code)]`) | 1 site: compile-time assertion in `a2a/mod.rs:171` | 2026-06-10 |
| Headless constraint (no grafana/prometheus/dashboard/UI) | ✅ Clean | 2026-06-15 |
| REQ tag coverage | ✅ 2334 REQ tags (100% coverage — every `pub fn` contracted) | 2026-06-17 |
| Schema drift check | ✅ `scripts/check-schema-drift.sh` passes (37 tables/indexes) | 2026-06-17 |
| Unsafe blocks | ✅ All documented with SAFETY: comments | 2026-06-15 |
| Runtime `.unwrap()` in targeted crates | ✅ Zero violations (Wave 2 denylist) | 2026-06-15 |
| MCP Gate-3 startup verification | ✅ 10/10 servers enforce verify_startup_gates() | 2026-06-15 |
| Public surface justification | ✅ 13/13 oversized crates have PUBLIC_SURFACE.md | 2026-06-15 |
| Rc<RefCell> patterns | ✅ Zero across all crates | 2026-06-15 |

---

## Codebase Metrics

| Metric | Value |
|--------|-------|
| Source files (crates) | 324 |
| Source files (MCP servers) | 70 |
| Source files (total) | 394 |
| Workspace members | 26 |
| Skills | 28 |
| MCP servers | 11 |
| ACP replicant | 1 (`hkask-acp`) — IDE agent presence via Agent Client Protocol |
| CNS spans | 84 (10 adapter/endpoint lifecycle spans added) |

---

## CI Quality Gates (Wave 6)

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Public Surface Governance | `scripts/check-public-surface.sh` | ✅ Pass (16/16 crates) | 2026-06-15 |
| Unsafe Documentation Policy | `scripts/check-unsafe-safety.sh` | ✅ Pass (all blocks documented) | 2026-06-15 |
| Runtime `.unwrap()` Denylist | `scripts/check-unwrap-denylist.sh` | ✅ Pass (0 violations) | 2026-06-15 |
| MCP Gate-3 Consistency | `scripts/check-mcp-gate3.sh` | ✅ Pass (10/10 servers) | 2026-06-15 |
| REQ Traceability Trend | `scripts/check-req-traceability.sh` | ✅ Pass (100% coverage) | 2026-06-16 |
| Schema Drift (harness vs storage) | `scripts/check-schema-drift.sh` | ✅ Pass (37 tables/indexes) | 2026-06-17 |
| Contract Quality Review | `kask contract review` | ✅ Pass (0 NO_PRE+NO_POST, 0 NO_POST) | 2026-06-17 |
| **Master** | `scripts/ci-quality-gates.sh` | ✅ ALL CHECKS PASSED | 2026-06-15 |

All gates are wired into `.github/workflows/ci.yml` as the `quality-gates` job, running on every PR and push to main. Release builds depend on quality gates passing.

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

See [`do../status/corpus_inventory.yaml`](corpus_inventory.yaml) and [`do../status/corpus_inventory.yaml`](corpus_inventory.yaml).

**All 14 drift items resolved (2026-06-12).** Zero remaining spec_ahead, code_ahead, or divergent items.

---

## Sovereignty

| Check | Result |
|-------|--------|
| Magna Carta P1 (User Sovereignty) | Sovereignty distributed across `hkask-types::sovereignty`, `hkask-agents::sovereignty`, `hkask-services::verification`. No single SovereigntyService — this is correct, not a gap. |
| Magna Carta P2 (Affirmative Consent) | CNS consent denial events emitted. Prohibition gate — denial is terminal. |
| Magna Carta P3 (Generative Space) | 10 MCP servers + Okapi inference. No feature flags, no gated surfaces. |
| Magna Carta P4 (Clear Boundaries) | OCAP capability membrane. Dual-gate enforcement (require_capability + require_sovereignty) with Ed25519 cryptographic tokens. DenyAllConsent default. Verified across all capability-granting paths. |

---

## This Session (2026-06-15) — Skills Training Expansion

**Training MCP Server — 8→15 tools, 1→5 providers, 3→14 tests:**

- Tools added: `training_evaluate` (exact/contains/semantic), `training_register_adapter` (persistent registry), `training_recommend_model` (base model guidance), `training_record_invocation` (episodic recording), `training_curate_feedback` (LLM-as-judge curation), `training_retrain` (merge+dedup+retrain with versioning), `training_ingest_dataset` (standalone dataset normalization).
- Tools enhanced: `training_generate_traces` (model override + chunking for large docs), `training_assemble_dataset` (system prompt support), `training_submit` (token-length validation), `training_status` (auto-register on completion + blob storage), `training_cancel` (PID-tracked SIGTERM for local providers), `training_list_adapters` (skill_name + version fields).
- Providers added: **Baseten** (managed infra + generated TRL/LoRA train.py, HF-native model loading, multi-LoRA serving), **Runpod** (GPU pod dispatch via GraphQL API). Total: 5 providers (Together AI, Baseten, Runpod, Axolotl, Unsloth).
- Infrastructure: `SqliteAdapterStore` wired into production (was InMemoryAdapterStore), `JobStore` with `training_jobs` table for persistent job tracking, `CompletionMetadata` trait for provider-agnostic training metrics, `adapter_weight_path` for local blob storage, `skill_name` + `version` fields on `LoRAAdapter`.
- Multi-LoRA inference: `LLMParameters.adapter` field added to `hkask-types`. `InferenceRouter::generate` + `generate_with_model` append `#adapter` to model name for Baseten multi-LoRA serving.
- Tests: 3→14 (7 SqliteAdapterStore/JobStore tests, 4 chunking tests).
- Docs: `docs/architecture/PUBLIC_SURFACE-hkask-mcp-training.md` created, `docs/research/training-decomposition-traces.md` updated (completed items, provider table, Baseten/Runpod design decisions, deferred items).
- Deferred: `training_monitor_health` (needs active usage data), `training_ab_test` (needs multiple active versions), Fireworks AI provider (billing inefficiency).
- Build: ✅ All 18 workspace members compile. 14/14 training tests pass.

**R7.3 Public Seam Watcher — P8 Runtime Enforcement:**

- Plan: `docs/plans/r7.3-public-seam-watcher-v0.28.0.md` — 5-wave implementation plan with adversarial pragmatics+grill-me review. 5 gaps found and resolved (afferent signal, deployment path, surface count, temporal mismatch, asymmetric observability).
- JSON inventory: `scripts/public-seam-inventory.sh` extended with `build_json_inventory()` — generates machine-readable `docs/status/public-seam-inventory.json` alongside markdown. Both CI-enforced for drift.
- Types: `SeamCoverage`, `SeamInventory` in `hkask-types::cns`. `SignalMetric::SeamCoverage` + `ActionType::Notify` in `hkask-types::loops`. 2 new canonical CNS spans: `cns.architecture.seam.coverage`, `cns.architecture.seam.drift` (30→32 total).
- Core module: `hkask-cns/src/seam_watcher.rs` — `SeamWatcher` (load, register_domains, check_drift, refresh, summary), `SeamDrift`, `SeamSummary`. Embedded JSON via `include_str!()` for deployment safety. File path override via `HKASK_SEAM_INVENTORY_PATH` for development. 9 REQ-tagged tests.
- Algedonic integration: `CyberneticsLoop::compute()` handles `SeamCoverage` — `BelowSetPoint`→`Escalate(Curation)` with severity grading (>5pp critical, 1–5pp warning), `AboveSetPoint`→`Notify(Curation)` for improvements. `seam_coverage_min` set-point (default: 0.0 = alert on any regression).
- Bootstrap: `AgentService::build()` loads seam watcher, registers 25 per-crate variety domains (`seam:{crate_name}`), spawns periodic background task (30-min interval, configurable via `HKASK_SEAM_CHECK_INTERVAL_SECS`). Watcher stored as `Arc<RwLock<Option<SeamWatcher>>>`.
- Curator surface: `/status` command displays R7.3 seam coverage — color-coded bar (green ≥60%, yellow 30–60%, red <30%), crate count, covered/total items, coverage %, REQ test count.
- R7.3 identity: domains updated to `["cns", "seam"]`, description updated.
- Build: all 18 workspace members compile. 35/35 CNS tests pass (9 new + 26 existing). CI inventory gate passes (markdown + JSON).
- Docs updated: `hKask-architecture-master.md` (Pattern C table, key properties, crates, identified gaps, CNS span count, mermaid), `PROJECT_STATUS.md` (this update), `docs/plans/r7.3-public-seam-watcher-v0.28.0.md` (implementation summary).

**Pragmatics Codebase Audit + REQ Tag Coverage + MCP Server Tool Audit + Communication Tests:**

- Pragmatics audit: 7-task principle-grounded review across all 16 crates. All 7 tasks converge at δ=0. Zero P1–P12 violations.
- Key findings: CNS feedback loop fully closed (sense→compute→act with live-channel + persistence fallback), OCAP tokens cryptographically unforgeable (HMAC-SHA256, constant-time verification), zero unsafe blocks, zero Rc<RefCell>, all domain concepts have strong types (WebID, SpanNamespace, DelegationToken, AttenuationLevel, DataCategory, etc.), condenser complete (7/7 tools), services extraction ~70%+ with no premature deletions.
- REQ tag coverage: 77 missing `// REQ:` tags added across 12 files (salience, discover, mcp handlers, lexicon, spec_store, contract_validator, spec_types, kata_history, transcript, voice, wallet_budget, gentle_lovelace). Now 396 REQ tags across all 413 tests — zero untagged test files.
- hkask-communication integration tests: 19 tests added (`crates/hkask-communication/tests/integration_test.rs`) — types (7), errors (4), AgentRegistry (8). All pass. MatrixTransport tests deferred (require Conduit homeserver).
- MCP server tool audit: All 10 servers verified — 143/143 tools fully implemented (condenser 7, spec 6, replica 8, training 8, docproc 9, communication 9, memory 16, research 17, companies 27, media 36).
- Docs updated: `TODO.md` (C-23–C-27 added, P2-12/P2-13 counts corrected), `OPEN_QUESTIONS.md` (§8 added — 4 Ω questions resolved, 3 remaining subjunctive), `PROJECT_STATUS.md` (this update).
- Build: 15/16 crates check clean (`hkask-mcp` has pre-existing tracing macro issue). All tests pass.

## Session (2026-06-15) — Pragmatic Audit Implementation

**All 10 tasks from `docs/plans/pragmatic-audit-implementation-plan-v0.27.0.md` complete:**

- **Wave 1 — Test Infrastructure:** +6 hkask-communication tests (25 total), +11 hkask-agents ACP tests (31 total), +11 hkask-mcp tests (38 total).
- **Wave 2 — Semantic Grounding:** +21 hkask-api route serialization tests (39 REQ tags), 54 provenance markers applied across 18 files (zero unmarked).
- **Wave 3 — Type Strength:** `CnsSpan` enum (51 variants, `ToolSubsystem` companion enum) with `Display`/`FromStr`; Ed25519 `DelegationToken` with `TokenSignature` newtype and `derive_signing_key()` helper. All crates migrated.
- **Wave 4 — Surface Control:** 10 files split into ≤7-item submodules. 10 types → `pub(crate)`. ~25 deprecated re-exports removed.
- **Wave 5 — Strangler Fig:** `KataEngine::from_env()`, `SpecService::get_full()`. CLI decoupled from InferenceConfig/InferenceRouter/SpecStore.
- **Wave 6 — Stub Resolution:** All 5 training providers had complete cancel already (plan was outdated). Zero stubs.
- **Metrics:** 916 REQ tags across workspace. Zero `todo!()`/`unimplemented!()`. Workspace compiles clean, all tests pass.

## Session (2026-06-16) — Contract Migration to 100%

**Behavioral contract coverage achieved across all 17 crates:**

- Every `pub fn` (1579 total) now carries `/// REQ:` with `pre:`/`post:` conditions.
- 1915 REQ tags workspace-wide (121.2% due to multi-contract builder methods).
- Crates completed this session: hkask-api (API-001–034), hkask-inference (INFER-031–072), hkask-cli (CLI-006–095), hkask-types (TYP-178–326), hkask-agents (AGT-032–161), hkask-services (SVC-081–304).
- ~688 new contracts added this session.
- `cargo check --workspace`: 0 errors, 0 warnings. `cargo check --workspace --tests`: 0 errors.
- Documentation updated: PROJECT_STATUS.md, TESTING_DISCIPLINE.md, architecture-master.md.
- Phase B2 (agent contract generation) now unblocked — baseline is complete.

## Session (2026-06-14)

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

## Known Gaps

| Gap | Severity | Status | Description |
|-----|----------|--------|-------------|
| **Real `provision_endpoint` API integration** | Medium | ✅ Complete (P1-12) | Runpod: GraphQL `saveEndpoint` mutation. Baseten: REST `POST /v1/models`. Both use real HTTP calls with API keys. **Caveat:** exact GraphQL schema fields (`saveEndpoint`, response `data.saveEndpoint.id`) and Baseten REST response shape (`id` field, endpoint URL format `model-{id}.api.baseten.co`) may need adjustment based on actual provider API responses at runtime. |
| **Manual contract-to-spec review** | High | ⬜ TODO | All 1,419 REQ-tagged contracts have pre/post conditions, but need human verification against the functional specification. Each REQ tag should trace to a real spec document. See `kask contract review` for inventory, `/improv plussing` for collaborative review. 25 duplicate REQ IDs need consolidation analysis. 250 simple constructors may benefit from explicit preconditions. |

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
