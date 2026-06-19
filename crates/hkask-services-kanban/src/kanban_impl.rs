//! KanbanService — core kanban board and task coordination.
//!
//! Implements kanban board and task coordination operations.
//! Every operation carries ownership tracking (P12) and enforces agent consent
//! on assignment (P1). State transitions are column-ordered.
//!
//! Persistence: boards and tasks stored as RDF triples via TripleStore (MDS §2).
//! Triple scheme:
//!   kanban:board → {board_id} → JSON Board
//!   kanban:task  → {task_id}  → JSON Task
//!   kanban:board_tasks:{board_id} → {task_id} → task_id (index)

use hkask_storage::{Triple, TripleStore};
use hkask_types::{
    Board, BoardId, ColumnDef, Comment, ConsentProof, Phase, PhaseId, Task, TaskFilter, TaskId,
    TaskSpec, TaskStatus, Verification, WebID,
};
use serde_json::Value;
use std::sync::Arc;

pub(crate) mod comments;
pub(crate) mod decompose;
pub(crate) mod dejam;
pub(crate) mod kata;
pub(crate) mod phases;
pub(crate) mod spawn;
pub(crate) mod verification;

/// Core kanban coordination service.
///
/// Persists boards and tasks as RDF triples in a TripleStore.
/// Public surface: board and task coordination operations.
#[derive(Clone)]
pub struct KanbanService {
    store: TripleStore,
    pod_manager: Option<Arc<hkask_agents::pod::ActivePods>>,
}

// Triple entity prefixes
const BOARD_ENTITY: &str = "kanban:board";
const TASK_ENTITY: &str = "kanban:task";
const BOARD_TASKS_PREFIX: &str = "kanban:board_tasks:";

impl KanbanService {
    /// Create a KanbanService backed by the given TripleStore.
    ///
    pub fn new(store: TripleStore) -> Self {
        Self {
            store,
            pod_manager: None,
        }
    }

    /// Attach a PodManager for live spawn capability.
    ///
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_pod_manager(mut self, pm: Arc<hkask_agents::pod::ActivePods>) -> Self {
        self.pod_manager = Some(pm);
        self
    }

    // ── Board operations ──────────────────────────────────────────────────

    /// Create a new kanban board.
    ///
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

        // P9: CNS span
        tracing::info!(
            target: "cns.kanban",
            operation = "board_created",
            board_id = %board.id,
            name = %name,
            owner = %owner,
            "CNS"
        );

        Ok(board)
    }

    /// Create a board from a YAML template file.
    ///
    pub fn board_create_from_template(
        &self,
        owner: WebID,
        name: &str,
        template_yaml: &str,
    ) -> Result<Board, KanbanError> {
        #[derive(serde::Deserialize)]
        struct TemplateColumns {
            name: String,
            status: String,
            wip_limit: Option<u32>,
        }
        #[derive(serde::Deserialize)]
        struct TemplatePhase {
            name: String,
        }
        #[derive(serde::Deserialize)]
        struct BoardTemplate {
            columns: Vec<TemplateColumns>,
            phases: Vec<TemplatePhase>,
        }

        let template: BoardTemplate = serde_yaml_neo::from_str(template_yaml)
            .map_err(|e| KanbanError::InvalidInput(format!("Invalid template YAML: {e}")))?;

        let columns: Vec<ColumnDef> = template
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let status = TaskStatus::parse_str(&c.status).unwrap_or(TaskStatus::Backlog);
                let mut col = ColumnDef::new(c.name.clone(), status, i as u32);
                if let Some(wip) = c.wip_limit {
                    col = col.with_wip_limit(wip);
                }
                col
            })
            .collect();

        let board = self.board_create(owner, name, &columns)?;

        // Create phases
        for phase in &template.phases {
            self.board_add_phase(board.id, &phase.name, 0)?;
        }

        Ok(board)
    }

    /// List all board templates.
    ///
    pub fn list_templates() -> Vec<String> {
        vec![
            "software-project".into(),
            "writing-project".into(),
            "scientific-research".into(),
            "investment-research".into(),
        ]
    }

    /// List all boards for a given owner.
    ///
    pub fn board_list(&self, owner: &WebID) -> Result<Vec<Board>, KanbanError> {
        let triples = self
            .store
            .query_by_entity(BOARD_ENTITY)
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;

        let mut boards: Vec<Board> = Vec::new();
        for t in &triples {
            if t.access.owner_webid == *owner
                && let Ok(board) = serde_json::from_value::<Board>(t.value.clone())
            {
                boards.push(board);
            }
        }

        boards.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(boards)
    }

    /// Get a board by ID.
    ///
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

    /// Render a text-based kanban board view.
    ///
    ///       WIP limits, story points, labels, overdue indicators, and verification status
    pub fn board_view(
        &self,
        board_id: BoardId,
        filter: Option<&str>,
    ) -> Result<String, KanbanError> {
        let board = self
            .board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;
        let mut tasks = self.task_list(board_id, TaskFilter::all())?;

        // Apply filter if present
        let filter_desc = if let Some(f) = filter {
            if let Some(s) = TaskStatus::parse_str(f) {
                tasks.retain(|t| t.status == s);
                Some(format!("status={}", s))
            } else if let Some(p) = hkask_types::Priority::parse_str(f) {
                tasks.retain(|t| t.priority == Some(p));
                Some(format!("priority={}", p))
            } else if f.len() > 30 && f.parse::<WebID>().is_ok() {
                let wid: WebID = f.parse().expect("validated by is_ok check above");
                tasks.retain(|t| t.assignee == Some(wid));
                Some(format!("assignee={}", wid.redacted_display()))
            } else {
                let lower = f.to_lowercase();
                tasks.retain(|t| t.labels.iter().any(|l| l.to_lowercase().contains(&lower)));
                Some(format!("label~{}", f))
            }
        } else {
            None
        };

        let mut by_status: std::collections::HashMap<TaskStatus, Vec<&Task>> =
            std::collections::HashMap::new();
        for t in &tasks {
            by_status.entry(t.status).or_default().push(t);
        }

        let mut out = format!("{}  {}", board.name, board.id);
        if let Some(ref d) = filter_desc {
            out.push_str(&format!("  [{}]", d));
        }
        out.push_str("\n\n");

        for col in &board.columns {
            let count = by_status.get(&col.status).map(|v| v.len()).unwrap_or(0);
            if count == 0 && !tasks.is_empty() {
                continue;
            }
            let wip = col.wip_limit.map_or(String::new(), |l| format!("/{}", l));
            out.push_str(&format!("  {}{} ({}{})\n", col.name, wip, count, wip));
        }
        out.push('\n');

        for col in &board.columns {
            let col_tasks = by_status
                .get(&col.status)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            if col_tasks.is_empty() {
                continue;
            }
            out.push_str(&format!("  {}:\n", col.name));
            for task in col_tasks {
                let idx = tasks.iter().position(|t| t.id == task.id).unwrap_or(0) + 1;
                let a = task
                    .assignee
                    .map(|a| format!(" <- {}", a.redacted_display()))
                    .unwrap_or_default();
                let p = task
                    .priority
                    .map(|p| match p {
                        hkask_types::Priority::Critical => " !!",
                        hkask_types::Priority::High => " !",
                        _ => "",
                    })
                    .unwrap_or("");
                out.push_str(&format!("    {}. {}{}{}\n", idx, task.title, p, a));
            }
            out.push('\n');
        }

        if tasks.is_empty() && filter.is_some() {
            out.push_str("  (no tasks match)\n");
        }

        Ok(out)
    } // ── Task operations ───────────────────────────────────────────────────

    /// Create a new task on a board.
    ///
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
        let pr = spec.priority;
        let lbls = spec.labels.clone();
        let ph = spec.phase_id;
        let mut task = Task::new(board_id, spec, owner);
        task.story_points = sp;
        task.estimated_hours = eh;
        task.labels = lbls;
        task.priority = pr;
        task.phase_id = ph;
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

        // P9: CNS span
        tracing::info!(
            target: "cns.kanban",
            operation = "task_created",
            task_id = %task.id,
            board_id = %board_id,
            owner = %owner,
            "CNS"
        );

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
                    if let Ok(task) = serde_json::from_value::<Task>(t.value.clone())
                        && task.status == status
                    {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    /// List tasks on a board, optionally filtered.
    ///
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
                        let status_match = filter.status.is_none_or(|s| task.status == s);
                        let assignee_match =
                            filter.assignee.is_none_or(|a| task.assignee == Some(a));
                        let priority_match =
                            filter.priority.is_none_or(|p| task.priority == Some(p));

                        if status_match && assignee_match && priority_match {
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

        let from_status = task.status;

        // WIP limit enforcement (Anderson §4: "limit WIP to expose problems")
        if let Some(board) = self.board_get(task.board_id)?
            && let Some(col) = board.column_for_status(target)
            && let Some(wip_limit) = col.wip_limit
        {
            let current_count = self.count_tasks_in_status(task.board_id, target)?;
            if current_count >= wip_limit as usize {
                return Err(KanbanError::WipLimitExceeded {
                    column: col.name.clone(),
                    limit: wip_limit,
                    current: current_count as u32,
                });
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

        // P9: CNS span
        tracing::info!(
            target: "cns.kanban",
            operation = "task_moved",
            task_id = %task_id,
            from = %from_status,
            to = %target,
            actor = %actor,
            "CNS"
        );

        Ok(task)
    }

    /// Assign a task to an agent with consent proof.
    ///
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

        // P9: CNS span
        tracing::info!(
            target: "cns.kanban",
            operation = "task_assigned",
            task_id = %task_id,
            agent = %agent,
            "CNS"
        );

        Ok(task)
    }

    /// Verify a task's completion against its acceptance criteria.
    ///
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

        // rSolidity contract-based verification
        // The task IS a contract. Both agent and replicant run the same assertions.
        let mut contract =
            hkask_types::TaskContract::new("inline".into(), task.owner, verifier, &task, vec![]);
        let result = contract.check_completion(evidence);

        let passed = result.passed;
        let reasoning = result.reasoning;

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

        // P9: CNS span
        tracing::info!(
            target: "cns.kanban",
            operation = "task_verified",
            task_id = %task_id,
            passed = passed,
            verifier = %verifier,
            "CNS"
        );

        Ok((task, verification))
    }

    // ── Decomposition + Spawn ─────────────────────────────────────────

    // Moved to decompose.rs and spawn.rs.

    // ── Comments (mini-REPL per task) ─────────────────────────────────

    // Moved to comments.rs.

    // ── Deliverables (file path / URL links) ──────────────────────────

    // Moved to comments.rs.

    // ── Phases ────────────────────────────────────────────────────────

    // Moved to phases.rs.

    // ── Lifecycle operations (P0) ─────────────────────────────────────

    /// Delete a task and its board index entry.
    ///
    pub fn task_delete(&self, task_id: TaskId) -> Result<(), KanbanError> {
        let task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        // Close the task triple
        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        for t in &triples {
            self.store
                .close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("triple close failed: {e}")))?;
        }

        // Close the index triple
        let index_entity = format!("{BOARD_TASKS_PREFIX}{}", task.board_id);
        let idx_triples = self
            .store
            .query_by_entity_attribute(&index_entity, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("index query failed: {e}")))?;
        for t in &idx_triples {
            self.store
                .close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("index close failed: {e}")))?;
        }

        Ok(())
    }

    /// Unassign a task — remove the assignee.
    ///
    pub fn task_unassign(&self, task_id: TaskId) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        task.assignee = None;
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }

    /// Reopen a completed task — move from Done back to InProgress.
    ///
    pub fn task_reopen(&self, task_id: TaskId) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if task.status != TaskStatus::Done {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: TaskStatus::InProgress,
            });
        }

        task.status = TaskStatus::InProgress;
        task.verification = None;
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }

    /// Delete a board and all its tasks.
    ///
    pub fn board_delete(&self, board_id: BoardId) -> Result<usize, KanbanError> {
        let board = self
            .board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;

        // Delete all tasks on this board
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let task_count = tasks.len();
        for task in &tasks {
            let _ = self.task_delete(task.id);
        }

        // Close the board triple
        let triples = self
            .store
            .query_by_entity_attribute(BOARD_ENTITY, &board_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        for t in &triples {
            self.store
                .close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("triple close failed: {e}")))?;
        }
        let _ = board;

        Ok(task_count)
    }

    // ── De-jamming ────────────────────────────────────────────────────

    // Moved to dejam.rs.

    // ── LLM Verification ──────────────────────────────────────────────

    // Moved to verification.rs.

    // ── Kata Integration (task-scoped scientific thinking) ──────────

    // Moved to kata.rs.

    // ── Helpers ───────────────────────────────────────────────────────

    fn update_task_triple(&self, task: &Task) -> Result<(), KanbanError> {
        let new_value = serde_json::to_value(task)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;
        let triples = self
            .store
            .query_by_entity_attribute(TASK_ENTITY, &task.id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        if let Some(t) = triples.into_iter().next() {
            self.store
                .update(&t.id, new_value, 1.0f64)
                .map_err(|e| KanbanError::Internal(format!("triple update failed: {e}")))?;
        }
        Ok(())
    }

    fn update_board_triple(&self, board: &Board) -> Result<(), KanbanError> {
        let new_value = serde_json::to_value(board)
            .map_err(|e| KanbanError::Internal(format!("serialization failed: {e}")))?;
        let triples = self
            .store
            .query_by_entity_attribute(BOARD_ENTITY, &board.id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        if let Some(t) = triples.into_iter().next() {
            self.store
                .update(&t.id, new_value, 1.0f64)
                .map_err(|e| KanbanError::Internal(format!("triple update failed: {e}")))?;
        }
        Ok(())
    }
}

/// UnjamItem — a stuck state detected by the de-jammer.
#[derive(Debug, Clone)]
pub struct UnjamItem {
    pub task_id: hkask_types::TaskId,
    pub task_title: String,
    pub issue: String,
    pub suggestion: String,
}

/// UnjamFix — records an auto-fix action taken by the de-jammer.
#[derive(Debug, Clone)]
pub struct UnjamFix {
    pub task_id: hkask_types::TaskId,
    pub task_title: String,
    pub action: String,
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
    use super::*;
    use hkask_storage::Store;
    use hkask_types::VerificationCriterion;
    use rusqlite::Connection;
    use std::sync::Arc;
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

    #[test]
    fn board_create_rejects_empty_name() {
        let svc = KanbanService::new(make_store());
        let result = svc.board_create(WebID::new(), "", &make_default_columns());
        assert!(result.is_err());
    }

    #[test]
    fn board_create_rejects_empty_columns() {
        let svc = KanbanService::new(make_store());
        let result = svc.board_create(WebID::new(), "Board", &[]);
        assert!(result.is_err());
    }

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

    #[test]
    fn task_create_defaults_to_backlog() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Backlog);
        assert_eq!(task.board_id, board.id);
    }

    #[test]
    fn task_create_rejects_unknown_board() {
        let svc = KanbanService::new(make_store());
        let result = svc.task_create(BoardId::new(), TaskSpec::new("Test".into()), WebID::new());
        assert!(result.is_err());
    }

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

    #[test]
    fn task_move_rejects_skip() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_move(task.id, TaskStatus::InProgress, owner);
        assert!(result.is_err());
    }

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

    #[test]
    fn task_verify_rejects_non_review() {
        let (svc, board, owner) = make_service_with_board();
        let task = svc
            .task_create(board.id, TaskSpec::new("Test".into()), owner)
            .unwrap();

        let result = svc.task_verify(task.id, "evidence", owner);
        assert!(result.is_err());
    }

    #[test]
    fn board_get_succeeds() {
        let (svc, board, _owner) = make_service_with_board();
        let retrieved = svc.board_get(board.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Board");
    }

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
