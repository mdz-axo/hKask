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

    // REQ: classify_tool maps known tool name substrings to correct categories
    #[test]
    fn classify_tool_shell_variants() {
        assert_eq!(classify_tool("git_status"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("docker_ps"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("npm_install"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("shell_exec"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("bash_run"), ContextCategory::ShellCommand);
    }

    // REQ: classify_tool maps test/build/file/chat/json/log tools; more-specific categories take precedence over ShellCommand
    #[test]
    fn classify_tool_all_categories() {
        assert_eq!(classify_tool("pytest_run"), ContextCategory::TestOutput);
        assert_eq!(classify_tool("build_compile"), ContextCategory::BuildOutput);
        assert_eq!(classify_tool("file_read"), ContextCategory::FileContents);
        assert_eq!(
            classify_tool("chat_conversation"),
            ContextCategory::ConversationHistory
        );
        assert_eq!(classify_tool("json_api"), ContextCategory::StructuredData);
        assert_eq!(classify_tool("log_journal"), ContextCategory::LogOutput);
    }

    // REQ: classify_tool maps unrecognized names to Unknown
    #[test]
    fn classify_tool_unknown_fallback() {
        assert_eq!(classify_tool("custom_tool"), ContextCategory::Unknown);
    }

    // REQ: classify_tool is case-insensitive
    #[test]
    fn classify_tool_case_insensitive() {
        assert_eq!(classify_tool("GIT_STATUS"), ContextCategory::ShellCommand);
        assert_eq!(classify_tool("Docker_Run"), ContextCategory::ShellCommand);
    }

    // REQ: classify_tool splits on _ and - for token matching (Phase 1)
    #[test]
    fn classify_tool_splits_on_separators() {
        assert_eq!(classify_tool("pytest-run"), ContextCategory::TestOutput);
        assert_eq!(classify_tool("build-compile"), ContextCategory::BuildOutput);
        assert_eq!(classify_tool("git-status"), ContextCategory::ShellCommand);
    }

    // REQ: classify_tool Phase 1 matches first token, so "cargo_test" → ShellCommand (cargo first)
    #[test]
    fn classify_tool_first_token_wins() {
        assert_eq!(classify_tool("cargo_test"), ContextCategory::ShellCommand);
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

pub fn classify_tool(tool_name: &str) -> ContextCategory {
    let lower = tool_name.to_lowercase();
    let parts: Vec<&str> = lower.split('_').chain(lower.split('-')).collect();

    // Phase 1: known tool name prefixes — exact, no false positives
    for part in &parts {
        match *part {
            "git" | "docker" | "cargo" | "npm" | "shell" | "bash" | "exec" | "run" => {
                return ContextCategory::ShellCommand;
            }
            "test" | "pytest" | "spec" => return ContextCategory::TestOutput,
            "build" | "compile" | "make" => return ContextCategory::BuildOutput,
            "chat" | "conversation" | "message" => return ContextCategory::ConversationHistory,
            "log" | "journal" | "trace" => return ContextCategory::LogOutput,
            "json" | "api" | "query" => return ContextCategory::StructuredData,
            "file" | "read" | "cat" => return ContextCategory::FileContents,
            _ => {}
        }
    }

    // Phase 2: substring heuristic for compound/unknown tool names
    if lower.contains("git")
        || lower.contains("docker")
        || lower.contains("cargo")
        || lower.contains("npm")
        || lower.contains("shell")
        || lower.contains("exec")
        || lower.contains("run")
        || lower.contains("bash")
    {
        ContextCategory::ShellCommand
    } else if lower.contains("test") || lower.contains("pytest") || lower.contains("spec") {
        ContextCategory::TestOutput
    } else if lower.contains("build") || lower.contains("compile") || lower.contains("make") {
        ContextCategory::BuildOutput
    } else if lower.contains("chat") || lower.contains("conversation") || lower.contains("message")
    {
        ContextCategory::ConversationHistory
    } else if lower.contains("log") || lower.contains("journal") || lower.contains("trace") {
        ContextCategory::LogOutput
    } else if lower.contains("json") || lower.contains("api") || lower.contains("query") {
        ContextCategory::StructuredData
    } else if lower.contains("file") || lower.contains("read") || lower.contains("cat") {
        ContextCategory::FileContents
    } else {
        ContextCategory::Unknown
    }
}

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
