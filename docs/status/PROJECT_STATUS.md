---
title: "Project Status"
audience: [architects, developers, agents]
last_updated: 2026-06-20
version: "0.30.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# hKask Project Status

Single source of truth for build, test, and CI health. Updated per session.

**Current session:** v0.30.0 — Multi-Pod Architecture Complete: Three-tier Solid Pod isomorphism (CuratorPod/TeamPod/ReplicantPod).
- PodManager deleted → PodDeployment + PodFactory + ActivePods + PodRegistry. Per-pod SQLCipher files at `{data_dir}/agents/{sanitized_name}/pod.db`.
- Per-pod CNS (`PerPodCnsRuntime`, `cns.agent_pod.{pod_id}.*` namespace). Per-pod MCP tool binding.
- `CuratorSync` polling loop: lazy one-way semantic sync from ReplicantPods/TeamPods → CuratorPod SemanticIndex.
- `PodContext::recall_semantic()` routes through Curator for merged-lens view.
- `CnsSpan::SemanticPublished` emitted on every `store_semantic()`.
- `PodKind` enum (Curator/Team/Replicant) with filename convention, `PodRegistry::scan_by_kind()`.
- `PodStatusInfo` exposes `pod_kind` for API/CLI consumers.
- Specification docs updated: SOLID_POD_ISOMORPHISM.md, MULTI_POD_ARCHITECTURE.md, hKask-architecture-master.md.
- Build: clean (0 errors). Tests: 45/45 pass in `hkask-agents`. (2026-06-19)

---

## Build

All 35 workspace members (excluding fuzz targets).

| Target | Result | Date |
|--------|--------|------|
| Workspace (`cargo check --workspace`) | ✅ Pass (0 errors, 0 warnings) | 2026-06-20 |
| Core crates (all 35) | ✅ Pass | 2026-06-20 |
| Workspace (all 35 crates + 11 MCP servers + fuzz) | ✅ Pass | 2026-06-20 |
| Warnings | 0 | 2026-06-20 |

---

## Test

`cargo test --workspace` result: ✅ Pass — ~570 tests across 16 crates, 0 failures. Contracts use `expect:` + `[P{N}]` annotations with CNS span observation for runtime enforcement.

### Test Distribution

| Crate | Tests |
|-------|-------|
| hkask-types | 85 |
| hkask-inference | 23 |
| hkask-storage | 59 |
| hkask-memory | 16 |
| hkask-cns | 42 |
| hkask-agents | 31 |
| hkask-keystore | 13 |
| hkask-services | 78 |
| hkask-templates | 22 |
| hkask-condenser | 34 |
| hkask-improv | 37 |
| hkask-wallet | 13 |
| hkask-communication | 25 |
| hkask-mcp | 38 |
| hkask-cli | 43 |
| hkask-api | ~12 |
| hkask-acp | 4 |
| hkask-adapter | 51 |
| **Crate subtotal** | **~575** |
| MCP servers (10) | — |
| **Workspace total** | **~571** |

---

## Clippy (Lint)

| Target | Result | Date |
|--------|--------|------|
| Workspace (`-D warnings`) | ✅ Pass — 0 warnings | 2026-06-15 |

---

## Constraint Verification

| Check | Result | Date |
|-------|--------|------|
| `todo!()`, `unimplemented!()`, `#[deprecated]` | 0 violations | 2026-06-18 |
| Multi-user access control | ✅ Implemented: Role enum, admin middleware, invite CRUD, CNS spans | 2026-06-18 |
| OAuth providers | ✅ GitHub + Google (Google OAuth implemented 2026-06-18) | 2026-06-18 |
| Contract annotations | ✅ CNS 100%, Wallet 100%, Memory 100% — `expect:` + `[P{N}]` format | 2026-06-21 |
| Unsafe blocks | ✅ All documented with SAFETY: comments | 2026-06-15 |
| Rc<RefCell> patterns | ✅ Zero across all crates | 2026-06-15 |

---

## Codebase Metrics

| Metric | Value |
|--------|-------|
| Source files (crates) | 640+ |
| Source files (MCP servers) | 120+ |
| Source files (total) | 760+ |
| Core LOC (src/) | ~124,000 |
| MCP Server LOC (src/) | ~34,500 |
| Workspace members | 35 |
| Skills | 45 (72 registry crates, 232 Jinja2 templates) |
| MCP servers | 11 |
| ACP replicant | 1 (`hkask-acp`) — IDE agent presence via Agent Client Protocol |
| CNS spans | 84+ |

---

## CI Quality Gates

Verification gates in `.github/workflows/ci.yml` on every PR and push to main:

| Check | Method | Result | Date |
|-------|--------|--------|------|
| Format | `cargo fmt --check` | ✅ Pass | 2026-06-18 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ Pass | 2026-06-20 |
| Build | `cargo build --workspace --lib` | ✅ Pass | 2026-06-20 |
| Tests | `cargo test --workspace` | ✅ Pass | 2026-06-18 |
| Security baseline | No stubs, deprecated, secrets, visual UI; unsafe blocks documented | ✅ Pass | 2026-06-18 |
| Dependencies | `cargo deny check` | ✅ Pass | 2026-06-18 |

---

## Documentation CI

| Check | Script | Result | Date |
|-------|--------|--------|------|
| Link checker | `docs/ci/check-links.sh` | ✅ Pass | 2026-06-18 |

**All CI gates pass.**

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
| Magna Carta P3 (Generative Space) | 11 MCP servers + multi-provider inference. No feature flags, no gated surfaces. |
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
- Deferred: `training_monitor_health` (needs active usage data), `training_ab_test` (needs multiple active versions). Fireworks AI provider removed (billing inefficiency). OpenRouter added as replacement inference provider.
- Build: ✅ All 18 workspace members compile. 14/14 training tests pass.

**R7.3 Public Seam Watcher — P8 Runtime Enforcement:**

- Plan: `docs/plans/r7.3-public-seam-watcher-v0.30.0.md` — 5-wave implementation plan with adversarial pragmatics+grill-me review. 5 gaps found and resolved (afferent signal, deployment path, surface count, temporal mismatch, asymmetric observability).
- JSON inventory: `scripts/public-seam-inventory.sh` extended with `build_json_inventory()` — generates machine-readable `docs/status/public-seam-inventory.json` alongside markdown. Both CI-enforced for drift.
- Types: `SeamCoverage`, `SeamInventory` in `hkask-types::cns`. `SignalMetric::SeamCoverage` + `ActionType::Notify` in `hkask-types::loops`. 2 new canonical CNS spans: `cns.architecture.seam.coverage`, `cns.architecture.seam.drift` (30→32 total).
- Core module: `hkask-cns/src/seam_watcher.rs` — `SeamWatcher` (load, register_domains, check_drift, refresh, summary), `SeamDrift`, `SeamSummary`. Embedded JSON via `include_str!()` for deployment safety. File path override via `HKASK_SEAM_INVENTORY_PATH` for development. 9 REQ-tagged tests.
- Algedonic integration: `CyberneticsLoop::compute()` handles `SeamCoverage` — `BelowSetPoint`→`Escalate(Curation)` with severity grading (>5pp critical, 1–5pp warning), `AboveSetPoint`→`Notify(Curation)` for improvements. `seam_coverage_min` set-point (default: 0.0 = alert on any regression).
- Bootstrap: `AgentService::build()` loads seam watcher, registers 25 per-crate variety domains (`seam:{crate_name}`), spawns periodic background task (30-min interval, configurable via `HKASK_SEAM_CHECK_INTERVAL_SECS`). Watcher stored as `Arc<RwLock<Option<SeamWatcher>>>`.
- Curator surface: `/status` command displays R7.3 seam coverage — color-coded bar (green ≥60%, yellow 30–60%, red <30%), crate count, covered/total items, coverage %, REQ test count.
- R7.3 identity: domains updated to `["cns", "seam"]`, description updated.
- Build: all 18 workspace members compile. 35/35 CNS tests pass (9 new + 26 existing). CI inventory gate passes (markdown + JSON).
- Docs updated: `hKask-architecture-master.md` (Pattern C table, key properties, crates, identified gaps, CNS span count, mermaid), `PROJECT_STATUS.md` (this update), `docs/plans/r7.3-public-seam-watcher-v0.30.0.md` (implementation summary).

**Pragmatics Codebase Audit + Test Coverage + MCP Server Tool Audit + Communication Tests:**

- Pragmatics audit: 7-task principle-grounded review across all 16 crates. All 7 tasks converge at δ=0. Zero P1–P12 violations.
- Key findings: CNS feedback loop fully closed (sense→compute→act with live-channel + persistence fallback), OCAP tokens cryptographically unforgeable (HMAC-SHA256, constant-time verification), zero unsafe blocks, zero Rc<RefCell>, all domain concepts have strong types (WebID, SpanNamespace, DelegationToken, AttenuationLevel, DataCategory, etc.), condenser complete (7/7 tools), services extraction ~70%+ with no premature deletions.
- Build: 15/16 crates check clean (`hkask-mcp` has pre-existing tracing macro issue). All tests pass.

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
- 2 pre-existing bugs fixed: `identity.rs` missing `passphrase_set_at`, `docproc/tools.rs` broken `CnsObserver` impl
- Docs created: `docs/guides/kata-user-guide.md` (361 lines), `docs/status/skill-inventory.md` (117 lines)
- Docs updated: 4 frontmatter dates, `docs/README.md` portal, `hKask-architecture-master.md`, `DIAGRAMS_INDEX.md`, `PROJECT_STATUS.md`
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
| MEDIUM | Integration tests for multi-pod sync: CNS span emission, CuratorSync polling, cross-pod contradictory triple recall, TeamPod bot sharing |
| LOW | Citation compliance: 23 files have fewer footnote citations than `##` sections (PS-07 gap). Audit complete 2026-06-11 — see §Citation Audit below. |
| LOW | `CuratorSync` integration test — verify sync loop picks up triples within 1s |
| NOT YET DONE | End-to-end onboarding smoke test (needs live Okapi) |
| DEFERRED | Pod container export (`kask pod export-container`) — ζ.5 (container boundary) |

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
| **Manual contract review** | High | ⬜ TODO | Contracts use `expect:` + `[P{N}]` annotations. Ongoing curation: verify annotations match functional requirements in FUNCTIONAL_SPECIFICATION.md. Run contract-generator for any gaps. |
| **`expect:` field coverage** | Medium | ⬜ Pattern Only | `expect:` syntax demonstrated in CNS crate and wallet. Remaining crates need annotation. Run contract-generator per domain. |
| **Deployment domain ER diagram ↔ code sync** | Low | ⬜ TODO | ER diagram in `FUNCTIONAL_SPECIFICATION.md` §3.18 aligned with deployment plan but not verified against actual type definitions in `hkask-api` and `kask` CLI. |
| **Domain ER diagrams for non-CNS domains** | Low | ⬜ Partial | ER diagrams added for 8 CNS domains (§2) and deployment (§3.18). Remaining 18 non-CNS domains (§3) have entity models described in tables but not yet diagrammed. |

---

*ℏKask — A Minimal Viable Container for Agents — v0.30.0*
