//! KanbanService — core kanban board and task coordination.
//!
//! Implements the 7 public operations from the kanban agent coordination spec.
//! Every operation carries ownership tracking (P12) and enforces agent consent
//! on assignment (P1). State transitions are column-ordered.
//!
//! Persistence: boards and tasks stored as RDF triples via TripleStore (MDS §2).
//! Triple scheme:
//!   kanban:board → {board_id} → JSON Board
//!   kanban:task  → {task_id}  → JSON Task
//!   kanban:board_tasks:{board_id} → {task_id} → task_id (index)

use std::sync::Arc;

use hkask_storage::{Triple, TripleStore};
use hkask_types::{
    Board, BoardId, ColumnDef, ConsentProof, Task, TaskFilter, TaskId, TaskSpec, TaskStatus,
    Verification, WebID,
};
use serde_json::Value;

/// Core kanban coordination service.
///
/// Persists boards and tasks as RDF triples in a TripleStore.
/// Public surface: exactly 7 operations (deep-module discipline).
#[derive(Clone)]
pub struct KanbanService {
    store: TripleStore,
}

// Triple entity prefixes
const BOARD_ENTITY: &str = "kanban:board";
const TASK_ENTITY: &str = "kanban:task";
const BOARD_TASKS_PREFIX: &str = "kanban:board_tasks:";

impl KanbanService {
    /// Create a KanbanService backed by the given TripleStore.
    ///
    /// REQ: KAN-SVC-001
    /// pre:  store must have the triples table initialized
    /// post: returns a KanbanService ready for use
    pub fn new(store: TripleStore) -> Self {
        Self { store }
    }

    // ── Board operations ──────────────────────────────────────────────────

    /// Create a new kanban board.
    ///
    /// REQ: KAN-SVC-002
    /// pre:  owner is a valid WebID; name is non-empty; columns is non-empty
    /// post: board is persisted as a triple; returns the created Board
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
        let value = serde_json::to_value(&board)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;

        let triple = Triple::new(BOARD_ENTITY, &board.id.to_string(), value, owner);
        self.store
            .insert(&triple)
            .map_err(|e| KanbanError::Internal(format!("triple insert failed: {e}")))?;

        Ok(board)
    }

    /// List all boards for a given owner.
    ///
    /// REQ: KAN-SVC-003
    /// pre:  owner is a valid WebID
    /// post: returns all boards owned by this replicant
    pub fn board_list(&self, owner: &WebID) -> Result<Vec<Board>, KanbanError> {
        let triples = self
            .store
            .query_by_entity(BOARD_ENTITY)
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        let mut boards: Vec<Board> = Vec::new();
        for t in &triples {
            if t.access.owner_webid == *owner {
                if let Ok(board) = serde_json::from_value::<Board>(t.value.clone()) {
                    boards.push(board);
                }
            }
        }

        boards.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(boards)
    }

    /// Get a board by ID.
    ///
    /// REQ: KAN-SVC-004
    /// pre:  board_id is valid
    /// post: returns Some(Board) if found, None otherwise
    pub fn board_get(&self, board_id: BoardId) -> Result<Option<Board>, KanbanError> {
        let triples = self
            .store
            .query_by_entity_attribute(BOARD_ENTITY, &board_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        if let Some(t) = triples.into_iter().next() {
            let board = serde_json::from_value::<Board>(t.value)
                .map_err(|e| KanbanError::Internal(format!("deserialization failed: {e}")))?;
            Ok(Some(board))
        } else {
            Ok(None)
        }
    }

    // ── Task operations ───────────────────────────────────────────────────

    /// Create a new task on a board.
    ///
    /// REQ: KAN-SVC-005
    /// pre:  board_id refers to an existing board; spec.title is non-empty; owner is valid
    /// post: task is persisted as a triple; returns the created Task
    pub fn task_create(
        &self,
        board_id: BoardId,
        spec: TaskSpec,
        owner: WebID,
    ) -> Result<Task, KanbanError> {
        // Verify board exists
        let board = self
            .board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;
        let _ = board;

        // Extract sizing fields before Task::new consumes the spec
        let sp = spec.story_points;
        let eh = spec.estimated_hours;
        let dd = spec.due_date;
        let lbls = spec.labels.clone();
        let mut task = Task::new(board_id, spec, owner);
        task.story_points = sp;
        task.estimated_hours = eh;
        task.due_date = dd;
        task.labels = lbls;
        let value = serde_json::to_value(&task)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;

        // Persist the task
        let triple = Triple::new(TASK_ENTITY, &task.id.to_string(), value, owner);
        self.store
            .insert(&triple)
            .map_err(|e| KanbanError::Internal(format!("triple insert failed: {e}")))?;

        // Persist board→task index
        let index_entity = format!("{BOARD_TASKS_PREFIX}{board_id}");
        let index_triple = Triple::new(
            &index_entity,
            &task.id.to_string(),
            Value::String(task.id.to_string()),
            owner,
        );
        self.store
            .insert(&index_triple)
            .map_err(|e| KanbanError::Internal(format!("index triple insert failed: {e}")))?;

        Ok(task)
    }

    /// Count tasks in a given status on a board (for WIP enforcement).
    fn count_tasks_in_status(
        &self,
        board_id: BoardId,
        status: TaskStatus,
    ) -> Result<usize, KanbanError> {
        let index_entity = format!("{BOARD_TASKS_PREFIX}{board_id}");
        let index_triples = self
            .store
            .query_by_entity(&index_entity)
            .map_err(|e| KanbanError::Internal(format!("index query failed: {e}")))?;

        let mut count = 0usize;
        for idx_t in &index_triples {
            if let Some(task_id_str) = idx_t.value.as_str() {
                let task_triples = self
                    .store
                    .query_by_entity_attribute(TASK_ENTITY, task_id_str)
                    .map_err(|e| KanbanError::Internal(format!("task query failed: {e}")))?;
                for t in &task_triples {
                    if let Ok(task) = serde_json::from_value::<Task>(t.value.clone()) {
                        if task.status == status {
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(count)
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
        let index_entity = format!("{BOARD_TASKS_PREFIX}{board_id}");

        // Get task IDs from the index
        let index_triples = self
            .store
            .query_by_entity(&index_entity)
            .map_err(|e| KanbanError::Internal(format!("index query failed: {e}")))?;

        let mut tasks: Vec<Task> = Vec::new();
        for idx_t in &index_triples {
            if let Some(task_id_str) = idx_t.value.as_str() {
                let task_triples = self
                    .store
                    .query_by_entity_attribute(TASK_ENTITY, task_id_str)
                    .map_err(|e| KanbanError::Internal(format!("task query failed: {e}")))?;

                for t in &task_triples {
                    if let Ok(task) = serde_json::from_value::<Task>(t.value.clone()) {
                        let status_match = filter.status.map_or(true, |s| task.status == s);
                        let assignee_match =
                            filter.assignee.map_or(true, |a| task.assignee == Some(a));

                        if status_match && assignee_match {
                            tasks.push(task);
                        }
                    }
                }
            }
        }

        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(limit) = filter.limit {
            tasks.truncate(limit);
        }

        Ok(tasks)
    }

    /// Get a task by ID.
    ///
    /// REQ: KAN-SVC-007
    /// pre:  task_id is valid
    /// post: returns Some(Task) if found, None otherwise
    pub fn task_get(&self, task_id: TaskId) -> Result<Option<Task>, KanbanError> {
        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        if let Some(t) = triples.into_iter().next() {
            let task = serde_json::from_value::<Task>(t.value)
                .map_err(|e| KanbanError::Internal(format!("deserialization failed: {e}")))?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Move a task to a new column (state transition).
    ///
    /// REQ: KAN-SVC-008
    /// pre:  task_id refers to an existing task; target is a valid transition from current status
    /// pre:  actor is a valid WebID (P12)
    /// post: task.status is updated; updated_at is refreshed
    pub fn task_move(
        &self,
        task_id: TaskId,
        target: TaskStatus,
        actor: WebID,
    ) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if !task.can_move_to(target) {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: target,
            });
        }

        // WIP limit enforcement (Anderson §4: "limit WIP to expose problems")
        if let Some(board) = self.board_get(task.board_id)? {
            if let Some(col) = board.column_for_status(target) {
                if let Some(wip_limit) = col.wip_limit {
                    let current_count = self.count_tasks_in_status(task.board_id, target)?;
                    if current_count >= wip_limit as usize {
                        return Err(KanbanError::WipLimitExceeded {
                            column: col.name.clone(),
                            limit: wip_limit,
                            current: current_count as u32,
                        });
                    }
                }
            }
        }

        task.status = target;
        task.updated_at = chrono::Utc::now();
        let _ = actor;

        // Update the triple value
        let new_value = serde_json::to_value(&task)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;

        // Find the triple ID and update
        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        if let Some(t) = triples.into_iter().next() {
            self.store
                .update(&t.id, new_value, 1.0f64)
                .map_err(|e| KanbanError::Internal(format!("triple update failed: {e}")))?;
        }

        Ok(task)
    }

    /// Assign a task to an agent with consent proof.
    ///
    /// REQ: KAN-SVC-009
    /// pre:  task_id refers to an existing task; consent.agent matches the assignee
    /// pre:  consent.task_id matches task_id
    /// post: task.assignee is set to consent.agent
    /// fails: if consent is invalid → ConsentViolation
    pub fn task_assign(
        &self,
        task_id: TaskId,
        agent: WebID,
        consent: ConsentProof,
    ) -> Result<Task, KanbanError> {
        // P1: Verify consent
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

        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        task.assignee = Some(agent);
        task.updated_at = chrono::Utc::now();

        let new_value = serde_json::to_value(&task)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;

        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        if let Some(t) = triples.into_iter().next() {
            self.store
                .update(&t.id, new_value, 1.0f64)
                .map_err(|e| KanbanError::Internal(format!("triple update failed: {e}")))?;
        }

        Ok(task)
    }

    /// Verify a task's completion against its acceptance criteria.
    ///
    /// REQ: KAN-SVC-010
    /// pre:  task_id refers to an existing task in Review status
    /// pre:  verifier is a valid WebID
    /// post: task.verification is set; task moves to Done if passed
    pub fn task_verify(
        &self,
        task_id: TaskId,
        evidence: &str,
        verifier: WebID,
    ) -> Result<(Task, Verification), KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if task.status != TaskStatus::Review {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: TaskStatus::Done,
            });
        }

        // Keyword-based verification
        let passed = if task.criteria.is_empty() {
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

        let new_value = serde_json::to_value(&task)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;

        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        if let Some(t) = triples.into_iter().next() {
            self.store
                .update(&t.id, new_value, 1.0f64)
                .map_err(|e| KanbanError::Internal(format!("triple update failed: {e}")))?;
        }

        Ok((task, verification))
    }

    /// Decompose a project description into kanban tasks.
    ///
    /// REQ: KAN-SVC-020
    pub fn decompose_prompt(
        &self,
        board_id: BoardId,
        project_description: &str,
        target_task_points: Option<u32>,
        target_hours: Option<f64>,
    ) -> Result<String, KanbanError> {
        self.board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;
        let sizing_guidance = match (target_task_points, target_hours) {
            (Some(p), Some(h)) => format!("Each task should be approximately {p} story points or {h} hours."),
            (Some(p), None) => format!("Each task should be approximately {p} story points."),
            (None, Some(h)) => format!("Each task should be approximately {h} hours."),
            (None, None) => "Aim for tasks of 2-8 hours each.".to_string(),
        };
        Ok(format!("Decompose project into kanban tasks. Project: {project_description}. Sizing: {sizing_guidance}. For each task provide: title, description, story_points (int), estimated_hours (float), labels (comma-separated), criteria (list of acceptance criteria strings). Return JSON array of objects."))
    }

    /// Spawn a sub-replicant to execute a task.
    ///
    /// REQ: KAN-SVC-021
    pub fn spawn_task(
        &self,
        task_id: TaskId,
        spawn_spec: hkask_types::SpawnSpec,
    ) -> Result<String, KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        Ok(format!("Spawn for task '{}': level={}, skills={:?}, memory={}, tools={:?}, gas={:?}, timeout={:?}s. [Future: pod activation]",
            task.title, spawn_spec.delegation_level, spawn_spec.delegated_skills,
            spawn_spec.memory_scope, spawn_spec.tool_servers, spawn_spec.gas_budget, spawn_spec.timeout_seconds))
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

    #[error("WIP limit exceeded: column '{column}' has {current}/{limit} tasks (limit: {limit})")]
    WipLimitExceeded {
        column: String,
        limit: u32,
        current: u32,
    },
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use hkask_storage::Store;
    use super::*;
    use hkask_types::VerificationCriterion;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn make_store() -> TripleStore {
        let conn = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory DB"),
        ));
        let store = TripleStore::new(conn);
        store
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE triples (
                    id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                    value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                    confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
                    owner_webid TEXT NOT NULL
                )",
            )
            .unwrap();
        store
    }

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
        let svc = KanbanService::new(make_store());
        let owner = WebID::new();
        let board = svc
            .board_create(owner, "Test Board", &make_default_columns())
            .unwrap();
        (svc, board, owner)
    }

    // REQ: KAN-SVC-T-001
    #[test]
    fn board_create_succeeds() {
        let svc = KanbanService::new(make_store());
        let owner = WebID::new();
        let board = svc
            .board_create(owner, "My Board", &make_default_columns())
            .unwrap();
        assert_eq!(board.name, "My Board");
        assert_eq!(board.owner, owner);
        assert_eq!(board.columns.len(), 5);
    }

    // REQ: KAN-SVC-T-002
    #[test]
    fn board_create_rejects_empty_name() {
        let svc = KanbanService::new(make_store());
        let result = svc.board_create(WebID::new(), "", &make_default_columns());
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-003
    #[test]
    fn board_create_rejects_empty_columns() {
        let svc = KanbanService::new(make_store());
        let result = svc.board_create(WebID::new(), "Board", &[]);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-004
    #[test]
    fn board_list_by_owner() {
        let svc = KanbanService::new(make_store());
        let alice = WebID::new();
        let bob = WebID::new();

        svc.board_create(alice, "Alice's Board", &make_default_columns())
            .unwrap();
        svc.board_create(bob, "Bob's Board", &make_default_columns())
            .unwrap();

        let alice_boards = svc.board_list(&alice).unwrap();
        assert_eq!(alice_boards.len(), 1);
        assert_eq!(alice_boards[0].name, "Alice's Board");
    }

    // REQ: KAN-SVC-T-005
    #[test]
    fn task_create_defaults_to_backlog() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Backlog);
        assert_eq!(task.board_id, board.id);
    }

    // REQ: KAN-SVC-T-006
    #[test]
    fn task_create_rejects_unknown_board() {
        let svc = KanbanService::new(make_store());
        let result = svc.task_create(BoardId::new(), TaskSpec::new("Test".into()), WebID::new());
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-007
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

    // REQ: KAN-SVC-T-008
    #[test]
    fn task_list_filter_by_status() {
        let (svc, board, owner) = make_service_with_board();
        let t1 = svc
            .task_create(board.id, TaskSpec::new("T1".into()), owner)
            .unwrap();
        svc.task_move(t1.id, TaskStatus::Ready, owner).unwrap();
        svc.task_move(t1.id, TaskStatus::InProgress, owner).unwrap();

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

    // REQ: KAN-SVC-T-009
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

    // REQ: KAN-SVC-T-010
    #[test]
    fn task_move_rejects_skip() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_move(task.id, TaskStatus::InProgress, owner);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-011
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

    // REQ: KAN-SVC-T-012
    #[test]
    fn task_assign_rejects_invalid_consent() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        let agent = WebID::new();
        let other_agent = WebID::new();
        let bad_consent = ConsentProof::new(other_agent, task.id);

        let result = svc.task_assign(task.id, agent, bad_consent);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-013
    #[test]
    fn task_verify_pass() {
        let (svc, board, owner) = make_service_with_board();
        let spec = TaskSpec::new("Test".into())
            .with_criteria(vec![VerificationCriterion::new("compile".into())]);
        let task = svc.task_create(board.id, spec, owner).unwrap();

        svc.task_move(task.id, TaskStatus::Ready, owner).unwrap();
        svc.task_move(task.id, TaskStatus::InProgress, owner)
            .unwrap();
        svc.task_move(task.id, TaskStatus::Review, owner).unwrap();

        let (verified, _verif) = svc
            .task_verify(task.id, "The code compiles successfully", owner)
            .unwrap();
        assert_eq!(verified.status, TaskStatus::Done);
        assert!(verified.verification.as_ref().unwrap().passed);
    }

    // REQ: KAN-SVC-T-014
    #[test]
    fn task_verify_rejects_non_review() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_verify(task.id, "evidence", owner);
        assert!(result.is_err());
    }

    // REQ: KAN-SVC-T-015
    #[test]
    fn board_get_succeeds() {
        let (svc, board, _owner) = make_service_with_board();
        let retrieved = svc.board_get(board.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Board");
    }

    // REQ: KAN-SVC-T-016
    #[test]
    fn board_isolation() {
        let svc = KanbanService::new(make_store());
        let alice = WebID::new();
        let bob = WebID::new();

        svc.board_create(alice, "Alice's Board", &make_default_columns())
            .unwrap();
        svc.board_create(bob, "Bob's Board", &make_default_columns())
            .unwrap();

        let alice_boards = svc.board_list(&alice).unwrap();
        assert_eq!(alice_boards.len(), 1);
        assert_eq!(alice_boards[0].name, "Alice's Board");
    }
}
