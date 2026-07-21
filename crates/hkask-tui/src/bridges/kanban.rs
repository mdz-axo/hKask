//! KanbanDataBridge — trait for kanban board/task data in the TUI.
//!
//! Provides the Kanban window with live board data. Implemented by
//! the CLI via kanban service.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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

/// Trait for querying and mutating kanban subsystem state.
pub trait KanbanDataBridge: Send + Sync {
    /// List all boards the agent owns.
    fn board_list(&self) -> Vec<KanbanBoardSummary>;

    /// Tasks filtered by status string ("backlog", "in_progress", "done", etc.).
    fn tasks_by_status(&self, status: &str, limit: usize) -> Vec<KanbanTaskSummary>;

    /// Count of tasks per status in the first board.
    fn status_counts(&self) -> KanbanStatusCounts;

    /// All tasks across all statuses (for board overview).
    fn all_tasks(&self, limit: usize) -> Vec<KanbanTaskSummary>;

    /// Move a task to a new status column.
    /// Returns Err(message) if the transition is invalid or the task is not found.
    fn move_task(&self, task_id: &str, to_status: &str) -> anyhow::Result<KanbanTaskSummary>;
}

/// Mock implementation for TUI development and testing.
///
/// Uses interior mutability (Mutex) so `&self` methods can mutate state,
/// matching the trait's object-safe signature.
pub struct MockKanbanBridge {
    pub boards: Vec<KanbanBoardSummary>,
    backlog_tasks: Mutex<Vec<KanbanTaskSummary>>,
    ready_tasks: Mutex<Vec<KanbanTaskSummary>>,
    in_progress_tasks: Mutex<Vec<KanbanTaskSummary>>,
    review_tasks: Mutex<Vec<KanbanTaskSummary>>,
    done_tasks: Mutex<Vec<KanbanTaskSummary>>,
    query_count: AtomicUsize,
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
            backlog_tasks: Mutex::new(Vec::new()),
            in_progress_tasks: Mutex::new(Vec::new()),
            done_tasks: Mutex::new(Vec::new()),
            ready_tasks: Mutex::new(Vec::new()),
            review_tasks: Mutex::new(Vec::new()),
            query_count: AtomicUsize::new(0),
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
            backlog_tasks: Mutex::new(vec![
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
            ]),
            ready_tasks: Mutex::new(vec![KanbanTaskSummary {
                id: "t7".into(),
                title: "Memory live data bridge".into(),
                status: "ready".into(),
                assignee: None,
                priority: Some("medium".into()),
                labels: vec!["tui".into(), "memory".into()],
            }]),
            in_progress_tasks: Mutex::new(vec![
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
            ]),
            review_tasks: Mutex::new(Vec::new()),
            done_tasks: Mutex::new(vec![
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
            ]),
            query_count: AtomicUsize::new(0),
        }
    }

    pub fn query_count(&self) -> usize {
        self.query_count.load(Ordering::Relaxed)
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Map status string to the correct Mutex-guarded vec.
    fn task_vec(&self, status: &str) -> &Mutex<Vec<KanbanTaskSummary>> {
        match status {
            "backlog" => &self.backlog_tasks,
            "ready" => &self.ready_tasks,
            "in_progress" => &self.in_progress_tasks,
            "review" => &self.review_tasks,
            "done" => &self.done_tasks,
            _ => &self.backlog_tasks, // fallback, won't find anything
        }
    }
}

impl KanbanDataBridge for MockKanbanBridge {
    fn board_list(&self) -> Vec<KanbanBoardSummary> {
        self.boards.clone()
    }

    fn tasks_by_status(&self, status: &str, _limit: usize) -> Vec<KanbanTaskSummary> {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.task_vec(status)
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn status_counts(&self) -> KanbanStatusCounts {
        KanbanStatusCounts {
            backlog: self
                .backlog_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            ready: self
                .ready_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            in_progress: self
                .in_progress_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            review: self
                .review_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            done: self
                .done_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        }
    }

    fn all_tasks(&self, _limit: usize) -> Vec<KanbanTaskSummary> {
        let mut tasks = Vec::new();
        tasks.extend(
            self.backlog_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        tasks.extend(
            self.ready_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        tasks.extend(
            self.in_progress_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        tasks.extend(
            self.review_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        tasks.extend(
            self.done_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        tasks
    }

    fn move_task(&self, task_id: &str, to_status: &str) -> anyhow::Result<KanbanTaskSummary> {
        // Find and remove from all columns
        let statuses = ["backlog", "ready", "in_progress", "review", "done"];
        for status in &statuses {
            let mut tasks = self
                .task_vec(status)
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
                let mut task = tasks.remove(pos);
                let from = task.status.clone();

                // Validate transition — simple forward/backward one step
                let valid = matches!(
                    (from.as_str(), to_status),
                    ("backlog", "ready")
                        | ("ready", "backlog")
                        | ("ready", "in_progress")
                        | ("in_progress", "ready")
                        | ("in_progress", "review")
                        | ("review", "in_progress")
                        | ("review", "done"),
                );

                if !valid {
                    // Put it back
                    tasks.push(task);
                    return Err(anyhow::anyhow!("invalid transition: {from} → {to_status}"));
                }

                task.status = to_status.to_string();
                let summary = task.clone();

                // Insert into target column
                drop(tasks);
                self.task_vec(to_status)
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(task);

                return Ok(summary);
            }
        }
        Err(anyhow::anyhow!("task not found: {task_id}"))
    }
}
