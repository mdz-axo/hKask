# TODO — Architecture Improvement: Crate Elimination + MCP Server Deepening

## Phase A: Eliminate corpus-ingest crate

- [ ] **A.1** Create `docproc_purge_qa` MCP tool in storage.rs
  - [ ] Migrate purge logic from corpus-ingest main.rs
  - [ ] Add PurgeQaRequest struct
  - [ ] Register in combined_router
  - [ ] `cargo build -p hkask-mcp-docproc` passes
- [ ] **A.2** Delete `hkask-corpus-ingest` crate
  - [ ] Remove from workspace Cargo.toml
  - [ ] Delete crates/hkask-corpus-ingest/ directory
  - [ ] `cargo build` succeeds without it

**Checkpoint A:** No corpus-ingest crate, purge-qa is docproc tool

## Phase B: Clean up replica server

- [ ] **B.1** Update replica pipeline_run step executor
  - [ ] Replace corpus_* match arms with deprecation messages
  - [ ] Remove `std::process::Command::new("corpus-ingest")`
  - [ ] `cargo build -p hkask-mcp-replica` passes
  - [ ] `cargo test -p hkask-mcp-replica` passes

**Checkpoint B:** No subprocess calls in replica server

## Phase C: Co-locate request structs

- [ ] **C.1** Move document structs (ConvertRequest, OcrRequest, ChunkRequest) → document.rs
- [ ] **C.2** Move semantic structs (GenerateQaRequest, BatchQaPrompt, GenerateQaBatchRequest, ExtractTriplesRequest, EmbedRequest) → semantic.rs
- [ ] **C.3** Move corpus + tagging structs (DedupChunksRequest, ConsolidateChunksRequest, TagChunksRequest, BuildPromptsRequest, IngestQaRequest, PurgeQaRequest) → corpus.rs + tagging/ops.rs
- [ ] **C.4** Move storage structs (CacheRequest, QueryRequest, ClearIndexRequest) → storage.rs

**Checkpoint C:** All structs co-located, lib.rs < 1000 lines

## Phase D: Extract template module

- [ ] **D.1** Create template.rs module
  - [ ] Move TEMPLATE_CACHE + render_docproc_template
  - [ ] Update tool files to use crate::template::render_docproc_template
  - [ ] `cargo build -p hkask-mcp-docproc` passes

**Checkpoint D:** Template engine isolated, lib.rs < 900 lines