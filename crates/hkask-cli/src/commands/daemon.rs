//! `kask daemon` — Start the hKask daemon process
//!
//! Binds a Unix domain socket at `~/.config/hkask/daemon.sock`, starts the
//! CNS runtime with all 6 loops, enables contract test monitoring, and serves
//! authentication/capability queries from MCP server binaries.
//!
//! The daemon is the persistent runtime that ACP clients, MCP servers, and
//! the `kask test` CLI surface all depend on. Without it, CNS background
//! monitoring dies when the CLI command exits.

use crate::cli::DaemonAction;
use hkask_mcp::daemon::{DaemonHandler, DaemonListener, daemon_socket_path};
use std::sync::Arc;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is a valid DaemonAction variant
/// post: starts, checks status, or stops the daemon; prints result to stdout
pub fn run(rt: &tokio::runtime::Runtime, action: DaemonAction) {
    match action {
        DaemonAction::Start => {
            if let Err(e) = rt.block_on(run_daemon()) {
                eprintln!("Daemon failed to start: {}", e);
                std::process::exit(1);
            }
        }
        DaemonAction::Status => {
            let path = daemon_socket_path();
            if path.exists() {
                println!("Daemon is running (socket: {})", path.display());
            } else {
                println!("Daemon is not running (no socket at {})", path.display());
            }
        }
        DaemonAction::Stop => {
            let path = daemon_socket_path();
            if path.exists() {
                std::fs::remove_file(&path).ok();
                println!("Daemon socket removed.");
            } else {
                println!("Daemon is not running (no socket at {})", path.display());
            }
        }
    }
}

async fn run_daemon() -> Result<(), String> {
    // Build config — prefer env, fall back to in-memory (no wallet needed)
    let config = hkask_services_core::ServiceConfig::from_env()
        .unwrap_or_else(|_| hkask_services_core::ServiceConfig::in_memory());

    // Try build — if wallet fails, retry with in-memory config
    let ctx = match hkask_services_context::AgentService::build(config).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(target: "hkask.daemon", error = %e, "Build with env config failed, retrying in-memory");
            hkask_services_context::AgentService::build(hkask_services_core::ServiceConfig::in_memory())
                .await
                .map_err(|e| format!("Failed to build service context (in-memory fallback): {e}"))?
        }
    };

    // Start the loop system
    ctx.loop_system()
        .start()
        .await
        .map_err(|e| format!("Failed to start loop system: {e}"))?;

    let loop_ids = ctx.loop_system().registered_loop_ids().await;
    tracing::info!(target: "hkask.daemon", loops_num = loop_ids.len(), "Loop system started");

    // Bind daemon socket
    let mut listener = DaemonListener::new();
    listener
        .bind()
        .await
        .map_err(|e| format!("Failed to bind daemon socket: {e}"))?;

    let handler_raw = Arc::clone(ctx.daemon_handler());
    let handler: Arc<dyn DaemonHandler> = handler_raw;

    let interval = std::env::var("HKASK_CONTRACT_TEST_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);

    println!(
        "hKask daemon started — socket: {}, {} loops active",
        daemon_socket_path().display(),
        loop_ids.len()
    );
    println!("CNS contract monitoring active (interval: {}s)", interval);
    println!("Press Ctrl+C to stop.");

    tokio::select! {
        result = listener.serve(handler) => {
            if let Err(e) = result {
                eprintln!("Daemon serve error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down daemon...");
        }
    }

    ctx.loop_system().shutdown();
    println!("Daemon shut down.");
    Ok(())
}
