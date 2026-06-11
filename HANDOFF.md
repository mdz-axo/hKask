# Handoff — Replica Metadata Layer & Academic Pipeline

## Session Context

Built the complete metadata layer for the style replica system: salience scoring (graph centrality), entity tagging (5W1H), method signal extraction, budget-gated triple storage, and Jinja2 system prompt rendering. Added entity declarations to all 5 corpus YAMLs. Documented the academic author pipeline in MDS and replica MCP server. The entire system compiles clean (40/40 tests pass) but has **not been run end-to-end with real data** — that is the clear next step.

## What Was Done

### New file
- `crates/hkask-memory/src/salience.rs` — 753 lines, 12 tests. Method signal computation (10 stylometric metrics), entity tagging (substring matching), batch salience scoring (`(one_hop + two_hop/2) / 2`), declared method matching with signal thresholds, budget config (`per_100_pages` or `absolute`).

### Code changes
- `crates/hkask-memory/src/lib.rs` — Added `pub mod salience`
- `crates/hkask-services/src/embed.rs` — 6-phase pipeline: tag → salience → budget gate → embed → triples → centroid. Added `text` triple for exemplar retrieval. New config types: `EntityConfig`, `Entity`, `DeclaredMethod`, `BudgetConfig`. New result fields: `budget`, `tagged_passages`, `triples_stored`, `embedding_only`.
- `crates/hkask-services/src/compose.rs` — Added `jinja2_template: Option<String>` to `CognitionConfig`. `render_jinja2_prompt()` renders system prompt from YAML template with context variables (`prompt`, `exemplars`, `author`, `no_validate`, `centroid_distance_max`). Salience-filtered retrieval pipeline (`salience_min`, `salience_top_k` in `RetrievalSection`). Hardcoded per-author system prompts removed; `generic_system_prompt()` is the fallback.
- `crates/hkask-services/src/error.rs` — Added `Compose(String)` variant
- `crates/hkask-services/Cargo.toml` — Added `minijinja.workspace = true`
- `crates/hkask-services/src/lib.rs` — Updated exports
- `crates/hkask-cli/src/commands/embed_corpus.rs` — Budget/storage stats in CLI output
- `mcp-servers/hkask-mcp-replica/src/main.rs` — `BuildResult` new fields, `okapi_url` wired to `replica_build`, registry Remove purges triples, `replica_explain` documents metadata layer + exemplar types + academic pipeline

### YAML configs
- `registry/registries/cognition/*.yaml` ×5 — All now have `author:`, `jinja2_template` (system prompt), `salience_min`, `salience_top_k` in retrieval section
- `registry/styles/*/corpus.yaml` ×5 — All now have `entities:` (characters, places, events, concepts), `methods:` (declared with signal thresholds), `budget:` (per_100_pages: 3750)

### Documentation
- `docs/architecture/MDS.md` — Added Replicant Architecture section: three exemplar types table, academic author pipeline (5-step discovery), human exemplar principle
- `mcp-servers/hkask-mcp-replica/src/main.rs` `replica_explain` — `exemplar_types` object documenting all three types + pipeline steps + infrastructure reuse

### Build status
- `cargo check -p hkask-services` — ✅ Clean
- `cargo check -p hkask-mcp-replica` — ✅ Clean
- `cargo test -p hkask-memory` — ✅ 12/12
- `cargo test -p hkask-services` — ✅ 28/28
- `cargo check -p hkask-cli` — ⚠️ Pre-existing error in `curator.rs` (unrelated `&mut` vs `&` mismatch)

## What Remains

### HIGH — End-to-end validation

The metadata layer is wired but never tested with real data. Entity declarations exist in YAML but no corpus has been embedded with them. Priority steps:

1. **Run `kask embed-corpus run` against Hemingway** with the updated `corpus.yaml` (entities, methods, budget declared). Verify:
   - Entity tags are produced (non-zero character/place/event/concept counts)
   - Method signals are computed and methods are matched
   - Salience scores are non-zero (passages sharing entities get connectedness)
   - Budget gate selects a subset of passages for triple storage
   - Triples are stored correctly: `text`, structural metadata, entity tags, method tags, method signals, salience
   - CLI output shows budget stats

2. **Run `kask compose run` with Hemingway synthesizer** to verify Jinja2 rendering:
   - `--cognition registry/registries/cognition/hemingway-style-synthesizer.yaml`
   - Verify the Jinja2 template renders correctly (no `UndefinedBehavior::Strict` errors)
   - Verify exemplar passages contain actual prose text (not metadata)
   - Verify centroid validation runs

3. **Repeat for Woolf** to verify the second synthesizer template works with a different author

### MEDIUM — Academic author pipeline design

Four open architectural questions surfaced at end of session:

1. **Content acquisition**: `download_text()` does HTTP GET → bytes → String. Works for Gutenberg plaintext. Fails for PDFs (arXiv), HTML (institutional pages), YouTube transcripts. Research MCP has `web_extract` for HTML. `hkask-mcp-markitdown` handles PDFs. Need a provider dispatch layer or a decision to keep Gutenberg-only and have academic configs point to pre-processed `.txt` files.

2. **Entity model for academics**: The 5W1H model is literary (characters, places, events, concepts). For academics it would be co-authors, venues, topics, paradigms. Method signals (parataxis ratio, dialogue ratio) are meaningless for academic prose. Need either a `corpus_type: "literary" | "academic"` field or a separate `AcademicCorpusConfig`.

3. **Work enumeration**: "Find all work by David Dunning" requires multiple search calls across Google Scholar, arXiv, institutional pages, YouTube. Research MCP's `web_search` returns 10 results per call. Need either a new `replica_discover` MCP tool or agent-driven orchestration of existing tools.

4. **Confirmation boundary**: Disambiguation requires Curator confirmation ("Is this the David Dunning from Michigan?"). MCP tools are request/response — they don't pause mid-execution. Should disambiguation happen at the agent/Curator level, outside the tool?

### LOW — Stale manifest

`registry/manifests/style-corpus-embed.yaml` describes an 8-step MCP-tool pipeline that doesn't match the actual `EmbedService::embed_corpus` Rust function. Should be deleted or rewritten.

## Key Decisions to Preserve

1. **Salience = `(one_hop + two_hop/2) / 2`** — Pure graph centrality. No config weights, no position boost, no diversity bonus. One-hop is fraction of passages sharing ≥1 entity. Two-hop is fraction reachable within 2 hops (always ≥ one-hop). The `/2` on two-hop biases toward direct connections. This replaced an earlier weighted-category formula that had arbitrary character/place/event/concept/method weights.

2. **Budget gates triple storage, not embedding** — All passages get vectors. Only budget-selected passages get metadata triples. Foundational rules always get triples regardless of budget. Budget derived from `passage_count / 250 × per_100_pages` (default 3,750/100pg).

3. **Jinja2 templates are the canonical system prompt source** — When `jinja2_template` is present in cognition YAML, it's rendered with context variables and used as the system prompt. When absent, `generic_system_prompt()` is the fallback. Hardcoded per-author Rust functions were removed. The `format` filter bug (`"%.2f"` vs minijinja's `{}` syntax) was fixed by using direct `{{ centroid_distance_max }}` interpolation.

4. **Human exemplar principle** — All replica types model a named human individual whose body of work constitutes a representational corpus. The logical validity derives from the relationship between the human and their work. This applies equally to public domain authors, mashup personas, and academic authors.

5. **Academic pipeline reuses research MCP, not new infrastructure** — `web_search`, `web_extract`, `web_find_similar`, `web_browse` from `hkask-mcp-research` provide the discovery layer. No new search infrastructure needed.

## Recommended Skills

- **coding-guidelines** — Before any code changes, surface assumptions and enforce simplicity
- **tdd** — For any new tests during end-to-end validation
- **condenser-continuation** — If this session's context needs restoration after a reset

## Commands

```bash
# Build and test
cargo check -p hkask-services -p hkask-mcp-replica
cargo test -p hkask-memory -- salience
cargo test -p hkask-services

# End-to-end validation (requires Okapi running)
kask embed-corpus run \
  --config registry/styles/hemingway/corpus.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass \
  --okapi-url http://127.0.0.1:11435

kask compose run \
  --prompt "Write a war scene in the style of Hemingway." \
  --cognition registry/registries/cognition/hemingway-style-synthesizer.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass

# Verify triples
sqlite3 /tmp/hkask-test-styles.db "SELECT COUNT(*) FROM triples;"
sqlite3 /tmp/hkask-test-styles.db "SELECT entity, attribute, value FROM triples WHERE entity LIKE 'style:hemingway:%' LIMIT 20;"
```
