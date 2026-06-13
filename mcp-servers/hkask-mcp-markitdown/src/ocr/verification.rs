//! Verification Checkpoint — Post-pipeline quality signal.
//!
//! The only module that answers "is this output good?"
//! Delete it → pipeline produces output with no quality signal.
//! It earns its existence.

use hkask_types::ocr::{PageVerificationDetail, PipelineError, VerificationReport};
use image::DynamicImage;

/// Verify assembled output against expected page count and source images.
///
/// # Checks
/// 1. Page count match: actual pages vs expected.
/// 2. Empty-page detection: flag pages with zero text.
/// 3. Word-count heuristic: flag if delta > 50% (coarse guardrail).
/// 4. Error tally: count all pipeline errors.
///
/// `passed = (error_count == 0 && all_checks_pass)`.
pub fn verify_output(
    assembled_text: &str,
    expected_pages: usize,
    page_images: &[DynamicImage],
    errors: &[PipelineError],
) -> VerificationReport {
    // Count pages from assembled text (count page markers)
    let actual_pages = assembled_text.match_indices("--- PAGE ").count();

    let page_count_match = actual_pages == expected_pages;

    // Detect empty pages by splitting on page markers
    let mut empty_pages: Vec<usize> = Vec::new();
    let mut page_details: Vec<PageVerificationDetail> = Vec::new();
    let mut total_words: usize = 0;

    let sections: Vec<&str> = assembled_text
        .split("--- PAGE ")
        .filter(|s| !s.is_empty())
        .collect();

    for (idx, section) in sections.iter().enumerate() {
        // Extract content after "N ---\n"
        let content = if let Some(newline_pos) = section.find('\n') {
            &section[newline_pos + 1..]
        } else {
            section
        };
        let word_count = content.split_whitespace().count();
        let is_empty = content.trim().is_empty();

        total_words += word_count;

        if is_empty {
            empty_pages.push(idx);
        }

        page_details.push(PageVerificationDetail {
            page_index: idx,
            word_count,
            is_empty,
            backend_used: None, // filled by caller if needed
            was_fallback: false,
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
    let total_pixels: u64 = pages
        .iter()
        .map(|img| {
            let (w, h) = (img.width() as u64, img.height() as u64);
            w * h
        })
        .sum();

    // Heuristic: ~2000 pixels per word (roughly 40×50 px per word area)
    (total_pixels / 2000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ:ocr-verify-01 — Correct document passes verification
    #[test]
    fn correct_document_passes() {
        let text = "--- PAGE 1 ---\nThe quick brown fox jumps over the lazy dog.\n\n--- PAGE 2 ---\nThe second page has more text content for testing.\n";

        // Two blank images (low pixel density → low expected words)
        let pages = vec![
            DynamicImage::new_luma8(100, 100),
            DynamicImage::new_luma8(100, 100),
        ];

        let report = verify_output(text, 2, &pages, &[]);
        assert!(report.page_count_match, "page count should match");
        assert!(report.empty_pages.is_empty(), "no empty pages");
        assert!(report.passed, "clean document should pass: {:#?}", report);
    }

    // REQ:ocr-verify-02 — Missing page fails verification
    #[test]
    fn missing_page_fails() {
        let text = "--- PAGE 1 ---\nSome content.\n";
        let pages = vec![DynamicImage::new_luma8(100, 100)]; // 2 expected but only 1 actual section
        let report = verify_output(text, 2, &pages, &[]);
        assert!(
            !report.page_count_match,
            "page count mismatch should be detected"
        );
        assert!(!report.passed, "mismatch should cause failure");
    }

    // REQ:ocr-verify-03 — Empty page is flagged
    #[test]
    fn empty_page_flagged() {
        let text = "--- PAGE 1 ---\nSome text.\n\n--- PAGE 2 ---\n   \n";
        let pages = vec![
            DynamicImage::new_luma8(100, 100),
            DynamicImage::new_luma8(100, 100),
        ];
        let report = verify_output(text, 2, &pages, &[]);
        assert!(
            !report.empty_pages.is_empty(),
            "empty page should be flagged"
        );
        assert_eq!(report.empty_pages, vec![1]); // 0-indexed
        assert!(!report.passed, "empty page should cause failure");
    }

    // REQ:ocr-verify-04 — Garbled text flags word anomaly
    #[test]
    fn garbled_text_flags_word_anomaly() {
        // Lots of text on a tiny image — should be a word count anomaly
        let mut lots_of_words = String::from("--- PAGE 1 ---\n");
        for i in 0..500 {
            lots_of_words.push_str(&format!("word{} ", i));
        }
        lots_of_words.push('\n');

        // Tiny image → low estimated word count
        let pages = vec![DynamicImage::new_luma8(10, 10)];
        let report = verify_output(&lots_of_words, 1, &pages, &[]);
        // 500 words actual vs ~0.05 words estimated → huge delta
        assert!(
            report.word_count_delta_pct > 50.0,
            "word count anomaly should be large, got {:.1}%",
            report.word_count_delta_pct
        );
        assert!(!report.passed, "word anomaly should cause failure");
    }
}
