# Handoff — OCR Pipeline Deepening + fal.ai Integration Prep

**Date**: 2026-06-13
**Session scope**: OCR pipeline reification, CNS alignment, fal.ai architecture planning
**Completion**: OCR pipeline ~95% complete. Self-tuning thresholds + fal.ai router remain.

---

## 1. Session Context

Reified the OCR pipeline of `hkask-mcp-markitdown` into a typed, multi-backend, self-verifying pipeline exploiting existing hKask infrastructure (`hkask-types`, `hkask-inference`, `hkask-cns`, `hkask-storage`). Added semantic decomposition, complexity-driven routing, statistical cross-validation, mandatory verification checkpoint, contrast-stretched PDF decimation, and CNS-aligned observability. All 9 specification tasks + 6 deferred items addressed. Essentialist review applied (dead code deleted). Grill-me self-assessment completed. Architecture corrected to align with CNS → Curator → User flow. fal.ai integration into inference router designed but not built.

---

## 2. What Was Done

### Types (`hkask-types/src/ocr.rs`)
- Sealed type hierarchy: `ComplexityTier`, `ComplexityScore`, `OcrBackend` (Tesseract, LlmOcr), `OcrResult`, `CrossValidation` (with `semantic_similarity`), `PipelineError`, `VerificationReport` (computed `passed`), `PageVerificationDetail`, `PipelineOutcome`, `OcrVerificationSpan`, `OcrCrossValidationSpan`, `BackendUsage`, `ThresholdConfig` (with `classify()`)
- `thresholds` module reduced to single live constant `DEFAULT_LLM_OCR_MODEL`
- 6 tests (REQ:ocr-type-01 through 06)

### Inference (`hkask-inference/src/lib.rs`, `inference_router.rs`)
- `RouterModelEntry::supports_vision: Option<bool>` with `infer_vision_support()` heuristic (14 known vision families)
- `InferenceRouter::list_vision_models()` convenience method
- Populated in all three backends (Ollama, Fireworks, DeepInfra)

### Pipeline Modules (`mcp-servers/hkask-mcp-markitdown/src/ocr/`)
- `complexity.rs` — Sobel edge-density heuristic, `ThresholdConfig`-driven, 6 tests
- `routing.rs` — Deterministic round-robin `SamplingState`, property-tested (±5% over 1000 iterations), 6 tests
- `pipeline.rs` — Sequential state machine: Decimate → Score → Route → OCR → Assemble → Verify. `OcrExecutor` trait (async_trait). Streaming-compatible (`impl IntoIterator`). Direct `tracing::info!` CNS emission under `cns.pipeline.ocr` target. 4 tests.
- `verification.rs` — Multi-signal checkpoint: page count, word delta, empty pages, error tally. `estimate_word_count()` per-image. 4 tests.
- `cross_validation.rs` — Normalized Levenshtein similarity, space-optimized DP. 6 tests.
- `semantic.rs` — `enrich_with_semantic()` via `EmbeddingRouter`, `cosine_similarity()`. 4 tests.
- `decimation.rs` — `pdf_to_images()` via `pdftoppm` subprocess. `stretch_contrast()` preprocessing (no new deps). 5 tests.

### Server Integration (`tools.rs`, `main.rs`)
- `OcrExecutor` impl for `MarkitdownServer`: Tesseract (native CLI when available, LLM fallback), LlmOcr (vision model via inference router)
- `resolve_ocr_model()` now async, validates `supports_vision` via `list_vision_models()`
- `persist_pipeline_outcome()` — persists verification data to daemon for CNS → Curator flow
- `load_ocr_thresholds()` — reads `settings.json`, falls back to `ThresholdConfig::default()`
- `EmbeddingRouter` built at server init, passed to pipeline for semantic cross-validation
- `markitdown_convert` PDF path: tries decimation → pipeline first, falls back to `pdf-extract` + `do_ocr`
- `MarkitdownCnsObserver` scaffolded — implements real `hkask_types::ports::CnsObserver` trait, `#[allow(dead_code)]` for future wiring

### Settings Surfaces
- CLI `/repl`: `ocr_simple_max`, `ocr_moderate_max`, `ocr_sample_rate` subcommands
- CLI `kask settings`: `show_all`, `show_one`, `apply_setting` handle all three
- API `GET/PUT /api/settings`: `SettingsResponse` + `UpdateSettingsRequest` include all three (range-validated 0.0–1.0)

### Integration Tests (`tests/integration.rs`)
- 3 `#[ignore]` tests: LLM OCR (ran successfully with real Ollama), Tesseract, PDF pipeline
- Run: `cargo test -p hkask-mcp-markitdown --test integration -- --ignored`

### Documentation
- `docs/architecture/OCR-PIPELINE-INSIGHTS.md` — session insights, architecture decisions, tradeoffs, grill-me assessment

### Essentialist Deletions
- `AssembledDocument` struct (dead code)
- `thresholds::{SIMPLE_MAX, MODERATE_MAX, DEFAULT_MODERATE_SAMPLE_RATE}` (dead constants)
- Custom `CnsObserver` trait in `pipeline.rs` (unauthorized parallel path)
- `to_threshold_config()` in `hkask-cli` (dead code, removed by another process)

### Test Count: 47
- `hkask-types`: 17
- `hkask-mcp-markitdown` unit: 44
- `hkask-mcp-markitdown` integration (`#[ignore]`): 3
- All pass, zero warnings, workspace clean

---

## 3. What Remains

### HIGH — Self-Tuning OCR Thresholds

**What**: When cross-validation data shows Tesseract and LlmOcr agree consistently on Moderate pages, emit a CNS alert suggesting threshold adjustment. Human approves via Curator replicant (P4 consent).

**Where**:
- `PipelineOutcome.cross_validations` already accumulates similarity data
- Need a function that analyzes patterns (e.g., "over last N runs, Moderate pages dual-routed had >95% similarity") and emits a `NuEvent` with `cns.pipeline` namespace
- `CurationLoop.sense()` reads from `NuEventStore` → `CuratorAgent` presents to human
- Human runs `kask settings set ocr_moderate_max 0.25` to approve

**Strategy**:
1. Add `analyze_threshold_drift(outcomes: &[PipelineOutcome]) -> Option<ThresholdDriftAlert>` to pipeline or a new `ocr/calibration.rs` module
2. Emit as `tracing::warn!` for now (same pattern as current CNS emission)
3. Future: construct `NuEvent` with `SpanNamespace("cns.pipeline")` and persist to `NuEventStore`
4. Threshold: if >100 Moderate dual-routed pages show >95% mean similarity, suggest raising `moderate_max`

**Dependencies**: None — cross-validation data already exists in `PipelineOutcome`.

### HIGH — fal.ai Inference Router Integration

**What**: Add `ProviderId::Fal` with `FA/` prefix to `hkask-inference`, following the same pattern as Fireworks (`FW/`) and DeepInfra (`DI/`).

**Where**:
- `crates/hkask-inference/src/config.rs` — add `ProviderId::Fal`, `FA_API_KEY` env var
- `crates/hkask-inference/src/fal_backend.rs` — new module: auth, model listing, `generate`, `generate_vision`, `generate_stream`
- `crates/hkask-inference/src/inference_router.rs` — add `FalBackend` to router, dispatch `FA/` prefix
- `crates/hkask-inference/src/lib.rs` — add fal model families to `infer_vision_support()` (paddleocr, nemotron-parse, etc.)

**Strategy**:
1. Study `fireworks_backend.rs` as template — fal.ai has OpenAI-compatible chat endpoint
2. fal.ai-specific: media models may use different endpoints (image generation, video). Start with text/vision models using chat completions endpoint.
3. Model listing: fal.ai doesn't have a standard list endpoint — use a static catalog or documentation scrape
4. `FA_API_KEY` from env, `FA_BASE_URL` default `https://api.fal.ai`

**Dependencies**: fal.ai API key (user has one).

### MEDIUM — Image Preprocessing via fal.ai for PDF Decimation

**What**: Use a low-cost fast fal.ai vision model to preprocess/clean up page images before OCR. Evaluate cost-benefit vs. the current `stretch_contrast()` (free, local).

**Where**: `mcp-servers/hkask-mcp-markitdown/src/ocr/decimation.rs` — add optional `preprocess_with_fal()` step after `stretch_contrast()`.

**Strategy**:
1. First: **cost-benefit analysis**. Compare:
   - Current: `stretch_contrast()` — free, O(w·h), no network, improves edge detection
   - fal.ai model (e.g., a 1B vision model): ~$0.15/1M tokens, network latency, could do deskew + denoise + enhancement
2. If justified: add `preprocess_via_fal(image, model)` that sends image to fal.ai with a "clean up this document image for OCR" prompt, returns enhanced image
3. Gate behind config flag or `FA_API_KEY` presence — falls back to `stretch_contrast()` when fal.ai unavailable
4. Measure: compare OCR accuracy with/without fal.ai preprocessing on a sample of real scanned documents

**Dependencies**: fal.ai router integration (HIGH item above) must be done first.

### LOW — `MarkitdownCnsObserver` Wiring

**What**: Wire the scaffolded `MarkitdownCnsObserver` (implements real `CnsObserver` trait) into the CNS runtime so OCR spans flow through `NuEvent` → `NuEventStore` → `CurationLoop`.

**Where**: `tools.rs` — `MarkitdownCnsObserver` is `#[allow(dead_code)]`. Needs to be registered with CNS runtime at server init.

**Strategy**: Defer until pipeline has access to `WebID` + `NuEventStore` at runtime. Current `tracing::info!` emission is adequate for development.

---

## 4. Recommended Skills and Tools

| Skill | When |
|-------|------|
| `coding-guidelines` | Before any implementation — enforce simplicity, surgical changes |
| `tdd` | Building self-tuning threshold analysis — RED→GREEN→REFACTOR |
| `rust-expertise` | Implementing `FalBackend` — type-driven design, ownership patterns |
| `condenser-continuation` | If context resets during fal.ai work — restores session state |

**Commands**:
```bash
cargo check -p hkask-inference        # After FalBackend changes
cargo check -p hkask-mcp-markitdown   # After threshold/decimation changes
cargo test -p hkask-mcp-markitdown    # Full test suite
cargo clippy -p hkask-inference -- -D warnings
cargo check --workspace               # Final verification
```

---

## 5. Key Decisions to Preserve

1. **Pipeline is deliberately sequential (P1)**. Concurrency is a future optimization gated by throughput data. Do not parallelize without measuring baseline first.

2. **CNS flow is NuEvent → NuEventStore → CurationLoop → CuratorAgent → human**. Do not create custom observer traits or bypass this path. The `CnsObserver` trait in `hkask_types::ports` is the single authorized interface.

3. **Self-tuning requires P4 affirmative consent**. Thresholds must never auto-adjust. CNS alert → human approval via `kask settings set` is the only valid path.

4. **fal.ai belongs in inference router, not in individual MCPs**. `FA/` prefix pattern same as `FW/` and `DI/`. Every MCP accesses fal.ai models through the router — no per-MCP fal.ai integration.

5. **MCPs expose tools (discrete operations), not services (persistent capabilities)**. Voice, memory, streaming inference are agent services, not MCP tools. Media MCP provides construction tools; agents provide runtime services.

6. **`OcrExecutor` trait isolates backend invocation**. Adding new backends (fal.ai models, new OCR services) requires zero pipeline changes — just implement the trait.

7. **Contrast stretching is free and local**. fal.ai preprocessing must demonstrate measurable OCR accuracy improvement over `stretch_contrast()` before it replaces or supplements it. Do cost-benefit analysis first.

8. **`ThresholdConfig` is the single source of truth for routing thresholds**. The legacy `thresholds` module constants were deleted. All threshold configuration flows through `settings.json` → `ThresholdConfig`.
