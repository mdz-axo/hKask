//! Contract discipline CNS span emission and kanban task bridging.
//!
//! Provides functions to emit `cns.contract.violated` and `cns.contract.coverage`
//! spans into the CNS event stream, and to create kanban tasks for replicant-driven
//! remediation. These spans feed the P9 homeostatic feedback loop: contract violations
//! trigger algedonic alerts, coverage drops trigger variety deficit signals.
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
//! - **Kanban task bridge**: `create_contract_violation_task()` persists a
//!   task triple in the store for each contract violation. This closes the
//!   sense→act loop: violation detected → CNS span emitted → kanban task
//!   created → replicant assigned to fix. Tasks use the canonical kanban
//!   triple scheme so they're queryable through KanbanService.
//!
//! # Reference
//!
//! - Testing Discipline §9.3 — CNS span registration
//! - canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`)
//! - contract-first-migration-plan-v0.27.0.md §5.4
//! - test-harness-maturation-plan-v0.27.0.md §10.3 — replicant-driven test proposals

use hkask_rsolidity as rs;
use hkask_storage::{Triple, TripleStore};
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use serde_json::json;

/// Entity key for the auto-created "Contract Violations" board triple.
const VIOLATIONS_BOARD_ENTITY: &str = "cns:contract_violations_board";
/// Entity key for contract violation task triples.
const VIOLATION_TASK_ENTITY: &str = "cns:contract_violation_task";

/// Error type for contract→kanban bridge operations.
#[derive(Debug, thiserror::Error)]
pub enum ContractBridgeError {
    #[error("triple store operation failed: {0}")]
    Storage(String),
}

/// Emit a `cns.contract.violated` span when a contracted function's test fails.
///
///
/// Called by CI or test infrastructure when a proptest with a `// REQ:` tag
/// fails. The span carries the function name, contract id, and failure
/// reason for algedonic routing.
///
/// # Arguments
/// - `sink` — the CNS event sink (from CyberneticsLoop or API)
/// - `function_name` — the fully-qualified function name (e.g., "energy::EnergyBudget::reserve")
/// - `contract_id` — the `P{N}-{domain}-{operation}` contract ID
/// - `failure_reason` — human-readable description of the contract violation
pub fn emit_contract_violated(
    sink: &dyn NuEventSink,
    function_name: &str,
    contract_id: &str,
    failure_reason: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractViolated), "violated");
    let event = NuEvent::new(
        WebID::from_persona(b"contract-discipline"),
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
/// - `expectation_completeness_pct` — percentage of contracted fns carrying `expect:` field (0.0–100.0, v0.28.0)
pub fn emit_contract_coverage(
    sink: &dyn NuEventSink,
    total_pub_fns: u64,
    contracted_fns: u64,
    coverage_pct: f64,
    expectation_completeness_pct: f64,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractCoverage), "measured");
    let event = NuEvent::new(
        WebID::from_persona(b"contract-discipline"),
        span,
        Phase::Sense,
        json!({
            "total_pub_fns": total_pub_fns,
            "contracted_fns": contracted_fns,
            "coverage_pct": coverage_pct,
            "expectation_completeness_pct": expectation_completeness_pct,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            total_pub_fns = total_pub_fns,
            contracted_fns = contracted_fns,
            coverage_pct = coverage_pct,
            expectation_completeness_pct = expectation_completeness_pct,
            error = %e,
            "Failed to persist contract coverage span"
        );
    }
}

// ── Contract violation → kanban task bridge ─────────────────────────────────

/// Ensure the "Contract Violations" board exists in the triple store.
///
/// Idempotent — subsequent calls with the same board_id are no-ops.
/// Uses a Mutex-guarded flag for in-process deduplication.
fn ensure_violations_board(store: &TripleStore, owner: WebID) -> Result<(), ContractBridgeError> {
    // Check if board already exists
    let existing = store
        .query_by_entity_attribute(VIOLATIONS_BOARD_ENTITY, "board")
        .map_err(|e| ContractBridgeError::Storage(e.to_string()))?;
    if !existing.is_empty() {
        return Ok(());
    }

    let board_value = json!({
        "name": "Contract Violations",
        "columns": [
            {"name": "Backlog", "status": "backlog", "position": 0},
            {"name": "In Progress", "status": "in_progress", "position": 1},
            {"name": "Resolved", "status": "done", "position": 2}
        ],
        "owner": owner.to_string(),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let board_triple = Triple::new(VIOLATIONS_BOARD_ENTITY, "board", board_value, owner);
    store
        .insert(&board_triple)
        .map_err(|e| ContractBridgeError::Storage(e.to_string()))?;

    Ok(())
}

/// Create a kanban task for a contract violation.
///
/// Persists a task triple using the canonical kanban scheme so it's queryable
/// through `KanbanService`. The task carries the contract ID, function name,
/// failure reason, optional counterexample, and owner WebID. Auto-creates a
/// "Contract Violations" board on first call.
///
/// # Returns
/// The task ID (UUID string) for the created task.
///
/// # Arguments
/// - `store` — the TripleStore for persistence
/// - `function_name` — fully-qualified function name (e.g., "energy::EnergyBudget::reserve")
/// - `contract_id` — the `// REQ:` spec_id (e.g., "CNS-001")
/// - `failure_reason` — human-readable description of the violation
/// - `counterexample` — optional JSON value capturing the failing input (e.g., proptest shrunk value)
/// - `owner` — WebID of the owner (usually the CNS system identity)
///
/// # Example
/// ```ignore
/// let task_id = create_contract_violation_task(
///     &store,
///     "wallet::balance",
///     "WAL-003",
///     "invariant violated: balance went negative",
///     None,
///     cns_webid,
/// ).unwrap();
/// ```
///
/// [P12] Constraining: Subscriber Consent — task owner is CNS system identity
pub fn create_contract_violation_task(
    store: &TripleStore,
    function_name: &str,
    contract_id: &str,
    failure_reason: &str,
    counterexample: Option<&serde_json::Value>,
    owner: WebID,
) -> Result<String, ContractBridgeError> {
    ensure_violations_board(store, owner)?;

    let task_id =
        WebID::from_persona(format!("contract-task:{function_name}").as_bytes()).to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let title = format!("CV: {contract_id} — {function_name}");
    let description = format!(
        "Contract `{contract_id}` violated in `{function_name}`.\n\nFailure: {failure_reason}\n\nAction: Write a regression test that captures this violation, then fix the implementation. Add `// REQ: {contract_id}` to both the contract and the regression test."
    );

    let mut origin = json!({
        "function": function_name,
        "contract_id": contract_id,
        "failure_reason": failure_reason,
    });
    if let Some(ce) = counterexample {
        origin["counterexample"] = ce.clone();
    }

    let task_value = json!({
        "id": task_id,
        "board_entity": VIOLATIONS_BOARD_ENTITY,
        "title": title,
        "description": description,
        "owner": owner.to_string(),
        "status": "backlog",
        "created_at": now,
        "labels": ["contract-violation", "cns-auto"],
        "priority": "high",
        "origin": origin,
    });

    let triple = Triple::new(VIOLATION_TASK_ENTITY, &task_id, task_value, owner);
    store
        .insert(&triple)
        .map_err(|e| ContractBridgeError::Storage(e.to_string()))?;

    tracing::info!(
        target: "cns.contract.bridge",
        task_id = %task_id,
        contract_id = %contract_id,
        function = %function_name,
        "Created kanban task for contract violation"
    );

    Ok(task_id)
}

/// Emit a CNS span AND create a kanban task for a contract violation.
///
/// This is the primary entry point for CI/test harness integration.
/// It combines the two independent concerns (observability + remediation)
/// into a single call so callers don't need to coordinate two APIs.
///
pub fn emit_contract_violated_with_task(
    sink: &dyn NuEventSink,
    store: &TripleStore,
    function_name: &str,
    contract_id: &str,
    failure_reason: &str,
    counterexample: Option<&serde_json::Value>,
) -> Result<String, ContractBridgeError> {
    // Ownership for violation tasks is CNS system identity.
    // In production this would be a configurable system WebID.
    let owner = WebID::from_persona(b"cns-contract-discipline");

    emit_contract_violated(sink, function_name, contract_id, failure_reason);
    create_contract_violation_task(
        store,
        function_name,
        contract_id,
        failure_reason,
        counterexample,
        owner,
    )
}

// ── Phase B2–B4 lifecycle spans (canonical CNS span registry) ──────────────

/// Emit `cns.contract.proposed` when a replicant proposes a contract.
///
/// Called during the Phase B2 workflow: agent analyzes function, proposes
/// a `// REQ:` contract + proptest, and opens a PR. This span records
/// the proposal event for CNS observability.
///
pub fn emit_contract_proposed(
    sink: &dyn NuEventSink,
    replicant_webid: &str,
    crate_name: &str,
    function_name: &str,
    contract_id: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractProposed), "proposed");
    let event = NuEvent::new(
        WebID::from_persona(replicant_webid.as_bytes()),
        span,
        Phase::Compute,
        json!({
            "replicant": replicant_webid,
            "crate": crate_name,
            "function": function_name,
            "contract_id": contract_id,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            replicant = replicant_webid,
            function = function_name,
            contract_id = contract_id,
            error = %e,
            "Failed to persist contract proposed span"
        );
    }
}

/// Emit `cns.contract.accepted` when a human approves and merges a contract proposal.
///
/// Called during the Phase B3 consent gate: human reviews the PR, approves it,
/// and the merge triggers this span. Closes the proposal→acceptance loop.
///
pub fn emit_contract_accepted(
    sink: &dyn NuEventSink,
    reviewer_webid: &str,
    replicant_webid: &str,
    crate_name: &str,
    function_name: &str,
    contract_id: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractAccepted), "accepted");
    let event = NuEvent::new(
        WebID::from_persona(reviewer_webid.as_bytes()),
        span,
        Phase::Act,
        json!({
            "reviewer": reviewer_webid,
            "replicant": replicant_webid,
            "crate": crate_name,
            "function": function_name,
            "contract_id": contract_id,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            reviewer = reviewer_webid,
            function = function_name,
            error = %e,
            "Failed to persist contract rejected span"
        );
    }
}

/// Emit `cns.contract.quality.violated` when a 4-layer contract quality check fails.
///
///
/// Called by the contract audit (`--contract-quality` flag) or the TDD verify step
/// when a contract is missing required layers (expect:, [P{N}], Constraining:).
/// Distinguished from `cns.contract.violated` (runtime test failures) — this is a
/// structural/process violation, not a code bug.
///
/// # Arguments
/// - `sink` — the CNS event sink
/// - `function_name` — the fully-qualified function name
/// - `contract_id` — the `P{N}-{domain}-{operation}` contract ID
/// - `violation_type` — one of: missing-expect, missing-goal-principle, missing-constraining, contract-id-mismatch
/// - `location` — file:line of the contract
/// - `description` — human-readable description of the violation
pub fn emit_contract_quality_violated(
    sink: &dyn NuEventSink,
    function_name: &str,
    contract_id: &str,
    violation_type: &str,
    location: &str,
    description: &str,
) {
    let span = Span::new(
        SpanNamespace::from(CnsSpan::ContractQualityViolated),
        "quality_violated",
    );
    let event = NuEvent::new(
        WebID::from_persona(b"contract-discipline"),
        span,
        Phase::Compare,
        json!({
            "function": function_name,
            "contract_id": contract_id,
            "violation_type": violation_type,
            "location": location,
            "description": description,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            function = function_name,
            contract_id = contract_id,
            violation_type = violation_type,
            error = %e,
            "Failed to persist contract quality violation span"
        );
    }
}

/// Emit `cns.contract.rejected` when a human rejects a contract proposal.
///
/// Called during the Phase B3 consent gate: human reviews the PR, rejects it
/// with rationale. The rejected contract is archived as a curation decision.
///
pub fn emit_contract_rejected(
    sink: &dyn NuEventSink,
    reviewer_webid: &str,
    replicant_webid: &str,
    crate_name: &str,
    function_name: &str,
    contract_id: &str,
    rationale: &str,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::ContractRejected), "rejected");
    let event = NuEvent::new(
        WebID::from_persona(reviewer_webid.as_bytes()),
        span,
        Phase::Act,
        json!({
            "reviewer": reviewer_webid,
            "replicant": replicant_webid,
            "crate": crate_name,
            "function": function_name,
            "contract_id": contract_id,
            "rationale": rationale,
        }),
        0,
    );
    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.contract",
            reviewer = reviewer_webid,
            function = function_name,
            error = %e,
            "Failed to persist contract rejected span"
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

    // contract: cns-contract-violation-event-001
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

    // contract: cns-contract-coverage-event-001
    #[test]
    fn emit_contract_coverage_persists_event() {
        let sink = CaptureSink::new();
        emit_contract_coverage(&sink, 1531, 55, 3.6, 87.3);

        let event = sink.last_event.lock().unwrap().clone().unwrap();
        let obs = &event.observation;
        assert_eq!(obs["total_pub_fns"], 1531);
        assert_eq!(obs["contracted_fns"], 55);
        assert!((obs["coverage_pct"].as_f64().unwrap() - 3.6).abs() < 0.01);
        assert!((obs["expectation_completeness_pct"].as_f64().unwrap() - 87.3).abs() < 0.01);
    }

    // ── Kanban bridge tests ──────────────────────────────────────────────

    fn test_store() -> TripleStore {
        let db = hkask_storage::in_memory_db();
        TripleStore::new(db.conn_arc())
    }

    fn test_webid() -> WebID {
        WebID::from_persona(b"test-cns")
    }

    #[test]
    fn create_violation_task_persists() {
        let store = test_store();
        let owner = test_webid();

        let task_id = create_contract_violation_task(
            &store,
            "wallet::balance",
            "WAL-003",
            "invariant violated: balance went negative",
            None,
            owner,
        )
        .unwrap();

        assert!(!task_id.is_empty(), "task_id should be non-empty UUID");

        // Verify task triple exists
        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 1);
        let task_val: serde_json::Value = tasks[0].value.clone();
        assert_eq!(task_val["id"], task_id);
        assert_eq!(task_val["status"], "backlog");
        assert_eq!(task_val["priority"], "high");
        assert_eq!(task_val["origin"]["contract_id"], "WAL-003");
        assert_eq!(task_val["origin"]["function"], "wallet::balance");
        assert!(task_val["title"].as_str().unwrap().contains("WAL-003"));
        assert!(
            task_val["description"]
                .as_str()
                .unwrap()
                .contains("invariant violated")
        );
    }

    // contract: CNS-CVB-001
    #[test]
    fn violations_board_is_created_once() {
        let store = test_store();
        let owner = test_webid();

        // First call creates board
        create_contract_violation_task(&store, "f1", "C1", "fail1", None, owner).unwrap();
        let boards = store.query_by_entity(VIOLATIONS_BOARD_ENTITY).unwrap();
        assert_eq!(boards.len(), 1);

        // Second call reuses existing board (no duplicate)
        create_contract_violation_task(&store, "f2", "C2", "fail2", None, owner).unwrap();
        let boards_after = store.query_by_entity(VIOLATIONS_BOARD_ENTITY).unwrap();
        assert_eq!(boards_after.len(), 1, "board should not be duplicated");
    }

    // contract: CNS-CVB-001
    #[test]
    fn each_violation_creates_distinct_task() {
        let store = test_store();
        let owner = test_webid();

        let id1 =
            create_contract_violation_task(&store, "a::foo", "REQ-1", "e1", None, owner).unwrap();
        let id2 =
            create_contract_violation_task(&store, "b::bar", "REQ-2", "e2", None, owner).unwrap();

        assert_ne!(id1, id2);
        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    // contract: CNS-CVB-001
    #[test]
    fn tasks_carry_distinct_origin() {
        let store = test_store();
        let owner = test_webid();

        create_contract_violation_task(&store, "mod::fn_a", "REQ-A", "bad input", None, owner)
            .unwrap();
        create_contract_violation_task(&store, "mod::fn_b", "REQ-B", "timeout", None, owner)
            .unwrap();

        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 2);

        let origins: Vec<&str> = tasks
            .iter()
            .map(|t| t.value["origin"]["contract_id"].as_str().unwrap())
            .collect();
        assert!(origins.contains(&"REQ-A"));
        assert!(origins.contains(&"REQ-B"));
    }

    // contract: CNS-CVB-001
    #[test]
    fn counterexample_persisted_in_task() {
        let store = test_store();
        let owner = test_webid();

        let counterexample = json!({"input": -1, "expected": "positive balance"});
        let task_id = create_contract_violation_task(
            &store,
            "wallet::deduct",
            "WAL-004",
            "post-condition violated: balance negative after deduct",
            Some(&counterexample),
            owner,
        )
        .unwrap();

        assert!(!task_id.is_empty());

        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 1);
        let task_val = &tasks[0].value;
        assert_eq!(task_val["origin"]["counterexample"], counterexample);
        assert_eq!(task_val["origin"]["contract_id"], "WAL-004");
        assert_eq!(task_val["origin"]["function"], "wallet::deduct");
    }

    // contract: CNS-CVB-001
    #[test]
    fn counterexample_absent_when_none() {
        let store = test_store();
        let owner = test_webid();

        create_contract_violation_task(
            &store,
            "wallet::deduct",
            "WAL-004",
            "post-condition violated",
            None,
            owner,
        )
        .unwrap();

        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 1);
        let task_val = &tasks[0].value;
        assert!(task_val["origin"].get("counterexample").is_none());
        assert_eq!(task_val["origin"]["contract_id"], "WAL-004");
    }

    // contract: CNS-CVB-002
    #[test]
    fn emit_and_task_creates_both() {
        let sink = CaptureSink::new();
        let store = test_store();

        let task_id = emit_contract_violated_with_task(
            &sink,
            &store,
            "crate::func",
            "CNS-XYZ",
            "test failure",
            None,
        )
        .unwrap();

        assert!(!task_id.is_empty());

        // Verify CNS span persisted
        let event = sink.last_event.lock().unwrap().clone().unwrap();
        assert_eq!(event.observation["contract_id"], "CNS-XYZ");

        // Verify task persisted
        let tasks = store.query_by_entity(VIOLATION_TASK_ENTITY).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].value["origin"]["contract_id"], "CNS-XYZ");
    }

    #[test]
    fn emit_contract_quality_violated_persists_event() {
        let sink = CaptureSink::new();
        emit_contract_quality_violated(
            &sink,
            "energy::EnergyBudget::can_proceed",
            "P9-cns-energy-budget-can-proceed",
            "missing-expect",
            "crates/hkask-cns/src/energy.rs:42",
            "Contract missing expect: field — user expectation not captured",
        );
        let event = sink.last_event.lock().unwrap().clone().unwrap();
        assert_eq!(event.observation["violation_type"], "missing-expect");
        assert_eq!(
            event.observation["contract_id"],
            "P9-cns-energy-budget-can-proceed"
        );
        assert!(
            event.observation["description"]
                .as_str()
                .unwrap()
                .contains("user expectation")
        );
    }
}
