//! OCR Pipeline — Sequential state machine: Decimate → Score → Route → OCR → Assemble.
//!
//! ```text
//! PDF → [Decimate] → PageQueue → [Score → Route → OCR] → ResultBuffer → [Assembly] → VerifiedDocument
//! ```
//!
//! Deliberately sequential to guarantee page ordering without
//! reorder logic (P1: simplicity over concurrency).

use std::time::Instant;

use async_trait::async_trait;

use hkask_types::ocr::{
    CrossValidation, OcrBackend, OcrResult, PipelineError, PipelineOutcome, ThresholdConfig,
};
use image::DynamicImage;

use crate::ocr::complexity::score_page_complexity;
use crate::ocr::cross_validation::compute_cross_validation;
use crate::ocr::routing::{SamplingState, route_page};
use crate::ocr::verification::verify_output;

/// Trait for executing OCR on a single page image via a specific backend.
///
/// Implementors plug in the concrete invocation path for each backend
/// (Tesseract → local binary, LightOn → HTTP endpoint, LlmOcr → inference router).
/// Single async method with a clear contract.
#[async_trait]
pub trait OcrExecutor: Send + Sync {
    /// Execute OCR on a single page image.
    ///
    /// Returns `Ok(OcrResult)` on success, or `Err(String)` with a
    /// human-readable error message.
    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String>;
}

/// Run the OCR pipeline on a set of page images.
///
/// # Arguments
/// * `pages` — Decimated page images in document order.
/// * `executor` — Pluggable OCR executor for each backend.
/// * `thresholds` — Complexity scoring thresholds (configurable via registry).
/// * `llm_model` — Optional model ID for `LlmOcr` backend routing.
///
/// # Returns
/// `PipelineOutcome` — the single sealed output. No partial state escapes.
pub async fn run_pipeline(
    pages: &[DynamicImage],
    executor: &(dyn OcrExecutor + '_),
    thresholds: &ThresholdConfig,
    llm_model: Option<&str>,
) -> PipelineOutcome {
    let start = Instant::now();
    let expected_pages = pages.len();
    let mut state = SamplingState::new(thresholds.moderate_sample_rate);
    let mut results: Vec<OcrResult> = Vec::with_capacity(expected_pages);
    let mut errors: Vec<PipelineError> = Vec::new();
    let mut cross_validations: Vec<CrossValidation> = Vec::new();

    for (page_index, image) in pages.iter().enumerate() {
        // Step 1: Score complexity
        let score = score_page_complexity(image, thresholds);

        // Step 2: Route to backends
        let backends = route_page(score, &mut state, None, llm_model);

        if backends.is_empty() {
            // No backends available (all excluded)
            errors.push(PipelineError::OcrFailed {
                page_index,
                backends_tried: vec![],
            });
            continue;
        }

        // Step 3: Execute OCR (primary attempt)
        let mut primary_result: Option<OcrResult> = None;
        let mut secondary_result: Option<OcrResult> = None;
        let mut backends_tried: Vec<OcrBackend> = Vec::new();

        for (backend_idx, backend) in backends.iter().enumerate() {
            backends_tried.push(backend.clone());

            match executor.execute(page_index, backend, image, false).await {
                Ok(result) => {
                    if backend_idx == 0 {
                        primary_result = Some(result);
                    } else {
                        secondary_result = Some(result);
                    }
                }
                Err(_err_msg) => {
                    // Fallback: re-route with this backend excluded
                    let fallback_backends = route_page(score, &mut state, Some(backend), llm_model);

                    let mut fallback_ok = false;
                    for fb in &fallback_backends {
                        backends_tried.push(fb.clone());
                        match executor.execute(page_index, fb, image, true).await {
                            Ok(mut result) => {
                                result.was_fallback = true;
                                if backend_idx == 0 {
                                    primary_result = Some(result);
                                } else {
                                    secondary_result = Some(result);
                                }
                                fallback_ok = true;
                                break;
                            }
                            Err(_) => {
                                // Continue trying next fallback backend
                            }
                        }
                    }

                    if !fallback_ok {
                        errors.push(PipelineError::OcrFailed {
                            page_index,
                            backends_tried: backends_tried.clone(),
                        });
                    }
                }
            }

            // If primary succeeded, we have our result for this page
            if primary_result.is_some() {
                break;
            }
        }

        // If primary (first backend) failed completely, skip this page
        if primary_result.is_none() {
            continue;
        }

        let primary = primary_result.unwrap();

        // Step 4: Cross-validation if dual-routed
        if let Some(secondary) = secondary_result {
            if let Some(cv) = compute_cross_validation(&primary, &secondary) {
                cross_validations.push(cv);
            }
            // Store both results (primary is the authoritative one)
        }

        results.push(primary);
    }

    let _duration_ms = start.elapsed().as_millis() as u64;

    // Step 5: Assembly — concatenate results with page markers
    let _assembled = assemble_document(&results);

    // Step 6: Verification checkpoint
    let report = verify_output(expected_pages, &results, pages, &errors);

    PipelineOutcome {
        results,
        report,
        cross_validations,
        errors,
    }
}

/// Assemble OCR results into a single document string with page markers.
///
/// Pure function of the result buffer. No side effects.
fn assemble_document(results: &[OcrResult]) -> String {
    let mut assembled = String::new();
    for result in results {
        if !assembled.is_empty() {
            assembled.push('\n');
        }
        assembled.push_str(&format!(
            "--- PAGE {} ---\n{}",
            result.page_index + 1,
            result.text
        ));
    }
    assembled
}

/// Assembled document wrapper for verification.
pub struct AssembledDocument {
    pub text: String,
    pub page_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    /// Test executor that always returns a fixed result.
    struct TestExecutor {
        results: Vec<Option<String>>, // None = simulate failure
        call_count: std::sync::Mutex<usize>,
    }

    impl TestExecutor {
        fn new(results: Vec<Option<String>>) -> Self {
            Self {
                results,
                call_count: std::sync::Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl OcrExecutor for TestExecutor {
        async fn execute(
            &self,
            page_index: usize,
            backend: &OcrBackend,
            _image: &DynamicImage,
            is_fallback: bool,
        ) -> Result<OcrResult, String> {
            let mut count = self.call_count.lock().unwrap();
            let idx = *count;
            *count += 1;

            if let Some(Some(text)) = self.results.get(idx) {
                Ok(OcrResult {
                    page_index,
                    backend: backend.clone(),
                    text: text.clone(),
                    confidence: 0.9,
                    duration_ms: 10,
                    was_fallback: is_fallback,
                })
            } else {
                Err("simulated failure".into())
            }
        }
    }

    fn default_thresholds() -> ThresholdConfig {
        ThresholdConfig::default()
    }

    /// Helper: create a blank RGB image for testing.
    fn blank_page() -> DynamicImage {
        let img: RgbImage = image::ImageBuffer::new(100, 100);
        DynamicImage::ImageRgb8(img)
    }

    // REQ:ocr-pipeline-01 — Single page produces correct output with page marker
    #[tokio::test]
    async fn single_page_pipeline() {
        let pages = vec![blank_page()];
        let executor = TestExecutor::new(vec![Some("Hello world".into())]);
        let t = default_thresholds();

        let outcome = run_pipeline(&pages, &executor, &t, None).await;

        assert_eq!(outcome.results.len(), 1);
        assert!(outcome.results[0].text.contains("Hello world"));
        assert_eq!(outcome.errors.len(), 0);
    }

    // REQ:ocr-pipeline-02 — Three pages produce correct page markers in order
    #[tokio::test]
    async fn three_page_pipeline_markers() {
        let pages = vec![blank_page(), blank_page(), blank_page()];
        let executor = TestExecutor::new(vec![
            Some("Page one".into()),
            Some("Page two".into()),
            Some("Page three".into()),
        ]);

        let t = default_thresholds();
        let outcome = run_pipeline(&pages, &executor, &t, None).await;

        assert_eq!(outcome.results.len(), 3);
        // Results should be in page order
        assert_eq!(outcome.results[0].page_index, 0);
        assert_eq!(outcome.results[1].page_index, 1);
        assert_eq!(outcome.results[2].page_index, 2);

        // Assemble and check markers
        let assembled = assemble_document(&outcome.results);
        assert!(assembled.contains("--- PAGE 1 ---"));
        assert!(assembled.contains("--- PAGE 2 ---"));
        assert!(assembled.contains("--- PAGE 3 ---"));
        assert!(assembled.contains("Page one"));
        assert!(assembled.contains("Page two"));
        assert!(assembled.contains("Page three"));
    }

    // REQ:ocr-pipeline-03 — Failed page produces error, pipeline continues
    #[tokio::test]
    async fn failed_page_non_fatal() {
        let pages = vec![blank_page(), blank_page()];
        // First call succeeds, second fails
        let executor = TestExecutor::new(vec![Some("Good".into()), None]);

        let t = default_thresholds();
        let outcome = run_pipeline(&pages, &executor, &t, None).await;

        assert_eq!(outcome.results.len(), 1, "only first page should succeed");
        assert_eq!(outcome.errors.len(), 1, "second page should produce error");
        assert!(!outcome.report.passed, "report should not pass with errors");
    }

    // REQ:ocr-pipeline-04 — assemble_document is a pure function
    #[test]
    fn assemble_document_pure() {
        let results = vec![
            OcrResult {
                page_index: 0,
                backend: OcrBackend::Tesseract,
                text: "Alpha".into(),
                confidence: 0.95,
                duration_ms: 10,
                was_fallback: false,
            },
            OcrResult {
                page_index: 1,
                backend: OcrBackend::Tesseract,
                text: "Beta".into(),
                confidence: 0.90,
                duration_ms: 12,
                was_fallback: false,
            },
        ];
        let a = assemble_document(&results);
        let b = assemble_document(&results);
        assert_eq!(a, b);
        assert!(a.contains("--- PAGE 1 ---"));
        assert!(a.contains("--- PAGE 2 ---"));
    }
}
