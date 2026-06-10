# Agatha Eliot — Option B: True Two-Vector MoE Architecture

**Status:** Design document. Not implemented.
**Created:** 2026-06-10
**Author:** The Curator

---

## Problem

The current Option A implementation (single blended corpus) treats Christie
and Eliot passages as a unified pool under `style:agatha-eliot:`. KNN
retrieval pulls the nearest N passages from the combined space. The "MoE"
separation exists only in the system prompt.

This works for adjacent authors (Austen/Wilde, Grant/Twain) whose embedding
regions naturally overlap. But Christie and Eliot are **far apart** in
embedding space. Their corpus vectors occupy different regions (forensic
clarity vs. psychological depth, declarative syntax vs. hypotactic
accumulation). A single blended centroid produces a location that neither
author alone occupies — which is conceptually interesting but may dilute
retrieval precision. The model sees both Christie and Eliot passages
interleaved by similarity to the prompt, without knowing which register to
use at which narrative moment.

**True MoE** would maintain separate retrieval indices and route the model
between them based on narrative function.

---

## Architecture

```
                     ┌──────────────────┐
                     │   User Prompt     │
                     └────────┬─────────┘
                              │ embed
                              ▼
                     ┌──────────────────┐
                     │  Prompt Vector    │
                     └────────┬─────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
     ┌────────────────┐              ┌────────────────┐
     │ Christie Corpus │              │  Eliot Corpus   │
     │ prefix:         │              │ prefix:         │
     │ style:christie: │              │ style:eliot:    │
     │                 │              │                 │
     │ KNN (k=3)       │              │ KNN (k=3)       │
     │ → 3 passages    │              │ → 3 passages    │
     └────────┬────────┘              └────────┬────────┘
              │                               │
              │  Christie exemplars           │  Eliot exemplars
              │  (forensic clarity,           │  (consciousness,
              │   clue rhythm,                │   free indirect,
              │   structural beats)           │   moral depth)
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │  System Prompt    │
                     │  (MoE routing)    │
                     └────────┬─────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │  Inference        │
                     └────────┬─────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │  Generated Prose  │
                     └────────┬─────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
     ┌────────────────┐              ┌────────────────┐
     │ Christie        │              │ Eliot           │
     │ Centroid        │              │ Centroid        │
     │ Validation      │              │ Validation      │
     │                 │              │                 │
     │ distance ≤ 0.15 │              │ distance ≤ 0.20 │
     └────────┬────────┘              └────────┬────────┘
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │ Combined Pass/Fail│
                     └──────────────────┘
```

---

## Required Changes (by layer)

### Layer 1: Separate Corpus Configs (registry/)

Two `corpus.yaml` files instead of one:

```
registry/styles/christie/corpus.yaml    # 11 Christie novels
registry/styles/eliot/corpus.yaml       # 6 Eliot novels
registry/registries/cognition/christie-style-synthesizer.yaml
registry/registries/cognition/eliot-style-synthesizer.yaml
```

Each built independently via:
```bash
kask embed-corpus run --config registry/styles/christie/corpus.yaml ...
kask embed-corpus run --config registry/styles/eliot/corpus.yaml ...
```

### Layer 2: Dual-Retrieval ComposePath (crates/hkask-services/src/compose.rs)

New struct and service method:

```rust
pub struct DualComposeRequest {
    pub prompt: String,
    pub db_path: PathBuf,
    pub db_passphrase: String,
    pub author_a: String,          // "christie"
    pub author_b: String,          // "eliot"
    pub blend_ratio: f64,          // 0.0 = pure A, 1.0 = pure B, 0.5 = equal
    pub cognition_a: CognitionConfig,
    pub cognition_b: CognitionConfig,
    pub inference_ctx: InferenceContext,
    pub no_validate: bool,
}

pub struct DualComposeResult {
    pub generated_prose: String,
    pub exemplar_count_a: usize,
    pub exemplar_count_b: usize,
    pub centroid_distance_a: Option<f64>,
    pub centroid_distance_b: Option<f64>,
    pub blend_ratio: f64,
}
```

The `compose_dual()` method:
1. Embed prompt
2. KNN against `style:christie:` prefix → Christie exemplars
3. KNN against `style:eliot:` prefix → Eliot exemplars
4. Assemble system prompt with **routing instructions** (see Layer 3)
5. Generate prose
6. Validate against **both** centroids independently

### Layer 3: MoE Routing in System Prompt

The system prompt explicitly routes the model between experts:

```
You are Agatha Eliot — a Mixture of Experts narrator.

When writing facts, plot beats, dialogue, or structural transitions,
consult the CHRISTIE EXPERT exemplars below.

When rendering interior consciousness, moral reflection, free indirect
discourse, or the web of community relations, consult the ELIOT EXPERT
exemplars below.

## Christie Expert Passages (use for structure, facts, dialogue)
[exemplar_passages_a]

## Eliot Expert Passages (use for consciousness, moral depth)
[exemplar_passages_b]
```

### Layer 4: Jinja2 Template with MoE Macros

New template `registry/templates/composition/agatha-eliot-moe.j2` with:
- `{% macro christie_expert(exemplars) %}` — renders Christie exemplar block
- `{% macro eliot_expert(exemplars) %}` — renders Eliot exemplar block
- `{% macro moe_system_prompt(prompt, christie_passages, eliot_passages, ...) %}` — full MoE system prompt
- `{% macro forensic_declarative(...) %}` — Christie structural macros (from existing `.j2`)
- `{% macro free_indirect_interrogation(...) %}` — Eliot consciousness macros (from existing `.j2`)
- `{% macro widening_ripple(...) %}` — hybrid macros that combine both registers

### Layer 5: Replica MCP Server (hkask-mcp-replica)

New tool: `replica_mashup_moe` — accepts `author_a`, `author_b`, `prompt`,
`blend` (default 0.5). Internally calls `ComposeService::compose_dual()`.

### Layer 6: Script

```bash
bash embed-mashups.sh christie    # Build Christie corpus
bash embed-mashups.sh eliot       # Build Eliot corpus
```

---

## Decision: When to Trigger This

| Condition | Action |
|-----------|--------|
| Adjacent authors (e.g., Austen/Wilde) | Use single-blend Option A pattern |
| Distant authors (e.g., Christie/Eliot) | Use true MoE Option B |
| Heuristic: centroid cosine distance > 0.40 | Recommend Option B |

The heuristic can be checked at build time: after building both corpora,
compute `cosine_distance(christie_centroid, eliot_centroid)`. If > 0.40,
the `embed-mashups.sh` script warns and recommends MoE.

---

## Risks

1. **Dual validation may be overly strict.** Generated prose must be close to
   two potentially distant centroids simultaneously. A blended centroid
   (runtime interpolation) may be a more practical validation target.
   Mitigation: validate against a `blend_ratio`-weighted runtime centroid.

2. **Dual KNN doubles retrieval cost.** Two prefix-based KNN scans instead
   of one. Negligible at current corpus sizes (< 10,000 passages).

3. **MoE routing relies on model instruction-following.** The model may blend
   registers regardless of routing instructions. The system prompt must be
   tested iteratively for compliance.

---

## Files to Create/Modify (summary)

### New files
- `registry/styles/christie/corpus.yaml`
- `registry/styles/christie/.cache/.gitignore`
- `registry/styles/eliot/corpus.yaml`
- `registry/styles/eliot/.cache/.gitignore`
- `registry/registries/cognition/christie-style-synthesizer.yaml`
- `registry/registries/cognition/eliot-style-synthesizer.yaml`
- `registry/templates/composition/agatha-eliot-moe.j2`

### Modified files
- `crates/hkask-services/src/compose.rs` — add `compose_dual()`, `DualComposeRequest`, `DualComposeResult`
- `mcp-servers/hkask-mcp-replica/src/main.rs` — add `replica_mashup_moe` tool
- `embed-mashups.sh` — add `christie` and `eliot` dispatch, update `all`

---

## Migrating from Option A to Option B

Option A files remain intact. Option B is additive:
- Delete `registry/styles/agatha-eliot/` (the single blended corpus)
- Build separate Christie and Eliot corpora
- Dual retrieval composes from independent indices

The `agatha-eliot` author string in Option A becomes the MoE composite
name in Option B — the system prompt still says "You are Agatha Eliot..."
but now retrieves from two distinct pools.
