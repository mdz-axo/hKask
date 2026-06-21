//! hkask-mcp-kanban — Kanban board coordination MCP server.
//!
//! Provides 8 MCP tools for kanban board and task management.
//! All tools carry the caller's WebID for P12 compliance.
//!
//! The KanbanServer struct and tool methods are exported from the library
//! target to enable fuzz testing (P5 Testing Discipline, P4 Clear Boundaries).

pub mod types;

use hkask_mcp::server::{ServerContext, ToolSpanGuard};
use hkask_services::KanbanService;
use hkask_services_kanban::{ConsentProof, TaskFilter, TaskSpec, VerificationCriterion};
use hkask_storage::Store;
use hkask_storage::TripleStore;
use hkask_types::WebID;
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
    pub service: KanbanService,
    pub webid: WebID,
    /// Replicant identity serving this MCP server
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<hkask_mcp::DaemonClient>,
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
    pub async fn kanban_board_create(
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
                .map(|(i, input)| {
                    match hkask_services_kanban::TaskStatus::parse_str(&input.status) {
                        Some(s) => Ok(hkask_services_kanban::ColumnDef::new(
                            input.name, s, i as u32,
                        )),
                        None => Err(format!("invalid status: {}", input.status)),
                    }
                })
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
    pub async fn kanban_board_list(
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
    pub async fn kanban_task_create(
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
    pub async fn kanban_task_list(
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
            Some(s) => match hkask_services_kanban::TaskStatus::parse_str(&s) {
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
    pub async fn kanban_task_move(
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
        let target = match hkask_services_kanban::TaskStatus::parse_str(&target_status) {
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
    pub async fn kanban_task_assign(
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
    pub async fn kanban_task_verify(
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

    /// Create kanban tasks for contracts missing `expect:` user-voice annotations.
    ///
    /// Takes a JSON list of ExpectProposal structs (from test-harness
    /// `propose_missing_expect_annotations`) and creates a task per contract gap.
    /// Owning replicants can claim and resolve these tasks by submitting
    /// `expect:` annotation PRs (P2 consent required for merge).
    ///
    /// contract: P3-svc-kanban-009
    /// expect: "I can create kanban tasks from contract expectation gaps so replicants can ground them" \[P3\]
    /// \[P5\] Constraining: Essentialism — one batch operation, no individual task editing
    /// pre:  proposals is a non-empty JSON array of ExpectProposal structs
    /// pre:  board_id is a valid board ID
    /// post: returns created task IDs (one per proposal)
    #[tool(
        description = "Create kanban tasks for contracts missing expect: annotations. Takes JSON from propose_missing_expect_annotations."
    )]
    pub async fn contract_propose_expect(
        &self,
        Parameters(ContractProposeExpect {
            board_id,
            proposals_json,
        }): Parameters<ContractProposeExpect>,
    ) -> String {
        let span = ToolSpanGuard::new("contract_propose_expect", &self.webid);

        let bid = match board_id.parse::<hkask_types::BoardId>() {
            Ok(id) => id,
            Err(e) => return err(span, &format!("invalid board_id: {e}")),
        };

        let proposals: Vec<hkask_test_harness::ExpectProposal> =
            match serde_json::from_str(&proposals_json) {
                Ok(p) => p,
                Err(e) => return err(span, &format!("invalid proposals JSON: {e}")),
            };

        if proposals.is_empty() {
            return err(span, "proposals must be non-empty");
        }

        let mut created: Vec<String> = Vec::new();
        for prop in &proposals {
            let title = format!(
                "contract({}): add expect: to {}",
                prop.crate_name, prop.function,
            );
            let description = format!(
                "File: {}:{}\nContract: {}\nPre: {}\nPost: {}\n\nTemplate:\n{}\n\nSuggested principle: {}\nConstraining: {:?}",
                prop.file,
                prop.line,
                prop.contract_id,
                prop.pre,
                prop.post,
                prop.expect_template,
                prop.suggested_goal_principle,
                prop.existing_constraining_principles,
            );
            let spec = TaskSpec::new(title).with_description(description);
            match self.service.task_create(bid, spec, self.webid) {
                Ok(task) => created.push(task.id.to_string()),
                Err(e) => {
                    return err(
                        span,
                        &format!("failed to create task for {}: {e}", prop.function),
                    );
                }
            }
        }

        respond(
            span,
            &serde_json::json!({
                "created": created.len(),
                "task_ids": created,
                "crate": proposals[0].crate_name,
            }),
        )
    }
}

pub fn default_columns() -> Vec<hkask_services_kanban::ColumnDef> {
    vec![
        hkask_services_kanban::ColumnDef::new(
            "Backlog".into(),
            hkask_services_kanban::TaskStatus::Backlog,
            0,
        ),
        hkask_services_kanban::ColumnDef::new(
            "Ready".into(),
            hkask_services_kanban::TaskStatus::Ready,
            1,
        ),
        hkask_services_kanban::ColumnDef::new(
            "In Progress".into(),
            hkask_services_kanban::TaskStatus::InProgress,
            2,
        ),
        hkask_services_kanban::ColumnDef::new(
            "Review".into(),
            hkask_services_kanban::TaskStatus::Review,
            3,
        ),
        hkask_services_kanban::ColumnDef::new(
            "Done".into(),
            hkask_services_kanban::TaskStatus::Done,
            4,
        ),
    ]
}

/// Run the kanban MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
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
