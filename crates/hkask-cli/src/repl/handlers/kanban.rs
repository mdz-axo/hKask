//! REPL handler for `/kanban` slash commands.

use crate::repl::ReplState;
use hkask_services::KanbanService;
use hkask_storage::Store;
use hkask_storage::TripleStore;
use hkask_types::{ConsentProof, TaskFilter, TaskSpec};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Handle `/kanban` REPL commands.
pub(crate) fn handle_kanban(
    subcommand: &str,
    rest: &str,
    state: &mut ReplState,
    _rt: &tokio::runtime::Handle,
) {
    let service = kanban_service(state);
    let webid = state.agent_webid;

    match subcommand {
        "" | "help" => {
            println!("  \x1b[1mKanban Commands\x1b[0m");
            println!("    \x1b[36m/kanban board create <name>\x1b[0m     Create a board");
            println!("    \x1b[36m/kanban board list\x1b[0m              List boards");
            println!("    \x1b[36m/kanban task create <board> <title>\x1b[0m  Create a task");
            println!("    \x1b[36m/kanban task list <board> [status]\x1b[0m List tasks");
            println!("    \x1b[36m/kanban task show <id>\x1b[0m          Show task details");
            println!(
                "    \x1b[36m/kanban move <task> <status>\x1b[0m     Move task between columns"
            );
            println!("    \x1b[36m/kanban accept <task>\x1b[0m            Accept task assignment");
            println!(
                "    \x1b[36m/kanban submit <task> <evidence>\x1b[0m  Submit for verification"
            );
            println!("    \x1b[36m/kanban decompose <project>\x1b[0m     LLM decompose into tasks");
            println!(
                "    \x1b[36m/kanban spawn <task>\x1b[0m            Spawn replicant to execute"
            );
            println!();
        }

        "board" => {
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            match parts.first().copied().unwrap_or("") {
                "create" => {
                    let name = parts.get(1).copied().unwrap_or("Unnamed Board");
                    match service.board_create(webid, name, &default_columns()) {
                        Ok(board) => {
                            println!("  Board created: {} ({})", board.name, board.id);
                            println!("  Columns: Backlog → Ready → InProgress → Review → Done");
                        }
                        Err(e) => println!("  Error: {e}"),
                    }
                }
                "list" => match service.board_list(&webid) {
                    Ok(boards) => {
                        if boards.is_empty() {
                            println!("  No boards found.");
                        } else {
                            for b in &boards {
                                println!("  {}  {}", b.id, b.name);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {e}"),
                },
                _ => println!("  Usage: /kanban board create|list"),
            }
        }

        "task" => {
            let parts: Vec<&str> = rest.splitn(4, ' ').collect();
            match parts.first().copied().unwrap_or("") {
                "create" => {
                    let board = parts.get(1).copied().unwrap_or("");
                    let title = parts.get(2).copied().unwrap_or("");
                    if board.is_empty() || title.is_empty() {
                        println!("  Usage: /kanban task create <board-id> <title>");
                        return;
                    }
                    let bid = match board.parse() {
                        Ok(id) => id,
                        Err(_) => {
                            println!("  Invalid board ID");
                            return;
                        }
                    };
                    match service.task_create(bid, TaskSpec::new(title.into()), webid) {
                        Ok(task) => println!("  Task created: {} ({})", task.title, task.id),
                        Err(e) => println!("  Error: {e}"),
                    }
                }
                "list" => {
                    let board = parts.get(1).copied().unwrap_or("");
                    let status = parts.get(2).copied();
                    if board.is_empty() {
                        println!("  Usage: /kanban task list <board-id> [status]");
                        return;
                    }
                    let bid = match board.parse() {
                        Ok(id) => id,
                        Err(_) => {
                            println!("  Invalid board ID");
                            return;
                        }
                    };
                    let filter = match status.and_then(|s| hkask_types::TaskStatus::parse_str(s)) {
                        Some(st) => TaskFilter::by_status(st),
                        None => TaskFilter::all(),
                    };
                    match service.task_list(bid, filter) {
                        Ok(tasks) => {
                            if tasks.is_empty() {
                                println!("  No tasks found.");
                            } else {
                                for (i, t) in tasks.iter().enumerate() {
                                    let a = t
                                        .assignee
                                        .map(|a| a.redacted_display())
                                        .unwrap_or_else(|| "unassigned".into());
                                    println!("  {}. [{}] {} — {}", i + 1, t.status, t.title, a);
                                }
                            }
                        }
                        Err(e) => println!("  Error: {e}"),
                    }
                }
                "show" => {
                    let tid_str = parts.get(1).copied().unwrap_or("");
                    if tid_str.is_empty() {
                        println!("  Usage: /kanban task show <task-id>");
                        return;
                    }
                    let tid = match tid_str.parse() {
                        Ok(id) => id,
                        Err(_) => {
                            println!("  Invalid task ID");
                            return;
                        }
                    };
                    match service.task_get(tid) {
                        Ok(Some(task)) => {
                            println!("  Task: {}", task.title);
                            println!("    ID:     {}", task.id);
                            println!("    Status: {}", task.status);
                            if let Some(ref d) = task.description {
                                println!("    Desc:   {d}");
                            }
                        }
                        Ok(None) => println!("  Task not found."),
                        Err(e) => println!("  Error: {e}"),
                    }
                }
                _ => println!("  Usage: /kanban task create|list|show"),
            }
        }

        "move" => {
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            let task_str = parts.first().copied().unwrap_or("");
            let target_str = parts.get(1).copied().unwrap_or("");
            if task_str.is_empty() || target_str.is_empty() {
                println!("  Usage: /kanban move <task-id> <status>");
                return;
            }
            let tid = match task_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("  Invalid task ID");
                    return;
                }
            };
            let target = match hkask_types::TaskStatus::parse_str(target_str) {
                Some(s) => s,
                None => {
                    println!("  Invalid status: {target_str}");
                    return;
                }
            };
            match service.task_move(tid, target, webid) {
                Ok(task) => println!("  Task moved to {}", task.status),
                Err(e) => println!("  Error: {e}"),
            }
        }

        "accept" => {
            let task_str = rest.trim();
            if task_str.is_empty() {
                println!("  Usage: /kanban accept <task-id>");
                return;
            }
            let tid = match task_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("  Invalid task ID");
                    return;
                }
            };
            // Agent consents to their own assignment
            let consent = ConsentProof::new(webid, tid);
            match service.task_assign(tid, webid, consent) {
                Ok(task) => println!("  Task accepted: {}", task.title),
                Err(e) => println!("  Error: {e}"),
            }
        }

        "submit" => {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let task_str = parts.first().copied().unwrap_or("");
            let evidence = parts.get(1).copied().unwrap_or("");
            if task_str.is_empty() {
                println!("  Usage: /kanban submit <task-id> <evidence>");
                return;
            }
            let tid = match task_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("  Invalid task ID");
                    return;
                }
            };
            match service.task_verify(tid, evidence, webid) {
                Ok((task, v)) => {
                    println!(
                        "  Verification {} — {}",
                        if v.passed { "PASSED" } else { "FAILED" },
                        v.reasoning
                    );
                    println!("  Task status: {}", task.status);
                }
                Err(e) => println!("  Error: {e}"),
            }
        }

        "decompose" => {
            println!("  \x1b[33mTask decomposition requires LLM integration (Task 6).\x1b[0m");
            println!("  Planned: Given a project description, decompose into");
            println!("  agile-sized tasks with acceptance criteria and populate");
            println!("  the kanban board. Target task size: configurable via");
            println!("  kanban skill manifest.");
            println!();
        }

        "spawn" => {
            println!("  \x1b[33mReplicant spawning requires pod infrastructure (future).\x1b[0m");
            println!("  Planned: Spawn a sub-replicant with delegated capabilities");
            println!("  (skills, memory scope, tool access) to execute a kanban task.");
            println!("  Spawning is consent-mediated — the replicant chooses what");
            println!("  to delegate (minimal vs maximal capability transfer).");
            println!();
        }

        _ => {
            println!("  Unknown kanban subcommand: {subcommand}");
            println!("  Try: board, task, move, accept, submit, decompose, spawn");
            println!();
        }
    }
}

fn kanban_service(state: &mut ReplState) -> KanbanService {
    // Use cached service or create new one
    state
        .kanban_service
        .get_or_insert_with(|| {
            let conn = Arc::new(Mutex::new(
                Connection::open_in_memory().expect("in-memory DB"),
            ));
            let store = TripleStore::new(conn);
            store
                .lock_conn()
                .unwrap()
                .execute_batch(
                    "CREATE TABLE IF NOT EXISTS triples (
                        id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                        value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                        confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
                        owner_webid TEXT NOT NULL
                    )",
                )
                .unwrap();
            KanbanService::new(store)
        })
        .clone()
}

fn default_columns() -> Vec<hkask_types::ColumnDef> {
    vec![
        hkask_types::ColumnDef::new("Backlog".into(), hkask_types::TaskStatus::Backlog, 0),
        hkask_types::ColumnDef::new("Ready".into(), hkask_types::TaskStatus::Ready, 1),
        hkask_types::ColumnDef::new("In Progress".into(), hkask_types::TaskStatus::InProgress, 2),
        hkask_types::ColumnDef::new("Review".into(), hkask_types::TaskStatus::Review, 3),
        hkask_types::ColumnDef::new("Done".into(), hkask_types::TaskStatus::Done, 4),
    ]
}
