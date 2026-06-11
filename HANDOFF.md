# Handoff — Inference Port Fix, Compose Validation & Remaining Work

## Session Context

This session fixed the inference port endpoint (pre-existing bug that blocked all non-streaming inference), validated the full embed+compose pipeline end-to-end, fixed cognition YAML structural mismatches, and deleted a stale manifest. All 57 unit tests pass. The compose pipeline is working — Hemingway-style prose was generated. The remaining issues are: exemplar retrieval returns 0 passages (tuning), `think: false` is not forwarded by Okapi for qwen3 models (Okapi limitation), and the four MEDIUM architectural questions from the prior session are unresolved.

---

## What Was Done

### Inference port — three-part fix (`crates/hkask-templates/src/inference_port.rs`)

1. **Wrong endpoint fixed:** `/api/generate` → `/v1/chat/completions`. The `OkapiRequest`/`OkapiResponse` structs were already OpenAI-compatible (`messages`/`choices`); only the URL was wrong.

2. **Missing `stream: false` fixed:** Without it, Okapi defaults to chunked transfer encoding for large responses. `response.json()` in reqwest cannot collect a streaming body as a single JSON blob → "error decoding response body". Added `Some(false)` to all three non-streaming `build_request` call sites (`generate`, `generate_with_model`, `generate_vision`).

3. **`think: bool = false` added to `OkapiRequest`:** qwen3 models spend all tokens on internal reasoning and produce empty visible content without this field. Non-thinking models (llama3.1, deepseek, etc.) silently ignore it. Added to `build_request`.

4. **`OkapiConfig::for_inference()` added** (`crates/hkask-templates/src/okapi_config.rs`): 120-second timeout instead of the default 30s, to accommodate model cold-start (10–30 s) before generation begins.

5. **`InferenceService::resolve_port()` updated** (`crates/hkask-services/src/inference.rs`): uses `OkapiConfig::for_inference()` instead of `OkapiConfig::default()`.

6. **Three pinning tests added** to `inference_port.rs`:
   - `okapi_response_deserializes_chat_completions_format` — deserializes actual Okapi wire format including unknown fields (`id`, `object`, `created`, `system_fingerprint`, `index`, `reasoning`)
   - `okapi_request_serializes_think_false` — asserts `think: false` and `stream: false` are present in serialized request
   - `okapi_config_for_inference_has_extended_timeout` — asserts 120s timeout

### Cognition YAML structural fix

`CognitionConfig` expects `validation` at the top level, but two YAMLs had it nested under `embedding`. Fixed in:
- `registry/registries/cognition/hemingway-style-synthesizer.yaml`
- `registry/registries/cognition/woolf-style-synthesizer.yaml`

The three mashup YAMLs (`agatha-eliot`, `jane-wilde`, `ulysses-s-twain`) were already correct.

### Stale manifest deleted

`registry/manifests/style-corpus-embed.yaml` described an 8-step MCP-tool orchestration pipeline (v0.23.0 era) that no longer reflects reality. The actual pipeline is the Rust `EmbedService::embed_corpus` function. Deleted.

### End-to-end validation

- **Embed:** `kask embed-corpus run` against Hemingway corpus → 1,832 passages embedded, 1,360 earned triple storage, 472 embedding-only, 27,564 triples stored, centroid computed from 1,827 prose passages. Entity tags, method signals, salience scores, and budget gate all working correctly.
- **Compose:** `kask compose run` with `deepseek-v4-flash:cloud` model → Hemingway-style war scene prose generated. Jinja2 template rendered with style rules. Pipeline ran: YAML parse → DB open → prompt embed → KNN search → exemplar lookup → Jinja2 render → inference → output.

### Test status

| Crate | Tests |
|-------|-------|
| `hkask-templates` | 14/14 ✅ (3 new inference port tests) |
| `hkask-services` | 29/29 ✅ (1 new embed budget test from prior session) |
| `hkask-memory` | 14/14 ✅ |

---

## What Remains

### HIGH — Exemplar retrieval returns 0 passages

**Symptom:** Compose runs without exemplars; the Jinja2 `{% if exemplars %}` block is skipped. CLI warns "No exemplar passages found."

**Probable cause:** The distance threshold (`distance_threshold: 0.30` in the cognition YAML) is too tight for instruction-style prompts. The prompt "Write a war scene in the style of Hemingway." is an instruction, not prose — its embedding sits further from corpus passages than 0.30 cosine distance.

**Investigation steps:**
1. Add a debug log in `ComposeService::compose()` just after `search_similar` to print the top-5 distances returned, regardless of threshold. (Check `crates/hkask-services/src/compose.rs` around line 182.)
2. If distances are all > 0.30, loosen `distance_threshold` to 0.50 in `registry/registries/cognition/hemingway-style-synthesizer.yaml` and retry.
3. Alternatively, embed the prompt as prose ("The men walked toward the river. It was hot.") to confirm exemplar retrieval works at all.

**Where:** `crates/hkask-services/src/compose.rs` (retrieval filter loop, ~line 188–213), and `registry/registries/cognition/hemingway-style-synthesizer.yaml` (`retrieval.distance_threshold`).

### HIGH — `think: false` not forwarded by Okapi for qwen3

**Symptom:** `think: false` is correctly serialized in the request body, but Okapi does not forward it to the underlying Ollama model through `/v1/chat/completions`. qwen3 models spend all tokens on reasoning, produce empty visible content, and block the Okapi queue with stuck thinking requests.

**Root cause:** This is an Okapi proxy limitation, not an hKask bug. Okapi's `/v1/chat/completions` handler does not pass the `think` field through to Ollama's native chat API.

**Workaround (works now):** Use non-thinking models: `deepseek-v4-flash:cloud`, `llama3.1:8b`, `ministral-3:8b`, `lfm2.5:8b`, etc.

**Proper fix:** File an issue or PR with mdz-axo/Okapi to forward unknown fields from `/v1/chat/completions` to Ollama's native `/api/chat`, or expose a dedicated `think` parameter. Until then, the hKask inference port is correct — the field is being sent.

**If stuck qwen3 is blocking Okapi queue**, unload it with:
```bash
curl -s -X POST http://127.0.0.1:11435/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model":"qwen3:4b","keep_alive":0}'
```

### MEDIUM — Four open architectural questions (academic author pipeline)

These were identified in the prior session and remain unresolved. Design decisions are already documented — no implementation needed yet, but they are blockers for the academic author pipeline.

1. **Content acquisition for non-Gutenberg sources** — `download_text()` is HTTP GET → plaintext only. For PDFs (arXiv), HTML (institutional pages), YouTube transcripts: pre-process into `.cache/{slug}.txt` using existing MCP tools (`hkask-mcp-markitdown` for PDFs, `hkask-mcp-research`'s `web_extract` for HTML). No code change to `embed.rs` needed — cache-first logic already checks for the `.txt` file.

2. **Entity model for academic corpora** — Current `EntityConfig` is literary (characters, places, events, concepts). For academics: co-authors, venues, topics, paradigms. Method signals (parataxis ratio, dialogue ratio) are meaningless for academic prose. Add `corpus_type: "literary" | "academic"` to `CorpusConfig` in `crates/hkask-services/src/embed.rs`. When `"academic"`, use different entity categories and route to academic-specific method signals in `crates/hkask-memory/src/salience.rs`.

3. **Work enumeration for academic authors** — Agent-driven orchestration of existing research MCP tools (`web_search`, `web_extract`, `web_find_similar`). No new MCP tool needed. The Curator orchestrates discovery.

4. **Disambiguation confirmation boundary** — At the agent/Curator level, not inside MCP tools. MCP tools are stateless; disambiguation requires conversation state. Standard pattern: agent presents candidates → user confirms → agent proceeds.

### LOW — Woolf corpus not yet embedded

The Woolf corpus (`registry/styles/woolf/corpus.yaml`) has entities and methods declared (from the prior session) but has never been run through the embed pipeline. Run:
```bash
kask embed-corpus run \
  --config registry/styles/woolf/corpus.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass \
  --okapi-url http://127.0.0.1:11435
```
Then run compose with `registry/registries/cognition/woolf-style-synthesizer.yaml` to validate the second synthesizer.

### LOW — Exemplar retrieval text fallback produces placeholder strings

When a passage is in the embedding-only set (not budget-selected for triples), the compose pipeline falls back to `"[work_title: entity_ref]"` strings as exemplars. These are useless as style exemplars. Two options: (a) lower budget threshold so more passages get `text` triples, or (b) store `text` triples for ALL passages regardless of budget and only gate the richer metadata (entity tags, method signals, salience) on the budget. The `text` triple is tiny (~150 words) compared to the 11 method signals and entity tags. This is an architectural question for the budget design.

---

## Key Decisions to Preserve

1. **`/v1/chat/completions` is the canonical inference endpoint.** The request format (messages-based) and response format (choices-based) were already OpenAI-compatible. The old `/api/generate` Ollama-native endpoint returns a completely different response schema. Do not revert to `/api/generate`.

2. **`stream: false` must be explicit in all non-streaming requests.** Without it, Okapi defaults to chunked transfer encoding for large responses (`max_tokens >= ~50`). `response.json()` in reqwest cannot collect a streaming body. This is not a quirk to work around — it is correct API usage.

3. **`think: false` always sent in `OkapiRequest`.** Non-thinking models ignore it. Thinking models (qwen3, deepseek-r1) disable CoT when they receive it. This is the correct default for any structured generation task. If thinking mode is ever desired (e.g., for a reasoning-heavy Curator task), it must be exposed as an explicit opt-in, not by removing `think: false` globally.

4. **`OkapiConfig::for_inference()` at 120s timeout, not `OkapiConfig::default()` at 30s.** Embedding uses 30s (batched GPU matmul, bounded). Chat inference uses 120s to cover model cold-start (10–30 s) + generation. Do not conflate these.

5. **`validation` is a top-level key in `CognitionConfig` YAML, not nested under `embedding`.** The `CognitionConfig` struct deserializes `embedding` (model, dim, centroid_ref, retrieval) and `validation` (centroid_distance_max) as sibling keys. The nested form was a structural bug — fixed in hemingway and woolf YAMLs.

6. **Salience formula is `(one_hop + two_hop/2) / 2`.** Budget gates triple storage, not embedding. All passages get vectors; only budget-selected passages get metadata triples. Foundational rules always get triples regardless of budget. From the prior session — do not alter.

---

## Recommended Skills

- **coding-guidelines** — Before any code changes. Especially for the `corpus_type` academic branching work.
- **tdd** — For the exemplar retrieval debug work. Write a test against a known embedded corpus before changing thresholds.
- **diagnose** — For the exemplar retrieval 0-count issue. Reproduce with a debug log before changing threshold config.

---

## Commands

```bash
# Health check
cargo check -p hkask-templates -p hkask-services -p hkask-memory
cargo test -p hkask-templates && cargo test -p hkask-services && cargo test -p hkask-memory

# Unload stuck qwen3 if Okapi queue is blocked
curl -s -X POST http://127.0.0.1:11435/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model":"qwen3:4b","keep_alive":0}'

# Embed Woolf corpus (not yet done)
kask embed-corpus run \
  --config registry/styles/woolf/corpus.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass \
  --okapi-url http://127.0.0.1:11435

# Compose — use a non-thinking model until Okapi forwards think:false
OKAPI_MODEL=deepseek-v4-flash:cloud kask compose run \
  --prompt "Write a war scene in the style of Hemingway." \
  --cognition registry/registries/cognition/hemingway-style-synthesizer.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass \
  --okapi-url http://127.0.0.1:11435

# Debug exemplar retrieval — widen distance threshold to test
# Edit registry/registries/cognition/hemingway-style-synthesizer.yaml:
#   retrieval.distance_threshold: 0.50  (from 0.30)
# then re-run compose above
```
