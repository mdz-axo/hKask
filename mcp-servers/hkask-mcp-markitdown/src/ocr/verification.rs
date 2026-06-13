//! Verification Checkpoint — Post-pipeline quality signal.
//!
//! The only module that answers "is this output good?"
//! Delete it → pipeline produces output with no quality signal.
//! It earns its existence.

use hkask_types::ocr::{OcrResult, PageVerificationDetail, PipelineError, VerificationReport};
use image::DynamicImage;

/// Verify assembled output against expected page count and source images.
///
/// # Checks
/// 1. Page count match: actual results vs expected images.
/// 2. Empty-page detection: flag pages with zero text.
/// 3. Word-count heuristic: flag if delta > 50% (coarse guardrail).
/// 4. Error tally: count all pipeline errors.
///
/// `passed = (error_count == 0 && all_checks_pass)`.
pub fn verify_output(
    expected_pages: usize,
    results: &[OcrResult],
    page_images: &[DynamicImage],
    errors: &[PipelineError],
) -> VerificationReport {
    let actual_pages = results.len();
    let page_count_match = actual_pages == expected_pages;

    // Detect empty pages and collect per-page details from results
    let mut empty_pages: Vec<usize> = Vec::new();
    let mut page_details: Vec<PageVerificationDetail> = Vec::new();
    let mut total_words: usize = 0;

    for idx in 0..actual_pages {
        let result = &results[idx];
        let text = &result.text;
        let word_count = text.split_whitespace().count();
        let is_empty = text.trim().is_empty();

        total_words += word_count;

        if is_empty {
            empty_pages.push(idx);
        }

        page_details.push(PageVerificationDetail {
            page_index: idx,
            word_count,
            is_empty,
            backend_used: Some(result.backend.clone()),
            was_fallback: result.was_fallback,
            error: None,
        });
    }

    // Estimate expected words from aggregate pixel density
    let estimated_words = estimate_word_count(page_images);
    let word_count_delta_pct = if estimated_words > 0 {
        ((total_words as f32 - estimated_words as f32) / estimated_words as f32) * 100.0
    } else {
        0.0
    };

    let error_count = errors.len();

    VerificationReport::new(
        page_count_match,
        word_count_delta_pct,
        empty_pages,
        error_count,
        page_details,
    )
}

/// Crude word-count estimation from aggregate pixel density.
///
/// Not a precision metric — coarse guardrail only.
/// Assumes ~2000 pixels per word on average for 300 DPI text.
fn estimate_word_count(pages: &[DynamicImage]) -> usize {
    if pages.is_empty() {
        return 0;
    }
    let total_pixels: u64 = pages
        .iter()
        .map(|img| {
            let (w, h) = (img.width() as u64, img.height() as u64);
            w * h
        })
        .sum();

    // Heuristic: ~2000 pixels per word (roughly 40×50 px per word area)
    // Minimum 1 to avoid divide-by-zero in delta calculation
    (total_pixels / 2000).max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ:ocr-verify-01 — Correct document passes verification
    #[test]
    fn correct_document_passes() {
        use hkask_types::ocr::OcrBackend;

        let results = vec![
            OcrResult {
                page_index: 0,
                backend: OcrBackend::Tesseract,
                text: "The quick brown fox jumps over the lazy dog.".into(),
                confidence: 0.95,
                duration_ms: 100,
                was_fallback: false,
            },
            OcrResult {
                page_index: 1,
                backend: OcrBackend::Tesseract,
                text: "The second page has more text content for testing.".into(),
                confidence: 0.90,
                duration_ms: 120,
                was_fallback: false,
            },
        ];

        // Two images sized to roughly match word count
        let pages = vec![
            DynamicImage::new_luma8(140, 140),
            DynamicImage::new_luma8(140, 140),
        ];

        let report = verify_output(2, &results, &pages, &[]);
        assert!(report.page_count_match, "page count should match");
        assert!(report.empty_pages.is_empty(), "no empty pages");
        assert!(report.passed, "clean document should pass: {:#?}", report);
        // Check that per-page details have backend info
        assert_eq!(report.page_details.len(), 2);
        assert_eq!(
            report.page_details[0].backend_used,
            Some(OcrBackend::Tesseract)
        );
    }

    // REQ:ocr-verify-02 — Missing page fails verification (results < expected)
    #[test]
    fn missing_page_fails() {
        use hkask_types::ocr::OcrBackend;

        let results = vec![OcrResult {
            page_index: 0,
            backend: OcrBackend::Tesseract,
            text: "Some content.".into(),
            confidence: 0.9,
            duration_ms: 50,
            was_fallback: false,
        }];
        let pages = vec![DynamicImage::new_luma8(100, 100)];
        // 1 result, 2 expected → mismatch
        let report = verify_output(2, &results, &pages, &[]);
        assert!(
            !report.page_count_match,
            "page count mismatch should be detected"
        );
        assert!(!report.passed, "mismatch should cause failure");
    }

    // REQ:ocr-verify-03 — Empty page is flagged
    #[test]
    fn empty_page_flagged() {
        use hkask_types::ocr::OcrBackend;

        let results = vec![
            OcrResult {
                page_index: 0,
                backend: OcrBackend::Tesseract,
                text: "Some text.".into(),
                confidence: 0.9,
                duration_ms: 50,
                was_fallback: false,
            },
            OcrResult {
                page_index: 1,
                backend: OcrBackend::LlmOcr("lighton".into()),
                text: "   ".into(), // whitespace-only = empty
                confidence: 0.0,
                duration_ms: 200,
                was_fallback: true,
            },
        ];
        let pages = vec![
            DynamicImage::new_luma8(100, 100),
            DynamicImage::new_luma8(100, 100),
        ];
        let report = verify_output(2, &results, &pages, &[]);
        assert!(
            !report.empty_pages.is_empty(),
            "empty page should be flagged"
        );
        assert_eq!(report.empty_pages, vec![1]);
        assert!(
            report.page_details[1].was_fallback,
            "fallback should be recorded"
        );
        assert!(!report.passed, "empty page should cause failure");
    }

    // REQ:ocr-verify-04 — Garbled text flags word anomaly
    #[test]
    fn garbled_text_flags_word_anomaly() {
        use hkask_types::ocr::OcrBackend;

        // Lots of words on a tiny image — should be a word count anomaly
        let mut text = String::new();
        for i in 0..500 {
            text.push_str(&format!("word{} ", i));
        }

        let results = vec![OcrResult {
            page_index: 0,
            backend: OcrBackend::Tesseract,
            text,
            confidence: 0.5,
            duration_ms: 50,
            was_fallback: false,
        }];

        // Tiny image → low estimated word count
        let pages = vec![DynamicImage::new_luma8(10, 10)];
        let report = verify_output(1, &results, &pages, &[]);
        // 500 words actual vs ~0.05 words estimated → huge delta
        assert!(
            report.word_count_delta_pct > 50.0,
            "word count anomaly should be large, got {:.1}%",
            report.word_count_delta_pct
        );
        assert!(!report.passed, "word anomaly should cause failure");
    }
}
