//! Contract test monitor — periodic `cargo test` on priority crates with CNS span emission.
//!
//! Closes the sense-loop for contract violations: test failures previously
//! invisible to the CNS are surfaced as `cns.contract.violated` events.

use hkask_cns::emit_contract_violated_with_task;
use hkask_test_harness::test_runner;

pub(crate) fn spawn_contract_test_loop(
    event_sink: &std::sync::Arc<dyn hkask_types::event::NuEventSink>,
    triple_store: &std::sync::Arc<hkask_storage::TripleStore>,
    workspace_root: &str,
) {
    let interval_secs: u64 = std::env::var("HKASK_CONTRACT_TEST_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);

    if interval_secs == 0 {
        tracing::info!(
            target: "cns.contract",
            "Contract test loop disabled (HKASK_CONTRACT_TEST_INTERVAL_SECS=0)"
        );
        return;
    }

    let sink = std::sync::Arc::clone(event_sink);
    let store = std::sync::Arc::clone(triple_store);
    let root = workspace_root.to_string();

    let priority_crates: &[&str] = &[
        "hkask-cns",
        "hkask-wallet",
        "hkask-keystore",
        "hkask-condenser",
        "hkask-storage",
        "hkask-services",
        "hkask-mcp",
        "hkask-mcp-kanban",
        "hkask-mcp-replica",
    ];

    tokio::spawn(async move {
        tracing::info!(
            target: "cns.contract",
            interval_secs = %interval_secs,
            crates = ?priority_crates,
            "Contract test monitor started"
        );

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;

            for crate_name in priority_crates {
                let result = run_contract_tests_for_crate(crate_name, &root);
                match result {
                    Some(r) if r.failed > 0 => {
                        tracing::warn!(
                            target: "cns.contract",
                            crate_name = %r.crate_name,
                            failed = %r.failed,
                            passed = %r.passed,
                            violations = %r.violations.len(),
                            "Contract tests failed — emitting CNS spans"
                        );
                        for violation in &r.violations {
                            let _ = emit_contract_violated_with_task(
                                &*sink,
                                &store,
                                &violation.test_name,
                                &violation.contract_id,
                                &violation.failure_reason,
                                None,
                            );
                        }
                    }
                    Some(r) => {
                        tracing::debug!(
                            target: "cns.contract",
                            crate_name = %r.crate_name,
                            passed = %r.passed,
                            "Contract tests passed"
                        );
                        let _ = r;
                    }
                    None => {
                        tracing::debug!(
                            target: "cns.contract",
                            crate_name = %crate_name,
                            "Contract test runner unavailable — cargo not found"
                        );
                    }
                }
            }
        }
    });
}

fn run_contract_tests_for_crate(
    crate_name: &str,
    workspace_root: &str,
) -> Option<test_runner::ContractTestResult> {
    test_runner::run_contract_tests(crate_name, workspace_root)
}
