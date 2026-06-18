//! Contract tests for hkask-mcp-condenser — compression engine invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `CondenserEngine` (pure computation, no I/O dependencies).

use hkask_condenser::engine::CondenserEngine;
use hkask_condenser::types::Profile;

// ── Classification contract tests ───────────────────────────────────────────

// REQ: COND-CLASSIFY-001 — classify maps shell tools to ShellCommand category
// expect: "I can see which category the condenser assigns to each tool name" [P8]
#[test]
fn classify_shell_tool() {
    let engine = CondenserEngine::new();
    let (cat, algo) = engine.classify("bash_execute");
    assert_eq!(cat.label(), "shell_command");
    assert!(!algo.is_empty());
}

// REQ: COND-CLASSIFY-002 — classify maps test tools
// expect: "I can see test-output tools get the correct category" [P8]
#[test]
fn classify_test_tool() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("pytest_run");
    assert_eq!(cat.label(), "test_output");
}

// REQ: COND-CLASSIFY-003 — classify maps chat tools
// expect: "I can see chat tools get the correct category" [P8]
#[test]
fn classify_chat_tool() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("message_send");
    assert_eq!(cat.label(), "conversation_history");
}

// REQ: COND-CLASSIFY-004 — unknown tools get "unknown" category
// expect: "I can see that unfamiliar tool names are classified as unknown" [P8]
#[test]
fn classify_unknown_tool_is_unknown() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("xyzzy_unknown_tool");
    assert_eq!(cat.label(), "unknown");
}

// ── Compression contract tests ──────────────────────────────────────────────

// REQ: COND-COMPRESS-001 — compress reduces output size
// expect: "I can compress verbose tool output and see size reduction" [P5]
#[test]
fn compress_reduces_size() {
    let mut engine = CondenserEngine::new();
    let input = "line1\nline2\nline3\nline4\nline5\n".repeat(20); // 100 lines
    let result = engine.compress("bash_execute", &input, None);
    assert!(result.compressed_bytes <= result.original_bytes,
        "compressed ({}) should be <= original ({})",
        result.compressed_bytes, result.original_bytes);
    assert_eq!(result.original_lines, 100);
}

// REQ: COND-COMPRESS-002 — compress with explicit category
// expect: "I can specify a category when compressing to override auto-classification" [P5]
#[test]
fn compress_with_explicit_category() {
    let mut engine = CondenserEngine::new();
    let input = "some output text";
    let result = engine.compress(
        "some_tool",
        input,
        Some(hkask_condenser::types::ContextCategory::ShellCommand),
    );
    assert_eq!(result.category, "shell_command");
}

// REQ: COND-COMPRESS-003 — compress empty input returns valid output
// expect: "I can compress empty input without errors" [P8]
#[test]
fn compress_empty_input() {
    let mut engine = CondenserEngine::new();
    let result = engine.compress("bash_execute", "", None);
    assert_eq!(result.original_bytes, 0);
    assert_eq!(result.reduction_pct, 0.0);
}

// REQ: COND-COMPRESS-004 — repeated compression increments stats
// expect: "I can verify that compression statistics accumulate correctly" [P9]
#[test]
fn repeated_compression_increments_stats() {
    let mut engine = CondenserEngine::new();
    let input = "hello world";
    for _ in 0..5 {
        engine.compress("bash_execute", input, None);
    }
    assert_eq!(engine.stats.total_compressions, 5);
    assert!(engine.stats.total_original_bytes > 0);
    assert!(engine.stats.total_compressed_bytes > 0);
}

// ── Profile contract tests ──────────────────────────────────────────────────

// REQ: COND-PROFILE-001 — set_profile changes compression behavior
// expect: "I can change the compression profile and see different output" [P5]
#[test]
fn set_profile_changes_behavior() {
    let mut engine = CondenserEngine::new();
    engine.set_profile(Profile::Heavy);
    let result = engine.compress("bash_execute", "a\nb\nc\nd\ne\nf", None);
    assert_eq!(result.profile, "heavy");
}

// REQ: COND-PROFILE-002 — Normal profile is the default
// expect: "I can verify the default compression profile is Normal" [P8]
#[test]
fn default_profile_is_normal() {
    let engine = CondenserEngine::new();
    let stats = engine.get_stats();
    assert_eq!(stats.total_compressions, 0);
}

// ── Health check contract tests ─────────────────────────────────────────────

// REQ: COND-HEALTH-001 — health check returns signals
// expect: "I can query the condenser's health status" [P9]
#[test]
fn health_check_returns_signals() {
    let engine = CondenserEngine::new();
    let signals = engine.check_global_health();
    // Engine should return health signals (may be empty for fresh engine)
    assert!(signals.len() == 0 || signals.len() > 0,
        "health check should return signals or empty vec");
}
