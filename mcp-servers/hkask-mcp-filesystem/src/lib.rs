//! hkask-mcp-filesystem — MCP server providing OCAP-governed filesystem
//! and shell access to AI agents.
//!
//! Tools (7):
//!   fs.read    — Read file contents with optional line ranges + stats
//!   fs.write   — Create or overwrite a file
//!   fs.edit    — Apply targeted text replacements
//!   fs.list    — List directory contents
//!   fs.search  — Regex search across files
//!   fs.delete  — Delete a file or empty directory
//!   shell.exec — Execute a shell command with timeout + output guard
//!
//! # Security
//!
//! All file I/O is sandboxed to the project root. Paths are canonicalized and
//! verified against the root before any read or write. Path traversal (`../`)
//! is rejected at the sandbox boundary.
//!
//! # CNS Spans
//!
//! All spans use `CnsSpan::Tool { subsystem: ToolSubsystem::Filesystem }`.
//! Operations: `file.read`, `file.written`, `file.deleted`,
//! `command.completed`, `command.failed`, `path.rejected`.

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

pub mod types;
use types::*;

use hkask_mcp::server::{CapabilityTier, McpToolError, execute_tool};
use hkask_types::cns::{CnsSpan, ToolSubsystem};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ── Server ───────────────────────────────────────────────────────────────

hkask_mcp::mcp_server!(
    pub struct FileSystemServer {
        pub project_root: PathBuf,
        pub capability_tier: CapabilityTier,
    }
);

impl FileSystemServer {
    /// Emit a CNS span for a filesystem operation.
    fn emit_cns(&self, operation: &str) {
        CnsSpan::Tool {
            subsystem: ToolSubsystem::Filesystem,
        }
        .emit(operation);
    }

    /// Validate that `raw_path` is within `self.project_root` after canonicalization.
    pub fn sandbox_path(&self, raw_path: &str) -> Result<PathBuf, McpToolError> {
        if raw_path.is_empty() {
            return Err(McpToolError::invalid_argument("path must not be empty"));
        }
        let candidate = Path::new(raw_path);
        let resolved = if candidate.is_relative() {
            self.project_root.join(candidate)
        } else {
            candidate.to_path_buf()
        };
        let canonical_root = self
            .project_root
            .canonicalize()
            .unwrap_or_else(|_| self.project_root.clone());

        // Fast path: the target exists — canonicalize the full path so symlinks are
        // resolved and containment is checked against the canonical root.
        if let Ok(canonical) = resolved.canonicalize() {
            if !canonical.starts_with(&canonical_root) {
                self.emit_cns("path.rejected");
                return Err(McpToolError::invalid_argument(format!(
                    "Path '{raw_path}' is outside the project root '{}'",
                    self.project_root.display()
                )));
            }
            return Ok(canonical);
        }

        // Slow path: the target does not exist yet (e.g. fs_write creating a new
        // file). Canonicalizing the full path would fail with ENOENT, so resolve
        // the longest existing ancestor, verify it is within the sandbox, then
        // re-append the non-existent tail. This lets callers create new files and
        // directories inside the project root while still rejecting traversal.
        let mut ancestor = resolved.clone();
        while !ancestor.exists() {
            match ancestor.parent() {
                Some(parent) if parent != ancestor => ancestor = parent.to_path_buf(),
                _ => break,
            }
        }
        let canonical_ancestor = ancestor.canonicalize().map_err(|e| {
            McpToolError::invalid_argument(format!("Cannot resolve path '{raw_path}': {e}"))
        })?;
        if !canonical_ancestor.starts_with(&canonical_root) {
            self.emit_cns("path.rejected");
            return Err(McpToolError::invalid_argument(format!(
                "Path '{raw_path}' is outside the project root '{}'",
                self.project_root.display()
            )));
        }
        // Lexical suffix relative to the existing ancestor (strip_prefix works on
        // non-existent paths). Joined onto the canonical ancestor, the result is
        // an absolute path inside the sandbox.
        let suffix = resolved
            .strip_prefix(&ancestor)
            .map_err(|e| McpToolError::internal(format!("path strip failed: {e}")))?;
        Ok(canonical_ancestor.join(suffix))
    }
}

/// Truncate `s` to at most `max_bytes` without splitting a UTF-8 codepoint.
/// Walks back from `max_bytes` to the nearest char boundary so slicing never
/// panics on a multibyte character.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut idx = max_bytes;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl FileSystemServer {
    #[tool(
        description = "Read a file's contents. Use start_line/end_line for targeted reads. Returns content, line count, file size, and modification time."
    )]
    pub async fn fs_read(&self, Parameters(req): Parameters<FsReadRequest>) -> String {
        execute_tool(self, "fs.read", async {
            let sandboxed = self.sandbox_path(&req.path)?;
            let path_str = sandboxed.to_string_lossy().to_string();

            let meta = tokio::fs::metadata(&sandboxed).await.map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot access {}: {e}", req.path))
            })?;

            let content = tokio::fs::read_to_string(&sandboxed).await.map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot read {}: {e}", req.path))
            })?;

            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();

            // Normalize the 1-based inclusive range. Each bound is meaningful on
            // its own: start only → from start to end; end only → from beginning
            // to end; both → start..end; neither → full content.
            let (start_idx, end_idx, range) = match (req.start_line, req.end_line) {
                (Some(s), Some(e)) => {
                    if s == 0 || e < s {
                        return Err(McpToolError::invalid_argument(format!(
                            "Invalid line range {s}-{e}: start_line must be >= 1 and end_line >= start_line"
                        )));
                    }
                    let start = (s.saturating_sub(1) as usize).min(total_lines);
                    let end = (e as usize).min(total_lines);
                    (start, end, Some(format!("{s}-{e}")))
                }
                (Some(s), None) => {
                    if s == 0 {
                        return Err(McpToolError::invalid_argument(
                            "Invalid line range: start_line must be >= 1".to_string(),
                        ));
                    }
                    let start = (s.saturating_sub(1) as usize).min(total_lines);
                    (start, total_lines, Some(format!("{s}-")))
                }
                (None, Some(e)) => {
                    if e == 0 {
                        return Err(McpToolError::invalid_argument(
                            "Invalid line range: end_line must be >= 1".to_string(),
                        ));
                    }
                    let end = (e as usize).min(total_lines);
                    (0, end, Some(format!("-{e}")))
                }
                (None, None) => (0, total_lines, None),
            };

            let output = if start_idx == 0 && end_idx == total_lines {
                content
            } else {
                lines[start_idx..end_idx].join("\n")
            };

            let modified = meta.modified().ok().map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            });

            self.emit_cns("file.read");
            Ok(serde_json::json!({
                "content": output,
                "path": path_str,
                "total_lines": total_lines,
                "size_bytes": meta.len(),
                "modified": modified,
                "range": range,
            }))
        })
        .await
    }

    #[tool(description = "Create or overwrite a file. Creates parent directories if needed.")]
    pub async fn fs_write(&self, Parameters(req): Parameters<FsWriteRequest>) -> String {
        execute_tool(self, "fs.write", async {
            let sandboxed = self.sandbox_path(&req.path)?;
            let path_str = sandboxed.to_string_lossy().to_string();

            if let Some(parent) = sandboxed.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| McpToolError::internal(format!("Cannot create directory: {e}")))?;
            }

            tokio::fs::write(&sandboxed, &req.content)
                .await
                .map_err(|e| McpToolError::internal(format!("Cannot write {}: {e}", req.path)))?;

            self.emit_cns("file.written");
            Ok(serde_json::json!({
                "written": true,
                "path": path_str,
                "bytes": req.content.len(),
            }))
        })
        .await
    }

    #[tool(
        description = "Apply targeted text replacements to a file. Edits apply sequentially: each replaces the first occurrence of old_text in the current content, so later edits see earlier edits' output (chaining). Repeat an edit to replace successive occurrences of the same text. Returns count of applied edits."
    )]
    pub async fn fs_edit(&self, Parameters(req): Parameters<FsEditRequest>) -> String {
        execute_tool(self, "fs.edit", async {
            let sandboxed = self.sandbox_path(&req.path)?;
            let path_str = sandboxed.to_string_lossy().to_string();

            let mut content = tokio::fs::read_to_string(&sandboxed).await.map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot read {}: {e}", req.path))
            })?;

            let mut applied = 0u32;
            for edit in &req.edits {
                if content.contains(&edit.old_text) {
                    content = content.replacen(&edit.old_text, &edit.new_text, 1);
                    applied += 1;
                }
            }

            if applied > 0 {
                tokio::fs::write(&sandboxed, &content).await.map_err(|e| {
                    McpToolError::internal(format!("Cannot write {}: {e}", req.path))
                })?;
                // Only emit the written span when a write actually occurred — a
                // no-op edit (zero matches) must not signal a file modification.
                self.emit_cns("file.written");
            }
            Ok(serde_json::json!({
                "edited": applied > 0,
                "path": path_str,
                "edits_applied": applied,
                "total_edits": req.edits.len(),
            }))
        })
        .await
    }

    #[tool(description = "List directory contents. Returns entry names, paths, types, and sizes.")]
    pub async fn fs_list(&self, Parameters(req): Parameters<FsListRequest>) -> String {
        execute_tool(self, "fs.list", async {
            let sandboxed = self.sandbox_path(&req.path)?;
            let path_str = sandboxed.to_string_lossy().to_string();

            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&sandboxed).await.map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot list {}: {e}", req.path))
            })?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| McpToolError::internal(format!("read_dir error: {e}")))?
            {
                let name = entry.file_name().to_string_lossy().to_string();
                let (is_dir, size) = match entry.metadata().await {
                    Ok(meta) => (meta.is_dir(), meta.len()),
                    Err(_) => (false, 0),
                };
                entries.push(serde_json::json!({
                    "name": name,
                    "path": entry.path().to_string_lossy(),
                    "is_dir": is_dir,
                    "size": size,
                }));
            }

            self.emit_cns("file.read");
            Ok(serde_json::json!({
                "path": path_str,
                "entries": entries,
                "count": entries.len(),
            }))
        })
        .await
    }

    #[tool(
        description = "Search files for a regex pattern. Walks directories up to max_depth (default 3). Returns file path, line number, and matching line content."
    )]
    pub async fn fs_search(&self, Parameters(req): Parameters<FsSearchRequest>) -> String {
        execute_tool(self, "fs.search", async {
            let sandboxed = self.sandbox_path(&req.path)?;

            // A non-existent or non-directory search root would otherwise walk
            // an empty iterator and silently return zero matches, masking the
            // bad input as "no hits". Surface it as an explicit error instead.
            if !sandboxed.is_dir() {
                return Err(McpToolError::invalid_argument(format!(
                    "Search path is not a directory: {}",
                    req.path
                )));
            }

            let re = regex::Regex::new(&req.pattern)
                .map_err(|e| McpToolError::invalid_argument(format!("Invalid regex: {e}")))?;

            let depth = req.max_depth.unwrap_or(3) as usize;
            let root = sandboxed;
            // Run the synchronous walkdir + file reads on a blocking thread so the
            // async runtime worker is not stalled, and surface unreadable / oversized
            // files instead of silently dropping them.
            let (matches, files_skipped) = tokio::task::spawn_blocking(move || {
                const MAX_FILE_BYTES: u64 = 1024 * 1024;
                let mut matches = Vec::new();
                let mut files_skipped = 0u64;
                for entry in walkdir::WalkDir::new(&root)
                    .max_depth(depth)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    if std::fs::metadata(entry.path())
                        .map(|m| m.len() > MAX_FILE_BYTES)
                        .unwrap_or(false)
                    {
                        files_skipped += 1;
                        continue;
                    }
                    match std::fs::read_to_string(entry.path()) {
                        Ok(content) => {
                            for (i, line) in content.lines().enumerate() {
                                if re.is_match(line) {
                                    matches.push(serde_json::json!({
                                        "path": entry.path().to_string_lossy(),
                                        "line": i + 1,
                                        "content": line.trim(),
                                    }));
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "hkask.mcp.filesystem",
                                path = %entry.path().display(),
                                error = %e,
                                "fs_search: skipped unreadable file"
                            );
                            files_skipped += 1;
                        }
                    }
                }
                (matches, files_skipped)
            })
            .await
            .map_err(|e| McpToolError::internal(format!("search task failed: {e}")))?;

            self.emit_cns("file.read");
            Ok(serde_json::json!({
                "pattern": req.pattern,
                "matches": matches,
                "count": matches.len(),
                "files_skipped": files_skipped,
            }))
        })
        .await
    }

    #[tool(description = "Delete a file or empty directory. Returns whether deletion succeeded.")]
    pub async fn fs_delete(&self, Parameters(req): Parameters<FsDeleteRequest>) -> String {
        execute_tool(self, "fs.delete", async {
            let sandboxed = self.sandbox_path(&req.path)?;
            let path_str = sandboxed.to_string_lossy().to_string();

            if !sandboxed
                .try_exists()
                .map_err(|e| McpToolError::internal(format!("Cannot check {}: {e}", req.path)))?
            {
                return Err(McpToolError::invalid_argument(format!(
                    "File not found: {}",
                    req.path
                )));
            }

            let delete_result = if sandboxed.is_dir() {
                tokio::fs::remove_dir(&sandboxed).await
            } else {
                tokio::fs::remove_file(&sandboxed).await
            };

            if let Err(e) = delete_result {
                return Err(McpToolError::invalid_argument(format!(
                    "Cannot delete {}: {e}",
                    req.path
                )));
            }

            self.emit_cns("file.deleted");
            Ok(serde_json::json!({"deleted": true, "path": path_str}))
        })
        .await
    }

    #[tool(
        description = "Execute a shell command via `sh -c`. Use cwd to set working directory. timeout_ms defaults to 30000 (30s). stdout and stderr are each truncated at max_output_bytes (default 102400 = 100KB) on a UTF-8 char boundary. Returns stdout, stderr, exit code, duration, and truncated flag. Note: the command string is not confined to project_root; only cwd is sandboxed."
    )]
    pub async fn shell_exec(&self, Parameters(req): Parameters<ShellExecRequest>) -> String {
        execute_tool(self, "shell.exec", async {
            let timeout_ms = req.timeout_ms.unwrap_or(30_000);
            // Cap at 10 MiB to bound memory and avoid u64→usize truncation on
            // 32-bit targets. The default is 100 KB.
            const MAX_OUTPUT_CAP: u64 = 10 * 1024 * 1024;
            let max_bytes = req.max_output_bytes.unwrap_or(102_400).min(MAX_OUTPUT_CAP) as usize;
            let start = Instant::now();

            let mut cmd = tokio::process::Command::new("sh");
            // kill_on_drop ensures a timed-out command is reaped when the
            // wait_with_output future is dropped on Elapsed. Without it the
            // child would be orphaned and keep running after we return a
            // timeout error to the caller.
            cmd.kill_on_drop(true)
                .arg("-c")
                .arg(&req.command)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .stdin(std::process::Stdio::null());

            // Sandbox cwd: if provided, canonicalize and verify within project_root.
            // If not provided, default to project_root.
            let cwd = match req.cwd {
                Some(ref c) => self.sandbox_path(c)?.to_string_lossy().to_string(),
                None => self.project_root.to_string_lossy().to_string(),
            };
            cmd.current_dir(&cwd);

            let child = cmd.spawn().map_err(|e| {
                McpToolError::invalid_argument(format!("Cannot spawn command: {e}"))
            })?;

            let output = tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                child.wait_with_output(),
            )
            .await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match output {
                Ok(Ok(out)) => {
                    let stdout_lossy = String::from_utf8_lossy(&out.stdout);
                    let stderr_lossy = String::from_utf8_lossy(&out.stderr);

                    let stdout_truncated = stdout_lossy.len() > max_bytes;
                    let stderr_truncated = stderr_lossy.len() > max_bytes;
                    let stdout_str =
                        truncate_at_char_boundary(&stdout_lossy, max_bytes).to_string();
                    let stderr_str =
                        truncate_at_char_boundary(&stderr_lossy, max_bytes).to_string();
                    let truncated = stdout_truncated || stderr_truncated;

                    let exit_code = out.status.code().unwrap_or(-1);
                    if exit_code != 0 {
                        self.emit_cns("command.failed");
                    } else {
                        self.emit_cns("command.completed");
                    }

                    Ok(serde_json::json!({
                        "stdout": stdout_str,
                        "stderr": stderr_str,
                        "exit_code": exit_code,
                        "duration_ms": duration_ms,
                        "truncated": truncated,
                    }))
                }
                Ok(Err(e)) => {
                    self.emit_cns("command.failed");
                    Err(McpToolError::internal(format!("Command error: {e}")))
                }
                Err(_) => {
                    self.emit_cns("command.failed");
                    Err(McpToolError::internal(format!(
                        "Command timed out after {timeout_ms}ms"
                    )))
                }
            }
        })
        .await
    }
}
