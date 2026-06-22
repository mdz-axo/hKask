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
pub async fn transcribe_pages(
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
        if result.text.trim() == "BLANK" || result.text.trim().is_empty() {
            blank_filtered += 1;
            tracing::debug!(target: "cns.pipeline.kindle-zip.transcribe.page",
                page = result.page_index + 1, "Blank page filtered");
            continue;
        }

        // Clean Kindle UI chrome from OCR output
        let cleaned = clean_kindle_text(&result.text);

        let page = page_map
            .get(result.page_index)
            .copied()
            .unwrap_or(result.page_index + 1);
        let wc = cleaned.split_whitespace().count();
        total_words += wc;
        total_confidence += result.confidence;

        tracing::debug!(target: "cns.pipeline.kindle-zip.transcribe.page",
            page, word_count = wc, confidence = format!("{:.3}", result.confidence),
            backend = %result.backend.label(), "Page transcribed");

        chunks.push(ContentChunk {
            index: result.page_index,
            page,
            text: cleaned,
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

    // Strip repeated running headers (book title on every page)
    strip_repeated_headers(&mut chunks);
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

/// Clean Kindle UI chrome from transcribed text.
/// Strips toolbar fragments, page indicators, reading speed, garbled OCR.
pub fn clean_kindle_text(raw: &str) -> String {
    let cleaned: String = raw
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return false;
            }
            // Kindle toolbar fragments
            if t == "= Q Aa" || t.starts_with("= Q") || t == "Aa" || t == "Q Aa" {
                return false;
            }
            // Page/location indicators
            if t.starts_with("Learning reading speed") {
                return false;
            }
            if (t.starts_with("Location ") || t.starts_with("Page "))
                && (t.contains("of") || t.contains("%"))
            {
                return false;
            }
            if t.ends_with("%") && t.len() <= 6 {
                return false;
            }
            // Garbled OCR artifacts
            if t.contains("â") && t.len() < 10 {
                return false;
            }
            if t.contains("eo") && t.len() < 10 {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n");
    // Collapse 3+ consecutive newlines
    let mut result = String::with_capacity(cleaned.len());
    let mut blank_count = 0u8;
    for ch in cleaned.chars() {
        if ch == '\n' {
            blank_count += 1;
            if blank_count <= 2 {
                result.push(ch);
            }
        } else {
            blank_count = 0;
            result.push(ch);
        }
    }
    result.trim().to_string()
}

/// Strip repeated headers that appear on >30% of pages (book title as running header).
pub fn strip_repeated_headers(chunks: &mut [ContentChunk]) {
    if chunks.len() < 3 {
        return;
    }
    // Collect first lines from each page (scoped so borrow drops before mutation)
    let headers: Vec<String> = {
        use std::collections::HashMap;
        let first_lines: Vec<&str> = chunks
            .iter()
            .map(|c| c.text.lines().next().unwrap_or(""))
            .collect();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for line in &first_lines {
            if line.len() > 10
                && line
                    .chars()
                    .all(|c| c.is_uppercase() || c.is_whitespace() || c == ':')
            {
                *counts.entry(line).or_insert(0) += 1;
            }
        }
        let threshold = (chunks.len() as f64 * 0.3) as usize;
        counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold.max(2))
            .map(|(line, _)| line.to_string())
            .collect()
    }; // first_lines borrow released here

    if headers.is_empty() {
        return;
    }

    for chunk in chunks.iter_mut() {
        let fl_end = chunk.text.find('\n').unwrap_or(chunk.text.len());
        let first_line = chunk.text[..fl_end].to_string();
        let is_header = headers.iter().any(|h| first_line.contains(h));
        if is_header && fl_end < chunk.text.len() {
            // Copy the remaining text out before mutating
            let rest = chunk.text[fl_end + 1..].trim().to_string();
            if !rest.is_empty() {
                chunk.text = rest;
            }
        }
    }
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
