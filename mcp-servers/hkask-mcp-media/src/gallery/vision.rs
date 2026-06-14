//! Vision LLM wrappers for gallery image analysis.
//!
//! Uses the hKask inference router to call vision-capable LLMs
//! (Llama 3.2 Vision, Qwen2-VL, Gemma 4, etc.) for:
//! - Object detection
//! - Face detection and description
//! - Image captioning
//! - Tag generation
//! - Image classification
//! - Face reference validation
//! - Face matching (same person?)
//!
//! All prompts are backed by Jinja2 templates embedded in templates.rs.

use hkask_inference::InferenceRouter;
use hkask_types::LLMParameters;
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Result of face reference validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceValidationResult {
    /// Whether the image passes all criteria for use as a face reference.
    pub valid: bool,
    /// Number of faces detected.
    pub face_count: u32,
    /// Estimated percentage of image occupied by the face.
    pub face_coverage_pct: u32,
    /// Pose assessment.
    pub pose: String,
    /// Lighting quality.
    pub lighting: String,
    /// Occlusion assessment.
    pub occlusion: String,
    /// Image clarity / focus.
    pub clarity: String,
    /// List of failing criteria with explanations (empty if valid).
    pub issues: Vec<String>,
}

/// Result of comparing two face images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatchResult {
    /// Whether the two faces are the same person.
    pub is_match: bool,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Human-readable reasoning for the decision.
    pub reasoning: String,
}

/// Validate a reference image for use in facial recognition.
///
/// Sends the image to a vision LLM with the `validate_face_ref` template.
/// Returns structured pass/fail with specific reasons for rejection.
///
/// REQ: media-face-validate-01
pub async fn validate_face_reference(
    inference: &Arc<InferenceRouter>,
    template_env: &Environment<'static>,
    image_url: &str,
    vision_model: Option<&str>,
) -> Result<FaceValidationResult, String> {
    let prompt = crate::templates::render(
        template_env,
        "validate_face_ref",
        &std::collections::HashMap::new(),
    )?;

    let params = LLMParameters {
        temperature: 0.1, // Low temperature for consistent, objective assessment
        max_tokens: 512,
        ..Default::default()
    };

    let result = inference
        .generate_vision(&prompt, &[image_url.to_string()], &params, vision_model)
        .await
        .map_err(|e| format!("Vision LLM call failed: {}", e))?;

    let parsed: FaceValidationResult = serde_json::from_str(&result.text).map_err(|e| {
        format!(
            "Failed to parse validation result: {} — raw: {}",
            e,
            &result.text[..200.min(result.text.len())]
        )
    })?;

    Ok(parsed)
}

/// Compare two face images to determine if they show the same person.
///
/// Sends both images to a vision LLM with the `match_faces` template.
/// Image 1 is the reference portrait, Image 2 is the query face.
///
/// REQ: media-face-match-01
pub async fn match_faces(
    inference: &Arc<InferenceRouter>,
    template_env: &Environment<'static>,
    reference_url: &str,
    query_url: &str,
    vision_model: Option<&str>,
) -> Result<FaceMatchResult, String> {
    let prompt = crate::templates::render(
        template_env,
        "match_faces",
        &std::collections::HashMap::new(),
    )?;

    let params = LLMParameters {
        temperature: 0.1,
        max_tokens: 512,
        ..Default::default()
    };

    let result = inference
        .generate_vision(
            &prompt,
            &[reference_url.to_string(), query_url.to_string()],
            &params,
            vision_model,
        )
        .await
        .map_err(|e| format!("Vision LLM call failed: {}", e))?;

    let parsed: FaceMatchResult = serde_json::from_str(&result.text).map_err(|e| {
        format!(
            "Failed to parse match result: {} — raw: {}",
            e,
            &result.text[..200.min(result.text.len())]
        )
    })?;

    Ok(parsed)
}
