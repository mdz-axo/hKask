# Architecture Improvement Plan — Crate Elimination + MCP Server Deepening

**Date:** 2026-07-14  
**Skills applied:** metacognition, improve-codebase-architecture, kata-improvement, grill-me, pragmatic-laziness, essentialist, task-breakdown

---

## Adversarial Review (6-skill synthesis)

### Essentialist: Does `hkask-corpus-ingest` survive the deletion test?

**G1 — EXIST:**
- `purge-qa`: If deleted, caller needs `SemanticMemory::open()` + `purge_by_prefix()` + `query_by_attribute()` + `delete_h_mem()` (~60 lines). Behavior IS lost on deletion → survives G1. But this is a DB operation, not a user-facing CLI — it belongs as an MCP tool on docproc, not as a standalone binary.
- `ocr`: If deleted, caller uses `docproc_convert` with `force_ocr: true` (already exists). No behavior lost → **FAILS G1.** Pure pass-through to `hkask-services-corpus::ocr_pdf_bytes`.
- The crate itself: If deleted, only `purge-qa` is lost. That can move to docproc. → **The crate should be DELETED.**

**G2 — SURFACE:** After moving purge-qa to docproc, corpus-ingest has 0 public items. → **PASS (vacuously).**

**G3 — CONTRACT:** The crate is a pass-through wrapper around `hkask-memory` and `hkask-services-corpus`. No genuine abstraction. → **FAILS G3.**

**Essentialist verdict: DELETE the crate. Move `purge-qa` to docproc as `docproc_purge_qa` MCP tool.**

### Pragmatic-laziness: Path of least action

**Syntax layer:** corpus-ingest is a 213-line binary crate with 2 deps, 2 subcommands, no lib target. No other crate depends on it.

**Semantics layer:** `purge-qa` opens SemanticMemory, purges embeddings + h_mems by prefix. `ocr` calls `ocr_pdf_bytes()` from services-corpus. Both are thin.

**Pragmatics layer:** The crate exists because it used to be the pipeline binary. The pipeline moved to MCP tools. The crate is vestigial — it exists for historical reasons, not because it serves a current need.

**Effort hotspot:** Maintaining a separate crate + Cargo.toml + binary target for 2 utility commands that could be 1 MCP tool + 1 existing MCP tool (`docproc_convert` with `force_ocr`).

**Brachistochrone check:** Deleting the crate and moving purge-qa to docproc reduces total system action — no separate compile target, no separate binary, no subprocess calls from replica server.

**Stationary action verdict:** The crate must be deleted. Keeping it adds action (maintenance, build time, subprocess calls) without adding value.

### Improve-codebase-architecture: Friction points

| # | Friction | Signal | Severity |
|---|---------|--------|----------|
| F1 | `hkask-corpus-ingest` is a vestigial crate — 213 lines, 2 utilities, no dependents | Shallow module, pass-through | High |
| F2 | Replica `replica_pipeline_run` step executor still calls `std::process::Command::new("corpus-ingest")` for `corpus_*` tools | Pass-through subprocess | High |
| F3 | docproc `lib.rs` is 1457 lines — 14 request structs + template engine + JSON utils + server struct + OCR + helpers + entry + tests | God Object, shallow module | Medium |
| F4 | Request structs in lib.rs not co-located with tool implementations | Missing locality | Medium |
| F5 | Replica `lib.rs` is 1321 lines with inline `ReplicaStepExecutor` struct (90 lines) | Missing locality | Low |
| F6 | `hkask-services-corpus` (3624 lines) merges discovery + embedding — different concerns | Shallow module boundary | Low |

### Grill-me: Key interrogation findings

**Q: Why does corpus-ingest still exist?**  
A: Historical — it was the pipeline binary. Pipeline moved to MCP tools. Only purge-qa and ocr remain.

**Q: Should purge-qa be CLI or MCP tool?**  
A: MCP tool. It's a DB operation. docproc already has SemanticMemory access. No reason for a separate binary.

**Q: What about the replica pipeline_run subprocess calls?**  
A: Pipeline YAML v7.0 uses `docproc_*` tools. The `corpus_*` match arms in the step executor are dead code for new pipelines. They should return deprecation messages, matching the standalone tool behavior.

**Q: What about request struct co-location — is it worth the churn?**  
A: Yes — it shrinks lib.rs by ~400 lines and co-locates each struct with its tool. The risk is mechanical (import paths), not conceptual.

### Metacognition: Assessment

**Target condition:** No corpus-ingest crate. `docproc_purge_qa` MCP tool. Replica pipeline_run returns deprecation for corpus_* tools. lib.rs < 1000 lines. All builds + tests pass.

**Actual condition:** corpus-ingest is 213 lines. Replica pipeline_run has 3 subprocess call paths. lib.rs is 1457 lines. All builds pass.

**Progress:** 0.60 — Phases 0-4 done, crate elimination + struct co-location remain.

**Obstacles:**
1. (dependency_block, high) Must move purge-qa to docproc BEFORE deleting corpus-ingest
2. (complexity, medium) Request struct co-location touches 4 tool files + lib.rs
3. (tool_limitation, low) Replica pipeline_run step executor cleanup is straightforward

### Kata-improvement: PDCA experiment

**Next experiment:** Move `purge-qa` to docproc as `docproc_purge_qa` MCP tool. Delete corpus-ingest crate. Update replica pipeline_run. Verify build + tests.

**Prediction:** Build will pass. Tests will pass. lib.rs will shrink by ~400 lines when structs are co-located.

**Success criterion:** `cargo build` + `cargo test` pass for all affected crates. No `corpus-ingest` references remain.

---

## Tasks

### Phase A: Eliminate corpus-ingest crate (foundation — fail fast)

**Task A.1: Create `docproc_purge_qa` MCP tool**
- **Slice:** `mcp-purge-qa`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/tools/storage.rs`, `mcp-servers/hkask-mcp-docproc/src/lib.rs` (request struct)
- Migrate `run_purge_qa` logic from `crates/hkask-corpus-ingest/src/main.rs` (lines 75-213) to a new `docproc_purge_qa` tool in storage.rs
- Add `PurgeQaRequest` struct to lib.rs: `prefix: String`, `db_path: String`, `passphrase: String`
- Tool opens SemanticMemory, purges embeddings by prefix, purges h_mems by attribute/prefix
- Acceptance:
  - `cargo build -p hkask-mcp-docproc` succeeds
  - `docproc_purge_qa` tool registered in combined_router
  - Tool has docproc server's `record_experience` call
- **Dependencies:** None
- **Scope:** S

**Task A.2: Delete `hkask-corpus-ingest` crate**
- **Slice:** `delete-corpus-ingest`
- **Files:** Delete `crates/hkask-corpus-ingest/` directory, remove from workspace `Cargo.toml`
- Verify no other crate depends on `hkask-corpus-ingest` (confirmed: no dependents)
- Acceptance:
  - `cargo build` succeeds without `hkask-corpus-ingest`
  - No references to `corpus-ingest` in workspace
- **Dependencies:** Task A.1
- **Scope:** XS

**Checkpoint A:** No corpus-ingest crate. purge-qa is a docproc MCP tool. Build passes.

### Phase B: Clean up replica server (core — remove remaining subprocess calls)

**Task B.1: Update replica pipeline_run step executor**
- **Slice:** `replica-pipeline-cleanup`
- **Files:** `mcp-servers/hkask-mcp-replica/src/lib.rs`
- Replace `corpus_embed`, `corpus_build_prompts`, `corpus_ingest_qa` match arms in `ReplicaStepExecutor::execute` with deprecation messages (same pattern as the standalone tools)
- Remove `std::process::Command::new("corpus-ingest")` — no subprocess calls remain
- Acceptance:
  - No `std::process::Command::new("corpus-ingest")` in replica server
  - `cargo build -p hkask-mcp-replica` succeeds
  - `cargo test -p hkask-mcp-replica` passes
- **Dependencies:** Task A.2 (corpus-ingest must be deleted first)
- **Scope:** S

**Checkpoint B:** No subprocess calls in replica server. All corpus_* tools return deprecation.

### Phase C: Co-locate request structs (deepening — locality improvement)

**Task C.1: Move document tool request structs**
- **Slice:** `co-locate-document-structs`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/tools/document.rs`, `mcp-servers/hkask-mcp-docproc/src/lib.rs`
- Move `ConvertRequest`, `OcrRequest`, `ChunkRequest` from lib.rs to document.rs
- Move `default_true()` helper
- Update imports in document.rs (already has `use crate::*`)
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds
- **Dependencies:** None
- **Scope:** S

**Task C.2: Move semantic tool request structs**
- **Slice:** `co-locate-semantic-structs`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/tools/semantic.rs`, `mcp-servers/hkask-mcp-docproc/src/lib.rs`
- Move `GenerateQaRequest`, `BatchQaPrompt`, `GenerateQaBatchRequest`, `ExtractTriplesRequest`, `EmbedRequest` from lib.rs to semantic.rs
- Move `default_batch_concurrency()`, `default_owner()` (shared)
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds
- **Dependencies:** None
- **Scope:** S

**Task C.3: Move corpus + tagging tool request structs**
- **Slice:** `co-locate-corpus-structs`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/tools/corpus.rs`, `mcp-servers/hkask-mcp-docproc/src/tools/tagging/ops.rs`, `mcp-servers/hkask-mcp-docproc/src/lib.rs`
- Move `DedupChunksRequest`, `ConsolidateChunksRequest`, `TagChunksRequest`, `BuildPromptsRequest`, `IngestQaRequest`, `PurgeQaRequest` from lib.rs to their respective tool files
- Move all `default_*()` helpers for these structs
- Keep `DEFAULT_OWNER` const + `default_owner()` in lib.rs (shared across tools)
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds, lib.rs shrinks by ~400 lines
- **Dependencies:** Task C.1, C.2 (to avoid merge conflicts)
- **Scope:** M

**Task C.4: Move storage tool request structs**
- **Slice:** `co-locate-storage-structs`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/tools/storage.rs`, `mcp-servers/hkask-mcp-docproc/src/lib.rs`
- Move `CacheRequest`, `QueryRequest`, `ClearIndexRequest` from lib.rs to storage.rs
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds
- **Dependencies:** None
- **Scope:** XS

**Checkpoint C:** All request structs co-located with their tools. lib.rs < 1000 lines. Build passes.

### Phase D: Extract template engine module (polish — locality)

**Task D.1: Extract template rendering to its own module**
- **Slice:** `extract-template-module`
- **Files:** `mcp-servers/hkask-mcp-docproc/src/template.rs` (new), `mcp-servers/hkask-mcp-docproc/src/lib.rs`
- Move `TEMPLATE_CACHE`, `render_docproc_template` to `template.rs`
- Add `pub mod template;` to lib.rs
- Update tool files to use `crate::template::render_docproc_template`
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds, template logic isolated
- **Dependencies:** Task C.3 (structs moved first to avoid conflicts)
- **Scope:** S

**Checkpoint D:** Template engine is its own module. lib.rs < 900 lines.

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Moving structs breaks imports | Medium | `use crate::*` glob imports already in tool files |
| Deleting corpus-ingest breaks something unexpected | Low | No dependents confirmed |
| Replica pipeline_run old manifests break | Low | Deprecation messages guide users to docproc tools |
| Struct co-location merge conflicts | Low | Do sequentially: C.1 → C.2 → C.3 → C.4 |

## Open Questions

| # | Question | Recommendation |
|---|----------|----------------|
| Q1 | Should `hkask-services-corpus` be split into `hkask-services-discovery` + `hkask-services-embed`? | Defer — 3624 lines is manageable, splitting adds crate overhead |
| Q2 | Should replica server types all move to types.rs? | Defer — partial extraction already done, remaining types are small |
| Q3 | Should `docproc_purge_qa` also purge by `owner` persona? | Yes — add optional `owner` param for targeted purging |