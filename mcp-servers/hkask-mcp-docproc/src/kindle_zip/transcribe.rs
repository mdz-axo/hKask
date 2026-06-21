//! OCR transcription of Kindle page screenshots via hKask's multi-backend pipeline.
//!
//! Loads PNG pages, routes through Tesseract (simple) and LLM OCR (complex),
//! writes content.json with MDS provenance, and filters blank/empty pages.

use std::path::Path;
use std::time::Instant;

use crate::kindle_zip::types::{BookMetadata, ContentChunk, ProvenanceRecord, TranscribeResult};
use crate::ocr::pipeline::run_pipeline;

/// Transcribe page screenshots to text using the OCR pipeline.
///
/// Gap 14: Filters chunks whose text is "BLANK" (per ocr-system-prompt.j2 rules).
/// Gap 16: Emits per-page CNS spans with word count and confidence.
/// Gap 5: OCR prompt — the `ocr-system-prompt.j2` template specifies book-page-specific
///   rules (ignore page numbers, preserve paragraph breaks). The `LlmOcrExecutor`
///   uses its own hardcoded prompt. For template-driven prompts, the executor
///   must be extended to accept a custom `system_prompt` parameter.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn transcribe_pages(
    _pages_dir: &Path,
    metadata_path: &Path,
    output_dir: &Path,
    asin: &str,
    ocr_executor: &(dyn crate::ocr::pipeline::OcrExecutor + '_),
    ocr_thresholds: &crate::ocr::ThresholdConfig,
    ocr_model: Option<&str>,
    embedding_router: Option<(&hkask_inference::EmbeddingRouter, &str)>,
) -> Result<TranscribeResult, String> {
    let start = Instant::now();
    let cns_span_id = uuid::Uuid::new_v4().to_string();

    let json =
        std::fs::read_to_string(metadata_path).map_err(|e| format!("Read metadata: {}", e))?;
    let metadata: BookMetadata =
        serde_json::from_str(&json).map_err(|e| format!("Parse metadata: {}", e))?;

    let mut sorted: Vec<&crate::kindle_zip::types::PageEntry> = metadata.pages.iter().collect();
    sorted.sort_by_key(|p| p.index);

    let mut images: Vec<image::DynamicImage> = Vec::with_capacity(sorted.len());
    let mut page_map: Vec<usize> = Vec::with_capacity(sorted.len());

    for entry in &sorted {
        let img = image::open(&entry.screenshot)
            .map_err(|e| format!("Open {}: {}", entry.screenshot.display(), e))?;
        images.push(img);
        page_map.push(entry.page);
    }

    let expected = images.len();
    if expected == 0 {
        return Err("No page images found".to_string());
    }

    tracing::info!(target: "cns.pipeline.kindle-zip.transcribe",
        asin = %asin, pages = expected, span_id = %cns_span_id, "Starting OCR transcription");

    let outcome = run_pipeline(
        images,
        expected,
        ocr_executor,
        ocr_thresholds,
        ocr_model,
        embedding_router,
    )
    .await;

    let mut chunks: Vec<ContentChunk> = Vec::with_capacity(outcome.results.len());
    let mut total_words = 0usize;
    let mut total_confidence = 0.0f32;
    let mut blank_filtered = 0usize;

    let param_hash = Some(format!(
        "{:x}",
        md5::compute(serde_json::to_string(&ocr_thresholds).unwrap_or_default())
    ));

    for result in &outcome.results {
        // Gap 14: Filter blank pages (OCR produced "BLANK" per prompt rules)
        if result.text.trim() == "BLANK" || result.text.trim().is_empty() {
            blank_filtered += 1;
            tracing::debug!(target: "cns.pipeline.kindle-zip.transcribe.page",
                page = result.page_index + 1, "Blank page filtered");
            continue;
        }

        let page = page_map
            .get(result.page_index)
            .copied()
            .unwrap_or(result.page_index + 1);
        let wc = result.text.split_whitespace().count();
        total_words += wc;
        total_confidence += result.confidence;

        // Gap 16: Per-page CNS span
        tracing::debug!(target: "cns.pipeline.kindle-zip.transcribe.page",
            page, word_count = wc, confidence = format!("{:.3}", result.confidence),
            backend = %result.backend.label(), "Page transcribed");

        chunks.push(ContentChunk {
            index: result.page_index,
            page,
            text: result.text.clone(),
            screenshot: None,
            confidence: Some(result.confidence),
            provenance: Some(ProvenanceRecord {
                step_id: "kindle-zip.transcribe".into(),
                engine: result.backend.label().to_string(),
                model: ocr_model.map(String::from),
                parameter_hash: param_hash.clone(),
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
            }),
        });
    }

    let transcribed = chunks.len();
    let failed = expected.saturating_sub(outcome.results.len());
    let mean_confidence = if transcribed > 0 {
        total_confidence / transcribed as f32
    } else {
        0.0
    };

    let content_path = output_dir.join(asin).join("content.json");
    let content_json =
        serde_json::to_string_pretty(&chunks).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&content_path, content_json).map_err(|e| format!("Write: {}", e))?;

    let elapsed = start.elapsed();
    tracing::info!(target: "cns.pipeline.kindle-zip.transcribe",
        asin = %asin, span_id = %cns_span_id,
        transcribed, failed, blank_filtered, total_words,
        mean_confidence = format!("{:.3}", mean_confidence),
        duration_s = elapsed.as_secs(), "Transcription complete");

    Ok(TranscribeResult {
        content_path,
        total_words,
        transcribed_pages: transcribed,
        failed_pages: failed + blank_filtered,
        mean_confidence,
        cns_span_id: Some(cns_span_id),
    })
}

/// Gap 7: Assemble transcribed content into chapter-structured text.
///
/// Basic assembly: joins chunks with paragraph breaks, filters blanks.
/// Full strategy is described in `assemble-content.j2` (KnowAct template):
/// TOC-anchored chapter boundaries, artifact removal, whitespace normalization.
/// For LLM-driven assembly, invoke the template via hkask-templates renderer.
pub fn assemble_chunks(
    chunks: &[ContentChunk],
    _toc: &[crate::kindle_zip::types::TocItem],
) -> String {
    chunks
        .iter()
        .map(|c| c.text.trim().to_string())
        .filter(|t| !t.is_empty() && t != "BLANK")
        .collect::<Vec<_>>()
        .join("\n\n")
}
