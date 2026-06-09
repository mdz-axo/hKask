//! hKask MCP Condenser — Request and domain types

use hkask_mcp::server::McpToolError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompressRequest {
    pub tool_name: String,
    pub output: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetProfileRequest {
    pub profile: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClassifyRequest {
    pub tool_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PersistRequest {
    /// Tool name that produced the content.
    pub tool_name: String,
    /// Content to persist (compressed output or thread summary).
    pub compressed_output: String,
    /// Optional confidence for the stored triple (0.0–1.0, default 1.0).
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Heavy,
    Normal,
    Soft,
    Light,
}

impl Profile {
    pub fn retention_pct(&self) -> f64 {
        match self {
            Profile::Heavy => 0.10,
            Profile::Normal => 0.20,
            Profile::Soft => 0.60,
            Profile::Light => 0.95,
        }
    }

    pub fn max_lines(&self) -> Option<usize> {
        match self {
            Profile::Heavy => Some(30),
            Profile::Normal => Some(80),
            Profile::Soft => Some(200),
            Profile::Light => None,
        }
    }
}

impl std::str::FromStr for Profile {
    type Err = McpToolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "heavy" => Ok(Profile::Heavy),
            "normal" => Ok(Profile::Normal),
            "soft" => Ok(Profile::Soft),
            "light" => Ok(Profile::Light),
            _ => Err(McpToolError::invalid_argument(format!(
                "Unknown profile '{s}'. Use: heavy, normal, soft, light"
            ))),
        }
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Profile::Heavy => write!(f, "heavy"),
            Profile::Normal => write!(f, "normal"),
            Profile::Soft => write!(f, "soft"),
            Profile::Light => write!(f, "light"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::McpErrorKind;

    // REQ: Profile retention percentages follow the specification (heavy=10%, normal=20%, soft=60%, light=95%)
    #[test]
    fn profile_retention_pct_matches_spec() {
        assert!((Profile::Heavy.retention_pct() - 0.10).abs() < f64::EPSILON);
        assert!((Profile::Normal.retention_pct() - 0.20).abs() < f64::EPSILON);
        assert!((Profile::Soft.retention_pct() - 0.60).abs() < f64::EPSILON);
        assert!((Profile::Light.retention_pct() - 0.95).abs() < f64::EPSILON);
    }

    // REQ: Profile max_lines decreases monotonically: heavy < normal < soft, light is unbounded
    #[test]
    fn profile_max_lines_monotonic_with_light_unbounded() {
        assert!(Profile::Heavy.max_lines() < Profile::Normal.max_lines());
        assert!(Profile::Normal.max_lines() < Profile::Soft.max_lines());
        assert_eq!(Profile::Light.max_lines(), None);
    }

    // REQ: Profile round-trips through FromStr and Display
    #[test]
    fn profile_round_trips_str() {
        for name in ["heavy", "normal", "soft", "light"] {
            let parsed: Profile = name.parse().unwrap();
            assert_eq!(parsed.to_string(), name);
        }
    }

    // REQ: Profile FromStr is case-insensitive
    #[test]
    fn profile_parse_case_insensitive() {
        assert_eq!("Heavy".parse::<Profile>().unwrap(), Profile::Heavy);
        assert_eq!("NORMAL".parse::<Profile>().unwrap(), Profile::Normal);
        assert_eq!("SoFt".parse::<Profile>().unwrap(), Profile::Soft);
    }

    // REQ: Profile FromStr rejects unknown values
    #[test]
    fn profile_parse_rejects_unknown() {
        let err = "turbo".parse::<Profile>().unwrap_err();
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);
    }

    // REQ: ContextCategory label returns snake_case string
    #[test]
    fn context_category_labels() {
        assert_eq!(ContextCategory::ShellCommand.label(), "shell_command");
        assert_eq!(ContextCategory::TestOutput.label(), "test_output");
        assert_eq!(ContextCategory::BuildOutput.label(), "build_output");
        assert_eq!(ContextCategory::FileContents.label(), "file_contents");
        assert_eq!(
            ContextCategory::ConversationHistory.label(),
            "conversation_history"
        );
        assert_eq!(ContextCategory::StructuredData.label(), "structured_data");
        assert_eq!(ContextCategory::LogOutput.label(), "log_output");
        assert_eq!(ContextCategory::Unknown.label(), "unknown");
    }

    // REQ: ContextCategory FromStr round-trips for all variants; unknown strings map to Unknown
    #[test]
    fn context_category_round_trips() {
        for cat in [
            ContextCategory::ShellCommand,
            ContextCategory::TestOutput,
            ContextCategory::BuildOutput,
            ContextCategory::FileContents,
            ContextCategory::ConversationHistory,
            ContextCategory::StructuredData,
            ContextCategory::LogOutput,
        ] {
            let parsed: ContextCategory = cat.label().parse().unwrap();
            assert_eq!(parsed, cat);
        }
    }

    // REQ: ContextCategory FromStr maps unrecognized strings to Unknown
    #[test]
    fn context_category_unknown_fallback() {
        let parsed: ContextCategory = "something_unrecognized".parse().unwrap();
        assert_eq!(parsed, ContextCategory::Unknown);
    }

    // REQ: CondenserStats defaults to normal profile and zero counters
    #[test]
    fn condenser_stats_default() {
        let stats = CondenserStats::default();
        assert_eq!(stats.total_compressions, 0);
        assert_eq!(stats.total_original_bytes, 0);
        assert_eq!(stats.total_compressed_bytes, 0);
        assert_eq!(stats.current_profile, "normal");
        assert!(stats.algorithm_usage.is_empty());
        assert!(stats.category_usage.is_empty());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextCategory {
    ShellCommand,
    TestOutput,
    BuildOutput,
    FileContents,
    ConversationHistory,
    StructuredData,
    LogOutput,
    Unknown,
}

impl ContextCategory {
    pub fn label(&self) -> &str {
        match self {
            ContextCategory::ShellCommand => "shell_command",
            ContextCategory::TestOutput => "test_output",
            ContextCategory::BuildOutput => "build_output",
            ContextCategory::FileContents => "file_contents",
            ContextCategory::ConversationHistory => "conversation_history",
            ContextCategory::StructuredData => "structured_data",
            ContextCategory::LogOutput => "log_output",
            ContextCategory::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for ContextCategory {
    type Err = McpToolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shell_command" => Ok(ContextCategory::ShellCommand),
            "test_output" => Ok(ContextCategory::TestOutput),
            "build_output" => Ok(ContextCategory::BuildOutput),
            "file_contents" => Ok(ContextCategory::FileContents),
            "conversation_history" => Ok(ContextCategory::ConversationHistory),
            "structured_data" => Ok(ContextCategory::StructuredData),
            "log_output" => Ok(ContextCategory::LogOutput),
            _ => Ok(ContextCategory::Unknown),
        }
    }
}

// Moved to algorithms.rs — re-export preserves existing call sites
pub use crate::algorithms::classify_tool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompressedOutput {
    pub content: String,
    pub algorithm: String,
    pub category: String,
    pub profile: String,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub reduction_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CondenserStats {
    pub total_compressions: u64,
    pub total_original_bytes: u64,
    pub total_compressed_bytes: u64,
    pub algorithm_usage: std::collections::HashMap<String, u64>,
    pub category_usage: std::collections::HashMap<String, u64>,
    pub current_profile: String,
}

impl Default for CondenserStats {
    fn default() -> Self {
        Self {
            total_compressions: 0,
            total_original_bytes: 0,
            total_compressed_bytes: 0,
            algorithm_usage: std::collections::HashMap::new(),
            category_usage: std::collections::HashMap::new(),
            current_profile: "normal".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ThreadSummaryRequest {
    /// Conversation messages to summarize, as an array of {role, content} objects.
    pub messages: Vec<serde_json::Value>,
    /// The current user query for relevance-weighted summarization.
    pub current_query: String,
    /// Maximum tokens for the summary output (default 500).
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThreadSummaryOutput {
    pub summary: String,
    pub original_message_count: usize,
    pub summary_tokens_approx: usize,
    pub inference_model: String,
    pub inference_url: String,
}
