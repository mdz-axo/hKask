//! hkask-mcp-kanban — Kanban board coordination MCP server.
//!
//! Provides 8 MCP tools for kanban board and task management.
//! All tools carry the caller's WebID for P12 compliance.
//!
//! The KanbanServer struct and tool methods are exported from the library
//! target to enable fuzz testing (P5 Testing Discipline, P4 Clear Boundaries).

pub mod types;

use hkask_mcp::server::{McpToolError, ServerContext, execute_tool};
use hkask_services_kanban::KanbanService;
use hkask_services_kanban::{ConsentProof, TaskFilter, TaskSpec, VerificationCriterion};
use hkask_storage::Store;
use hkask_storage::TripleStore;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use types::*;

// ── Server ──────────────────────────────────────────────────────────────────

pub struct KanbanServer {
    pub service: KanbanService,
    pub webid: WebID,
    /// Replicant identity serving this MCP server
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<hkask_mcp::DaemonClient>,
    /// Per-agent persistent database connection (None if not yet opened)
    pub db: Option<Arc<Mutex<Connection>>>,
}

impl KanbanServer {
    pub fn new(
        service: KanbanService,
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        db: Option<Arc<Mutex<Connection>>>,
    ) -> Self {
        Self {
            service,
            webid,
            replicant,
            daemon,
            db,
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
        execute_tool(self, "kanban_board_create", async {
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
                Err(e) => return Err(McpToolError::invalid_argument(e)),
            };
            match self.service.board_create(self.webid, &name, &cols) {
                Ok(board) => Ok(serde_json::to_value(BoardCreateResponse {
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
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
    }

    #[tool(description = "List all kanban boards owned by the caller")]
    pub async fn kanban_board_list(
        &self,
        Parameters(BoardListRequest {
            capability_token: _cap,
        }): Parameters<BoardListRequest>,
    ) -> String {
        execute_tool(self, "kanban_board_list", async {
            match self.service.board_list(&self.webid) {
                Ok(boards) => Ok(serde_json::to_value(BoardListResponse {
                    boards: boards
                        .into_iter()
                        .map(|b| BoardInfo {
                            board_id: b.id.to_string(),
                            name: b.name,
                            column_count: b.columns.len(),
                        })
                        .collect(),
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "kanban_task_create", async {
            let bid = match board_id.parse::<hkask_types::BoardId>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid board_id: {e}"
                    )));
                }
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
                    Err(e) => {
                        return Err(McpToolError::invalid_argument(format!(
                            "invalid assignee: {e}"
                        )));
                    }
                }
            }
            match self.service.task_create(bid, spec, self.webid) {
                Ok(task) => Ok(serde_json::to_value(TaskCreateResponse {
                    task_id: task.id.to_string(),
                    board_id: task.board_id.to_string(),
                    title: task.title,
                    status: task.status.to_string(),
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "kanban_task_list", async {
            let bid = match board_id.parse::<hkask_types::BoardId>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid board_id: {e}"
                    )));
                }
            };
            let filter = match status {
                Some(s) => match hkask_services_kanban::TaskStatus::parse_str(&s) {
                    Some(st) => TaskFilter::by_status(st),
                    None => {
                        return Err(McpToolError::invalid_argument(format!(
                            "invalid status: {s}"
                        )));
                    }
                },
                None => TaskFilter::all(),
            };
            match self.service.task_list(bid, filter) {
                Ok(tasks) => Ok(serde_json::to_value(TaskListResponse {
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
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "kanban_task_move", async {
            let tid = match task_id.parse::<hkask_types::TaskId>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid task_id: {e}"
                    )));
                }
            };
            let target = match hkask_services_kanban::TaskStatus::parse_str(&target_status) {
                Some(s) => s,
                None => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid target_status: {target_status}"
                    )));
                }
            };
            match self.service.task_move(tid, target, self.webid) {
                Ok(task) => Ok(serde_json::to_value(TaskMoveResponse {
                    task_id: task.id.to_string(),
                    previous_status: target_status,
                    new_status: task.status.to_string(),
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "kanban_task_assign", async {
            let tid = match task_id.parse::<hkask_types::TaskId>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid task_id: {e}"
                    )));
                }
            };
            let agent = match agent_webid.parse::<hkask_types::WebID>() {
                Ok(a) => a,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid agent: {e}"
                    )));
                }
            };
            let consent_agent = match consent_proof_agent_webid.parse::<hkask_types::WebID>() {
                Ok(a) => a,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid consent agent: {e}"
                    )));
                }
            };
            match self
                .service
                .task_assign(tid, agent, ConsentProof::new(consent_agent, tid))
            {
                Ok(task) => Ok(serde_json::to_value(TaskAssignResponse {
                    task_id: task.id.to_string(),
                    assignee: task.assignee.map(|a| a.to_string()).unwrap_or_default(),
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "kanban_task_verify", async {
            let tid = match task_id.parse::<hkask_types::TaskId>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "invalid task_id: {e}"
                    )));
                }
            };
            match self.service.task_verify(tid, &evidence, self.webid) {
                Ok((task, verification)) => Ok(serde_json::to_value(TaskVerifyResponse {
                    task_id: task.id.to_string(),
                    passed: verification.passed,
                    reasoning: verification.reasoning,
                    new_status: task.status.to_string(),
                })
                .unwrap()),
                Err(e) => Err(McpToolError::invalid_argument(e.to_string())),
            }
        })
        .await
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
        execute_tool(self, "contract_propose_expect", async {
            let bid = match board_id.parse::<hkask_types::BoardId>() {
                Ok(id) => id,
                Err(e) => return Err(McpToolError::invalid_argument(format!("invalid board_id: {e}"))),
            };

            let proposals: Vec<hkask_test_harness::ExpectProposal> =
                match serde_json::from_str(&proposals_json) {
                    Ok(p) => p,
                    Err(e) => return Err(McpToolError::invalid_argument(format!("invalid proposals JSON: {e}"))),
                };

            if proposals.is_empty() {
                return Err(McpToolError::invalid_argument("proposals must be non-empty"));
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
                        return Err(McpToolError::invalid_argument(format!(
                            "failed to create task for {}: {e}",
                            prop.function,
                        )));
                    }
                }
            }

            Ok(serde_json::json!({
                "created": created.len(),
                "task_ids": created,
                "crate": proposals[0].crate_name,
            }))
        })
        .await
    }
}

impl hkask_mcp::server::ToolContext for KanbanServer {
    fn webid(&self) -> &hkask_types::WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
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
            Ok((|| -> anyhow::Result<KanbanServer> {
                // Use the standard per-agent kanban DB path when not explicitly set.
                let kanban_db_path = ctx
                    .credentials
                    .get("HKASK_KANBAN_DB")
                    .cloned()
                    .unwrap_or_else(|| {
                        let default_path = hkask_types::agent_paths::agent_kanban_db(&replicant);
                        if let Some(parent) = default_path.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }
                        tracing::info!(
                            target: "hkask.mcp.kanban",
                            path = %default_path.display(),
                            replicant = %replicant,
                            "Using default per-agent kanban database"
                        );
                        default_path.to_string_lossy().to_string()
                    });
                let db = if let Some(passphrase) = ctx.credentials.get("HKASK_DB_PASSPHRASE") {
                    hkask_storage::Database::open(&kanban_db_path, passphrase)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    hkask_storage::Database::in_memory().map_err(|e| anyhow::anyhow!("{e}"))?
                };
                let conn = db.conn_arc();
                let store = TripleStore::new(Arc::clone(&conn));
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
                let service = KanbanService::new(store);
                Ok(KanbanServer::new(
                    service,
                    ctx.webid,
                    replicant.clone(),
                    daemon_client.clone(),
                    Some(db.conn_arc()),
                ))
            })()?)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_KANBAN_DB",
                "Path to per-agent kanban database file (defaults to agents/{replicant}/kanban.db)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "SQLCipher encryption passphrase (resolved via hkask keystore chain when not set)",
            ),
        ],
    )
    .await
}
