//! PDF Decimation — Render PDF pages to images via pdftoppm.
//!
//! Converts a PDF file into per-page `DynamicImage` buffers for the
//! OCR pipeline. Uses `pdftoppm` from poppler-utils as a subprocess.
//! Falls back gracefully if poppler is not installed.

use hkask_types::ocr::PipelineError;
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
        let img = image::open(&path).map_err(|e| {
            PipelineError::DecimationFailed(format!("Failed to load page {} image: {}", page, e))
        })?;
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
}
