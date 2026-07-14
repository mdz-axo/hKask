//! Daemon socket — Unix domain socket transport for MCP binary ↔ hKask communication.
//!
//! MCP server binaries connect to the hKask daemon over a Unix domain socket at
//! `~/.config/hkask/daemon.sock` to authenticate, verify role assignments, and
//! check capability tokens. The protocol is newline-delimited JSON.
//!
//! # Protocol
//!
//! Request (MCP binary → daemon):
//! ```json
//! {"type":"auth_query","replicant":"bob"}
//! {"type":"assignment_query","replicant":"bob","role":"research"}
//! {"type":"capability_query","replicant":"bob","tool":"web_search"}
//! ```rust,no_run
//!
//! Response (daemon → MCP binary):
//! ```json
//! {"type":"auth_response","authenticated":true,"webid":"bob-xxxx"}
//! {"type":"auth_response","authenticated":false,"action":"prompt_user"}
//! {"type":"assignment_response","assigned":true}
//! {"type":"capability_response","granted":true}
//! ```

use std::path::PathBuf;

pub mod client;
pub mod handler;
pub mod listener;
pub mod protocol;

#[cfg(test)]
mod tests;

pub use client::DaemonClient;
pub use handler::DaemonHandler;
pub use listener::DaemonListener;
pub use protocol::{DaemonRequest, DaemonResponse};

/// Well-known path for the hKask daemon socket.
///
/// post: returns PathBuf to the daemon socket (config dir or /tmp fallback)
#[must_use]
pub fn daemon_socket_path() -> PathBuf {
    let base = dirs_next().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("daemon.sock")
}

fn dirs_next() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("hkask"))
}
