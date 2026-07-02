//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

use std::path::{Path, PathBuf};

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

/// Convenience: `build_service_context_inner(None)` with exit-on-failure.
/// Preserves backward compatibility for all call sites that already
/// expected a non-fallible return.
pub fn build_service_context() -> hkask_services_context::AgentService {
    or_exit(
        build_service_context_inner(None),
        "Failed to build service context",
    )
}

/// Build an AgentService from environment config or pre-resolved secrets.
/// Returns `Result` so callers that need graceful error handling (e.g. chat)
/// can map the error to their own response type instead of exiting.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  if `from_secrets` is Some → (agent_name, ResolvedSecrets) used
/// pre:  if `from_secrets` is None → ServiceConfig::from_env() used
/// post: returns Ok(AgentService) or Err(String) describing the failure
pub fn build_service_context_from_secrets(
    from_secrets: Option<(&str, &hkask_services_onboarding::ResolvedSecrets)>,
) -> Result<hkask_services_context::AgentService, String> {
    build_service_context_inner(from_secrets)
}

fn build_service_context_inner(
    from_secrets: Option<(&str, &hkask_services_onboarding::ResolvedSecrets)>,
) -> Result<hkask_services_context::AgentService, String> {
    let config = match from_secrets {
        Some((name, secrets)) => hkask_services_core::ServiceConfig::from_secrets(
            secrets.a2a_secret.clone(),
            secrets.db_passphrase.clone(),
            secrets.mcp_secret.clone(),
            name.to_string(),
        ),
        None => hkask_services_core::ServiceConfig::from_env()
            .map_err(|e| format!("Failed to resolve service config: {}", e))?,
    };
    match tokio::runtime::Handle::try_current() {
        Ok(_handle) => {
            // Already inside a tokio runtime — spawn on a separate OS thread
            // to avoid nested block_on panics (Handle::block_on is forbidden
            // from within a tokio worker or block_on context).
            let (tx, rx) = std::sync::mpsc::channel();
            let cfg = config.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for service build");
                let result = rt.block_on(hkask_services_context::AgentService::build(cfg));
                let _ = tx.send(result);
            });
            rx.recv()
                .map_err(|_| "Service build thread panicked".to_string())
                .and_then(|r| r.map_err(|e| e.to_string()))
        }
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            rt.block_on(hkask_services_context::AgentService::build(config))
                .map_err(|e| e.to_string())
        }
    }
}

/// Write content to a file or print to stdout.
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

pub fn start_mcp_server(
    rt: &tokio::runtime::Runtime,
    ctx: &hkask_services_context::AgentService,
    server_id: &str,
    command: &str,
) -> bool {
    match rt.block_on(ctx.infra().mcp.clone().start_server(server_id, command)) {
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
        println!("  {}", format_item(item));
    }
    println!("{} total.", items.len());
}

pub fn start_mcp_servers_with_env(
    rt: &tokio::runtime::Runtime,
    ctx: &hkask_services_context::AgentService,
    servers: &[(&str, &str)],
    replicant_name: &str,
) -> usize {
    let mut extra_env = std::collections::HashMap::new();
    extra_env.insert("HKASK_MCP_HOST".to_string(), replicant_name.to_string());
    let mut started = 0;
    for (server_id, command) in servers {
        match rt.block_on(ctx.infra().mcp.clone().start_server_with_env(
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

/// Resolve the `deploy/k8s/` source directory for manifest operations.
///
/// Tries in order:
/// 1. `HKASK_DEPLOY_DIR` env var (for installed or custom paths)
/// 2. `deploy/k8s/` relative to current working directory (dev / repo root)
///
/// Used by both `pod::export_k8s` and `curator::copy_conduit_manifests`.
/// Single source of truth for deploy directory resolution.
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
    Err("Cannot find deploy/k8s/. Set HKASK_DEPLOY_DIR or run from repo root.".into())
}
