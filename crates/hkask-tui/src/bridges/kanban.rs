//! KanbanDataBridge — trait for kanban board/task data in the TUI.
//!
//! Provides the Kanban window with live board data. Implemented by
//! the CLI via kanban service.

use std::sync::Arc;

/// Summary of a task for TUI display.
#[derive(Debug, Clone)]
pub struct KanbanTaskSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Vec<String>,
}

/// Summary of a kanban board's column definitions.
#[derive(Debug, Clone)]
pub struct KanbanBoardSummary {
    pub id: String,
    pub name: String,
    pub columns: Vec<String>,
    pub task_count: usize,
}

/// Status counts for a board.
#[derive(Debug, Clone)]
pub struct KanbanStatusCounts {
    pub backlog: usize,
    pub ready: usize,
    pub in_progress: usize,
    pub review: usize,
    pub done: usize,
}

/// Trait for querying kanban subsystem state.
pub trait KanbanDataBridge: Send + Sync {
    /// List all boards the agent owns.
    fn board_list(&self) -> Vec<KanbanBoardSummary>;

    /// Tasks filtered by status string ("backlog", "in_progress", "done", etc.).
    fn tasks_by_status(&self, status: &str, limit: usize) -> Vec<KanbanTaskSummary>;

    /// Count of tasks per status in the first board.
    fn status_counts(&self) -> KanbanStatusCounts;

    /// All tasks across all statuses (for board overview).
    fn all_tasks(&self, limit: usize) -> Vec<KanbanTaskSummary>;
}

/// Mock implementation for TUI development and testing.
pub struct MockKanbanBridge {
    pub boards: Vec<KanbanBoardSummary>,
    pub backlog_tasks: Vec<KanbanTaskSummary>,
    pub in_progress_tasks: Vec<KanbanTaskSummary>,
    pub done_tasks: Vec<KanbanTaskSummary>,
    pub ready_tasks: Vec<KanbanTaskSummary>,
    pub review_tasks: Vec<KanbanTaskSummary>,
}

impl MockKanbanBridge {
    pub fn new() -> Self {
        Self {
            boards: vec![KanbanBoardSummary {
                id: "board-1".into(),
                name: "Development".into(),
                columns: vec![
                    "Backlog".into(),
                    "Ready".into(),
                    "In Progress".into(),
                    "Review".into(),
                    "Done".into(),
                ],
                task_count: 0,
            }],
            backlog_tasks: Vec::new(),
            in_progress_tasks: Vec::new(),
            done_tasks: Vec::new(),
            ready_tasks: Vec::new(),
            review_tasks: Vec::new(),
        }
    }

    pub fn with_sample_data() -> Self {
        Self {
            boards: vec![KanbanBoardSummary {
                id: "board-1".into(),
                name: "Development".into(),
                columns: vec![
                    "Backlog".into(),
                    "Ready".into(),
                    "In Progress".into(),
                    "Review".into(),
                    "Done".into(),
                ],
                task_count: 7,
            }],
            backlog_tasks: vec![
                KanbanTaskSummary {
                    id: "t1".into(),
                    title: "Implement wallet TUI bridge".into(),
                    status: "backlog".into(),
                    assignee: None,
                    priority: Some("medium".into()),
                    labels: vec!["tui".into(), "wallet".into()],
                },
                KanbanTaskSummary {
                    id: "t2".into(),
                    title: "Add PTY support to terminal".into(),
                    status: "backlog".into(),
                    assignee: None,
                    priority: Some("low".into()),
                    labels: vec!["tui".into()],
                },
            ],
            in_progress_tasks: vec![
                KanbanTaskSummary {
                    id: "t3".into(),
                    title: "Wire editor file save".into(),
                    status: "in_progress".into(),
                    assignee: Some("agent-1".into()),
                    priority: Some("high".into()),
                    labels: vec!["tui".into(), "editor".into()],
                },
                KanbanTaskSummary {
                    id: "t4".into(),
                    title: "Registry live data bridge".into(),
                    status: "in_progress".into(),
                    assignee: Some("agent-1".into()),
                    priority: Some("high".into()),
                    labels: vec!["tui".into(), "registry".into()],
                },
            ],
            done_tasks: vec![
                KanbanTaskSummary {
                    id: "t5".into(),
                    title: "Fix Ctrl+N routing".into(),
                    status: "done".into(),
                    assignee: Some("agent-1".into()),
                    priority: Some("high".into()),
                    labels: vec!["tui".into(), "bug".into()],
                },
                KanbanTaskSummary {
                    id: "t6".into(),
                    title: "Add integration tests".into(),
                    status: "done".into(),
                    assignee: Some("agent-1".into()),
                    priority: Some("high".into()),
                    labels: vec!["tui".into(), "testing".into()],
                },
            ],
            ready_tasks: vec![KanbanTaskSummary {
                id: "t7".into(),
                title: "Memory live data bridge".into(),
                status: "ready".into(),
                assignee: None,
                priority: Some("medium".into()),
                labels: vec!["tui".into(), "memory".into()],
            }],
            review_tasks: Vec::new(),
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl KanbanDataBridge for MockKanbanBridge {
    fn board_list(&self) -> Vec<KanbanBoardSummary> {
        self.boards.clone()
    }

    fn tasks_by_status(&self, status: &str, _limit: usize) -> Vec<KanbanTaskSummary> {
        match status {
            "backlog" => self.backlog_tasks.clone(),
            "ready" => self.ready_tasks.clone(),
            "in_progress" => self.in_progress_tasks.clone(),
            "review" => self.review_tasks.clone(),
            "done" => self.done_tasks.clone(),
            _ => Vec::new(),
        }
    }

    fn status_counts(&self) -> KanbanStatusCounts {
        KanbanStatusCounts {
            backlog: self.backlog_tasks.len(),
            ready: self.ready_tasks.len(),
            in_progress: self.in_progress_tasks.len(),
            review: self.review_tasks.len(),
            done: self.done_tasks.len(),
        }
    }

    fn all_tasks(&self, _limit: usize) -> Vec<KanbanTaskSummary> {
        let mut tasks = Vec::new();
        tasks.extend(self.backlog_tasks.clone());
        tasks.extend(self.ready_tasks.clone());
        tasks.extend(self.in_progress_tasks.clone());
        tasks.extend(self.review_tasks.clone());
        tasks.extend(self.done_tasks.clone());
        tasks
    }
}
