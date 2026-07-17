//! Contract tests for hkask-mcp-filesystem — sandbox path resolution and
//! tool-behavior invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seams:
//! - `FileSystemServer::sandbox_path()` (path sandbox boundary)
//! - Tool contracts: fs_write creates new files, fs_read range invariants,
//!   shell_exec truncation safety (stdout + stderr, multibyte), fs_search
//!   skip visibility, fs_delete error specificity.

use hkask_mcp_filesystem::FileSystemServer;
use hkask_mcp_filesystem::types::*;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::path::PathBuf;

fn test_server(root: PathBuf) -> FileSystemServer {
    FileSystemServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        root,
        hkask_mcp::server::CapabilityTier::detect(&std::collections::HashMap::new()),
    )
}

// ── Sandbox path tests ────────────────────────────────────────────────────

#[test]
fn sandbox_allows_file_in_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("test.txt"), "hello").expect("write file");

    let server = test_server(dir.path().to_path_buf());
    let result = server.sandbox_path("test.txt");
    assert!(
        result.is_ok(),
        "should allow file in root: {:?}",
        result.err()
    );
}

#[test]
fn sandbox_allows_nested_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).expect("create dirs");

    let server = test_server(dir.path().to_path_buf());
    let result = server.sandbox_path("a/b/c");
    assert!(
        result.is_ok(),
        "should allow nested dir: {:?}",
        result.err()
    );
}

#[test]
fn sandbox_rejects_parent_traversal() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Create a file outside the sandbox
    let outside = dir.path().parent().unwrap().join("outside.txt");
    std::fs::write(&outside, "secret").expect("write outside file");

    let server = test_server(dir.path().to_path_buf());
    let result = server.sandbox_path("../outside.txt");
    assert!(result.is_err(), "should reject ../ traversal");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("outside"),
        "error should mention outside path: {err}"
    );
}

#[test]
fn sandbox_rejects_absolute_path_outside_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    // Attempt to access /etc (always exists on Linux)
    let result = server.sandbox_path("/etc/passwd");
    assert!(result.is_err(), "should reject absolute path outside root");
}

#[test]
fn sandbox_accepts_nonexistent_path_inside_root() {
    // Paths that do not yet exist but are conceptually inside the root must be
    // accepted so fs_write can create new files. The sandbox resolves the
    // longest existing ancestor, verifies containment, and re-appends the
    // non-existent tail (fix for the create-new-file defect).
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    let result = server.sandbox_path("not_created_yet.txt");
    assert!(
        result.is_ok(),
        "nonexistent path inside root should resolve to inside-root canonical path: {:?}",
        result.err()
    );
    let resolved = result.unwrap();
    assert!(
        resolved.starts_with(
            dir.path()
                .canonicalize()
                .unwrap_or_else(|_| dir.path().to_path_buf())
        ),
        "resolved path must stay inside the sandbox root: {}",
        resolved.display()
    );
}

#[test]
fn sandbox_allows_root_itself() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    let result = server.sandbox_path(".");
    assert!(
        result.is_ok(),
        "should allow root itself: {:?}",
        result.err()
    );
}

// ── Server construction test ───────────────────────────────────────────────

#[test]
fn server_constructs_with_project_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let server = test_server(temp.path().to_path_buf());
    assert_eq!(
        server.project_root,
        temp.path()
            .canonicalize()
            .unwrap_or_else(|_| temp.path().to_path_buf())
    );
    assert_eq!(server.replicant, "test-replicant");
}

// ── Tool-behavior contract tests ──────────────────────────────────────────
//
// These exercise the actual tool contracts through the public tool methods
// (the seam an agent uses), closing the test-variety gap that hid the
// create-new-file, range-inversion, and multibyte-truncation defects.

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `error` message from an error envelope, if present.
fn error_message(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("error").and_then(|e| e.as_str()).map(String::from)
}

// REQ: fs.write creates a file that does not yet exist (P5 Testing Discipline).
// expect: a new file is written inside the sandbox and its contents persist.
#[tokio::test]
async fn fs_write_creates_new_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_write(Parameters(FsWriteRequest {
            path: "brand_new.txt".into(),
            content: "hello".into(),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["written"], true);
    assert_eq!(content["bytes"], 5);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("brand_new.txt")).unwrap(),
        "hello"
    );
}

// REQ: fs.write creates parent directories and the file (P5).
#[tokio::test]
async fn fs_write_creates_parent_dirs_and_new_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_write(Parameters(FsWriteRequest {
            path: "a/b/c/nested.txt".into(),
            content: "deep".into(),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["written"], true);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a/b/c/nested.txt")).unwrap(),
        "deep"
    );
}

// REQ: fs.read rejects an inverted range with an error, never a panic (P5).
// expect: start_line > end_line returns InvalidArgument, not a slice panic.
#[tokio::test]
async fn fs_read_range_inversion_returns_error_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("lines.txt"), "l1\nl2\nl3\nl4\nl5").unwrap();
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_read(Parameters(FsReadRequest {
            path: "lines.txt".into(),
            start_line: Some(4),
            end_line: Some(2),
        }))
        .await;
    let err = error_message(&out).expect("expected error for inverted range");
    assert!(err.contains("Invalid line range"), "got: {err}");
    assert!(err.contains("end_line"), "got: {err}");
}

// REQ: fs.read returns exactly the requested 1-based inclusive range.
#[tokio::test]
async fn fs_read_returns_requested_range() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("lines.txt"), "l1\nl2\nl3\nl4\nl5").unwrap();
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_read(Parameters(FsReadRequest {
            path: "lines.txt".into(),
            start_line: Some(2),
            end_line: Some(3),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["content"], "l2\nl3");
    assert_eq!(content["total_lines"], 5);
}

// REQ: shell.exec truncates stdout on a UTF-8 char boundary without panicking
// when the byte cap lands inside a multibyte codepoint (P5).
#[tokio::test]
async fn shell_exec_truncates_multibyte_without_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    // '€' is 3 UTF-8 bytes; a byte cap of 1 would split the codepoint.
    let out = server
        .shell_exec(Parameters(ShellExecRequest {
            command: "printf '€'".into(),
            cwd: None,
            timeout_ms: Some(5000),
            max_output_bytes: Some(1),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["truncated"], true);
    assert!(content["stdout"].is_string());
}

// REQ: shell.exec truncates stderr at the same byte cap as stdout (P5).
#[tokio::test]
async fn shell_exec_truncates_stderr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    // ~10KB to stderr, cap at 100 bytes.
    let out = server
        .shell_exec(Parameters(ShellExecRequest {
            command: "yes x | head -c 10000 >&2".into(),
            cwd: None,
            timeout_ms: Some(5000),
            max_output_bytes: Some(100),
        }))
        .await;
    let content = parse_content(&out);
    let stderr = content["stderr"].as_str().unwrap_or("");
    assert!(
        stderr.len() <= 100,
        "stderr must be truncated to <=100 bytes, got {}",
        stderr.len()
    );
    assert_eq!(content["truncated"], true);
}

// REQ: fs.search reports skipped unreadable files instead of silently dropping
// them (P5, observability).
#[tokio::test]
async fn fs_search_reports_skipped_unreadable_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("good.txt"), "hello world\nfoo bar\n").unwrap();
    // Invalid-UTF-8 bytes: read_to_string fails → skipped + logged.
    std::fs::write(dir.path().join("bad.bin"), [0xFF, 0xFE, 0x00, b'x']).unwrap();
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_search(Parameters(FsSearchRequest {
            pattern: "foo".into(),
            path: ".".into(),
            max_depth: Some(2),
        }))
        .await;
    let content = parse_content(&out);
    let matches = content["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1, "should match 'foo' in good.txt");
    assert_eq!(content["count"], 1);
    assert!(
        content["files_skipped"].as_u64().unwrap_or(0) >= 1,
        "binary file should be reported as skipped"
    );
}

// REQ: fs.delete reports the real OS error for a non-empty directory rather
// than collapsing to a generic message (P5, diagnosability).
#[tokio::test]
async fn fs_delete_reports_specific_error_for_non_empty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("nonempty")).unwrap();
    std::fs::write(dir.path().join("nonempty/child.txt"), "x").unwrap();
    let server = test_server(dir.path().to_path_buf());
    let out = server
        .fs_delete(Parameters(FsDeleteRequest {
            path: "nonempty".into(),
        }))
        .await;
    let err = error_message(&out).expect("expected error for non-empty dir delete");
    assert!(
        err.contains("not empty"),
        "should report the real OS reason, got: {err}"
    );
    // The old collapsed message said "or permission denied"; the real OS error
    // for a non-empty directory does not include that phrase.
    assert!(
        !err.contains("permission denied"),
        "should not collapse to generic, got: {err}"
    );
}
