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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    if lower.contains("git") || lower.contains("docker") || lower.contains("cargo")
        || lower.contains("npm") || lower.contains("shell") || lower.contains("exec")
        || lower.contains("run") || lower.contains("bash")
    {
        ContextCategory::ShellCommand
    } else if lower.contains("test") || lower.contains("pytest") || lower.contains("spec") {
        ContextCategory::TestOutput
    } else if lower.contains("build") || lower.contains("compile") || lower.contains("make") {
        ContextCategory::BuildOutput
    } else if lower.contains("file") || lower.contains("read") || lower.contains("cat") {
        ContextCategory::FileContents
    } else if lower.contains("chat") || lower.contains("conversation") || lower.contains("message") {
        ContextCategory::ConversationHistory
    } else if lower.contains("json") || lower.contains("api") || lower.contains("query") {
        ContextCategory::StructuredData
    } else if lower.contains("log") || lower.contains("journal") || lower.contains("trace") {
        ContextCategory::LogOutput
    } else {
        ContextCategory::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
