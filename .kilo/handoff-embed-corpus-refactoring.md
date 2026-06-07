# Handoff Document — hKask Embed Corpus Refactoring

**Session Purpose:** Convert `embed_corpus.rs` from a monolithic Rust pipeline to a thin CLI orchestrator backed by MCP tools and declarative manifests/skills, implement `kask compose` CLI command, and validate the full pipeline.
**Timestamp:** 2026-06-05
**Version:** 2.0

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
| 20 | Implement `hkask-mcp-doc-knowledge` server | ✅ Done (8 tools: ping, detect_format, chunk, extract_markdown, extract_html, parse, generate_qa, store_qa) |
| 21 | Wire `embed_corpus.rs` to use `SemanticMemory` instead of direct store calls | ✅ Done (delegates to SemanticMemory methods, not MCP dispatcher) |
| 22 | Fix compilation errors in `compose.rs` | ✅ Done |
| 23 | Create `kask compose run` CLI command | ✅ Done |
| 24 | Wire `kask compose` into CLI dispatch | ✅ Done |
| 25 | Update `hemingway-style-synthesizer.j2` with `system_prompt` macro | ✅ Done |
| 26 | Update `hemingway-style-synthesizer.yaml` manifest (v0.23.00, inputs, fixed steps) | ✅ Done |
| 27 | Fix `mcp_doc_extract.yaml` (`hkask-cns` → `hkask-mcp-cns`, v0.23.00) | ✅ Done |
| 28 | Align `doc-knowledge/manifest.yaml` with actual 8-tool implementation | ✅ Done |
| 29 | End-to-end build validation | ✅ Done (`cargo build --release` passes) |
| 30 | Gap audit (stale refs, dead code, version mismatches) | ✅ Done |

---

## Key Decisions & Rationale

1. **`kask compose` is a separate CLI command, not wired into `kask chat`.** Rationale: Same pattern as `kask embed-corpus` — a focused pipeline orchestrator. Wiring exemplar retrieval into `kask chat` would require threading `EmbeddingGenerationPort` and `EmbeddingPort` through the entire REPL state, which is a larger change with cascading implications.

2. **`embed_corpus.rs` delegates to `SemanticMemory` methods, not MCP dispatcher.** Rationale: The CLI is a Rust binary with direct access to `SemanticMemory`. Routing through MCP would add network overhead and complexity with no benefit — the manifest runtime executor doesn't exist yet. When it does, the CLI can be refactored to use it.

3. **System prompt is hardcoded in Rust (compose.rs), not rendered from Jinja2.** Rationale: `kask compose` is a CLI tool, not a template renderer. The Jinja2 template is available for the manifest runtime executor when it exists.

4. **Generation model defaults to embedding model but can be overridden via `OKAPI_MODEL` env.** Rationale: The embedding model (e.g., `qwen3-embedding:0.6b`) may not be suitable for generation. The env var provides a simple override without adding CLI flags.

5. **`k_min` is used as a lower-bound warning, not a hard filter.** Rationale: If fewer than `k_min` passages survive the distance threshold filter, the user is warned but the pipeline continues. Hard-filtering would make the command unusable when the corpus is small.

6. **SQL `LIKE` prefix query replaces zero-vector KNN scan.** Rationale: The old `purge_by_prefix` and `compute_centroid` used a zero-vector KNN scan with limit 10000, then filtered by prefix. This doesn't scale and is semantically wrong.

7. **`compute_centroid` returns `CentroidResult` struct, not just `Vec<f32>`.** Rationale: With `store_as`/`model` params, the caller needs to know whether the centroid was stored, and how many passages contributed.

8. **`hkask-mcp-doc-knowledge` is a separate 8-tool MCP server.** Rationale: Document parsing, chunking, and QA generation are distinct from semantic memory operations. The server includes a `doc_knowledge_ping` health check.

---

## Current State

All code compiles cleanly. All tests pass:
- `cargo check -p hkask-cli -p hkask-mcp-doc-knowledge -p hkask-mcp-semantic` — 0 errors
- `cargo test -p hkask-cli compose` — 4 tests pass
- `cargo test -p hkask-cli embed_corpus` — 7 tests pass
- `cargo test -p hkask-storage -p hkask-memory` — 21 tests pass
- `cargo build --release` — binary builds successfully
- `./target/release/kask compose run --help` — subcommand works
- `./target/release/kask embed-corpus run --help` — subcommand works

No `#[allow(dead_code)]` needed in compose.rs. All imports are used.

---

## Artifact References

| Type | Path | Description |
|------|------|-------------|
| source | `crates/hkask-cli/src/commands/compose.rs` | `kask compose run` subcommand — Hemingway style pipeline |
| source | `crates/hkask-cli/src/commands/embed_corpus.rs` | `kask embed-corpus run` subcommand — corpus embedding pipeline |
| source | `crates/hkask-cli/src/cli/actions.rs` | `ComposeAction` enum |
| source | `crates/hkask-cli/src/cli/mod.rs` | `Compose` subcommand definition |
| source | `crates/hkask-cli/src/commands/mod.rs` | `pub mod compose` |
| source | `crates/hkask-cli/src/main.rs` | `Commands::Compose` dispatch |
| source | `mcp-servers/hkask-mcp-doc-knowledge/` | 8-tool doc-knowledge MCP server |
| source | `crates/hkask-memory/src/semantic.rs` | `SemanticMemory` with `chunk_text`, `strip_gutenberg_headers`, `purge_by_prefix`, `compute_centroid`, `search_similar` |
| source | `crates/hkask-storage/src/embeddings.rs` | `EmbeddingStore` with `query_by_prefix` |
| source | `crates/hkask-templates/src/inference_port.rs` | `OkapiInference::new`, `InferencePort::generate` |
| template | `registry/templates/composition/hemingway-style-synthesizer.j2` | Jinja2 template with `system_prompt` macro |
| manifest | `registry/manifests/hemingway-style-synthesizer.yaml` | Process manifest v0.23.00 |
| manifest | `registry/manifests/mcp_doc_extract.yaml` | Fixed server name + v0.23.00 |
| config | `registry/templates/templates/doc-knowledge/manifest.yaml` | Aligned with 8-tool implementation |
| cognition | `registry/registries/cognition/hemingway-style-synthesizer.yaml` | Cognition config (no changes needed) |

---

## Open Questions & Risks

| Question | Risk | Context |
|----------|------|---------|
| Should `embed_corpus.rs` eventually route through MCP dispatcher? | Medium | Current code calls `SemanticMemory` directly. The manifest references MCP tools but there's no executor. The CLI works but isn't manifest-driven at runtime. |
| Does `query_by_prefix` using SQL `LIKE` need `%` or `_` escaping for entity_refs containing those chars? | Low | Current entity_ref format is `style:{author}:{slug}:{index}` — no `%` or `_` characters. |
| How does `semantic_chunk` handle very large texts (whole novels)? | Low | Entire text is passed as a single string parameter. May hit MCP message size limits for very large texts. |
| `idx_embeddings_entity_ref` index — existing databases won't have it until reopened | Low | SQLite `IF NOT EXISTS` means the index is created on next DB open. |

---

## Redaction Summary

No sensitive data (API keys, passwords, tokens, PII) were found in session context. 0 redactions applied.