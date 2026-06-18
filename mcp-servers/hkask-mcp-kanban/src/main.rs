//! hkask-mcp-kanban — Kanban board coordination MCP server.
//!
//! Provides 7 MCP tools for kanban board and task management.
//! All tools carry the caller's WebID for P12 compliance.

pub mod types;

use hkask_mcp::server::{ServerContext, ToolSpanGuard};
use hkask_services::KanbanService;
use hkask_storage::Store;
use hkask_storage::TripleStore;
use hkask_types::{ConsentProof, TaskFilter, TaskSpec, VerificationCriterion, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use types::*;

// ── Helpers ────────────────────────────────────────────────────────────────

fn respond<T: serde::Serialize>(span: ToolSpanGuard, resp: &T) -> String {
    match serde_json::to_value(resp) {
        Ok(val) => span.ok_json(val),
        Err(e) => {
            span.internal_error(serde_json::json!({"error": format!("serialization failed: {e}")}))
        }
    }
}

fn err(span: ToolSpanGuard, msg: &str) -> String {
    span.internal_error(serde_json::json!({"error": msg}))
}

// ── Server ──────────────────────────────────────────────────────────────────

#[allow(dead_code)] // fields read by future CNS/daemon integration
pub struct KanbanServer {
    service: KanbanService,
    webid: WebID,
    /// Replicant identity serving this MCP server
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<hkask_mcp::DaemonClient>,
}

impl KanbanServer {
    pub fn new(webid: WebID, replicant: String, daemon: Option<hkask_mcp::DaemonClient>) -> Self {
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
        Self {
            service: KanbanService::new(store),
            webid,
            replicant,
            daemon,
        }
    }
}

#[tool_router(server_handler)]
impl KanbanServer {
    #[tool(description = "Create a new kanban board with optional custom columns")]
    async fn kanban_board_create(
        &self,
        Parameters(BoardCreateRequest {
            name,
            columns,
            capability_token: _cap,
        }): Parameters<BoardCreateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_board_create", &self.webid);
        let column_defs = match columns {
            Some(inputs) => inputs
                .into_iter()
                .enumerate()
                .map(
                    |(i, input)| match hkask_types::TaskStatus::parse_str(&input.status) {
                        Some(s) => Ok(hkask_types::ColumnDef::new(input.name, s, i as u32)),
                        None => Err(format!("invalid status: {}", input.status)),
                    },
                )
                .collect::<Result<Vec<_>, _>>(),
            None => Ok(default_columns()),
        };
        let cols = match column_defs {
            Ok(c) => c,
            Err(e) => return err(span, &e),
        };
        match self.service.board_create(self.webid, &name, &cols) {
            Ok(board) => respond(
                span,
                &BoardCreateResponse {
                    board_id: board.id.to_string(),
                    name: board.name,
                    columns: board
                        .columns
                        .iter()
                        .map(|c| ColumnInfo {
                            id: c.id.to_string(),
                            name: c.name.clone(),
                            status: c.status.to_string(),
                        })
                        .collect(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "List all kanban boards owned by the caller")]
    async fn kanban_board_list(
        &self,
        Parameters(BoardListRequest {
            capability_token: _cap,
        }): Parameters<BoardListRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_board_list", &self.webid);
        match self.service.board_list(&self.webid) {
            Ok(boards) => respond(
                span,
                &BoardListResponse {
                    boards: boards
                        .into_iter()
                        .map(|b| BoardInfo {
                            board_id: b.id.to_string(),
                            name: b.name,
                            column_count: b.columns.len(),
                        })
                        .collect(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "Create a new task on a kanban board")]
    async fn kanban_task_create(
        &self,
        Parameters(TaskCreateRequest {
            board_id,
            title,
            description,
            criteria,
            assignee_webid,
            capability_token: _cap,
        }): Parameters<TaskCreateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_task_create", &self.webid);
        let bid = match board_id.parse::<hkask_types::BoardId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid board_id: {e}")),
        };
        let mut spec = TaskSpec::new(title);
        if let Some(d) = description {
            spec = spec.with_description(d);
        }
        if let Some(cs) = criteria {
            spec = spec.with_criteria(cs.into_iter().map(VerificationCriterion::new).collect());
        }
        if let Some(a) = assignee_webid {
            match a.parse::<hkask_types::WebID>() {
                Ok(w) => spec = spec.with_assignee(w),
                Err(e) => return err(span, &format!("invalid assignee: {e}")),
            }
        }
        match self.service.task_create(bid, spec, self.webid) {
            Ok(task) => respond(
                span,
                &TaskCreateResponse {
                    task_id: task.id.to_string(),
                    board_id: task.board_id.to_string(),
                    title: task.title,
                    status: task.status.to_string(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "List tasks on a kanban board, optionally filtered by status")]
    async fn kanban_task_list(
        &self,
        Parameters(TaskListRequest {
            board_id,
            status,
            capability_token: _cap,
        }): Parameters<TaskListRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_task_list", &self.webid);
        let bid = match board_id.parse::<hkask_types::BoardId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid board_id: {e}")),
        };
        let filter = match status {
            Some(s) => match hkask_types::TaskStatus::parse_str(&s) {
                Some(st) => TaskFilter::by_status(st),
                None => return err(span, &format!("invalid status: {s}")),
            },
            None => TaskFilter::all(),
        };
        match self.service.task_list(bid, filter) {
            Ok(tasks) => respond(
                span,
                &TaskListResponse {
                    tasks: tasks
                        .into_iter()
                        .map(|t| TaskInfo {
                            task_id: t.id.to_string(),
                            title: t.title,
                            status: t.status.to_string(),
                            assignee: t.assignee.map(|a| a.to_string()),
                            criteria_count: t.criteria.len(),
                        })
                        .collect(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "Move a task to a new column (status transition)")]
    async fn kanban_task_move(
        &self,
        Parameters(TaskMoveRequest {
            task_id,
            target_status,
            capability_token: _cap,
        }): Parameters<TaskMoveRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_task_move", &self.webid);
        let tid = match task_id.parse::<hkask_types::TaskId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid task_id: {e}")),
        };
        let target = match hkask_types::TaskStatus::parse_str(&target_status) {
            Some(s) => s,
            None => return err(span, &format!("invalid target_status: {target_status}")),
        };
        match self.service.task_move(tid, target, self.webid) {
            Ok(task) => respond(
                span,
                &TaskMoveResponse {
                    task_id: task.id.to_string(),
                    previous_status: target_status,
                    new_status: task.status.to_string(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "Assign a task to an agent with consent proof (P1 compliance)")]
    async fn kanban_task_assign(
        &self,
        Parameters(TaskAssignRequest {
            task_id,
            agent_webid,
            consent_proof_agent_webid,
            capability_token: _cap,
        }): Parameters<TaskAssignRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_task_assign", &self.webid);
        let tid = match task_id.parse::<hkask_types::TaskId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid task_id: {e}")),
        };
        let agent = match agent_webid.parse::<hkask_types::WebID>() {
            Ok(a) => a,
            Err(e) => return err(span, &format!("invalid agent: {e}")),
        };
        let consent_agent = match consent_proof_agent_webid.parse::<hkask_types::WebID>() {
            Ok(a) => a,
            Err(e) => return err(span, &format!("invalid consent agent: {e}")),
        };
        match self
            .service
            .task_assign(tid, agent, ConsentProof::new(consent_agent, tid))
        {
            Ok(task) => respond(
                span,
                &TaskAssignResponse {
                    task_id: task.id.to_string(),
                    assignee: task.assignee.map(|a| a.to_string()).unwrap_or_default(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }

    #[tool(description = "Verify a task against its acceptance criteria")]
    async fn kanban_task_verify(
        &self,
        Parameters(TaskVerifyRequest {
            task_id,
            evidence,
            capability_token: _cap,
        }): Parameters<TaskVerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("kanban_task_verify", &self.webid);
        let tid = match task_id.parse::<hkask_types::TaskId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid task_id: {e}")),
        };
        match self.service.task_verify(tid, &evidence, self.webid) {
            Ok((task, verification)) => respond(
                span,
                &TaskVerifyResponse {
                    task_id: task.id.to_string(),
                    passed: verification.passed,
                    reasoning: verification.reasoning,
                    new_status: task.status.to_string(),
                },
            ),
            Err(e) => err(span, &e.to_string()),
        }
    }
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

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.kanban", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-kanban",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let server = KanbanServer::new(ctx.webid, replicant.clone(), daemon_client.clone());
            Ok(server)
        },
        vec![],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "kanban", &[]).await?;
    tracing::info!(target: "hkask.mcp.kanban", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
