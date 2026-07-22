//! Contract tests for hkask-mcp-kanban — KanbanService behavioral contracts.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → Pre/Post → Test`
//!
//! Tested through the KanbanService public API (the core logic seam),
//! using `TestDb` for isolated in-memory storage.

use hkask_services_kata_kanban::{KanbanError, KanbanService};
use hkask_services_kata_kanban::{TaskFilter, TaskSpec, TaskStatus, VerificationCriterion};
use hkask_storage::HMemStore;
use hkask_test_harness::TestWebId;
use hkask_types::WebID;
use proptest::prelude::*;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

fn setup() -> (KanbanService, WebID) {
    let driver = hkask_storage::database::sqlite::SqliteDriver::in_memory_driver();
    let store = HMemStore::from_driver(driver);
    let service = KanbanService::new(store);
    (service, TestWebId::alice())
}

fn default_columns() -> Vec<hkask_services_kata_kanban::ColumnDef> {
    hkask_mcp_kata_kanban::default_columns()
}

// ── Board CRUD contract tests ──────────────────────────────────────────────

// [P1] Constraining: board is owned by a user WebID — sovereignty boundary
#[test]
fn board_create_list_get_delete() {
    let (svc, owner) = setup();

    let board = svc
        .board_create(owner, "Test Board", &default_columns())
        .expect("board_create should succeed");
    assert_eq!(board.name, "Test Board");
    assert_eq!(board.columns.len(), 5);

    let list = svc.board_list(&owner).expect("board_list should succeed");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Test Board");

    let fetched = svc
        .board_get(board.id)
        .expect("board_get should succeed")
        .expect("board should exist");
    assert_eq!(fetched.name, "Test Board");

    let deleted = svc
        .board_delete(board.id)
        .expect("board_delete should succeed");
    assert_eq!(deleted, 0); // board had no tasks

    let after = svc
        .board_list(&owner)
        .expect("board_list after delete should succeed");
    assert!(after.is_empty());
}

#[test]
fn board_create_rejects_empty_name() {
    let (svc, owner) = setup();
    let err = svc
        .board_create(owner, "", &default_columns())
        .expect_err("should reject empty name");
    assert!(matches!(err, KanbanError::InvalidInput(_)));
}

#[test]
fn board_create_rejects_empty_columns() {
    let (svc, owner) = setup();
    let err = svc
        .board_create(owner, "No Cols", &[])
        .expect_err("should reject empty columns");
    assert!(matches!(err, KanbanError::InvalidInput(_)));
}

// ── Task lifecycle contract tests ───────────────────────────────────────────

// [P12] Constraining: every task carries the creator's WebID — no anonymous agency
#[test]
fn task_create_list_get() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Project", &default_columns())
        .expect("board_create");

    let spec = TaskSpec::new("Implement login".to_string())
        .with_description("OAuth2 login flow".to_string())
        .with_criteria(vec![
            VerificationCriterion::new("Redirect to provider".to_string()),
            VerificationCriterion::new("Handle callback".to_string()),
        ]);
    let task = svc
        .task_create(board.id, spec, owner)
        .expect("task_create should succeed");
    assert_eq!(task.title, "Implement login");
    assert_eq!(task.status, TaskStatus::Backlog);

    let tasks = svc
        .task_list(board.id, TaskFilter::all())
        .expect("task_list should succeed");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Implement login");

    let fetched = svc
        .task_get(task.id)
        .expect("task_get should succeed")
        .expect("task should exist");
    assert_eq!(fetched.criteria.len(), 2);
}

#[test]
fn task_list_filters_by_status() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Sprint", &default_columns())
        .expect("board_create");

    svc.task_create(board.id, TaskSpec::new("Feature A".to_string()), owner)
        .expect("task 1");
    svc.task_create(board.id, TaskSpec::new("Feature B".to_string()), owner)
        .expect("task 2");

    let backlogs = svc
        .task_list(board.id, TaskFilter::by_status(TaskStatus::Backlog))
        .expect("task_list backlog");
    assert_eq!(backlogs.len(), 2);

    let in_progress = svc
        .task_list(board.id, TaskFilter::by_status(TaskStatus::InProgress))
        .expect("task_list in_progress");
    assert!(in_progress.is_empty());
}

#[test]
fn task_move_transitions() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Workflow", &default_columns())
        .expect("board_create");

    let task = svc
        .task_create(
            board.id,
            TaskSpec::new("Refactor module".to_string()),
            owner,
        )
        .expect("task_create");
    assert_eq!(task.status, TaskStatus::Backlog);

    let moved = svc
        .task_move(task.id, TaskStatus::Ready, owner)
        .expect("move to ready");
    assert_eq!(moved.status, TaskStatus::Ready);

    let moved = svc
        .task_move(moved.id, TaskStatus::InProgress, owner)
        .expect("move to in-progress");
    assert_eq!(moved.status, TaskStatus::InProgress);

    let moved = svc
        .task_move(moved.id, TaskStatus::Review, owner)
        .expect("move to review");
    assert_eq!(moved.status, TaskStatus::Review);

    let moved = svc
        .task_move(moved.id, TaskStatus::Done, owner)
        .expect("move to done");
    assert_eq!(moved.status, TaskStatus::Done);
}

#[test]
fn task_move_rejects_backward_transition() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "WF", &default_columns())
        .expect("board_create");

    let task = svc
        .task_create(board.id, TaskSpec::new("Done early".to_string()), owner)
        .expect("task_create");

    let moved = svc
        .task_move(task.id, TaskStatus::Ready, owner)
        .expect("move to ready");

    let moved = svc
        .task_move(moved.id, TaskStatus::InProgress, owner)
        .expect("move to in-progress");

    let moved = svc
        .task_move(moved.id, TaskStatus::Review, owner)
        .expect("move to review");

    let moved = svc
        .task_move(moved.id, TaskStatus::Done, owner)
        .expect("move to done");

    // Done has no reverse — cannot go backward
    let err = svc
        .task_move(moved.id, TaskStatus::Review, owner)
        .expect_err("should reject backward move from Done");
    assert!(matches!(err, KanbanError::InvalidTransition { .. }));
}

#[test]
fn task_move_nonexistent_returns_not_found() {
    let (svc, _owner) = setup();
    let fake_id = hkask_types::TaskId::new();
    let err = svc
        .task_move(fake_id, TaskStatus::InProgress, TestWebId::bob())
        .expect_err("should return not found");
    assert!(matches!(err, KanbanError::NotFound(_)));
}

// [P1] Goal: User Sovereignty — consent-gated assignment
// [P12] Constraining: both assigner and assignee carry authenticated WebIDs
#[test]
fn task_claim_records_assignee() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Team", &default_columns())
        .expect("board_create");

    let task = svc
        .task_create(board.id, TaskSpec::new("Write tests".to_string()), owner)
        .expect("task_create");

    let agent = TestWebId::bob();
    let assigned = svc
        .task_claim(task.id, agent)
        .expect("task_claim should succeed");
    assert!(assigned.assignee.is_some());

    let fetched = svc
        .task_get(task.id)
        .expect("task_get")
        .expect("task should exist");
    assert!(fetched.assignee.is_some());
}

#[test]
fn task_verify_passes_on_evidence() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Verify Board", &default_columns())
        .expect("board_create");

    let spec = TaskSpec::new("Add rate limiting".to_string()).with_criteria(vec![
        VerificationCriterion::new("Rate limit per user".to_string()),
        VerificationCriterion::new("429 responses documented".to_string()),
    ]);
    let task = svc.task_create(board.id, spec, owner).expect("task_create");

    // Move to Review (verification requires Review status)
    let task = svc
        .task_move(task.id, TaskStatus::Ready, owner)
        .expect("to ready");
    let task = svc
        .task_move(task.id, TaskStatus::InProgress, owner)
        .expect("to in-progress");
    let task = svc
        .task_move(task.id, TaskStatus::Review, owner)
        .expect("to review");

    let (verified_task, v) = svc
        .task_verify(
            task.id,
            "All criteria met: rate limiting implemented, 429 docs added",
            owner,
        )
        .expect("task_verify should succeed");
    assert!(v.passed);
    assert!(!v.reasoning.is_empty());
    assert_eq!(verified_task.status, TaskStatus::Done);
}

#[test]
fn task_delete_removes() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Cleanup", &default_columns())
        .expect("board_create");

    let task = svc
        .task_create(
            board.id,
            TaskSpec::new("Remove old code".to_string()),
            owner,
        )
        .expect("task_create");

    svc.task_delete(task.id)
        .expect("task_delete should succeed");

    let tasks = svc
        .task_list(board.id, TaskFilter::all())
        .expect("task_list after delete");
    assert!(tasks.is_empty());
}

// ── Property-based: task lifecycle invariant ────────────────────────────────

proptest! {
    #[test]
    fn task_board_invariant_holds(
        (title, desc_len) in (
            "[A-Za-z ]{3,50}",
            0usize..200usize,
        )
    ) {
        let (svc, owner) = setup();
        let board = svc
            .board_create(owner, "Inv", &default_columns())
            .expect("board_create");

        let desc = if desc_len > 0 {
            format!("Description of length {}", desc_len)
        } else {
            String::new()
        };
        let mut spec = TaskSpec::new(title);
        if !desc.is_empty() {
            spec = spec.with_description(desc);
        }
        let task = svc
            .task_create(board.id, spec, owner)
            .expect("task_create");
        prop_assert_eq!(task.board_id, board.id);
        prop_assert_eq!(task.status, TaskStatus::Backlog);

        let fetched = svc.task_get(task.id).expect("get").expect("exists");
        prop_assert_eq!(fetched.board_id, board.id);
    }
}

// ── MCP response fidelity tests (exercises the full KanbanServer tool path) ───

/// Construct a KanbanServer backed by an in-memory database for testing.
fn test_mcp_server() -> hkask_mcp_kata_kanban::KanbanServer {
    let driver = hkask_storage::database::sqlite::SqliteDriver::in_memory_driver();
    let store = hkask_storage::HMemStore::from_driver(Arc::clone(&driver));
    let service = hkask_services_kata_kanban::KanbanService::new(store);
    hkask_mcp_kata_kanban::KanbanServer::new(
        TestWebId::alice(),
        "test-userpod".into(),
        None,
        service,
    )
}

/// Call an async tool method and return the JSON string output.
fn call_tool_output<F: std::future::Future<Output = String>>(f: F) -> String {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(f)
}

/// Parse a tool output string into `{"content": [...], "error": ..., "kind": ...}`.
fn parse_tool_output(output: &str) -> serde_json::Value {
    serde_json::from_str(output).expect("tool output must be valid JSON")
}

#[test]
fn mcp_task_move_reports_correct_previous_status() {
    use hkask_mcp_kata_kanban::types::*;

    let server = test_mcp_server();

    // Create board
    let board_json = call_tool_output(server.kanban_board_create(Parameters(BoardCreateRequest {
        name: "Fidelity Test".into(),
        columns: None,
    })));
    let board_val = parse_tool_output(&board_json);
    // McpToolOutput wraps response as {"content": <value>}
    let board_data = &board_val["content"];
    let board_id = board_data["board_id"]
        .as_str()
        .map(String::from)
        .expect("board_id must be present");

    // Create task
    let task_json = call_tool_output(server.kanban_task_create(Parameters(TaskCreateRequest {
        board_id: board_id.clone(),
        title: "Test Move Fidelity".into(),
        description: None,
        criteria: None,
        gas_budget: None,
        rjoule_budget: None,
    })));
    let task_val = parse_tool_output(&task_json);
    let task_id = task_val["content"]["task_id"]
        .as_str()
        .map(String::from)
        .expect("task_id must be present");

    // Move task from Backlog → Ready
    let move_json = call_tool_output(server.kanban_task_move(Parameters(TaskMoveRequest {
        task_id: task_id.clone(),
        target_status: "Ready".into(),
    })));
    let move_val = parse_tool_output(&move_json);
    let move_data = &move_val["content"];

    let previous = move_data["previous_status"]
        .as_str()
        .expect("previous_status must be present");
    let new_status = move_data["new_status"]
        .as_str()
        .expect("new_status must be present");

    assert_eq!(
        previous, "backlog",
        "previous_status should be the status before the move"
    );
    assert_eq!(
        new_status, "ready",
        "new_status should be the target status"
    );
    assert_ne!(
        previous, new_status,
        "previous_status must differ from new_status"
    );
}

#[test]
fn mcp_not_found_error_uses_not_found_kind() {
    use hkask_mcp_kata_kanban::types::*;

    let server = test_mcp_server();

    // Try to move a non-existent task — valid UUID format, won't exist
    let output = call_tool_output(server.kanban_task_move(Parameters(TaskMoveRequest {
        task_id: "00000000-0000-0000-0000-000000000000".into(),
        target_status: "ready".into(),
    })));
    let val = parse_tool_output(&output);

    // Error responses from finish() use McpToolError::to_json_string() format.
    // Success is {"content": <value>}; error is {"error": "...", "kind": "..."}
    let error_kind = val["kind"].as_str();
    assert!(
        error_kind == Some("not_found"),
        "not_found error should use 'not_found' kind, got: {error_kind:?}. Full response: {val}"
    );
    assert!(
        val.get("error").is_some(),
        "not_found should have an error field"
    );
}

#[test]
fn mcp_task_verify_rejects_empty_evidence() {
    use hkask_mcp_kata_kanban::types::*;

    let server = test_mcp_server();

    // Create board + task + move to Review first
    let board_json = call_tool_output(server.kanban_board_create(Parameters(BoardCreateRequest {
        name: "Verify Test".into(),
        columns: None,
    })));
    let board_val = parse_tool_output(&board_json);
    let board_id = board_val["content"]["board_id"]
        .as_str()
        .map(String::from)
        .expect("board_id");

    let task_json = call_tool_output(server.kanban_task_create(Parameters(TaskCreateRequest {
        board_id: board_id.clone(),
        title: "Empty Evidence Test".into(),
        description: None,
        criteria: Some(vec!["criterion 1".into()]),
        gas_budget: None,
        rjoule_budget: None,
    })));
    let task_val = parse_tool_output(&task_json);
    let task_id = task_val["content"]["task_id"]
        .as_str()
        .map(String::from)
        .expect("task_id");

    // Move to Review
    for status in &["Ready", "InProgress", "Review"] {
        call_tool_output(server.kanban_task_move(Parameters(TaskMoveRequest {
            task_id: task_id.clone(),
            target_status: status.to_string(),
        })));
    }

    // Verify with empty evidence should be rejected
    let empty_output = call_tool_output(server.kanban_task_verify(Parameters(TaskVerifyRequest {
        task_id: task_id.clone(),
        evidence: String::new(),
    })));
    let empty_val = parse_tool_output(&empty_output);
    assert!(
        empty_val.get("error").is_some(),
        "empty evidence should produce an error. Got: {empty_val}"
    );

    // Whitespace-only evidence should also be rejected
    let ws_output = call_tool_output(server.kanban_task_verify(Parameters(TaskVerifyRequest {
        task_id,
        evidence: "   ".into(),
    })));
    let ws_val = parse_tool_output(&ws_output);
    assert!(
        ws_val.get("error").is_some(),
        "whitespace-only evidence should produce an error. Got: {ws_val}"
    );
}

#[test]
fn mcp_board_create_supports_wip_limit() {
    use hkask_mcp_kata_kanban::types::*;

    let server = test_mcp_server();

    let board_json = call_tool_output(server.kanban_board_create(Parameters(BoardCreateRequest {
        name: "WIP Test".into(),
        columns: Some(vec![ColumnDefInput {
            name: "Todo".into(),
            status: "Backlog".into(),
            wip_limit: Some(3),
        }]),
    })));
    let board_val = parse_tool_output(&board_json);
    let board_data = &board_val["content"];

    // The response should include the board with columns
    let board_id = board_data["board_id"].as_str().expect("board_id");
    assert!(!board_id.is_empty());

    // Verify the column was created (WIP limit is internal, not in response)
    let columns = board_data["columns"]
        .as_array()
        .expect("columns must be array");
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0]["name"].as_str().unwrap(), "Todo");
}
