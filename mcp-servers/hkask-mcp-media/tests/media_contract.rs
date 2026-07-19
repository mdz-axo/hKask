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

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

use hkask_database::sqlite::SqliteDriver;
use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp_media::MediaServer;
use hkask_mcp_media::types::FaceListRequest;
use hkask_storage::GalleryStore;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::{Arc, Mutex};

/// Construct a MediaServer with an in-memory gallery store and no gallery state.
fn test_server() -> MediaServer {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let gallery_store = Arc::new(GalleryStore::from_driver(driver));
    MediaServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        Arc::new(InferenceRouter::new(InferenceConfig::default())),
        Arc::new(Mutex::new(None)), // no gallery state
        gallery_store,
        minijinja::Environment::new(),
        hkask_mcp_media::video::FfmpegRunner::detect(),
        None, // no face analyzer
    )
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

// REQ: gallery_status reports no_gallery when no gallery is organized (P5).
// expect: gallery_status returns status=no_gallery for a fresh server.
#[tokio::test]
async fn gallery_status_reports_no_gallery_via_parameters_seam() {
    let server = test_server();
    let out = server.gallery_status().await;
    let content = parse_content(&out);
    assert_eq!(content["status"], "no_gallery", "got: {out}");
}

// REQ: face_list returns an empty list for a fresh server (P5 Testing Discipline).
// expect: face_list returns count=0 and an empty faces array.
#[tokio::test]
async fn face_list_returns_empty_via_parameters_seam() {
    let server = test_server();
    let req: FaceListRequest = serde_json::from_value(serde_json::json!({"status": null}))
        .expect("deserialize FaceListRequest");
    let out = server.face_list(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 0, "got: {out}");
    assert!(
        content["faces"].is_array(),
        "faces should be an array: {out}"
    );
}

// REQ: face_list with a status filter returns an empty list (P5).
// expect: filtering by 'valid' returns count=0 for a fresh server.
#[tokio::test]
async fn face_list_with_status_filter_returns_empty_via_parameters_seam() {
    let server = test_server();
    let req: FaceListRequest = serde_json::from_value(serde_json::json!({"status": "valid"}))
        .expect("deserialize FaceListRequest");
    let out = server.face_list(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(content["count"], 0, "got: {out}");
}
