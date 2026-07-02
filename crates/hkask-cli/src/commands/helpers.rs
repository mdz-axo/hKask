//! Shared helper functions for CLI command handlers

use std::path::{Path, PathBuf};

use hkask_services_context::AgentService;
use hkask_services_core::ServiceConfig;
use hkask_services_onboarding::ResolvedSecrets;

/// Unwrap a `Result` or print an error message and exit.
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
/// Always creates a fresh tokio runtime. Suitable for one-shot CLI commands.
pub fn build_agent_service() -> AgentService {
    or_exit(
        build_agent_service_inner(None),
        "Failed to build AgentService",
    )
}

/// Build an AgentService from pre-resolved secrets (used by the chat path).
///
/// Returns `Result` — callers handle errors gracefully rather than exiting.
pub fn build_agent_service_from_secrets(
    from_secrets: Option<(&str, &ResolvedSecrets)>,
) -> Result<AgentService, String> {
    build_agent_service_inner(from_secrets)
}

/// Shared implementation — safe to call from any context.
fn build_agent_service_inner(
    from_secrets: Option<(&str, &ResolvedSecrets)>,
) -> Result<AgentService, String> {
    let config = match from_secrets {
        Some((name, secrets)) => ServiceConfig::from_secrets(
            secrets.a2a_secret.clone(),
            secrets.db_passphrase.clone(),
            secrets.mcp_secret.clone(),
            name.to_string(),
        ),
        None => ServiceConfig::from_env()
            .map_err(|e| format!("Failed to resolve service config: {}", e))?,
    };
    // `Runtime::block_on` panics if the current thread is driving a tokio
    // runtime (e.g., when called from within `rt.block_on(...)`). Use
    // `block_in_place` to move blocking work to the blocking thread pool.
    match tokio::runtime::Handle::try_current() {
        Ok(_handle) => tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {}", e))?;
            rt.block_on(AgentService::build(config))
                .map_err(|e| e.to_string())
        }),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {}", e))?;
            rt.block_on(AgentService::build(config))
                .map_err(|e| e.to_string())
        }
    }
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

/// Resolve the current user's WebID.
pub fn resolve_user_webid() -> hkask_types::WebID {
    let name = std::env::var("HKASK_USER_WEBID").unwrap_or_else(|_| "cli-user".to_string());
    hkask_types::WebID::from_persona_with_namespace(name.as_bytes(), "replicant")
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

/// Resolve the deploy directory from environment or default.
pub fn resolve_deploy_dir() -> Result<PathBuf, String> {
    if let Ok(d) = std::env::var("HKASK_DEPLOY_DIR") {
        let p = PathBuf::from(&d);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(format!("HKASK_DEPLOY_DIR set but not a directory: {d}"));
    }
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    let candidate = cwd.join("deploy").join("k8s");
    if candidate.is_dir() {
        return Ok(candidate);
    }
    Err(format!(
        "deploy/k8s not found at {} and HKASK_DEPLOY_DIR not set",
        cwd.display()
    ))
}

/// Print a list of items with a header.
pub fn print_item_list<T>(
    items: &[T],
    empty_label: &str,
    label: &str,
    format_item: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        println!("{}", empty_label);
        return;
    }
    println!("{} ({}):", label, items.len());
    for item in items {
        println!("  - {}", format_item(item));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: `build_agent_service()` must not panic when called from
    /// inside `rt.block_on()`. The old `build_service_context_inner` used
    /// `Handle::block_on` which panicked with "Cannot start a runtime from
    /// within a runtime" when a tokio handle was active.
    #[test]
    fn build_agent_service_from_within_block_on() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        // This must not panic — the original bug caused a panic here
        let _svc = rt.block_on(async { build_agent_service() });
    }
}
