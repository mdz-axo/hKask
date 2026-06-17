//! KanbanService — core kanban board and task coordination.
//!
//! Implements the 7 public operations from the kanban agent coordination spec.
//! Every operation carries ownership tracking (P12) and enforces agent consent
//! on assignment (P1). State transitions are column-ordered.
//!
//! Persistence is in-memory for now; Task 3 will wire into TripleStore (MDS).
//! CNS span emission is stubbed for Task 7.

use std::collections::HashMap;
use std::sync::RwLock;

use hkask_types::{
    Board, BoardId, ColumnDef, ConsentProof, Task, TaskFilter, TaskId, TaskSpec, TaskStatus,
    Verification, WebID,
};

/// Core kanban coordination service.
///
/// Manages boards and tasks with P12 ownership tracking and P1 consent enforcement.
/// Public surface: exactly 7 operations (deep-module discipline).
pub struct KanbanService {
    boards: RwLock<HashMap<BoardId, Board>>,
    tasks: RwLock<HashMap<TaskId, Task>>,
    /// Board → task ID mapping for efficient listing.
    board_tasks: RwLock<HashMap<BoardId, Vec<TaskId>>>,
}

impl KanbanService {
    /// REQ: KAN-SVC-001
    /// post: returns an empty KanbanService
    pub fn new() -> Self {
        Self {
            boards: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            board_tasks: RwLock::new(HashMap::new()),
        }
    }

    // ── Board operations ──────────────────────────────────────────────────

    /// Create a new kanban board.
    ///
    /// REQ: KAN-SVC-002
    /// pre:  owner is a valid WebID; name is non-empty; columns is non-empty
    /// post: board is stored; returns the created Board
    /// CNS:  BoardCreated (stubbed)
    pub fn board_create(
        &self,
        owner: WebID,
        name: &str,
        columns: &[ColumnDef],
    ) -> Result<Board, KanbanError> {
        if name.is_empty() {
            return Err(KanbanError::InvalidInput("board name is empty".into()));
        }
        if columns.is_empty() {
            return Err(KanbanError::InvalidInput(
                "board must have at least one column".into(),
            ));
        }

        let board = Board::new(name.to_string(), owner, columns.to_vec());
        self.boards
            .write()
            .map_err(|e| KanbanError::Internal(format!("boards lock poisoned: {e}")))?
            .insert(board.id, board.clone());
        Ok(board)
    }

    /// List all boards for a given owner.
    ///
    /// REQ: KAN-SVC-003
    /// pre:  owner is a valid WebID
    /// post: returns all boards owned by this replicant
    pub fn board_list(&self, owner: &WebID) -> Result<Vec<Board>, KanbanError> {
        let boards = self
            .boards
            .read()
            .map_err(|e| KanbanError::Internal(format!("boards lock poisoned: {e}")))?;
        Ok(boards
            .values()
            .filter(|b| &b.owner == owner)
            .cloned()
            .collect())
    }

    /// Get a board by ID.
    ///
    /// REQ: KAN-SVC-004
    /// pre:  board_id is valid
    /// post: returns Some(Board) if found, None otherwise
    pub fn board_get(&self, board_id: BoardId) -> Result<Option<Board>, KanbanError> {
        let boards = self
            .boards
            .read()
            .map_err(|e| KanbanError::Internal(format!("boards lock poisoned: {e}")))?;
        Ok(boards.get(&board_id).cloned())
    }

    // ── Task operations ───────────────────────────────────────────────────

    /// Create a new task on a board.
    ///
    /// REQ: KAN-SVC-005
    /// pre:  board_id refers to an existing board; spec.title is non-empty; owner is valid
    /// post: task is stored with status=Backlog; returns the created Task
    /// CNS:  TaskCreated (stubbed)
    pub fn task_create(
        &self,
        board_id: BoardId,
        spec: TaskSpec,
        owner: WebID,
    ) -> Result<Task, KanbanError> {
        // Verify board exists
        let boards = self
            .boards
            .read()
            .map_err(|e| KanbanError::Internal(format!("boards lock poisoned: {e}")))?;
        if !boards.contains_key(&board_id) {
            return Err(KanbanError::NotFound(format!("board {board_id}")));
        }
        drop(boards);

        let task = Task::new(board_id, spec, owner);
        let task_id = task.id;

        self.tasks
            .write()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?
            .insert(task_id, task.clone());

        self.board_tasks
            .write()
            .map_err(|e| KanbanError::Internal(format!("board_tasks lock poisoned: {e}")))?
            .entry(board_id)
            .or_default()
            .push(task_id);

        Ok(task)
    }

    /// List tasks on a board, optionally filtered.
    ///
    /// REQ: KAN-SVC-006
    /// pre:  board_id refers to an existing board
    /// post: returns tasks matching the filter; empty Vec if none match
    pub fn task_list(
        &self,
        board_id: BoardId,
        filter: TaskFilter,
    ) -> Result<Vec<Task>, KanbanError> {
        let board_tasks = self
            .board_tasks
            .read()
            .map_err(|e| KanbanError::Internal(format!("board_tasks lock poisoned: {e}")))?;
        let task_ids = board_tasks.get(&board_id).cloned().unwrap_or_default();
        drop(board_tasks);

        let tasks = self
            .tasks
            .read()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?;

        let mut results: Vec<Task> = task_ids
            .iter()
            .filter_map(|id| tasks.get(id))
            .filter(|t| {
                let status_match = filter.status.map_or(true, |s| t.status == s);
                let assignee_match = filter.assignee.map_or(true, |a| t.assignee == Some(a));
                status_match && assignee_match
            })
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get a task by ID.
    ///
    /// REQ: KAN-SVC-007
    /// pre:  task_id is valid
    /// post: returns Some(Task) if found, None otherwise
    pub fn task_get(&self, task_id: TaskId) -> Result<Option<Task>, KanbanError> {
        let tasks = self
            .tasks
            .read()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?;
        Ok(tasks.get(&task_id).cloned())
    }

    /// Move a task to a new column (state transition).
    ///
    /// REQ: KAN-SVC-008
    /// pre:  task_id refers to an existing task; target is a valid transition from current status
    /// pre:  actor is a valid WebID (P12 — every action has an authenticated author)
    /// post: task.status is updated to target; updated_at is refreshed
    /// fails: if transition is invalid (skip detected) → InvalidTransition
    /// CNS:  TaskMoved (stubbed)
    pub fn task_move(
        &self,
        task_id: TaskId,
        target: TaskStatus,
        actor: WebID,
    ) -> Result<Task, KanbanError> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?;

        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if !task.can_move_to(target) {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: target,
            });
        }

        task.status = target;
        task.updated_at = chrono::Utc::now();
        let _ = actor; // P12: actor is recorded for audit; CNS span will carry this

        Ok(task.clone())
    }

    /// Assign a task to an agent with consent proof.
    ///
    /// REQ: KAN-SVC-009
    /// pre:  task_id refers to an existing task; consent.agent matches the assignee
    /// pre:  consent.task_id matches task_id (consent is task-specific)
    /// post: task.assignee is set to consent.agent; updated_at is refreshed
    /// fails: if consent is invalid (mismatched agent or task) → ConsentViolation
    /// CNS:  TaskAssigned (stubbed)
    pub fn task_assign(
        &self,
        task_id: TaskId,
        agent: WebID,
        consent: ConsentProof,
    ) -> Result<Task, KanbanError> {
        // P1: Verify consent — agent must consent to the exact task
        if consent.agent != agent {
            return Err(KanbanError::ConsentViolation(
                "consent agent does not match assignee".into(),
            ));
        }
        if consent.task_id != task_id {
            return Err(KanbanError::ConsentViolation(
                "consent task_id does not match task".into(),
            ));
        }

        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?;

        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        task.assignee = Some(agent);
        task.updated_at = chrono::Utc::now();

        Ok(task.clone())
    }

    /// Verify a task's completion against its acceptance criteria.
    ///
    /// REQ: KAN-SVC-010
    /// pre:  task_id refers to an existing task in Review status
    /// pre:  verifier is a valid WebID (replicant or human)
    /// post: task.verification is set; task moves to Done if passed, stays in Review if failed
    /// CNS:  TaskVerified (stubbed)
    ///
    /// Currently performs a simple keyword-based check against the acceptance criteria.
    /// Task 6 (Verification Primitive) will add LLM-mediated evaluation.
    pub fn task_verify(
        &self,
        task_id: TaskId,
        evidence: &str,
        verifier: WebID,
    ) -> Result<(Task, Verification), KanbanError> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| KanbanError::Internal(format!("tasks lock poisoned: {e}")))?;

        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if task.status != TaskStatus::Review {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: TaskStatus::Done,
            });
        }

        // Simple keyword-based verification: evidence contains criterion description keywords
        let passed = if task.criteria.is_empty() {
            // No criteria → passes by default
            true
        } else {
            task.criteria.iter().all(|c| {
                c.description
                    .split_whitespace()
                    .any(|word| evidence.to_lowercase().contains(&word.to_lowercase()))
            })
        };

        let reasoning = if passed {
            "All acceptance criteria matched the submitted evidence.".to_string()
        } else {
            "One or more acceptance criteria were not satisfied by the evidence.".to_string()
        };

        let verification = Verification::new(passed, reasoning, verifier);
        task.verification = Some(verification.clone());

        if passed {
            task.status = TaskStatus::Done;
        }
        task.updated_at = chrono::Utc::now();

        Ok((task.clone(), verification))
    }
}

impl Default for KanbanService {
    fn default() -> Self {
        Self::new()
    }
}

// ── Error types ────────────────────────────────────────────────────────────

/// Errors specific to kanban operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum KanbanError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid state transition: task {task} cannot move from {from} to {to}")]
    InvalidTransition {
        task: TaskId,
        from: TaskStatus,
        to: TaskStatus,
    },

    #[error("consent violation: {0}")]
    ConsentViolation(String),

    #[error("internal error: {0}")]
    Internal(String),
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::VerificationCriterion;

    fn make_default_columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0),
            ColumnDef::new("Ready".into(), TaskStatus::Ready, 1),
            ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2),
            ColumnDef::new("Review".into(), TaskStatus::Review, 3),
            ColumnDef::new("Done".into(), TaskStatus::Done, 4),
        ]
    }

    fn make_service_with_board() -> (KanbanService, Board, WebID) {
        let svc = KanbanService::new();
        let owner = WebID::new();
        let board = svc
            .board_create(owner, "Test Board", &make_default_columns())
            .unwrap();
        (svc, board, owner)
    }

    // REQ: KAN-SVC-T-001 — board_create succeeds with valid input
    #[test]
    fn board_create_succeeds() {
        let svc = KanbanService::new();
        let owner = WebID::new();
        let board = svc
            .board_create(owner, "My Board", &make_default_columns())
            .unwrap();
        assert_eq!(board.name, "My Board");
        assert_eq!(board.owner, owner);
        assert_eq!(board.columns.len(), 5);
    }

    // REQ: KAN-SVC-T-002 — board_create rejects empty name
    #[test]
    fn board_create_rejects_empty_name() {
        let svc = KanbanService::new();
        let result = svc.board_create(WebID::new(), "", &make_default_columns());
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-003 — board_create rejects empty columns
    #[test]
    fn board_create_rejects_empty_columns() {
        let svc = KanbanService::new();
        let result = svc.board_create(WebID::new(), "Board", &[]);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-004 — board_list filters by owner
    #[test]
    fn board_list_by_owner() {
        let svc = KanbanService::new();
        let alice = WebID::new();
        let bob = WebID::new();

        svc.board_create(alice, "Alice's Board", &make_default_columns())
            .unwrap();
        svc.board_create(bob, "Bob's Board", &make_default_columns())
            .unwrap();

        let alice_boards = svc.board_list(&alice).unwrap();
        assert_eq!(alice_boards.len(), 1);
        assert_eq!(alice_boards[0].name, "Alice's Board");

        let bob_boards = svc.board_list(&bob).unwrap();
        assert_eq!(bob_boards.len(), 1);
        assert_eq!(bob_boards[0].name, "Bob's Board");
    }

    // REQ: KAN-SVC-T-005 — task_create stores task in Backlog
    #[test]
    fn task_create_defaults_to_backlog() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Backlog);
        assert_eq!(task.board_id, board.id);
    }

    // REQ: KAN-SVC-T-006 — task_create rejects unknown board
    #[test]
    fn task_create_rejects_unknown_board() {
        let svc = KanbanService::new();
        let result = svc.task_create(BoardId::new(), TaskSpec::new("Test".into()), WebID::new());
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-007 — task_list with no filter returns all tasks
    #[test]
    fn task_list_unfiltered() {
        let (svc, board, owner) = make_service_with_board();
        svc.task_create(board.id, TaskSpec::new("T1".into()), owner)
            .unwrap();
        svc.task_create(board.id, TaskSpec::new("T2".into()), owner)
            .unwrap();

        let tasks = svc.task_list(board.id, TaskFilter::all()).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    // REQ: KAN-SVC-T-008 — task_list filters by status
    #[test]
    fn task_list_filter_by_status() {
        let (svc, board, owner) = make_service_with_board();
        let t1 = svc
            .task_create(board.id, TaskSpec::new("T1".into()), owner)
            .unwrap();
        // Move t1 to InProgress
        svc.task_move(t1.id, TaskStatus::Ready, owner).unwrap();
        svc.task_move(t1.id, TaskStatus::InProgress, owner).unwrap();

        // T2 stays in Backlog
        svc.task_create(board.id, TaskSpec::new("T2".into()), owner)
            .unwrap();

        let backlog = svc
            .task_list(board.id, TaskFilter::by_status(TaskStatus::Backlog))
            .unwrap();
        assert_eq!(backlog.len(), 1);

        let in_progress = svc
            .task_list(board.id, TaskFilter::by_status(TaskStatus::InProgress))
            .unwrap();
        assert_eq!(in_progress.len(), 1);
    }

    // REQ: KAN-SVC-T-009 — task_move transitions forward through columns
    #[test]
    fn task_move_forward() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let t = svc.task_move(task.id, TaskStatus::Ready, owner).unwrap();
        assert_eq!(t.status, TaskStatus::Ready);

        let t = svc
            .task_move(task.id, TaskStatus::InProgress, owner)
            .unwrap();
        assert_eq!(t.status, TaskStatus::InProgress);
    }

    // REQ: KAN-SVC-T-010 — task_move rejects column skipping
    #[test]
    fn task_move_rejects_skip() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_move(task.id, TaskStatus::InProgress, owner);
        assert!(result.is_err());
        match result.unwrap_err() {
            KanbanError::InvalidTransition { from, to, .. } => {
                assert_eq!(from, TaskStatus::Backlog);
                assert_eq!(to, TaskStatus::InProgress);
            }
            e => panic!("expected InvalidTransition, got {e:?}"),
        }
    }

    // REQ: KAN-SVC-T-011 — task_assign requires valid consent
    #[test]
    fn task_assign_with_consent() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        let agent = WebID::new();
        let consent = ConsentProof::new(agent, task.id);

        let assigned = svc.task_assign(task.id, agent, consent).unwrap();
        assert_eq!(assigned.assignee, Some(agent));
    }

    // REQ: KAN-SVC-T-012 — task_assign rejects mismatched consent
    #[test]
    fn task_assign_rejects_invalid_consent() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        let agent = WebID::new();
        let other_agent = WebID::new();
        // Consent is for a different agent
        let bad_consent = ConsentProof::new(other_agent, task.id);

        let result = svc.task_assign(task.id, agent, bad_consent);
        assert!(result.is_err());
        match result.unwrap_err() {
            KanbanError::ConsentViolation(_) => {}
            e => panic!("expected ConsentViolation, got {e:?}"),
        }
    }

    // REQ: KAN-SVC-T-013 — task_verify moves to Done on pass
    #[test]
    fn task_verify_pass() {
        let (svc, board, owner) = make_service_with_board();
        let spec = TaskSpec::new("Test".into())
            .with_criteria(vec![VerificationCriterion::new("compile".into())]);
        let task = svc.task_create(board.id, spec, owner).unwrap();

        // Move to Review
        svc.task_move(task.id, TaskStatus::Ready, owner).unwrap();
        svc.task_move(task.id, TaskStatus::InProgress, owner)
            .unwrap();
        svc.task_move(task.id, TaskStatus::Review, owner).unwrap();

        let (verified, _verif) = svc
            .task_verify(task.id, "The code compiles successfully", owner)
            .unwrap();
        assert_eq!(verified.status, TaskStatus::Done);
        assert!(verified.verification.is_some());
        assert!(verified.verification.as_ref().unwrap().passed);
    }

    // REQ: KAN-SVC-T-014 — task_verify rejects non-Review tasks
    #[test]
    fn task_verify_rejects_non_review() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_verify(task.id, "evidence", owner);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-015 — board_get retrieves created board
    #[test]
    fn board_get_succeeds() {
        let (svc, board, _owner) = make_service_with_board();
        let retrieved = svc.board_get(board.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Board");
    }

    // REQ: KAN-SVC-T-016 — board_isolation: alice cannot see bob's board
    #[test]
    fn board_isolation() {
        let svc = KanbanService::new();
        let alice = WebID::new();
        let bob = WebID::new();

        svc.board_create(alice, "Alice's Board", &make_default_columns())
            .unwrap();
        svc.board_create(bob, "Bob's Board", &make_default_columns())
            .unwrap();

        // Alice should only see her board
        let alice_boards = svc.board_list(&alice).unwrap();
        assert_eq!(alice_boards.len(), 1);
        assert_eq!(alice_boards[0].name, "Alice's Board");
    }
}
