//! Kanban types — Agent coordination via headless task boards.
//!
//! Every type carries `owner: WebID` (P12 — anonymous agency prohibition).
//! Task status transitions are column-ordered: Backlog → Ready → InProgress → Review → Done.
//! Verification criteria accept natural-language acceptance specs with optional LLM evaluation prompts.

use chrono::{DateTime, Utc};
use hkask_capability::capability_from_server_id;
use hkask_types::id::{BoardId, ColumnId, CommentId, PhaseId, TaskId, WebID};
use serde::{Deserialize, Serialize};

// ── Priority ────────────────────────────────────────────────────────────────

/// Priority level for kanban tasks.
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
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
#[non_exhaustive]
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCriterion {
    /// Human-readable acceptance spec.
    pub description: String,
    /// Optional prompt for LLM-mediated evaluation.
    pub llm_prompt: Option<String>,
}

impl VerificationCriterion {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  description is non-empty
    /// post: returns a VerificationCriterion with no LLM prompt
    pub fn new(description: String) -> Self {
        Self {
            description,
            llm_prompt: None,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
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
#[non_exhaustive]
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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

// ── Kanban phase ───────────────────────────────────────────────────────────

/// Kanban phase — a grouping category for tasks within a board.
///
/// Phases group work for reassembly: when all tasks in a phase complete,
/// their deliverables can be composed into a coherent output.
/// Unlike columns (which track workflow state), phases track project structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanbanPhase {
    pub id: PhaseId,
    pub name: String,
    pub description: Option<String>,
    /// Display order (0-based).
    pub order: u32,
    /// When the phase was created.
    pub created_at: DateTime<Utc>,
}

impl KanbanPhase {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post: returns new instance with defaults
    /// pre:  name is non-empty, order is a valid u32
    /// post: returns KanbanPhase with generated PhaseId and created_at set to now
    pub fn new(name: String, order: u32) -> Self {
        Self {
            id: PhaseId::new(),
            name,
            description: None,
            order,
            created_at: Utc::now(),
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  desc is a non-empty description string
    /// post: returns Self with description set to Some(desc)
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
#[non_exhaustive]
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
    pub phases: Vec<KanbanPhase>,
    /// When the board was created.
    pub created_at: DateTime<Utc>,
}

impl Board {
    /// expect: "System types preserve semantic identity and are provenance-aware"
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self.columns is sorted by position
    /// post: returns the first column (by position) — typically Backlog
    pub fn first_column(&self) -> Option<&ColumnDef> {
        self.columns.first()
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self.columns is sorted by position
    /// post: returns the last column (by position) — typically Done
    pub fn last_column(&self) -> Option<&ColumnDef> {
        self.columns.last()
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  status is a valid TaskStatus
    /// post: returns the ColumnDef matching the given status, if present
    pub fn column_for_status(&self, status: TaskStatus) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.status == status)
    }
}

// ── Task ───────────────────────────────────────────────────────────────────

/// TaskSpec — input specification for creating a new task.
#[non_exhaustive]
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
    /// Software-compute gas budget for this task (template exec, tool dispatch).
    pub gas_budget: Option<u64>,
    /// Inference/API rJoule budget (250k rJoules ≈ $1 inference spend).
    pub rjoule_budget: Option<u64>,
}

impl TaskSpec {
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
            gas_budget: None,
            rjoule_budget: None,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns self with description set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns self with criteria set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_criteria(mut self, criteria: Vec<VerificationCriterion>) -> Self {
        self.criteria = criteria;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid story points
    /// post: returns Self with story points set
    /// pre:  self is valid; assignee is a valid WebID
    /// post: returns self with assignee set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_assignee(mut self, assignee: WebID) -> Self {
        self.assignee = Some(assignee);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid estimated hours
    /// post: returns Self with estimated hours set
    /// pre:  points is a valid u32
    /// post: returns self with story_points set to Some(points)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_story_points(mut self, points: u32) -> Self {
        self.story_points = Some(points);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid labels
    /// post: returns Self with labels set
    /// pre:  hours is a non-negative f64
    /// post: returns self with estimated_hours set to Some(hours)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_estimated_hours(mut self, hours: f64) -> Self {
        self.estimated_hours = Some(hours);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  priority is a valid Priority variant
    /// post: returns self with priority set to Some(priority)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  labels is a vector of label strings
    /// post: returns self with labels set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  phase_id is a valid PhaseId
    /// post: returns self with phase_id set to Some(phase_id)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_phase(mut self, phase_id: PhaseId) -> Self {
        self.phase_id = Some(phase_id);
        self
    }

    /// Set the gas/rJoule budget for the subagent working on this task.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas_budget(mut self, gas: u64) -> Self {
        self.gas_budget = Some(gas);
        self
    }

    /// Set the inference/API rJoule budget for the subagent.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_rjoule_budget(mut self, rjoules: u64) -> Self {
        self.rjoule_budget = Some(rjoules);
        self
    }
}

/// GasEntry — a record of gas consumed or added on a task.
///
/// Each entry tracks what operation consumed or granted gas, how much,
/// and when. This is the audit trail for subagent resource usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasEntry {
    /// Amount consumed (positive) or added (also positive, context is in `kind`).
    pub amount: u64,
    /// "spend" (consumed) or "refill" (added by delegator).
    pub kind: String,
    /// What consumed the gas: "inference: deepseek-v4", "template: bug-hunt",
    /// "tool: kanban_task_list", etc.
    pub reason: String,
    /// When this entry was recorded.
    pub at: DateTime<Utc>,
}

impl GasEntry {
    pub fn gas_spend(amount: u64, reason: String) -> Self {
        Self {
            amount,
            kind: "gas_spend".into(),
            reason,
            at: Utc::now(),
        }
    }
    pub fn rjoule_spend(amount: u64, reason: String) -> Self {
        Self {
            amount,
            kind: "rjoule_spend".into(),
            reason,
            at: Utc::now(),
        }
    }
    pub fn gas_refill(amount: u64) -> Self {
        Self {
            amount,
            kind: "gas_refill".into(),
            reason: "delegator added gas".into(),
            at: Utc::now(),
        }
    }
    pub fn rjoule_refill(amount: u64) -> Self {
        Self {
            amount,
            kind: "rjoule_refill".into(),
            reason: "delegator added rJoules".into(),
            at: Utc::now(),
        }
    }
}

/// Task — a single work item on a kanban board.
///
/// Every task carries `owner: WebID` (P12) — the creator of the task.
/// Assignment is separate and requires agent consent (P1).
#[non_exhaustive]
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
    /// Gas/rJoules remaining in the subagent's budget for this task.
    /// Initialized from `TaskSpec.gas_budget`. When this hits 0, the task
    /// auto-completes via the gas exhaustion completion path.
    pub gas_remaining: Option<u64>,
    /// rJoules remaining for inference/API calls (250k ≈ $1 spend).
    pub rjoule_remaining: Option<u64>,
    /// Audit trail of gas and rJoule consumption/refills.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gas_spend: Vec<GasEntry>,
}

impl Task {
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
            gas_remaining: spec.gas_budget,
            rjoule_remaining: spec.rjoule_budget,
            gas_spend: Vec::new(),
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post: returns new instance with defaults
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post:      /// post: returns new instance with defaults
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
#[non_exhaustive]
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns an empty filter (matches all tasks)
    pub fn all() -> Self {
        Self {
            status: None,
            assignee: None,
            priority: None,
            limit: None,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post: returns expected result
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  priority is a valid Priority
    /// post:      /// post: returns tasks sorted by priority
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
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
    pub delegator: WebID,
    /// The agent receiving the delegation.
    pub delegate: WebID,
    /// The task this contract governs.
    pub task_id: TaskId,
    /// Task title for display.
    pub task_title: String,
    /// Pre-conditions (acceptance criteria) — informational expectations.
    pub pre_conditions: Vec<String>,
    /// Post-conditions — informational expectations.
    pub post_conditions: Vec<String>,
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post:      /// post: returns new instance with defaults
    pub fn new(package_name: String, delegator: WebID, delegate: WebID, task: &Task) -> Self {
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
            gas_limit: 50000,
            timeout: 3600,
            max_attenuation: 3,
            state: ContractState::Pending,
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  state allows activation
    /// post:      /// post: state transitioned to active
    /// The agent now has authority to work on the task.
    pub fn activate(&mut self) {
        self.state = ContractState::Active;
    }

    /// Task completion is user-feedback-driven. The agent submits evidence
    /// (a description of what was done) and the user confirms. Criteria are
    /// informational expectations — they guide the work but don't gate completion.
    /// Completion produces: task output (deliverables) + CNS spans + user feedback
    /// → learning signal for the system.
    pub fn check_completion(&mut self, evidence: &str) -> ContractVerification {
        // Evidence IS the completion signal. Non-empty evidence = user confirmed.
        if evidence.trim().is_empty() {
            self.state = ContractState::Violated;
            return ContractVerification {
                passed: false,
                reasoning: "No evidence provided — task not verified.".into(),
                results: vec![],
            };
        }

        self.state = ContractState::Completed;
        let criteria_list: Vec<String> = self
            .pre_conditions
            .iter()
            .map(|c| format!("  - {c}"))
            .collect();
        let criteria_block = if criteria_list.is_empty() {
            String::new()
        } else {
            format!("\nCriteria:\n{}", criteria_list.join("\n"))
        };

        ContractVerification {
            passed: true,
            reasoning: format!(
                "User feedback received.{} Evidence length: {} chars.",
                criteria_block,
                evidence.len()
            ),
            results: vec![],
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  span data is valid
    /// post:      /// post: CNS span emitted to event sink
    pub fn emit_span(&self, verb: &str) -> String {
        format!(
            "TaskContract[{}] '{}': delegator={} delegate={} task='{}' pre_conds={} gas={} timeout={}s state={:?}",
            verb,
            self.package_name,
            self.delegator.redacted_display(),
            self.delegate.redacted_display(),
            self.task_title,
            self.pre_conditions.len(),
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
#[non_exhaustive]
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid skills
    /// post: returns Self with skills set
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid timeout
    /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for skills
    /// post:      /// post: returns Self with skills set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.delegated_skills = skills;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for memory
    /// post:      /// post: returns Self with memory set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for gas budget
    /// post:      /// post: returns Self with gas budget set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas_budget(mut self, budget: u64) -> Self {
        self.gas_budget = Some(budget);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for timeout
    /// post:      /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for registries
    /// post:      /// post: returns Self with registries set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for artifacts
    /// post:      /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for capability tokens
    /// post:      /// post: returns Self with capability tokens set
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
/// packages they composed, so delegations can reference them by name.
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  arguments are valid
    /// post:      /// post: returns new instance with defaults
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid memory
    /// post: returns Self with memory set
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid artifacts
    /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_level(mut self, level: &str) -> Self {
        self.delegation_level = level.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid gas
    /// post: returns Self with gas set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is a valid timeout
    /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_memory(mut self, scope: &str) -> Self {
        self.memory_scope = scope.into();
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for tools
    /// post:      /// post: returns Self with tools set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tool_servers = tools;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for registries
    /// post:      /// post: returns Self with registries set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_registries(mut self, registries: Vec<String>) -> Self {
        self.registries = registries;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for artifacts
    /// post:      /// post: returns Self with artifacts set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns converted value
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_gas(mut self, budget: u64) -> Self {
        self.default_gas_budget = Some(budget);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for timeout
    /// post:      /// post: returns Self with timeout set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.default_timeout_seconds = Some(seconds);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post: returns converted value
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_capability_tokens(mut self, tokens: Vec<String>) -> Self {
        self.capability_tokens = tokens;
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  value is valid for max attenuation
    /// post:      /// post: returns Self with max attenuation set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_max_attenuation(mut self, level: u8) -> Self {
        self.max_attenuation = level.min(7);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  tools list is non-empty
    /// post: populates capability_tokens from tool_servers
    /// Converts "hkask-mcp-kanban" → "tool:kanban:execute".
    pub fn derive_tokens_from_tools(&mut self) {
        self.capability_tokens = self
            .tool_servers
            .iter()
            .filter_map(|server_id| capability_from_server_id(server_id))
            .collect();
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post:      /// post: returns converted representation
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml_neo::to_string(self).map_err(|e| e.to_string())
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  input is valid
    /// post:      /// post: returns parsed object
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml_neo::from_str(yaml).map_err(|e| e.to_string())
    }

    // ── rSolidity Contract Integration ─────────────────────────────────

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is valid
    /// post:      /// post: returns converted representation
    /// task contract. The contract binds delegator and delegate with
    /// pre-conditions (acceptance criteria), post-conditions (verification),
    /// and OCAP gates (capability tokens).
    ///
    /// Returns a structured representation suitable for rSolidity
    /// contract execution and CNS span emission.
    pub fn to_task_contract(&self, task: &Task, delegator: WebID, delegate: WebID) -> TaskContract {
        TaskContract {
            package_name: self.name.clone(),
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
                "All acceptance criteria satisfied".into(),
                "Deliverables submitted and verified".into(),
            ],
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

    #[test]
    fn task_status_next() {
        assert_eq!(TaskStatus::Backlog.next(), Some(TaskStatus::Ready));
        assert_eq!(TaskStatus::Ready.next(), Some(TaskStatus::InProgress));
        assert_eq!(TaskStatus::InProgress.next(), Some(TaskStatus::Review));
        assert_eq!(TaskStatus::Review.next(), Some(TaskStatus::Done));
        assert_eq!(TaskStatus::Done.next(), None);
    }

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

    #[test]
    fn task_created_in_backlog() {
        let spec = TaskSpec::new("Test task".into());
        let task = Task::new(BoardId::new(), spec, WebID::new());
        assert_eq!(task.status, TaskStatus::Backlog);
        assert!(task.verification.is_none());
        assert!(task.assignee.is_none());
    }

    #[test]
    fn task_spec_builder() {
        let spec = TaskSpec::new("Build CI".into())
            .with_description("Set up CI pipeline".into())
            .with_criteria(vec![VerificationCriterion::new("All tests pass".into())]);

        assert_eq!(spec.title, "Build CI");
        assert_eq!(spec.description, Some("Set up CI pipeline".into()));
        assert_eq!(spec.criteria.len(), 1);
    }

    #[test]
    fn verification_criterion_with_llm() {
        let vc = VerificationCriterion::new("Task must compile".into())
            .with_llm_prompt("Check if the code compiles without errors".into());

        assert_eq!(vc.description, "Task must compile");
        assert!(vc.llm_prompt.is_some());
    }

    #[test]
    fn task_filter_by_status() {
        let filter = TaskFilter::by_status(TaskStatus::InProgress);
        assert_eq!(filter.status, Some(TaskStatus::InProgress));
        assert!(filter.assignee.is_none());
    }

    #[test]
    fn consent_proof_creation() {
        let agent = WebID::new();
        let task_id = TaskId::new();
        let proof = ConsentProof::new(agent, task_id);
        assert_eq!(proof.agent, agent);
        assert_eq!(proof.task_id, task_id);
    }
}
