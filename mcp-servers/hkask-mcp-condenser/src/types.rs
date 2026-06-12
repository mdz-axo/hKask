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

/// Context category for compressor algorithm dispatch.
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

/// Output of a compression operation.
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
    /// Health signals — populated when algorithmic behavior is unexpected.
    /// Absent means the compression ran within expected bounds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub health_signals: Vec<CondenserHealthSignal>,
}

/// Signal emitted when a condenser algorithm exhibits unexpected behavior.
/// These are CNS `cns.condenser.*` ν-event candidates — they indicate that
/// the algorithmic performance deviated from expected bounds, not that the
/// compression failed (content is still returned).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CondenserHealthSignal {
    /// Algorithm that produced the signal.
    pub algorithm: String,
    /// Signal type: "negative_compression", "low_signal", "budget_shortfall".
    pub signal_type: String,
    /// Human-readable diagnostic.
    pub detail: String,
    /// Lines that scored zero (only for "low_signal" signals).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero_score_count: Option<usize>,
    /// Budget requested vs. actually filled (only for "budget_shortfall").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_requested: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_filled: Option<usize>,
}

/// Cumulative compression statistics.
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

/// Request for thread summarization via local inference.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ThreadSummaryRequest {
    /// Conversation messages to summarize, as an array of {role, content} objects.
    pub messages: Vec<serde_json::Value>,
    /// The current user query for relevance-weighted summarization.
    pub current_query: String,
    /// Maximum tokens for the summary output (default 500).
    pub max_tokens: Option<u32>,
    /// Override the server's default inference model.
    /// When provided, this model is used instead of the server-configured default.
    /// Useful for IDE installations where the active session model should match.
    pub model: Option<String>,
    /// Override the server's default inference URL.
    /// When provided, requests are sent to this URL instead of the server-configured default.
    pub inference_url: Option<String>,
}

/// Output of a thread summarization.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThreadSummaryOutput {
    pub summary: String,
    pub original_message_count: usize,
    pub summary_tokens_approx: usize,
    pub inference_model: String,
    pub inference_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: CNS-CONDENSER-PROFILE — Profile must parse from lowercase strings with known retention percentages
    #[test]
    fn profile_parsing_known_values() {
        assert_eq!("heavy".parse::<Profile>().unwrap(), Profile::Heavy);
        assert_eq!("normal".parse::<Profile>().unwrap(), Profile::Normal);
        assert_eq!("soft".parse::<Profile>().unwrap(), Profile::Soft);
        assert_eq!("light".parse::<Profile>().unwrap(), Profile::Light);
    }

    // REQ: CNS-CONDENSER-PROFILE — Profile parsing is case-insensitive
    #[test]
    fn profile_parsing_case_insensitive() {
        assert_eq!("HEAVY".parse::<Profile>().unwrap(), Profile::Heavy);
        assert_eq!("Normal".parse::<Profile>().unwrap(), Profile::Normal);
        assert_eq!("SoFt".parse::<Profile>().unwrap(), Profile::Soft);
    }

    // REQ: CNS-CONDENSER-PROFILE — Unknown profile strings produce an error
    #[test]
    fn profile_parsing_unknown_is_error() {
        assert!("extreme".parse::<Profile>().is_err());
        assert!("super_heavy".parse::<Profile>().is_err());
        assert!("".parse::<Profile>().is_err());
    }

    // REQ: CNS-CONDENSER-PROFILE — Each profile has expected retention percentage
    #[test]
    fn profile_retention_pct_bounds() {
        assert!((Profile::Heavy.retention_pct() - 0.10).abs() < 0.001);
        assert!((Profile::Normal.retention_pct() - 0.20).abs() < 0.001);
        assert!((Profile::Soft.retention_pct() - 0.60).abs() < 0.001);
        assert!((Profile::Light.retention_pct() - 0.95).abs() < 0.001);
        for profile in &[
            Profile::Heavy,
            Profile::Normal,
            Profile::Soft,
            Profile::Light,
        ] {
            let pct = profile.retention_pct();
            assert!(
                pct > 0.0 && pct < 1.0,
                "{profile}: retention {pct} out of bounds"
            );
        }
    }

    // REQ: CNS-CONDENSER-PROFILE — Profile max_lines returns expected caps
    #[test]
    fn profile_max_lines() {
        assert_eq!(Profile::Heavy.max_lines(), Some(30));
        assert_eq!(Profile::Normal.max_lines(), Some(80));
        assert_eq!(Profile::Soft.max_lines(), Some(200));
        assert_eq!(Profile::Light.max_lines(), None);
    }

    // REQ: CNS-CONDENSER-PROFILE — Profile Display round-trips through FromStr
    #[test]
    fn profile_display_roundtrip() {
        for original in &[
            Profile::Heavy,
            Profile::Normal,
            Profile::Soft,
            Profile::Light,
        ] {
            let s = original.to_string();
            let parsed: Profile = s.parse().unwrap();
            assert_eq!(parsed, *original);
        }
    }

    // REQ: CNS-CONDENSER-CTX — ContextCategory parses from snake_case labels
    #[test]
    fn context_category_parsing() {
        assert_eq!(
            "shell_command".parse::<ContextCategory>().unwrap(),
            ContextCategory::ShellCommand
        );
        assert_eq!(
            "test_output".parse::<ContextCategory>().unwrap(),
            ContextCategory::TestOutput
        );
        assert_eq!(
            "build_output".parse::<ContextCategory>().unwrap(),
            ContextCategory::BuildOutput
        );
        assert_eq!(
            "file_contents".parse::<ContextCategory>().unwrap(),
            ContextCategory::FileContents
        );
        assert_eq!(
            "conversation_history".parse::<ContextCategory>().unwrap(),
            ContextCategory::ConversationHistory
        );
        assert_eq!(
            "structured_data".parse::<ContextCategory>().unwrap(),
            ContextCategory::StructuredData
        );
        assert_eq!(
            "log_output".parse::<ContextCategory>().unwrap(),
            ContextCategory::LogOutput
        );
    }

    // REQ: CNS-CONDENSER-CTX — Unknown category strings default to Unknown (not error)
    #[test]
    fn context_category_unknown_fallback() {
        assert_eq!(
            "garbage".parse::<ContextCategory>().unwrap(),
            ContextCategory::Unknown
        );
        assert_eq!(
            "".parse::<ContextCategory>().unwrap(),
            ContextCategory::Unknown
        );
    }

    // REQ: CNS-CONDENSER-CTX — ContextCategory labels round-trip through FromStr
    #[test]
    fn context_category_label_roundtrip() {
        let all = [
            ContextCategory::ShellCommand,
            ContextCategory::TestOutput,
            ContextCategory::BuildOutput,
            ContextCategory::FileContents,
            ContextCategory::ConversationHistory,
            ContextCategory::StructuredData,
            ContextCategory::LogOutput,
            ContextCategory::Unknown,
        ];
        for cat in &all {
            let label = cat.label();
            let parsed: ContextCategory = label.parse().unwrap();
            assert_eq!(parsed, *cat, "round-trip failed for {cat:?}");
        }
    }
}
