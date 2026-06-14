---
title: "MCP Media Server — Design & Implementation Plan"
version: "0.27.0"
audience: [architects, developers]
last_updated: 2026-06-13
status: "Draft"
domain: "Technology"
mds_categories: [domain, composition, lifecycle]
---

# MCP Media Server — Design & Implementation Plan

**Goal:** Transform `hkask-mcp-media` from a thin fal.ai API proxy (9 passthrough tools) into a deep media server with two tool families: Image Gallery Management and Short Video/GIF Creation — layered on open-weight vision LLMs via the hKask inference router, coordinated through Jinja2 manifest templates.

---

## 1. Architecture Decision: Inference Router Integration

### Current State
`hkask-mcp-media` calls fal.ai directly via `fal_post` / `queue_post`. It has no access to the hKask inference router (`hkask-inference`), which provides multi-provider vision model access (DeepInfra, Fireworks, fal.ai, Ollama).

### Decision
**Add `hkask-inference` as a dependency.** The media server instantiates its own `InferenceRouter` at bootstrap. Vision "understanding" tasks (detection, captioning, classification, tagging) route through the inference router. Generation tasks (image gen, video gen, upscale) continue using fal.ai directly since those are specialized diffusion/transformer models not available through the standard chat completions API.

```
┌─────────────────────────────────────────────────────┐
│                  hkask-mcp-media                     │
│                                                      │
│  ┌──────────────────┐    ┌────────────────────────┐ │
│  │  Gallery Tools   │    │   Video/GIF Tools      │ │
│  │  (Family A)      │    │   (Family B)           │ │
│  └──────┬───────────┘    └──────┬─────────────────┘ │
│         │                       │                    │
│         ▼                       ▼                    │
│  ┌──────────────────┐    ┌────────────────────────┐ │
│  │ InferenceRouter  │    │  fal.ai direct (HTTP)  │ │
│  │ (vision models)  │    │  (minimax, flux, etc.) │ │
│  └──────────────────┘    └────────────────────────┘ │
│                                                      │
│  ┌──────────────────┐                               │
│  │ Template Engine  │  ← Jinja2 manifests from      │
│  │ (hkask-templates)│    hkask-templates             │
│  └──────────────────┘                               │
└─────────────────────────────────────────────────────┘
```

**Rationale:**
- Vision LLMs (Llama 3.2 Vision, Qwen2-VL, Gemma 4, Pixtral) are already available through the inference router across DeepInfra, Fireworks, and Ollama
- These models handle detection, captioning, classification, and tagging without needing specialized endpoints
- fal.ai's specialized endpoints (FLUX, MiniMax, Hunyuan3D) remain the generation backbone
- Template manifests make prompt engineering configurable and versionable per P1 (no trait without two consumers — templates are the consumer)

---

## 2. Template Manifest System

### Concept
Each media tool is backed by a Jinja2 manifest (YAML frontmatter + Jinja2 body) loaded via `hkask-templates`. The manifest defines:
- The prompt template (rendered with tool parameters)
- The vision model to use (with provider prefix)
- Output parsing instructions
- Fallback behavior

### Example: `gallery_detect_objects` manifest
```yaml
# manifests/media/detect_objects.yaml
template_type: WordAct
name: media_detect_objects
version: "0.27.0"
description: "Detect and label objects in an image"
model: "DI/meta-llama/Llama-3.2-11B-Vision-Instruct"
parameters:
  - name: detail_level
    type: enum
    values: [basic, detailed]
    default: detailed
  - name: max_objects
    type: integer
    default: 20
---
Analyze this image and detect all visible objects.

{% if detail_level == "detailed" %}
For each object, provide:
1. Object name
2. Bounding box description (e.g., "upper-left", "center", "lower-right")
3. Confidence level (high/medium/low)
4. Brief description of appearance (color, size, condition)
{% else %}
List each object with its name and general location in the image.
{% endif %}

Limit to {{ max_objects }} most prominent objects.

Return as JSON array with fields: name, location, confidence, description.
```

### Manifest Loading
At server init, `MediaServer` loads manifests via `hkask_templates::ManifestExecutor`:
```rust
pub struct MediaServer {
    webid: WebID,
    replicant: String,
    daemon: Option<DaemonClient>,
    fal_client: reqwest::Client,        // fal.ai direct
    inference: InferenceRouter,          // vision LLMs
    manifests: ManifestRegistry,         // prompt templates
    gallery_state: Option<GalleryState>, // gallery config
}
```

---

## 3. Tool Families

### Family A: Image Gallery Management (11 tools)

These tools manage a local image directory, using vision LLMs for understanding and organizing.

| # | Tool | Description | LLM Used | fal.ai Used |
|---|------|-------------|----------|-------------|
| A1 | `gallery_init` | Initialize gallery (path, mode: original/copy) | — | — |
| A2 | `gallery_scan` | Scan directory, index images with checksums | — | — |
| A3 | `gallery_info` | Get gallery status (count, size, config) | — | — |
| A4 | `gallery_detect_objects` | Detect objects in image(s) | Vision LLM (router) | — |
| A5 | `gallery_detect_faces` | Detect and group faces | Vision LLM (router) | — |
| A6 | `gallery_caption` | Generate descriptive caption | Vision LLM (router) | — |
| A7 | `gallery_tag` | Auto-tag with categories/keywords | Vision LLM (router) | — |
| A8 | `gallery_search` | Semantic search by description | Embedding model (router) | — |
| A9 | `gallery_classify` | Classify into predefined categories | Vision LLM (router) | — |
| A10 | `gallery_collage` | Create collage from selected images | — | FLUX (fal.ai) |
| A11 | `gallery_derivative` | Create derivative art / style transfer | — | FLUX img2img |

#### A1: `gallery_init`
```
Initialize or reconfigure a gallery.

Parameters:
  path: string (required) — Absolute path to gallery folder
  mode: enum {original, copy} (default: original)
        original = files are read-only, never modified
        copy = files can be edited, originals preserved elsewhere

Side effects:
  - Creates .hkask-gallery/ subdirectory for index, tags, metadata
  - Creates SQLite index database
  - Sets gallery_state on server
```

#### A2: `gallery_scan`
```
Scan gallery directory for new/changed/removed images.
Computes SHA-256 checksums for deduplication.

Parameters:
  recursive: bool (default: true)
  extensions: string[] (default: ["jpg","jpeg","png","webp","gif","bmp","tiff"])

Returns:
  { added: u32, removed: u32, unchanged: u32, total: u32 }
```

#### A3: `gallery_info`
```
Return gallery summary.

Returns:
  { path, mode, image_count, total_size_bytes, last_scan, tags_count }
```

#### A4: `gallery_detect_objects`
```
Run object detection on one or more images using a vision LLM.

Parameters:
  images: string[] — Image filenames or "all" / "new" / "untagged"
  detail_level: enum {basic, detailed} (default: detailed)
  max_objects: u32 (default: 20)

Returns:
  Per-image object lists with name, location, confidence, description.
  Results persisted to gallery index for search/filtering.
```

**Manifest:** `media_detect_objects` (Jinja2, shown above)

#### A5: `gallery_detect_faces`
```
Detect faces across gallery images. Groups similar faces.

Parameters:
  images: string[] — Image filenames or "all" / "new"

Returns:
  Per-image face detections with:
  - face_id (unique across gallery)
  - location in image
  - group (clustered by similarity)
  - estimated age_range, gender (if model supports)
```

**Manifest:** `media_detect_faces`

#### A6: `gallery_caption`
```
Generate descriptive captions for images.

Parameters:
  images: string[]
  style: enum {descriptive, artistic, technical, alt_text} (default: descriptive)

Returns:
  Per-image captions persisted to gallery index.
```

**Manifest:** `media_caption`

#### A7: `gallery_tag`
```
Auto-generate tags/keywords for images.

Parameters:
  images: string[]
  taxonomy: string (optional) — Custom tag vocabulary
  max_tags: u32 (default: 10)

Returns:
  Per-image tag lists with confidence scores.
```

**Manifest:** `media_tag`

#### A8: `gallery_search`
```
Semantic search over gallery images by natural language description.

Parameters:
  query: string — "sunset over mountains with lake"
  limit: u32 (default: 10)

Implementation:
  - Uses embedding model (via inference router) to embed query
  - Compares against stored image embeddings (generated during scan/caption)
  - Returns ranked results with similarity scores
```

#### A9: `gallery_classify`
```
Classify images into predefined or discovered categories.

Parameters:
  images: string[]
  categories: string[] (optional) — If empty, auto-discover categories

Returns:
  Per-image category assignments with confidence.
```

**Manifest:** `media_classify`

#### A10: `gallery_collage`
```
Create a collage from selected images using FLUX for layout/composition.

Parameters:
  images: string[] — Up to 9 images
  layout: enum {grid, mason, freeform, artistic} (default: grid)
  caption: string (optional)
  output_path: string (optional)

Implementation:
  - Composes images into a single prompt for FLUX image-to-image
  - Uses fal.ai FLUX dev/pro for generation
```

#### A11: `gallery_derivative`
```
Create derivative art from an image (style transfer, remix, variation).

Parameters:
  image: string
  style: string — Style description or reference
  strength: f32 (0.0–1.0, default: 0.7) — How much to transform
  variations: u32 (default: 1)

Implementation:
  - Uses fal.ai FLUX image-to-image with style prompt
  - Optionally generates multiple variations with different seeds
```

---

### Family B: Short Video / GIF Creation (8 tools)

These tools create and edit short videos (<60s), focused on meme generation and viral content.

| # | Tool | Description | LLM Used | fal.ai Used |
|---|------|-------------|----------|-------------|
| B1 | `video_from_image` | Image → short video clip | — | MiniMax img2video |
| B2 | `video_from_images` | Image sequence → video/GIF | — | — (local ffmpeg) |
| B3 | `video_to_gif` | Video segment → GIF | — | — (local ffmpeg) |
| B4 | `video_trim` | Trim video to segment | — | — (local ffmpeg) |
| B5 | `video_meme` | Create meme video (image + text + effects) | Vision LLM | MiniMax |
| B6 | `video_add_text` | Add text overlay to video | — | — (local ffmpeg) |
| B7 | `video_caption` | Generate description of video content | Vision LLM | — |
| B8 | `video_concat` | Concatenate multiple clips | — | — (local ffmpeg) |

#### B1: `video_from_image`
```
Create a short video from a single image using AI motion generation.

Parameters:
  image: string — Source image path
  prompt: string — Motion description
  duration: u32 (default: 6, max: 60) — Seconds
  resolution: enum {512p, 768p, 1080p} (default: 768p)
  end_image: string (optional) — Last frame image

Implementation:
  - Uses fal.ai MiniMax Hailuo image-to-video
  - Currently: fal-ai/minimax/video-01-live for text-to-video
  - New: fal-ai/minimax/hailuo-02/standard/image-to-video
```

#### B2: `video_from_images`
```
Create a video or GIF from a sequence of images.

Parameters:
  images: string[] — Ordered image paths
  fps: u32 (default: 24)
  format: enum {mp4, gif, webp} (default: mp4)
  output_path: string (optional)

Implementation:
  - Uses ffmpeg to encode image sequence
  - No AI needed — pure media encoding
```

#### B3: `video_to_gif`
```
Convert a video segment to GIF format.

Parameters:
  video: string — Source video path
  start_time: f32 (seconds)
  duration: f32 (seconds, max: 15)
  width: u32 (optional) — Resize width
  fps: u32 (default: 10)
  quality: enum {small, medium, large} (default: medium)

Implementation:
  - ffmpeg with palettegen + paletteuse for quality GIFs
```

#### B4: `video_trim`
```
Trim a video to a specific segment.

Parameters:
  video: string
  start_time: f32 (seconds)
  duration: f32 (seconds)
  output_path: string (optional)

Implementation:
  - ffmpeg stream copy (no re-encode when possible)
```

#### B5: `video_meme`
```
Create a meme video: take an image, add text overlay, generate motion.

Parameters:
  image: string — Base image
  top_text: string (optional)
  bottom_text: string (optional)
  motion_prompt: string — "slow zoom in", "shake", "spin", etc.
  duration: u32 (default: 3)

Implementation:
  1. Use vision LLM to suggest optimal text placement (avoiding faces/objects)
  2. Add text overlay to image (local image processing)
  3. Generate video from annotated image via MiniMax with motion prompt
```

#### B6: `video_add_text`
```
Add text overlay to an existing video.

Parameters:
  video: string
  text: string
  position: enum {top, bottom, center, custom} (default: bottom)
  font_size: u32 (default: 24)
  start_time: f32 (default: 0)
  duration: f32 (default: entire video)
  color: string (default: "white")

Implementation:
  - ffmpeg drawtext filter
```

#### B7: `video_caption`
```
Generate a description of video content using a vision LLM.

Parameters:
  video: string — Path to video file
  style: enum {descriptive, summary, hashtags} (default: descriptive)

Implementation:
  - Extract keyframes from video (1 frame/second or scene detection)
  - Send frames to vision LLM for description
  - Aggregate into coherent caption
```

**Manifest:** `media_video_caption`

#### B8: `video_concat`
```
Concatenate multiple video clips into one.

Parameters:
  videos: string[] — Ordered video paths
  output_path: string (optional)
  transition: enum {none, fade, wipe} (default: none)

Implementation:
  - ffmpeg concat demuxer
```

---

## 4. Model Catalog

### Vision LLMs (via Inference Router) — for "understanding" tasks

| Model | Provider | Use Case | Cost (per 1M tokens) |
|-------|----------|----------|----------------------|
| `DI/meta-llama/Llama-3.2-11B-Vision-Instruct` | DeepInfra | Object detection, captioning, tagging | $0.345 |
| `DI/Qwen/Qwen2.5-VL-72B-Instruct` | DeepInfra | High-quality detection, detailed captions | ~$1.50 |
| `DI/google/gemma-4-26B-A4B-it` | DeepInfra | Multimodal, image+text input | $0.07 in / $0.34 out |
| `FA/paddleocr` | fal.ai | Document OCR, text extraction | — |
| `FA/nemotron-parse` | fal.ai | Document parsing | — |
| `OM/llava:13b` | Ollama (local) | Free local detection/captioning | Free |
| `OM/minicpm-v:latest` | Ollama (local) | Free local vision | Free |

### Generation Models (via fal.ai direct) — for "creation" tasks

| Model | Endpoint | Use Case |
|-------|----------|----------|
| `fal-ai/flux/schnell` | Text-to-image | Fast image generation |
| `fal-ai/flux/dev/image-to-image` | Image-to-image | Transformations, style transfer |
| `fal-ai/flux/pro` | Text-to-image | High-quality generation |
| `fal-ai/minimax/video-01-live` | Text-to-video | Video from prompt |
| `fal-ai/minimax/hailuo-02/standard/image-to-video` | Image-to-video | Video from image + prompt |
| `fal-ai/minimax/hailuo-2.3/pro/image-to-video` | Image-to-video | Pro quality, 1080p |
| `fal-ai/hunyuan3d` | Image-to-3D | 3D model generation |
| `fal-ai/imageutils/u2net` | Upscale | Image upscaling |

### Fallback Strategy
- If no cloud API keys configured, use Ollama for vision tasks (free, local)
- If `FA_API_KEY` is missing, generation tools are unavailable (return clear error)
- Gallery tools that only use local processing (scan, info, video_from_images, video_trim, video_concat) work without any API keys

---

## 5. Implementation Phases

### Phase 1: Foundation (Scaffold + Gallery Core)
**Estimated:** 1 session

1. Add `hkask-inference` and `hkask-templates` as dependencies
2. Restructure `FalServer` → `MediaServer` with inference router
3. Create manifest directory `manifests/media/`
4. Implement gallery state management (GalleryState struct, SQLite index)
5. Build `gallery_init`, `gallery_scan`, `gallery_info` (no AI needed)

**Deliverable:** Gallery can be initialized and images indexed.

### Phase 2: Vision Understanding Tools
**Estimated:** 1–2 sessions

1. Create vision prompt manifests (detect_objects, detect_faces, caption, tag, classify)
2. Implement `gallery_detect_objects` (vision LLM → structured JSON output)
3. Implement `gallery_caption` (vision LLM → descriptive text)
4. Implement `gallery_tag` (vision LLM → keyword arrays)
5. Implement `gallery_classify` (vision LLM → category assignment)
6. Implement `gallery_detect_faces` (vision LLM → face detection + grouping)

**Deliverable:** Gallery images can be analyzed, captioned, tagged, and classified.

### Phase 3: Gallery Creation Tools
**Estimated:** 1 session

1. Implement `gallery_search` (embedding-based semantic search)
2. Implement `gallery_collage` (FLUX image-to-image composition)
3. Implement `gallery_derivative` (FLUX style transfer / variations)

**Deliverable:** Gallery supports search and creative composition.

### Phase 4: Video / GIF Tools
**Estimated:** 1–2 sessions

1. Add ffmpeg dependency (subprocess or bindings)
2. Implement `video_from_images`, `video_to_gif`, `video_trim`, `video_add_text`, `video_concat` (local ffmpeg)
3. Implement `video_from_image` (MiniMax img2video)
4. Implement `video_meme` (composed: vision LLM + image processing + MiniMax)
5. Implement `video_caption` (keyframe extraction + vision LLM)

**Deliverable:** Full short video / GIF creation pipeline.

### Phase 5: Polish & Integration
**Estimated:** 1 session

1. Update all documentation references (mcp-tools-inventory.md, test-inventory.md, etc.)
2. Add `#[tool]` attributes and register in tool router
3. Integration testing with real API keys
4. CNS span emission for all new tools
5. Energy estimation updates in `table_energy_estimator.rs`

---

## 6. Dependency Additions

```toml
# mcp-servers/hkask-mcp-media/Cargo.toml additions
hkask-inference = { path = "../../crates/hkask-inference" }
hkask-templates = { path = "../../crates/hkask-templates" }
walkdir = { workspace = true }          # Directory scanning
image = { workspace = true }            # Image metadata (EXIF, dimensions)
sha2 = { workspace = true }             # File checksums
base64 = { workspace = true }           # Image encoding for vision LLMs
```

**ffmpeg:** Used as a subprocess (no Rust bindings). Detect at startup, tools gracefully degrade if not found. This follows the same pattern as `pdftoppm` in `hkask-mcp-markitdown`.

---

## 7. File Structure

```
mcp-servers/hkask-mcp-media/
├── Cargo.toml
├── manifests/
│   └── media/
│       ├── detect_objects.yaml
│       ├── detect_faces.yaml
│       ├── caption.yaml
│       ├── tag.yaml
│       ├── classify.yaml
│       └── video_caption.yaml
└── src/
    ├── main.rs              # Server bootstrap + tool registration
    ├── gallery/
    │   ├── mod.rs
    │   ├── state.rs         # GalleryState, init/scan/info
    │   ├── index.rs         # SQLite index for images/tags/metadata
    │   └── vision.rs        # Vision LLM wrapper (detect, caption, tag, classify)
    ├── video/
    │   ├── mod.rs
    │   ├── ffmpeg.rs        # ffmpeg subprocess wrappers
    │   └── generation.rs    # fal.ai video generation wrappers
    └── manifests/
        ├── mod.rs
        └── loader.rs        # Manifest loading + execution
```

---

## 8. Key Decisions

1. **Inference router, not direct HTTP.** Vision "understanding" tasks use the hKask inference router. This gives multi-provider fallback (DeepInfra → Fireworks → Ollama) and leverages existing auth/config infrastructure.

2. **Manifests as single source of truth.** Prompt templates live in YAML+Jinja2 manifests, not hardcoded strings. This makes prompts versionable, auditable, and swappable without recompilation (P1: templates are the consumer).

3. **ffmpeg as subprocess, not Rust binding.** Follows the `pdftoppm` pattern from `hkask-mcp-markitdown`. Graceful degradation when ffmpeg is not installed.

4. **Gallery index as SQLite.** Lightweight, no new dependencies. Schema: `images(id, path, checksum, width, height, format, added_at)`, `tags(image_id, tag, confidence, source)`, `captions(image_id, caption, style, model)`, `objects(image_id, name, location, confidence)`, `faces(image_id, face_id, group_id, location)`.

5. **Embeddings stored in gallery index.** `gallery_search` embeds images during scan/caption, stores vectors in SQLite. Search is cosine similarity over stored embeddings. Uses the same embedding model configured for `hkask-memory`.

6. **Server ID stays `"fal"`.** Tool dispatch and role assignment unchanged. The binary/crate is `hkask-mcp-media`, but the short server ID remains `"fal"` for backward compatibility.

---

## 9. Open Questions

1. **Face grouping algorithm:** Should face grouping be LLM-based (send pairs to vision model: "are these the same person?") or use traditional embedding clustering? LLM is more accurate but expensive at scale; embedding clustering is fast but less precise. **Decision (2026-06-14):** Start with vision LLM pairwise comparison for face matching against the registry. This is accurate and leverages existing infrastructure. Upgrade path: dedicated ONNX face embedding model (InsightFace/ArcFace) for sub-millisecond vector similarity search at scale.

2. **Gallery embedding model:** Should we use the same embedding model as `hkask-memory`, or allow gallery-specific configuration? **Proposal:** Use the same model by default (`HKASK_EMBEDDING_MODEL`), allow override via gallery config.

3. **Image storage for fal.ai:** fal.ai tools require image URLs, not local paths. Options: (a) require user to host images somewhere, (b) upload to temporary storage, (c) use base64 data URIs. **Proposal:** Support base64 data URIs for small images (<10MB), document that larger images need a URL.

4. **Batch processing limits:** How many images can be processed in a single tool call? Vision LLM calls are expensive (time + tokens). **Proposal:** Default max 10 images per call, configurable via settings. `gallery_detect_objects` on "all" images processes in batches with progress reporting.

---

## 10. Face Recognition System

**Added:** 2026-06-14 · **Status:** Active

### 10.1 Design Rationale

hKask is headless — no GUI, no dashboards. Face recognition needs a reference image per person, but there is no UI for users to upload or select faces. The solution: users place portrait-style reference images into their gallery (convention: `gallery/faces/` subfolder), then register them via MCP tools. This is headless-compatible, minimal, and puts the user in explicit control (Magna Carta P1: User Sovereignty).

### 10.2 Architecture

```
User drops reference     face_validate tool       face_register tool       gallery_refresh
portraits into           checks each image        stores in face_registry  (include_faces=true)
gallery/faces/           meets requirements       table with name          auto-matches detected
        │                       │                       │                  faces → registry
        ▼                       ▼                       ▼                      ▼
   Gallery scan          Vision LLM assesses:     INSERT INTO              For each detected face:
   indexes images         • Exactly 1 face?       face_registry            vision LLM compares
                          • Face ≥15% of img?     (id, first_name,         against each registry
                          • Frontal pose?         last_name,               entry: "same person?"
                          • Adequate lighting?    image_id,                → name + confidence
                          • No occlusion?         status, notes)           or "unmatched"
                          • Min resolution?
                                │
                          REJECT with structured
                          reasons if invalid
```

### 10.3 Face Registry Schema

```sql
CREATE TABLE IF NOT EXISTS face_registry (
    id          TEXT PRIMARY KEY,          -- UUID
    first_name  TEXT NOT NULL,
    last_name   TEXT NOT NULL,
    image_id    TEXT NOT NULL REFERENCES gallery_images(id) ON DELETE CASCADE,
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending | valid | rejected
    notes       TEXT NOT NULL DEFAULT '',          -- validation notes / rejection reasons
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

### 10.4 Reference Image Requirements

| Criterion | How Verified | Reject If |
|-----------|-------------|-----------|
| Exactly 1 face | Vision LLM face detection | 0 faces or >1 face detected |
| Face occupies ≥15% of image | `face_size` from detection ÷ image dimensions | Face too small for reliable matching |
| Frontal or near-frontal pose | Vision LLM assessment | Profile/side angle (matching degrades) |
| Adequate lighting | Vision LLM assessment | Heavy shadow, backlight, severe underexposure |
| No heavy occlusion | Vision LLM assessment | Sunglasses, masks, hands covering face |
| Minimum resolution | Image dimensions check | Face region < 112×112 px |

### 10.5 New Tools

| Tool | Description |
|------|-------------|
| `face_validate` | Validate a gallery image as a face reference. Returns structured pass/fail with reasons. |
| `face_register` | Register a validated face reference with a name (first_name, last_name). |
| `face_list` | List all registered faces in the registry. |
| `face_remove` | Remove a face from the registry by ID. |

### 10.6 Auto-Matching Integration

`gallery_refresh` with `include_faces: true` now includes an auto-matching step after face detection:

1. Detect faces in all images (existing `tag_faces` pipeline)
2. For each detected face, compare against all `valid` entries in `face_registry`
3. Comparison: vision LLM receives both face crops and answers "Are these the same person?" with confidence
4. Matched faces get named tags (e.g., `"name": "Alice Chen", "confidence": 0.94`)
5. Unmatched faces remain as unnamed face groups (user can later name via `gallery_name_face` or register via `face_register`)

### 10.7 Future Upgrade Path

Replace vision LLM pairwise comparison with dedicated ONNX face embedding model (InsightFace/ArcFace):
- Add `embedding` column (BLOB) to `face_registry`
- Extract embeddings at `face_register` time
- Use `sqlite-vec` for sub-millisecond vector similarity search
- Vision LLM retained as fallback for borderline cases (confidence < 0.85)

---

## 11. Related Documents

| Document | Relevance |
|----------|-----------|
| [`docs/plans/mcp-server-roadmap.md`](mcp-server-roadmap.md) | §2.3 — Value-add layer targets for media server |
| [`docs/status/mcp-tools-inventory.md`](../status/mcp-tools-inventory.md) | Current 9 fal tools catalog |
| [`docs/architecture/loop-architecture.md`](../architecture/loop-architecture.md) | L4 Communication loop mapping |
| [`AGENTS.md`](../../AGENTS.md) | Crate map, constraints, commands |
| [`docs/architecture/ADR-024-unified-registry.md`](../architecture/ADR-024-unified-registry.md) | Template registry architecture |
