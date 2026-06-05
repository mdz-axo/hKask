//! hKask MCP Goal — Goal coordination substrate tools.
//!
//! Mirrors the CLI `kask goal` surface and the HTTP `/api/goals` routes for
//! MCP ≡ CLI ≡ API equivalence (REQ-IFC-001). Authority is co-located with
//! effect: the caller's WebID is passed directly to the goal repository, and
//! denials are observed through the goal repository's CNS telemetry sink
//! (ADR-029).

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_storage::{NuEventStore, SqliteGoalRepository};
use hkask_types::event::NuEventSink;
use hkask_types::goal::GoalState;
use hkask_types::id::{GoalID, WebID};
use hkask_types::visibility::Visibility;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateGoalRequest {
    /// Goal text.
    pub text: String,
    /// Visibility: private | shared | public. Defaults to private.
    pub visibility: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListGoalsRequest {
    /// Optional state filter: pending | active | completed | blocked | abandoned.
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetGoalStateRequest {
    /// Goal ID.
    pub goal_id: String,
    /// Target state: pending | active | completed | blocked | abandoned.
    pub state: String,
}

pub struct GoalServer {
    repo: SqliteGoalRepository,
    webid: WebID,
}

impl GoalServer {
    /// Construct from a server context.
    ///
    /// - `HKASK_GOAL_DB` (optional): path to the SQLite database, plus
    ///   `HKASK_DB_PASSPHRASE` for encryption. Absent → in-memory (ephemeral).
    pub fn new(ctx: hkask_mcp::ServerContext) -> anyhow::Result<Self> {
        let db = ctx.open_database("HKASK_GOAL_DB")?;
        let conn = db.conn_arc();

        // Wire CNS denial telemetry over the same connection (ADR-029).
        let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));
        let repo = SqliteGoalRepository::new(conn).with_telemetry(sink);

        Ok(Self {
            repo,
            webid: ctx.webid,
        })
    }

    /// Map a repository error to an MCP tool error of the correct kind.
    fn repo_error(e: hkask_storage::GoalRepositoryError) -> McpToolError {
        use hkask_storage::GoalRepositoryError as E;
        match e {
            E::VisibilityDenied(m) => McpToolError::permission_denied(m),
            E::NotFound(m) => McpToolError::not_found(m),
            E::InvalidTransition(m) | E::MaxDepthExceeded(m) => McpToolError::invalid_argument(m),
            other => McpToolError::internal(other.to_string()),
        }
    }
}

#[tool_router(server_handler)]
impl GoalServer {
    #[tool(description = "Create a goal owned by the calling agent")]
    async fn goal_create(
        &self,
        Parameters(CreateGoalRequest { text, visibility }): Parameters<CreateGoalRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("goal:create", &self.webid);

        // Goal text is free-form prose (not an identifier): bound length and
        // reject empty, but allow arbitrary characters.
        if text.trim().is_empty() {
            let err = McpToolError::invalid_argument("text must not be empty");
            return span.error(err.kind, err.to_json_string());
        }
        if text.len() > 4096 {
            let err = McpToolError::invalid_argument("text exceeds maximum length of 4096");
            return span.error(err.kind, err.to_json_string());
        }
        let visibility_str = visibility.as_deref().unwrap_or("private");
        let Some(vis) = Visibility::parse_str(visibility_str) else {
            let err =
                McpToolError::invalid_argument("visibility must be private | shared | public");
            return span.error(err.kind, err.to_json_string());
        };

        match self.repo.create_goal(&self.webid, &text, vis) {
            Ok(goal) => span.ok_json(json!({
                "id": goal.id.to_string(),
                "text": goal.text,
                "state": goal.state.as_str(),
                "visibility": goal.visibility.as_str(),
            })),
            Err(e) => {
                let err = Self::repo_error(e);
                span.error(err.kind, err.to_json_string())
            }
        }
    }

    #[tool(description = "List the calling agent's goals, optionally filtered by state")]
    async fn goal_list(
        &self,
        Parameters(ListGoalsRequest { state }): Parameters<ListGoalsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("goal:list", &self.webid);

        let state_filter = match &state {
            Some(s) => match GoalState::parse_str(s) {
                Some(st) => Some(st),
                None => {
                    let err = McpToolError::invalid_argument(format!("invalid state filter '{s}'"));
                    return span.error(err.kind, err.to_json_string());
                }
            },
            None => None,
        };

        match self.repo.list_goals(&self.webid, state_filter) {
            Ok(goals) => {
                let items: Vec<serde_json::Value> = goals
                    .into_iter()
                    .map(|g| {
                        json!({
                            "id": g.id.to_string(),
                            "text": g.text,
                            "state": g.state.as_str(),
                            "visibility": g.visibility.as_str(),
                        })
                    })
                    .collect();
                span.ok_json(json!({ "goals": items }))
            }
            Err(e) => {
                let err = Self::repo_error(e);
                span.error(err.kind, err.to_json_string())
            }
        }
    }

    #[tool(description = "Transition a goal to a new state (legal transitions only)")]
    async fn goal_set_state(
        &self,
        Parameters(SetGoalStateRequest { goal_id, state }): Parameters<SetGoalStateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("goal:set_state", &self.webid);

        validate_field!(span, "goal_id", &goal_id, 128);
        let Some(new_state) = GoalState::parse_str(&state) else {
            let err = McpToolError::invalid_argument(
                "state must be pending | active | completed | blocked | abandoned",
            );
            return span.error(err.kind, err.to_json_string());
        };

        let gid = GoalID::from_string(&goal_id);
        match self.repo.update_goal_state(gid, new_state) {
            Ok(()) => span.ok_json(json!({
                "id": gid.to_string(),
                "state": new_state.as_str(),
            })),
            Err(e) => {
                let err = Self::repo_error(e);
                span.error(err.kind, err.to_json_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;

    fn server() -> GoalServer {
        let conn = Database::in_memory().expect("in-memory db").conn_arc();
        let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));
        GoalServer {
            repo: SqliteGoalRepository::new(conn).with_telemetry(sink),
            webid: WebID::from_persona(b"mcp-goal-test"),
        }
    }

    #[tokio::test]
    async fn create_then_list_round_trips() {
        let s = server();
        let created = s
            .goal_create(Parameters(CreateGoalRequest {
                text: "ship the mcp surface".to_string(),
                visibility: Some("private".to_string()),
            }))
            .await;
        assert!(
            created.contains("ship the mcp surface"),
            "create output: {created}"
        );

        let listed = s
            .goal_list(Parameters(ListGoalsRequest { state: None }))
            .await;
        assert!(
            listed.contains("ship the mcp surface"),
            "list output: {listed}"
        );
    }

    #[tokio::test]
    async fn illegal_transition_is_rejected() {
        let s = server();
        let created = s
            .goal_create(Parameters(CreateGoalRequest {
                text: "task".to_string(),
                visibility: None,
            }))
            .await;
        let v: serde_json::Value = serde_json::from_str(&created).expect("json");
        let id = v["content"]["id"].as_str().expect("id").to_string();

        // pending -> completed is illegal (must pass through active).
        let denied = s
            .goal_set_state(Parameters(SetGoalStateRequest {
                goal_id: id,
                state: "completed".to_string(),
            }))
            .await;
        assert!(
            denied.contains("error") && denied.contains("not a legal transition"),
            "illegal transition must be rejected: {denied}"
        );
    }

    #[tokio::test]
    async fn invalid_visibility_is_rejected() {
        let s = server();
        let out = s
            .goal_create(Parameters(CreateGoalRequest {
                text: "x".to_string(),
                visibility: Some("nonsense".to_string()),
            }))
            .await;
        assert!(
            out.contains("error"),
            "invalid visibility must error: {out}"
        );
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-goal",
    factory: |ctx: hkask_mcp::ServerContext| { GoalServer::new(ctx) },
    credentials: vec![
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_GOAL_DB",
            "Path to the goal SQLite database (in-memory if absent)",
        ),
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_DB_PASSPHRASE",
            "Passphrase for the goal database (required if HKASK_GOAL_DB is set)",
        ),
    ]
);
