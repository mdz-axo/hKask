# Continuation Prompt — hKask Media Server

**Date:** 2026-06-14
**Session scope:** Media server consolidation, analysis tools, voice/audio integration
**Status:** 28 tools implemented, 12 tests passing, compiles clean

---

## Recommended Skills

Load these before starting work:

| Skill | Why |
|-------|-----|
| `coding-guidelines` | Before any implementation — enforce simplicity, surgical changes, goal-driven execution |
| `essentialist` | Before adding anything — "always take away, never add." The server was just consolidated from 32→28 tools. Every new tool must survive the deletion test. |
| `pragmatic-semantics` | When making claims about system behavior — distinguish IS from OUGHT, declarative from subjunctive |
| `condenser-continuation` | If context resets — restores session state from this handoff |
| `rust-expertise` | For type-driven design, ownership patterns, async safety (MutexGuard across awaits) |

---

## Project Principles (from PRINCIPLES.md)

These are **non-negotiable**. Violations get deleted.

- **P1–P4 (Prohibitions):** User Sovereignty, Affirmative Consent, Generative Space, Clear Boundaries (OCAP). No admin override, no hidden settings, no data sharing without consent.
- **P5 (Guardrail):** Essentialism & Minimalism. Seek to remove, never to add. No stubs (`todo!()`, `unimplemented!()`), no `#[deprecated]`, no dead code. The deletion test: if you delete it, does behavior vanish?
- **P6 (Guideline):** Space for Replicants & Bots. Agents working with agents vs. agents working with humans.
- **P7 (Guardrail):** Evolutionary Architecture. Types emerge from usage, not speculation.
- **P8 (Guardrail):** Semantic Grounding. Every claim has provenance. ν-events are canonical.
- **P9 (Guardrail):** Homeostatic Self-Regulation. CNS spans (`cns.*` namespace) for observability.
- **P10–P12 (Guardrail/Prohibition):** Bot/Replicant taxonomy, public/private sphere, replicant host mandate — every action has an author.
- **Headless only.** No Grafana, dashboards, web frontends, GUIs. CLI/MCP/API only.
- **No monitoring stacks.** Prometheus, Alertmanager, external observability forbidden.

**Verification:**
```bash
cargo check --workspace
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"  # must be empty
grep -r "grafana\|prometheus\|dashboard" crates/ --include="*.rs"  # must be empty
```

---

## What Was Done

### Session 1 — Crate rename + scaffolding
- Renamed `hkask-mcp-fal` → `hkask-mcp-media` (17 locations across code and docs)
- Added `hkask-inference`, `hkask-templates`, `walkdir`, `sha2`, `base64`, `image`, `nom-exif` dependencies
- Created module structure: `gallery/`, `video/`, `templates.rs`
- Built `GalleryState` with scan/hash/dimensions, 7 tests

### Session 2 — Tool consolidation (32 → 28 tools)
- **Deleted 12 engineering artifacts:** `fal_ping`, `gallery_get_image`, `gallery_get_metadata`, `tag_faces`, `tag_objects`, `tag_colors`, `tag_composition`, `image_describe_scene`, `image_classify_style`, `fal_caption`, `gallery_set_root`, `gallery_scan`
- **Merged:** `gallery_set_root` + `gallery_scan` → `gallery_organize`, `fal_caption` + `image_describe_scene` → `describe_image`
- **Renamed:** `fal_generate_image` → `generate_image`, `fal_image_to_image` → `transform_image`, `fal_upscale` → `upscale_image`, `fal_generate_video` → `generate_video`, `gallery_info` → `gallery_status`
- **Added:** Tag persistence, EXIF extraction (`nom-exif`), collage composition (`image` crate), Levenshtein search
- **Added model fallback:** `resolve_vision_model()` — DeepInfra → Fireworks → Ollama chain

### Session 3 — Analysis + voice/audio tools
- **Added 4 analysis tools:** `gallery_analyze` (batch vision pipeline), `gallery_name_face` (name face groups), `extract_object` (Florence-2 segmentation), `gallery_timeline` (EXIF-based temporal grouping)
- **Added `segment_object`** to `FalBackend` and `InferenceRouter`
- **Audio tools** (built by parallel agent): `transcribe`, `transcribe_bundle`, `audio_capture`, `record_and_transcribe`
- **Voice tools** (built by parallel agent): `voice_design`, `generate_speech`, `VoiceDesign` type in `hkask-types`

---

## Current State

### File structure
```
mcp-servers/hkask-mcp-media/
├── Cargo.toml
├── manifests/media/          # Reference YAML/J2 manifests (embedded versions in templates.rs)
│   ├── caption.yaml, classify.yaml, detect_faces.yaml, detect_objects.yaml
│   ├── tag.yaml, video_caption.yaml, voice_design.j2
├── research/
│   ├── design-schema.md      # Gallery ERD, tool signatures, model routing table
│   └── media-landscape.md    # Model catalog (20+ models), dependency lattice
└── src/
    ├── main.rs               # MediaServer, 28 tools, ~2600 lines
    ├── templates.rs          # Embedded minijinja templates (8 templates)
    ├── gallery/
    │   ├── mod.rs, state.rs  # GalleryState, scan, 7 tests
    │   └── vision.rs         # Stub for future vision pipeline module
    └── video/
        ├── mod.rs, ffmpeg.rs # FfmpegRunner: clip, to_gif, add_caption, capture_audio,
        │                       images_to_video, concat, extract_keyframes
        └── generation.rs     # Stub for future video generation module
```

### Tool inventory (28 tools)

| Family | Tools |
|--------|-------|
| Gallery (3) | `organize`, `status`, `search` |
| Analysis (4) | `analyze`, `name_face`, `extract_object`, `timeline` |
| Image (4) | `describe`, `remove_background`, `apply_style`, `create_collage` |
| Video (7) | `clip`, `to_gif`, `image_to_video`, `add_caption`, `remix`, `concat`, `from_images` |
| Voice (2) | `design`, `generate_speech` |
| Audio (4) | `transcribe`, `transcribe_bundle`, `audio_capture`, `record_and_transcribe` |
| Generation (4) | `generate_image`, `transform_image`, `upscale_image`, `generate_video` |

### Tests: 12 (all pass)
- Gallery state: 7 (init 4, scan 2, info 1)
- Levenshtein: 5 (identical, different, case-insensitive, typo-tolerant, empty)

### Key internal helpers (not exposed as tools, used by analyze pipeline)
- `resolve_vision_model()` — async, probes available backends, returns best model
- `resolve_image_url()` — index → base64 data URI
- `resolve_image_id()` — index → SQLite image ID
- `persist_tag()` — store tag to GalleryStore
- `extract_exif()` — nom-exif EXIF reader (camera, date, GPS, etc.)
- `levenshtein_similarity()` — normalized 0.0–1.0 fuzzy string match
- `render_prompt()` — minijinja template rendering

### Inference router extensions
`hkask-inference` now has: `generate_image`, `image_to_image`, `remove_background`, `upscale`, `generate_video`, `image_to_video`, `generate_speech`, `segment_object` — all with multi-provider fallback.

---

## Remaining Tasks

### P1 — Auto-tagging on organize
**What:** When `gallery_organize` runs, it should automatically trigger `gallery_analyze` for new images. Currently they're separate steps — the user must call `gallery_analyze` after `gallery_organize`.
**Where:** `gallery_organize` method in `main.rs` — add an optional `auto_analyze: bool` parameter.
**Challenge:** The analyze pipeline runs vision LLMs which are expensive. Should be opt-in with a clear cost warning.

### P1 — "New" mode for gallery_analyze
**What:** `gallery_analyze(mode="new")` should only process images that don't have tags yet. Currently it processes all images because tag tracking per image isn't implemented.
**Where:** `gallery_analyze` method — check `gallery_store.get_tags(image_id)` to skip already-tagged images.

### P2 — gallery_find_similar (embedding search)
**What:** "Find images that look like this one" or "Find images matching this description" — embedding-based similarity search. Different from `gallery_search` (tag-based fuzzy).
**Design:** Three modes: (a) `image_index` → embed image, find nearest neighbors, (b) `text` → embed text, find nearest neighbors, (c) `generate_and_find` → generate image from text, embed it, search.
**Dependencies:** Needs embedding model via inference router, stored image embeddings in SQLite.

### P2 — video_meme (composed pipeline)
**What:** Image + text overlay + motion generation. "Make a meme video from this image with 'WHEN YOU SEE IT' text and a slow zoom."
**Design:** Compose vision LLM (suggest text placement avoiding faces) + imageproc (text overlay) + MiniMax (image-to-video with motion prompt).
**Where:** New tool in Video family.

### P3 — CNS spans + energy estimates
**What:** All tools should emit CNS spans and the energy estimator should reflect the new tool costs.
**Where:** `table_energy_estimator.rs`, add `tracing::info!` under `cns.*` namespace in each tool.

### P3 — MediaError enum
**What:** Replace ad-hoc string errors with a proper `MediaError` enum.
**Where:** New `error.rs` module.

### P3 — GalleryStore tag deduplication
**What:** `gallery_analyze` re-running on the same image creates duplicate tags. Should upsert by (image_id, tag_type, value).
**Where:** `GalleryStore::tag_image` in `hkask-storage`.

### Deferred — manifests/ cleanup
The `manifests/media/` directory contains YAML/J2 files that are reference docs. The actual templates are embedded in `templates.rs`. Decide whether to delete the manifests directory or keep as documentation.

---

## Build Commands

```bash
cargo check -p hkask-mcp-media          # Check media server
cargo test -p hkask-mcp-media            # Run tests (12)
cargo clippy -p hkask-mcp-media          # Lint
cargo check -p hkask-cli                 # Verify CLI still compiles
cargo check --workspace                  # Full workspace (hkask-mcp-docproc has pre-existing error)
```

## Key Design Decisions

1. **Inference router is the single dispatch point.** All model calls go through `InferenceRouter` — no direct HTTP to fal.ai from the media server.
2. **Templates are embedded, not loaded from disk.** `templates.rs` contains `const` strings compiled into the binary. The `manifests/` directory is reference documentation.
3. **ffmpeg as subprocess, not Rust binding.** Follows `pdftoppm` pattern from `hkask-mcp-docproc`. Graceful degradation when ffmpeg not installed.
4. **GalleryStore in hkask-storage.** SQLite-backed with `galleries`, `gallery_images`, `gallery_tags` tables. Tag persistence is best-effort (logs errors, doesn't fail the tool).
5. **`gallery_state` uses `Arc<Mutex<Option<GalleryState>>>`** because `#[tool]` methods take `&self`, not `&mut self`. Lock scope must not cross `.await` points.
6. **Server ID is "media".** Role assignment uses "media" (was "fal" in earlier versions — no backward compat requirement).
7. **Credentials are optional.** Server starts with any of DI_API_KEY, FA_API_KEY, or FW_API_KEY. Tools gracefully error if the required backend is unavailable.

---

## Related Documents

| Document | Relevance |
|----------|-----------|
| `docs/plans/mcp-media-server-design.md` | Original design plan with architecture decisions |
| `docs/status/mcp-tools-inventory.md` | Current tool catalog (28 media tools) |
| `docs/status/test-inventory.md` | Test coverage (12 media tests) |
| `docs/architecture/PRINCIPLES.md` | P1-P12 principles — must read before implementing |
| `docs/architecture/loop-architecture.md` | L4 Communication loop mapping |
| `AGENTS.md` | Crate map, commands, constraints |
| `research/media-landscape.md` | Model catalog (fal.ai + DeepInfra) |
| `research/design-schema.md` | Gallery ERD, tool signatures, model routing |
