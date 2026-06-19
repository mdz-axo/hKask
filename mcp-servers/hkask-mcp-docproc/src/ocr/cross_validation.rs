//! Cross-Validation — Compare dual-routed OCR results for calibration.
//!
//! Observation only (P4: no autonomous routing change).
//! Data accumulates for future threshold self-tuning.

use hkask_types::ocr::{ComplexityTier, CrossValidation, OcrResult};

/// Compute cross-validation between two OCR results for the same page.
///
/// Returns `None` if the results are not comparable (different page index).
/// Otherwise computes normalized Levenshtein similarity and bundles
/// per-backend confidence scores with the complexity tier.
pub fn compute_cross_validation(
    primary: &OcrResult,
    secondary: &OcrResult,
) -> Option<CrossValidation> {
    if primary.page_index != secondary.page_index {
        return None;
    }

    let similarity = normalized_levenshtein_similarity(&primary.text, &secondary.text);

    Some(CrossValidation {
        page_index: primary.page_index,
        similarity,
        tier: ComplexityTier::Moderate, // dual-routing only happens for Moderate
        backend_a: primary.backend.clone(),
        backend_b: secondary.backend.clone(),
        confidence_a: primary.confidence,
        confidence_b: secondary.confidence,
        semantic_similarity: None,
    })
}

/// Normalized Levenshtein similarity in [0.0, 1.0].
///
/// 1.0 = identical texts; 0.0 = completely different.
fn normalized_levenshtein_similarity(a: &str, b: &str) -> f32 {
    let dist = levenshtein_distance(a, b);
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (dist as f32 / max_len as f32)
}

/// Compute Levenshtein (edit) distance between two strings.
///
/// Uses the standard dynamic programming approach with O(n·m) time
/// and O(min(n,m)) space (single-row optimization).
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    // Ensure a is the shorter string for space optimization
    if a_len > b_len {
        return levenshtein_distance(b, a);
    }

    let mut prev_row: Vec<usize> = (0..=a_len).collect();
    let mut curr_row: Vec<usize> = vec![0; a_len + 1];

    for j in 1..=b_len {
        curr_row[0] = j;
        for i in 1..=a_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[i] = (curr_row[i - 1] + 1) // insertion
                .min(prev_row[i] + 1) // deletion
                .min(prev_row[i - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[a_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ocr::OcrBackend;

    // contract: ocr-xval-01
    #[test]
    fn identical_texts() {
        let sim = normalized_levenshtein_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < 0.001);
    }

    // contract: ocr-xval-02
    #[test]
    fn completely_different_texts() {
        let sim = normalized_levenshtein_similarity("abc", "xyz");
        assert!(sim < 0.5);
    }

    // contract: ocr-xval-03
    #[test]
    fn empty_strings() {
        let sim = normalized_levenshtein_similarity("", "");
        assert!((sim - 1.0).abs() < 0.001);
    }

    // contract: ocr-xval-04
    #[test]
    fn cross_validation_same_page() {
        let primary = OcrResult {
            page_index: 0,
            backend: OcrBackend::Tesseract,
            text: "The quick brown fox".into(),
            confidence: 0.95,
            duration_ms: 100,
            was_fallback: false,
        };
        let secondary = OcrResult {
            page_index: 0,
            backend: OcrBackend::LlmOcr("minicpm".into()),
            text: "The quick brown fox jumps".into(),
            confidence: 0.89,
            duration_ms: 200,
            was_fallback: false,
        };

        let cv = compute_cross_validation(&primary, &secondary);
        assert!(cv.is_some());
        let cv = cv.unwrap();
        assert_eq!(cv.page_index, 0);
        assert!(
            cv.similarity > 0.5,
            "similarity should be high for similar texts"
        );
        assert_eq!(cv.confidence_a, 0.95);
        assert_eq!(cv.confidence_b, 0.89);
    }

    // contract: ocr-xval-05
    #[test]
    fn cross_validation_different_pages() {
        let primary = OcrResult {
            page_index: 0,
            backend: OcrBackend::Tesseract,
            text: "page zero".into(),
            confidence: 0.9,
            duration_ms: 100,
            was_fallback: false,
        };
        let secondary = OcrResult {
            page_index: 1,
            backend: OcrBackend::LlmOcr("minicpm".into()),
            text: "page one".into(),
            confidence: 0.9,
            duration_ms: 100,
            was_fallback: false,
        };

        let cv = compute_cross_validation(&primary, &secondary);
        assert!(
            cv.is_none(),
            "different page indices should not produce cross-validation"
        );
    }

    // contract: ocr-xval-06
    #[test]
    fn levenshtein_edge_cases() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("a", ""), 1);
        assert_eq!(levenshtein_distance("", "a"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }
}
