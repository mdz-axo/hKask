//! hKask MCP Replicant — Replicant chat MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `replicant:chat` — Send a message to a replicant and receive a response
//! - `replicant:status` — Check replicant registration and identity
//! - `replicant:history` — List recent conversation turns in the current session
//!
//! # Environment Variables
//!
//! - `HKASK_AGENT_PERSONA` — Replicant persona name (default: "Curator")
//! - `HKASK_DEFAULT_MODEL` — Default model for inference (default: "deepseek-v4-pro")
//! - `HKASK_REGISTRY_PATH` — Registry path for YAML agent definitions (default: "registry/bots")
//! - `HKASK_DB_PATH` — SQLite registry database path (default: "hkask.db")
//! - `HKASK_DB_PASSPHRASE` — Database encryption passphrase (optional)
//! - `OKAPI_BASE_URL` — Okapi API base URL (default: "http://127.0.0.1:11435")
//!
//! # ACP Integration
//!
//! The server resolves ACP secrets through the full derivation chain (master key →
//! env → keychain → deterministic default), ensuring capability tokens are compatible with
//! the CLI and other MCP servers.
//!
//! # System Prompt Richness
//!
//! If the agent registry database or YAML files are available, the server loads
//! the full agent definition (charter, responsibilities, rights, voice/tone) to
//! construct rich system prompts. Falls back to a minimal persona otherwise.
//!
//! # Session Persistence
//!
//! Conversation history is maintained in-memory across `replicant:chat` calls,
//! providing context continuity. History is bounded to 20 turns to manage token
//! budget.

use hkask_mcp::server::ServerContext;
use hkask_mcp_replicant::tools::ReplicantServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-replicant",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let persona = ctx
                .credentials
                .get("HKASK_AGENT_PERSONA")
                .cloned()
                .unwrap_or_else(|| "Curator".to_string());
            let default_model = ctx
                .credentials
                .get("HKASK_DEFAULT_MODEL")
                .cloned()
                .unwrap_or_else(|| "deepseek-v4-pro".to_string());
            ReplicantServer::new(ctx.webid, &persona, &default_model, Some(&ctx.credentials))
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_AGENT_PERSONA",
                "Replicant persona name (default: Curator)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DEFAULT_MODEL",
                "Default LLM model for inference (default: deepseek-v4-pro)",
            ),
        ],
    )
    .await
}
