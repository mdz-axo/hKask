//! PDF Decimation — Render PDF pages to images via pdftoppm.
//!
//! Converts a PDF file into per-page `DynamicImage` buffers for the
//! OCR pipeline. Uses `pdftoppm` from poppler-utils as a subprocess.
//! Falls back gracefully if poppler is not installed.
//!
//! Applies contrast stretching to each page image to improve edge
//! detection for complexity scoring and OCR quality on low-contrast scans.
//! Optional fal.ai preprocessing (gated behind FA_API_KEY) can supplement
//! contrast stretching with AI-based enhancement when available.

use hkask_types::ocr::PipelineError;
use image::{DynamicImage, GenericImageView};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Render a PDF to per-page images.
///
/// # Arguments
/// * `pdf_path` — Path to the PDF file.
/// * `dpi` — Render resolution (default: 300). Higher = better OCR quality.
///
/// # Returns
/// Ordered vector of page images, or a `PipelineError` if decimation fails.
///
/// # Preprocessing
/// Each page image is preprocessed for OCR quality:
/// - If `FAL_KEY`/`FA_API_KEY` is set: sends to `fal-ai/docres` for
///   AI-based binarization (deshadow, deblur, clean B&W output).
/// - Otherwise: applies local `stretch_contrast()` (free, O(w·h)).
///
/// # Dependencies
/// Requires `pdftoppm` from poppler-utils. On failure, returns
/// `DecimationFailed` with installation guidance.
pub async fn pdf_to_images(pdf_path: &Path, dpi: u32) -> Result<Vec<DynamicImage>, PipelineError> {
    if !pdf_path.exists() {
        return Err(PipelineError::DecimationFailed(format!(
            "PDF file not found: {}",
            pdf_path.display()
        )));
    }

    // Create temp directory for page images
    let temp_dir = tempfile::tempdir().map_err(|e| {
        PipelineError::DecimationFailed(format!("Failed to create temp directory: {}", e))
    })?;
    let prefix = temp_dir.path().join("page");

    // Invoke pdftoppm
    let output = Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg(dpi.to_string())
        .arg(pdf_path)
        .arg(&prefix)
        .output()
        .map_err(|e| pdftoppm_error(&e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Detect common failure modes
        if stderr.contains("May not be a PDF file") || stderr.contains("Error") {
            return Err(PipelineError::DecimationFailed(format!(
                "PDF may be corrupted or encrypted: {}",
                stderr.trim()
            )));
        }
        return Err(PipelineError::DecimationFailed(format!(
            "pdftoppm failed: {}",
            stderr.trim()
        )));
    }

    // Collect output images in page order
    let mut images: Vec<DynamicImage> = Vec::new();
    let mut page = 1;
    loop {
        let page_path = format!("{}-{}.png", prefix.display(), page);
        let path = PathBuf::from(&page_path);
        if !path.exists() {
            break;
        }
        let mut img = image::open(&path).map_err(|e| {
            PipelineError::DecimationFailed(format!("Failed to load page {} image: {}", page, e))
        })?;
        preprocess_via_fal(&mut img).await;
        images.push(img);
        page += 1;
    }

    if images.is_empty() {
        return Err(PipelineError::DecimationFailed(
            "pdftoppm produced no output images — PDF may be empty or unrenderable".into(),
        ));
    }

    // temp_dir is dropped here, cleaning up page files
    Ok(images)
}

/// Preprocess a page image via fal.ai `docres` for OCR quality improvement.
///
/// Sends the image to `fal-ai/docres` with `binarization` task for clean
/// black/white output optimized for Tesseract and LLM OCR. Uses base64 data
/// URIs — no public URL hosting needed.
///
/// # Cost
/// $0.025/megapixel. At 150 DPI (~2 MP/letter page): ~$0.05/page.
///
/// # Concurrency
/// fal.ai queue-based: requests never rejected, just queued. Default limit
/// 2 concurrent, scales to 40 with credit purchases.
///
/// # Fallback
/// If `FAL_KEY`/`FA_API_KEY` is not set, or the fal.ai call fails for any
/// reason, falls back to local `stretch_contrast()` at zero cost.
pub(crate) async fn preprocess_via_fal(image: &mut DynamicImage) {
    // Check for API key
    let api_key = std::env::var("FAL_KEY")
        .or_else(|_| std::env::var("FA_API_KEY"))
        .unwrap_or_default();

    if api_key.is_empty() {
        tracing::debug!(target: "cns.pipeline.ocr", "FAL_KEY not set, using stretch_contrast");
        stretch_contrast(image);
        return;
    }

    // Encode image as PNG base64 data URI
    let mut png_bytes: Vec<u8> = Vec::new();
    if image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .is_err()
    {
        tracing::warn!(target: "cns.pipeline.ocr", "Failed to encode image for fal.ai, falling back");
        stretch_contrast(image);
        return;
    }

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);
    let data_uri = format!("data:image/png;base64,{}", b64);

    // Build HTTP client (reuse connection pool across calls)
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(target: "cns.pipeline.ocr", error = %e, "Failed to build HTTP client for fal.ai");
            stretch_contrast(image);
            return;
        }
    };

    // POST to fal.run/fal-ai/docres with binarization task
    let request_body = serde_json::json!({
        "image_url": data_uri,
        "task": "binarization",
    });

    let response = match client
        .post("https://fal.run/fal-ai/docres")
        .header("Authorization", format!("Key {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "cns.pipeline.ocr", error = %e, "fal.ai docres request failed, falling back");
            stretch_contrast(image);
            return;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        tracing::warn!(target: "cns.pipeline.ocr", status = %status, error = %error_text, "fal.ai docres returned error, falling back");
        stretch_contrast(image);
        return;
    }

    // Parse response: {"image": {"url": "...", "content_type": "image/png"}}
    let result: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(target: "cns.pipeline.ocr", error = %e, "Failed to parse fal.ai response");
            stretch_contrast(image);
            return;
        }
    };

    let image_url = match result["image"]["url"].as_str() {
        Some(url) => url,
        None => {
            tracing::warn!(target: "cns.pipeline.ocr", "fal.ai response missing image.url, falling back");
            stretch_contrast(image);
            return;
        }
    };

    // Download the enhanced image
    let enhanced_bytes = match client.get(image_url).send().await {
        Ok(r) => match r.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(target: "cns.pipeline.ocr", error = %e, "Failed to download enhanced image");
                stretch_contrast(image);
                return;
            }
        },
        Err(e) => {
            tracing::warn!(target: "cns.pipeline.ocr", error = %e, "Failed to fetch enhanced image URL");
            stretch_contrast(image);
            return;
        }
    };

    // Decode and replace
    match image::load_from_memory(&enhanced_bytes) {
        Ok(enhanced) => {
            tracing::info!(target: "cns.pipeline.ocr", "fal.ai docres binarization applied successfully");
            *image = enhanced;
        }
        Err(e) => {
            tracing::warn!(target: "cns.pipeline.ocr", error = %e, "Failed to decode enhanced image, keeping original");
            stretch_contrast(image);
        }
    }
}

/// Stretch contrast of a page image to full 0–255 range.
///
/// Improves edge detection for complexity scoring and OCR quality
/// on low-contrast scans (e.g., faded text, uneven lighting).
/// Pure function, O(w·h), no new dependencies.
pub(crate) fn stretch_contrast(img: &mut DynamicImage) {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return;
    }

    let gray = img.to_luma8();
    let pixels = gray.as_raw();

    // Find min/max pixel values
    let mut min: u8 = 255;
    let mut max: u8 = 0;
    for &p in pixels.iter() {
        if p < min {
            min = p;
        }
        if p > max {
            max = p;
        }
    }

    // Skip if image is already full-range or uniform
    if max <= min {
        return;
    }

    // Rescale to 0–255
    let range = (max - min) as f32;
    let stretched: Vec<u8> = pixels
        .iter()
        .map(|&p| ((p as f32 - min as f32) / range * 255.0) as u8)
        .collect();

    *img = DynamicImage::ImageLuma8(
        image::ImageBuffer::from_raw(w, h, stretched).expect("stretched buffer matches dimensions"),
    );
}

/// Format a user-friendly error when pdftoppm is not found.
fn pdftoppm_error(detail: &str) -> PipelineError {
    if detail.contains("No such file") || detail.contains("not found") {
        PipelineError::DecimationFailed(
            "pdftoppm is not installed. Install poppler-utils:\n  Ubuntu/Debian: sudo apt install poppler-utils\n  macOS: brew install poppler\n  Fedora: sudo dnf install poppler-utils"
                .into(),
        )
    } else {
        PipelineError::DecimationFailed(format!("Failed to run pdftoppm: {}", detail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check if pdftoppm is available on this system.
    fn pdftoppm_available() -> bool {
        Command::new("pdftoppm")
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Create a minimal valid PDF for testing.
    fn minimal_pdf() -> Vec<u8> {
        // Minimal hand-crafted PDF with one page containing "Hello"
        b"%PDF-1.4\n\
          1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
          2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
          3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n\
          4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
          5 0 obj<</Length 44>>stream\n\
          BT /F1 24 Tf 100 700 Td (Hello) Tj ET\n\
          endstream\n\
          endobj\n\
          xref\n\
          0 6\n\
          0000000000 65535 f \n\
          0000000009 00000 n \n\
          0000000058 00000 n \n\
          0000000115 00000 n \n\
          0000000277 00000 n \n\
          0000000349 00000 n \n\
          trailer<</Size 6/Root 1 0 R>>\n\
          startxref\n\
          441\n\
          %%EOF\n"
            .to_vec()
    }

    // REQ:ocr-decimate-01 — Valid PDF produces images
    #[tokio::test]
    async fn valid_pdf_produces_images() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, minimal_pdf()).unwrap();

        let images = pdf_to_images(&pdf_path, 150).await.unwrap();
        assert_eq!(images.len(), 1, "one-page PDF should produce one image");
        assert!(images[0].width() > 0);
        assert!(images[0].height() > 0);
    }

    // REQ:ocr-decimate-02 — Missing file returns error
    #[tokio::test]
    async fn missing_file_returns_error() {
        let result = pdf_to_images(Path::new("/nonexistent/path.pdf"), 150).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PipelineError::DecimationFailed(_)),
            "expected DecimationFailed, got {:?}",
            err
        );
    }

    // REQ:ocr-decimate-03 — Corrupt PDF returns error
    #[tokio::test]
    async fn corrupt_pdf_returns_error() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("corrupt.pdf");
        std::fs::write(&pdf_path, b"not a pdf file").unwrap();

        let result = pdf_to_images(&pdf_path, 150).await;
        assert!(result.is_err());
    }

    // REQ:ocr-decimate-04 — Contrast stretching expands narrow range to full 0–255
    #[test]
    fn contrast_stretch_expands_range() {
        // Create a low-contrast image: pixels only in range 100–150
        let mut img = DynamicImage::ImageLuma8(image::ImageBuffer::from_fn(100, 100, |x, y| {
            image::Luma([100 + ((x + y) % 51) as u8])
        }));

        stretch_contrast(&mut img);

        // After stretching, should have pixels at 0 and 255
        let gray = img.as_luma8().unwrap();
        let pixels = gray.as_raw();
        let min = pixels.iter().min().copied().unwrap();
        let max = pixels.iter().max().copied().unwrap();
        assert_eq!(min, 0, "min should be 0 after stretch, got {}", min);
        assert_eq!(max, 255, "max should be 255 after stretch, got {}", max);
    }

    // REQ:ocr-decimate-05 — Contrast stretching is idempotent on full-range images
    #[test]
    fn contrast_stretch_idempotent() {
        // Create a full-range image (already 0–255)
        let mut img = DynamicImage::ImageLuma8(image::ImageBuffer::from_fn(100, 100, |x, y| {
            image::Luma([if (x + y) % 2 == 0 { 0 } else { 255 }])
        }));

        let gray_before = img.as_luma8().unwrap();
        let pixels_before = gray_before.as_raw().to_vec();

        stretch_contrast(&mut img);

        // Full-range image should be unchanged
        let gray_after = img.as_luma8().unwrap();
        assert_eq!(gray_after.as_raw(), &pixels_before);
    }
}
