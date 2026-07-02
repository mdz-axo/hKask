//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

use std::path::{Path, PathBuf};

use hkask_services_context::AgentService;
use hkask_services_core::ServiceConfig;

/// Unwrap a `Result` or print an error message and exit.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  result is a `Result<T, E>`; label is a human-readable context string
/// post: returns Ok value or prints "{label}: {error}" to stderr and exits with code 1
pub fn or_exit<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {}", label, e);
            std::process::exit(1);
        }
    }
}

/// Build an AgentService from environment config.
///
/// Always creates a fresh tokio runtime — safe to call from any context.
/// This is the single entry point for one-shot CLI commands.
pub fn build_agent_service() -> AgentService {
    let config = or_exit(
        ServiceConfig::from_env(),
        "Failed to resolve service config",
    );
    let rt =
        tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for service build");
    or_exit(
        rt.block_on(AgentService::build(config)),
        "Failed to build AgentService",
    )
}

/// Write content to a file or print to stdout.
pub fn write_or_print(content: &str, output: Option<&Path>, label: &str) {
    match output {
        Some(path) => {
            std::fs::write(path, content).unwrap_or_else(|e| {
                eprintln!("Failed to write {} to {}: {}", label, path.display(), e);
                std::process::exit(1);
            });
        }
        None => println!("{}", content),
    }
}

/// Resolve the current user's WebID for commands that need an author identity.
pub fn resolve_user_webid() -> hkask_types::WebID {
    let name = std::env::var("HKASK_USER_WEBID").unwrap_or_else(|_| "cli-user".to_string());
    hkask_types::WebID::from_persona_with_namespace(name.as_bytes(), "replicant")
}

/// Start a specific MCP server.
pub fn start_mcp_server(
    rt: &tokio::runtime::Runtime,
    ctx: &AgentService,
    server_id: &str,
    binary: &str,
) -> bool {
    let mcp_runtime = ctx.infra().mcp.clone();
    let replicant_name = ctx.config().agent_name.clone();
    let mut env = std::collections::HashMap::new();
    env.insert("HKASK_MCP_HOST".to_string(), replicant_name);
    match rt.block_on(
        mcp_runtime
            .as_ref()
            .start_server_with_env(server_id, binary, env),
    ) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("Failed to start MCP server '{}': {}", server_id, e);
            false
        }
    }
}

/// Start MCP servers with custom environment.
pub fn start_mcp_servers_with_env(
    rt: &tokio::runtime::Runtime,
    ctx: &AgentService,
    servers: &[(&str, &str)],
    replicant_name: &str,
) {
    let mcp_runtime = ctx.infra().mcp.clone();
    let mut env = std::collections::HashMap::new();
    env.insert("HKASK_MCP_HOST".to_string(), replicant_name.to_string());
    for (server_id, binary) in servers {
        if let Err(e) = rt.block_on(mcp_runtime.as_ref().start_server_with_env(
            server_id,
            binary,
            env.clone(),
        )) {
            eprintln!("Failed to start MCP server '{}': {}", server_id, e);
        }
    }
}

/// Resolve an agent name from an optional argument or environment.
pub fn resolve_agent_name(agent: Option<&str>) -> String {
    if let Some(name) = agent {
        return name.to_string();
    }
    std::env::var("HKASK_MCP_HOST").unwrap_or_else(|_| "anonymous".to_string())
}

/// Resolve a WebID from an optional argument or derive from agent name.
pub fn resolve_webid(agent: Option<&str>) -> hkask_types::WebID {
    let name = resolve_agent_name(agent);
    hkask_types::WebID::from_persona_with_namespace(name.as_bytes(), "replicant")
}

/// Resolve the deploy directory from environment or default.
pub fn resolve_deploy_dir() -> PathBuf {
    std::env::var("HKASK_DEPLOY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hkask")
                .join("deploy")
        })
}

/// Print a list of items with a header.
pub fn print_item_list(header: &str, items: &[String]) {
    if items.is_empty() {
        println!("{} (none)", header);
    } else {
        println!("{}:", header);
        for item in items {
            println!("  - {}", item);
        }
    }
}
