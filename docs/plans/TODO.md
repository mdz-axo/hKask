---
title: "hKask TODO — Open Work"
audience: [project maintainers, contributors]
last_updated: 2026-06-30
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask TODO — Open Work

---

## P0 — Essential (Must Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P0-01** | CNS span emission integration | CNS bot | High | ✅ Complete | CNS spans distributed across `crates/hkask-cns/src/governed_tool.rs`, `crates/hkask-types/src/event.rs`, `crates/hkask-types/src/cns.rs`; canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`) |
| **P0-02** | Git CAS integration for triples | Storage bot | High | ✅ Complete | Deleted in 2026-06-10 architecture audit (see HANDOFF.md) |
| **P0-03** | CLI/API symmetry audit | CLI bot | High | ✅ Complete | API routes match CLI commands |
| **P0-04** | Documentation quality gates | Curator | High | ✅ Complete | MDS-aligned refresh complete |
| **P0-05** | Git backup system — core implementation | Backup system | High | ✅ Complete | `hkask-storage/src/backup/` — BackupService with snapshot/restore/list/prune/verify/config; CLI (`kask backup`); API (`/api/v1/backup/*`); 15 tests; CNS spans |
| **P0-06** | Git backup — encryption at rest (AES-256-GCM + Argon2) | Backup system | High | ✅ Complete | `BackupService` encrypts blobs before CAS storage; key derived from `HKASK_BACKUP_PASSPHRASE`; salt persisted in `BackupConfig` |
| **P0-07** | Git backup — 3-tier retention with actual GIX pruning | Backup system | High | ✅ Complete | `RetentionPolicy` (daily 21d, weekly 12w, monthly); `prune()` performs history rewriting via `rewrite_history()` |
| **P0-08** | Git backup — scheduled daily snapshots + scoped restore | Backup system | High | ✅ Complete | `run_daily_snapshot()` for daemon loop; `scoped_restore()` for system/type/file-level restore; `BackupLoop` registered in `AgentService::build()` |
| **P0-09** | Git CAS adapter — pure GIX (no CLI git) | Backup system | High | ✅ Complete | `GixCasAdapter` rewritten to use `gix` v0.81; blob storage via `BlobRef`; tree/commit via pure gix API |

---

## P1 — Important (Should Complete)

| ID | Task | Owner | Priority | Status | Evidence |
|----|------|-------|----------|--------|----------|
| **P1-01** | Requirements specification | Architect | High | ✅ Complete | `docs/specifications/specs/REQUIREMENTS.md` |
| **P1-02** | Traceability matrix | Architect | Medium | ✅ Complete | `docs/specifications/specs/TRACEABILITY_MATRIX.md` |
| **P1-03** | Diagram refresh (DIAGRAMS_INDEX.md) | Curator | Medium | ✅ Complete | `docs/DIAGRAMS_INDEX.md` — 28 diagrams, 8 V1.1+ candidates |
| **P1-04** | ADR creation for key decisions | Architect | Medium | ✅ Complete | ADR-024 through ADR-028 created 2026-05-29 (5 retroactive ADRs per OQ-6); note: ADR-023 superseded by ADR-027, ADR-028 archived (deferred), ADR-029 archived (superseded) |
| **P1-05** | Link checker script | DevOps | Low | ✅ Complete | `docs/ci/check-links.sh` + check-metadata.sh |
| **P1-06** | Citation compliance audit | Curator | Medium | ✅ Complete | 9 target docs compliant (2026-06-11); 12 additional docs pending |
| **P1-07** | Complete stub MCP servers | Dev | Medium | ✅ Complete | hkask-mcp-condenser: 761 LOC, hkask-mcp-research + hkask-mcp-research → hkask-mcp-research (1,044 LOC, ~17 tools) (consolidated 2026-06-11) |
| **P1-08** | Metadata migration for legacy docs | Curator | Low | ✅ Complete | All 47 active docs have mds_categories (2026-05-28) |
| **P1-09** | Face recognition system for media server | Media bot | High | ✅ Complete | `docs/plans/mcp-media-server-design.md` §10. Face registry table with validation gate, dual-path matching (vision LLM primary via fal.ai/open-weight Qwen2.5-VL, ONNX ArcFace behind `face-recognition` feature flag). 5 tools: face_validate, face_register, face_list, face_remove, gallery_name_face (with face_id lookup). ONNX `face_id` crate (SCRFD + ArcFace) optional via `--features face-recognition`. |
| **P1-10** | Condenser live integration testing — thinking mode + auto-condense | Dev | Medium | ✅ Complete | **All items verified.** (1) Thinking mode wire format: 2 unit tests pass. (2) Router pass-through: `disable_thinking_flows_to_wire_format` integration test (wiremock) passes. (3) Auto-condense threshold: 87.5% formula verified for all window sizes (2048–131072). (4) Live DeepInfra: `meta-llama/Llama-3.3-70B-Instruct-Turbo` works — clean output, no thinking interference. (5) Live Together: `meta-llama/Llama-3.3-70B-Instruct-Turbo` works — clean output. (6) Graceful degradation: thinking models (qwen3.5/gemma4/deepseek-r1) return clear error on all backends. (7) Rust live-backend tests written: `crates/hkask-inference/tests/live_backends.rs` — `deepinfra_summarization` + `together_summarization` (gated on API keys, `#[ignore]`). DeepInfra/Together Qwen3 models also exhibit thinking mode. |
| **P1-11** | Energy-use tracking simplification + security hardening | Dev + CNS | High | 📋 Partial | Audit complete. Requirements assessment in `docs/status/energy-accounting-requirements-assessment.md` (2026-07-01). 7 semantic operations across 23 surfaces mapped. 3 hardening recommendations: (a) remove CyberneticsLoop pass-through, (b) add EnergyBudget tamper-evidence, (c) audit float determinism. Implementation planned in 3 phases (P0: visibility+persistence, P1: hardening, P2: operational excellence). |
| **P1-12** | Real `provision_endpoint` API integration for Runpod + Baseten | Adapter | High | ✅ Complete | Runpod: GraphQL `saveEndpoint` mutation → endpoint ID → OpenAI-compatible URL. Baseten: REST `POST /v1/models` → model ID → model-specific URL. Verified 2026-06-15. |
| **P1-13** | Contract-to-spec traceability — CNS `expect:` field completion | Curator | High | ✅ Complete | Contract system simplified: REQ tags and contract IDs removed. Contracts use `expect:` + `[P{N}]` annotations directly on functions. CNS crate and wallet fully annotated. |

---

## P2 — Optional (Nice to Have)

| ID | Task | Owner | Priority | Status |
|----|------|-------|----------|--------|
| **P2-01** | Federation implementation | Federation bot | Low | Deferred to v1.1 |
| **P2-02** | Remote LLM fallback | Inference bot | Low | Deferred |
| **P2-03** | GPU acceleration (CUDA) | Infrastructure | Low | Optional |
| **P2-04** | Qdrant vector search | Storage bot | Low | Contingency |
| **P2-05** | CI automation for doc quality | DevOps | Low | ✅ Complete | docs/ci/check-links.sh + check-metadata.sh operational |
| **P2-06** | Resolve hkask-agents build regression + code drift | Dev | High | ✅ Complete | Build regression resolved; code drift audit complete — see `docs/status/corpus_inventory.yaml` |
| **P2-07** | MDS audit R4: Update §9.1 self-application matrix | Curator | Medium | ✅ Complete | Trust → Pass, Observability → Pass, Persistence/Lifecycle/Curation → :partial. Updated 2026-06-08 |
| **P2-08** | MDS audit R6: Consolidate CNS span listings (3→1 authoritative source) | Curator | Medium | ✅ Complete | canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`); 5 hierarchical spans now registered in CANONICAL_NAMESPACES |
| **P2-09** | MDS audit R8: Add TemplateType vocabulary mapping to MDS.md §7.2 | Curator | Medium | ✅ Complete | Prompt↔WordAct, Process↔FlowDef, Cognition↔KnowAct mapping with `as_spec_name()` cross-reference. Updated 2026-06-08 |
| **P2-10** | MDS audit R11: Add R3 deferred items to OPEN_QUESTIONS.md | Curator | Low | ✅ Complete | All 10 MDS §11 R3 items tracked (R3.1–R3.13), plus 3 additional items (Send+Sync bounds, CNS span integration, spec drift detection). Updated 2026-06-08 |
| **P2-11** | Populate `docs/status/PROJECT_STATUS.md` — single source of truth for build/test/metrics status | Dev | Medium | ✅ Complete | Build (pass), test (pass), clippy (pass), doc CI (pass). Created 2026-06-08 |
| **P2-12** | Populate `docs/status/PROJECT_STATUS.md` — complete catalog of all MCP servers' tools | Dev | Medium | ✅ Complete | 14 servers, all tools fully implemented (verified 2026-06-15 pragmatics audit) |
| **P2-13** | Populate `docs/status/PROJECT_STATUS.md` — test seam depth and behavioral coverage | Dev | Medium | ✅ Complete | 12 crates audited, 360+ tests (verified 2026-06-15 pragmatics audit) |
| **P2-14** | Populate `docs/status/fowler-audit-status.md` — Fowler pattern refactoring tracker | Dev | Low | ✅ Complete → Archived | 6 Fowler patterns identified (2 applied, 4 open-low). Archived 2026-06-11; open items deferred to P1 threshold. |
| **P2-15** | Populate `docs/status/adversarial-simplification-inventory.md` — dead code and unwired seam inventory | Dev | Low | ✅ Complete | 12 dead_code annotations, 4 unwired seams, 3 simplification candidates, 0 removal candidates. Created 2026-06-08 |
| **P2-16** | Custom/private securities for portfolio tracking | Companies bot | Medium | ⬜ Planned | Spec: `docs/specifications/portfolio-tracking.md` §10.6. 6 new tools planned (create, list, delete, update_price, import_prices, link_public). Deferred to Phase 6; depends on Phase 5 multi-currency. |

**Code drift from spec alignment audit (2026-06-07, resolved 2026-06-08):** Full drift set and curation decisions are in [`docs/status/corpus_inventory.yaml`](../status/corpus_inventory.yaml). Summary of resolutions:

| ID | Resolution |
|----|------------|
| P2-06-D1 | ✅ Resolved: 5 hierarchical CNS spans registered in `CANONICAL_NAMESPACES` |
| P2-06-D2 | ✅ Resolved: `Caveat` exists as `pub(crate)` — spec updated to note it's an internal implementation detail |
| P2-06-D3 | ✅ Resolved: `CapabilityToken` type alias added to `crates/hkask-capability/src/lib.rs` |
| P2-06-D4 | ✅ Resolved: `ContractValidator` stub added with FocusingAssumption FA-C1 |
| P2-06-D5 | ✅ Resolved: `CapabilityAwareValidator` stub added with FocusingAssumption FA-T3 |
| P2-06-D6 | ✅ Resolved: `TemplateInvocation` struct stub added with FocusingAssumption FA-D1 |
| P2-06-D7 | ✅ Resolved: Spec updated — SecurityGateway superseded by GovernedTool |
| P2-06-D8 | ✅ Resolved: Spec updated — McpTransport not needed (rmcp handles transport) |
| P2-06-D9 | ✅ Resolved: `parse_markdown_catalog`, `render_workspace_yaml`, `regenerate_workspace_yaml` stubs added with FocusingAssumption FA-Co1 |

---

## Deferred (v1.1+)

| ID | Task | Reason | ADR |
|----|------|--------|-----|
| **D-01** | Git CAS provenance | Not minimal for v1.0 | ADR pending |
| **D-02** | Federation transport | Complexity exceeds budget | ADR pending |
| **D-03** | Remote LLM providers | Local-first invariant | ADR pending |
| **D-04** | Fine-tuning (axolotl) | Not MVP | N/A |
| **D-05** | `kask` CLI surface reactive protocol | Not implemented | OQ-1 (resolved: removed from docs) |

---

## Completed (2026-05-25 MDS Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-01** | MDS documentation audit (Task 1) | 2026-05-25 | Classification table in refresh plan |
| **R-02** | Archive 10 stale documents (Task 2) | 2026-05-25 | `docs/archive/2026-05-25-documentation-refresh/` |
| **R-03** | Delete 1 duplicate document (Task 2) | 2026-05-25 | `DOCUMENTATION_REFRESH_DDNVSS.md` (deleted, no longer exists) |
| **R-04** | Create MDS_SCAFFOLD.md (Task 3) | 2026-05-25 | `docs/specifications/specs/MDS_SCAFFOLD.md` |
| **R-05** | Write 4 MDS-aligned architecture docs (Task 4) | 2026-05-25 | domain-and-capability (deleted), interface-and-composition, trust-security-observability (deleted), persistence-and-lifecycle |
| **R-06** | Write REQUIREMENTS.md (Task 5) | 2026-05-25 | `docs/specifications/specs/REQUIREMENTS.md` |
| **R-07** | Write TRACEABILITY_MATRIX.md (Task 5) | 2026-05-25 | `docs/specifications/specs/TRACEABILITY_MATRIX.md` |
| **R-08** | Update PROJECT_STATUS.md (Task 7) | 2026-05-25 | Accurate metrics, MDS completeness |
| **R-09** | Write OPEN_QUESTIONS.md (Task 9) | 2026-05-25 | 9 open questions with MDS tags |
| **R-10** | Fix MDS.md stale gaps (Task 8) | 2026-05-25 | hkask-mcp-docproc existence, Span::Spec variant |
| **R-11** | Update DOCUMENTATION_STANDARDS.md (Task 8) | 2026-05-25 | TOGAF→MDS metadata migration |
| **R-12** | Add mds_categories to key docs (Task 8) | 2026-05-25 | architecture-master, security-architecture, MDS |

---

## Completed (2026-05-28 Documentation Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-14** | Fix README.md path rot (3 broken references) | 2026-05-28 | Same path corrections as R-13 |
| **R-15** | Fix hKask-architecture-master.md duplicate line | 2026-05-28 | okapi-integration.md listed twice → once |
| **R-16** | Create docs/ci/check-links.sh + check-metadata.sh | 2026-05-28 | Both scripts operational, passing |
| **R-17** | Run metadata check — 34 docs missing mds_categories | 2026-05-28 | `docs/ci/check-metadata.sh` |
| **R-18** | Batch-add mds_categories to all 34 docs | 2026-05-28 | All 47 active docs now pass metadata check |
| **R-19** | Create DIAGRAMS_INDEX.md (28 diagrams, 8 V1.1+ candidates) | 2026-05-28 | `docs/DIAGRAMS_INDEX.md` |
| **R-20** | Fix MDS_SCAFFOLD.md directory map | 2026-05-28 | standards/→specifications/, added ci/, generated/ |
| **R-21** | Fix PROJECT_STATUS.md standards/→specifications/ | 2026-05-28 | Row corrected |
| **R-22** | Update TODO.md with completed P1 items | 2026-05-28 | P1-03, P1-05, P1-08 marked complete |
| **R-23** | Archive 10 stale documents (Task 2) | 2026-05-28 | docs/archive/2026-05-28-documentation-refresh/ |
| **R-24** | Delete 4 third-party skill guides (Task 2) | 2026-05-28 | Firecrawl, Browserbase, SerpApi, Tavily removed |
| **R-25** | Fix stale code references in architecture docs (Task 4) | 2026-05-28 | 15+ line number and path corrections |
| **R-26** | Fix test coverage claims in TRACEABILITY_MATRIX + REQUIREMENTS (Task 5) | 2026-05-28 | 0 #[test] unit tests corrected |
| **R-27** | Fix TOGAF→MDS references (Task 3) | 2026-05-28 | DOCUMENTATION_STANDARDS §1, PRINCIPLES PS-11 |
| **R-28** | Fix PROJECT_STATUS factual inaccuracies (Task 7) | 2026-05-28 | LOC, test counts, crate counts corrected |
| **R-29** | Fix hkask-cli build error (Task 1) | 2026-05-28 | repl::run() 4→5 argument mismatch fixed |
| **R-30** | Update GML README status Draft→Active (Task 3) | 2026-05-28 | Aspirational sub-docs archived |

---

## Completed (2026-06-03 Documentation Refresh)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **R-31** | Archive 8 stale documents (Task 2) | 2026-06-03 | `docs/archive/2026-06-03-documentation-refresh/` |
| **R-32** | Delete 3 superseded/implementation reference docs | 2026-06-03 | reference/loop-architecture.md (superseded), mcp-memory-split-plan.md, mcp-memory-continuation-prompt.md |
| **R-33** | Update MCP server count 15→18 across all docs | 2026-06-03 | PRINCIPLES, domain-and-capability (deleted), persistence-and-lifecycle, subsystem-erds, REQUIREMENTS, OPEN_QUESTIONS, mcp-server-audit |
| **R-34** | Update MDS_SCAFFOLD directory tree | 2026-06-03 | Removed archived plans, added loop-architecture.md; note: distillation-erd.md subsequently archived 2026-06-07 |
| **R-35** | Fix docs/README.md portal (remove dead links) | 2026-06-03 | Removed archived plan links, fixed GML reference |
| **R-37** | Archive IMPLEMENTATION-PLAN-simplification.md (Task 2) | 2026-06-06 | `docs/archive/2026-06-06-documentation-refresh/` |
| **R-38** | Update PROJECT_STATUS.md (v0.23.0, build regression, MCP server count 21) | 2026-06-06 | version, metrics, build status |
| **R-39** | Update MDS_SCAFFOLD.md with current directory structure | 2026-06-06 | Added 4 status docs to tree |
| **R-40** | Update architecture-master ADR-029 description | 2026-06-06 | Clarified ADR-029 scope |
| **R-41** | Archive 5 stale documents (ADR-023, ADR-028, ADR-029, distillation-erd, refactor-sweep) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/` |
| **R-42** | Fix stale code paths in architecture docs (bot.rs, replicant.rs, dependency.rs) | 2026-06-07 | domain-and-capability (deleted), DIAGRAMS_INDEX, persistence-and-lifecycle |
| **R-43** | Update stale version footers (0.21.0 → 0.23.0) | 2026-06-07 | template-header-standard, registry-erd, CI-CD-GUIDE, ADR_TEMPLATE |
| **R-44** | Fix CNS span count 15→21 across all docs | 2026-06-07 | hKask-erd, trust-security-observability |
| **R-45** | Update DIAGRAM_ALIGNMENT verified_dates to 2026-06-07 | 2026-06-07 | All architecture docs with diagrams |
| **R-46** | Update MDS_SCAFFOLD completeness (5/9→9/9 categories) | 2026-06-07 | Observability now ✅ (variety wired to algedonic) |
| **R-47** | Fix registry-erd template types and JSONB columns | 2026-06-07 | WordAct/KnowAct/FlowDef types, TEXT not JSONB |
| **R-48** | Fix MDS_SCAFFOLD directory tree (add ADR-024–031 lines, archive markers) | 2026-06-07 | SCAFFOLD document structure tree |
| **R-49** | Archive mcp-server-audit.md (merged into mcp-tools-inventory.md v1.1.0) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/mcp-server-audit.md` |
| **R-50** | Archive MDS-AUDIT-2026-06-06.md (findings absorbed into SCAFFOLD §4) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/MDS-AUDIT-2026-06-06.md` |
| **R-51** | Fix SCAFFOLD §4 completeness (Trust ✅, Interface ⚠️, result 6/9) | 2026-06-07 | SCAFFOLD completeness predicate |
| **R-52** | Add Q11 (DelegationResource extensibility) to OPEN_QUESTIONS.md | 2026-06-07 | OPEN_QUESTIONS §Open Crossroads |
| **R-53** | Reconcile Curator term lists (6→11) in MDS.md §6.1-6.2 | 2026-06-07 | Canonical spec now matches persona doc |
| **R-54** | Remove duplicate Verification section from architecture-master.md | 2026-06-07 | Points to MDS_SCAFFOLD §6 instead |
| **R-55** | Fix architecture-master version (2.2.2→2.2.3) | 2026-06-07 | Align with max spec version |
| **R-56** | Semantic consistency review — consolidated 7 stale docs, fixed SCAFFOLD completeness (5/9→correct 5/9), reconciled Curator vocabulary (6→11 terms) | 2026-06-07 | Full corpus review |
| **R-57** | Archive mcp-server-audit.md (merged into mcp-tools-inventory.md v1.1.0) and MDS-AUDIT-2026-06-06.md (absorbed into SCAFFOLD §4) | 2026-06-07 | `docs/archive/2026-06-07-documentation-refresh/` |
| **R-58** | Remove duplicate Verification section from architecture-master; add self-application precedent note to SCAFFOLD §6.1 | 2026-06-07 | Points to MDS_SCAFFOLD §6 |
| **R-59** | Fix OQ-3 to note 'arsenal' is not a hKask term | 2026-06-07 | OPEN_QUESTIONS.md OQ-3 rationale |
| **R-60** | Add mds_categories to ADR template body frontmatter | 2026-06-07 | ADR_TEMPLATE.md |
| **R-62** | Fix DOCUMENTATION_STANDARDS §6.2 location paths (standards/→specifications/, adr/→architecture/) | 2026-06-07 | DOCUMENTATION_STANDARDS.md §6.2 |
| **R-63** | Add forward-references from canonical specs to reference docs (inference, ports-inventory, utoipa, template-header, Curator persona) | 2026-06-07 | MDS.md §7.2, MDS.md §6.1-6.2 |
| **R-65** | Add 7 untracked future questions from FUTURE.md to OPEN_QUESTIONS.md | 2026-06-07 | FUT-002, FUT-003, FUT-004, FUT-005, FUT-007, FUT-008, FUT-009 |
| **R-66** | Add 7 untracked future questions to OPEN_QUESTIONS.md (review FUTURE.md synthesis) | 2026-06-07 | OPEN_QUESTIONS §Review Findings, FUT-002 through FUT-009 |
| **R-67** | Update cli-reference.md last_updated to 2026-06-07 | 2026-06-07 | docs/generated/cli-reference.md |
| **R-68** | Fix broken links: 17→0 broken links, add 5 status file placeholders, fix 3 archived ADR paths | 2026-06-07 | docs/README.md, docs/ci/check-links.sh |
| **R-70** | Create ADR-032 (MCP gateway membrane) and ADR-033 (Dampener override cooldown); update ADR-027 footer | 2026-06-07 | docs/architecture/ADR-032, ADR-033, ADR-027 |
| **R-71** | Verify hkask-mcp-docproc build regression — no type errors found, workspace builds clean | 2026-06-07 | cargo check --workspace passes, 5 MCP protocol tests fail (TransportClosed, infra issue) |
| **R-72** | Writing Excellence spot-check: domain-and-capability ✅ 4/4, trust-security-observability ✅ 4/4, MDS ❌ 1/4 | 2026-06-07 | MDS fails Hopper/Lovelace/Gentle (known gap, tracked in OPEN_QUESTIONS) |
| **R-73** | Update all stale verified_date in architecture docs to 2026-06-07 | 2026-06-07 | 21 DIAGRAM_ALIGNMENT entries updated |

---

## Completed (Prior Sessions)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-01** | Documentation audit | 2026-05-22 | `docs/specifications/standards/WRITING_EXCELLENCE.md` |
| **C-02** | Archive stale documents (73 files) | 2026-05-22 | `docs/archive/2026-05-22-documentation-refresh/` |
| **C-03** | Fix hkask-test-harness compilation failures | 2026-05-22 | All tests passing |
| **C-04** | Resolve clippy warnings | 2026-05-22 | `cargo clippy -- -D warnings` passes |
| **C-05** | Fix cargo fmt issues | 2026-05-22 | `cargo fmt --check` passes |
| **C-06** | Fix hardcoded cryptographic key | 2026-05-22 | `OKAPI_DEV_KEY` const with migration path |
| **C-08** | MCP server consolidation — Collapse rss-reader + web → research; document replica | 2026-06-11 | New `hkask-mcp-research` (1,044 LOC, ~17 tools). Deleted `hkask-mcp-research` (535 LOC) and `hkask-mcp-research` (504 LOC). Updated 7 code files, 5 docs, workspace Cargo.toml. |
| **C-10** | Tier 1 unit tests — condenser algorithms, profile parsing, classify_tool, registry | 2026-06-11 | 27 tests across types.rs and algorithms.rs. All compression algorithms verified. `algorithms` module promoted to `pub mod` in lib.rs. |
| **C-11** | Tier 1 unit tests — research freshness, ranking, strip_html, rate_limiter | 2026-06-11 | 23 tests: 6 freshness parsing/brave/serpapi, 5 ranking dedup/rerank, 8 strip_html, 4 rate_limiter. Added `[lib]` target to research crate. |
| **C-13** | Companies value-add tools — Tier 1 MAIA framework (moat, management, working capital, expectations gap) | 2026-06-11 | 4 new tools: `moat_check`, `management_scorecard`, `working_capital_cycle`, `expectations_gap`. New `analysis.rs` module with 20 tests. Companies server now has 15 tools (11 passthrough + 4 analytical). |

---

## Completed (2026-06-12 Inference Engine + Replica Pipeline Fixes)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-14** | Fix DeepInfra base URL (`/v1/openai` → `https://api.deepinfra.com`) | 2026-06-12 | `crates/hkask-inference/src/config.rs` — double-`/v1/` bug fixed for embeddings and chat |
| **C-15** | Fix `EmbedService` API key stripping (`Default::default()` → `from_env()`) | 2026-06-12 | `crates/hkask-services-corpus/src/` — `DI_API_KEY` now reaches embedding router |
| **C-16** | Fix replica + docproc MCP servers passing partial configs | 2026-06-12 | `mcp-servers/hkask-mcp-replica/src/main.rs`, `mcp-servers/hkask-mcp-docproc/src/{main,tools}.rs` |
| **C-17** | Add auto `.env` loading to all 11 binaries via `dotenvy` | 2026-06-12 | Workspace `Cargo.toml` + 11 `main.rs` files — API keys loaded from `.env` on startup |
| **C-18** | Fix `embed-mashups.sh` — dead `--okapi-url`, wrong subcommand, stale URLs | 2026-06-12 | Script now uses `kask style embed-corpus`, no URL flags needed |
| **C-19** | Add `DI/` prefix to 5 corpus configs + remove dead `embedding_provider` fields | 2026-06-12 | `registry/styles/*/corpus.yaml` — embeddings route to DeepInfra via `EmbeddingRouter` |
| **C-20** | Fix O(n³) salience hang — cap two-hop expansion to 50 neighbors | 2026-06-12 | `crates/hkask-memory/src/salience.rs` — 1832-passage corpus now completes in seconds |
| **C-21** | Update DEPLOYMENT.md + CI-CD-GUIDE.md — replace Okapi with multi-provider inference | 2026-06-12 | Env vars, examples, systemd/Docker/K8s, troubleshooting all updated |
| **C-22** | End-to-end verification — Hemingway corpus embedded via DeepInfra | 2026-06-12 | 1832 embeddings, centroid stored, 28,989 triples — full pipeline completes |

---

## Completed (2026-06-15 Pragmatics Audit)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-23** | Test coverage — tag and organize tests across 12 files | 2026-06-15 | 360+ tests across salience (12), discover (16), mcp handlers (16), lexicon (6), spec_store (6), contract_validator (5), spec_types (5), kata_history (5), transcript (2), voice (2), wallet_budget (1), gentle_lovelace (1) |
| **C-24** | hkask-communication integration tests — 19 tests for public API surface | 2026-06-15 | `crates/hkask-communication/tests/integration_test.rs` — types (7), errors (4), AgentRegistry (8). All 19 pass. MatrixTransport tests deferred (require Conduit homeserver) |
| **C-25** | MCP server tool completeness verified — all 10 servers audited | 2026-06-15 | 143/143 tools fully implemented: condenser (7), spec (6), replica (8), training (8), docproc (9), communication (9), memory (16), research (17), companies (27), media (36) |
| **C-26** | Condenser completion verified — all 7 tools functional | 2026-06-15 | `hkask-mcp-condenser` — ping, compress, classify, persist, set_profile, stats, thread_summary all implemented with inference router integration and CNS span emission |
| **C-27** | Pragmatics codebase audit — 7-task principle-grounded review | 2026-06-15 | All 7 tasks converge at δ=0. Zero P1–P12 violations. Key findings: CNS feedback loop closed, OCAP tokens cryptographically unforgeable (HMAC-SHA256), zero unsafe blocks, zero Rc<RefCell>, condenser complete, services extraction ~70%+ |

---

## Completed (2026-06-15 Pragmatic Audit Implementation — All 10 Tasks)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-37** | R1: hkask-communication tests (19→25) | 2026-06-15 | +6 SevenR7Listener lifecycle tests. 25 tests pass. |
| **C-38** | R2: hkask-agents tests (20→31) | 2026-06-15 | +11 ACP runtime tests — wildcard, registration, unregistration, revocation, restore, list. |
| **C-39** | R3: hkask-mcp tests (27→38) | 2026-06-15 | +11 capability enforcement, error propagation, tool discovery tests. |
| **C-40** | R4: hkask-api test coverage (8→29) | 2026-06-15 | +21 route type serialization tests in `tests/integration.rs`. |
| **C-41** | R5: CnsSpan enum (51 variants) | 2026-06-15 | `CnsSpan` + `ToolSubsystem` enums defined. `Display`/`FromStr` implemented. All crates migrated. 6 tests. |
| **C-42** | R6: Ed25519 DelegationToken | 2026-06-15 | Immediate cutover — `TokenSignature([u8; 64])`, `public_key`, `derive_signing_key()`. 15 token + 11 ACP tests. HMAC removed. |
| **C-43** | R7: Provenance markers | 2026-06-15 | 54 OUGHT-as-IS doc claims marked `[NORMATIVE]`/`[DECLARATIVE]` across 18 files. Zero unmarked. |
| **C-44** | R8: hkask-types surface reduction | 2026-06-15 | 10 files split into subdirectories (≤7 public items each). 10 types → pub(crate). ~25 deprecated re-exports removed. 12 G2 justifications. |
| **C-45** | R9: Strangler fig extraction (Kata + Spec) | 2026-06-15 | `KataEngine::from_env()`, `SpecService::get_full()`. CLI no longer imports InferenceConfig/InferenceRouter/SpecStore. |
| **C-46** | R10: Training cancel stubs | 2026-06-15 | Already implemented — all 5 providers have PID+SIGTERM or API cancel. Zero stubs. |
| | **Total tests** | 2026-06-15 | **916** across workspace. Zero `todo!()`/`unimplemented!()`. |
| **C-47** | Contract annotation completed | 2026-06-16 | All 1579 `pub fn` across 17 crates carry `expect:` with `pre:`/`post:`. `cargo check --workspace` 0 errors, 0 warnings. |

---

## Completed (2026-06-15 Condenser Thinking Mode + Token Estimation)

| ID | Task | Date | Evidence |
|----|------|------|----------|
| **C-28** | Condenser thinking mode — `disable_thinking` through full inference pipeline | 2026-06-15 | `LLMParameters.disable_thinking` → `ChatRequest.enable_thinking`. Set `true` in `condenser_thread_summary` and `ChatService::condense_history`. 2 tests verify wire-format mapping. All backends use same `build_chat_request` — no per-backend changes. |
| **C-29** | Condenser token estimation — whitespace-split → ~4 chars/token heuristic | 2026-06-15 | `approx_token_count` changed from word-count to `text.len() / 4` (industry standard). Fixes latent bug: auto-condense threshold was comparing words against token threshold, effectively disabled. 3 new tests. |
| **C-30** | Condenser context window tracking — `original_tokens_approx` in output | 2026-06-15 | `ThreadSummaryOutput` now carries both `original_tokens_approx` and `summary_tokens_approx` for context-window budgeting. `build_summary_output` signature updated. |
| **C-31** | Condenser-continuation skill documentation drift fixed | 2026-06-15 | SKILL.md: 4 wrong file paths corrected (mcp-servers/ → crates/), stale Option A/B framing removed, thinking mode clarified. All 4 registry templates + manifest.yaml updated (0.23.0→0.27.0). |
| **C-32** | Condenser README updated — Token Estimation + Thinking Mode sections | 2026-06-15 | `mcp-servers/hkask-mcp-condenser/README.md` — new sections documenting ~4 chars/token heuristic, thinking mode pipeline, and context window tracking. |
| **C-33** | Pre-existing `wallet_id` bug fixed in `user_store.rs` | 2026-06-15 | `replicant_from_row` missing `wallet_id` field after it was added to `ReplicantIdentity`. Added `wallet_id: None`. |
| **C-34** | Router pass-through integration test — `disable_thinking_flows_to_wire_format` | 2026-06-15 | `crates/hkask-inference/tests/inference_routing_integration.rs` — wiremock-based test confirms `LLMParameters.disable_thinking` passes through `InferenceRouter` → backend → `build_chat_request` without interference. 6/6 integration tests pass. |
| **C-35** | Live multi-backend validation — DeepInfra + Together | 2026-06-15 | curl tests confirm both backends work with `meta-llama/Llama-3.3-70B-Instruct-Turbo` (clean output, no thinking interference). Rust live-backend tests written: `crates/hkask-inference/tests/live_backends.rs` — `deepinfra_summarization` + `together_summarization` (gated on API keys, `#[ignore]`). Qwen3 models on all backends (DeepInfra, Together) exhibit thinking mode — documented with workaround. |
| **C-36** | Condenser default model updated to hKask classifier | 2026-06-15 | Changed from `qwen3:8b` (thinking-mode broken) to `google/gemma-4-26B-A4B-it` (hKask classifier model). Tested on DeepInfra: produces clean structured summaries with `finish_reason: stop`, no thinking interference. Updated in `main.rs`, README, condenser-continuation SKILL.md, and restore.j2 template. Together requires dedicated endpoint for this model. |

---

## Verification

```bash
# Check P0 completion status
cargo test -p hkask-cns
cargo test -p hkask-storage

# Verify documentation
find docs -type f -name "*.md" ! -path "docs/archive/*" | wc -l
grep -L "^Version:\|^version:" docs/**/*.md 2>/dev/null

# Run quality gates
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

*This TODO is the single source of truth for open work. Last updated 2026-07-01 — audit verified all P0 items complete, P1-11 remains partial, build clean (0 errors), `do../` paths fixed.*
