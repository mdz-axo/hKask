//! Tesseract Backend — Classical OCR via libtesseract subprocess.
//!
//! Invokes the `tesseract` CLI binary with the image written to a temp file.
//! Falls back gracefully if tesseract is not installed.
use async_trait::async_trait;

use hkask_types::ocr::{OcrBackend, OcrResult};
use image::DynamicImage;
use std::process::Command;
use std::time::Instant;

use crate::ocr::pipeline::OcrExecutor;

/// Tesseract OCR executor using the system `tesseract` binary.
///
/// Writes the page image to a temporary PNG, invokes `tesseract`,
/// and reads the output text. Language defaults to English.
pub struct TesseractExecutor {
    /// Tesseract language data (default: "eng").
    language: String,
    /// Page segmentation mode (default: auto-detect = unset).
    psm: Option<u8>,
}

impl TesseractExecutor {
    /// Create a new tesseract executor with default settings.
    pub fn new() -> Self {
        Self {
            language: "eng".to_string(),
            psm: None,
        }
    }

    /// Set the language for OCR (e.g., "eng", "fra", "deu").
    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = lang.to_string();
        self
    }

    /// Set page segmentation mode (1-13, see tesseract docs).
    pub fn with_psm(mut self, psm: u8) -> Self {
        self.psm = Some(psm);
        self
    }
}

impl Default for TesseractExecutor {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl OcrExecutor for TesseractExecutor {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        if *backend != OcrBackend::Tesseract {
            return false;
        }
        Command::new("tesseract")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        if *backend != OcrBackend::Tesseract {
            return Err(format!(
                "TesseractExecutor cannot handle backend {:?}",
                backend
            ));
        }

        let start = Instant::now();

        // Write image to temp file
        let temp_dir = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
        let input_path = temp_dir.path().join("page.png");
        let output_base = temp_dir.path().join("output");

        image
            .save(&input_path)
            .map_err(|e| format!("Failed to save page image: {}", e))?;

        // Build tesseract command
        let mut cmd = Command::new("tesseract");
        cmd.arg(&input_path)
            .arg(&output_base)
            .arg("-l")
            .arg(&self.language);

        if let Some(psm) = self.psm {
            cmd.arg("--psm").arg(psm.to_string());
        }

        let output = cmd.output().map_err(|e| tesseract_error(&e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("tesseract failed: {}", stderr.trim()));
        }

        // Read output text (tesseract appends .txt to output base)
        let txt_path = output_base.with_extension("txt");
        let text = std::fs::read_to_string(&txt_path)
            .map_err(|e| format!("Failed to read tesseract output: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Tesseract doesn't provide per-page confidence in CLI mode;
        // we report a nominal value based on non-empty output.
        let confidence = if text.trim().is_empty() { 0.0 } else { 0.85 };

        Ok(OcrResult {
            page_index,
            backend: backend.clone(),
            text,
            confidence,
            duration_ms,
            was_fallback: is_fallback,
        })
    }
}

/// Format a user-friendly error when tesseract is not found.
fn tesseract_error(detail: &str) -> String {
    if detail.contains("No such file") || detail.contains("not found") {
        "tesseract is not installed. Install tesseract-ocr:\n  Ubuntu/Debian: sudo apt install tesseract-ocr tesseract-ocr-eng\n  macOS: brew install tesseract\n  Fedora: sudo dnf install tesseract".into()
    } else {
        format!("Failed to run tesseract: {}", detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};

    /// Check if tesseract is available on this system.
    fn tesseract_available() -> bool {
        Command::new("tesseract")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Create a simple test image with clear text-like content.
    fn text_image() -> DynamicImage {
        let mut img: RgbImage = ImageBuffer::new(600, 120);
        // White background
        for y in 0..120 {
            for x in 0..600 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        // Draw "HELLO" in large block letters at ~72pt equivalent
        // Each letter is ~80px wide, 80px tall, with 3px stroke
        let black = Rgb([0, 0, 0]);
        // H: two vertical bars + horizontal bar
        for y in 20..100 {
            img.put_pixel(30, y, black);
            img.put_pixel(31, y, black);
            img.put_pixel(32, y, black);
            img.put_pixel(100, y, black);
            img.put_pixel(101, y, black);
            img.put_pixel(102, y, black);
        }
        for x in 30..103 {
            for dy in 57..63 {
                img.put_pixel(x, dy, black);
            }
        }
        // E: vertical bar + three horizontals
        for y in 20..100 {
            img.put_pixel(130, y, black);
            img.put_pixel(131, y, black);
            img.put_pixel(132, y, black);
        }
        for y in [20, 21, 22, 57, 58, 59, 60, 61, 62, 97, 98, 99] {
            for x in 133..195 {
                img.put_pixel(x, y, black);
            }
        }
        // L: vertical bar + bottom horizontal
        for y in 20..100 {
            img.put_pixel(220, y, black);
            img.put_pixel(221, y, black);
            img.put_pixel(222, y, black);
        }
        for y in 97..100 {
            for x in 223..290 {
                img.put_pixel(x, y, black);
            }
        }
        // L (second): same
        for y in 20..100 {
            img.put_pixel(310, y, black);
            img.put_pixel(311, y, black);
            img.put_pixel(312, y, black);
        }
        for y in 97..100 {
            for x in 313..380 {
                img.put_pixel(x, y, black);
            }
        }
        // O: oval
        for y in 20..100 {
            for x in 410..480 {
                let dx = (x as i32 - 445).abs();
                let dy = (y as i32 - 60).abs();
                // Rough oval: distance from center
                let dist = (dx * dx + dy * dy * 2) as f32;
                if dist > 1200.0 && dist < 1600.0 {
                    img.put_pixel(x, y, black);
                }
            }
        }
        DynamicImage::ImageRgb8(img)
    }

    // contract: ocr-tesseract-01
    #[test]
    fn is_available_when_installed() {
        let executor = TesseractExecutor::new();
        let available = executor.is_available(&OcrBackend::Tesseract);
        assert_eq!(available, tesseract_available());
    }

    // contract: ocr-tesseract-02
    #[test]
    fn is_available_false_for_other_backends() {
        let executor = TesseractExecutor::new();
        assert!(!executor.is_available(&OcrBackend::LlmOcr("lighton".into())));
    }

    // contract: ocr-tesseract-03
    #[tokio::test]
    async fn execute_produces_text() {
        if !tesseract_available() {
            eprintln!("SKIP: tesseract not installed");
            return;
        }

        let executor = TesseractExecutor::new();
        let image = text_image();
        let result = executor
            .execute(0, &OcrBackend::Tesseract, &image, false)
            .await;

        assert!(
            result.is_ok(),
            "tesseract should succeed: {:?}",
            result.err()
        );
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.page_index, 0);
        assert!(!ocr_result.text.trim().is_empty(), "should produce text");
        assert!(ocr_result.duration_ms > 0);
    }

    // contract: ocr-tesseract-04
    #[tokio::test]
    async fn execute_rejects_wrong_backend() {
        let executor = TesseractExecutor::new();
        let image = text_image();
        let result = executor
            .execute(0, &OcrBackend::LlmOcr("lighton".into()), &image, false)
            .await;
        assert!(result.is_err());
    }
}
