---
title: "ADR-034: Academic Author Pipeline Architecture"
audience: [architects, developers]
last_updated: 2026-06-11
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [composition, curation]
---

# ADR-034 — Academic Author Pipeline Architecture

**Status:** Proposed (design decisions documented, implementation pending)
**Date:** 2026-06-11
**Version:** hKask v0.27.0

## Context

The current style composition pipeline (`EmbedService::embed_corpus` + `ComposeService::compose`) is designed for literary authors whose works are available as plaintext from Project Gutenberg. Extending the pipeline to academic authors (researchers, scientists, philosophers) requires resolving four architectural questions before implementation can begin. [^gutenberg]

[^gutenberg]: Project Gutenberg. "Free eBooks." https://www.gutenberg.org/ — primary source for public-domain literary plaintext.

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

**Rationale:** The embed pipeline is a batch processor, not a content acquisition system. Separating acquisition from embedding keeps `embed.rs` simple and lets the Curator orchestrate multi-source acquisition as a pre-processing step. This follows the Strangler Fig pattern: the existing cache-first logic is the "fig" — new acquisition paths are added alongside it without modifying the core pipeline. [^strangler]

[^strangler]: Fowler, Martin. "Strangler Fig Application." martinfowler.com, 2004. https://martinfowler.com/bliki/StranglerFigApplication.html — incremental migration by introducing new paths alongside existing ones.

### 2. Entity Model for Academic Corpora

**Problem:** Current `EntityConfig` is literary: characters, places, events, concepts. Method signals (parataxis ratio, dialogue ratio) are meaningless for academic prose. Academic corpora need different entity categories and different method signals.

**Decision:** Add `corpus_type: "literary" | "academic"` to `CorpusConfig` in `crates/hkask-services/src/embed.rs`. When `"academic"`:

- **Entity categories:** co-authors, venues (journals/conferences), topics (research areas), paradigms (theoretical frameworks)
- **Method signals:** Route to academic-specific signals in `crates/hkask-memory/src/salience.rs` — citation density, formalism ratio (math/code vs. prose), hedging density, technical term density
- **Entity reference prefix:** `academic:{author}:{slug}:{index}` instead of `style:{author}:{slug}:{index}`

**Rationale:** Literary and academic prose have fundamentally different structural properties. A single entity model would produce meaningless signals for one domain or the other. The `corpus_type` discriminator keeps both domains clean without duplicating the pipeline. This follows the deep-module discipline: the module's interface (a single `corpus_type` field) hides substantial complexity (different entity categories, different signal extraction paths). [^deep-module]

[^deep-module]: Ousterhout, John. *A Philosophy of Software Design.* Yaknyam Press, 2018. Chapter 4: "Modules Should Be Deep" — deep modules have simple interfaces that hide complex implementations.

### 3. Work Enumeration for Academic Authors

**Problem:** Literary authors have a fixed, well-known corpus (published books). Academic authors publish continuously across multiple venues (arXiv, conferences, journals, blogs). Discovering the complete corpus requires search and extraction.

**Decision:** Agent-driven orchestration of existing research MCP tools. No new MCP tool needed. The Curator orchestrates discovery:

1. `web_search` — find author's papers across arXiv, DBLP, Google Scholar
2. `web_extract` — extract paper metadata (title, venue, year, abstract)
3. `web_find_similar` — discover related papers, co-authored works
4. Curator presents enumerated corpus to user for confirmation
5. Confirmed works are written to a `CorpusConfig` YAML for the embed pipeline

**Rationale:** Work enumeration is a discovery task, not a batch processing task. It requires search, extraction, and user confirmation — all capabilities the Curator already has through existing MCP tools. Adding a dedicated MCP tool would duplicate the Curator's orchestration logic. The MCP specification explicitly supports tool composition: servers expose tools, clients (the Curator) compose them into workflows. [^mcp-spec]

[^mcp-spec]: Anthropic. "Model Context Protocol Specification." 2024. https://modelcontextprotocol.io/ — MCP tools are designed for client-side composition, not server-side orchestration.

### 4. Disambiguation Confirmation Boundary

**Problem:** Academic authors with common names (e.g., "J. Smith") require disambiguation. Where does the disambiguation conversation happen?

**Decision:** At the agent/Curator level, not inside MCP tools. MCP tools are stateless; disambiguation requires conversation state (presenting candidates, receiving user confirmation, proceeding with the selected identity).

**Pattern:**
1. Agent searches → finds multiple candidate author profiles
2. Agent presents candidates to user: "Found 3 researchers named J. Smith: [1] ML researcher at Stanford, [2] HCI researcher at MIT, [3] theoretical physicist at CERN. Which one?"
3. User confirms: "1"
4. Agent proceeds with the confirmed identity

**Rationale:** MCP tools are stateless by design (request → response). Disambiguation is inherently stateful (present → confirm → proceed). The Curator already manages conversation state; adding disambiguation state to MCP tools would violate the stateless contract. [^mcp-stateless]

[^mcp-stateless]: Anthropic. "Model Context Protocol Specification — Core Architecture." 2024. https://modelcontextprotocol.io/docs/concepts/architecture — MCP servers are stateless; stateful interactions belong to the host (Curator).

## Consequences

- **Positive:** Clear separation of concerns — acquisition (pre-processing), enumeration (Curator orchestration), embedding (batch pipeline), disambiguation (conversation state)
- **Positive:** No new MCP tools needed — all capabilities exist in the current tool set
- **Negative:** `corpus_type` discriminator adds branching to `embed.rs` and `salience.rs` — increases implementation complexity
- **Negative:** Pre-processing step adds a manual/Curator-driven phase before embedding — not fully automated

[^principles]: hKask PRINCIPLES.md §2.1 — Constraint Forces. Prohibitions (P1) are inviolable; Guidelines (P3) admit tradeoffs. The `corpus_type` branching is a P3 tradeoff (complexity for domain coverage), not a P1 violation.

## Implementation Order

1. `corpus_type` field + academic entity model + academic method signals (code changes to `embed.rs` and `salience.rs`)
2. Pre-processing documentation + Curator workflow for non-Gutenberg acquisition
3. Work enumeration Curator workflow (uses existing MCP tools, no new code)
4. Disambiguation confirmation boundary (Curator conversation pattern, no new code)

[^strangler-migration]: The implementation order follows the Strangler Fig migration pattern: start with the core abstraction (`corpus_type`), then add new paths (pre-processing, enumeration) alongside the existing literary pipeline, and finally add the stateful boundary (disambiguation) at the outermost layer.

## References

- `crates/hkask-services/src/embed.rs` — `EmbedService::embed_corpus`, `CorpusConfig`
- `crates/hkask-memory/src/salience.rs` — method signal extraction
- `crates/hkask-services/src/compose.rs` — `ComposeService::compose`, `CognitionConfig`
- `registry/styles/hemingway/corpus.yaml` — reference literary corpus config
- `registry/registries/cognition/hemingway-style-synthesizer.yaml` — reference cognition config
