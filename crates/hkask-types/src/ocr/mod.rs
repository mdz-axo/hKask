pub mod config;
pub mod document;

pub use config::*;
pub use document::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_outcome_roundtrip() {
        let outcome = PipelineOutcome {
            results: vec![OcrResult {
                page_index: 0,
                backend: OcrBackend::Tesseract,
                text: "Hello".into(),
                confidence: 0.95,
                duration_ms: 100,
                was_fallback: false,
            }],
            report: VerificationReport::new(true, 0.0, vec![], 0, vec![]),
            cross_validations: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let back: PipelineOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.results.len(), 1);
        assert!(back.report.passed);
    }

    #[test]
    fn verification_report_passed_is_computed() {
        let report = VerificationReport::new(false, 0.0, vec![], 0, vec![]);
        assert!(!report.passed, "page_count_match=false should fail");

        let report = VerificationReport::new(true, 0.0, vec![0], 0, vec![]);
        assert!(!report.passed, "empty page should fail");

        let report = VerificationReport::new(true, 0.0, vec![], 3, vec![]);
        assert!(!report.passed, "errors should fail");

        let report = VerificationReport::new(true, 60.0, vec![], 0, vec![]);
        assert!(!report.passed, "word delta >50% should fail");

        let report = VerificationReport::new(true, 10.0, vec![], 0, vec![]);
        assert!(report.passed, "clean report should pass");
    }

    #[test]
    fn complexity_tier_ordering() {
        assert!(ComplexityTier::Simple < ComplexityTier::Moderate);
        assert!(ComplexityTier::Moderate < ComplexityTier::Complex);
    }

    #[test]
    fn ocr_backend_labels() {
        assert_eq!(OcrBackend::Tesseract.label(), "tesseract");
        assert_eq!(OcrBackend::LlmOcr("lighton".into()).label(), "llm-ocr");
        assert_eq!(OcrBackend::LlmOcr("gpt4".into()).label(), "llm-ocr");
    }

    #[test]
    fn pipeline_error_display() {
        let err = PipelineError::OcrFailed {
            page_index: 2,
            backends_tried: vec![OcrBackend::Tesseract, OcrBackend::LlmOcr("lighton".into())],
        };
        let display = err.to_string();
        assert!(display.contains("page 2"));
        assert!(display.contains("tesseract"));
        assert!(display.contains("lighton"));
    }

    #[test]
    fn cross_validation_roundtrip() {
        let cv = CrossValidation {
            page_index: 3,
            similarity: 0.87,
            tier: ComplexityTier::Moderate,
            backend_a: OcrBackend::Tesseract,
            backend_b: OcrBackend::LlmOcr("minicpm".into()),
            confidence_a: 0.92,
            confidence_b: 0.89,
            semantic_similarity: None,
        };
        let json = serde_json::to_string(&cv).unwrap();
        let back: CrossValidation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.similarity, 0.87);
    }
}
