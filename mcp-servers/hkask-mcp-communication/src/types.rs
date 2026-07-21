//! Request types for hkask-mcp-communication MCP tools.
//!
//! Extracted from main.rs — these are the tool input structs that derive
//! Deserialize + JsonSchema for MCP parameter deserialization.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TtsSpeakRequest {
    pub text: String,
    #[serde(default = "default_espeak_voice")]
    pub voice: String,
}

fn default_espeak_voice() -> String {
    "default".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TtsGenerateRequest {
    pub text: String,
    #[serde(default = "default_espeak_voice")]
    pub voice: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListVoicesRequest {
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageRequest {
    pub room_id: String,
    pub body: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateThreadRequest {
    pub title: String,
    pub topic: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InviteAgentRequest {
    pub room_id: String,
    pub userpod_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MonitorThreadRequest {
    pub room_id: String,
    pub userpod_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagAgentRequest {
    pub room_id: String,
    pub userpod_id: String,
    pub body: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UploadFileRequest {
    pub filename: String,
    pub mime_type: String,
    /// Base64-encoded file content
    pub data_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendFileRequest {
    pub room_id: String,
    pub filename: String,
    pub mime_type: String,
    /// Base64-encoded file content
    pub data_base64: String,
    pub caption: Option<String>,
}
