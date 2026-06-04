//! hKask MCP Goal — Goal coordination substrate tools.
//!
//! Mirrors the CLI `kask goal` surface and the HTTP `/api/goals` routes for
//! MCP ≡ CLI ≡ API equivalence (REQ-IFC-001). All operations are OCAP-gated via
//! `GoalCapabilityToken`, authority is co-located with effect (owner/visibility
//! checks on every write), and denials are observed through the goal
//! repository's CNS telemetry sink (`cns.tool.goal.capability.denied`, ADR-029).

use hkask_mcp::server::{McpToolError, McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_storage::{Database, NuEventStore, SqliteGoalRepository};
use hkask_types::event::NuEventSink;
use hkask_types::goal::GoalState;
use hkask_types::goal_capability::{GoalCapabilityToken, GoalOp};
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
    secret: Vec<u8>,
}

impl GoalServer {
    /// Construct from a server context.
    ///
    /// - `HKASK_GOAL_DB` (optional): path to the SQLite database, plus
    ///   `HKASK_DB_PASSPHRASE` for encryption. Absent → in-memory (ephemeral).
    /// - `HKASK_OCAP_SECRET` (required): hex-encoded secret used to mint and
    ///   verify goal capability tokens.
    pub fn new(ctx: hkask_mcp::ServerContext) -> anyhow::Result<Self> {
        let conn = match ctx.credentials.get("HKASK_GOAL_DB") {
            Some(path) => {
                let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                    anyhow::anyhow!("HKASK_GOAL_DB set but HKASK_DB_PASSPHRASE missing")
                })?;
                Database::open(path, passphrase)
                    .map_err(|e| anyhow::anyhow!("Failed to open goal database: {e}"))?
                    .conn_arc()
            }
            None => Database::in_memory()
                .map_err(|e| anyhow::anyhow!("Failed to open in-memory database: {e}"))?
                .conn_arc(),
        };

        let secret_hex = ctx
            .credentials
            .get("HKASK_OCAP_SECRET")
            .ok_or_else(|| anyhow::anyhow!("HKASK_OCAP_SECRET is required for goal capability"))?;
        let secret = hex::decode(secret_hex)
            .map_err(|e| anyhow::anyhow!("HKASK_OCAP_SECRET must be hex-encoded: {e}"))?;

        // Wire CNS denial telemetry over the same connection (ADR-029).
        let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));
        let repo = SqliteGoalRepository::new(conn).with_telemetry(sink);

        Ok(Self {
            repo,
            webid: ctx.webid,
            secret,
        })
    }

    fn mint(&self, goal_id: GoalID, ops: Vec<GoalOp>) -> GoalCapabilityToken {
        GoalCapabilityToken::new(goal_id, self.webid, ops, &self.secret)
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
    #[tool(description = "Create a goal owned by the calling agent (OCAP-gated)")]
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

        let token = self.mint(GoalID::new(), vec![GoalOp::Create]);
        match self.repo.create_goal(&token, &self.webid, &text, vis) {
            Ok(goal) => span.ok(McpToolOutput::new(json!({
                "id": goal.id.to_string(),
                "text": goal.text,
                "state": goal.state.as_str(),
                "visibility": goal.visibility.as_str(),
            }))
            .to_json_string()),
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

        let token = self.mint(GoalID::new(), vec![GoalOp::Read]);
        match self.repo.list_goals(&token, &self.webid, state_filter) {
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
                span.ok(McpToolOutput::new(json!({ "goals": items })).to_json_string())
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

        if let Err(e) = validate_identifier("goal_id", &goal_id, 128) {
            return span.error(e.kind, e.to_json_string());
        }
        let Some(new_state) = GoalState::parse_str(&state) else {
            let err = McpToolError::invalid_argument(
                "state must be pending | active | completed | blocked | abandoned",
            );
            return span.error(err.kind, err.to_json_string());
        };

        let gid = GoalID::from_string(&goal_id);
        let token = self.mint(gid, vec![GoalOp::Update]);
        match self.repo.update_goal_state(&token, gid, new_state) {
            Ok(()) => span.ok(McpToolOutput::new(json!({
                "id": gid.to_string(),
                "state": new_state.as_str(),
            }))
            .to_json_string()),
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

    const SECRET_HEX: &str = "6b61736b2d676f616c2d746573742d7365637265742d33322d627974657321";

    fn server() -> GoalServer {
        let conn = Database::in_memory().expect("in-memory db").conn_arc();
        let secret = hex::decode(SECRET_HEX).expect("hex secret");
        let sink: Arc<dyn NuEventSink> = Arc::new(NuEventStore::new(Arc::clone(&conn)));
        GoalServer {
            repo: SqliteGoalRepository::new(conn).with_telemetry(sink),
            webid: WebID::from_persona(b"mcp-goal-test"),
            secret,
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
        hkask_mcp::CredentialRequirement::required(
            "HKASK_OCAP_SECRET",
            "Hex-encoded OCAP secret for minting/verifying goal capability tokens",
        ),
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
