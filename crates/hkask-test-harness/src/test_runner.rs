//! Contract test runner — shells out to `cargo test`, parses REQ-tagged
//! failures, returns structured results for CNS span emission.
//!
//! Also provides `discover_uncontracted_functions` for source-level contract
//! audit without running tests — useful for agent discovery workflows.
//!
//! # Principle grounding
//! - P8 (Semantic Grounding): every failure carries the REQ tag it violated
//! - P5 (Essentialism): one function, no framework — just runs cargo test and parses

use std::process::Command;

/// Result of running contract tests on a crate.
#[derive(Debug, Clone)]
pub struct ContractTestResult {
    pub crate_name: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    /// REQ-tagged failures: (function_name, contract_id, failure_message)
    pub violations: Vec<ContractViolation>,
}

/// A single contract violation — one REQ-tagged test that failed.
#[derive(Debug, Clone)]
pub struct ContractViolation {
    /// The test function name (e.g., "tests::proptest_tests::budget_never_exceeds_cap")
    pub test_name: String,
    /// The REQ tag from the source (e.g., "P9-cns-energy-budget-test")
    pub contract_id: String,
    /// Human-readable failure reason extracted from test output
    pub failure_reason: String,
    /// The source file containing the REQ tag
    pub source_file: String,
}

/// Run `cargo test` on the specified crate and parse REQ-tagged failures.
///
/// Returns `None` if `cargo test` could not be executed (not a Cargo project,
/// toolchain missing, etc.) — callers should treat this as non-fatal.
///
/// # Arguments
/// - `crate_name` — the Cargo package name (e.g., "hkask-cns")
/// - `workspace_root` — path to the workspace root containing `Cargo.toml`
///
/// REQ: HARN-012
/// pre:  workspace_root exists and contains Cargo.toml
/// post: returns ContractTestResult with pass/fail counts and REQ-tagged violations
pub fn run_contract_tests(crate_name: &str, workspace_root: &str) -> Option<ContractTestResult> {
    let output = Command::new("cargo")
        .args([
            "test",
            "-p",
            crate_name,
            "--lib",
            "--",
            "--test-threads=1",
            "--format=terse",
        ])
        .current_dir(workspace_root)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Parse: "test tests::foo::bar ... FAILED"
    let failed_tests: Vec<String> = combined
        .lines()
        .filter(|l| l.contains("... FAILED"))
        .filter_map(|l| {
            let parts: Vec<&str> = l.splitn(2, " ... FAILED").collect();
            parts
                .first()
                .map(|s| s.trim().strip_prefix("test ").unwrap_or(s).to_string())
        })
        .collect();

    // Quick counts from standard test summary line
    let total_tests = extract_count(&combined, "running ", " test");
    let total_failed = failed_tests.len();
    let actual_passed = total_tests.saturating_sub(total_failed);

    // Resolve REQ tags for failed tests
    let mut violations = Vec::new();
    for test_name in &failed_tests {
        // Test name format: "module::submodule::test_fn"
        // Extract the function name and search source files for the REQ tag
        let fn_name = test_name.split("::").last().unwrap_or(test_name);
        let (contract_id, source_file) = resolve_req_tag(workspace_root, crate_name, fn_name);

        let violation = ContractViolation {
            test_name: test_name.clone(),
            contract_id: contract_id.unwrap_or_else(|| "unknown".to_string()),
            failure_reason: extract_failure_message(&combined, test_name),
            source_file: source_file.unwrap_or_else(|| "unknown".to_string()),
        };
        violations.push(violation);
    }

    Some(ContractTestResult {
        crate_name: crate_name.to_string(),
        total_tests,
        passed: actual_passed,
        failed: total_failed,
        violations,
    })
}

/// Extract a numeric count from test output (e.g., "running 47 tests")
fn extract_count(output: &str, prefix: &str, suffix: &str) -> usize {
    for line in output.lines() {
        if let Some(start) = line.find(prefix) {
            let rest = &line[start + prefix.len()..];
            if let Some(end) = rest.find(suffix) {
                let num_str = &rest[..end].trim();
                if let Ok(n) = num_str.parse::<usize>() {
                    return n;
                }
            }
        }
    }
    0
}

/// Extract the failure message for a specific test from cargo test output.
fn extract_failure_message(output: &str, test_name: &str) -> String {
    // cargo test output structure after failures:
    // ---- test_name stdout ----
    // thread 'test_name' panicked at ...
    let marker = format!("---- {} stdout ----", test_name);
    if let Some(pos) = output.find(&marker) {
        let rest = &output[pos..];
        // Take up to 500 chars or until "failures:" section
        let snippet: String = rest.lines().take(15).collect::<Vec<_>>().join("\n");
        // Truncate to 500 chars
        if snippet.len() > 500 {
            format!("{}...", &snippet[..497])
        } else {
            snippet
        }
    } else {
        // Fallback: search for lines containing the test name near "panicked"
        output
            .lines()
            .filter(|l| l.contains(test_name) && (l.contains("panicked") || l.contains("FAILED")))
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Search source files in the crate for a REQ tag near the given function name.
///
/// Strategy: grep the crate's src/ and tests/ directories for `fn <name>`
/// and check the 10 lines above it for a `// REQ:` or `/// REQ:` tag.
fn resolve_req_tag(
    workspace_root: &str,
    crate_name: &str,
    fn_name: &str,
) -> (Option<String>, Option<String>) {
    let src_dir = format!("{}/crates/{}/src", workspace_root, crate_name);
    let tests_dir = format!("{}/crates/{}/tests", workspace_root, crate_name);

    for dir in &[&src_dir, &tests_dir] {
        let Ok(output) = Command::new("grep")
            .args(["-rn", &format!("fn {}", fn_name), dir])
            .output()
        else {
            continue;
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // Format: "file.rs:42:    fn test_name("
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() < 2 {
                continue;
            }
            let file = parts[0].to_string();
            let line_num: usize = parts[1]
                .split(':')
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            if line_num > 0 {
                // Read 10 lines above the function
                let Ok(content) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let lines: Vec<&str> = content.lines().collect();
                let start = line_num.saturating_sub(11).min(lines.len());
                let end = (line_num - 1).min(lines.len());
                if start < end {
                    let context = &lines[start..end];
                    for ctx_line in context {
                        if let Some(req) = extract_req_tag(ctx_line) {
                            return (Some(req), Some(file));
                        }
                    }
                }
            }
        }
    }

    (None, None)
}

/// Extract REQ tag from a comment line. Returns the tag value (e.g., "P9-cns-energy-budget-test").
fn extract_req_tag(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Match: // REQ: TAG or /// REQ: TAG
    if let Some(pos) = trimmed.find("REQ:") {
        let tag = trimmed[pos + 4..].trim();
        // Take up to first space or end of line
        let end = tag.find(|c: char| c.is_whitespace()).unwrap_or(tag.len());
        let req = tag[..end].trim_end_matches(&['.', ',', ';', ':', ')', ']', '}']);
        if !req.is_empty() {
            return Some(req.to_string());
        }
    }
    None
}

// ── Source-level contract discovery ────────────────────────────────────────

/// A public function without a REQ contract.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UncontractedFunction {
    pub crate_name: String,
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub signature: String,
}

/// Crate-level contract audit summary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractAudit {
    pub crate_name: String,
    pub total_pub_fns: usize,
    pub contracted: usize,
    pub coverage_pct: f64,
    pub uncontracted: Vec<UncontractedFunction>,
}

/// Discover public functions without REQ contracts in a crate.
///
/// Walks `crates/<crate_name>/src/` looking for `pub fn` and `pub async fn`
/// declarations, then checks if a `REQ:` tag exists within 10 lines above.
///
/// Returns `None` if the crate source directory doesn't exist.
///
/// REQ: HARN-043
/// pre:  workspace_root exists and contains crates/<crate_name>/src/
/// post: returns ContractAudit with counts and uncontracted function list
pub fn discover_uncontracted_functions(
    crate_name: &str,
    workspace_root: &str,
) -> Option<ContractAudit> {
    let src_dir = format!("{}/crates/{}/src", workspace_root, crate_name);
    let dir = std::path::Path::new(&src_dir);
    if !dir.exists() {
        return None;
    }

    let mut total = 0usize;
    let mut contracted = 0usize;
    let mut uncontracted = Vec::new();

    walk_rs_files(dir, &mut |file_path| {
        let Ok(content) = std::fs::read_to_string(file_path) else {
            return;
        };
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if (line.starts_with("pub fn ") || line.starts_with("pub async fn "))
                && !line.contains("cfg(test)")
            {
                total += 1;
                let fn_name = extract_function_name(line);
                // Check 15 lines above for REQ: tag OR #[rs::contract] attribute
                let has_req = (i.saturating_sub(15)..i).any(|j| {
                    j < lines.len()
                        && (extract_req_tag(lines[j]).is_some()
                            || lines[j].contains("#[rs::contract]")
                            || lines[j].contains("#[rs::contract("))
                });
                if has_req {
                    contracted += 1;
                } else {
                    uncontracted.push(UncontractedFunction {
                        crate_name: crate_name.to_string(),
                        function_name: fn_name,
                        file: file_path.to_string_lossy().to_string(),
                        line: i + 1,
                        signature: line.to_string(),
                    });
                }
            }
            i += 1;
        }
    });

    let coverage_pct = if total > 0 {
        (contracted as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    Some(ContractAudit {
        crate_name: crate_name.to_string(),
        total_pub_fns: total,
        contracted,
        coverage_pct,
        uncontracted,
    })
}

/// Walk all .rs files in a directory tree, calling `f` for each.
fn walk_rs_files(dir: &std::path::Path, f: &mut dyn FnMut(&std::path::Path)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().map_or(false, |n| n != "tests") {
                walk_rs_files(&path, f);
            } else if path.extension().map_or(false, |e| e == "rs") {
                f(&path);
            }
        }
    }
}

/// Extract the function name from a `pub fn` or `pub async fn` declaration.
fn extract_function_name(line: &str) -> String {
    let trimmed = line.trim();
    let after_fn = trimmed
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("fn ");
    after_fn
        .split('(')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_req_tag_from_line_comment() {
        let tag = extract_req_tag("    // REQ: P9-cns-energy-budget-test");
        assert_eq!(tag, Some("P9-cns-energy-budget-test".to_string()));
    }

    #[test]
    fn extract_req_tag_from_doc_comment() {
        let tag = extract_req_tag("/// REQ: CNS-001  pre: x > 0");
        assert_eq!(tag, Some("CNS-001".to_string()));
    }

    #[test]
    fn extract_req_tag_no_match() {
        assert_eq!(extract_req_tag("// just a comment"), None);
        assert_eq!(extract_req_tag(""), None);
    }

    #[test]
    fn extract_count_parses_cargo_output() {
        let output = "running 47 tests\ntest result: ok. 47 passed; 0 failed";
        assert_eq!(extract_count(output, "running ", " test"), 47);
        assert_eq!(extract_count("no match", "running ", " test"), 0);
    }

    #[test]
    fn contract_test_result_debug_format() {
        let result = ContractTestResult {
            crate_name: "test-crate".into(),
            total_tests: 10,
            passed: 9,
            failed: 1,
            violations: vec![],
        };
        let dbg = format!("{:?}", result);
        assert!(dbg.contains("test-crate"));
        assert!(dbg.contains("10"));
    }
}
