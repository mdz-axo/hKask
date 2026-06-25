//! Test command handlers for `kask test`
//!
//! Runs contract tests on specified crates and reports REQ-tagged violations.
//! Delegates to `hkask-test-harness::test_runner::run_contract_tests()`.

use std::time::Duration;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  crate_name is a valid workspace crate name or None; format is "text" or "json"
/// post: runs cargo test on the specified crate or all priority crates,
///       reports REQ-tagged failures to stdout
pub fn run(crate_name: Option<String>, format: &str, watch: Option<u64>) {
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
        match hkask_test_harness::test_runner::run_contract_tests(crate_name, workspace_root) {
            Some(result) => {
                total_passed += result.passed;
                total_failed += result.failed;
                for v in &result.violations {
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
