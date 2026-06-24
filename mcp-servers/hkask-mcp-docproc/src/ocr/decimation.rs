//! PDF Decimation — Render PDF pages to images via pdftoppm.
//!
//! Converts a PDF file into per-page `DynamicImage` buffers for the
//! OCR pipeline. Uses `pdftoppm` from poppler-utils as a subprocess.
//! Falls back gracefully if poppler is not installed.
//!
//! Applies Otsu binarization to each page image for clean B&W output
//! optimized for OCR. Optional fal.ai `docres` enhancement available
//! when `HKASK_USE_FAL_DOCRES=true` and `HKASK_FAL_API_KEY` is set.

use crate::ocr::PipelineError;
use image::DynamicImage;
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
/// - Default: local Otsu binarization (O(w·h), instant, free).
/// - Optional: fal.ai `docres` when `HKASK_FAL_API_KEY` is set
///   (falls back to Otsu on any failure).
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
    let temp_dir = tempfile::tempdir()
        .map_err(|e| PipelineError::DecimationFailed(format!("Failed to create temp directory: {}", e)))?;
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
        let mut img = image::open(&path)
            .map_err(|e| PipelineError::DecimationFailed(format!("Failed to load page {} image: {}", page, e)))?;
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

/// Preprocess a page image for OCR quality improvement.
///
/// Default: local Otsu binarization — O(w·h), instant, free.
/// Optional: fal.ai `docres` when `HKASK_USE_FAL_DOCRES=true` AND
/// `HKASK_FAL_API_KEY` is set. ~40s latency — opt-in only.
pub(crate) async fn preprocess_via_fal(image: &mut DynamicImage) {
    // Otsu first — always instant
    otsu_binarize(image);

    // fal.ai docres is opt-in only (explicit env var required due to ~40s latency)
    let use_fal = std::env::var("HKASK_USE_FAL_DOCRES")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !use_fal {
        return;
    }

    let api_key = std::env::var("HKASK_FAL_API_KEY")
        .or_else(|_| std::env::var("FAL_KEY"))
        .or_else(|_| std::env::var("FA_API_KEY"))
        .unwrap_or_default();

    if api_key.is_empty() {
        tracing::warn!(target: "cns.pipeline.ocr", "HKASK_USE_FAL_DOCRES set but no API key found");
        return;
    }

    // Try fal.ai enhancement on top of Otsu-binarized image
    if let Some(enhanced) = try_fal_docres(image, &api_key).await {
        tracing::info!(target: "cns.pipeline.ocr", "fal.ai docres enhancement applied");
        *image = enhanced;
    } else {
        tracing::warn!(target: "cns.pipeline.ocr", "fal.ai docres failed, keeping Otsu result");
    }
}

/// Try fal.ai docres binarization. Returns None on any failure.
async fn try_fal_docres(image: &DynamicImage, api_key: &str) -> Option<DynamicImage> {
    // Encode image as PNG base64 data URI
    let mut png_bytes: Vec<u8> = Vec::new();
    if image
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .is_err()
    {
        return None;
    }

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);
    let data_uri = format!("data:image/png;base64,{}", b64);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .ok()?;

    let request_body = serde_json::json!({
        "image_url": data_uri,
        "task": "binarization",
    });

    let response = client
        .post("https://fal.run/fal-ai/docres")
        .header("Authorization", format!("Key {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let result: serde_json::Value = response.json().await.ok()?;
    let image_url = result["image"]["url"].as_str()?;

    let enhanced_bytes = client.get(image_url).send().await.ok()?.bytes().await.ok()?;
    image::load_from_memory(&enhanced_bytes).ok()
}

/// Otsu binarization — local, instant, free.
///
/// Computes the optimal threshold that minimizes intra-class variance,
/// then applies it to produce a clean black/white image.
/// O(w·h), no allocations beyond the output buffer.
fn otsu_binarize(image: &mut DynamicImage) {
    use imageproc::contrast::threshold;

    // Convert to grayscale for histogram computation
    let gray = image.to_luma8();

    // Compute Otsu threshold from histogram
    let hist = histogram(&gray);
    let otsu_level = otsu_level(&hist);

    // Apply threshold: pixels > otsu_level → 255, else → 0
    let binarized = threshold(&gray, otsu_level as u8, imageproc::contrast::ThresholdType::Binary);
    *image = DynamicImage::ImageLuma8(binarized);
}

/// Build a 256-bin histogram from a grayscale image.
fn histogram(gray: &image::GrayImage) -> [u32; 256] {
    let mut hist = [0u32; 256];
    for &p in gray.as_raw().iter() {
        hist[p as usize] += 1;
    }
    hist
}

/// Otsu's method: find threshold that minimizes intra-class variance.
fn otsu_level(hist: &[u32; 256]) -> u8 {
    let total: u32 = hist.iter().sum();
    if total == 0 {
        return 128; // fallback for empty images
    }

    let mut sum_b: f64 = 0.0;
    let mut w_b: f64 = 0.0;
    let mut max_variance: f64 = 0.0;
    let mut best_threshold: u8 = 0;

    let sum_total: f64 = hist.iter().enumerate().map(|(i, &count)| i as f64 * count as f64).sum();

    for (t, &count_val) in hist.iter().enumerate() {
        let count = count_val as f64;
        w_b += count;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total as f64 - w_b;
        if w_f == 0.0 {
            break;
        }

        sum_b += t as f64 * count;
        let mean_b = sum_b / w_b;
        let mean_f = (sum_total - sum_b) / w_f;

        let variance = w_b * w_f * (mean_b - mean_f).powi(2);
        if variance > max_variance {
            max_variance = variance;
            best_threshold = t as u8;
        }
    }

    best_threshold
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

    #[test]
    fn otsu_binarization_bw_output() {
        // Create a text-like test image (dark text on light background)
        let mut img = DynamicImage::ImageLuma8(image::ImageBuffer::from_fn(400, 100, |x, y| {
            if !(30..=70).contains(&y) || (x / 10 + y / 15) % 3 == 0 {
                image::Luma([240]) // Light background
            } else {
                image::Luma([30]) // Dark "text" pixels
            }
        }));

        otsu_binarize(&mut img);

        // Should produce clean B&W output
        let luma = img.as_luma8().unwrap();
        let pixels = luma.as_raw();
        let unique: std::collections::BTreeSet<u8> = pixels.iter().copied().collect();
        assert!(
            unique.len() <= 2,
            "Otsu should produce ≤2 unique values (B&W), got {}: {:?}",
            unique.len(),
            unique
        );
        assert!(unique.contains(&0), "should contain black pixels");
        assert!(unique.contains(&255), "should contain white pixels");
    }

    #[test]
    fn otsu_uniform_image() {
        // Uniform gray image — Otsu should still produce valid output
        let mut img = DynamicImage::ImageLuma8(image::ImageBuffer::from_pixel(100, 100, image::Luma([128])));
        otsu_binarize(&mut img);
        // Should not panic, output is valid
        assert!(img.as_luma8().is_some());
    }

    #[tokio::test]
    async fn fal_docres_preprocessing_live() {
        // Only run when explicitly opted in (avoids 40s latency in default test suite)
        let use_fal = std::env::var("HKASK_USE_FAL_DOCRES")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        if !use_fal {
            eprintln!("SKIP: HKASK_USE_FAL_DOCRES not set to true");
            return;
        }

        // .env is at workspace root; cargo test runs from crate dir
        let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".env");
        dotenvy::from_filename(&env_path).ok();

        let api_key = std::env::var("HKASK_FAL_API_KEY")
            .or_else(|_| std::env::var("FAL_KEY"))
            .or_else(|_| std::env::var("FA_API_KEY"))
            .unwrap_or_default();

        if api_key.is_empty() {
            eprintln!("SKIP: no fal.ai API key found");
            return;
        }

        // Create a text-like test image
        let img = DynamicImage::ImageLuma8(image::ImageBuffer::from_fn(400, 100, |x, y| {
            if !(30..=70).contains(&y) || (x / 10 + y / 15) % 3 == 0 {
                image::Luma([240])
            } else {
                image::Luma([30])
            }
        }));

        eprintln!(
            "Sending {}x{} to fal.ai docres (binarization)...",
            img.width(),
            img.height()
        );
        let start = std::time::Instant::now();

        let result = try_fal_docres(&img, &api_key).await;

        let elapsed = start.elapsed();
        match result {
            Some(enhanced) => {
                eprintln!(
                    "fal.ai returned {}x{} in {:?}",
                    enhanced.width(),
                    enhanced.height(),
                    elapsed
                );
                if let Some(luma) = enhanced.as_luma8() {
                    let unique: std::collections::BTreeSet<u8> = luma.as_raw().iter().copied().collect();
                    eprintln!("Unique pixel values: {} ({:?})", unique.len(), unique);
                }
            }
            None => {
                eprintln!("fal.ai call failed after {:?}", elapsed);
            }
        }
    }
}
