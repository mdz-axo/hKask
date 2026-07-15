# Migration Plan — Corpus Pipeline MCP Migration + Architecture Improvement

**Date:** 2026-07-14  
**Creator:** Zed Agent (glm-5.2:cloud)  
**Skills applied:** zoom-out, metacognition, essentialist, grill-me, task-breakdown, improve-codebase-architecture, idiomatic-rust

---

## Overview

The MCP tools ARE the product. The corpus pipeline is the test case. An agent wrote custom embed/classify/decimate code into the CLI binary instead of using MCP tools — bypassing ContentGuard, using wrong models, storing h_mems with wrong entity association, and never wiring h_mems into build-prompts as a knowledge graph. This migration fixes all MCP tools, creates missing ones, deletes CLI pipeline code, runs the full pipeline through MCP tools with approved models, and trains with Axolotl + PiSSA.

**Additionally**, an `improve-codebase-architecture` + `idiomatic-rust` pass on both MCP servers (docproc + replica) to address structural problems that cause friction and make the codebase harder to extend than it should be.

---

## Architecture Analysis (improve-codebase-architecture + idiomatic-rust)

### Friction Points Identified

| # | Friction | Module | Severity | Type |
|---|---------|--------|----------|------|
| A1 | `lib.rs` is 1427 lines — God Object containing server struct, OCR pipeline, 14 request structs, template engine, JSON utils, helpers, entry point, tests | docproc/lib.rs | High | Shallow module |
| A2 | Replica server `corpus_*` tools are subprocess wrappers — pure pass-through to `corpus-ingest` CLI binary, violating P5 no-pass-through-abstractions | replica/lib.rs | High | Pass-through |
| A3 | `corpus_salience` on replica server calls non-existent CLI subcommand — dead code | replica/lib.rs | Medium | Dead code |
| A4 | Duplicated helpers: `embedding_dim()`, `normalize_in_place()`, `read_tagged_chunks()`, `cosine_similarity()` across docproc and CLI binary | both | Medium | Duplication |
| A5 | Template cache leaks memory via `Box::leak` on every call — key is leaked even when template already cached | docproc/lib.rs | Medium | Memory leak |
| A6 | 14 request structs in lib.rs instead of co-located with their tool implementations | docproc/lib.rs | Medium | Misplaced responsibility |
| A7 | `println!`/`eprintln!` in library code — interferes with MCP protocol, should use `tracing` | docproc tools | Medium | Non-idiomatic |
| A8 | Multiple `default_*_owner()` functions all return `"john-brooks"` — should be one const | docproc/lib.rs | Low | Duplication |
| A9 | `configured_qa_model` returns `Option<String>` but `classifier_model` returns `String` — type inconsistency | docproc/semantic.rs | Low | Type mismatch |
| A10 | Pipeline YAML references `corpus_ingest_embed`, `corpus_ingest_build_prompts`, `corpus_ingest_generate_qa`, `corpus_ingest_ingest_qa` — non-existent tools | corpus/pipeline YAML | High | Broken config |
| A11 | Pipeline YAML has wrong models (`Qwen3-30B-A3B` instead of `Qwen3.6-35B-A3B` and `GLM-5.2`) | corpus/pipeline YAML | High | Wrong config |
| A12 | `docproc_extract_triples` template name mismatch (`"extract-h_mems"` vs file `extract-hmems.j2`) | docproc/semantic.rs | Medium | Silent degradation |
| A13 | `docproc_tag_chunks` uses wrong model (`configured_qa_model` → `HKASK_QA_MODEL` instead of `classifier_model`) | docproc/tagging/ops.rs | High | Wrong model |
| A14 | `docproc_tag_chunks` stores h_mems with `owner="corpus"` instead of replicant persona | docproc/tagging/ops.rs | High | Wrong owner |
| A15 | `build-prompts.j2` not registered in `registry/templates/docproc/manifest.yaml` | registry | Low | Incomplete manifest |

### Template & FlowDef Opportunities

The codebase already has a Jinja2 template system (`render_docproc_template`) and a pipeline YAML manifest system. These should be used more systematically:

**Current template usage:**
| Tool | Template | Status |
|------|----------|--------|
| `docproc_convert` | None (text extraction, not LLM) | ✅ Correct |
| `docproc_chunk` | None (token splitting, not LLM) | ✅ Correct |
| `docproc_tag_chunks` | `tag-chunks.j2` | ✅ Works |
| `docproc_extract_triples` | `extract-hmems.j2` | ⚠️ Name mismatch bug |
| `docproc_dedup_chunks` | None (pure algorithm) | ✅ Correct |
| `docproc_consolidate_chunks` | `consolidate-chunks.j2` | ✅ Works |
| `docproc_generate_qa` | `generate-qa.j2` | ✅ Works |
| `docproc_generate_qa_batch` | `generate-qa.j2` | ✅ Works |
| `docproc_build_prompts` | `build-prompts.j2` (created) | ⚠️ Needs manifest registration |
| `docproc_ingest_qa` | None (data processing, no LLM) | ✅ Correct |
| `docproc_query` | `rag-answer.j2` | ✅ Works |

**Template improvements needed:**
1. Fix extract-hmems template name in code → `"extract-hmems"` (already done in Phase 1)
2. Register `build-prompts.j2` in `registry/templates/docproc/manifest.yaml`
3. Remove `classify-chunks.j2` from manifest — it was the combined template that introduced bugs, replaced by separate tag-chunks + extract-hmems
4. Improve `build-prompts.j2` template — current version is basic; should match the quality of `generate-qa.j2` with system/task/output_contract structure and examples

**Pipeline YAML improvements needed:**
1. Replace all `corpus_ingest_*` tool references with `docproc_*` MCP tools
2. Replace wrong models with approved models
3. Split `embed_classify_decimate` into separate `docproc_embed`, `docproc_tag_chunks`, `docproc_extract_triples` steps (separate tools, separate templates — the combined `classify-chunks.j2` approach is what caused the bugs)
4. Add `owner: john-brooks` to all steps that store h_mems

### Idiomatic Rust Issues

| # | Issue | Fix |
|---|-------|-----|
| R1 | `Box::leak` on every `render_docproc_template` call | Check cache before allocating key; only leak on first load |
| R2 | `println!`/`eprintln!` in library code | Replace with `tracing::info!`/`tracing::warn!` |
| R3 | Multiple `default_*_owner()` returning same string | Single `const DEFAULT_OWNER: &str = "john-brooks"` |
| R4 | `configured_qa_model` returns `Option<String>`, `classifier_model` returns `String` | Both should return `String` (they always have a fallback) |
| R5 | `WebID::from_persona(owner.as_bytes())` repeated 6+ times | Extract `fn owner_webid(owner: &str) -> WebID` helper |
| R6 | `env.lock().unwrap()` in template cache | Use `unwrap_or_else(|e| e.into_inner())` for poison recovery |
| R7 | Request structs in lib.rs not co-located with tools | Move each struct to its tool module |

---

## Tasks

### Phase 0: Delete custom CLI code ✅ DONE

| Task | Status |
|------|--------|
| 0.1 Delete custom embed/classify/decimate code from main.rs | ✅ |
| 0.2 Remove minijinja dependency from Cargo.toml | ✅ |

### Phase 1: Fix existing MCP tools ✅ DONE

| Task | Status |
|------|--------|
| 1.1 Add `extract_json_from_response` thinking-mode utility | ✅ 4 tests pass |
| 1.2 Fix `docproc_tag_chunks` (model, thinking-mode, owner) | ✅ |
| 1.3 Fix `docproc_embed` (DB storage, optional params) | ✅ |
| 1.4 Fix `docproc_extract_triples` (DB, thinking-mode, retry, entity, template name) | ✅ |
| 1.5 Fix `docproc_generate_qa` + batch (thinking-mode) | ✅ |
| 1.6 Cache compiled templates | ✅ |

### Phase 2: Create missing MCP tools (IN PROGRESS)

**Task 2.1: Create `docproc_build_prompts` MCP tool** — ⚠️ Code written, needs fixes
- Uses `render_docproc_template("build-prompts", &vars)` ✅
- Has h_mem knowledge graph via `semantic.query_deduped(chunk_ref)` ✅
- Needs: improve `build-prompts.j2` template quality to match `generate-qa.j2` standard
- Needs: register in manifest.yaml
- Needs: fix unused `webid` variable warning
- Acceptance: Tool builds, template renders, h_mem KG section appears in prompts

**Task 2.2: Create `docproc_ingest_qa` MCP tool** — ⚠️ Code written, needs verification
- SemDeDup with kmeans clustering ✅
- QA parsing (flat + envelope formats) ✅
- h_mem storage with 5W1H dimension ✅
- QA embedding storage ✅
- Needs: build verification (was blocked by build_prompts build error)
- Acceptance: Tool builds, SemDeDup works, h_mems stored

**Task 2.3: Improve `build-prompts.j2` template**
- Add `<system>`, `<task>`, `<output_contract>` structure matching `generate-qa.j2` and `tag-chunks.j2`
- Add examples of good QA output
- Add quality checklist
- Register in `registry/templates/docproc/manifest.yaml`
- Acceptance: Template renders with all variables, matches quality standard

**Task 2.4: Clean up template manifest**
- Remove `classify-chunks.j2` from manifest (combined template that caused bugs)
- Add `build-prompts.j2` to manifest
- Acceptance: Manifest matches actual templates in use

**Checkpoint 2:** All pipeline steps have MCP tool equivalents. Build succeeds. Templates registered.

### Phase 3: Architecture improvement (NEW — from improve-codebase-architecture + idiomatic-rust)

**Task 3.1: Move request structs to co-located tool modules**
- Move `ConvertRequest`, `OcrRequest`, `ChunkRequest` → `tools/document.rs`
- Move `GenerateQaRequest`, `GenerateQaBatchRequest`, `BatchQaPrompt`, `ExtractTriplesRequest`, `EmbedRequest` → `tools/semantic.rs`
- Move `DedupChunksRequest`, `ConsolidateChunksRequest`, `TagChunksRequest`, `BuildPromptsRequest`, `IngestQaRequest` → `tools/corpus.rs`
- Move `CacheRequest`, `QueryRequest`, `ClearIndexRequest` → `tools/storage.rs`
- Keep shared default functions and `ExtractOutcome` in lib.rs
- Acceptance: `cargo build -p hkask-mcp-docproc` succeeds. lib.rs shrinks by ~400 lines.

**Task 3.2: Fix template cache memory leak**
- Check `env_guard.get_template(template_key).is_err()` BEFORE allocating `Box::leak`
- Only leak on first load of a new template name
- Acceptance: No memory leak on repeated calls with same template name

**Task 3.3: Replace `println!`/`eprintln!` with `tracing` in library code**
- Files: `tools/corpus.rs`, `tools/tagging/ops.rs`, `tools/semantic.rs`
- Replace `println!("...")` → `tracing::info!(target: "hkask.mcp.docproc", "...")`
- Replace `eprintln!("WARN...")` → `tracing::warn!(target: "hkask.mcp.docproc", "...")`
- Acceptance: No `println!` or `eprintln!` in library code (only in binary main.rs)

**Task 3.4: Consolidate owner defaults**
- Replace 5 `default_*_owner()` functions with single `const DEFAULT_OWNER: &str = "john-brooks"`
- Use `#[serde(default = "default_owner")]` with `fn default_owner() -> String { DEFAULT_OWNER.to_string() }`
- Acceptance: Single source of truth for owner default

**Task 3.5: Extract shared helpers**
- Move `embedding_dim()`, `normalize_in_place()`, `read_tagged_chunks()` to a shared location (lib.rs or new `tools/common.rs`)
- Extract `owner_webid(owner: &str) -> WebID` helper
- Acceptance: No duplicated helper functions across tool modules

**Task 3.6: Remove dead `corpus_salience` from replica server**
- Delete tool, request struct, pipeline_run references
- Acceptance: No `corpus_salience` symbol remains

**Task 3.7: Update replica server corpus tools to call docproc MCP tools**
- `corpus_embed` → delegate to `docproc_embed` (in-process, not subprocess)
- `corpus_build_prompts` → delegate to `docproc_build_prompts`
- `corpus_ingest_qa` → delegate to `docproc_ingest_qa`
- Remove all `std::process::Command::new("corpus-ingest")` calls
- Acceptance: No subprocess calls in replica server

**Task 3.8: Remove pipeline logic from CLI binary**
- Delete `Command::BuildPrompts`, `Command::IngestQa`, `Command::GenerateQa` and handlers
- Delete `mod generate_qa` 
- Keep `purge-qa` and `ocr` utilities
- Acceptance: CLI binary has only utilities

**Checkpoint 3:** Clean architecture. No dead code. No subprocess wrappers. Request structs co-located.

### Phase 4: Update pipeline YAML

**Task 4.1: Rewrite pipeline YAML to use MCP tools with approved models**
- Replace `corpus_ingest_embed` → separate `docproc_embed` + `docproc_tag_chunks` + `docproc_extract_triples`
- Replace `corpus_ingest_build_prompts` → `docproc_build_prompts`
- Replace `corpus_ingest_generate_qa` → `docproc_generate_qa_batch`
- Replace `corpus_ingest_ingest_qa` → `docproc_ingest_qa`
- Update models: `HKASK_CLASSIFIER_MODEL_A=DI/Qwen/Qwen3.6-35B-A3B`, `HKASK_QA_MODEL=DI/zai-org/GLM-5.2`
- Add `owner: john-brooks` to all steps that store h_mems
- Acceptance: All steps reference `docproc_*` tools with correct models

**Task 4.2: Build and verify**
- `cargo build --release -p hkask-corpus-ingest -p hkask-mcp-docproc -p hkask-mcp-replica`
- `cargo test -p hkask-mcp-docproc`
- Verify .env models match approved values
- Delete ad-hoc scripts
- Acceptance: All build clean, tests pass, models correct

**Checkpoint 4:** Everything builds, tests pass, models correct, pipeline YAML valid.

### Phase 5: Run pipeline through MCP tools

| Task | Tool | Input → Output |
|------|------|----------------|
| 5.1 | Purge existing data | Keep chunks.jsonl only |
| 5.2 | `docproc_embed` | chunks.jsonl → DB vectors + h_mems |
| 5.3 | `docproc_tag_chunks` | chunks.jsonl → tagged_ontology.jsonl + tag h_mems |
| 5.4 | `docproc_extract_triples` | chunks → triple h_mems in DB |
| 5.5 | `docproc_dedup_chunks` | tagged → deduped.jsonl |
| 5.6 | `docproc_consolidate_chunks` | deduped → consolidated.jsonl |
| 5.7 | `docproc_build_prompts` | consolidated → prompts.jsonl (with h_mem KG) |
| 5.8 | `docproc_generate_qa_batch` | prompts → generated.jsonl (GLM-5.2) |
| 5.9 | `docproc_ingest_qa` | generated → train.jsonl + QA h_mems |

**Checkpoint 5:** Full pipeline complete through MCP tools.

### Phase 6: Train

| Task | Description |
|------|-------------|
| 6.1 | Convert train.jsonl to ChatML format |
| 6.2 | Verify Axolotl config |
| 6.3 | Train on RunPod H100 with PiSSA |

**Checkpoint 6:** LoRA adapter trained.

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| New MCP tools have bugs | High | Test each tool individually with small data before pipeline run |
| h_mem owner mismatch | High | All tools accept owner param, default "john-brooks" |
| Template name mismatch | Medium | Fixed to "extract-hmems", test template loading |
| Request struct move breaks imports | Medium | Use `use crate::*` glob imports in tool modules |
| DeepInfra rate limits | Medium | Concurrency limits, retry with backoff |
| DeepInfra credits | High | Monitor balance |
| Thinking-mode tokens | Low | Verified ~640-830 reasoning tokens, max_tokens=4096 |

## Key Design Decisions

1. **Separate tag + extract tools, not combined** — The `classify-chunks.j2` combined template caused the original bugs. Use `tag-chunks.j2` + `extract-hmems.j2` separately with `docproc_tag_chunks` + `docproc_extract_triples`.

2. **h_mem entity = chunk_ref, owner = john-brooks** — All knowledge about a chunk retrievable via `semantic.query_deduped(chunk_ref)`. Owner is the replicant persona.

3. **All prompt-generating tools use Jinja2 templates** — `tag-chunks.j2`, `extract-hmems.j2`, `consolidate-chunks.j2`, `generate-qa.j2`, `build-prompts.j2`, `rag-answer.j2`. Tools that don't call LLMs (dedup, embed, ingest-qa) don't use templates.

4. **Pipeline YAML is the single source of truth** — All steps, models, DB paths, and parameters in the YAML. Invoked via `kask mcp invoke`.

5. **Replica server corpus tools delegate to docproc** — No subprocess calls. In-process delegation.

6. **CLI binary is thin utilities only** — `purge-qa`, `ocr`. No pipeline logic.

## Open Questions

| # | Question | Recommendation |
|---|----------|----------------|
| Q1 | Should `build-prompts.j2` include examples like `generate-qa.j2`? | Yes — examples improve LLM output quality significantly |
| Q2 | Should the pipeline YAML use `kask mcp invoke` or the replica server's `replica_pipeline_run`? | `replica_pipeline_run` with checkpoint/resume is better for long runs |
| Q3 | Dedup threshold for ingest_qa? | 0.89 (moderate — proven balance of dedup vs coverage) |