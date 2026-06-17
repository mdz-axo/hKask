//! Vision LLM Backend — OCR via hkask-inference vision models.
//!
//! Sends page images as base64-encoded PNG to vision-capable LLMs
//! through the inference router. Supports provider-prefixed model names
//! (DI/, FW/, OM/) for backend routing.

use base64::Engine;
use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_types::ocr::{OcrBackend, OcrResult};
use hkask_types::template::LLMParameters;
use image::DynamicImage;
use std::time::Instant;

use crate::ocr::pipeline::OcrExecutor;

/// System prompt for OCR extraction — instructs the model to extract text faithfully.
const OCR_SYSTEM_PROMPT: &str = "Extract all text from this document image. Output the text exactly as it appears, preserving the document structure and layout as closely as possible. If the document contains tables, preserve them in a readable format. Do not add commentary or description — only the extracted text.";

/// Vision LLM OCR executor using the hkask-inference router.
///
/// Encodes page images as base64 PNG and dispatches to vision-capable
/// models via `generate_vision`. Supports all inference backends
/// (Ollama, Fireworks, DeepInfra) through provider-prefixed model names.
pub struct LlmOcrExecutor {
    /// Inference configuration for router construction.
    config: InferenceConfig,
    /// Maximum output tokens per page.
    max_tokens: u32,
}

impl LlmOcrExecutor {
    /// Create a new LLM OCR executor.
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            max_tokens: 4096,
        }
    }

    /// Set maximum output tokens per page.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}
impl OcrExecutor for LlmOcrExecutor {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        matches!(backend, OcrBackend::LlmOcr(_))
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        let model = match backend {
            OcrBackend::LlmOcr(model) => model.clone(),
            other => {
                return Err(format!("LlmOcrExecutor cannot handle backend {:?}", other));
            }
        };

        let start = Instant::now();

        // Encode image as base64 PNG
        let mut png_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )
            .map_err(|e| format!("Failed to encode page image as PNG: {}", e))?;

        let b64_data = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        // Build router and dispatch
        let router = InferenceRouter::new(self.config.clone());

        let params = LLMParameters {
            temperature: 0.1, // Low temperature for faithful extraction
            max_tokens: self.max_tokens,
            ..Default::default()
        };

        let result = router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(&model))
            .await
            .map_err(|e| format!("OCR inference failed: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Confidence heuristic: non-empty output = nominal confidence
        let confidence = if result.text.trim().is_empty() {
            0.0
        } else {
            0.85
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, RgbImage};

    /// Create a simple test image.
    fn test_image() -> DynamicImage {
        let img: RgbImage = ImageBuffer::new(100, 100);
        DynamicImage::ImageRgb8(img)
    }

    // REQ:ocr-llm-01 — is_available returns true for LlmOcr backends
    #[test]
    fn is_available_for_llm_ocr() {
        let executor = LlmOcrExecutor::new(InferenceConfig::from_env());
        assert!(executor.is_available(&OcrBackend::LlmOcr("any-model".into())));
    }

    // REQ:ocr-llm-02 — is_available returns false for non-LlmOcr backends
    #[test]
    fn is_available_false_for_other_backends() {
        let executor = LlmOcrExecutor::new(InferenceConfig::from_env());
        assert!(!executor.is_available(&OcrBackend::Tesseract));
    }

    // REQ:ocr-llm-03 — execute rejects wrong backend type
    #[tokio::test]
    async fn execute_rejects_wrong_backend() {
        let executor = LlmOcrExecutor::new(InferenceConfig::from_env());
        let image = test_image();
        let result = executor
            .execute(0, &OcrBackend::Tesseract, &image, false)
            .await;
        assert!(result.is_err());
    }
}
