//! Contract tests for hkask-mcp-kanban — KanbanService behavioral contracts.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → Pre/Post → Test`
//!
//! Tested through the KanbanService public API (the core logic seam),
//! using `TestDb` for isolated in-memory storage.

use hkask_services_kanban::{KanbanError, KanbanService};
use hkask_storage::TripleStore;
use hkask_test_harness::{TestDb, TestWebId};
use hkask_types::{ConsentProof, TaskFilter, TaskSpec, TaskStatus, VerificationCriterion, WebID};
use proptest::prelude::*;

fn setup() -> (KanbanService, WebID) {
    let db = TestDb::new();
    let store = TripleStore::new(db.conn_arc());
    let service = KanbanService::new(store);
    (service, TestWebId::alice())
}

fn default_columns() -> Vec<hkask_types::ColumnDef> {
    vec![
        hkask_types::ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0),
        hkask_types::ColumnDef::new("Ready".into(), TaskStatus::Ready, 1),
        hkask_types::ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2),
        hkask_types::ColumnDef::new("Review".into(), TaskStatus::Review, 3),
        hkask_types::ColumnDef::new("Done".into(), TaskStatus::Done, 4),
    ]
}

// ── Board CRUD contract tests ──────────────────────────────────────────────

// contract: P3-svc-kanban-002
// expect: "I can create a kanban board with a name and columns, and it persists" [P3]
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

// contract: P3-svc-kanban-002
// expect: "I get a clear error when I try to create a board with no name" [P3]
#[test]
fn board_create_rejects_empty_name() {
    let (svc, owner) = setup();
    let err = svc
        .board_create(owner, "", &default_columns())
        .expect_err("should reject empty name");
    assert!(matches!(err, KanbanError::InvalidInput(_)));
}

// contract: P3-svc-kanban-002
// expect: "I get a clear error when I try to create a board with no columns" [P3]
#[test]
fn board_create_rejects_empty_columns() {
    let (svc, owner) = setup();
    let err = svc
        .board_create(owner, "No Cols", &[])
        .expect_err("should reject empty columns");
    assert!(matches!(err, KanbanError::InvalidInput(_)));
}

// ── Task lifecycle contract tests ───────────────────────────────────────────

// contract: P3-svc-kanban-003
// expect: "I can create tasks on a board with title, description, and criteria" [P3]
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

// contract: P3-svc-kanban-003
// expect: "I can filter tasks by their workflow status — backlog, in progress, done" [P3]
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

// contract: P3-svc-kanban-004
// expect: "I can move a task through the workflow — backlog to ready to in-progress to review to done" [P3]
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

// contract: P3-svc-kanban-004
// expect: "I get a clear error when I try to move a task backward from Done — transitions are final" [P3]
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

// contract: P3-svc-kanban-004
// expect: "I get a not-found error when I try to move a task that doesn't exist" [P3]
#[test]
fn task_move_nonexistent_returns_not_found() {
    let (svc, _owner) = setup();
    let fake_id = hkask_types::TaskId::new();
    let err = svc
        .task_move(fake_id, TaskStatus::InProgress, TestWebId::bob())
        .expect_err("should return not found");
    assert!(matches!(err, KanbanError::NotFound(_)));
}

// contract: P3-svc-kanban-005
// expect: "I can assign a task to an agent with explicit consent proof" [P1]
// [P1] Goal: User Sovereignty — consent-gated assignment
// [P12] Constraining: both assigner and assignee carry authenticated WebIDs
#[test]
fn task_assign_with_consent() {
    let (svc, owner) = setup();
    let board = svc
        .board_create(owner, "Team", &default_columns())
        .expect("board_create");

    let task = svc
        .task_create(board.id, TaskSpec::new("Write tests".to_string()), owner)
        .expect("task_create");

    let agent = TestWebId::bob();
    let consent = ConsentProof::new(agent, task.id);
    let assigned = svc
        .task_assign(task.id, agent, consent)
        .expect("task_assign should succeed");
    assert!(assigned.assignee.is_some());

    let fetched = svc
        .task_get(task.id)
        .expect("task_get")
        .expect("task should exist");
    assert!(fetched.assignee.is_some());
}

// contract: P3-svc-kanban-006
// expect: "I can verify a task in Review against its acceptance criteria and see whether it passed" [P3]
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

// contract: P3-svc-kanban-007
// expect: "I can delete a task and it no longer appears in task lists" [P3]
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

// contract: P3-svc-kanban-008
// expect: "I know that every task I create belongs to exactly one board — this invariant always holds" [P3]
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
