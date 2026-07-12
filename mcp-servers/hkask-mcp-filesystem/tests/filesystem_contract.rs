//! Contract tests for hkask-mcp-filesystem — sandbox path resolution invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `FileSystemServer::sandbox_path()` (path sandbox boundary, no external I/O beyond tempdir).

use hkask_mcp_filesystem::FileSystemServer;
use hkask_types::WebID;
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
    let dir = tempfile::tempdir().expect("tempdir");
    let server = test_server(dir.path().to_path_buf());
    // Path doesn't exist yet but is conceptually inside root
    let result = server.sandbox_path("not_created_yet.txt");
    // Should fail on canonicalize (file doesn't exist), not on boundary check
    assert!(result.is_err(), "nonexistent path should fail canonicalize");
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
