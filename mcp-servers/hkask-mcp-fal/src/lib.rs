//! hkask-mcp-fal — Fal workflow execution MCP server.
//!
//! Exposes `execute_workflow` as an MCP tool for Strategy D PDCA loops.
//! Thin wrapper around `hkask_fal::FalClient`.
//!
//! # Tool
//!
//! - `execute_workflow` — Execute a workflow plan JSON against Fal GPU
//!   infrastructure. Parses the DAG, topologically sorts nodes, executes each
//!   model call, resolves `$references`, and returns output URLs + metadata.

use hkask_fal::FalClient;
use hkask_mcp::{
    DaemonClient, McpError, ServerContext,
    server::{McpToolError, ToolContext, execute_tool},
};
use hkask_types::WebID;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

// ── Server ──────────────────────────────────────────────────────────────

pub struct FalServer {
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<DaemonClient>,
    /// Fal API client wrapped in Arc for thread safety
    pub fal: Arc<FalClient>,
}

impl ToolContext for FalServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── Tool: execute_workflow ──────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct ExecuteWorkflowRequest {
    /// The workflow plan JSON string. Must be a valid Fal-compatible
    /// workflow DAG with input, run, and display nodes.
    workflow: String,
}

#[tool_router(server_handler)]
impl FalServer {
    #[tool(
        description = "Execute a Fal workflow plan. Provide a JSON string with a DAG of nodes (input, run, display types). Nodes execute in dependency order with $reference resolution between them."
    )]
    async fn execute_workflow(
        &self,
        Parameters(ExecuteWorkflowRequest { workflow }): Parameters<ExecuteWorkflowRequest>,
    ) -> String {
        execute_tool(self, "execute_workflow", async {
            let workflow_json: serde_json::Value = serde_json::from_str(&workflow)
                .map_err(|e| McpToolError::invalid_argument(format!("Invalid JSON: {e}")))?;

            self.fal
                .execute_workflow(&workflow_json)
                .await
                .map(|wr| {
                    serde_json::json!({
                        "output_urls": wr.output_urls,
                        "output_fields": wr.output_fields,
                        "elapsed_seconds": wr.elapsed_seconds,
                    })
                })
                .map_err(|e| McpToolError::unavailable(format!("Workflow execution failed: {e}")))
        })
        .await
    }
}

// ── Run function ────────────────────────────────────────────────────────

/// Run the Fal MCP server (used by binary target).
pub async fn run(replicant: String, daemon_client: Option<DaemonClient>) -> Result<(), McpError> {
    hkask_mcp::run_server(
        "hkask-mcp-fal",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let fal_key = ctx
                .credentials
                .get("HKASK_FAL_API_KEY")
                .cloned()
                .unwrap_or_default();

            if fal_key.is_empty() {
                tracing::warn!(
                    target: "hkask.mcp.fal",
                    "HKASK_FAL_API_KEY not set — workflow execution will fail at runtime"
                );
            }

            Ok(FalServer {
                webid: ctx.webid,
                replicant: replicant.clone(),
                daemon: daemon_client.clone(),
                fal: Arc::new(FalClient::new(fal_key)),
            })
        },
        vec![hkask_mcp::CredentialRequirement::optional(
            "HKASK_FAL_API_KEY",
            "Fal.ai API key for GPU model execution",
        )],
    )
    .await
}
