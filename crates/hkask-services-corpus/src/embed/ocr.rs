//! OCR utilities for scanned PDF fallback.

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_services_core::ServiceError;
use hkask_types::template::LLMParameters;

/// Default OCR model for scanned PDF fallback.
/// Override via settings.json or HKASK_OCR_MODEL env var.
fn ocr_model() -> String {
    hkask_services_core::HkaskSettings::load().ocr_model()
}

/// OCR system prompt — instructs the vision model to extract text faithfully.
const OCR_SYSTEM_PROMPT: &str = "Extract all text from this document image. Output the text exactly as it appears, preserving the document structure and layout as closely as possible. If the document contains tables, preserve them in a readable format. Do not add commentary or description — only the extracted text.";

/// Attempt OCR on PDF bytes using pdftoppm decimation + per-page vision OCR.
///
/// 1. Writes PDF bytes to a temp file.
/// 2. Decimates to per-page PNG images via pdftoppm.
/// 3. OCRs each page via the inference router.
/// 4. Returns concatenated text.
///
/// Falls back to sending raw PDF bytes as base64 if pdftoppm is not installed.
#[must_use = "result must be used"]
pub async fn ocr_pdf_bytes(bytes: &[u8], url: &str) -> Result<String, ServiceError> {
    // P9: CNS span
    tracing::info!(target: "cns.embed", operation = "ocr_pdf_bytes", url = %url, byte_len = bytes.len(), "CNS");

    let ocr_model = std::env::var("HKASK_OCR_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(ocr_model);

    // Try pdftoppm decimation first
    if let Ok(text) = ocr_via_decimation(bytes, &ocr_model).await {
        return Ok(text);
    }

    // Fallback: send raw PDF bytes as base64
    let b64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);

    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);

    let params = LLMParameters {
        temperature: 0.1,
        top_p: 1.0,
        top_k: 1,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        min_p: 0.0,
        typical_p: 0.0,
        max_tokens: 4096,
        seed: None,
        disable_thinking: false,
        adapter: None,
        bypass_fusion: true,
    };

    match router
        .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(&ocr_model))
        .await
    {
        Ok(result) => Ok(result.text),
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                Err(ServiceError::Embed {
                    source: None,
                    message: format!(
                        "OCR model '{}' is not available. Ensure it is configured with a cloud provider prefix (e.g., DI/).\n\nOriginal PDF '{}' (source: {}) could not be text-extracted (likely scanned). Set HKASK_OCR_MODEL to override the default model.",
                        ocr_model, url, ocr_model
                    ),
                })
            } else {
                Err(ServiceError::Embed {
                    source: None,
                    message: format!("OCR inference failed for '{}': {}", url, err_msg),
                })
            }
        }
    }
}

/// Decimate PDF to page images and OCR each page individually.
///
/// Returns concatenated text from all pages, or an error if pdftoppm
/// is unavailable or OCR fails on any page.
async fn ocr_via_decimation(bytes: &[u8], model: &str) -> Result<String, String> {
    // Write bytes to temp PDF file
    let temp_dir = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
    let pdf_path = temp_dir.path().join("input.pdf");
    std::fs::write(&pdf_path, bytes).map_err(|e| format!("write temp PDF: {}", e))?;

    // Decimate via pdftoppm
    let prefix = temp_dir.path().join("page");
    let output = std::process::Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("200")
        .arg(&pdf_path)
        .arg(&prefix)
        .output()
        .map_err(|e| format!("pdftoppm not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    // Collect page images
    let mut page = 1;
    let mut page_images: Vec<(usize, Vec<u8>)> = Vec::new();
    loop {
        let page_path = format!("{}-{}.png", prefix.display(), page);
        let path = std::path::Path::new(&page_path);
        if !path.exists() {
            break;
        }
        let png_bytes = std::fs::read(path).map_err(|e| format!("read page {}: {}", page, e))?;
        page_images.push((page, png_bytes));
        page += 1;
    }

    if page_images.is_empty() {
        return Err("pdftoppm produced no output images".into());
    }

    // OCR each page
    let inf_cfg = InferenceConfig::from_env();
    let router = InferenceRouter::new(inf_cfg);
    let params = LLMParameters {
        temperature: 0.1,
        max_tokens: 4096,
        bypass_fusion: true,
        ..Default::default()
    };

    let mut texts: Vec<String> = Vec::with_capacity(page_images.len());
    for (page_num, png_bytes) in &page_images {
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png_bytes);
        let result = router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64], &params, Some(model))
            .await
            .map_err(|e| format!("OCR failed for page {}: {}", page_num, e))?;
        if !result.text.trim().is_empty() {
            texts.push(result.text);
        }
    }

    Ok(texts.join("\n\n"))
}
