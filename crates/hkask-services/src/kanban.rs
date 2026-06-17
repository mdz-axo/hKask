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
use std::sync::Arc;
use hkask_types::{
    Board, BoardId, ColumnDef, Comment, ConsentProof, Phase, PhaseId, Task, TaskFilter, TaskId,
    TaskSpec, TaskStatus, Verification, WebID,
};
use serde_json::Value;

/// Core kanban coordination service.
///
/// Persists boards and tasks as RDF triples in a TripleStore.
/// Public surface: board and task coordination operations.
#[derive(Clone)]
pub struct KanbanService {
    store: TripleStore,
    pod_manager: Option<Arc<hkask_agents::pod::PodManager>>,
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
        Self { store, pod_manager: None }
    }

    /// Attach a PodManager for live spawn capability.
    ///
    /// REQ: KAN-SVC-001b
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_pod_manager(mut self, pm: Arc<hkask_agents::pod::PodManager>) -> Self {
        self.pod_manager = Some(pm);
        self
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

    /// Create a board from a YAML template file.
    ///
    /// REQ: KAN-SVC-002b
    /// pre:  template_path is a valid YAML file with board template schema
    /// post: board is created with template-defined columns, WIP limits, and phases
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

        let template: BoardTemplate = serde_yaml::from_str(template_yaml)
            .map_err(|e| KanbanError::InvalidInput(format!("Invalid template YAML: {e}")))?;

        let columns: Vec<ColumnDef> = template.columns.iter().enumerate().map(|(i, c)| {
            let status = TaskStatus::parse_str(&c.status).unwrap_or(TaskStatus::Backlog);
            let mut col = ColumnDef::new(c.name.clone(), status, i as u32);
            if let Some(wip) = c.wip_limit {
                col = col.with_wip_limit(wip);
            }
            col
        }).collect();

        let board = self.board_create(owner, name, &columns)?;

        // Create phases
        for phase in &template.phases {
            self.board_add_phase(board.id, &phase.name, 0)?;
        }

        Ok(board)
    }

    /// List all board templates.
    ///
    /// REQ: KAN-SVC-002c
    pub fn list_templates() -> Vec<String> {
        vec!["software-project".into(), "writing-project".into(), "scientific-research".into(), "investment-research".into()]
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

    /// Render a text-based kanban board view.
    ///
    /// REQ: KAN-SVC-004b
    /// pre:  board_id refers to an existing board
    /// post: returns a formatted string showing columns with tasks arranged by status,
    ///       WIP limits, story points, labels, overdue indicators, and verification status
    pub fn board_view(&self, board_id: BoardId, filter: Option<&str>) -> Result<String, KanbanError> {
        let board = self.board_get(board_id)?
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
                let wid: WebID = f.parse().unwrap();
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
            if count == 0 && !tasks.is_empty() { continue; }
            let wip = col.wip_limit.map_or(String::new(), |l| format!("/{}", l));
            out.push_str(&format!("  {}{} ({}{})\n", col.name, wip, count, wip));
        }
        out.push_str("\n");

        for col in &board.columns {
            let col_tasks = by_status.get(&col.status).map(|v| v.as_slice()).unwrap_or(&[]);
            if col_tasks.is_empty() { continue; }
            out.push_str(&format!("  {}:\n", col.name));
            for task in col_tasks {
                let idx = tasks.iter().position(|t| t.id == task.id).unwrap_or(0) + 1;
                let a = task.assignee.map(|a| format!(" <- {}", a.redacted_display())).unwrap_or_default();
                let p = task.priority.map(|p| match p {
                    hkask_types::Priority::Critical => " !!",
                    hkask_types::Priority::High => " !",
                    _ => "",
                }).unwrap_or("");
                out.push_str(&format!("    {}. {}{}{}\n", idx, task.title, p, a));
            }
            out.push_str("\n");
        }

        if tasks.is_empty() && filter.is_some() {
            out.push_str("  (no tasks match)\n");
        }

        Ok(out)
    }    // ── Task operations ───────────────────────────────────────────────────

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
                        let priority_match =
                            filter.priority.map_or(true, |p| task.priority == Some(p));

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

        // rSolidity contract-based verification
        // The task IS a contract. Both agent and replicant run the same assertions.
        let mut contract = hkask_types::TaskContract::new(
            "inline".into(),
            task.owner,
            verifier,
            &task,
            vec![],
        );
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
            (Some(p), Some(h)) => {
                format!("Each task should be approximately {p} story points or {h} hours.")
            }
            (Some(p), None) => format!("Each task should be approximately {p} story points."),
            (None, Some(h)) => format!("Each task should be approximately {h} hours."),
            (None, None) => "Aim for tasks of 2-8 hours each.".to_string(),
        };
        let prompt = format!(
            "Decompose this project into kanban tasks.

             Project: {project_description}
             Sizing: {sizing_guidance}

             Return JSON with a tasks array. Each task: title, description, story_points (int),             estimated_hours (float), labels (array), criteria (array), priority, dependencies.             Include recomposition strategy.",
            project_description = project_description,
            sizing_guidance = sizing_guidance
        );
        Ok(prompt)
    }

    /// Populate the board from an LLM decomposition JSON response.
    ///
    /// REQ: KAN-SVC-020b
    /// pre:  board_id is valid; json_output is a JSON string from the LLM
    /// post: tasks from the JSON are created on the board; returns count and recomposition info
    pub fn decompose_populate(
        &self,
        board_id: BoardId,
        owner: WebID,
        json_output: &str,
    ) -> Result<(usize, Option<String>), KanbanError> {
        // Verify board exists
        self.board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;

        // Parse + validate the JSON
        let parsed: serde_json::Value = serde_json::from_str(json_output)
            .map_err(|e| KanbanError::InvalidInput(format!("Invalid JSON: {e}")))?;

        // Schema validation
        if parsed.get("tasks").is_none() {
            return Err(KanbanError::InvalidInput(
                "JSON must have a tasks array at top level".into()
            ));
        }
        let tasks_array = parsed["tasks"].as_array()
            .ok_or_else(|| KanbanError::InvalidInput("'tasks' must be an array".into()))?;
        if tasks_array.is_empty() {
            return Err(KanbanError::InvalidInput("'tasks' array is empty — nothing to create".into()));
        }

        // Validate each task has a title
        for (i, task_val) in tasks_array.iter().enumerate() {
            if task_val.get("title").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                return Err(KanbanError::InvalidInput(
                    format!("Task {} is missing 'title' field", i + 1)
                ));
            }
        }

        // Create phases from recomposition if present
        let mut phase_map: std::collections::HashMap<String, hkask_types::PhaseId> =
            std::collections::HashMap::new();
        if let Some(phases) = parsed["recomposition"]["phases"].as_array() {
            for (i, phase_val) in phases.iter().enumerate() {
                let name = phase_val["name"].as_str().unwrap_or("Unnamed");
                let desc = phase_val["description"].as_str();
                let mut phase = hkask_types::Phase::new(name.to_string(), i as u32);
                if let Some(d) = desc {
                    phase = phase.with_description(d.to_string());
                }
                // Store task_labels mapping for assignment
                if let Some(labels) = phase_val["task_labels"].as_array() {
                    for label in labels {
                        if let Some(l) = label.as_str() {
                            phase_map.insert(l.to_lowercase(), phase.id);
                        }
                    }
                }
                self.board_add_phase(board_id, &phase.name, phase.order)?;
            }
        }

        // Create tasks
        let mut created = 0usize;
        for task_val in tasks_array {
            let title = task_val["title"].as_str().unwrap_or("Untitled");
            let description = task_val["description"].as_str().map(|s| s.to_string());
            let story_points = task_val["story_points"].as_u64().map(|n| n as u32);
            let estimated_hours = task_val["estimated_hours"].as_f64();
            let labels: Vec<String> = task_val["labels"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let criteria: Vec<hkask_types::VerificationCriterion> = task_val["criteria"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()
                    .map(|s| hkask_types::VerificationCriterion::new(s.into()))).collect())
                .unwrap_or_default();
            let priority = task_val["priority"].as_str()
                .and_then(|s| hkask_types::Priority::parse_str(s));

            let mut spec = TaskSpec::new(title.into());
            if let Some(d) = description { spec = spec.with_description(d); }
            if !criteria.is_empty() { spec = spec.with_criteria(criteria); }
            if let Some(sp) = story_points { spec = spec.with_story_points(sp); }
            if let Some(eh) = estimated_hours { spec = spec.with_estimated_hours(eh); }
            if let Some(p) = priority { spec = spec.with_priority(p); }

            // Assign phase if any label matches a phase
            if !phase_map.is_empty() && !labels.is_empty() {
                for label in &labels {
                    if let Some(pid) = phase_map.get(&label.to_lowercase()) {
                        spec = spec.with_phase(*pid);
                        break;
                    }
                }
            }

            if !labels.is_empty() { spec = spec.with_labels(labels); }

            self.task_create(board_id, spec, owner)?;
            created += 1;
        }

        // Extract recomposition strategy
        let recomposition = parsed["recomposition"]["strategy"]
            .as_str()
            .or_else(|| parsed["recomposition"].as_str())
            .map(String::from);

        Ok((created, recomposition))
    }

    /// Spawn a sub-replicant to execute a task.
    ///
    /// REQ: KAN-SVC-021
    pub fn spawn_task(
        &self,
        task_id: TaskId,
        spawn_spec: hkask_types::SpawnSpec,
    ) -> Result<String, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        // Attempt live pod creation if PodManager is attached
        if let Some(ref pm) = self.pod_manager {
            let pod_name = format!("kanban-{}", task.title.chars().take(20).collect::<String>().replace(' ', "-"));
            let persona_yaml = format!(
                "agent:
  name: {name}
  type: bot
  version: 0.1.0
                 charter:
  description: Task: {title}
  editor: kanban
                 capabilities:
{skills}
",
                name = pod_name,
                title = task.title,
                skills = spawn_spec.delegated_skills.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("
"),
            );
            match hkask_agents::pod::AgentPersona::from_yaml(&persona_yaml) {
                Ok(persona) => {
                    let rt = tokio::runtime::Handle::current();
                    match rt.block_on(pm.create_pod("kanban-agent", &persona, Some(pod_name.clone()))) {
                        Ok(pod_id) => {
                            match rt.block_on(pm.activate_pod(&pod_id)) {
                                Ok(()) => {
                                    let webid = persona.webid();
                                    let note = format!(
                                        "Pod activated: id={}, webid={}, skills={:?}, tools={:?}",
                                        pod_id, webid.redacted_display(), spawn_spec.delegated_skills, spawn_spec.tool_servers
                                    );
                                    let comment = hkask_types::Comment::new(task_id, task.owner, note);
                                    task.comments.push(comment);
                                    task.updated_at = chrono::Utc::now();
                                    self.update_task_triple(&task)?;
                                    return Ok(format!("Pod {} activated (webid: {}). Use /kanban note {} to communicate.",
                                        pod_id, webid.redacted_display(), task_id));
                                }
                                Err(e) => return Ok(format!("Pod created but activation failed: {}", e)),
                            }
                        }
                        Err(e) => return Ok(format!("Pod creation failed: {}", e)),
                    }
                }
                Err(e) => return Ok(format!("Persona parse failed: {}", e)),
            }
        }

        // Fallback: string-based spawn when no PodManager
        let spawn_note = format!(
            "Spawn configured (no PodManager): level={}, skills={:?}, memory={}, tools={:?}",
            spawn_spec.delegation_level, spawn_spec.delegated_skills,
            spawn_spec.memory_scope, spawn_spec.tool_servers,
        );
        let comment = hkask_types::Comment::new(task_id, task.owner, spawn_note);
        task.comments.push(comment);
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(format!("Spawn configured for '{}' (no PodManager — string mode). Skills: {:?}", task.title, spawn_spec.delegated_skills))
    }

    // ── Comments (mini-REPL per task) ─────────────────────────────────

    /// Append a comment to a task.
    ///
    /// REQ: KAN-SVC-030
    /// pre:  task_id is valid; author is valid WebID; body is non-empty
    /// post: comment is appended to the task's comment thread
    pub fn task_comment(
        &self,
        task_id: TaskId,
        author: WebID,
        body: &str,
    ) -> Result<Comment, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        let comment = Comment::new(task_id, author, body.to_string());
        task.comments.push(comment.clone());
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(comment)
    }

    /// List all comments on a task.
    ///
    /// REQ: KAN-SVC-031
    pub fn task_comments(&self, task_id: TaskId) -> Result<Vec<Comment>, KanbanError> {
        let task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        Ok(task.comments)
    }

    // ── Deliverables (file path / URL links) ──────────────────────────

    /// Add a deliverable link to a task.
    ///
    /// REQ: KAN-SVC-032
    /// pre:  task_id is valid; path is a non-empty file path or URL
    /// post: path is appended to the task's deliverable list
    pub fn task_add_deliverable(&self, task_id: TaskId, path: &str) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        task.deliverables.push(path.to_string());
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }

    // ── Phases ────────────────────────────────────────────────────────

    /// Add a phase to a board.
    ///
    /// REQ: KAN-SVC-033
    pub fn board_add_phase(
        &self,
        board_id: BoardId,
        name: &str,
        order: u32,
    ) -> Result<Phase, KanbanError> {
        let mut board = self
            .board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;
        let phase = Phase::new(name.to_string(), order);
        board.phases.push(phase.clone());
        self.update_board_triple(&board)?;
        Ok(phase)
    }

    /// Set a task's phase.
    ///
    /// REQ: KAN-SVC-034
    pub fn task_set_phase(&self, task_id: TaskId, phase_id: PhaseId) -> Result<Task, KanbanError> {
        let mut task = self
            .task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        task.phase_id = Some(phase_id);
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }

    /// List tasks in a specific phase.
    ///
    /// REQ: KAN-SVC-035
    pub fn tasks_by_phase(
        &self,
        board_id: BoardId,
        phase_id: PhaseId,
    ) -> Result<Vec<Task>, KanbanError> {
        let all = self.task_list(board_id, TaskFilter::all())?;
        Ok(all
            .into_iter()
            .filter(|t| t.phase_id == Some(phase_id))
            .collect())
    }

    // ── Lifecycle operations (P0) ─────────────────────────────────────

    /// Delete a task and its board index entry.
    ///
    /// REQ: KAN-SVC-040
    /// pre:  task_id is valid
    /// post: task triple and index triple are soft-deleted
    pub fn task_delete(&self, task_id: TaskId) -> Result<(), KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        // Close the task triple
        let triples = self.store
            .query_by_entity_attribute(TASK_ENTITY, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        for t in &triples {
            self.store.close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("triple close failed: {e}")))?;
        }

        // Close the index triple
        let index_entity = format!("{BOARD_TASKS_PREFIX}{}", task.board_id);
        let idx_triples = self.store
            .query_by_entity_attribute(&index_entity, &task_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("index query failed: {e}")))?;
        for t in &idx_triples {
            self.store.close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("index close failed: {e}")))?;
        }

        Ok(())
    }

    /// Unassign a task — remove the assignee.
    ///
    /// REQ: KAN-SVC-041
    /// pre:  task_id is valid
    /// post: task.assignee is set to None
    pub fn task_unassign(&self, task_id: TaskId) -> Result<Task, KanbanError> {
        let mut task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;
        task.assignee = None;
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;
        Ok(task)
    }

    /// Reopen a completed task — move from Done back to InProgress.
    ///
    /// REQ: KAN-SVC-042
    /// pre:  task_id refers to a task in Done status
    /// post: task moves to InProgress, verification cleared
    pub fn task_reopen(&self, task_id: TaskId) -> Result<Task, KanbanError> {
        let mut task = self.task_get(task_id)?
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
    /// REQ: KAN-SVC-043
    /// pre:  board_id is valid
    /// post: board triple and all associated task/index triples are soft-deleted
    pub fn board_delete(&self, board_id: BoardId) -> Result<usize, KanbanError> {
        let board = self.board_get(board_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("board {board_id}")))?;

        // Delete all tasks on this board
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let task_count = tasks.len();
        for task in &tasks {
            let _ = self.task_delete(task.id);
        }

        // Close the board triple
        let triples = self.store
            .query_by_entity_attribute(BOARD_ENTITY, &board_id.to_string())
            .map_err(|e| KanbanError::Internal(format!("triple query failed: {e}")))?;
        for t in &triples {
            self.store.close_by_id(&t.id)
                .map_err(|e| KanbanError::Internal(format!("triple close failed: {e}")))?;
        }
        let _ = board;

        Ok(task_count)
    }

    // ── De-jamming ────────────────────────────────────────────────────

    /// Scan a board for stuck states and return a de-jam report.
    ///
    /// REQ: KAN-SVC-044
    /// pre:  board_id is valid
    /// post: returns a report of stuck tasks with suggested fixes
    pub fn unjam_report(&self, board_id: BoardId) -> Result<Vec<UnjamItem>, KanbanError> {
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let now = chrono::Utc::now();
        let mut items = Vec::new();

        for task in &tasks {
            // Stuck in InProgress: no movement for > estimated hours * 2
            if task.status == TaskStatus::InProgress
                || task.status == TaskStatus::Review
            {
                if let Some(hours) = task.estimated_hours {
                    let elapsed = (now - task.updated_at).num_hours();
                    if elapsed > (hours as i64) * 2 {
                        items.push(UnjamItem {
                            task_id: task.id,
                            task_title: task.title.clone(),
                            issue: format!("Stuck in {} for {}h (estimated {}h)", task.status, elapsed, hours),
                            suggestion: "Consider escalating or reassigning.".into(),
                        });
                    }
                }
            }

            // Assigned but never started (> 24h in Backlog/Ready)
            if task.assignee.is_some()
                && (task.status == TaskStatus::Backlog || task.status == TaskStatus::Ready)
            {
                let elapsed = (now - task.updated_at).num_hours();
                if elapsed > 24 {
                    items.push(UnjamItem {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        issue: format!("Assigned but not started for {}h", elapsed),
                        suggestion: "Consider unassigning or escalating.".into(),
                    });
                }
            }

            // Done without verification
            if task.status == TaskStatus::Done && task.verification.is_none() {
                items.push(UnjamItem {
                    task_id: task.id,
                    task_title: task.title.clone(),
                    issue: "Completed without verification.".into(),
                    suggestion: "Reopen and verify, or verify retroactively.".into(),
                });
            }
        }

        Ok(items)
    }

    /// Auto-fix clear-cut stuck states. Returns what was fixed.
    ///
    /// REQ: KAN-SVC-045
    /// pre:  board_id is valid
    /// post: stale assignments are unassigned; unverified Done tasks are reopened
    pub fn unjam_fix(&self, board_id: BoardId) -> Result<Vec<UnjamFix>, KanbanError> {
        let tasks = self.task_list(board_id, TaskFilter::all())?;
        let now = chrono::Utc::now();
        let mut fixes = Vec::new();

        for task in &tasks {
            // Auto-unassign stale assignments (>24h in Backlog/Ready with assignee)
            if task.assignee.is_some()
                && (task.status == TaskStatus::Backlog || task.status == TaskStatus::Ready)
            {
                let elapsed = (now - task.updated_at).num_hours();
                if elapsed > 24 {
                    match self.task_unassign(task.id) {
                        Ok(_) => fixes.push(UnjamFix {
                            task_id: task.id,
                            task_title: task.title.clone(),
                            action: format!("Unassigned after {}h idle", elapsed),
                        }),
                        Err(e) => fixes.push(UnjamFix {
                            task_id: task.id,
                            task_title: task.title.clone(),
                            action: format!("Unassign failed: {}", e),
                        }),
                    }
                }
            }

            // Auto-reopen unverified Done tasks
            if task.status == TaskStatus::Done && task.verification.is_none() {
                match self.task_reopen(task.id) {
                    Ok(_) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: "Reopened (was Done without verification)".into(),
                    }),
                    Err(e) => fixes.push(UnjamFix {
                        task_id: task.id,
                        task_title: task.title.clone(),
                        action: format!("Reopen failed: {}", e),
                    }),
                }
            }
        }

        Ok(fixes)
    }

    /// Generate a structured prompt for LLM-mediated verification.
    ///
    /// REQ: KAN-SVC-050
    /// pre:  task_id refers to a task in Review with acceptance criteria
    /// post: returns a prompt that, when fed to an LLM, produces structured verification JSON
    pub fn verification_prompt(
        &self,
        task_id: TaskId,
        evidence: &str,
    ) -> Result<String, KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if task.criteria.is_empty() {
            return Err(KanbanError::InvalidInput("Task has no acceptance criteria".into()));
        }

        let criteria_text: Vec<String> = task.criteria.iter()
            .enumerate()
            .map(|(i, c)| format!("{}. {}", i + 1, c.description))
            .collect();

        Ok(format!(
            "Verify whether this task satisfies its acceptance criteria.

             Task: {title}
             Evidence: {evidence}

             Criteria:
{criteria}

             Return JSON with: passed (bool), reasoning (string),              criteria_results (array of objects with: criterion, satisfied, evidence_found, feedback).              Be rigorous. A criterion is satisfied ONLY if concrete evidence exists.",
            title = task.title,
            evidence = evidence,
            criteria = criteria_text.join("
"),
        ))
    }

    /// Apply an LLM verification response to a task.
    ///
    /// REQ: KAN-SVC-051
    /// pre:  task_id refers to a task in Review; llm_json is valid verification JSON
    /// post: task.verification is set; task moves to Done if passed
    pub fn verify_with_llm(
        &self,
        task_id: TaskId,
        verifier: WebID,
        llm_json: &str,
    ) -> Result<(Task, Verification), KanbanError> {
        let mut task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        if task.status != TaskStatus::Review {
            return Err(KanbanError::InvalidTransition {
                task: task_id,
                from: task.status,
                to: TaskStatus::Done,
            });
        }

        let parsed: serde_json::Value = serde_json::from_str(llm_json)
            .map_err(|e| KanbanError::InvalidInput(format!("Invalid LLM JSON: {e}")))?;

        let passed = parsed["passed"].as_bool().unwrap_or(false);
        let reasoning = parsed["reasoning"].as_str().unwrap_or("No reasoning provided").to_string();

        let verification = Verification::new(passed, reasoning, verifier);
        task.verification = Some(verification.clone());

        if passed {
            task.status = TaskStatus::Done;
        }
        task.updated_at = chrono::Utc::now();
        self.update_task_triple(&task)?;

        Ok((task, verification))
    }

    // ── Kata Integration (task-scoped scientific thinking) ──────────

    /// Generate a coaching kata prompt scoped to this task.
    ///
    /// REQ: KAN-SVC-060
    /// pre:  task_id is valid
    /// post: returns a 5-question coaching prompt preloaded with task context.
    ///       The task's criteria ARE the target condition. The task's state,
    ///       comments, and deliverables ARE the actual condition. The unjam
    ///       report identifies obstacles.
    pub fn task_coaching_prompt(
        &self,
        task_id: TaskId,
    ) -> Result<String, KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        // Build target condition from acceptance criteria
        let target = if task.criteria.is_empty() {
            format!("Complete task '{}'", task.title)
        } else {
            task.criteria.iter()
                .map(|c| format!("- {}", c.description))
                .collect::<Vec<_>>()
                .join("
")
        };

        // Build actual condition from the full evidence corpus:
        // comments (chat between agent/replicant), deliverables (file links),
        // and task state (status, assignee, timing).
        let mut evidence = format!(
            "Status: {}
Assignee: {}
Est. hours: {}
Story points: {}
Updated: {}",
            task.status,
            task.assignee.map(|a| a.redacted_display()).unwrap_or_else(|| "none".into()),
            task.estimated_hours.map_or("?".into(), |h| format!("{}h", h)),
            task.story_points.map_or("?".into(), |p| format!("{}pt", p)),
            task.updated_at.format("%Y-%m-%d %H:%M"),
        );

        // Deliverables — the actual work output
        if !task.deliverables.is_empty() {
            evidence.push_str("

Deliverables (file links = work output):");
            for d in &task.deliverables {
                evidence.push_str(&format!("
  - {}", d));
            }
        }

        // Comments — the agent/replicant chat stream
        if !task.comments.is_empty() {
            evidence.push_str("

Comment thread (agent/replicant communication):");
            for c in &task.comments {
                evidence.push_str(&format!(
                    "
  [{}] {}: {}",
                    c.created_at.format("%H:%M"),
                    c.author.redacted_display(),
                    c.body,
                ));
            }
        }

        let actual = evidence;

        Ok(format!(
            "Coaching Kata — Task: {title}

             Q1 — Target Condition:
{target}

             Q2 — Actual Condition:
{actual}

             Q3 — Obstacles: What is preventing this task from reaching the target?              Which ONE obstacle are you addressing now?

             Q4 — Next Step: What experiment will you run? What do you expect?

             Q5 — How quickly can we go and see what we learned?

             Respond with your answers. The coach will guide, not solve.",
            title = task.title,
        ))
    }

    /// Generate an improvement kata prompt scoped to this task.
    ///
    /// REQ: KAN-SVC-061
    /// pre:  task_id is valid
    /// post: returns a 4-step improvement kata prompt with task as subject
    pub fn task_improvement_prompt(
        &self,
        task_id: TaskId,
    ) -> Result<String, KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        let direction = task.description.as_deref().unwrap_or(&task.title);
        let mut current = format!(
            "Task '{}' is in status '{}'.
Evidence: {} deliverables, {} comments, {} criteria.",
            task.title, task.status, task.deliverables.len(), task.comments.len(), task.criteria.len()
        );
        if !task.deliverables.is_empty() {
            current.push_str("
Deliverables:");
            for d in &task.deliverables { current.push_str(&format!("
  {}", d)); }
        }
        if !task.comments.is_empty() {
            current.push_str("
Recent comments:");
            for c in task.comments.iter().rev().take(3) {
                current.push_str(&format!("
  [{}] {}", c.created_at.format("%H:%M"), c.body));
            }
        }

        Ok(format!(
            "Improvement Kata — Task: {title}

             Step 1 — Understand the Direction:
{direction}

             Step 2 — Grasp the Current Condition:
{current}

             Step 3 — Establish the Next Target Condition:
             What specific, measurable condition do you want to achieve?

             Step 4 — Iterate: What ONE experiment will you run? What do you predict?
             Plan → Do → Check → Act. Record your experiment and result.",
            title = task.title,
            direction = direction,
            current = current,
        ))
    }

    /// Generate a starter kata observation drill for a task sub-problem.
    ///
    /// REQ: KAN-SVC-062
    /// pre:  task_id is valid; sub_problem describes what's being examined
    /// post: returns an observation drill prompt distinguishing facts from interpretations
    pub fn task_practice_prompt(
        &self,
        task_id: TaskId,
        sub_problem: &str,
    ) -> Result<String, KanbanError> {
        let task = self.task_get(task_id)?
            .ok_or_else(|| KanbanError::NotFound(format!("task {task_id}")))?;

        Ok(format!(
            "Starter Kata — Observation Drill
             Task: {title}
             Focus: {sub_problem}

             List what you OBSERVE (facts, data, evidence):
             1. 
2. 
3. 

             List what you INTERPRET (assumptions, guesses, theories):
             1. 
2. 
3. 

             For each interpretation, ask: How would I test this?              What experiment would distinguish this interpretation from alternatives?",
            title = task.title,
            sub_problem = sub_problem,
        ))
    }

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
    use std::sync::Arc;
    use super::*;
    use hkask_storage::Store;
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
