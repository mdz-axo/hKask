//! PDF Decimation — Render PDF pages to images via pdftoppm.
//!
//! Converts a PDF file into per-page `DynamicImage` buffers for the
//! OCR pipeline. Uses `pdftoppm` from poppler-utils as a subprocess.
//! Falls back gracefully if poppler is not installed.
//!
//! Applies contrast stretching to each page image to improve edge
//! detection for complexity scoring and OCR quality on low-contrast scans.

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
/// # Dependencies
/// Requires `pdftoppm` from poppler-utils. On failure, returns
/// `DecimationFailed` with installation guidance.
pub fn pdf_to_images(pdf_path: &Path, dpi: u32) -> Result<Vec<DynamicImage>, PipelineError> {
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
        stretch_contrast(&mut img);
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

    let mut gray = img.to_luma8();
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
    #[test]
    fn valid_pdf_produces_images() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("test.pdf");
        std::fs::write(&pdf_path, minimal_pdf()).unwrap();

        let images = pdf_to_images(&pdf_path, 150).unwrap();
        assert_eq!(images.len(), 1, "one-page PDF should produce one image");
        assert!(images[0].width() > 0);
        assert!(images[0].height() > 0);
    }

    // REQ:ocr-decimate-02 — Missing file returns error
    #[test]
    fn missing_file_returns_error() {
        let result = pdf_to_images(Path::new("/nonexistent/path.pdf"), 150);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PipelineError::DecimationFailed(_)),
            "expected DecimationFailed, got {:?}",
            err
        );
    }

    // REQ:ocr-decimate-03 — Corrupt PDF returns error
    #[test]
    fn corrupt_pdf_returns_error() {
        if !pdftoppm_available() {
            eprintln!("SKIP: pdftoppm not installed");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("corrupt.pdf");
        std::fs::write(&pdf_path, b"not a pdf file").unwrap();

        let result = pdf_to_images(&pdf_path, 150);
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
