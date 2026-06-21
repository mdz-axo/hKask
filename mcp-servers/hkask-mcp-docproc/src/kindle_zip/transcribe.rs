//! OCR transcription of Kindle page screenshots via hKask's multi-backend pipeline.
//!
//! Loads PNG pages, routes through Tesseract (simple) and LLM OCR (complex),
//! and writes transcribed content to content.json with MDS provenance.
//!
//! Public surface: 1 function (`transcribe_pages`).

use std::path::Path;
use std::time::Instant;

use crate::kindle_zip::types::{BookMetadata, ContentChunk, ProvenanceRecord, TranscribeResult};
use crate::ocr::pipeline::run_pipeline;

/// Transcribe page screenshots to text using the OCR pipeline.
///
/// Emits CNS span `kindle-zip.transcribe` with per-page and aggregate metrics.
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

    // Load metadata
    let json =
        std::fs::read_to_string(metadata_path).map_err(|e| format!("Read metadata: {}", e))?;
    let metadata: BookMetadata =
        serde_json::from_str(&json).map_err(|e| format!("Parse metadata: {}", e))?;

    // Load page images in sorted order
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

    tracing::info!(
        target: "cns.pipeline.kindle-zip.transcribe",
        asin = %asin, pages = expected, span_id = %cns_span_id,
        "Starting OCR transcription"
    );

    // Run the multi-backend OCR pipeline
    let outcome = run_pipeline(
        images,
        expected,
        ocr_executor,
        ocr_thresholds,
        ocr_model,
        embedding_router,
    )
    .await;

    // Build content chunks with provenance
    let mut chunks: Vec<ContentChunk> = Vec::with_capacity(outcome.results.len());
    let mut total_words = 0usize;
    let mut total_confidence = 0.0f32;

    let param_hash = Some(format!(
        "{:x}",
        md5::compute(serde_json::to_string(&ocr_thresholds).unwrap_or_default())
    ));

    for result in &outcome.results {
        let page = page_map
            .get(result.page_index)
            .copied()
            .unwrap_or(result.page_index + 1);
        let wc = result.text.split_whitespace().count();
        total_words += wc;
        total_confidence += result.confidence;

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

    let transcribed = outcome.results.len();
    let failed = expected.saturating_sub(transcribed);
    let mean_confidence = if transcribed > 0 {
        total_confidence / transcribed as f32
    } else {
        0.0
    };

    // Write content.json
    let content_path = output_dir.join(asin).join("content.json");
    let content_json =
        serde_json::to_string_pretty(&chunks).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&content_path, content_json).map_err(|e| format!("Write: {}", e))?;

    let elapsed = start.elapsed();
    tracing::info!(
        target: "cns.pipeline.kindle-zip.transcribe",
        asin = %asin, span_id = %cns_span_id,
        transcribed, failed, total_words,
        mean_confidence = format!("{:.3}", mean_confidence),
        duration_s = elapsed.as_secs(),
        "Transcription complete"
    );

    Ok(TranscribeResult {
        content_path,
        total_words,
        transcribed_pages: transcribed,
        failed_pages: failed,
        mean_confidence,
        cns_span_id: Some(cns_span_id),
    })
}
