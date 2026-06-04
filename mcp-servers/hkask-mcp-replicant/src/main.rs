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
//! - `OKAPI_BASE_URL` — Okapi API base URL (default: "http://127.0.0.1:11435")
//!
//! # ACP Integration
//!
//! The server resolves ACP secrets through the full derivation chain (master key →
//! env → keychain → insecure dev), ensuring capability tokens are compatible with
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

hkask_mcp::mcp_server_main!(
    "hkask-mcp-replicant",
    factory: |ctx: ServerContext| {
        let persona = std::env::var("HKASK_AGENT_PERSONA")
            .unwrap_or_else(|_| "Curator".to_string());
        let default_model = std::env::var("HKASK_DEFAULT_MODEL")
            .unwrap_or_else(|_| "deepseek-v4-pro".to_string());
        ReplicantServer::new(ctx.webid, &persona, &default_model)
    }
);
