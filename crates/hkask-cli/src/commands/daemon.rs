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
use crate::error::CliError;
use hkask_mcp::daemon::{DaemonHandler, DaemonListener, daemon_socket_path};
use std::sync::Arc;

/// Error type for daemon socket probing.
///
/// Used by `ping_daemon` to distinguish failure modes when checking whether
/// the daemon is live. Each variant maps to a specific failure in the
/// connect → write → read → parse chain.
#[derive(Debug, thiserror::Error)]
enum DaemonPingError {
    #[error("connect failed: {0}")]
    Connect(#[source] std::io::Error),
    #[error("serialize request: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("write failed: {0}")]
    Write(#[source] std::io::Error),
    #[error("shutdown failed: {0}")]
    Shutdown(#[source] std::io::Error),
    #[error("read failed: {0}")]
    Read(#[source] std::io::Error),
    #[error("empty response")]
    EmptyResponse,
    #[error("invalid JSON response: {0}")]
    InvalidJson(#[source] serde_json::Error),
}

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
            match rt.block_on(ping_daemon(&path)) {
                Ok(()) => {
                    println!("Daemon is running (socket: {})", path.display());
                }
                Err(reason) => {
                    if path.exists() {
                        println!(
                            "Daemon is not running (stale socket at {} — {})",
                            path.display(),
                            reason
                        );
                        println!(
                            "  Run `kask daemon stop` to remove the stale socket, then `kask daemon start`."
                        );
                    } else {
                        println!("Daemon is not running (no socket at {})", path.display());
                    }
                }
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

/// Probe the daemon socket by sending an `auth_query` for a sentinel replicant.
///
/// Returns Ok(()) if the socket accepts a connection and responds with valid
/// JSON; Err(DaemonPingError) otherwise. This distinguishes a live daemon from
/// a stale socket file left behind by a crashed process.
///
/// pre:  path is the daemon socket path
/// post: returns Ok(()) when the daemon is live, Err with a classified reason when not
async fn ping_daemon(path: &std::path::Path) -> Result<(), DaemonPingError> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let stream = UnixStream::connect(path)
        .await
        .map_err(DaemonPingError::Connect)?;
    let (reader, mut writer) = stream.into_split();

    // Send a minimal auth_query. The sentinel name "__ping__" is not a real
    // replicant; the daemon will return authenticated:false, but any valid
    // JSON response proves the socket is live.
    let request = serde_json::json!({
        "type": "auth_query",
        "replicant": "__ping__"
    });
    let mut json = serde_json::to_string(&request).map_err(DaemonPingError::Serialize)?;
    json.push('\n');
    writer
        .write_all(json.as_bytes())
        .await
        .map_err(DaemonPingError::Write)?;
    writer.shutdown().await.map_err(DaemonPingError::Shutdown)?;

    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    buf_reader
        .read_line(&mut line)
        .await
        .map_err(DaemonPingError::Read)?;

    if line.trim().is_empty() {
        return Err(DaemonPingError::EmptyResponse);
    }
    serde_json::from_str::<serde_json::Value>(&line).map_err(DaemonPingError::InvalidJson)?;
    Ok(())
}

async fn run_daemon() -> Result<(), CliError> {
    // Build config — prefer env, fall back to in-memory (no wallet needed)
    let config = hkask_services_core::ServiceConfig::from_env()
        .unwrap_or_else(|_| hkask_services_core::ServiceConfig::in_memory());

    // Try build — if wallet fails, retry with in-memory config
    let ctx = match hkask_services_context::AgentService::build(config).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(target: "hkask.daemon", error = %e, "Build with env config failed, retrying in-memory");
            hkask_services_context::AgentService::build(
                hkask_services_core::ServiceConfig::in_memory(),
            )
            .await
            .map_err(|e| {
                CliError::Daemon(format!(
                    "Failed to build service context (in-memory fallback): {e}"
                ))
            })?
        }
    };

    // Start the loop system
    ctx.cns().loops.start().await;

    let loop_ids = ctx.cns().loops.registered_loop_ids().await;
    tracing::info!(target: "hkask.daemon", loops_num = loop_ids.len(), "Loop system started");

    // Bind daemon socket
    let mut listener = DaemonListener::new();
    listener
        .bind()
        .await
        .map_err(|e| CliError::Daemon(format!("Failed to bind daemon socket: {e}")))?;

    let handler_raw = Arc::clone(&ctx.infra().daemon);
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

    ctx.cns().loops.shutdown();
    println!("Daemon shut down.");
    Ok(())
}
