# OCR Pipeline Deepening — Session Insights

**Date**: 2026-06-13  
**Scope**: `hkask-types`, `hkask-inference`, `hkask-mcp-markitdown`, `hkask-cli`, `hkask-api`  
**Tests**: 86 total (14 inference + 52 markitdown unit + 3 integration + 17 types)

---

## Architecture Decisions

### 1. Sealed Type Hierarchy (P3)
Every valid pipeline state is representable; every invalid state is unrepresentable. `PipelineOutcome` is the single sealed output — no partial state escapes. `VerificationReport::passed` is a computed field, never settable by consumers.

### 2. Sequential by Design (P1)
The pipeline is deliberately sequential to guarantee page ordering without reorder logic. Concurrency is a future optimization gated by empirical throughput data (< 2s per page is adequate).

### 3. Deterministic Routing (No Randomness)
`SamplingState` uses round-robin counters (`every_nth`), not randomness. This guarantees the ±5% dual-routing invariant without non-determinism. Property-tested over 1000 iterations.

### 4. Unified Fallback Path
Fallback is not a separate code fork — it reuses the same `route_page` logic with an `exclude_backend` flag. Fallback results carry `was_fallback: true` in `OcrResult` — first-class outputs.

### 5. Pluggable Backend Architecture
`OcrExecutor` trait isolates backend invocation. Tesseract (native CLI), LlmOcr (vision model via inference router). Backends are pluggable without pipeline changes. Adding a new backend requires zero pipeline code changes.

### 6. CNS Flow: NuEvent → NuEventStore → CurationLoop → CuratorAgent → Human
`MarkitdownCnsObserver` implements the real `CnsObserver` trait. After each pipeline run, `emit_pipeline_event()` constructs a `NuEvent` with `cns.pipeline` namespace and feeds it to the observer, which persists via daemon `store_experience`. Full CNS runtime registration is the future upgrade path when the daemon exposes observer subscription.

### 7. Self-Tuning Thresholds with P4 Consent
`calibration.rs` analyzes accumulated cross-validation data. When ≥100 Moderate-tier dual-routed pages show >95% mean similarity, emits a CNS alert suggesting threshold adjustment. **Never auto-adjusts** — human approval via `kask settings set ocr_moderate_max <value>` is the only valid path.

### 8. Cross-Validation Accumulation
`MarkitdownServer` maintains a `Mutex<Vec<CrossValidation>>` accumulator. Each pipeline run appends its cross-validations. When the accumulator crosses the 100-sample threshold with >95% mean similarity, a drift alert fires and the accumulator clears to avoid redundant alerts.

---

## fal.ai Integration

### Inference Router (`FA/` prefix)
`ProviderId::Fal` added to `hkask-inference` following the same pattern as `FW/` (Fireworks) and `DI/` (DeepInfra). `FalBackend` provides `generate`, `generate_vision`, `generate_stream` via OpenAI-compatible `/v1/chat/completions` at `https://api.fal.ai`. Auth: `Key` header (fal.ai convention, not `Bearer`).

**Static model catalog** (no `/v1/models` endpoint):
- `paddleocr` — PaddleOCR document OCR
- `nemotron-parse` — Nemotron Parse document parsing/OCR
- `docres` — DocRes document enhancement (deshadow, deblur, binarize, dewarp)

**Vision support heuristic**: `paddleocr` and `nemotron-parse` added to `infer_vision_support()` allowlist.

### Image Preprocessing (`fal-ai/docres`)
**Identified model**: `fal-ai/docres` — purpose-built for document cleanup. Endpoint: `https://fal.run/fal-ai/docres` (separate from chat completions). Tasks: `deshadowing`, `appearance`, `deblurring`, `binarization`. Also: `fal-ai/docres/dewarp` for folded documents.

**Current status**: `preprocess_via_fal()` is fully implemented as an async function. When `HKASK_FAL_API_KEY`/`FAL_KEY`/`FA_API_KEY` is set, it encodes the page image as a base64 data URI, POSTs to `fal.run/fal-ai/docres` with `binarization` task, downloads the enhanced image, and replaces the original. On any failure (no key, network error, bad response), falls back to local `stretch_contrast()`. Wired into `pdf_to_images()` which is now async.

**Cost**: $0.025/megapixel. At 150 DPI (~2 MP/letter page): ~$0.05/page.

**Concurrency**: fal.ai queue-based, starts at 2 concurrent, scales to 40. Requests never rejected.

**Latency**: ~60s per image (queue-based; first request includes cold start). Subsequent requests benefit from warm runners.

**Activation**: Set `HKASK_FAL_API_KEY`, `FAL_KEY`, or `FA_API_KEY`. No code changes needed.

**Live test** (2026-06-13): 400×100 text-like image → perfect binarization (2 unique values: {0, 255}). Dimensions preserved.

---

## Tradeoffs Made

| Decision | Rationale | Cost |
|----------|-----------|------|
| Sobel edge-density heuristic | Fast, deterministic, O(w·h) | Imperfect on symmetric patterns (regular grids cancel out) |
| Levenshtein over BLEU/ROUGE | Character-level edits match OCR error patterns | Less semantic, but cross-validation adds embedding similarity |
| `pdftoppm` subprocess over native Rust PDF renderer | `pdftoppm` is battle-tested, widely available | External dependency, falls back to raw bytes OCR when absent |
| `supports_vision` heuristic over runtime probing | Fast (static allowlist), no per-model API calls | Incomplete — misses unknown vision models |
| Word-count heuristic for verification | Simple, catches catastrophic failures | Doesn't catch subtle degradations (e.g., "cl" → "d") |
| Static fal.ai model catalog over live API | No standard `/v1/models` endpoint | Requires manual updates when new models ship |
| `Mutex` accumulator over persistent store | Simple, no storage dependency | Lost on server restart; future: persist to daemon |

---

## What Was Deferred (and Why)

| Item | Reason | Trigger to Revisit |
|------|--------|-------------------|
| fal.ai `docres` activation | No `FA_API_KEY` set; needs OCR accuracy benchmarks | API key available + ≥20 real scans to benchmark |
| CNS runtime registration for `MarkitdownCnsObserver` | Daemon doesn't expose observer subscription API | Daemon adds `subscribe_observer` endpoint |
| Persistent cross-validation accumulator | Current `Mutex<Vec<>>` lost on restart | Daemon/NuEventStore integration for persistence |
| Semantic verification depth (embedding vs. ground truth) | Current word-count catches high-signal failures | Empirical error rates justify deeper check |
| Streaming assembly for 1000+ page docs | Current in-memory buffer is < 50MB for typical docs | Documents exceed RAM |
| Multi-page scanned PDF without pdftoppm | Falls back to raw bytes OCR (single-page only) | User demand for poppler-free path |
| `fal-ai/got-ocr/v2` as alternative OCR backend | $0.05/image, supports multi-page formatted OCR | Benchmark vs. current LightOnOCR-2:1b accuracy |

---

## Completed Since Initial Session

| Item | Status |
|------|--------|
| fal.ai inference router (`FA/` prefix, `FalBackend`) | ✅ Done |
| Self-tuning threshold calibration (`calibration.rs`, 6 tests) | ✅ Done |
| Cross-validation accumulation across runs | ✅ Done |
| `MarkitdownCnsObserver` wiring (NuEvent construction + daemon persistence) | ✅ Done |
| `#[ignore]` → runtime guards on integration tests | ✅ Done |
| `unused_mut` warning fix in `decimation.rs` | ✅ Done |
| fal.ai `docres` model identified and documented | ✅ Done |
| Integration tests run with real services (Ollama, tesseract, pdftoppm) | ✅ Verified |

---

## Essentialist Review Findings

### Deleted (Prohibition)
- **`AssembledDocument` struct** — dead code, never referenced.
- **`thresholds::{SIMPLE_MAX, MODERATE_MAX, DEFAULT_MODERATE_SAMPLE_RATE}`** — dead constants, superseded by `ThresholdConfig`.
- **Custom `CnsObserver` trait in `pipeline.rs`** — unauthorized parallel path.
- **Per-run calibration call in `pipeline.rs`** — moved to server-level accumulation.

### Retained (Guideline)
- **`CnsObserver` trait impl** — now live, wired into server flow.
- **`preprocess_via_fal`** — real async implementation, base64 data URI, binarization task, graceful fallback.

---

## Grill-Me Self-Assessment

| Area | Rating | Key Finding |
|------|--------|-------------|
| Type design | 🟢 Solid | Sealed hierarchy, computed `passed` field |
| Routing strategy | 🟢 Solid | Deterministic, property-tested |
| Fallback path | 🟡 Partial | Simple-tier pages have no redundancy (single backend by design) |
| Verification depth | 🟢 Solid | Multi-signal: page count, word delta, empty pages, error tally |
| CNS integration | 🟢 Solid | NuEvent construction + CnsObserver + daemon persistence wired |
| Self-tuning | 🟢 Solid | Accumulation + drift analysis + P4-gated alert emission |
| PDF decimation | 🟢 Solid | `pdftoppm` + `preprocess_via_fal` (fal.ai docres when key set, stretch_contrast fallback) |
| Configurability | 🟢 Solid | All thresholds via CLI/API/REPL (P3: Generative Space) |
| fal.ai integration | 🟢 Solid | Router pattern complete; image-to-image path researched and documented |

---

## Configurable Settings Surface

All OCR thresholds are exposed identically across three surfaces (P3):

```bash
# CLI
kask settings set ocr_simple_max 0.08
kask settings set ocr_moderate_max 0.20
kask settings set ocr_sample_rate 0.15
kask settings show ocr_simple_max

# Interactive REPL
/repl ocr_simple_max 0.08
/repl ocr_moderate_max 0.20
/repl ocr_sample_rate 0.15

# HTTP API
curl -X PUT /api/settings -d '{"ocr_sample_rate": 0.15}'
curl /api/settings  # returns all fields including ocr_*
```

---

## Integration Test Results

3 integration tests (run by default, self-skip when services absent):

| Test | Status | Notes |
|------|--------|-------|
| `test_pipeline_with_llm_ocr` | ✅ Passed | Ran with real Ollama `maternion/LightOnOCR-2:1b`. Pipeline completed, CNS spans emitted, verification ran. |
| `test_pipeline_with_tesseract` | ✅ Passed | Tesseract available. Synthetic image returns empty (expected — no real text). |
| `test_pdf_pipeline` | ✅ Passed | Full pipeline: pdftoppm → OCR → verification. Extracted "Hello World\n" from minimal PDF. |

Run with:
```bash
cargo test -p hkask-mcp-markitdown --test integration
```
