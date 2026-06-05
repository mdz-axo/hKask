# Handoff Document — hKask Embed Corpus Refactoring

**Session Purpose:** Convert `embed_corpus.rs` from a monolithic Rust pipeline to a thin CLI orchestrator backed by MCP tools and declarative manifests/skills, and implement all follow-up recommendations.
**Timestamp:** 2026-06-05
**Version:** 1.0

---

## Next Session Purpose

Complete the remaining work from Recommendation 4 (implement `hkask-mcp-doc-knowledge` server) and perform final validation/integration across the full embed corpus pipeline. The code compiles and tests pass, but the doc-knowledge MCP server is still missing, and the `embed_corpus.rs` thin wrapper still contains local chunking logic that should delegate to `semantic_chunk` when wired through MCP.

---

## Progress Summary

| # | Task | Status |
|---|------|--------|
| 1 | Convert `embed_corpus.rs` to thin CLI orchestrator | ✅ Done |
| 2 | Create `registry/manifests/style-corpus-embed.yaml` FlowDef manifest | ✅ Done |
| 3 | Create `registry/registries/skills/style-corpus-embed.yaml` skill | ✅ Done |
| 4 | Add `semantic_centroid` MCP tool to `hkask-mcp-semantic` | ✅ Done |
| 5 | Add `semantic_purge` MCP tool to `hkask-mcp-semantic` | ✅ Done |
| 6 | Add `semantic_chunk` MCP tool to `hkask-mcp-semantic` | ✅ Done |
| 7 | Add `store_as` + `model` params to `semantic_centroid` (compute+store in one call) | ✅ Done |
| 8 | Replace zero-vector KNN scan with SQL `query_by_prefix` in `EmbeddingPort` | ✅ Done |
| 9 | Add `idx_embeddings_entity_ref` SQL index | ✅ Done |
| 10 | Add `CentroidResult` struct with `passage_count` and `stored` fields | ✅ Done |
| 11 | Move `chunk_text` + `strip_gutenberg_headers` to `SemanticMemory` | ✅ Done |
| 12 | Register all new tools in REPL builtin servers | ✅ Done |
| 13 | Update manifest Step 3 to use `semantic_chunk` | ✅ Done |
| 14 | Update manifest Step 6 to use `store_as`/`model` | ✅ Done |
| 15 | Update manifest Step 1 to drop `dim` from `semantic_purge` | ✅ Done |
| 16 | Update `hemingway-style-synthesizer.yaml` OCAP for centroid | ✅ Done |
| 17 | Update `corpus.yaml` header with manifest/skill references | ✅ Done |
| 18 | Remove `mcp_web_client.rs` (was already gone) | ✅ Done (pre-existing) |
| 19 | Add `query_by_prefix` test | ✅ Done |
| 20 | Implement `hkask-mcp-doc-knowledge` server | ❌ Not started |
| 21 | Wire `embed_corpus.rs` to call MCP tools instead of direct store calls | ❌ Not started |
| 22 | End-to-end integration test of full pipeline via MCP | ❌ Not started |

---

## Key Decisions & Rationale

1. **Chunking lives on `hkask-mcp-semantic`, not a new doc-knowledge server.** Rationale: The style-corpus-embed manifest already uses `hkask-mcp-semantic` for Steps 1, 5, 6. Adding `semantic_chunk` keeps all pipeline tools on one server, simplifying the manifest. The `doc-knowledge` manifest (`registry/templates/templates/doc-knowledge/manifest.yaml`) declares more tools (`doc_knowledge_parse`, `doc_knowledge_generate_qa`, etc.) that are beyond the embedding pipeline scope.

2. **SQL `LIKE` prefix query replaces zero-vector KNN scan.** Rationale: The old `purge_by_prefix` and `compute_centroid` used a zero-vector KNN scan with limit 10000, then filtered by prefix. This doesn't scale and is semantically wrong (KNN is for similarity, not prefix matching). The new `query_by_prefix` uses `SELECT entity_ref FROM embeddings WHERE entity_ref LIKE ?1` with a `idx_embeddings_entity_ref` index.

3. **`compute_centroid` returns `CentroidResult` struct, not just `Vec<f32>`.** Rationale: With `store_as`/`model` params, the caller needs to know whether the centroid was stored, and how many passages contributed. The struct `CentroidResult { centroid, passage_count, stored }` carries this.

4. **`semantic_purge` no longer requires `dim` parameter.** Rationale: The old API needed `dim` to construct a zero-vector for the KNN scan. With SQL prefix queries, the dimension is irrelevant — we just delete matching rows.

5. **`EmbeddingPort` trait gained `query_by_prefix`.** Rationale: This is a fundamental storage operation (prefix query) that belongs in the port interface, not just the concrete `EmbeddingStore`. All implementations should support it.

6. **`serde` added to `hkask-memory` dependencies.** Rationale: `CentroidResult` derives `Serialize`/`Deserialize` for the MCP server's JSON output.

---

## Current State

All code compiles cleanly (`cargo check -p hkask-storage -p hkask-memory -p hkask-mcp-semantic -p hkask-cli` passes). All tests pass (21 for hkask-storage, 8 for embed_corpus). The manifest and skill YAMLs are created and reference the correct MCP tools.

**Remaining gap:** The `embed_corpus.rs` thin wrapper still calls `EmbeddingStore` methods directly (`.store()`, `.query_by_prefix()`, `.get()`, `.delete()`) instead of routing through MCP tool calls. The comment at each step says "see manifest Step N" but the code doesn't actually dispatch through the MCP layer. This is the pragmatic bridge until the manifest runtime executor exists.

The `hkask-mcp-doc-knowledge` server (Recommendation 4) is not implemented. Its manifest exists at `registry/templates/templates/doc-knowledge/manifest.yaml` with 7 declared tools but no Rust binary.

---

## Artifact References

| Type | Path | Description | Relevance |
|------|------|-------------|-----------|
| source | `crates/hkask-cli/src/commands/embed_corpus.rs` | Thin CLI orchestrator, ~590 lines | Core file being refactored |
| source | `crates/hkask-memory/src/semantic.rs` | `SemanticMemory` with `compute_centroid`, `purge_by_prefix`, `chunk_text`, `strip_gutenberg_headers`, `CentroidResult` | Domain logic for corpus operations |
| source | `crates/hkask-storage/src/embeddings.rs` | `EmbeddingStore` with `query_by_prefix`, `idx_embeddings_entity_ref` index | Storage layer with new prefix query |
| source | `crates/hkask-storage/src/database.rs` | Schema with new `idx_embeddings_entity_ref` index | DB schema change |
| source | `crates/hkask-types/src/ports.rs` | `EmbeddingPort` trait with new `query_by_prefix` method | Port interface |
| source | `mcp-servers/hkask-mcp-semantic/src/main.rs` | 9-tool MCP server (added centroid, purge, chunk) | MCP tool server |
| source | `crates/hkask-cli/src/repl/builtin_servers.rs` | REPL tool registration for semantic_centroid, semantic_purge, semantic_chunk | REPL integration |
| manifest | `registry/manifests/style-corpus-embed.yaml` | 8-step FlowDef pipeline manifest | Declarative pipeline spec |
| skill | `registry/registries/skills/style-corpus-embed.yaml` | Skill manifest | Composable skill |
| manifest | `registry/manifests/hemingway-style-synthesizer.yaml` | Consumer of embeddings (updated OCAP) | Downstream dependency |
| config | `registry/styles/hemingway/corpus.yaml` | Corpus spec (updated header) | Input to pipeline |
| config | `registry/templates/templates/doc-knowledge/manifest.yaml` | Declares 7 doc-knowledge tools (no server yet) | Recommendation 4 target |
| source | `mcp-servers/hkask-mcp-condenser/src/main.rs` | Existing condenser server (reference for new server pattern) | Pattern reference |
| source | `mcp-servers/hkask-mcp-condenser/Cargo.toml` | Cargo config pattern for MCP servers | Pattern reference |

---

## Suggested Skills

| Skill | Reason | Priority |
|-------|--------|----------|
| `coding-guidelines` | Enforce Karpathy's principles on remaining implementation | recommended |
| `create-skill` | If doc-knowledge tools should be packaged as a skill | optional |

---

## Open Questions & Risks

| Question | Risk | Context |
|----------|------|---------|
| Should `embed_corpus.rs` route through MCP dispatcher now, or wait for manifest runtime? | Medium | Current code calls `EmbeddingStore` directly. The manifest references MCP tools but there's no executor. The CLI works but isn't manifest-driven at runtime. |
| Should `hkask-mcp-doc-knowledge` be a separate binary or merged into condenser? | Low | The doc-knowledge manifest declares tools like `doc_knowledge_generate_qa` and `doc_knowledge_store_qa` that are LLM-assisted, not pure text processing. This suggests it should be a separate server. |
| How does `semantic_chunk` handle very large texts (whole novels)? | Low | Currently the entire text is passed as a single string parameter. For very large texts, this may hit MCP message size limits. Consider chunk-per-chapter or streaming. |
| `idx_embeddings_entity_ref` index is `CREATE INDEX IF NOT EXISTS` — existing databases won't have it until reopened | Low | SQLite `IF NOT EXISTS` means the index is created on next DB open. No migration needed, but existing DBs won't benefit from the index until the schema init runs again. |
| Does `query_by_prefix` using SQL `LIKE` need `%` or `_` escaping for entity_refs containing those chars? | Low | Current entity_ref format is `style:{author}:{slug}:{index}` — no `%` or `_` characters. But if entity_refs ever contain these, the `LIKE` pattern `prefix%` could match incorrectly. |

---

## Implementation Instructions for Next Agent

### Task 1: Implement `hkask-mcp-doc-knowledge` MCP server

Create a new MCP server binary at `mcp-servers/hkask-mcp-doc-knowledge/` following the pattern of `hkask-mcp-condenser` (see `mcp-servers/hkask-mcp-condenser/Cargo.toml` and `src/main.rs`).

The manifest declaring its tools is at `registry/templates/templates/doc-knowledge/manifest.yaml`. Implement these tools:

1. **`doc_knowledge_chunk`** — Chunk text at configurable granularity (`max_tokens`, `overlap_tokens`). This is a more general version of `semantic_chunk`. Can delegate to `SemanticMemory::chunk_text` internally but with a different parameter model (tokens not words).
2. **`doc_knowledge_detect_format`** — Detect document format from path/extension.
3. **`doc_knowledge_extract_markdown`** — Extract text and image refs from markdown.
4. **`doc_knowledge_extract_html`** — Extract text from HTML.
5. **`doc_knowledge_parse`** — Parse document into IR with multi-tier chunking (coarse/medium/fine).
6. **`doc_knowledge_generate_qa`** — Generate QA pairs from text chunk (requires LLM via Okapi).
7. **`doc_knowledge_store_qa`** — Store QA items with provenance.

Steps:
1. Create `mcp-servers/hkask-mcp-doc-knowledge/Cargo.toml` — model after condenser's Cargo.toml, add `hkask-memory` and `hkask-templates` deps
2. Create `mcp-servers/hkask-mcp-doc-knowledge/src/main.rs` — implement the 7 tools
3. Add the binary to the workspace `Cargo.toml`
4. Register the server's tools in `crates/hkask-cli/src/repl/builtin_servers.rs`
5. Run `cargo check -p hkask-mcp-doc-knowledge` and `cargo test -p hkask-mcp-doc-knowledge`

### Task 2: Wire `embed_corpus.rs` to use MCP dispatcher

Currently `embed_corpus.rs` calls `EmbeddingStore` methods directly. Wire it to route through the MCP dispatcher so each step calls the corresponding MCP tool:

- Step 2 (purge) → `semantic_purge` via MCP
- Step 4 (chunk) → `semantic_chunk` via MCP
- Step 7 (embed) → `semantic_embed` via MCP
- Step 8 (centroid) → `semantic_centroid` via MCP with `store_as` + `model`

This requires creating an MCP client in `embed_corpus.rs` (similar to the old `McpWebClient` pattern that was removed) that connects to `hkask-mcp-semantic` via stdio transport and calls tools. Reference the existing `mcp_web_client.rs` pattern from git history if needed, or use the `hkask-mcp` crate's `McpDispatcher`.

### Task 3: End-to-end validation

After Tasks 1-2, run the full pipeline:
```bash
cargo build --release
kask embed-corpus run -c registry/styles/hemingway/corpus.yaml -d <db_path> --passphrase <pass>
```
Verify that:
- All 4 Hemingway works download and cache
- Passages are chunked correctly
- Embeddings are stored in sqlite-vec
- Centroid is computed and stored
- Re-running is idempotent (purge + re-embed)
- `semantic_search` finds Hemingway passages
- Centroid distance check works

### Task 4: Update `doc-knowledge` manifest to align with actual tool signatures

After Task 1, update `registry/templates/templates/doc-knowledge/manifest.yaml` to match the actual implemented tool signatures (input/output schemas).

---

## Redaction Summary

No sensitive data (API keys, passwords, tokens, PII) were found in session context. 0 redactions applied.