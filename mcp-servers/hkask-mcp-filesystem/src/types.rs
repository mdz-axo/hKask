//! Request and response types for hkask-mcp-filesystem tools.
//!
//! Each struct maps to a single tool's MCP parameters (deserialized
//! from JSON by rmcp's `Parameters<T>` wrapper). No response types
//! are needed — tools return serde_json::Value directly.

use schemars::JsonSchema;
use serde::Deserialize;

// ── fs.read ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsReadRequest {
    /// Path to the file to read.
    pub path: String,
    /// 1-based start line (inclusive). If omitted, reads from the beginning.
    pub start_line: Option<u32>,
    /// 1-based end line (inclusive). If omitted, reads to the end.
    pub end_line: Option<u32>,
}

// ── fs.write ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsWriteRequest {
    /// Path to the file to create or overwrite.
    pub path: String,
    /// Full content to write.
    pub content: String,
}

// ── fs.edit ──────────────────────────────────────────────────────────────

/// A single text replacement to apply.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TextEdit {
    /// Exact text to find and replace (first occurrence only).
    pub old_text: String,
    /// Replacement text.
    pub new_text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsEditRequest {
    /// Path to the file to edit.
    pub path: String,
    /// Ordered list of edits to apply. Each edit replaces the first
    /// occurrence of old_text with new_text. Edits that don't match
    /// are skipped (no-op).
    pub edits: Vec<TextEdit>,
}

// ── fs.list ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsListRequest {
    /// Directory path to list.
    pub path: String,
}

// ── fs.search ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsSearchRequest {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Root directory to search in.
    pub path: String,
    /// Maximum directory depth (default 3).
    pub max_depth: Option<u32>,
}

// ── fs.delete ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FsDeleteRequest {
    /// Path to the file or empty directory to delete.
    pub path: String,
}

// ── shell.exec ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellExecRequest {
    /// Shell command to execute (passed to `sh -c`).
    pub command: String,
    /// Working directory for the command. Defaults to current directory.
    pub cwd: Option<String>,
    /// Timeout in milliseconds. Default 30_000 (30s).
    pub timeout_ms: Option<u64>,
    /// Maximum bytes of stdout to return. Default 102_400 (100KB).
    /// Output beyond this limit is truncated and `truncated` is set to true.
    pub max_output_bytes: Option<u64>,
}
