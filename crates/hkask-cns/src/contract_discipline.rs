//! Contract discipline CNS span emission.
//!
//! Provides functions to emit `cns.contract.violated` and `cns.contract.coverage`
//! spans into the CNS event stream. These spans feed the P9 homeostatic feedback
//! loop: contract violations trigger algedonic alerts, coverage drops trigger
//! variety deficit signals.
//!
//! # Wiring points
//!
//! - **`cns.contract.violated`**: emitted by CI when a contracted function's
//!   proptest fails. The CI job reads test output, identifies which `// REQ:`
//!   tag failed, and calls `emit_contract_violated()` via the CNS API.
//!
//! - **`cns.contract.coverage`**: emitted periodically by the Cybernetics Loop
//!   during its regulation cycle, or by the `scripts/contract-audit.sh` CI job.
//!   The coverage ratio is compared against the set point in `SetPointsConfig`.
//!
//! # Reference
//!
//! - Testing Discipline §9.3 — CNS span registration
//! - PRINCIPLES.md §1.4 — canonical span list
//! - contract-first-migration-plan-v0.27.0.md §5.4

use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use serde_json::json;

/// Emit a `cns.contract.violated` span when a contracted function's test fails.
///
/// Called by CI or test infrastructure when a proptest with a `// REQ:` tag
/// fails. The span carries the function name, contract spec_id, and failure
/// reason for algedonic routing.
///
/// # Arguments
/// - `sink` — the CNS event sink (from CyberneticsLoop or API)
/// - `function_name` — the fully-qualified function name (e.g., "energy::EnergyBudget::reserve")
/// - `contract_id` — the `// REQ:` spec_id (e.g., "CNS-001")
/// - `failure_reason` — human-readable description of the contract violation
pub fn emit_contract_violated(
    sink: &dyn NuEventSink,
    function_name: &str,
    contract_id: &str,
    failure_reason: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractViolated), "violated");
    let event = NuEvent::new(
        WebID::new(),
        span,
        Phase::Compare,
        json!({
            "function": function_name,
            "contract_id": contract_id,
            "failure_reason": failure_reason,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            function = function_name,
            contract_id = contract_id,
            error = %e,
            "Failed to persist contract violation span"
        );
    }
}

/// Emit a `cns.contract.coverage` span with the current contract coverage ratio.
///
/// Called periodically by the Cybernetics Loop or CI to report the fraction
/// of `pub fn` that have `// REQ: pre:` contracts. The CNS compares this
/// against the variety set point and triggers algedonic alerts on regression.
///
/// # Arguments
/// - `sink` — the CNS event sink
/// - `total_pub_fns` — total number of public functions (excluding test code)
/// - `contracted_fns` — number of functions with `// REQ: pre:` contracts
/// - `coverage_pct` — coverage percentage (0.0–100.0)
pub fn emit_contract_coverage(
    sink: &dyn NuEventSink,
    total_pub_fns: u64,
    contracted_fns: u64,
    coverage_pct: f64,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractCoverage), "measured");
    let event = NuEvent::new(
        WebID::new(),
        span,
        Phase::Sense,
        json!({
            "total_pub_fns": total_pub_fns,
            "contracted_fns": contracted_fns,
            "coverage_pct": coverage_pct,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            total_pub_fns = total_pub_fns,
            contracted_fns = contracted_fns,
            coverage_pct = coverage_pct,
            error = %e,
            "Failed to persist contract coverage span"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::NuEventSink;
    use std::sync::Mutex;

    /// A test event sink that captures the last persisted event.
    struct CaptureSink {
        last_event: Mutex<Option<NuEvent>>,
    }

    impl CaptureSink {
        fn new() -> Self {
            Self {
                last_event: Mutex::new(None),
            }
        }
    }

    impl NuEventSink for CaptureSink {
        fn persist(&self, event: &NuEvent) -> Result<(), hkask_types::InfrastructureError> {
            *self.last_event.lock().unwrap() = Some(event.clone());
            Ok(())
        }
    }

    #[test]
    fn emit_contract_violated_persists_event() {
        let sink = CaptureSink::new();
        emit_contract_violated(
            &sink,
            "energy::EnergyBudget::reserve",
            "CNS-001",
            "cap exceeded",
        );

        let event = sink.last_event.lock().unwrap().clone().unwrap();
        let obs = &event.observation;
        assert_eq!(obs["function"], "energy::EnergyBudget::reserve");
        assert_eq!(obs["contract_id"], "CNS-001");
        assert!(
            obs["failure_reason"]
                .as_str()
                .unwrap()
                .contains("cap exceeded")
        );
    }

    #[test]
    fn emit_contract_coverage_persists_event() {
        let sink = CaptureSink::new();
        emit_contract_coverage(&sink, 1531, 55, 3.6);

        let event = sink.last_event.lock().unwrap().clone().unwrap();
        let obs = &event.observation;
        assert_eq!(obs["total_pub_fns"], 1531);
        assert_eq!(obs["contracted_fns"], 55);
        assert!((obs["coverage_pct"].as_f64().unwrap() - 3.6).abs() < 0.01);
    }
}
