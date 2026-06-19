//! Communication MCP server fuzz targets.
//!
//! Covers all 8 communication request types:
//!   tts_speak / tts_generate / list_voices / send_message / create_thread
//!   invite_agent / monitor_thread / tag_agent
//!   (list_threads has no params — skipped)
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_mcp_communication::types::*;

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

/// Deserialize arbitrary JSON into all communication request types — none may panic.
#[test]
fn fuzz_communication_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<TtsSpeakRequest>(s);
        let _ = serde_json::from_str::<TtsGenerateRequest>(s);
        let _ = serde_json::from_str::<ListVoicesRequest>(s);
        let _ = serde_json::from_str::<SendMessageRequest>(s);
        let _ = serde_json::from_str::<CreateThreadRequest>(s);
        let _ = serde_json::from_str::<InviteAgentRequest>(s);
        let _ = serde_json::from_str::<MonitorThreadRequest>(s);
        let _ = serde_json::from_str::<TagAgentRequest>(s);
    });
}
