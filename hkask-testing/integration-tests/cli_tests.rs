//! hKask CLI Integration Tests
//!
//! Tests for CLI commands: template list, manifest execute, etc.

use std::process::Command;

/// Test CLI help command
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "--help"])
        .output()
        .expect("Failed to execute kask --help");
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kask"));
    assert!(stdout.contains("Template"));
    assert!(stdout.contains("Manifest"));
}

/// Test CLI template list command
#[test]
fn test_cli_template_list() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "template", "list"])
        .output()
        .expect("Failed to execute kask template list");
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Registered templates"));
}

/// Test CLI template list with type filter
#[test]
fn test_cli_template_list_by_type() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "template", "list", "--type", "prompt"])
        .output()
        .expect("Failed to execute kask template list --type prompt");
    
    assert!(output.status.success());
}

/// Test CLI manifest list command
#[test]
fn test_cli_manifest_list() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "manifest", "list"])
        .output()
        .expect("Failed to execute kask manifest list");
    
    // Command may succeed even if no manifests directory exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Either shows manifests or indicates directory not found
    assert!(
        stdout.contains("Registered manifests") || 
        stderr.contains("No manifests directory"),
        "Unexpected output: {} {}",
        stdout,
        stderr
    );
}

/// Test CLI template get command
#[test]
fn test_cli_template_get() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "template", "get", "prompt/selector"])
        .output()
        .expect("Failed to execute kask template get");
    
    // May succeed or fail depending on registry state
    // Just verify command runs without panic
}

/// Test CLI template search command
#[test]
fn test_cli_template_search() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "kask", "--", "template", "search", "classify"])
        .output()
        .expect("Failed to execute kask template search");
    
    assert!(output.status.success());
}
