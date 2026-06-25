# Handoff: MCP Tool Splits + Training Provider Architecture

**Date:** 2026-06-25
**Session scope:** Refactor bloated MCP server lib.rs files + redesign training provider architecture
**Status:** ✅ Complete — Provider architecture refactored (7 files, cloud-only hosts, harness injection). MCP media tools split into 4 groups (gallery/processing/audio/generation).

---

## 1. Session Context

This session tackled two workstreams: (1) splitting monolithic MCP server `lib.rs` files into tool-group modules, and (2) redesigning the training provider architecture to cleanly separate Host/Harness/BaseModel concerns. The MCP splits are done and tested. The provider redesign is architecturally clarified but implementation ran into complexity from sed-based extraction — file-by-file surgical edits are needed.

---

## 2. What Was Done

### 2.1 Adapter Router Split (`crates/hkask-adapter`)

**Before:** `adapter_router.rs` — 1,813-line monolith with 3 provider backends + trait + router + guard + tests all in one file.

**After:** 5 files under `crates/hkask-adapter/src/adapter_router/`:
- `mod.rs` (1,247 lines) — `AdapterProviderBackend` trait, `AdapterRouter`, `EndpointGuard`, tests
- `together.rs` (232 lines) — `TogetherAdapterBackend`
- `runpod.rs` (126 lines) — `RunpodAdapterBackend`
- `baseten.rs` (150 lines) — `BasetenAdapterBackend`
- `openai.rs` (66 lines) — shared `openai_compatible_infer()` helper

**Visibility:** `AdapterProviderBackend` changed from private → `pub(super)` so sub-modules can impl it. Provider structs changed from private → `pub(super)` for mod.rs access.

**Tests:** 44 passed, 0 failed (identical to baseline).

### 2.2 MCP Server Tool Splits

All three servers now use the established multi-file tool pattern:

```rust
// tools/group_name.rs
use crate::*;
#[tool_router(router = group_router, vis = "pub")]
impl ServerName { /* tools */ }

// lib.rs
impl ServerName {
    fn combined_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::group_a_router() + Self::group_b_router()
    }
}
#[rmcp::tool_handler(router = Self::combined_router())]
impl rmcp::ServerHandler for ServerName {}
```

| Server | Before | After | Reduction | Tool Groups | Tests |
|--------|-------:|------:|:---------:|:-----------:|:-----:|
| `hkask-mcp-docproc` | 2,016 | 1,029 | -49% | 3 (document, semantic, storage) | 68 ✅ |
| `hkask-mcp-companies` | 3,941 | 710 | -82% | 5 (financial_data, analysis, portfolio, analytics, valuation) | 107 ✅ |
| `hkask-mcp-media` | 3,559 | 2,259 | -37% | 4 (gallery, processing, audio, generation) | 26 ✅ |

### 2.3 Bug Hunt

Three parallel audits confirmed:
- **Zero tool loss** — all 9 (docproc) + 38 (companies) + 36 (media) tools preserved exactly
- **Zero import issues** — `use crate::*` resolves all types correctly from the crate root
- **Zero CNS span drift** — all `cns.*` tracing targets unchanged
- **Zero public API changes** — all `pub struct`/`pub use` re-exports unchanged

### 2.4 Deleted Artifacts

Removed 4 `.bak` files that were git-tracked refactoring debris (11,150 lines total).

---

## 3. What Remains

### HIGH — Training Provider Architecture Refactor

**File:** `mcp-servers/hkask-mcp-training/src/providers.rs` (2,829 lines, 0 tests pre-session, now has characterization tests)

**Architecture decision (this session):** The training system is a **triple** — Host × Harness × BaseModel:

```
Host (where)      ×  Harness (how)    ×  BaseModel (what)
───────────────────────────────────────────────────────────
Together AI          Axolotl only         Resolved at submit time
Runpod               Axolotl only         via TrainingJob.base_model
Baseten              Axolotl or Unsloth
```

**No local hosts.** Cloud-only deployment. The current `AxolotlProvider` and `UnslothProvider` are local subprocess hosts that should be deleted.

**Specific steps (in order):**

1. **Extract `providers/types.rs`** (~530 lines) — lines 1-508 (imports + enums + TrainingJob + params + TrainingHost trait + ProviderError + CompletionMetadata) + CostEstimate at L2646-2667. Pure extraction, no behavior changes.

2. **Extract `providers/harness.rs`** (~350 lines) — only the harness definitions:
   - `HarnessCapability` enum + `cns_span()` impl (L555-599)
   - `HarnessAdapter` trait (L601-626)
   - `AxolotlHarness` struct + `impl HarnessAdapter for AxolotlHarness` (L691-818)
   - `UnslothHarness` struct + `impl HarnessAdapter for UnslothHarness` (L968-1108)
   - **Do NOT include:** `AxolotlProvider`, `UnslothProvider`, or their `TrainingHost` impls

3. **Create `providers/together.rs`** — rename `TogetherProvider` to `TogetherHost`, add `harness: Box<dyn HarnessAdapter>` field. Constructor: `new(api_key: String, harness: Box<dyn HarnessAdapter>)`.

4. **Create `providers/runpod.rs`** — same pattern as Together.

5. **Create `providers/baseten.rs`** — replace baked-in `TrainerHarness` with `Box<dyn HarnessAdapter>`. Fix `submit()` to use `self.harness.render_config(&resolved_job)?` instead of `render_with_model()`. The HuggingFace model ID resolution stays in Baseten (host-specific concern).

6. **Create `providers/mod.rs`** with:
   - `TrainingHostConfig` — drops `axolotl_path`/`python_path` fields
   - `create_host(config, harness)` — receives harness from caller, returns host
   - `TrainingHostRouter` — simplified to wrap a single host (no cascade/fallback chain)
   - Re-exports for lib.rs compatibility
   - Characterization tests

7. **Update `lib.rs`** — change `TrainingHostConfig` construction to remove axolotl_path/python_path. Change `create_host` call to pass a harness constructed from `harness_id`.

### Pitfalls from previous attempts

- **sed-based extraction loses brace context** — use file-by-file editing, not sed ranges
- **`HarnessAdapter` visibility** — must be `pub` in harness.rs and re-exported from mod.rs
- **Baseten's `render_with_model`** — must be converted to use `render_config` with a cloned+modified `TrainingJob`
- **`#[async_trait::async_trait]` attribute** — ensure it doesn't get orphaned when removing `impl TrainingHost for AxolotlProvider`
- **`extract_model_size_multiplier`** — is `fn` (not `pub`), needed in tests; consider making it `pub(crate)` or moving tests to types.rs

### MEDIUM — Split mcp-media tools into sub-groups

`mcp-servers/hkask-mcp-media/src/tools/all_tools.rs` is a single 2,239-line file with all 36 tools. Should be split into 4 groups using the python-based brace-tracker approach that worked for docproc and companies:

```
tools/media_tools.rs → 4 files:
  gallery.rs (884-1736), generation.rs (1738-2232),
  processing.rs (2269-2705), templates.rs (2748-3114)
```

The python script at the end of this session correctly identified these boundaries. Just needs file extraction using `git show HEAD` ranges.

---

## 4. Recommended Skills and Commands

**Skills to activate:**
- `coding-guidelines` — enforce surgical changes, simplicity first
- `rust-expertise` — idiomatic Rust for trait design, visibility, module structure
- `essentialist` — verify each new module passes the deletion test

**Verification commands:**
```bash
# After each file extraction
cargo check -p hkask-mcp-training

# After all changes
cargo test -p hkask-mcp-training
cargo test -p hkask-adapter -p hkask-mcp-docproc -p hkask-mcp-companies -p hkask-mcp-media
cargo check  # full workspace
```

---

## 5. Key Decisions to Preserve

1. **Cloud-only training.** No local subprocess hosts. `AxolotlProvider` and `UnslothProvider` are deleted. Rationale: hKask is deployed as cloud servers; local training doesn't fit the deployment model.

2. **Harness injected at construction, not baked in.** Cloud hosts receive `Box<dyn HarnessAdapter>` in their constructor. The caller (lib.rs or mod.rs) selects the harness based on config. Rationale: enables clean host/harness separation and future harness additions.

3. **`TrainerHarness` is a rogue abstraction.** It was created for Baseten without using the existing `HarnessAdapter` trait. It must be deleted and Baseten must use the standard harness interface. Rationale: single HarnessAdapter trait, no per-host harness variants.

4. **`TrainingHostRouter` is a single-host wrapper, not a cascade.** No fallback chain. If the selected host is unavailable, it fails gracefully. Rationale: cloud-only deployment means no local fallback; the cascade pattern was for local-backup which no longer exists.

5. **`#[tool_router]` + `#[tool_handler]` pattern for multi-file tools.** This replaces `#[tool_router(server_handler)]` across all MCP servers. Each tool group gets its own `impl` block with a named router; they're combined via `Self::router_a() + Self::router_b()`. Rationale: `server_handler` cannot see tools in sub-modules; the explicit pattern enables modular tool organization.

6. **`use crate::*` in tool sub-modules.** This is the correct import pattern — it brings in all items (including private `use` declarations) from the crate root. Rationale: simpler than listing imports per tool file, and the tools were originally in the crate root so they naturally accessed everything there.

---

## 6. Completion (2026-06-25 follow-up session)

Both remaining workstreams completed in a single follow-up session.

### 6.1 Training Provider Architecture Refactor

**File structure** (`mcp-servers/hkask-mcp-training/src/providers/`):

| File | Lines | Contents |
|------|------:|----------|
| `mod.rs` | 360 | Module root: re-exports, `TrainingHostConfig` (no local paths), `create_host(harness)`, `TrainingHostRouter` (single-host), 12 tests |
| `types.rs` | 528 | All types: enums, `TrainingJob`, params, `TrainingHost` trait, `ProviderError`, `CompletionMetadata`, `CostEstimate` |
| `harness.rs` | 355 | `HarnessCapability`, `HarnessAdapter` trait, `AxolotlHarness`, `UnslothHarness` |
| `together.rs` | 318 | `TogetherHost` (was `TogetherProvider`) with `harness: Box<dyn HarnessAdapter>` field |
| `runpod.rs` | 293 | `RunpodHost` (was `RunpodProvider`) with `harness: Box<dyn HarnessAdapter>` field |
| `baseten.rs` | 302 | `BasetenHost` (was `BasetenProvider`) — uses `render_config()` instead of `render_with_model()` |

**Total:** 2,156 lines across 6 files (down from 2,829-line monolith).

**Deleted:** `AxolotlProvider`, `UnslothProvider` (local subprocess hosts), `TrainerHarness` (rogue abstraction), `LocalTrainingConfig` + local helper functions, `TrainingHostConfig.harness`/`.axolotl_path`/`.python_path` fields.

**`lib.rs` changes:** Imports updated for new types. `TrainingHostConfig` construction drops `harness`/`axolotl_path`/`python_path` fields. `create_host()` receives `harness: Box<dyn HarnessAdapter>` constructed from `harness_id`. `host_config.harness` reference replaced with standalone `harness_id`.

**Tests:** 33 passed, 0 failed (1 deleted — `trainer_harness_has_trl_hub_path` for removed `TrainerHarness`).

### 6.2 MCP Media Tool Split

**File structure** (`mcp-servers/hkask-mcp-media/src/tools/`):

| File | Lines | Tools | Router |
|------|------:|-------|--------|
| `gallery.rs` | 1,102 | 16 tools (organize, status, search, find-similar, refresh, describe, analyze, name-face, face-validate/register/list/remove, extract-object, timeline) | `gallery_router` |
| `processing.rs` | 722 | 13 tools (remove-background, apply-style, create-collage, video-clip/to-gif/caption/concat/remix/meme, image-to-video, video-from-images, video-caption) | `processing_router` |
| `audio.rs` | 339 | 6 tools (voice-design, generate-speech, transcribe, transcribe-bundle, audio-capture, record-and-transcribe) | `audio_router` |
| `generation.rs` | 91 | 4 tools (generate-image, transform-image, upscale-image, generate-video) | `generation_router` |

**`lib.rs` combined_router:** `Self::gallery_router() + Self::processing_router() + Self::audio_router() + Self::generation_router()`. Deleted: `tools/all_tools.rs` (2,239-line monolith).

**Tests:** 26 passed, 0 failed (identical to baseline).

### 6.3 Validation

| Crate | Tests | Result |
|-------|------:|--------|
| `hkask-adapter` | 44 | ✅ Pass |
| `hkask-mcp-media` | 26 | ✅ Pass |
| `hkask-mcp-training` | 33 | ✅ Pass |
| `cargo check` (affected crates) | — | ✅ Clean (0 errors, 0 warnings) |

**Note:** `hkask-cli` has 7 pre-existing compile errors (missing types/fields in `repl/`) unrelated to these changes.
