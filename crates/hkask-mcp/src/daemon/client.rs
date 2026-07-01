// ── DaemonClient (used by MCP binaries) ──

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::daemon_socket_path;
use super::protocol::{DaemonRequest, DaemonResponse};

/// Client for connecting to the hKask daemon over a Unix domain socket.
///
/// Used by MCP server binaries to authenticate, verify role assignments,
/// and check capability tokens before starting the MCP server.
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a client that connects to the default daemon socket path.
    ///
    /// post: returns DaemonClient with default socket path
    #[must_use]
    pub fn new() -> Self {
        Self {
            socket_path: daemon_socket_path(),
        }
    }

    /// Create a client with a custom socket path (for testing).
    ///
    /// pre:  path is a valid filesystem path
    /// post: returns DaemonClient with custom socket path
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self { socket_path: path }
    }

    /// Send a request and receive a response.
    async fn send_recv(&self, request: &DaemonRequest) -> std::io::Result<DaemonResponse> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (reader, mut writer) = stream.into_split();

        let mut json = serde_json::to_string(request)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.shutdown().await?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;

        serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Query whether a replicant is authenticated.
    #[must_use = "result must be used"]
    pub async fn auth_query(&self, replicant: &str) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::AuthQuery {
            replicant: replicant.to_string(),
        })
        .await
    }

    /// Query whether a replicant is assigned to a specific MCP role.
    #[must_use = "result must be used"]
    pub async fn assignment_query(
        &self,
        replicant: &str,
        role: &str,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::AssignmentQuery {
            replicant: replicant.to_string(),
            role: role.to_string(),
        })
        .await
    }

    /// Query whether a replicant holds a capability token for a tool.
    #[must_use = "result must be used"]
    pub async fn capability_query(
        &self,
        replicant: &str,
        tool: &str,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::CapabilityQuery {
            replicant: replicant.to_string(),
            tool: tool.to_string(),
        })
        .await
    }

    /// Store an experience in both episodic and semantic memory.
    #[must_use = "result must be used"]
    pub async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::StoreExperience {
            replicant: replicant.to_string(),
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value: value.clone(),
            confidence,
        })
        .await
    }

    /// Dispatch a tool call through the daemon to an MCP server.
    #[must_use = "result must be used"]
    pub async fn tool_dispatch(
        &self,
        replicant: &str,
        tool: &str,
        input: &serde_json::Value,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::ToolDispatch {
            replicant: replicant.to_string(),
            tool: tool.to_string(),
            input: input.clone(),
        })
        .await
    }

    /// Query curator system health from the daemon.
    #[must_use = "result must be used"]
    pub async fn curator_health_query(&self, replicant: &str) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::CuratorHealthQuery {
            replicant: replicant.to_string(),
        })
        .await
    }

    /// Query live CNS status from the daemon.
    #[must_use = "result must be used"]
    pub async fn cns_status_query(
        &self,
        replicant: &str,
        domain: Option<&str>,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::CnsStatusQuery {
            replicant: replicant.to_string(),
            domain: domain.map(|d| d.to_string()),
        })
        .await
    }

    /// Query spec drift from the daemon.
    #[must_use = "result must be used"]
    pub async fn spec_drift_query(
        &self,
        replicant: &str,
        spec_id: Option<&str>,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::SpecDriftQuery {
            replicant: replicant.to_string(),
            spec_id: spec_id.map(|s| s.to_string()),
        })
        .await
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
