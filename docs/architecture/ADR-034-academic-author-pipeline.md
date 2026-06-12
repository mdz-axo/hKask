# ADR-034 — Academic Author Pipeline Architecture

**Status:** Proposed (design decisions documented, implementation pending)
**Date:** 2026-06-11
**Version:** hKask v0.27.0

## Context

The current style composition pipeline (`EmbedService::embed_corpus` + `ComposeService::compose`) is designed for literary authors whose works are available as plaintext from Project Gutenberg. Extending the pipeline to academic authors (researchers, scientists, philosophers) requires resolving four architectural questions before implementation can begin.

## Decisions

### 1. Content Acquisition for Non-Gutenberg Sources

**Problem:** `download_text()` in `embed.rs` performs HTTP GET → plaintext only. Academic corpora include PDFs (arXiv), HTML (institutional pages), and potentially YouTube transcripts.

**Decision:** Pre-process non-plaintext sources into `.cache/{slug}.txt` using existing MCP tools before the embed pipeline runs. No code change to `embed.rs` is needed — the cache-first logic already checks for the `.txt` file.

| Source Type | Pre-processing Tool | Output |
|-------------|-------------------|--------|
| PDF (arXiv) | `hkask-mcp-markitdown` → `parse_pdf` | `.cache/{slug}.txt` |
| HTML (institutional) | `hkask-mcp-research` → `web_extract` | `.cache/{slug}.txt` |
| YouTube transcript | `hkask-mcp-research` → `youtube_transcript` | `.cache/{slug}.txt` |
| Plaintext (Gutenberg, blog) | Direct HTTP GET (existing) | `.cache/{slug}.txt` |

**Rationale:** The embed pipeline is a batch processor, not a content acquisition system. Separating acquisition from embedding keeps `embed.rs` simple and lets the Curator orchestrate multi-source acquisition as a pre-processing step.

### 2. Entity Model for Academic Corpora

**Problem:** Current `EntityConfig` is literary: characters, places, events, concepts. Method signals (parataxis ratio, dialogue ratio) are meaningless for academic prose. Academic corpora need different entity categories and different method signals.

**Decision:** Add `corpus_type: "literary" | "academic"` to `CorpusConfig` in `crates/hkask-services/src/embed.rs`. When `"academic"`:

- **Entity categories:** co-authors, venues (journals/conferences), topics (research areas), paradigms (theoretical frameworks)
- **Method signals:** Route to academic-specific signals in `crates/hkask-memory/src/salience.rs` — citation density, formalism ratio (math/code vs. prose), hedging density, technical term density
- **Entity reference prefix:** `academic:{author}:{slug}:{index}` instead of `style:{author}:{slug}:{index}`

**Rationale:** Literary and academic prose have fundamentally different structural properties. A single entity model would produce meaningless signals for one domain or the other. The `corpus_type` discriminator keeps both domains clean without duplicating the pipeline.

### 3. Work Enumeration for Academic Authors

**Problem:** Literary authors have a fixed, well-known corpus (published books). Academic authors publish continuously across multiple venues (arXiv, conferences, journals, blogs). Discovering the complete corpus requires search and extraction.

**Decision:** Agent-driven orchestration of existing research MCP tools. No new MCP tool needed. The Curator orchestrates discovery:

1. `web_search` — find author's papers across arXiv, DBLP, Google Scholar
2. `web_extract` — extract paper metadata (title, venue, year, abstract)
3. `web_find_similar` — discover related papers, co-authored works
4. Curator presents enumerated corpus to user for confirmation
5. Confirmed works are written to a `CorpusConfig` YAML for the embed pipeline

**Rationale:** Work enumeration is a discovery task, not a batch processing task. It requires search, extraction, and user confirmation — all capabilities the Curator already has through existing MCP tools. Adding a dedicated MCP tool would duplicate the Curator's orchestration logic.

### 4. Disambiguation Confirmation Boundary

**Problem:** Academic authors with common names (e.g., "J. Smith") require disambiguation. Where does the disambiguation conversation happen?

**Decision:** At the agent/Curator level, not inside MCP tools. MCP tools are stateless; disambiguation requires conversation state (presenting candidates, receiving user confirmation, proceeding with the selected identity).

**Pattern:**
1. Agent searches → finds multiple candidate author profiles
2. Agent presents candidates to user: "Found 3 researchers named J. Smith: [1] ML researcher at Stanford, [2] HCI researcher at MIT, [3] theoretical physicist at CERN. Which one?"
3. User confirms: "1"
4. Agent proceeds with the confirmed identity

**Rationale:** MCP tools are stateless by design (request → response). Disambiguation is inherently stateful (present → confirm → proceed). The Curator already manages conversation state; adding disambiguation state to MCP tools would violate the stateless contract.

## Consequences

- **Positive:** Clear separation of concerns — acquisition (pre-processing), enumeration (Curator orchestration), embedding (batch pipeline), disambiguation (conversation state)
- **Positive:** No new MCP tools needed — all capabilities exist in the current tool set
- **Negative:** `corpus_type` discriminator adds branching to `embed.rs` and `salience.rs` — increases implementation complexity
- **Negative:** Pre-processing step adds a manual/Curator-driven phase before embedding — not fully automated

## Implementation Order

1. `corpus_type` field + academic entity model + academic method signals (code changes to `embed.rs` and `salience.rs`)
2. Pre-processing documentation + Curator workflow for non-Gutenberg acquisition
3. Work enumeration Curator workflow (uses existing MCP tools, no new code)
4. Disambiguation confirmation boundary (Curator conversation pattern, no new code)

## References

- `crates/hkask-services/src/embed.rs` — `EmbedService::embed_corpus`, `CorpusConfig`
- `crates/hkask-memory/src/salience.rs` — method signal extraction
- `crates/hkask-services/src/compose.rs` — `ComposeService::compose`, `CognitionConfig`
- `registry/styles/hemingway/corpus.yaml` — reference literary corpus config
- `registry/registries/cognition/hemingway-style-synthesizer.yaml` — reference cognition config
