//! Media MCP server fuzz targets.
//!
//! Covers all 35 media request types (gallery, face, image, video, audio, generation).
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_mcp_media::types::*;

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

/// Deserialize arbitrary JSON into all media request types — none may panic.
#[test]
fn fuzz_media_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        // Gallery
        let _ = serde_json::from_str::<GalleryOrganizeRequest>(s);
        let _ = serde_json::from_str::<GallerySearchRequest>(s);
        let _ = serde_json::from_str::<GalleryFindSimilarRequest>(s);
        let _ = serde_json::from_str::<GalleryRefreshRequest>(s);
        let _ = serde_json::from_str::<DescribeImageRequest>(s);
        let _ = serde_json::from_str::<GalleryAnalyzeRequest>(s);
        let _ = serde_json::from_str::<GalleryNameFaceRequest>(s);
        let _ = serde_json::from_str::<GalleryTimelineRequest>(s);
        // Face
        let _ = serde_json::from_str::<FaceValidateRequest>(s);
        let _ = serde_json::from_str::<FaceRegisterRequest>(s);
        let _ = serde_json::from_str::<FaceListRequest>(s);
        let _ = serde_json::from_str::<FaceRemoveRequest>(s);
        // Image
        let _ = serde_json::from_str::<ExtractObjectRequest>(s);
        let _ = serde_json::from_str::<RemoveBackgroundRequest>(s);
        let _ = serde_json::from_str::<ApplyStyleRequest>(s);
        let _ = serde_json::from_str::<CreateCollageRequest>(s);
        // Video
        let _ = serde_json::from_str::<VideoClipRequest>(s);
        let _ = serde_json::from_str::<VideoToGifRequest>(s);
        let _ = serde_json::from_str::<ImageToVideoRequest>(s);
        let _ = serde_json::from_str::<VideoAddCaptionRequest>(s);
        let _ = serde_json::from_str::<VideoRemixRequest>(s);
        let _ = serde_json::from_str::<VideoFromImagesRequest>(s);
        let _ = serde_json::from_str::<VideoConcatRequest>(s);
        let _ = serde_json::from_str::<VideoCaptionRequest>(s);
        let _ = serde_json::from_str::<VideoMemeRequest>(s);
        // Audio
        let _ = serde_json::from_str::<VoiceDesignRequest>(s);
        let _ = serde_json::from_str::<GenerateSpeechRequest>(s);
        let _ = serde_json::from_str::<TranscribeRequest>(s);
        let _ = serde_json::from_str::<AudioCaptureRequest>(s);
        let _ = serde_json::from_str::<RecordAndTranscribeRequest>(s);
        // Generation
        let _ = serde_json::from_str::<GenerateImageRequest>(s);
        let _ = serde_json::from_str::<TransformImageRequest>(s);
        let _ = serde_json::from_str::<UpscaleImageRequest>(s);
        let _ = serde_json::from_str::<GenerateVideoRequest>(s);
    });
}
