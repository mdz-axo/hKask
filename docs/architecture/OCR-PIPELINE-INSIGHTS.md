# OCR Pipeline Deepening — Session Insights

**Date**: 2026-06-13  
**Scope**: `hkask-types`, `hkask-inference`, `hkask-mcp-markitdown`, `hkask-cli`, `hkask-api`  
**Tests**: 50 unit + 3 integration = 53 total

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

### 5. Three-Backend Architecture
`OcrExecutor` trait isolates backend invocation. Tesseract (native CLI), LlmOcr (vision model via inference router), and LightOn (Ollama model, mapped to LlmOcr). Backends are pluggable without pipeline changes.

---

## Tradeoffs Made

| Decision | Rationale | Cost |
|----------|-----------|------|
| Sobel edge-density heuristic | Fast, deterministic, O(w·h) | Imperfect on symmetric patterns (regular grids cancel out) |
| Levenshtein over BLEU/ROUGE | Character-level edits match OCR error patterns | Less semantic, but cross-validation adds embedding similarity |
| `pdftoppm` subprocess over native Rust PDF renderer | `pdftoppm` is battle-tested, widely available | External dependency, falls back to raw bytes OCR when absent |
| `supports_vision` heuristic over runtime probing | Fast (static allowlist), no per-model API calls | Incomplete — misses unknown vision models |
| Word-count heuristic for verification | Simple, catches catastrophic failures | Doesn't catch subtle degradations (e.g., "cl" → "d") |

---

## What Was Deferred (and Why)

| Item | Reason | Trigger to Revisit |
|------|--------|-------------------|
| Self-tuning thresholds | P4: requires affirmative consent for behavioral change | Cross-validation data exists in production |
| Semantic verification depth (embedding vs. ground truth) | Current word-count catches high-signal failures | Empirical error rates justify deeper check |
| Histogram equalization preprocessing | Requires `imageproc` crate | Data shows quality gap on low-contrast scans |
| Streaming assembly for 1000+ page docs | Current in-memory buffer is < 50MB for typical docs | Documents exceed RAM |
| CNS daemon integration | `TracingCnsObserver` is a placeholder | Daemon observer exists in `hkask-api` |
| Multi-page scanned PDF without pdftoppm | Falls back to raw bytes OCR (single-page only) | User demand for poppler-free path |

---

## Essentialist Review Findings

### Deleted (Prohibition)
- **`AssembledDocument` struct** — dead code, never referenced. Deletion test: nothing vanishes.
- **`thresholds::{SIMPLE_MAX, MODERATE_MAX, DEFAULT_MODERATE_SAMPLE_RATE}`** — dead constants, superseded by `ThresholdConfig`.

### Retained (Guideline)
- **`CnsObserver` trait** — single-use (`TracingCnsObserver`). Retained as contract for future daemon integration.

### Score
- Items removed: 4 / ~60 public items → 6.7% reduction
- All modules ≤ 7 public items (types crate justified as data-definition)

---

## Grill-Me Self-Assessment

| Area | Rating | Key Finding |
|------|--------|-------------|
| Type design | 🟢 Solid | Sealed hierarchy, computed `passed` field |
| Routing strategy | 🟢 Solid | Deterministic, property-tested |
| Fallback path | 🟡 Partial | Simple-tier pages have no redundancy (single backend by design) |
| Verification depth | 🟢 Solid | Multi-signal: page count, word delta, empty pages, error tally |
| CNS integration | 🟡 Partial | Tracing placeholder, real daemon observer deferred |
| PDF decimation | 🟡 Partial | Requires `pdftoppm`; graceful fallback to raw bytes OCR |
| Configurability | 🟢 Solid | All thresholds via CLI/API/REPL (P3: Generative Space) |

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

3 integration tests (all `#[ignore]` by default):

| Test | Status | Notes |
|------|--------|-------|
| `test_pipeline_with_llm_ocr` | ✅ Passed | Ran with real Ollama `minicpm-v:8b`. Pipeline completed, CNS spans emitted, verification ran. |
| `test_pipeline_with_tesseract` | ⏭️ Skipped | Tesseract not installed on test system |
| `test_pdf_pipeline` | ⏭️ Skipped | `pdftoppm` not installed on test system |

Run with:
```bash
cargo test -p hkask-mcp-markitdown --test integration -- --ignored
```
