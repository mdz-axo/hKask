//! Vision LLM Backend — OCR via hkask-inference vision models.
//!
//! Sends page images as base64-encoded PNG to vision-capable LLMs
//! through the inference router. Supports provider-prefixed model names
//! (DI/, FW/, OM/) for backend routing.
//!
//! Includes a circuit breaker for rate-limit resilience: after N consecutive
//! 429 responses, all LLM requests pause for a cooldown period.
use async_trait::async_trait;

use crate::ocr::{OcrBackend, OcrResult};
use base64::Engine;
use hkask_inference::InferenceRouter;
use hkask_types::template::LLMParameters;
use image::DynamicImage;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Instant;

use crate::ocr::pipeline::{OcrError, OcrExecutor};

/// System prompt for OCR extraction — instructs the model to extract text faithfully.
/// Applied to all page types including Kindle book pages.
const OCR_SYSTEM_PROMPT: &str = "\
Extract all readable text from this page image. Output the text verbatim with these rules:\n\
1. Output ONLY the extracted text — no commentary, no markdown, no descriptions.\n2. Preserve paragraph breaks as blank lines between paragraphs.\n3. Preserve chapter headings, section breaks, and dialogue formatting.\n4. IGNORE page numbers, running headers, footers, and reader UI elements.\n5. IGNORE any embedded images, diagrams, or illustrations.\n6. If the page is blank or contains only non-text content, output the word BLANK.\n7. Do not summarize, paraphrase, or edit. Transcribe exactly what you see.\n8. Preserve punctuation, capitalization, and special characters as they appear.";

/// Circuit breaker for rate-limit resilience.
///
/// After `threshold` consecutive failures, pauses all requests until
/// `cooldown_secs` after the last failure. Embedded in `LlmOcrExecutor`.
struct CircuitBreaker {
    /// Consecutive failure count (429 or connection errors).
    failures: AtomicU64,
    /// Unix timestamp (seconds) until which the breaker is open.
    cooldown_until: AtomicI64,
    /// Consecutive failures before opening.
    threshold: u64,
    /// Cooldown duration in seconds.
    cooldown_secs: u64,
}

impl CircuitBreaker {
    const fn new(threshold: u64, cooldown_secs: u64) -> Self {
        Self {
            failures: AtomicU64::new(0),
            cooldown_until: AtomicI64::new(0),
            threshold,
            cooldown_secs,
        }
    }

    /// Check whether requests are allowed. Returns `true` if the circuit is closed.
    fn is_closed(&self) -> bool {
        let now = now_unix();
        let until = self.cooldown_until.load(Ordering::Relaxed);
        now >= until
    }

    /// Record a successful request — resets the failure counter.
    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.cooldown_until.store(0, Ordering::Relaxed);
    }

    /// Record a failure. If the threshold is reached, open the circuit.
    fn record_failure(&self) {
        let count = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.threshold {
            let until = now_unix() + self.cooldown_secs as i64;
            self.cooldown_until.store(until, Ordering::Relaxed);
            tracing::warn!(
                target: "reg.pipeline.ocr.circuit_breaker",
                failures = count,
                cooldown_secs = self.cooldown_secs,
                "Circuit breaker opened — pausing LLM OCR requests"
            );
        }
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Vision LLM OCR executor using the hkask-inference router.
///
/// Encodes page images as base64 PNG and dispatches to vision-capable
/// models via `generate_vision`. Supports all inference backends
/// (DeepInfra, Together AI) through provider-prefixed model names.
///
/// The router is constructed once and shared across all concurrent
/// OCR tasks via `Arc<InferenceRouter>`.
pub struct LlmOcrExecutor {
    /// Shared inference router (constructed once, used by all concurrent tasks).
    router: Arc<InferenceRouter>,
    /// Maximum output tokens per page.
    max_tokens: u32,
    /// Circuit breaker for rate-limit resilience.
    breaker: CircuitBreaker,
}

impl LlmOcrExecutor {
    /// Create a new LLM OCR executor with a shared router.
    pub fn new(router: Arc<InferenceRouter>) -> Self {
        Self {
            router,
            max_tokens: 4096,
            breaker: CircuitBreaker::new(5, 30), // 5 consecutive failures → 30s cooldown
        }
    }

    /// Set maximum output tokens per page.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    #[cfg(test)]
    fn force_open_circuit(&self) {
        for _ in 0..self.breaker.threshold {
            self.breaker.record_failure();
        }
    }
}
#[async_trait]
impl OcrExecutor for LlmOcrExecutor {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        if !matches!(backend, OcrBackend::LlmOcr(_)) {
            return false;
        }
        // Circuit breaker: if open, report as unavailable so the pipeline
        // falls back to Tesseract gracefully without explicit circuit checks.
        if !self.breaker.is_closed() {
            tracing::debug!(
                target: "reg.pipeline.ocr.circuit_breaker",
                "LLM OCR reported unavailable — circuit breaker open"
            );
            return false;
        }
        true
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, OcrError> {
        let model = match backend {
            OcrBackend::LlmOcr(model) => model.clone(),
            other => {
                return Err(OcrError::BackendFailed {
                    backend: format!("{:?}", other),
                    message: "LlmOcrExecutor cannot handle this backend".into(),
                });
            }
        };

        let start = Instant::now();

        // Encode image as base64 JPEG (smaller than PNG, fits 128K token limit)
        let mut img_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut img_bytes),
                image::ImageFormat::Jpeg,
            )
            .map_err(|e| OcrError::BackendFailed {
                backend: "llm_ocr".into(),
                message: format!("Failed to encode page image as JPEG: {e}"),
            })?;

        let b64_data = base64::engine::general_purpose::STANDARD.encode(&img_bytes);

        let params = LLMParameters {
            temperature: 0.1, // Low temperature for faithful extraction
            max_tokens: self.max_tokens,
            ..Default::default()
        };

        let result = self
            .router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(&model))
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                // GAP-4: CNS variety — detect rate-limit backpressure
                if err_str.contains("429")
                    || err_str.contains("rate limit")
                    || err_str.contains("Rate limit")
                {
                    tracing::warn!(
                        target: "reg.pipeline.ocr.rate_limit",
                        model = %model,
                        page_index = page_index,
                        "OCR inference rate-limited — circuit breaker tracking"
                    );
                }
                OcrError::InferenceFailed(err_str)
            });

        match &result {
            Ok(_) => self.breaker.record_success(),
            Err(OcrError::InferenceFailed(err_str)) => {
                if err_str.contains("429")
                    || err_str.contains("rate limit")
                    || err_str.contains("Rate limit")
                    || err_str.contains("timed out")
                    || err_str.contains("connection")
                {
                    self.breaker.record_failure();
                }
            }
            Err(_) => {}
        }

        let result = result?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Compute real confidence via quality heuristic (GAP-1).
        let confidence = ocr_quality_heuristic(&result.text, image.width(), image.height());

        // GAP-4: CNS variety — flag suspiciously low confidence for Curator review
        if confidence < 0.3 && !result.text.trim().is_empty() {
            tracing::warn!(
                target: "reg.pipeline.ocr.low_confidence",
                page_index = page_index,
                confidence = confidence,
                model = %model,
                text_len = result.text.len(),
                "LLM OCR produced low-confidence output — possible hallucination or poor image quality"
            );
        }

        Ok(OcrResult {
            page_index,
            backend: backend.clone(),
            text: result.text,
            confidence,
            duration_ms,
            was_fallback: is_fallback,
        })
    }
}

/// Compute OCR quality confidence from output text characteristics.
///
/// Multi-factor heuristic replacing the previous fixed 0.85 nominal:
/// - Base score (0.25): awarded if output is non-empty
/// - Length ratio (up to 0.40): how well output length matches expected
///   character count from image dimensions (~2000 px/char at 300 DPI)
/// - Lexical quality (up to 0.30): proportion of word tokens that are
///   well-formed (alphabetic, 2-20 chars)
///
/// Capped at 0.95 to acknowledge heuristic uncertainty.
fn ocr_quality_heuristic(text: &str, image_width: u32, image_height: u32) -> f32 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    // Base score for non-empty output
    let base = 0.25;

    // Length ratio: actual character count vs expected from image dimensions
    let pixels = (image_width as f64) * (image_height as f64);
    let expected_chars = (pixels / 2000.0).max(1.0);
    let actual_chars = trimmed.chars().count() as f64;
    let length_ratio = (actual_chars / expected_chars).clamp(0.0, 5.0);

    // Score peaks at ratio ~1.0, penalizes very short or very long output
    let length_score = if length_ratio > 0.05 && length_ratio < 4.0 {
        let distance_from_ideal = (length_ratio - 1.0).abs();
        (0.40 * (1.0 - distance_from_ideal / 3.0)).max(0.0)
    } else {
        0.0
    };

    // Lexical quality: proportion of well-formed word tokens
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let word_count = words.len().max(1);
    let valid_words = words
        .iter()
        .filter(|w| {
            let alpha = w.chars().filter(|c| c.is_alphabetic()).count();
            w.len() >= 2 && w.len() <= 25 && alpha as f64 / w.len().max(1) as f64 > 0.5
        })
        .count();
    let lexical_score = 0.30 * (valid_words as f32 / word_count as f32);

    (base + length_score as f32 + lexical_score).clamp(0.0, 0.95)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_inference::InferenceConfig;
    use image::{ImageBuffer, RgbImage};

    /// Create a simple test image.
    fn test_image() -> DynamicImage {
        let img: RgbImage = ImageBuffer::new(100, 100);
        DynamicImage::ImageRgb8(img)
    }

    fn test_executor() -> LlmOcrExecutor {
        LlmOcrExecutor::new(Arc::new(InferenceRouter::new(InferenceConfig::from_env())))
    }

    #[test]
    fn is_available_for_llm_ocr() {
        let executor = test_executor();
        // Circuit breaker starts closed, so should be available
        assert!(executor.is_available(&OcrBackend::LlmOcr("any-model".into())));
    }

    #[test]
    fn is_available_false_for_other_backends() {
        let executor = test_executor();
        assert!(!executor.is_available(&OcrBackend::Tesseract));
    }

    #[test]
    fn is_available_false_when_circuit_open() {
        let executor = test_executor();
        executor.force_open_circuit();
        assert!(
            !executor.is_available(&OcrBackend::LlmOcr("any-model".into())),
            "LLM OCR should be unavailable when circuit breaker is open"
        );
    }

    #[tokio::test]
    async fn execute_rejects_wrong_backend() {
        let executor = test_executor();
        let image = test_image();
        let result = executor
            .execute(0, &OcrBackend::Tesseract, &image, false)
            .await;
        assert!(result.is_err());
    }
}
