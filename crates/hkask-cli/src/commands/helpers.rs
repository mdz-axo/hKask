//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

use std::path::Path;

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

/// Build an AgentService from environment config. Shared across all commands
/// that previously duplicated `build_service_context()`.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  service config must be resolvable from environment
/// post: builds and returns an AgentService from environment config; exits on failure
pub fn build_service_context() -> hkask_services::AgentService {
    let config = or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve service config",
    );
    // Use current runtime if available, otherwise create a fresh one.
    // This avoids "Cannot start a runtime from within a runtime" panics
    // when called from inside an existing tokio context.
    let result: Result<hkask_services::AgentService, String> =
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let cfg = config.clone();
                handle.spawn(async move {
                    let _ = tx.send(hkask_services::AgentService::build(cfg).await);
                });
                rx.recv()
                    .map_err(|_| "Service build task panicked".to_string())
                    .and_then(|r| r.map_err(|e| e.to_string()))
            }
            Err(_) => {
                let rt = tokio::runtime::Runtime::new().expect("runtime should start");
                rt.block_on(hkask_services::AgentService::build(config))
                    .map_err(|e| e.to_string())
            }
        };
    or_exit(result, "Failed to build service context")
}

/// Write content to a file or print to stdout.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  content is a non-empty string; output is an optional file path; label is a human-readable description
/// post: writes content to the file path if provided, or prints to stdout; exits on write failure
pub fn write_or_print(content: &str, output: Option<&Path>, label: &str) {
    match output {
        Some(path) => {
            if let Some(parent) = path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                eprintln!(
                    "Failed to create output directory {}: {}",
                    parent.display(),
                    e
                );
                std::process::exit(1);
            }
            if let Err(e) = std::fs::write(path, content) {
                eprintln!("Failed to write {}: {}", label, e);
                std::process::exit(1);
            }
            println!("{} written to {}", label, path.display());
        }
        None => println!("{}", content),
    }
}

/// Run an async future on the tokio runtime and exit on error.
///
/// Shorthand for `or_exit(rt.block_on($fut), $label)`.
/// Eliminates the repeated `or_exit(rt.block_on(...), "...")` boilerplate
/// across command handlers.
#[macro_export]
macro_rules! block_on {
    ($rt:expr, $fut:expr, $label:literal) => {
        $crate::commands::helpers::or_exit($rt.block_on($fut), $label)
    };
}

pub fn resolve_user_webid() -> hkask_types::WebID {
    if let Ok(uuid_str) = std::env::var("HKASK_WEBID")
        && let Ok(webid) = uuid_str.parse::<hkask_types::WebID>()
    {
        return webid;
    }
    hkask_types::WebID::from_persona(b"cli-user")
}

/// Start a single MCP server and trace the result. Returns true on success.
pub fn start_mcp_server(
    rt: &tokio::runtime::Runtime,
    ctx: &hkask_services::AgentService,
    server_id: &str,
    command: &str,
) -> bool {
    match rt.block_on(ctx.mcp_runtime().start_server(server_id, command)) {
        Ok(()) => {
            tracing::info!(target: "hkask.cli", server_id = %server_id, "MCP server started");
            true
        }
        Err(e) => {
            tracing::warn!(target: "hkask.cli", server_id = %server_id, error = %e, "Failed to start MCP server");
            false
        }
    }
}

/// Start MCP servers with extra environment overrides. Returns count of successfully started servers.
pub fn start_mcp_servers_with_env(
    rt: &tokio::runtime::Runtime,
    ctx: &hkask_services::AgentService,
    servers: &[(&str, &str)],
    replicant_name: &str,
) -> usize {
    let mut extra_env = std::collections::HashMap::new();
    extra_env.insert("HKASK_REPLICANT".to_string(), replicant_name.to_string());
    let mut started = 0;
    for (server_id, command) in servers {
        match rt.block_on(ctx.mcp_runtime().start_server_with_env(
            server_id,
            command,
            extra_env.clone(),
        )) {
            Ok(()) => {
                started += 1;
                tracing::info!(target: "hkask.cli", server_id = %server_id, "MCP server started");
            }
            Err(e) => {
                tracing::warn!(target: "hkask.cli", server_id = %server_id, error = %e, "Failed to start MCP server");
            }
        }
    }
    started
}
