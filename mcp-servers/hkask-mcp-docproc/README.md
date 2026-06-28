# hkask-mcp-docproc

Unified document processing MCP server — format conversion, OCR, chunking, triple extraction,
embedding, QA generation, caching, and semantic query. Supersedes the former
`hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge` servers.

## Architecture

```
lib.rs           — Server struct, CNS observer, shared helpers (extract_text, cosine_similarity,
                   template rendering, embedding model resolution)
tools/
  document.rs    — docproc_convert, docproc_ocr, docproc_chunk
  semantic.rs    — docproc_generate_qa, docproc_extract_triples, docproc_embed
  storage.rs     — docproc_cache, docproc_query, docproc_clear_index
ocr/ (11 modules)
  pipeline.rs    — OcrExecutor trait, run_pipeline orchestrator, cross-validation,
                   semantic enrichment, Levenshtein distance
  config.rs      — ComplexityTier, ComplexityScore, OcrBackend, ThresholdConfig
  document.rs    — OcrResult, CrossValidation, PipelineError, VerificationReport, PipelineOutcome
  decimation.rs  — PDF→images via pdftoppm + Otsu binarization + optional fal.ai docres
  complexity.rs  — Sobel edge detection → page complexity scoring
  routing.rs     — Complexity-driven backend selection with deterministic sampling
  llm_ocr.rs     — Vision LLM OCR via hkask-inference
  tesseract.rs   — Classical OCR via tesseract CLI (TSV confidence parsing)
  verification.rs — Post-pipeline quality checks (page count, word count, empty pages)
  calibration.rs — Threshold drift analysis with CNS alerting (human approval required)
  mod.rs         — Re-exports
convert.rs        — Format detection, HTML stripping, markdown frontmatter removal
```

## Tools (9)

| Tool | Description |
|------|-------------|
| `docproc_convert` | Extract text from a document. For PDFs: tries fast text extraction first (~50ms for text-native), falls back to typed OCR pipeline (decimate→score→route→OCR→verify) if near-empty. Supports `force_ocr` mode. Formats: PDF, MD, HTML, TXT. |
| `docproc_ocr` | OCR a document using a local vision model. Requires `HKASK_OCR_MODEL` or explicit `model` parameter. |
| `docproc_chunk` | Chunk text into passages at configurable token granularity. Accepts raw text or file path. Supports single-tier and multi-tier (coarse/medium/fine). Auto-indexing into in-memory vector store. |
| `docproc_generate_qa` | Generate QA pairs from a text chunk via inference engine. Uses registry template `docproc/generate-qa.j2` (falls back to inline prompt). |
| `docproc_extract_triples` | Extract RDF triples with confidence scores. Uses registry template `docproc/extract-triples.j2` (falls back to inline prompt). |
| `docproc_embed` | Generate embedding vectors via the configured embedding model (`HKASK_EMBEDDING_MODEL` or `~/.config/hkask/settings.json`). |
| `docproc_cache` | Cache processed document text keyed by label in `~/.config/hkask/docproc-cache/`. |
| `docproc_query` | Semantic search over indexed passages. Embeds query, computes cosine similarity, returns top-k. Optional LLM-augmented answer via `docproc/rag-answer.j2` template. |
| `docproc_clear_index` | Clear the in-memory vector index between document sets. |

## OCR Pipeline

The OCR subsystem implements a **typed, multi-backend, self-verifying** pipeline:

```
PDF → [Decimate] → PageQueue → [Score → Route → OCR] → [Verify] → PipelineOutcome
```

- **Decimation:** PDF→page images via `pdftoppm` with Otsu binarization. Per-page fault tolerant — individual corrupt pages are skipped rather than aborting the entire document.
- **Scoring:** Sobel edge detection classifies pages as Simple/Moderate/Complex.
- **Routing:** Simple pages → Tesseract. Complex pages → LLM vision OCR. Moderate pages → Tesseract with 10% dual-routing for cross-validation.
- **Backends:** Tesseract (CLI with TSV confidence parsing) and LLM vision (via `hkask-inference`, quality heuristic confidence scoring).
- **Verification:** Page count matching, empty page detection, word count estimation (±50% guardrail).
- **Calibration:** Accumulates cross-validation data. When ≥100 samples show >95% agreement between backends, suggests raising routing thresholds via CNS alert. **Never auto-adjusts** — P4 affirmative consent required.

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_OCR_MODEL` | Vision model for OCR (e.g., `DI/allenai/olmOCR-2-7B-1025`). Required for OCR tools. Fallback: `~/.config/hkask/settings.json` → `ocr_model`. |
| `HKASK_EMBEDDING_MODEL` | Embedding model for vectorization and semantic search. Fallback: `~/.config/hkask/settings.json` → `embedding_model`. |
| `HKASK_REGISTRY_PATH` | Path to the `registry/` directory for prompt templates. Default: `registry` (relative to CWD). |
| `HKASK_USE_FAL_DOCRES` | Set to `true` to enable fal.ai docres binarization enhancement (opt-in, ~40s latency). Requires `FA_API_KEY`. |
| `HKASK_REPLICANT` | Replicant identity for CNS narrative memory. |

### OCR Thresholds (via env vars or `settings.json`)

| Variable | Default | Description |
|----------|---------|-------------|
| `HKASK_OCR_SIMPLE_MAX` | 0.05 | Edge-density threshold for Simple tier |
| `HKASK_OCR_MODERATE_MAX` | 0.15 | Edge-density threshold for Moderate tier |
| `HKASK_OCR_SAMPLE_RATE` | 0.10 | Dual-routing sample rate for Moderate pages |
| `HKASK_OCR_TUNEABLE` | true | Whether CNS calibration may suggest threshold adjustments |

## CNS Observability

The server emits CNS spans under these targets for cybernetic feedback:

| Target | When |
|--------|------|
| `cns.pipeline.ocr` | Pipeline verification (every run) |
| `cns.pipeline.ocr.verification_failed` | Verification report fails |
| `cns.pipeline.ocr.low_confidence` | LLM OCR confidence < 0.3 |
| `cns.pipeline.ocr.rate_limit` | Inference rate-limited (429) |
| `cns.pipeline.ocr.collusion` | Both backends produce empty output |
| `cns.pipeline.decimation` | Page load failures |
| `cns.pipeline.decimation.binarize` | Otsu produces uniform output |
| `cns.pipeline.calibration` | Threshold drift detected |
| `cns.docproc.index` | Indexing requested but embedding unavailable |

## Shared Infrastructure

Docproc integrates with hkask's shared service layer:

- **Settings:** Model defaults from `~/.config/hkask/settings.json` via `hkask-services-core::HkaskSettings`
- **Template rendering:** Minijinja-based (same pattern as `self_heal.rs` and `ManifestExecutor`)
- **Templates:** `registry/templates/docproc/{generate-qa,extract-triples,rag-answer}.j2`
- **CNS:** Daemon-backed event persistence for Curator consumption
- **Inference:** `hkask-inference` router with provider-prefixed model names

## Quick Start

```bash
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-docproc
```
