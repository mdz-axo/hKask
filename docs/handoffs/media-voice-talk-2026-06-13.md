# Handoff: Media MCP Server + Voice/Talk/Listen — 2026-06-13

## Session Context

Built the complete hKask media MCP server (image, video, voice, listen) with centralized inference routing. All 24 tools are real implementations — zero stubs. Added VoiceDesign type, talk service (TTS), listen service (STT + audio capture), TranscriptBundle with word-level timestamps, and a TUI transcript viewer. ~90% of original 14-task plan complete. Remaining: tracer-bullet integration tests, collage composition, gallery tool consolidation, REPL `/listen` commands.

## What Was Done

### Architecture Fix
- **Centralized inference:** Media server has zero HTTP clients. All model calls route through `InferenceRouter`. Added media generation methods to `FalBackend` (6 methods), `DeepInfraBackend` (4 methods), and dispatch on `InferenceRouter` (7 methods). Removed `reqwest` dependency from media server.
- Files: `crates/hkask-inference/src/fal_backend.rs`, `deepinfra_backend.rs`, `inference_router.rs`, `mcp-servers/hkask-mcp-media/src/main.rs`, `Cargo.toml`

### Gallery Infrastructure
- **GalleryStore** (`crates/hkask-storage/src/gallery.rs`): SQLite-backed with `galleries`, `gallery_images`, `gallery_tags` tables. 5 public methods (`create`, `add_image`, `get_image`, `tag_image`, `get_tags`). 7 tests pass.
- **GalleryStore + GalleryState unified:** `gallery_set_root` creates SQLite record + stores `gallery_id` in `GalleryState`. `gallery_scan` persists discovered images to SQLite. `gallery_get_image`, `gallery_get_metadata`, `resolve_image_url`, `resolve_image_path` all read from SQLite — no filesystem walks.
- Files: `crates/hkask-storage/src/gallery.rs`, `mcp-servers/hkask-mcp-media/src/gallery/state.rs`, `main.rs`

### Template Resolution
- **Embedded Jinja2 templates** (`mcp-servers/hkask-mcp-media/src/templates.rs`): 8 templates compiled into binary, rendered at runtime via `minijinja`. All vision tools use rendered templates instead of hardcoded strings.
- **Registry templates** (`registry/templates/media/`): 13 files (6 WordAct `.j2` + 7 FlowDef `.yaml`) + 3 bundle manifests.

### Voice/Talk/Listen
- **VoiceDesign** (`crates/hkask-types/src/voice.rs`): 9-field struct with `to_elevenlabs_voice()` mapping to 21 preset voices. 2 tests pass.
- **Talk service:** `voice_design` tool (Llama 3.3 70B → VoiceDesign JSON), `generate_speech` tool (DI Kokoro-82M → fal ElevenLabs v3). VoiceDesign stored on `AgentPod` via `set_voice()`/`get_voice()`/`voice_description()`.
- **Listen service:** `audio_capture` (ffmpeg, platform-specific: alsa/avfoundation/dshow), `transcribe` (DI Whisper-large-v3 → fal Whisper), `transcribe_bundle` (word-level timestamps), `record_and_transcribe` (capture + transcribe linked).
- **TranscriptBundle** (`crates/hkask-types/src/transcript.rs`): `TimedWord`, `TranscriptSegment`, `TranscriptBundle` with `word_at_ms()` and `segment_at_ms()`. 2 tests pass.
- Files: `crates/hkask-types/src/voice.rs`, `transcript.rs`, `crates/hkask-agents/src/pod/mod.rs`, `crates/hkask-inference/src/deepinfra_backend.rs`, `fal_backend.rs`, `inference_router.rs`, `mcp-servers/hkask-mcp-media/src/main.rs`, `templates.rs`, `video/ffmpeg.rs`

### TUI Transcript Viewer
- **`kask transcript view <bundle.json>`**: Fullscreen TUI with word-level highlighting (Richmond Gold HC-41 #B79163), progress gauge, keyboard navigation (Space play/pause, arrows seek, j/k words, [] segments, Home/End, PgUp/PgDn). Audio via `ffplay` subprocess (no rodio/ALSA dependency).
- Files: `crates/hkask-cli/src/transcript_viewer.rs`, `cli/mod.rs`, `main.rs`, `Cargo.toml`, workspace `Cargo.toml`

### Pod/Role Wiring
- `media` added to `BUILTIN_SERVERS` in both REPL and CLI commands. Daemon flow uses `media` role.
- Files: `crates/hkask-cli/src/repl/builtin_servers.rs`, `commands/mcp.rs`, `mcp-servers/hkask-mcp-media/src/main.rs`

### Documentation
- Research document: `mcp-servers/hkask-mcp-media/research/media-landscape.md` (T1-T3 + voice/talk/listen architecture)
- Design document: `mcp-servers/hkask-mcp-media/research/design-schema.md` (T4-T5)

### Compilation & Test Status
- Full workspace compiles cleanly (zero warnings)
- 40 tests pass: 21 hkask-types, 7 hkask-storage, 12 hkask-mcp-media

## What Remains

### HIGH — Collage Composition
**File:** `mcp-servers/hkask-mcp-media/src/main.rs` (`image_create_collage` tool)

Currently returns a manifest. Needs programmatic image composition. Design direction from user:

> "You either create a collage based on a slug of words that search for elements that represent the words OR based on an image you are looking for things similar to OR based on a list of artifacts you want collaged — and then you may limit the number of max items in the collage, and you may choose between a few layout patterns because the layout of the collage will be done programmatically and the image assembled."

**Approach:**
1. Redesign `CreateCollageRequest` to support three modes:
   - `search_terms: Vec<String>` — semantic search gallery for matching images
   - `similar_to_index: usize` — find visually similar images
   - `image_indices: Vec<usize>` — explicit list (existing)
2. Add `max_items: usize` (default 6) and `layout: enum { grid, horizontal, vertical, masonry }`
3. Implement layout math using `image` crate: resize images to fit grid cells, compose onto canvas with spacing
4. No GUI — pure programmatic assembly, output is an image blob

### HIGH — T13 Tracer-Bullet Integration Tests
**Files:** `mcp-servers/hkask-mcp-media/tests/` (new directory)

Write one vertical integration test per tool family that:
1. Mocks the LLM endpoint (or uses a lightweight test-only model)
2. Creates a gallery → tags an image → reads back tags → derives a new image
3. Verifies memory encoding in dual channels (episodic + semantic)
4. Tagged `// REQ: media-{tool}-{property}`

### MEDIUM — REPL `/listen` Slash Commands
**Files:** `crates/hkask-cli/src/repl/handlers/` (new `listen.rs` or extend `mcp.rs`)

Add `/listen start`, `/listen stop`, `/listen view` commands that:
1. Call `audio_capture` tool on media MCP server
2. Call `record_and_transcribe` on stop
3. Save TranscriptBundle JSON to disk
4. Open TUI viewer on `view`

### MEDIUM — Gallery Tool Consolidation
**File:** `mcp-servers/hkask-mcp-media/src/main.rs`

Two gallery init tools coexist: `gallery_init` (old, uses `original`/`copy` modes) and `gallery_set_root` (new, uses `read-only`/`copy-on-write`/`destructive`). Consolidate into one: keep `gallery_set_root` with the 3-state policy, remove `gallery_init`.

### LOW — `record_and_transcribe` → TranscriptBundle
**File:** `mcp-servers/hkask-mcp-media/src/main.rs`

Currently returns raw transcript JSON. Should return full `TranscriptBundle` with word-level timings (like `transcribe_bundle` does). Requires parsing Whisper verbose_json response in the composite tool.

## Recommended Skills and Tools

- **coding-guidelines** — before any implementation
- **tdd** — for T13 integration tests
- **deep-module** — for collage layout module design

Commands:
```bash
cargo check -p hkask-mcp-media -p hkask-storage -p hkask-cli -p hkask-inference -p hkask-agents
cargo test -p hkask-storage -- gallery
cargo test -p hkask-mcp-media
cargo test -p hkask-types
```

## Key Decisions to Preserve

1. **Centralized inference only.** Media server must never have its own HTTP client. All model calls route through `InferenceRouter`. This was a hard architectural fix — do not reintroduce direct provider calls.

2. **Gallery is a lens, not a copy.** Images are indexed by path + hash in SQLite. The filesystem is the source of truth. `gallery_scan` persists discovered images; lookups read from SQLite. Never walk the filesystem for lookups.

3. **Voice presets, not prose.** TTS APIs (DeepInfra ElevenLabs-compatible, fal.ai ElevenLabs) expect preset voice names (Rachel, Aria, etc.), not free-text descriptions. `VoiceDesign::to_elevenlabs_voice()` maps structured characteristics to presets. Do not pass raw prose to TTS endpoints.

4. **ffplay for audio, not rodio.** Rodio requires ALSA dev headers (`libasound2-dev`) which may not be available. The TUI viewer uses `ffplay` subprocess (already available with ffmpeg). Keep this approach.

5. **Collage is programmatic, not GUI.** No visual editor. Layout is computed from parameters (grid/horizontal/vertical/masonry) using `image` crate math. Input modes: search terms, similar-to-image, or explicit index list.

6. **Three-state gallery policy.** `read-only` | `copy-on-write` | `destructive` — no gray zone. The old `original`/`copy` two-state mode should be consolidated into this.

7. **TranscriptBundle format is `hkask-transcript-v1`.** JSON with `audio_path`, `words[]` (start_ms/end_ms), `segments[]`, `full_text`. Frontends use `word_at_ms()` for highlighting and click-to-seek.

8. **Richmond Gold HC-41 (#B79163) for TUI highlights.** Not yellow. RGB(183, 145, 99). Used for word highlight background and progress bar gauge.
