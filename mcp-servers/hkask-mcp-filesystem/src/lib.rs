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

pub mod types;
use types::*;

use hkask_mcp::server::{CapabilityTier, McpToolError, ToolContext, execute_tool};
use hkask_types::WebID;
use hkask_types::cns::{CnsSpan, ToolSubsystem};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ── Server ───────────────────────────────────────────────────────────────

pub struct FileSystemServer {
    pub webid: WebID,
    pub replicant: String,
    pub daemon: Option<hkask_mcp::DaemonClient>,
    /// Project root — all file I/O is sandboxed within this directory tree.
    pub project_root: PathBuf,
    pub capability_tier: CapabilityTier,
}

impl FileSystemServer {
    /// Emit a CNS span for a filesystem operation.
    fn emit_cns(&self, operation: &str) {
        CnsSpan::Tool {
            subsystem: ToolSubsystem::Filesystem,
        }
        .emit(operation);
    }

    /// Validate that `raw_path` is within `self.project_root` after canonicalization.
    fn sandbox_path(&self, raw_path: &str) -> Result<PathBuf, McpToolError> {
        let candidate = Path::new(raw_path);
        let resolved = if candidate.is_relative() {
            self.project_root.join(candidate)
        } else {
            candidate.to_path_buf()
        };
        let canonical = resolved.canonicalize().map_err(|e| {
            McpToolError::invalid_argument(format!("Cannot resolve path '{raw_path}': {e}"))
        })?;
        let canonical_root = self
            .project_root
            .canonicalize()
            .unwrap_or_else(|_| self.project_root.clone());
        if !canonical.starts_with(&canonical_root) {
            self.emit_cns("path.rejected");
            return Err(McpToolError::invalid_argument(format!(
                "Path '{raw_path}' is outside the project root '{}'",
                self.project_root.display()
            )));
        }
        Ok(canonical)
    }
}

impl ToolContext for FileSystemServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl FileSystemServer {
    #[tool(
        description = "Read a file's contents. Use start_line/end_line for targeted reads. Returns content, line count, file size, and modification time."
    )]
    async fn fs_read(&self, Parameters(req): Parameters<FsReadRequest>) -> String {
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

            let (output, range) = match (req.start_line, req.end_line) {
                (Some(s), Some(e)) => {
                    let start = (s.saturating_sub(1) as usize).min(total_lines);
                    let end = (e as usize).min(total_lines);
                    (lines[start..end].join("\n"), Some(format!("{s}-{e}")))
                }
                _ => (content, None),
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
    async fn fs_write(&self, Parameters(req): Parameters<FsWriteRequest>) -> String {
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
        description = "Apply targeted text replacements to a file. Each edit replaces the first occurrence of old_text with new_text. Returns count of applied edits."
    )]
    async fn fs_edit(&self, Parameters(req): Parameters<FsEditRequest>) -> String {
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
            }

            self.emit_cns("file.written");
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
    async fn fs_list(&self, Parameters(req): Parameters<FsListRequest>) -> String {
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
    async fn fs_search(&self, Parameters(req): Parameters<FsSearchRequest>) -> String {
        execute_tool(self, "fs.search", async {
            let sandboxed = self.sandbox_path(&req.path)?;

            let re = regex::Regex::new(&req.pattern)
                .map_err(|e| McpToolError::invalid_argument(format!("Invalid regex: {e}")))?;

            let depth = req.max_depth.unwrap_or(3) as usize;
            let mut matches = Vec::new();

            for entry in walkdir::WalkDir::new(&sandboxed)
                .max_depth(depth)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
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
            }

            self.emit_cns("file.read");
            Ok(serde_json::json!({
                "pattern": req.pattern,
                "matches": matches,
                "count": matches.len(),
            }))
        })
        .await
    }

    #[tool(description = "Delete a file or empty directory. Returns whether deletion succeeded.")]
    async fn fs_delete(&self, Parameters(req): Parameters<FsDeleteRequest>) -> String {
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

            let deleted = if sandboxed.is_dir() {
                tokio::fs::remove_dir(&sandboxed).await.is_ok()
            } else {
                tokio::fs::remove_file(&sandboxed).await.is_ok()
            };

            if !deleted {
                return Err(McpToolError::invalid_argument(format!(
                    "Cannot delete {}: directory not empty or permission denied",
                    req.path
                )));
            }

            self.emit_cns("file.deleted");
            Ok(serde_json::json!({"deleted": true, "path": path_str}))
        })
        .await
    }

    #[tool(
        description = "Execute a shell command via `sh -c`. Use cwd to set working directory. timeout_ms defaults to 30000 (30s). Output truncated at max_output_bytes (default 102400 = 100KB). Returns stdout, stderr, exit code, duration, and truncated flag."
    )]
    async fn shell_exec(&self, Parameters(req): Parameters<ShellExecRequest>) -> String {
        execute_tool(self, "shell.exec", async {
            let timeout_ms = req.timeout_ms.unwrap_or(30_000);
            let max_bytes = req.max_output_bytes.unwrap_or(102_400) as usize;
            let start = Instant::now();

            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c")
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

                    let (stdout_str, truncated) = if stdout_lossy.len() > max_bytes {
                        (stdout_lossy[..max_bytes].to_string(), true)
                    } else {
                        (stdout_lossy.to_string(), false)
                    };

                    let exit_code = out.status.code().unwrap_or(-1);
                    if exit_code != 0 {
                        self.emit_cns("command.failed");
                    } else {
                        self.emit_cns("command.completed");
                    }

                    Ok(serde_json::json!({
                        "stdout": stdout_str,
                        "stderr": stderr_lossy,
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
