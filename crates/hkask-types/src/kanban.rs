//! Kanban types — Agent coordination via headless task boards.
//!
//! Every type carries `owner: WebID` (P12 — anonymous agency prohibition).
//! Task status transitions are column-ordered: Backlog → Ready → InProgress → Review → Done.
//! Verification criteria accept natural-language acceptance specs with optional LLM evaluation prompts.

use crate::id::{BoardId, ColumnId, CommentId, PhaseId, TaskId, WebID};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


// ── Priority ────────────────────────────────────────────────────────────────

/// Priority level for kanban tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
            Priority::Critical => "critical",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Priority::Low),
            "medium" | "med" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            "critical" | "crit" => Some(Priority::Critical),
            _ => None,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Task Status ────────────────────────────────────────────────────────────

/// TaskStatus — lifecycle state of a kanban task.
///
/// Column ordering is strict: transitions may only advance forward
/// or regress one step backward. Skipping columns is prohibited.
///
/// ```text
/// Backlog → Ready → InProgress → Review → Done
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is queued, not yet ready for work.
    Backlog,
    /// Task is ready to be picked up.
    Ready,
    /// Task is actively being worked on.
    InProgress,
    /// Task is complete and awaiting review/verification.
    Review,
    /// Task has been verified and is done.
    Done,
}

impl TaskStatus {
    /// REQ: KAN-001
    /// pre:  self is any valid TaskStatus
    /// post: returns the string representation (lowercase)
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Backlog => "backlog",
            TaskStatus::Ready => "ready",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Review => "review",
            TaskStatus::Done => "done",
        }
    }

    /// REQ: KAN-002
    /// pre:  s is a case-insensitive string
    /// post: returns Some(TaskStatus) if valid, None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "backlog" => Some(TaskStatus::Backlog),
            "ready" => Some(TaskStatus::Ready),
            "in_progress" | "inprogress" | "in-progress" => Some(TaskStatus::InProgress),
            "review" => Some(TaskStatus::Review),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }

    /// REQ: KAN-003
    /// pre:  self is any valid TaskStatus
    /// post: returns true iff the transition from self to `target` is valid
    ///       (forward one step, or backward one step — no skipping)
    pub fn can_transition_to(&self, target: TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (self, target),
            (Backlog, Ready)
                | (Ready, Backlog)
                | (Ready, InProgress)
                | (InProgress, Ready)
                | (InProgress, Review)
                | (Review, InProgress)
                | (Review, Done)
        )
    }

    /// REQ: KAN-004
    /// pre:  self is any valid TaskStatus
    /// post: returns the next status in the workflow, or None if already Done
    pub fn next(&self) -> Option<TaskStatus> {
        match self {
            TaskStatus::Backlog => Some(TaskStatus::Ready),
            TaskStatus::Ready => Some(TaskStatus::InProgress),
            TaskStatus::InProgress => Some(TaskStatus::Review),
            TaskStatus::Review => Some(TaskStatus::Done),
            TaskStatus::Done => None,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).ok_or_else(|| format!("invalid TaskStatus: {s}"))
    }
}

// ── Column Definition ──────────────────────────────────────────────────────

/// ColumnDef — definition of a column on a board.
///
/// Each column maps to a `TaskStatus`. The column ordering on a board
/// determines the valid state transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDef {
    /// Unique identifier for this column.
    pub id: ColumnId,
    /// Display name (e.g., "Backlog", "In Progress").
    pub name: String,
    /// The task status this column represents.
    pub status: TaskStatus,
    /// Position in the column ordering (0-based).
    pub position: u32,
    /// Optional WIP limit — maximum tasks allowed in this column.
    /// None means no limit. Per Anderson: WIP limits are the core
    /// mechanism that exposes system problems and stimulates collaboration.
    pub wip_limit: Option<u32>,
}

impl ColumnDef {
    /// REQ: KAN-005
    /// pre:  name is non-empty; status is valid; position is >= 0
    /// post: returns a new ColumnDef with a random ColumnId, no WIP limit
    pub fn new(name: String, status: TaskStatus, position: u32) -> Self {
        Self {
            id: ColumnId::new(),
            name,
            status,
            position,
            wip_limit: None,
        }
    }

    pub fn with_wip_limit(mut self, limit: u32) -> Self {
        self.wip_limit = Some(limit);
        self
    }

}

// ── Verification Criterion ─────────────────────────────────────────────────

/// VerificationCriterion — an acceptance criterion for task completion.
///
/// Holds a natural-language specification of what "done" means for this task,
/// plus an optional LLM evaluation prompt for automated verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCriterion {
    /// Human-readable acceptance spec.
    pub description: String,
    /// Optional prompt for LLM-mediated evaluation.
    pub llm_prompt: Option<String>,
}

impl VerificationCriterion {
    /// REQ: KAN-006
    /// pre:  description is non-empty
    /// post: returns a VerificationCriterion with no LLM prompt
    pub fn new(description: String) -> Self {
        Self {
            description,
            llm_prompt: None,
        }
    }

    /// REQ: KAN-007
    /// pre:  self is valid; llm_prompt is non-empty
    /// post: returns self with llm_prompt set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_llm_prompt(mut self, prompt: String) -> Self {
        self.llm_prompt = Some(prompt);
        self
    }
}

// ── Verification Result ────────────────────────────────────────────────────

/// Verification — result of task verification.
///
/// Produced by `task_verify`: either an LLM-mediated evaluation against
/// the task's acceptance criteria, or a human-in-the-loop confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    /// Whether the task passed verification.
    pub passed: bool,
    /// Human-readable reasoning for the verdict.
    pub reasoning: String,
    /// The WebID of the verifier (LLM replicant or human).
    pub verifier: WebID,
    /// When the verification occurred.
    pub verified_at: DateTime<Utc>,
}

impl Verification {
    /// REQ: KAN-008
    /// pre:  verifier is a valid WebID
    /// post: returns Verification with verified_at=now
    pub fn new(passed: bool, reasoning: String, verifier: WebID) -> Self {
        Self {
            passed,
            reasoning,
            verifier,
            verified_at: Utc::now(),
        }
    }
}


// ── Phase ──────────────────────────────────────────────────────────────────

/// Phase — a grouping category for tasks within a board.
///
/// Phases group work for reassembly: when all tasks in a phase complete,
/// their deliverables can be composed into a coherent output.
/// Unlike columns (which track workflow state), phases track project structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phase {
    pub id: PhaseId,
    pub name: String,
    pub description: Option<String>,
    /// Display order (0-based).
    pub order: u32,
    /// When the phase was created.
    pub created_at: DateTime<Utc>,
}

impl Phase {
    /// REQ: KAN-040
    pub fn new(name: String, order: u32) -> Self {
        Self {
            id: PhaseId::new(),
            name,
            description: None,
            order,
            created_at: Utc::now(),
        }
    }

    /// REQ: KAN-041
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
}

// ── Board ──────────────────────────────────────────────────────────────────

/// Board — a kanban board containing columns and tasks.
///
/// Every board carries an `owner: WebID` for P12 compliance.
/// Boards are isolated — only members (agents assigned to the board)
/// can view or modify its contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    /// Unique board identifier.
    pub id: BoardId,
    /// Human-readable board name.
    pub name: String,
    /// The replicant who created/manages this board.
    pub owner: WebID,
    /// Columns in display order (position-sorted).
    pub columns: Vec<ColumnDef>,
    /// Project phases for work grouping and reassembly.
    pub phases: Vec<Phase>,
    /// When the board was created.
    pub created_at: DateTime<Utc>,
}

impl Board {
    /// REQ: KAN-009
    /// pre:  name is non-empty; owner is a valid WebID; columns is non-empty
    /// post: returns a new Board with created_at=now and a random BoardId
    pub fn new(name: String, owner: WebID, columns: Vec<ColumnDef>) -> Self {
        Self {
            id: BoardId::new(),
            name,
            owner,
            columns,
            phases: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// REQ: KAN-010
    /// pre:  self.columns is sorted by position
    /// post: returns the first column (by position) — typically Backlog
    pub fn first_column(&self) -> Option<&ColumnDef> {
        self.columns.first()
    }

    /// REQ: KAN-011
    /// pre:  self.columns is sorted by position
    /// post: returns the last column (by position) — typically Done
    pub fn last_column(&self) -> Option<&ColumnDef> {
        self.columns.last()
    }

    /// REQ: KAN-012
    /// pre:  status is a valid TaskStatus
    /// post: returns the ColumnDef matching the given status, if present
    pub fn column_for_status(&self, status: TaskStatus) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.status == status)
    }
}

// ── Task ───────────────────────────────────────────────────────────────────

/// TaskSpec — input specification for creating a new task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Short title for the task.
    pub title: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Acceptance criteria — what "done" means.
    pub criteria: Vec<VerificationCriterion>,
    /// Optional agent assignment (requires consent).
    pub assignee: Option<WebID>,
    /// Story points for relative sizing (agile convention).
    pub story_points: Option<u32>,
    /// Estimated hours for completion.
    pub estimated_hours: Option<f64>,
    /// Labels/tags for categorization.
    pub labels: Vec<String>,
    /// Priority level.
    pub priority: Option<Priority>,
    /// Optional phase grouping.
    pub phase_id: Option<PhaseId>,
}

impl TaskSpec {
    /// REQ: KAN-013
    /// pre:  title is non-empty
    /// post: returns a TaskSpec with no description, criteria, or assignee
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            criteria: Vec::new(),
            assignee: None,
            story_points: None,
            estimated_hours: None,
            labels: Vec::new(),
            priority: None,
            phase_id: None,
        }
    }

    /// REQ: KAN-014
    /// pre:  self is valid
    /// post: returns self with description set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    /// REQ: KAN-015
    /// pre:  self is valid
    /// post: returns self with criteria set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_criteria(mut self, criteria: Vec<VerificationCriterion>) -> Self {
        self.criteria = criteria;
        self
    }

    /// REQ: KAN-016
    /// pre:  self is valid; assignee is a valid WebID
    /// post: returns self with assignee set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_assignee(mut self, assignee: WebID) -> Self {
        self.assignee = Some(assignee);
        self
    }

    /// REQ: KAN-016b
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_story_points(mut self, points: u32) -> Self {
        self.story_points = Some(points);
        self
    }

    /// REQ: KAN-016c
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_estimated_hours(mut self, hours: f64) -> Self {
        self.estimated_hours = Some(hours);
        self
    }

    /// REQ: KAN-016d
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// REQ: KAN-016e
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// REQ: KAN-016f
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_phase(mut self, phase_id: PhaseId) -> Self {
        self.phase_id = Some(phase_id);
        self
    }
}

/// Task — a single work item on a kanban board.
///
/// Every task carries `owner: WebID` (P12) — the creator of the task.
/// Assignment is separate and requires agent consent (P1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: TaskId,
    /// The board this task belongs to.
    pub board_id: BoardId,
    /// Short title.
    pub title: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Current status (determines which column the task is in).
    pub status: TaskStatus,
    /// The replicant who created this task (P12).
    pub owner: WebID,
    /// The agent assigned to work on this task (requires consent).
    pub assignee: Option<WebID>,
    /// Acceptance criteria for task completion.
    pub criteria: Vec<VerificationCriterion>,
    /// Verification result, if the task has been verified.
    pub verification: Option<Verification>,
    /// Story points (relative sizing, agile convention).
    pub story_points: Option<u32>,
    /// Estimated hours for completion.
    pub estimated_hours: Option<f64>,
    /// Priority level.
    pub priority: Option<Priority>,
    /// Labels/tags for categorization and filtering.
    pub labels: Vec<String>,
    /// Task comments — mini-REPL thread for in-process communication.
    pub comments: Vec<Comment>,
    /// Deliverable links — file paths or URLs pointing to work outputs.
    pub deliverables: Vec<String>,
    /// Optional phase grouping for work reassembly.
    pub phase_id: Option<PhaseId>,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// REQ: KAN-017
    /// pre:  board_id is a valid BoardId; spec contains non-empty title; owner is valid
    /// post: returns a new Task with status=Backlog, created_at=now, updated_at=now
    pub fn new(board_id: BoardId, spec: TaskSpec, owner: WebID) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            board_id,
            title: spec.title,
            description: spec.description,
            status: TaskStatus::Backlog,
            owner,
            assignee: spec.assignee,
            criteria: spec.criteria,
            verification: None,
            story_points: None,
            estimated_hours: None,
            labels: Vec::new(),
            priority: None,
            comments: Vec::new(),
            deliverables: Vec::new(),
            phase_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// REQ: KAN-018
    /// pre:  target is a valid transition from self.status
    /// post: returns true iff self.status.can_transition_to(target)
    pub fn can_move_to(&self, target: TaskStatus) -> bool {
        self.status.can_transition_to(target)
    }
}


// ── Comment ────────────────────────────────────────────────────────────────

/// Comment — a text note appended to a task by an agent.
///
/// Forms a mini-REPL thread attached to each task: agents append notes
/// as they work, and the coordinating replicant responds inline.
/// Every comment carries `author: WebID` (P12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub task_id: TaskId,
    pub author: WebID,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl Comment {
    /// REQ: KAN-050
    pub fn new(task_id: TaskId, author: WebID, body: String) -> Self {
        Self { id: CommentId::new(), task_id, author, body, created_at: Utc::now() }
    }
}

// ── Filter ─────────────────────────────────────────────────────────────────

/// TaskFilter — criteria for listing/filtering tasks on a board.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by status (column).
    pub status: Option<TaskStatus>,
    /// Filter by assignee.
    pub assignee: Option<WebID>,
    /// Filter by priority.
    pub priority: Option<Priority>,
    /// Limit the number of results.
    pub limit: Option<usize>,
}

impl TaskFilter {
    /// REQ: KAN-019
    /// post: returns an empty filter (matches all tasks)
    pub fn all() -> Self {
        Self {
            status: None,
            assignee: None,
            priority: None,
            limit: None,
        }
    }

    /// REQ: KAN-020
    /// pre:  status is a valid TaskStatus
    /// post: returns a filter matching only tasks with the given status
    pub fn by_status(status: TaskStatus) -> Self {
        Self {
            status: Some(status),
            assignee: None,
            priority: None,
            limit: None,
        }
    }

    /// REQ: KAN-021
    /// pre:  assignee is a valid WebID
    /// post: returns a filter matching only tasks assigned to the given agent
    pub fn by_assignee(assignee: WebID) -> Self {
        Self {
            status: None,
            assignee: Some(assignee),
            priority: None,
            limit: None,
        }
    }

    /// REQ: KAN-021b
    pub fn by_priority(priority: Priority) -> Self {
        Self {
            status: None,
            assignee: None,
            priority: Some(priority),
            limit: None,
        }
    }
}

// ── Consent Proof ──────────────────────────────────────────────────────────

/// ConsentProof — evidence that an agent has consented to a task assignment.
///
/// P1 (User Sovereignty) §4: "No agent is assigned work without consent."
/// This type is deliberately opaque — the service layer validates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentProof {
    /// The WebID of the consenting agent.
    pub agent: WebID,
    /// The task being consented to.
    pub task_id: TaskId,
    /// When consent was given.
    pub consented_at: DateTime<Utc>,
}

impl ConsentProof {
    /// REQ: KAN-022
    /// pre:  agent and task_id are valid
    /// post: returns ConsentProof with consented_at=now
    pub fn new(agent: WebID, task_id: TaskId) -> Self {
        Self {
            agent,
            task_id,
            consented_at: Utc::now(),
        }
    }
}

// ── Task Contract (rSolidity) ─────────────────────────────────────────────

/// TaskContract — a kanban task assignment expressed as an rSolidity contract.
///
/// Binds delegator and delegate with:
/// - Pre-conditions: acceptance criteria (what must be true before work starts)
/// - Post-conditions: verification conditions (what must be true to accept work)
/// - OCAP gates: capability tokens delegated for the work
/// - Gas limit: maximum energy budget
/// - Timeout: maximum execution time
///
/// Maps to rSolidity's require!/assert!/emit! macros for CNS-observable
/// contract execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskContract {
    /// Name of the capability package used.
    pub package_name: String,
    /// The replicant delegating the work.
    pub delegator: crate::WebID,
    /// The agent receiving the delegation.
    pub delegate: crate::WebID,
    /// The task this contract governs.
    pub task_id: TaskId,
    /// Task title for display.
    pub task_title: String,
    /// Pre-conditions (acceptance criteria) — require!() gates.
    /// These must be satisfied before work can be considered complete.
    pub pre_conditions: Vec<String>,
    /// Post-conditions — assert!() gates.
    /// These are verified after the agent submits deliverables.
    pub post_conditions: Vec<String>,
    /// OCAP capability token specs delegated.
    pub ocap_gates: Vec<String>,
    /// Maximum gas/energy budget.
    pub gas_limit: u64,
    /// Maximum execution time in seconds.
    pub timeout: u64,
    /// Maximum attenuation level.
    pub max_attenuation: u8,
    /// Contract state: pending, active, completed, violated.
    pub state: ContractState,
}

/// ContractState — the execution state of a TaskContract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractState {
    /// Contract created but not yet active.
    Pending,
    /// Agent is actively working on the contract.
    Active,
    /// All post-conditions satisfied — contract fulfilled.
    Completed,
    /// One or more post-conditions violated.
    Violated,
}

impl TaskContract {
    /// REQ: KAN-090
    pub fn new(
        package_name: String,
        delegator: crate::WebID,
        delegate: crate::WebID,
        task: &Task,
        ocap_gates: Vec<String>,
    ) -> Self {
        Self {
            package_name,
            delegator,
            delegate,
            task_id: task.id,
            task_title: task.title.clone(),
            pre_conditions: task.criteria.iter().map(|c| c.description.clone()).collect(),
            post_conditions: vec!["All criteria satisfied".into(), "Deliverables verified".into()],
            ocap_gates,
            gas_limit: 50000,
            timeout: 3600,
            max_attenuation: 3,
            state: ContractState::Pending,
        }
    }

    /// REQ: KAN-091 — Activate the contract. Sets state to Active.
    /// The agent now has authority to work on the task.
    pub fn activate(&mut self) {
        self.state = ContractState::Active;
    }

    /// REQ: KAN-092 — Check if the contract is complete.
    ///
    /// THIS is the method both agent and replicant call.
    /// The agent calls it to self-check: "Have I satisfied the contract?"
    /// The replicant calls it to verify: "Did the agent complete the contract?"
    ///
    /// Each pre_condition is evaluated against the evidence. If all pass,
    /// the contract state moves to Completed. If any fail, Violated.
    ///
    /// The evidence is a free-text description of what was done — the same
    /// text the agent provides as a comment when submitting deliverables.
    /// The matching is keyword-based for now; LLM-mediated evaluation (Task 6)
    /// will replace this with semantic matching against the actual deliverables.
    pub fn check_completion(
        &mut self,
        evidence: &str,
    ) -> ContractVerification {
        if self.pre_conditions.is_empty() {
            self.state = ContractState::Completed;
            return ContractVerification {
                passed: true,
                reasoning: "No pre-conditions — contract auto-completed.".into(),
                results: vec![],
            };
        }

        let evidence_lower = evidence.to_lowercase();
        let mut results = Vec::new();
        let mut all_passed = true;

        for condition in &self.pre_conditions {
            let condition_lower = condition.to_lowercase();
            let passed = condition_lower
                .split_whitespace()
                .any(|word| evidence_lower.contains(word));

            let result = ConditionResult {
                condition: condition.clone(),
                passed,
                reason: if passed {
                    "Evidence references this requirement".into()
                } else {
                    format!("No evidence found for: {}", condition)
                },
            };

            if !passed {
                all_passed = false;
            }
            results.push(result);
        }

        if all_passed {
            self.state = ContractState::Completed;
        } else {
            self.state = ContractState::Violated;
        }

        ContractVerification {
            passed: all_passed,
            reasoning: if all_passed {
                format!("All {} pre-conditions satisfied. Contract fulfilled.", self.pre_conditions.len())
            } else {
                let failed = results.iter().filter(|r| !r.passed).count();
                format!("{} of {} conditions not met. Contract violated.", failed, self.pre_conditions.len())
            },
            results,
        }
    }

    /// REQ: KAN-093 — Emit the contract as a CNS span.
    pub fn emit_span(&self, verb: &str) -> String {
        format!(
            "TaskContract[{}] '{}': delegator={} delegate={} task='{}' gates={} gas={} timeout={}s state={:?}",
            verb,
            self.package_name,
            self.delegator.redacted_display(),
            self.delegate.redacted_display(),
            self.task_title,
            self.ocap_gates.len(),
            self.gas_limit,
            self.timeout,
            self.state,
        )
    }
}

/// ContractVerification — result of checking a TaskContract's completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractVerification {
    pub passed: bool,
    pub reasoning: String,
    pub results: Vec<ConditionResult>,
}

/// ConditionResult — per-condition evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionResult {
    pub condition: String,
    pub passed: bool,
    pub reason: String,
}

// ── Spawn Specification ─────────────────────────────────────────────────

/// SpawnSpec — configuration for spawning a sub-replicant to execute a task.
///
/// Defines what capabilities (skills, memory scope, tool access) the parent
/// replicant delegates to the spawned sub-agent. Spawning is consent-mediated
/// (P1) — the parent chooses what to delegate.
///
/// Delegation levels:
/// - Minimal: read-only access to the task, no memory, restricted tools
/// - Standard: read-write task access, episodic memory, kanban tools
/// - Maximal: full replicant capabilities within the task scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnSpec {
    /// The task this spawn is for.
    pub task_id: TaskId,
    /// Delegation level: "minimal", "standard", or "maximal".
    pub delegation_level: String,
    /// Skills to delegate to the spawned replicant.
    pub delegated_skills: Vec<String>,
    /// Memory scope: "none", "episodic", or "full".
    pub memory_scope: String,
    /// Tool servers accessible to the spawned replicant.
    pub tool_servers: Vec<String>,
    /// Maximum gas/energy budget for the spawned replicant.
    pub gas_budget: Option<u64>,
    /// Maximum time the spawned replicant can run (seconds).
    pub timeout_seconds: Option<u64>,
    /// Template/skill registries accessible to the spawned replicant.
    pub registries: Vec<String>,
    /// File paths or artifact roots the agent can access.
    pub artifacts: Vec<String>,
    /// OCAP capability token specs (e.g. "tool:kanban:execute").
    /// These are validated against the parent replicant's tokens at spawn time.
    /// Each entry is a CapabilitySpec string: "resource:domain:action".
    pub capability_tokens: Vec<String>,
}

impl SpawnSpec {
    /// REQ: KAN-030
    /// pre:  task_id is valid
    /// post: returns a SpawnSpec with standard delegation defaults
    pub fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            delegation_level: "standard".into(),
            delegated_skills: vec!["kanban".into()],
            memory_scope: "episodic".into(),
            tool_servers: vec!["hkask-mcp-kanban".into()],
            gas_budget: None,
            timeout_seconds: None,
            registries: Vec::new(),
            artifacts: Vec::new(),
            capability_tokens: Vec::new(),
        }
    }

    /// REQ: KAN-031
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// REQ: KAN-032
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.delegated_skills = skills;
        self
    }

    /// REQ: KAN-033
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// REQ: KAN-034
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas_budget(mut self, budget: u64) -> Self {
        self.gas_budget = Some(budget);
        self
    }

    /// REQ: KAN-035
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// REQ: KAN-036
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// REQ: KAN-037
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// REQ: KAN-038 — Set OCAP capability token specs.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_capability_tokens(mut self, tokens: Vec<String>) -> Self {
        self.capability_tokens = tokens;
        self
    }
}



// ── Capability Package ────────────────────────────────────────────────────

/// CapabilityPackage — a named, reusable bundle of delegated capabilities.
///
/// Saved spawn configurations that can be reused across tasks and projects.
/// After a board completes, the user is prompted to save any capability
/// packages they composed, so future delegations can reference them by name.
///
/// Stored as YAML in registry/capabilities/ alongside kata manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityPackage {
    /// Unique package name (e.g., "backend-dev", "docs-writer").
    pub name: String,
    /// Human-readable description of what this package provides.
    pub description: String,
    /// Delegation level: minimal, standard, maximal.
    pub delegation_level: String,
    /// Skills delegated to the agent.
    pub skills: Vec<String>,
    /// Memory scope: none, episodic, full.
    pub memory_scope: String,
    /// MCP tool servers accessible.
    pub tool_servers: Vec<String>,
    /// Template/skill registries accessible.
    pub registries: Vec<String>,
    /// File paths or artifact roots.
    pub artifacts: Vec<String>,
    /// Default gas budget (can be overridden per task).
    pub default_gas_budget: Option<u64>,
    /// Default timeout in seconds (can be overridden per task).
    pub default_timeout_seconds: Option<u64>,
    /// OCAP capability token specs delegated to the agent.
    /// These are the actual OCAP strings validated at spawn time
    /// (e.g. "tool:kanban:execute", "registry:templates:read").
    pub capability_tokens: Vec<String>,
    /// Maximum attenuation level for delegated tokens (0-7).
    /// The spawned agent cannot further attenuate beyond this.
    pub max_attenuation: u8,
}

impl CapabilityPackage {
    /// REQ: KAN-060
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            delegation_level: "standard".into(),
            skills: Vec::new(),
            memory_scope: "episodic".into(),
            tool_servers: Vec::new(),
            registries: Vec::new(),
            artifacts: Vec::new(),
            default_gas_budget: None,
            default_timeout_seconds: None,
            capability_tokens: Vec::new(),
            max_attenuation: 3,
        }
    }

    /// REQ: KAN-061 — Convert to a SpawnSpec for a specific task.
    pub fn to_spawn_spec(&self, task_id: TaskId) -> SpawnSpec {
        SpawnSpec {
            task_id,
            delegation_level: self.delegation_level.clone(),
            delegated_skills: self.skills.clone(),
            memory_scope: self.memory_scope.clone(),
            tool_servers: self.tool_servers.clone(),
            gas_budget: self.default_gas_budget,
            timeout_seconds: self.default_timeout_seconds,
            registries: self.registries.clone(),
            artifacts: self.artifacts.clone(),
            capability_tokens: self.capability_tokens.clone(),
        }
    }

    /// REQ: KAN-062 — Builder: set delegation level.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// REQ: KAN-063
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    /// REQ: KAN-064
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// REQ: KAN-065
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tool_servers = tools;
        self
    }

    /// REQ: KAN-066
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// REQ: KAN-067
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// REQ: KAN-068
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas(mut self, budget: u64) -> Self {
        self.default_gas_budget = Some(budget);
        self
    }

    /// REQ: KAN-069
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.default_timeout_seconds = Some(seconds);
        self
    }

    /// REQ: KAN-070
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_capability_tokens(mut self, tokens: Vec<String>) -> Self {
        self.capability_tokens = tokens;
        self
    }

    /// REQ: KAN-071 — Set max attenuation (0-7, clamped to SYSTEM_MAX).
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_max_attenuation(mut self, level: u8) -> Self {
        self.max_attenuation = level.min(7);
        self
    }

    /// REQ: KAN-072 — Derive capability token specs from tool servers.
    /// Converts "hkask-mcp-kanban" → "tool:kanban:execute".
    pub fn derive_tokens_from_tools(&mut self) {
        for server in &self.tool_servers.clone() {
            if let Some(cap) = crate::capability::capability_from_server_id(server) {
                if !self.capability_tokens.contains(&cap) {
                    self.capability_tokens.push(cap);
                }
            }
        }
    }

    /// REQ: KAN-073 — Serialize to YAML for saving as a reusable package.
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| e.to_string())
    }

    /// REQ: KAN-074 — Deserialize from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| e.to_string())
    }



    // ── rSolidity Contract Integration ─────────────────────────────────

    /// REQ: KAN-082 — Express this capability package as an rSolidity
    /// task contract. The contract binds delegator and delegate with
    /// pre-conditions (acceptance criteria), post-conditions (verification),
    /// and OCAP gates (capability tokens).
    ///
    /// Returns a structured representation suitable for rSolidity
    /// contract execution and CNS span emission.
    pub fn to_task_contract(
        &self,
        task: &Task,
        delegator: crate::WebID,
        delegate: crate::WebID,
    ) -> TaskContract {
        TaskContract {
            package_name: self.name.clone(),
            delegator,
            delegate,
            task_id: task.id,
            task_title: task.title.clone(),
            pre_conditions: task.criteria.iter().map(|c| c.description.clone()).collect(),
            post_conditions: vec![
                "All acceptance criteria satisfied".into(),
                "Deliverables submitted and verified".into(),
            ],
            ocap_gates: self.capability_tokens.clone(),
            gas_limit: self.default_gas_budget.unwrap_or(50000),
            timeout: self.default_timeout_seconds.unwrap_or(3600),
            max_attenuation: self.max_attenuation,
            state: ContractState::Pending,
        }
    }
}


// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: KAN-T-001 — TaskStatus transitions follow column ordering
    #[test]
    fn task_status_transitions() {
        // Forward transitions
        assert!(TaskStatus::Backlog.can_transition_to(TaskStatus::Ready));
        assert!(TaskStatus::Ready.can_transition_to(TaskStatus::InProgress));
        assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Review));
        assert!(TaskStatus::Review.can_transition_to(TaskStatus::Done));

        // Backward transitions (one step only)
        assert!(TaskStatus::Ready.can_transition_to(TaskStatus::Backlog));
        assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Ready));
        assert!(TaskStatus::Review.can_transition_to(TaskStatus::InProgress));

        // Done cannot transition anywhere
        assert!(!TaskStatus::Done.can_transition_to(TaskStatus::Review));
        assert!(!TaskStatus::Done.can_transition_to(TaskStatus::Backlog));

        // Skipping columns is prohibited
        assert!(!TaskStatus::Backlog.can_transition_to(TaskStatus::InProgress));
        assert!(!TaskStatus::Backlog.can_transition_to(TaskStatus::Review));
        assert!(!TaskStatus::Backlog.can_transition_to(TaskStatus::Done));
        assert!(!TaskStatus::Ready.can_transition_to(TaskStatus::Review));
        assert!(!TaskStatus::Ready.can_transition_to(TaskStatus::Done));
        assert!(!TaskStatus::InProgress.can_transition_to(TaskStatus::Done));
        assert!(!TaskStatus::InProgress.can_transition_to(TaskStatus::Backlog));
        assert!(!TaskStatus::Review.can_transition_to(TaskStatus::Backlog));
    }

    // REQ: KAN-T-002 — TaskStatus::next() returns correct successor
    #[test]
    fn task_status_next() {
        assert_eq!(TaskStatus::Backlog.next(), Some(TaskStatus::Ready));
        assert_eq!(TaskStatus::Ready.next(), Some(TaskStatus::InProgress));
        assert_eq!(TaskStatus::InProgress.next(), Some(TaskStatus::Review));
        assert_eq!(TaskStatus::Review.next(), Some(TaskStatus::Done));
        assert_eq!(TaskStatus::Done.next(), None);
    }

    // REQ: KAN-T-003 — TaskStatus round-trips through string representation
    #[test]
    fn task_status_string_roundtrip() {
        for status in &[
            TaskStatus::Backlog,
            TaskStatus::Ready,
            TaskStatus::InProgress,
            TaskStatus::Review,
            TaskStatus::Done,
        ] {
            let s = status.as_str();
            let parsed = TaskStatus::parse_str(s).unwrap();
            assert_eq!(*status, parsed);

            // Also test via FromStr
            let from_str: TaskStatus = s.parse().unwrap();
            assert_eq!(*status, from_str);
        }
    }

    // REQ: KAN-T-004 — TaskStatus parse accepts alternate forms
    #[test]
    fn task_status_parse_aliases() {
        assert_eq!(
            TaskStatus::parse_str("inprogress"),
            Some(TaskStatus::InProgress)
        );
        assert_eq!(
            TaskStatus::parse_str("in-progress"),
            Some(TaskStatus::InProgress)
        );
        assert_eq!(
            TaskStatus::parse_str("IN_PROGRESS"),
            Some(TaskStatus::InProgress)
        );
        assert_eq!(TaskStatus::parse_str("Done"), Some(TaskStatus::Done));
        assert_eq!(TaskStatus::parse_str("invalid"), None);
    }

    // REQ: KAN-T-005 — Board::column_for_status finds correct column
    #[test]
    fn board_column_for_status() {
        let columns = vec![
            ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0),
            ColumnDef::new("Ready".into(), TaskStatus::Ready, 1),
            ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2),
            ColumnDef::new("Review".into(), TaskStatus::Review, 3),
            ColumnDef::new("Done".into(), TaskStatus::Done, 4),
        ];
        let board = Board::new("Test Board".into(), WebID::new(), columns);

        assert_eq!(
            board.column_for_status(TaskStatus::Backlog).unwrap().status,
            TaskStatus::Backlog
        );
        assert_eq!(
            board.column_for_status(TaskStatus::Done).unwrap().status,
            TaskStatus::Done
        );
    }

    // REQ: KAN-T-006 — Task is created in Backlog status
    #[test]
    fn task_created_in_backlog() {
        let spec = TaskSpec::new("Test task".into());
        let task = Task::new(BoardId::new(), spec, WebID::new());
        assert_eq!(task.status, TaskStatus::Backlog);
        assert!(task.verification.is_none());
        assert!(task.assignee.is_none());
    }

    // REQ: KAN-T-007 — TaskSpec builder pattern
    #[test]
    fn task_spec_builder() {
        let spec = TaskSpec::new("Build CI".into())
            .with_description("Set up CI pipeline".into())
            .with_criteria(vec![VerificationCriterion::new("All tests pass".into())]);

        assert_eq!(spec.title, "Build CI");
        assert_eq!(spec.description, Some("Set up CI pipeline".into()));
        assert_eq!(spec.criteria.len(), 1);
    }

    // REQ: KAN-T-008 — VerificationCriterion builder
    #[test]
    fn verification_criterion_with_llm() {
        let vc = VerificationCriterion::new("Task must compile".into())
            .with_llm_prompt("Check if the code compiles without errors".into());

        assert_eq!(vc.description, "Task must compile");
        assert!(vc.llm_prompt.is_some());
    }

    // REQ: KAN-T-009 — TaskFilter construction
    #[test]
    fn task_filter_by_status() {
        let filter = TaskFilter::by_status(TaskStatus::InProgress);
        assert_eq!(filter.status, Some(TaskStatus::InProgress));
        assert!(filter.assignee.is_none());
    }

    // REQ: KAN-T-010 — ConsentProof construction
    #[test]
    fn consent_proof_creation() {
        let agent = WebID::new();
        let task_id = TaskId::new();
        let proof = ConsentProof::new(agent, task_id);
        assert_eq!(proof.agent, agent);
        assert_eq!(proof.task_id, task_id);
    }
}
