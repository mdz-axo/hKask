# docproc vs. LlamaIndex / LlamaParse / LiteParse — Capability Comparison & Learning Study

**What this is:** A comparison of hKask's `mcp-servers/hkask-mcp-docproc` against the actual source of the LlamaIndex family, with concrete proposals for what docproc can learn. Every claim is grounded in source read this session (file refs on both sides). This is the study the task actually asked for; it supersedes the deleted planning artifacts.

**Date:** 2026-07-21

---

## 0. The three systems (disambiguated — the prior session conflated these)

| System | Repo | Stack | What it is | Relevance to docproc |
|---|---|---|---|---|
| **LlamaIndex** | `run-llama/llama_index` (51k★, Python) | Python framework | The umbrella data-agent/RAG framework. Not a parser itself. | Low — it's a framework that *consumes* parsed docs; its patterns are RAG/indexing, not OCR. |
| **LlamaParse** | `run-llama/llama_parse` (4.3k★, TS) + `llama-parse-py` SDK | **Cloud SaaS**, REST API | Enterprise cloud OCR/parsing: async jobs, agentic-LLM tier, schema extraction, 130+ formats, managed infra. | Reference for the *cloud tier* and API ergonomics. Not directly portable (hKask is local-first, P1 sovereignty). |
| **LiteParse** | `run-llama/liteparse` (11.7k★, **Rust 84%**) | Local Rust lib + CLI (`lit`) + napi/pyo3/wasm bindings | Standalone local PDF parser: PDFium text extraction, selective OCR, native+OCR merge, grid projection → structured markdown, bounding boxes, screenshots, complexity triage. | **The primary comparison target.** Same language, same local-first stance, same problem space. Architecturally closest to docproc. |

**The fair comparison is docproc ↔ LiteParse** (both local Rust PDF/OCR pipelines), with LlamaParse as the "what the managed cloud tier does that local can't easily replicate" reference. LlamaIndex-the-framework is out of scope for an OCR study.

---

## 1. Capability-by-capability comparison

Grounded in: docproc source read end-to-end this session (`tools/document.rs`, `lib.rs`, `ocr/*`, `crates/hkask-types/src/document.rs`); LiteParse source (`parser.rs`, `ocr_merge.rs`, `projection.rs`, `markdown_layout/*`, `main.rs` CLI, `OCR_API_SPEC.md`, `types.rs`); LlamaParse SDK (`api.md`, `README.md`).

### 1.1 Text extraction (the foundation)

| Dimension | docproc | LiteParse | LlamaParse |
|---|---|---|---|
| PDF text source | `pdftotext -layout` subprocess (poppler) — whole doc, one call (`lib.rs:289`) | **PDFium C library via own Rust FFI** (`crates/pdfium/`) — per-page `TextItem`s with bbox, font, rotation | Cloud; not exposed |
| Per-page granularity | **No** — one `pdftotext` call, one word-count check (`lib.rs:299`) | **Yes** — PDFium yields per-page text items natively | Yes (job result is per-page) |
| Spatial provenance | None — `pdftotext` output is positioned plain text, bbox discarded | **Full** — every `TextItem`/`WordBox` carries `[x1,y1,x2,y2]` + rotation (`types.rs:16,85`) | `BBox` on every item |
| Reading order | `pdftotext -layout` heuristic (indentation-preserving) | **XY-cut grid projection** (`projection.rs`) — recursive axis cuts into `Region`/`RegionKind` tree, column detection, flowing-text detection | Agentic model-based |
| Garbled/font recovery | None — trusts `pdftotext` output | cmap/AGL glyph recovery + optional `GlyphResolver` font-DB hook for obfuscated fonts (`parser.rs`, `glyph_resolver`) | Cloud-side |

**Learning:** docproc's `pdftotext -layout` is a black-box subprocess that throws away the spatial structure PDFium/poppler already computes. LiteParse keeps `TextItem`+bbox end-to-end, which is what makes its grid projection and merge possible. docproc can't do per-page triage or per-region merge well *because it discards per-item geometry at the first step.*

### 1.2 Complexity triage (needs-OCR detection)

| Dimension | docproc | LiteParse |
|---|---|---|
| Where it runs | Inline in `extract_text`, **per-document** (`lib.rs:299`) | **Standalone `lit is-complex` command** + inline per-page gate (`ocr_merge.rs`) |
| Signal | `pdftotext` whole-doc word count vs `OCR_FALLBACK_WORD_THRESHOLD=100` (`lib.rs:98`) — a single doc-level scalar | **Per-page semantic signals** from the text layer: `text_coverage`, `image_coverage`, `largest_image_coverage`, `full_page_image`, `uncovered_vector_area`, `is_garbled` (`ocr_merge.rs` `PageComplexityStats`) |
| Reasons | Binary: enough words / not enough | **6 typed reasons**: `Scanned`, `NoText`, `SparseText`, `EmbeddedImages`, `Garbled`, `VectorText` (`ComplexityReason` enum) + layout-difficulty signals (columns, tables, dense graphics) |
| Cost | One `pdftotext` call (cheap) | **Text-layer-only pass, no rendering** — cheaper than OCR, runs before committing to render |
| Failure mode | **Silent loss**: a 100-page doc with 1 scanned page clears the 100-word threshold → scanned page dropped (empirically demonstrated this session: 11-page mixed PDF, 4749 words, page 5 image silently lost) | Per-page gate catches the scanned page; OCR runs only on flagged pages |

**Learning:** docproc's triage is a single doc-level scalar with no semantic discrimination. LiteParse's is a per-page multi-signal classifier that distinguishes *why* a page needs OCR (scan vs. garbled-font vs. vector-text vs. embedded-image) — each reason implies a different downstream action. The 6-reason taxonomy is directly portable to docproc as a `TriageVerdict` enum, and the per-page granularity is what fixes the demonstrated silent-loss bug.

### 1.3 OCR execution & backends

| Dimension | docproc | LiteParse | LlamaParse |
|---|---|---|---|
| Backends | Tesseract (CLI+TSV) + LLM vision (`hkask-inference`) — 2 backends, Rust trait `OcrExecutor` (`pipeline.rs:48`) | Tesseract (bundled) + **any HTTP server** via `POST /ocr` spec (`OCR_API_SPEC.md`) + caller-injected `OcrEngine` (for WASM/JS callback) | Cloud agentic models |
| Routing | Sobel edge-density → Simple/Moderate/Complex → backend (`complexity.rs`, `routing.rs`); Moderate 10% dual-route for cross-validation | Complexity reasons → OCR only on flagged pages; no Sobel (uses text-layer signals instead) | Agentic tier selection |
| Pluggability | **Rust-internal trait** — new backend = recompile | **HTTP spec** — new backend = stand up a server conforming to `POST /ocr` (PaddleOCR, EasyOCR, custom); no recompile | N/A (managed) |
| Concurrency | `HKASK_OCR_CONCURRENCY` (default 4), semaphore-gated parallel (`pipeline.rs:211`) | `--num-workers` (default CPU-1) | Server-side |
| Output | `OcrResult { text, confidence, duration_ms, was_fallback }` — **no bbox** (`document.rs:9`) | `OcrResult { text, bbox, confidence, polygon? }` — **bbox + 4-point polygon** for rotation recovery (`OCR_API_SPEC.md`) | `BBox` on items |

**Learning:** Two distinct ideas here. (a) **The HTTP OCR spec** is the cleanest pluggability pattern — decouples engine from parser, enables CJK via PaddleOCR without recompiling docproc. It does add a network surface (tension with P1 local-first), so it should be *opt-in* and *local-loopback by default*. (b) **Bounding boxes + polygon** are what enable rotation recovery (vertical sidebars, sideways table headers) and per-region merge. docproc's `OcrResult` has no geometry, which is why it can only do doc-level max-pick, not per-region merge.

### 1.4 Native-text + OCR merge

| Dimension | docproc | LiteParse |
|---|---|---|
| Strategy | Two paths: `run_pipeline` branch **wholesale-replaces** page text with OCR (`document.rs:194-214`); `NeedsOcr`→`do_ocr` fallback does **doc-level max-pick** between OCR and partial text (`document.rs:267-276`) | **Per-region merge with overlap dedup** — OCR results are overlaid on native text; an overlap filter discards OCR results that duplicate native text (`ocr_merge.rs`, `UNCOVERED_VECTOR_AREA_THRESHOLD`) |
| When it helps | max-pick helps only when whole-doc OCR < whole-doc native text | Helps whenever a page has *both* vector text *and* scanned/image regions — the common mixed-page case |
| Threshold logic | `OCR_FALLBACK_WORD_THRESHOLD=100` (doc-level, not under calibration) | `UNCOVERED_VECTOR_AREA_THRESHOLD=400pt²` (~one word at 10-12pt) — per-page, tuned against a 121-page financial report |

**Learning:** docproc's "merge" is really a coarse fallback, not a merge. LiteParse's per-region merge with dedup is the pattern that actually preserves vector-text fidelity on mixed pages. **But**: it requires bbox on both native text items and OCR results — which means docproc would need to stop discarding geometry at extraction time (§1.1) before per-region merge is even possible. This is a dependency chain, not an isolated feature.

### 1.5 Structure reconstruction (→ `DocStructure` / markdown blocks)

| Dimension | docproc | LiteParse | LlamaParse |
|---|---|---|---|
| PDF → structure | **None.** PDFs return `structure: None` (`lib.rs:308`); `pdftotext -layout` emits no `#` markers | **Full `markdown_layout/` pipeline**: heading detection (font-size histograms + PDF outline/struct_tree + bold/numbered heuristics), ruled **and** borderless table detection, list detection, HR detection, figure injection, **running header/footer stripping**, rotated-line exclusion (`markdown_layout/{mod,classify,headings,tables,lists,...}.rs`) | `HeadingItem`/`TableItem`/`ListItem`/`Form`/etc. |
| Office formats | Native Rust backends: docx-rs, calamine, pptx-to-md → `markdown_to_structure` (keys off `#` markers) (`backend/{docx,pptx,xlsx}.rs`) | Converts DOCX/XLSX/PPTX → PDF via LibreOffice/ImageMagick, then parses the PDF uniformly (`conversion.rs`) | 130+ formats, cloud |
| Block model | `Block = Paragraph \| Heading{level} \| Table{rows} \| List{ordered,items}` — 4 variants, no bbox (`document.rs:102`) | `Block` = heading/paragraph/list-item/code-block/table(ruled+borderless)/HR/figure — richer, with bbox | Rich typed items + forms |
| Consumer | `chunk_structure` uses `block.text()` + `is_heading()` only (`helpers.rs:55`) | Markdown/JSON/text emitters | Index/extract pipelines |

**Learning:** This is the biggest capability gap. docproc produces **zero structure for PDFs** — the most common input format — while LiteParse produces rich structured markdown from PDFs via pure heuristics (no neural layout model). The continuation prompt's "just wire `pdftotext -layout` through `markdown_to_structure`" is a dead end (verified: `pdftotext` emits no `#` markers). Real PDF structure needs either (a) PDFium-level text items + grid projection + font-size heading detection (the LiteParse path), or (b) an LLM layout pass (the LlamaParse agentic path). The LiteParse path is Rust-only and local — consistent with hKask's tooling policy — but it's a *large* implementation, not a small enhancement.

### 1.6 Operational features

| Feature | docproc | LiteParse | LlamaParse |
|---|---|---|---|
| Page-range parsing | **No** (`extract_text`/`decimation` process whole doc) | `--target-pages "1-5,10,15-20"` (`main.rs`) | Per-job config |
| Screenshot output | **No** — page images rendered for OCR then discarded | `lit screenshot` — PNG renders at configurable DPI, first-class output for LLM agents | Page images extractable |
| Encrypted PDFs | **No** | `--password` | Yes |
| Self-verification | **Yes** — page-count, empty-page, word-count-delta ±50% (`verification.rs`) | Complexity stats + overlap filter | Job status/result |
| Calibration / self-tuning | **Yes, distinctive** — cross-validation accumulation, ≥100 samples >95% similarity → CNS drift alert, **never auto-adjusts** (P4 consent) (`calibration.rs`) | None | None |
| CNS / governance observability | **Yes, distinctive** — `cns.pipeline.*` spans, affirmative-consent thresholds | None | None |
| Format coverage (local) | PDF/MD/HTML/TXT + DOCX/XLSX/PPTX (native Rust) | PDF (native) + DOCX/XLSX/PPTX/images via LibreOffice conversion | Cloud 130+ |

**Learning:** docproc's **calibration + CNS governance is a genuine advantage** neither Llama system has — it's the cybernetic layer that fits hKask's model. LiteParse wins on operational features docproc lacks: page-range, screenshots, encrypted-PDF, and per-page complexity as a standalone tool. Screenshots are the one to think about carefully: docproc's downstream (QA-generation, RAG) consumes text, not images — so a screenshot tool needs a consumer to justify it (the embarrassing-if-true risk).

### 1.7 Architecture & API shape

| Dimension | docproc | LiteParse | LlamaParse |
|---|---|---|---|
| Interface | **MCP server** (17 tools, JSON-RPC over stdio) — agent-callable | CLI (`lit`) + Rust lib + napi/pyo3/wasm bindings | REST + Python/TS SDKs + MCP server |
| Job model | Synchronous tool calls | Synchronous | **Async job + polling** (upload → create job → poll → result) |
| Output sealing | `PipelineOutcome` sealed; no partial state escapes (`document.rs:163`) | `ParseResult { pages, text, outline, images }` | Job result |
| Typed result model | `OcrResult`/`VerificationReport`/`DocStructure` | `ParsedPage`/`ProjectedLine`/`Region`/`Block` + `PageComplexityStats` | Rich `*Item` types + `BBox` |

**Learning:** LlamaParse's **async job + polling** pattern is worth noting for *batch/corpus* workloads (docproc's `convert_directory`/`chunk_directory` are synchronous and would block an agent on a 300-page OCR). docproc's MCP-tool shape is good for interactive use; a job API for batch would help. LiteParse's multi-binding strategy (Rust lib + CLI + language bindings) is a distribution pattern docproc doesn't need (it's an MCP server, consumed by the agent runtime).

---

## 2. Architectural / philosophical differences

1. **Geometry-first vs. text-only.** LiteParse carries `TextItem`+bbox from PDFium through projection through merge through output. docproc discards geometry at `pdftotext` and operates on flat strings ever after. This single difference *causes* most of the capability gaps (per-page triage, per-region merge, structure reconstruction, rotation recovery all depend on having geometry).

2. **One-parser-many-formats vs. native-per-format.** LiteParse converts office formats → PDF (via LibreOffice) and parses PDF uniformly — one structure pipeline. docproc has native Rust backends per office format + a separate (geometry-less) PDF path. docproc's choice is more dependency-light (no LibreOffice) but means the PDF path — the most common and hardest format — gets the *least* capable treatment.

3. **Cloud agentic vs. local heuristic.** LlamaParse's `tier="agentic"` uses LLMs for hard layouts (dense tables, charts, handwriting). LiteParse and docproc are local-heuristic. docproc *does* have an LLM-vision OCR backend, but uses it only for OCR text extraction, not for layout/structure understanding.

4. **Governance vs. none.** docproc's calibration + CNS + affirmative-consent thresholds have no equivalent in either Llama system. This is hKask-specific and should be preserved through any enhancement.

---

## 3. What docproc can learn — proposed improvements, prioritized

Prioritized by (value to hKask's corpus pipeline) ÷ (implementation cost & risk), and filtered through hKask's constraints: **Rust-only, local-first (P1), simplicity (P5), deep modules (P7), CNS observability, affirmative consent.**

### Tier 1 — High value, low/medium cost, fits constraints (do these)

**P1. Per-page complexity triage with typed reasons.** Replace the single doc-level `word_count vs 100` check with a per-page classifier emitting a `TriageVerdict` with reasons `{Scanned, NoText, SparseText, EmbeddedImages, Garbled, VectorText}`. This **directly fixes the demonstrated silent-loss bug** and is the foundation for selective OCR. *Cost:* medium — needs per-page text extraction (`pdftotext -f N -l N` per page, or split whole-doc output on form-feed `\x0c` to avoid N subprocess spawns). *Constraint fit:* yes; expose as a `docproc_is_complex` MCP tool for composability (LiteParse's `lit is-complex` pattern). Emit `cns.pipeline.triage` spans. Put triage thresholds under the existing `ThresholdConfig` calibration regime (≥100 samples, human approval).

**P2. Page-range support (`target_pages`).** Accept `"1-5,10,15-20"`; skip pages outside the range in extraction + decimation. *Cost:* small. *Constraint fit:* trivially yes. High ROI for large docs where an agent needs one section.

**P3. Selective OCR — only OCR pages triage flags, keep native text for the rest.** This is the docproc-native analogue of LiteParse's merge, achievable *without* bbox by doing per-page triage then per-page routing: text-native pages use `pdftotext` output; flagged pages go through `decimation` (only those pages) + `run_pipeline`. *Cost:* medium; depends on P1. *Constraint fit:* yes. This gets most of LiteParse's merge benefit (don't re-OCR good vector text) without needing geometry, because the routing unit is the *page*, not the region.

### Tier 2 — High value, high cost, needs design (study, then decide)

**P4. PDF → `DocStructure` via grid projection + font-size heading detection.** This is the biggest capability gap (PDFs produce no structure today). The LiteParse path is pure Rust heuristics: per-page `TextItem`s → XY-cut projection → font-size histogram heading detection → ruled/borderless table detection → list/HR detection. *Cost:* **large** — this is most of LiteParse's `projection.rs` + `markdown_layout/` (~3000+ lines). *Constraint fit:* Rust-only, local, no neural model — fits. But it conflicts with P5 simplicity unless scoped as a new deep module (`crates/hkask-pdf-structure`?) with a small interface. *Decision needed:* is PDF structure worth a multi-thousand-line module, or is `pdftotext -layout` + flat chunking "good enough" for hKask's corpus? **Answer this with a measurement on real corpus docs before building.**

**P5. Geometry end-to-end (bbox on `TextItem`/`OcrResult`/`Block`).** This is the enabler for per-region merge, rotation recovery, and richer structure. *Cost:* large — requires switching from `pdftotext` to a Rust PDFium binding (like LiteParse's `crates/pdfium/`) or `pdfium-render` crate, and threading bbox through `OcrResult`, `Block`, and `chunk_structure`. *Constraint fit:* Rust-only yes; but it's a foundational change to the extraction layer. *Decision needed:* only justified if P4 (structure) or per-region merge is justified *and* a downstream consumer (chunking, QA citations) is committed to use the geometry. **Don't add geometry without a consumer** (the lesson from the deleted session's H3 — no consumer means dead weight).

### Tier 3 — Selectively adopt (opt-in, guard with constraints)

**P6. Pluggable OCR via HTTP spec (opt-in, loopback-default).** Adopt LiteParse's `POST /ocr` spec as an *alternative* to the `OcrExecutor` trait — lets a PaddleOCR/EasyOCR server handle CJK without recompiling docproc. *Constraint fit:* tension with P1 (local-first) and P5 (simplicity) — mitigate by making it opt-in and documenting that the server is expected on loopback. *Decision needed:* only justified if a CJK/non-Latin corpus materializes. Until then, document the spec as a future option, don't build the client.

**P7. Screenshot generation.** `lit screenshot` produces PNG page renders for LLM agents. *Cost:* small (decimation already renders pages; just persist them). *Constraint fit:* yes. *Decision needed:* **identify a consumer first** — does any hKask tool/agent consume page images? If the QA/RAG pipeline is text-only, screenshots are dead weight. The LlamaParse `examples/parse/parse_extract_page_images.py` shows the cloud use case (feed page images to a vision model); docproc would need an analogous consumer.

### What docproc should NOT copy (and why)

- **The cloud agentic tier (LlamaParse).** Conflicts with P1 (user sovereignty / local-first) and hKask's no-cloud-dependency stance. docproc already has an LLM-vision OCR backend for the hard cases; the cloud's value is managed infra + scale, which hKask deliberately doesn't adopt.
- **LibreOffice-based format conversion (LiteParse).** docproc's native Rust office backends are more dependency-light and hKask's tooling policy prefers Rust binaries over system deps. The trade-off (N parsers vs. 1) is already made; don't unmake it for PDF-structure convenience alone.
- **Neural layout models (DocLayNet/TableFormer).** Neither LiteParse nor this study recommends them for the local path; LiteParse's grid projection is explicitly heuristic. P5 simplicity rules out a layout model unless a corpus demands it and a lighter heuristic fails first.
- **LlamaIndex-the-framework's indexing patterns.** Out of scope for an OCR/extraction study; docproc already has its own embedding/vector/QA pipeline (`docproc_embed`/`docproc_query`/`docproc_generate_qa`).

---

## 4. docproc's distinctive strengths to preserve

These are things docproc has that the Llama systems don't — any enhancement must not regress them:

1. **Calibration with affirmative consent** (`calibration.rs`) — cross-validation-driven threshold drift detection, ≥100 samples, CNS alert, never auto-adjusts. LiteParse has hardcoded thresholds (`UNCOVERED_VECTOR_AREA_THRESHOLD=400`, etc.).
2. **CNS observability** (`cns.pipeline.*` spans) — cybernetic feedback for the Regulation layer. Neither Llama system has governance observability.
3. **Typed, sealed pipeline outcome** (`PipelineOutcome`) — no partial state escapes; verification report with computed `passed`.
4. **MCP-native** — agent-callable tools, not a CLI/library a user must wire. Fits hKask's userpod model.
5. **Dual-backend cross-validation** — Moderate-tier 10% dual-routing to compare Tesseract vs LLM-vision is a calibration data source LiteParse lacks.

---

## 5. The honest bottom line

The single highest-leverage learning is **§1.1's geometry point**: docproc discards spatial structure at the first step (`pdftotext`), and that one decision is upstream of most gaps (per-page triage, per-region merge, PDF structure, rotation). LiteParse's entire pipeline is built on keeping `TextItem`+bbox from PDFium through to output.

But "adopt PDFium + geometry end-to-end" (P5) is a foundational rewrite of the extraction layer — not a quick enhancement. The pragmatic path that captures most of the value at a fraction of the cost is **Tier 1 (P1+P2+P3)**: per-page triage + page-range + selective per-page OCR, all achievable *without* geometry by making the page the routing unit. This fixes the demonstrated silent-loss bug, skips OCR on text-native pages, and respects hKask's constraints — while leaving the geometry/structure rewrite (P4/P5) as a measured decision for after benchmarking on a real corpus.

**What's still missing to make any of this a decision:** a benchmark corpus (text-native + scanned + mixed PDFs) and measured wall-clock/accuracy before-and-after. The capability comparison above is grounded; the *improvement* claims must be grounded too, which means building the corpus and measuring — not asserting.