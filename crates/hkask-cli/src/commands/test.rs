//! Test command handlers for `kask test`
//!
//! Runs contract tests on specified crates and reports REQ-tagged violations.
//! Uses the embedded test runner from `hkask-services` to shell out to
//! `cargo test` and parse results.

use std::time::Duration;

/// REQ: CLI-090
/// pre:  rt is a valid tokio Runtime
/// post: runs cargo test on the specified crate or all priority crates,
///       reports REQ-tagged failures to stdout
pub fn run(
    _rt: &tokio::runtime::Runtime,
    crate_name: Option<String>,
    format: &str,
    watch: Option<u64>,
) {
    let workspace_root = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let crates: Vec<String> = if let Some(c) = crate_name {
        vec![c]
    } else {
        vec![
            "hkask-cns".into(),
            "hkask-wallet".into(),
            "hkask-keystore".into(),
            "hkask-condenser".into(),
            "hkask-storage".into(),
            "hkask-services".into(),
            "hkask-mcp".into(),
        ]
    };

    if let Some(interval_secs) = watch {
        println!(
            "Contract test monitor — watching every {}s (Ctrl+C to stop)",
            interval_secs
        );
        loop {
            run_all(&crates, &workspace_root, format);
            std::thread::sleep(Duration::from_secs(interval_secs));
        }
    } else {
        run_all(&crates, &workspace_root, format);
    }
}

fn run_all(crates: &[String], workspace_root: &str, format: &str) {
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut all_violations: Vec<(String, String, String)> = Vec::new();

    for crate_name in crates {
        match run_one(crate_name, workspace_root) {
            Some(outcome) => {
                total_passed += outcome.passed;
                total_failed += outcome.failed;
                for v in &outcome.violations {
                    all_violations.push((
                        crate_name.clone(),
                        v.test_name.clone(),
                        v.contract_id.clone(),
                    ));
                }
            }
            None => {
                if format == "json" {
                    eprintln!(
                        "{{\"crate\":\"{}\",\"error\":\"cargo not available\"}}",
                        crate_name
                    );
                } else {
                    eprintln!("  {}: cargo not available (skipping)", crate_name);
                }
            }
        }
    }

    match format {
        "json" => {
            println!(
                "{{\"total_passed\":{},\"total_failed\":{},\"violations\":[",
                total_passed, total_failed
            );
            for (i, (crate_name, test_name, contract_id)) in all_violations.iter().enumerate() {
                let comma = if i + 1 < all_violations.len() {
                    ","
                } else {
                    ""
                };
                println!(
                    "  {{\"crate\":\"{}\",\"test\":\"{}\",\"contract\":\"{}\"}}{}",
                    crate_name, test_name, contract_id, comma
                );
            }
            println!("]}}");
        }
        _ => {
            println!();
            println!("Contract Test Results");
            println!("=====================");
            println!("  Passed:  {}", total_passed);
            println!("  Failed:  {}", total_failed);
            if !all_violations.is_empty() {
                println!();
                println!("REQ-Tagged Violations:");
                for (crate_name, test_name, contract_id) in &all_violations {
                    println!("  • {}::{}  [{}]", crate_name, test_name, contract_id);
                }
            }
            if total_failed == 0 && total_passed > 0 {
                println!();
                println!("All contract tests passed.");
            }
        }
    }
}

/// Lightweight outcome from running one crate's tests.
struct CrateOutcome {
    passed: usize,
    failed: usize,
    violations: Vec<CrateViolation>,
}

struct CrateViolation {
    test_name: String,
    contract_id: String,
}

fn run_one(crate_name: &str, workspace_root: &str) -> Option<CrateOutcome> {
    let output = std::process::Command::new("cargo")
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

    let total_tests = {
        let mut count = 0usize;
        for line in combined.lines() {
            if line.starts_with("running ") && line.contains(" test") {
                let num = line
                    .trim_start_matches("running ")
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                count += num;
            }
        }
        count
    };

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

    let total_failed = failed_tests.len();
    let actual_passed = total_tests.saturating_sub(total_failed);

    let violations: Vec<CrateViolation> = failed_tests
        .iter()
        .map(|test_name| {
            let fn_name = test_name.split("::").last().unwrap_or(test_name);
            let contract_id = find_req_for_fn(workspace_root, crate_name, fn_name);
            CrateViolation {
                test_name: test_name.clone(),
                contract_id,
            }
        })
        .collect();

    Some(CrateOutcome {
        passed: actual_passed,
        failed: total_failed,
        violations,
    })
}

/// Find a REQ tag near the given function name in the crate's source.
fn find_req_for_fn(workspace_root: &str, crate_name: &str, fn_name: &str) -> String {
    let src_dir = format!("{}/crates/{}/src", workspace_root, crate_name);
    let tests_dir = format!("{}/crates/{}/tests", workspace_root, crate_name);

    for dir in &[&src_dir, &tests_dir] {
        let Ok(output) = std::process::Command::new("grep")
            .args(["-rn", &format!("fn {}", fn_name), dir])
            .output()
        else {
            continue;
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() < 2 {
                continue;
            }
            let file = parts[0];
            let line_num: usize = parts[1]
                .split(':')
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            if line_num > 0 {
                let Ok(content) = std::fs::read_to_string(file) else {
                    continue;
                };
                let lines: Vec<&str> = content.lines().collect();
                let start = line_num.saturating_sub(11).min(lines.len());
                let end = (line_num - 1).min(lines.len());
                if start < end {
                    for ctx_line in &lines[start..end] {
                        let trimmed = ctx_line.trim();
                        if let Some(pos) = trimmed.find("REQ:") {
                            let tag = trimmed[pos + 4..].trim();
                            let end_pos =
                                tag.find(|c: char| c.is_whitespace()).unwrap_or(tag.len());
                            let req = tag[..end_pos]
                                .trim_end_matches(&['.', ',', ';', ':', ')', ']', '}']);
                            if !req.is_empty() {
                                return req.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    "unknown".to_string()
}
