//! Kanban types — Agent coordination via headless task boards.
//!
//! Every type carries `owner: WebID` (P12 — anonymous agency prohibition).
//! Task status transitions are column-ordered: Backlog → Ready → InProgress → Review → Done.
//! Verification criteria accept natural-language acceptance specs with optional LLM evaluation prompts.

use crate::id::{BoardId, ColumnId, TaskId, WebID};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

impl ColumnDef {
    /// REQ: KAN-005
    /// pre:  name is non-empty; status is valid; position is >= 0
    /// post: returns a new ColumnDef with a random ColumnId
    pub fn new(name: String, status: TaskStatus, position: u32) -> Self {
        Self {
            id: ColumnId::new(),
            name,
            status,
            position,
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Short title for the task.
    pub title: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Acceptance criteria — what "done" means.
    pub criteria: Vec<VerificationCriterion>,
    /// Optional agent assignment (requires consent).
    pub assignee: Option<WebID>,
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
}

/// Task — a single work item on a kanban board.
///
/// Every task carries `owner: WebID` (P12) — the creator of the task.
/// Assignment is separate and requires agent consent (P1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// ── Filter ─────────────────────────────────────────────────────────────────

/// TaskFilter — criteria for listing/filtering tasks on a board.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by status (column).
    pub status: Option<TaskStatus>,
    /// Filter by assignee.
    pub assignee: Option<WebID>,
    /// Limit the number of results.
    pub limit: Option<usize>,
}

impl TaskFilter {
    /// REQ: KAN-019
    /// post: returns an empty filter (matches all tasks)
    pub fn all() -> Self {
        Self::default()
    }

    /// REQ: KAN-020
    /// pre:  status is a valid TaskStatus
    /// post: returns a filter matching only tasks with the given status
    pub fn by_status(status: TaskStatus) -> Self {
        Self {
            status: Some(status),
            assignee: None,
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
