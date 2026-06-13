//! Integration tests for the OCR pipeline with real inference backends.
//!
//! These tests require external services and are ignored by default.
//! Run with: `cargo test --package hkask-mcp-docproc --test integration -- --ignored`
//!
//! Prerequisites:
//! - Ollama running at http://127.0.0.1:11434
//! - A vision-capable model pulled (e.g., `ollama pull minicpm-v:8b`)
//! - `pdftoppm` from poppler-utils installed (for PDF decimation tests)
//! - `tesseract` installed (for native Tesseract tests)

use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp_docproc::ocr::decimation;
use hkask_mcp_docproc::ocr::pipeline::{self, OcrExecutor};
use hkask_types::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use image::DynamicImage;

fn ollama_available() -> bool {
    std::process::Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "http://127.0.0.1:11434/api/tags",
        ])
        .output()
        .map(|o| {
            let code = String::from_utf8_lossy(&o.stdout);
            code.trim() == "200"
        })
        .unwrap_or(false)
}

fn pdftoppm_available() -> bool {
    std::process::Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn tesseract_available() -> bool {
    std::process::Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

struct RealExecutor {
    router: InferenceRouter,
    ocr_model: Option<String>,
}

impl RealExecutor {
    fn new(config: InferenceConfig, ocr_model: Option<String>) -> Self {
        Self {
            router: InferenceRouter::new(config),
            ocr_model,
        }
    }
}

#[async_trait::async_trait]
impl OcrExecutor for RealExecutor {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        match backend {
            OcrBackend::Tesseract => tesseract_available(),
            OcrBackend::LlmOcr(_) => self.ocr_model.is_some(),
        }
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        let start = std::time::Instant::now();

        if matches!(backend, OcrBackend::Tesseract) && tesseract_available() {
            // Native Tesseract
            let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
            let input_path = dir.path().join(format!("page_{}.png", page_index));
            let output_prefix = dir.path().join(format!("page_{}", page_index));

            image
                .save(&input_path)
                .map_err(|e| format!("save: {}", e))?;

            let output = std::process::Command::new("tesseract")
                .arg(&input_path)
                .arg(&output_prefix)
                .arg("-l")
                .arg("eng")
                .arg("--psm")
                .arg("3")
                .output()
                .map_err(|e| format!("tesseract: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Tesseract: {}", stderr.trim()));
            }

            let txt_path = output_prefix.with_extension("txt");
            let text = std::fs::read_to_string(&txt_path).map_err(|e| format!("read: {}", e))?;

            let duration_ms = start.elapsed().as_millis() as u64;
            let wc = text.split_whitespace().count();
            return Ok(OcrResult {
                page_index,
                backend: backend.clone(),
                text,
                confidence: if wc > 0 { 0.80 } else { 0.1 },
                duration_ms,
                was_fallback: is_fallback,
            });
        }

        // LLM OCR
        let model = match backend {
            OcrBackend::LlmOcr(m) => m.clone(),
            _ => self
                .ocr_model
                .clone()
                .ok_or_else(|| "No OCR model configured".to_string())?,
        };

        let mut png_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )
            .map_err(|e| format!("PNG encode: {}", e))?;

        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes);

        let result = self
            .router
            .generate_vision(
                "Extract all text from this image. Output only the extracted text, nothing else.",
                &[b64],
                &hkask_types::LLMParameters {
                    temperature: 0.1,
                    max_tokens: 8192,
                    ..Default::default()
                },
                Some(&model),
            )
            .await
            .map_err(|e| format!("inference: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let wc = result.text.split_whitespace().count();
        Ok(OcrResult {
            page_index,
            backend: backend.clone(),
            text: result.text,
            confidence: if wc > 0 { 0.85 } else { 0.1 },
            duration_ms,
            was_fallback: is_fallback,
        })
    }
}

// ── Helper ─────────────────────────────────────────────────────────────────

fn text_like_image() -> DynamicImage {
    let mut img = image::RgbImage::new(400, 100);
    for y in 0..100 {
        for x in 0..400 {
            img.put_pixel(x, y, image::Rgb([255, 255, 255]));
        }
    }
    for y in 30..50 {
        for x in 20..380 {
            if (x / 10 + y / 15) % 3 != 0 {
                img.put_pixel(x, y, image::Rgb([0, 0, 0]));
            }
        }
    }
    DynamicImage::ImageRgb8(img)
}

fn minimal_pdf() -> Vec<u8> {
    b"%PDF-1.4\n\
      1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
      2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
      3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n\
      4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
      5 0 obj<</Length 44>>stream\n\
      BT /F1 24 Tf 100 700 Td (Hello World) Tj ET\n\
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

// ── Tests ──────────────────────────────────────────────────────────────────

// REQ:ocr-integration-01 — Pipeline with real LLM OCR
#[tokio::test]
async fn test_pipeline_with_llm_ocr() {
    if !ollama_available() {
        eprintln!("SKIP: Ollama not reachable");
        return;
    }

    let config = InferenceConfig::from_env();
    let executor = RealExecutor::new(config.clone(), Some("maternion/LightOnOCR-2:1b".into()));
    let thresholds = ThresholdConfig::default();
    let embedding_router = EmbeddingRouter::new(config);

    let outcome = pipeline::run_pipeline(
        vec![text_like_image()],
        1,
        &executor,
        &thresholds,
        Some("maternion/LightOnOCR-2:1b"),
        Some((&embedding_router, "DI/Qwen/Qwen3-Embedding-0.6B")),
    )
    .await;

    eprintln!("LLM result: {:?}", outcome.results.first().map(|r| &r.text));
    eprintln!(
        "Verification: passed={}, errors={}",
        outcome.report.passed,
        outcome.errors.len()
    );
    // Pipeline should complete without errors (text may be empty for synthetic images)
    assert!(
        !outcome.results.is_empty(),
        "pipeline should produce results"
    );
    assert!(
        outcome.results[0].confidence > 0.0,
        "confidence should be > 0"
    );
}

// REQ:ocr-integration-02 — Pipeline with Tesseract
#[tokio::test]
async fn test_pipeline_with_tesseract() {
    if !tesseract_available() {
        eprintln!("SKIP: tesseract not installed");
        return;
    }

    let config = InferenceConfig::from_env();
    let executor = RealExecutor::new(config, None);
    let thresholds = ThresholdConfig::default();

    let outcome = pipeline::run_pipeline(
        vec![text_like_image()],
        1,
        &executor,
        &thresholds,
        None,
        None,
    )
    .await;

    eprintln!(
        "Tesseract result: {:?}",
        outcome.results.first().map(|r| &r.text)
    );
    assert!(!outcome.results.is_empty());
}

// REQ:ocr-integration-03 — PDF decimation + pipeline
#[tokio::test]
async fn test_pdf_pipeline() {
    if !ollama_available() || !pdftoppm_available() {
        eprintln!("SKIP: Ollama or pdftoppm not available");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let pdf_path = dir.path().join("test.pdf");
    std::fs::write(&pdf_path, minimal_pdf()).unwrap();

    let pages = decimation::pdf_to_images(&pdf_path, 150)
        .await
        .expect("decimate");
    eprintln!("Decimated {} pages", pages.len());

    let config = InferenceConfig::from_env();
    let executor = RealExecutor::new(config, Some("maternion/LightOnOCR-2:1b".into()));
    let thresholds = ThresholdConfig::default();

    let outcome = pipeline::run_pipeline(
        pages,
        1,
        &executor,
        &thresholds,
        Some("maternion/LightOnOCR-2:1b"),
        None,
    )
    .await;

    eprintln!("PDF result: {:?}", outcome.results.first().map(|r| &r.text));
    assert!(!outcome.results.is_empty());
    eprintln!("Verification passed: {}", outcome.report.passed);
}
