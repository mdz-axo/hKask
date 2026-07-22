# Phase 1 — Hypothesis Framer (FINER + PICO)

**Skill:** `hypothesis-framer`
**Target:** docproc OCR pipeline (`mcp-servers/hkask-mcp-docproc`)
**Exemplars studied:** LlamaIndex (`run-llama/llama_index`), LiteParse (`run-llama/liteparse`)
**Date:** 2026-07-21
**Status:** ADMITTED to Phase 2 (falsifiability)

---

## 1. Broad Topic

> "Adopting LiteParse's complexity-triage, pluggable-OCR, and spatial-merge patterns will improve docproc's OCR efficiency and effectiveness."

This is a broad IS-mode claim that requires refinement before resources are committed.

---

## 2. FINER Evaluation

| Dimension | Score (0-10) | Rationale |
|---|---|---|
| **Feasible** | 7 | docproc is Rust; LiteParse is Rust (83.8%). Patterns are portable. However: L3 (bounding boxes) requires changes to `OcrResult` (`mcp-servers/hkask-mcp-docproc/src/ocr/document.rs:9-22`), `DocStructure`/`Block` (`crates/hkask-types/src/document.rs:101-112`), and downstream chunking/tagging — non-trivial. L4 (merge not replace) requires a merge stage that doesn't exist in `run_pipeline` (`mcp-servers/hkask-mcp-docproc/src/ocr/pipeline.rs:98-132`). L5 (grid projection) is a new heuristic stage. |
| **Interesting** | 8 | docproc's downstream QA-generation pipeline (the `qa-generation` skill, John Brooks persona) would benefit from spatial provenance in citations. But hKask's userpod model may not need LLM-agent screenshots (L6) — the QA pipeline consumes text, not images. |
| **Novel** | 6 | **Lowest dimension.** The patterns themselves are not novel — LiteParse ships them. The novelty is adapting them to hKask's cybernetic-governance model (CNS spans, calibration discipline, affirmative consent for threshold changes). No prior art exists for "LiteParse patterns under Regulation observability." |
| **Ethical** | 9 | Local execution, no cloud dependency (P1 user sovereignty preserved). L2 (pluggable HTTP OCR) introduces a network surface — must be opt-in only and is gated by P5 (simplicity). No human-subjects concerns. |
| **Relevant** | 8 | docproc's OCR is the entry point for the entire corpus pipeline. Efficiency gains compound downstream. But relevance depends on corpus composition — if most docs are text-native PDFs, L1 (triage) is the only high-ROI change. |

### Refinement for the lowest dimension (Novel = 6)

The refined question must move beyond "copy LiteParse" toward "which LiteParse patterns, adapted to hKask's cybernetic model, produce measurable OCR efficiency gains without violating P5 (simplicity) or P7 (deep module)?"

The novelty is reframed as: **cybernetic-governance adaptation of standalone parser patterns** — every adopted pattern must emit `cns.pipeline.*` spans, every threshold change must follow the calibration discipline (≥100 samples, >95% agreement, human approval, never auto-adjust), and every new tool must be opt-in.

---

## 3. PICO Structure

| Element | Definition | Completeness |
|---|---|---|
| **Population** | PDF documents processed by `docproc_convert` in hKask v0.31.0, spanning text-native, scanned, mixed (part vector / part scanned), and multi-column layouts. Inclusion: PDFs only (DOCX/XLSX/PPTX already have native backends). Exclusion: encrypted PDFs (L10 — out of scope, low priority). | Complete |
| **Intervention** | A subset of {L1 complexity triage, L2 pluggable OCR API, L3 bounding-box preservation, L4 native+OCR merge, L5 grid projection, L6 screenshots, L8 page-range}. The specific subset is determined by Phase 2 (falsifiability) and Phase 3 (sequential-inquiry). | Partial — subset TBD |
| **Comparison** | docproc's current pipeline: Sobel scoring → route → OCR → verify → calibrate, text-only output, no standalone triage tool, no spatial provenance, no page-range support. The inline check at `mcp-servers/hkask-mcp-docproc/src/lib.rs:299` (`word_count < OCR_FALLBACK_WORD_THRESHOLD`) is the existing fast-path. | Complete |
| **Outcome** | **Primary:** OCR wall-clock time per document. **Secondary:** text-extraction word-count accuracy vs. ground truth. **Tertiary:** downstream QA-generation quality (measured by the existing John Brooks persona comparison in the `qa-generation` skill). | Complete |

---

## 4. Synthesized Research Question

> "In hKask's docproc OCR pipeline, which subset of LiteParse-derived patterns — when adapted to preserve CNS observability and affirmative-consent calibration — produces a measurable reduction in OCR wall-clock time per document without degrading extraction accuracy on text-native, scanned, and mixed PDFs?"

**Question type:** Intervention (multi-pattern adoption study).

---

## 5. Hypothesis Type

**Difference / superiority** (multi-arm). Each pattern subset is an arm; the comparison is the current pipeline. Non-inferiority margin (δ) applies to extraction accuracy: a pattern may not reduce word-count accuracy by more than 5% relative to the current pipeline on the same document.

---

## 6. Research Hypothesis (H₁)

> In PDF documents processed by `docproc_convert` (hKask v0.31.0), adopting the subset of LiteParse-derived patterns that survive Phase 2 falsifiability will reduce OCR wall-clock time per document by ≥30% on text-native PDFs and ≥20% on mixed PDFs, without reducing extraction word-count accuracy by more than 5% relative to the current Sobel-route-OCR-verify-calibrate pipeline.

---

## 7. Null Hypothesis (H₀)

> In PDF documents processed by `docproc_convert`, there is no difference in OCR wall-clock time per document or extraction word-count accuracy between the current pipeline and any subset of LiteParse-derived patterns adapted under CNS observability and affirmative-consent calibration.

---

## 8. Primary Aim

> The primary aim of this study is to **identify and validate** the subset of LiteParse-derived patterns that, when adapted to hKask's cybernetic-governance model, reduces docproc OCR wall-clock time per document without degrading extraction accuracy.

---

## 9. Primary Objectives

1. **Identify** which LiteParse patterns (L1–L10) are admissible, falsifiable, and testable in hKask's context (Phase 2 output: a falsification log with verdicts).
2. **Implement** the surviving patterns as vertical slices (Phase 4 output: `tasks/plan.md` + `tasks/todo.md`).
3. **Measure** OCR wall-clock time per document on a 50-PDF benchmark corpus (25 text-native, 15 scanned, 10 mixed) before and after adoption (Phase 7 output: kata-improvement baseline + post-implementation measurement).
4. **Verify** that extraction word-count accuracy does not degrade by more than 5% on any corpus subset.

### Secondary aim

Characterize the cybernetic-governance overhead (CNS span emission, calibration discipline) of each adopted pattern, to quantify the cost of the novelty dimension.

---

## 10. Testability Assessment

| Criterion | Status |
|---|---|
| Measurable outcome with validated method | ✅ Wall-clock time via `time`; word count via `split_whitespace().count()` (existing pattern at `lib.rs:298`) |
| Specified population | ✅ PDFs in hKask v0.31.0, four layout categories |
| Defined comparison | ✅ Current pipeline (Sobel → route → OCR → verify → calibrate) |
| Suggested statistical test | ✅ Paired t-test on per-document wall-clock times (before vs. after), α = 0.05, n = 50 |
| Clinically meaningful effect size | ✅ ≥30% wall-clock reduction on text-native; ≥20% on mixed; ≤5% accuracy degradation |
| Non-inferiority margin (δ) | ✅ δ = 5% word-count accuracy |
| Sample size feasibility | ✅ 50 PDFs is achievable; corpus does not yet exist (Phase 7 obstacle) |

---

## 11. Five-Link Alignment

| Link | Status | Notes |
|---|---|---|
| Question → Hypothesis | ✅ Aligned | Hypothesis directly answers "which subset… produces a measurable reduction…" |
| Hypothesis → Primary Aim | ✅ Aligned | Aim is to "identify and validate" — hypothesis is the validation claim |
| Primary Aim → Objectives | ✅ Aligned | Each objective maps to a phase (2, 4, 7) |
| Objectives → PICO Outcome | ✅ Aligned | Objectives 3 and 4 measure the primary and secondary outcomes |
| Hypothesis → Null Hypothesis | ✅ Aligned | H₀ postulates no difference; H₁ postulates a directional difference |

No misalignments detected.

---

## 12. Feasibility Recheck

After operationalizing into aims and objectives, new concerns:

| Concern | Severity | Mitigation |
|---|---|---|
| No 50-PDF benchmark corpus exists | High | Phase 7 Step 2 must assemble the corpus before any code is written. Use hKask's existing corpus if available; otherwise synthesize from public PDFs (arXiv papers for text-native, scanned books from archive.org for scanned, mixed via manual inset). |
| L3 (bounding boxes) requires changes to `hkask-types` — a foundation crate | Medium | VS5 is gated on Phase 2's H3 falsification. Only proceed if a downstream consumer is identified. |
| L5 (grid projection) is a new heuristic stage with no existing test coverage | Medium | VS3 is sized M; if `pdftotext -layout` + `markdown_to_structure` produces usable `Block::Heading`, L5 is unnecessary (Phase 3 Branch B). |
| L2 (pluggable HTTP OCR) introduces a network surface | Low | Out of scope for initial adoption; revisit only if H4 (CJK corpus) is corroborated. |

No new concerns block Phase 2.

---

## 13. Convergence Metric

| Dimension | Weight | Score (0 = ready, 1 = blocked) | Weighted |
|---|---|---|---|
| FINER compliance (all dimensions ≥ 7, or refinement plan for lowest) | 0.25 | 0.10 (Novel = 6, but refinement plan exists) | 0.025 |
| PICO completeness | 0.25 | 0.20 (Intervention subset is partial — TBD by Phase 2) | 0.050 |
| Hypothesis coherence (testable, directional, falsifiable) | 0.25 | 0.05 | 0.013 |
| Aims alignment (5-link verified) | 0.25 | 0.05 | 0.013 |
| **Total** | | | **0.101** |

**Convergence metric: 0.101** (threshold 0.05). Not yet converged — the partial PICO completeness (intervention subset TBD) is the blocker. This is expected: Phase 2 (falsifiability) resolves which patterns survive, which closes the PICO intervention element.

**Decision:** Proceed to Phase 2 (falsifiability) to resolve the intervention subset. Re-evaluate convergence after Phase 2.

---

## 14. Blockers

| Blocker | Resolution path |
|---|---|
| Intervention subset is partial | Phase 2 falsifiability eliminates inadmissible/untestable patterns; Phase 3 sequential-inquiry resolves trade-offs |
| No benchmark corpus | Phase 7 Step 2 (kata-improvement baseline) assembles corpus before implementation |

---

## 15. PDCA Iteration 1 Verdict

**Not converged (0.101 > 0.05).** Iteration 2 will occur after Phase 2 produces the falsification log. The refined intervention subset will replace the partial PICO element, and convergence is expected to drop below threshold.
