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
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Backlog => "backlog",
            TaskStatus::Ready => "ready",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Review => "review",
            TaskStatus::Done => "done",
        }
    }

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
    pub fn new(description: String) -> Self {
        Self {
            description,
            llm_prompt: None,
        }
    }

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
    pub fn new(name: String, order: u32) -> Self {
        Self {
            id: PhaseId::new(),
            name,
            description: None,
            order,
            created_at: Utc::now(),
        }
    }

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

    pub fn first_column(&self) -> Option<&ColumnDef> {
        self.columns.first()
    }

    pub fn last_column(&self) -> Option<&ColumnDef> {
        self.columns.last()
    }

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

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_criteria(mut self, criteria: Vec<VerificationCriterion>) -> Self {
        self.criteria = criteria;
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_assignee(mut self, assignee: WebID) -> Self {
        self.assignee = Some(assignee);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_story_points(mut self, points: u32) -> Self {
        self.story_points = Some(points);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_estimated_hours(mut self, hours: f64) -> Self {
        self.estimated_hours = Some(hours);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

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
    pub fn new(task_id: TaskId, author: WebID, body: String) -> Self {
        Self {
            id: CommentId::new(),
            task_id,
            author,
            body,
            created_at: Utc::now(),
        }
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
    pub fn all() -> Self {
        Self {
            status: None,
            assignee: None,
            priority: None,
            limit: None,
        }
    }

    pub fn by_status(status: TaskStatus) -> Self {
        Self {
            status: Some(status),
            assignee: None,
            priority: None,
            limit: None,
        }
    }

    pub fn by_assignee(assignee: WebID) -> Self {
        Self {
            status: None,
            assignee: Some(assignee),
            priority: None,
            limit: None,
        }
    }

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
            pre_conditions: task
                .criteria
                .iter()
                .map(|c| c.description.clone())
                .collect(),
            post_conditions: vec![
                "All criteria satisfied".into(),
                "Deliverables verified".into(),
            ],
            ocap_gates,
            gas_limit: 50000,
            timeout: 3600,
            max_attenuation: 3,
            state: ContractState::Pending,
        }
    }

    /// The agent now has authority to work on the task.
