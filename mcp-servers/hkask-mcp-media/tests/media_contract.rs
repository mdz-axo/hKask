//! Contract tests for hkask-mcp-media — model resolution and request types.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: Model resolution functions (pure, no I/O) and request deserialization.

// ── Model resolution tests ─────────────────────────────────────────────────

static TTS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn model_resolution_returns_default_when_env_unset() {
    let _lock = TTS_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("HKASK_MEDIA_TTS_MODEL") };
    let model = hkask_mcp_media::models::tts_model();
    assert!(!model.is_empty(), "default TTS model should not be empty");
}

#[test]
fn model_resolution_respects_env_override() {
    let _lock = TTS_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("HKASK_MEDIA_TTS_MODEL", "test-tts-model") };
    let model = hkask_mcp_media::models::tts_model();
    assert_eq!(model, "test-tts-model");
    unsafe { std::env::remove_var("HKASK_MEDIA_TTS_MODEL") };
}

#[test]
fn stt_model_has_default() {
    unsafe { std::env::remove_var("HKASK_MEDIA_STT_MODEL") };
    let model = hkask_mcp_media::models::stt_model();
    assert!(
        !model.is_empty(),
        "STT model should not be empty, got: {model}"
    );
}

#[test]
fn vision_model_has_default() {
    unsafe { std::env::remove_var("HKASK_MEDIA_VISION_MODEL") };
    let model = hkask_mcp_media::models::vision_model();
    assert!(!model.is_empty(), "vision model should not be empty");
}

#[test]
fn image_gen_model_has_default() {
    unsafe { std::env::remove_var("HKASK_MEDIA_IMAGE_GEN_MODEL") };
    let model = hkask_mcp_media::models::image_gen_model();
    assert!(!model.is_empty(), "image gen model should not be empty");
}

// ── Request type deserialization tests ─────────────────────────────────────

#[test]
fn generate_image_request_parses_valid_json() {
    let json = serde_json::json!({
        "prompt": "A test image",
        "image_size": "1024x1024",
        "num_images": 1
    });
    let req: hkask_mcp_media::types::GenerateImageRequest =
        serde_json::from_value(json).expect("should parse generate image request");
    assert_eq!(req.prompt, "A test image");
    assert_eq!(req.image_size, Some("1024x1024".to_string()));
}

#[test]
fn transform_image_request_parses_valid_json() {
    let json = serde_json::json!({
        "prompt": "Make it darker",
        "image_url": "https://example.com/img.png",
        "strength": 0.5
    });
    let req: hkask_mcp_media::types::TransformImageRequest =
        serde_json::from_value(json).expect("should parse transform request");
    assert_eq!(req.prompt, "Make it darker");
    assert_eq!(req.image_url, "https://example.com/img.png");
}

// ── Constant sanity tests ──────────────────────────────────────────────────

#[test]
fn model_constants_are_non_empty() {
    assert!(!hkask_mcp_media::models::TTS_DEFAULT.is_empty());
    assert!(!hkask_mcp_media::models::STT_DEFAULT.is_empty());
    assert!(!hkask_mcp_media::models::VISION_DEFAULT.is_empty());
    assert!(!hkask_mcp_media::models::IMAGE_GEN_DEFAULT.is_empty());
}
