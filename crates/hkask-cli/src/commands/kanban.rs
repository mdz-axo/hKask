//! Kanban CLI — board and task management from the command line.

use hkask_services::KanbanService;
use hkask_storage::Store;
use hkask_storage::TripleStore;
use hkask_types::{ConsentProof, TaskFilter, TaskSpec, TaskStatus, WebID};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::cli::KanbanAction;

/// Run a kanban subcommand.
pub fn run_cli(action: KanbanAction, replicant_webid: WebID, _db_path: Option<&str>) {
    let service = KanbanService::new(in_memory_store());

    match action {
        KanbanAction::BoardCreate { name, columns } => {
            let cols = parse_columns(&columns.unwrap_or_default());
            match service.board_create(replicant_webid, &name, &cols) {
                Ok(board) => {
                    println!("Board created: {} ({})", board.name, board.id);
                    for c in &board.columns {
                        println!("  [{}] {}", c.position, c.name);
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::BoardView { board_id } => {
            let bid = match board_id.parse() {
                Ok(id) => id,
                Err(e) => { eprintln!("Invalid board ID: {e}"); return; }
            };
            match service.board_view(bid, None) {
                Ok(view) => println!("{}", view),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::BoardList => match service.board_list(&replicant_webid) {
            Ok(boards) => {
                if boards.is_empty() {
                    println!("No boards found.");
                } else {
                    for b in &boards {
                        println!(
                            "{}  {}  ({} columns, created {})",
                            b.id,
                            b.name,
                            b.columns.len(),
                            b.created_at.format("%Y-%m-%d %H:%M")
                        );
                    }
                }
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        KanbanAction::TaskCreate {
            board_id,
            title,
            description,
            criteria,
            assign,
        } => {
            let bid = match board_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid board ID: {e}");
                    return;
                }
            };
            let mut spec = TaskSpec::new(title);
            if let Some(d) = description {
                spec = spec.with_description(d);
            }
            if !criteria.is_empty() {
                let vcs: Vec<_> = criteria
                    .into_iter()
                    .map(hkask_types::VerificationCriterion::new)
                    .collect();
                spec = spec.with_criteria(vcs);
            }
            if let Some(a) = assign {
                match a.parse::<WebID>() {
                    Ok(w) => spec = spec.with_assignee(w),
                    Err(e) => {
                        eprintln!("Invalid assignee WebID: {e}");
                        return;
                    }
                }
            }
            match service.task_create(bid, spec, replicant_webid) {
                Ok(task) => println!("Task created: {} ({})", task.title, task.id),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::TaskList { board_id, status } => {
            let bid = match board_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid board ID: {e}");
                    return;
                }
            };
            let filter = match status {
                Some(s) => match TaskStatus::parse_str(&s) {
                    Some(st) => TaskFilter::by_status(st),
                    None => {
                        eprintln!("Invalid status: {s}");
                        return;
                    }
                },
                None => TaskFilter::all(),
            };
            match service.task_list(bid, filter) {
                Ok(tasks) => {
                    if tasks.is_empty() {
                        println!("No tasks found.");
                    } else {
                        for (i, t) in tasks.iter().enumerate() {
                            let assignee = t
                                .assignee
                                .map(|a| a.redacted_display())
                                .unwrap_or_else(|| "unassigned".to_string());
                            println!("  {}. [{}] {} — {}", i + 1, t.status, t.title, assignee);
                        }
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::TaskShow { task_id } => {
            let tid = match task_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid task ID: {e}");
                    return;
                }
            };
            match service.task_get(tid) {
                Ok(Some(task)) => {
                    println!("Task: {}", task.title);
                    println!("  ID:     {}", task.id);
                    println!("  Status: {}", task.status);
                    println!("  Owner:  {}", task.owner.redacted_display());
                    if let Some(a) = task.assignee {
                        println!("  Assignee: {}", a.redacted_display());
                    }
                    if let Some(ref desc) = task.description {
                        println!("  Description: {desc}");
                    }
                    if !task.criteria.is_empty() {
                        println!("  Criteria:");
                        for c in &task.criteria {
                            println!("    - {}", c.description);
                        }
                    }
                    if let Some(ref v) = task.verification {
                        println!(
                            "  Verification: {} — {}",
                            if v.passed { "PASSED" } else { "FAILED" },
                            v.reasoning
                        );
                    }
                }
                Ok(None) => println!("Task not found."),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::TaskMove { task_id, status } => {
            let tid = match task_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid task ID: {e}");
                    return;
                }
            };
            let target = match TaskStatus::parse_str(&status) {
                Some(s) => s,
                None => {
                    eprintln!("Invalid status: {status}");
                    return;
                }
            };
            match service.task_move(tid, target, replicant_webid) {
                Ok(task) => println!("Task {} moved to {}", task.title, task.status),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::TaskAssign { task_id, agent } => {
            let tid = match task_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid task ID: {e}");
                    return;
                }
            };
            let agent_wid = match agent.parse::<WebID>() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Invalid agent WebID: {e}");
                    return;
                }
            };
            let consent = ConsentProof::new(agent_wid, tid);
            match service.task_assign(tid, agent_wid, consent) {
                Ok(task) => {
                    println!(
                        "Task assigned to {}",
                        task.assignee
                            .map(|a| a.redacted_display())
                            .unwrap_or_default()
                    );
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        KanbanAction::TaskVerify { task_id, evidence } => {
            let tid = match task_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Invalid task ID: {e}");
                    return;
                }
            };
            match service.task_verify(tid, &evidence, replicant_webid) {
                Ok((task, verification)) => {
                    println!(
                        "Verification {}: {}",
                        if verification.passed {
                            "PASSED"
                        } else {
                            "FAILED"
                        },
                        verification.reasoning
                    );
                    println!("Task status: {}", task.status);
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }
}

fn parse_columns(cols: &str) -> Vec<hkask_types::ColumnDef> {
    if cols.is_empty() {
        return default_columns();
    }
    cols.split(',')
        .enumerate()
        .map(|(i, name)| {
            let status = match name.trim().to_lowercase().as_str() {
                "backlog" => TaskStatus::Backlog,
                "ready" => TaskStatus::Ready,
                "inprogress" | "in progress" | "in_progress" => TaskStatus::InProgress,
                "review" => TaskStatus::Review,
                "done" => TaskStatus::Done,
                _ => TaskStatus::Backlog,
            };
            hkask_types::ColumnDef::new(name.trim().to_string(), status, i as u32)
        })
        .collect()
}

fn default_columns() -> Vec<hkask_types::ColumnDef> {
    vec![
        hkask_types::ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0),
        hkask_types::ColumnDef::new("Ready".into(), TaskStatus::Ready, 1),
        hkask_types::ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2),
        hkask_types::ColumnDef::new("Review".into(), TaskStatus::Review, 3),
        hkask_types::ColumnDef::new("Done".into(), TaskStatus::Done, 4),
    ]
}

fn in_memory_store() -> TripleStore {
    let conn = Arc::new(Mutex::new(
        Connection::open_in_memory().expect("in-memory DB"),
    ));
    let store = TripleStore::new(conn);
    store
        .lock_conn()
        .expect("mutex not poisoned")
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
                owner_webid TEXT NOT NULL
            )",
        )
        .expect("DDL batch must succeed");
    store
}
