//! MCP server scaffolding — shared helpers for hKask MCP server binaries.
//!
//! Each server uses `rmcp`'s `#[tool]` + `#[tool_router(server_handler)]` macros
//! for the wire protocol. This module adds:
//!
//! - `McpToolError` — structured errors with `McpErrorKind` classification
//! - `McpToolOutput` — structured output with optional metadata
//! - `CredentialRequirement` — declarative credential needs (bridges to keystore)
//! - `run_stdio_server()` — common main() boilerplate (tracing, credential check, rmcp serve)
//! - `classify_http_error()` — HTTP status → McpToolError mapping (shared across all API servers)
//! - `api_get()` / `api_post()` — shared HTTP helpers with automatic error classification
//! - `resolve_credential()` — credential resolution via hkask-keystore with env var fallback
//! - `emit_tool_span()` — CNS tool span emission for observability
//!
//! ## Usage
//!
//! ```rust,ignore
//! use hkask_mcp::server::{run_stdio_server, CredentialRequirement};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     run_stdio_server(
//!         "hkask-mcp-github",
//!         env!("CARGO_PKG_VERSION"),
//!         || Ok(GithubServer::new()),
//!         vec![CredentialRequirement::required("HKASK_GITHUB_TOKEN", "GitHub PAT")],
//!     ).await
//! }
//! ```

use hkask_types::McpErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

// =============================================================================
// CredentialRequirement
// =============================================================================

/// A credential that an MCP server requires to function.
///
/// Servers declare these; the runtime resolves them from `hkask-keystore`
/// and injects the values into the server process environment at launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRequirement {
    /// Environment variable name the server expects (e.g., `"HKASK_GITHUB_TOKEN"`).
    pub env_var: String,
    /// Human-readable description of what this credential is for.
    pub description: String,
    /// Whether the server cannot function without this credential.
    /// Optional credentials allow degraded operation.
    pub required: bool,
}

impl CredentialRequirement {
    /// Declare a required credential.
    pub fn required(env_var: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
            description: description.into(),
            required: true,
        }
    }

    /// Declare an optional credential (allows degraded operation).
    pub fn optional(env_var: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
            description: description.into(),
            required: false,
        }
    }
}

// =============================================================================
// McpToolOutput
// =============================================================================

/// Successful result from a tool dispatch, with optional observability metadata.
///
/// Tool methods return `McpToolOutput::to_json_string()` which rmcp wraps
/// in the MCP content envelope. Metadata is optional structured context
/// for CNS observability (latency, model used, page count, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolOutput {
    /// The tool's result content (typically a JSON-serialized value).
    pub content: Value,
    /// Optional structured metadata for observability or downstream processing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl McpToolOutput {
    /// Create output with JSON content and no metadata.
    pub fn new(content: Value) -> Self {
        Self {
            content,
            metadata: None,
        }
    }

    /// Create output with metadata attached.
    pub fn with_metadata(content: Value, metadata: Value) -> Self {
        Self {
            content,
            metadata: Some(metadata),
        }
    }

    /// Create output with timing metadata (duration in milliseconds).
    pub fn with_timing(content: Value, start: Instant) -> Self {
        Self::with_metadata(
            content,
            serde_json::json!({
                "duration_ms": start.elapsed().as_millis() as u64,
            }),
        )
    }

    /// Serialize to JSON string for rmcp tool return value.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            serde_json::json!({
                "content": format!("serialization error: {e}"),
            })
            .to_string()
        })
    }
}

// =============================================================================
// McpToolError
// =============================================================================

/// Structured error from a tool dispatch, carrying semantic classification.
///
/// The `kind` field allows the dispatch layer to reason about failures without
/// parsing error message strings. This enables:
/// - Retry logic (retry on `Timeout`/`Unavailable`, don't on `InvalidArgument`)
/// - User-facing error categorization
/// - CNS observability bucketing by error class
/// - OCAP policy decisions
///
/// Tool methods return `McpToolError::to_json_string()` which rmcp wraps
/// in the MCP error content envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolError {
    /// Semantic classification from the `McpErrorKind` taxonomy.
    pub kind: McpErrorKind,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details (stack traces, validation failures, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl McpToolError {
    /// Create an error with kind and message.
    pub fn new(kind: McpErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    /// Create an internal error (the most common case).
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Internal, message)
    }

    /// Create a not-found error (unknown tool name, missing resource).
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::NotFound, message)
    }

    /// Create an invalid-argument error (bad tool params, schema violation).
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidArgument, message)
    }

    /// Create an unavailable error (upstream service down, network failure).
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Unavailable, message)
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Timeout, message)
    }

    /// Create a permission-denied error (OCAP capability check failed).
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::PermissionDenied, message)
    }

    /// Create a rate-limited error.
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::RateLimited, message)
    }

    /// Create a failed-precondition error (server not initialized, feature disabled).
    pub fn failed_precondition(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::FailedPrecondition, message)
    }

    /// Create an error with structured details.
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }

    /// Serialize to JSON string for rmcp tool return value.
    ///
    /// Returns a JSON object with `"error"`, `"kind"`, and optional `"details"`.
    /// This format is consumed by the dispatch layer and CNS observability.
    pub fn to_json_string(&self) -> String {
        serde_json::json!({
            "error": self.message,
            "kind": self.kind.to_string(),
        })
        .to_string()
    }
}

impl std::fmt::Display for McpToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for McpToolError {}

// =============================================================================
// classify_http_error — Shared HTTP Status → McpToolError mapping
// =============================================================================

/// Classify an HTTP error response into a structured `McpToolError`.
///
/// Every hKask API server maps HTTP status codes the same way:
/// - 401/403 → `PermissionDenied`
/// - 404 → `NotFound`
/// - 422 → `InvalidArgument`
/// - 429 → `RateLimited`
/// - 502/503 + other 5xx → `Unavailable`
/// - Everything else → `Internal`
///
/// The `service` parameter prefixes the error message (e.g., `"GitHub"`, `"FMP"`).
pub fn classify_http_error(
    service: &str,
    status: reqwest::StatusCode,
    body: &str,
) -> McpToolError {
    let msg = format!("{service} API returned {status}: {}", body.trim());
    match status.as_u16() {
        401 | 403 => McpToolError::permission_denied(msg),
        404 => McpToolError::not_found(msg),
        422 => McpToolError::invalid_argument(msg),
        429 => McpToolError::rate_limited(msg),
        502 | 503 => McpToolError::unavailable(msg),
        _ if status.is_server_error() => McpToolError::unavailable(msg),
        _ => McpToolError::internal(msg),
    }
}

// =============================================================================
// api_get / api_post — Shared HTTP helpers
// =============================================================================

/// Perform an authenticated GET request with automatic error classification.
///
/// On success, parses the response body as JSON. On failure, classifies
/// the HTTP status using `classify_http_error()`.
pub async fn api_get(
    client: &reqwest::Client,
    service: &str,
    url: &str,
) -> Result<Value, McpToolError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| McpToolError::unavailable(format!("{service} request failed: {e}")))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_http_error(service, status, &body));
    }
    serde_json::from_str(&body)
        .map_err(|e| McpToolError::internal(format!("Failed to parse {service} response: {e}")))
}

/// Perform an authenticated POST request with automatic error classification.
///
/// On success, parses the response body as JSON. On failure, classifies
/// the HTTP status using `classify_http_error()`.
pub async fn api_post(
    client: &reqwest::Client,
    service: &str,
    url: &str,
    payload: &Value,
) -> Result<Value, McpToolError> {
    let resp = client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| McpToolError::unavailable(format!("{service} request failed: {e}")))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_http_error(service, status, &body));
    }
    serde_json::from_str(&body)
        .map_err(|e| McpToolError::internal(format!("Failed to parse {service} response: {e}")))
}

// =============================================================================
// resolve_credential — Keystore-first credential resolution
// =============================================================================

/// Resolve a credential value, trying hkask-keystore first, then env vars.
///
/// Resolution order:
/// 1. OS keychain via `hkask_keystore::Keychain` (key = env_var name)
/// 2. Environment variable (traditional `std::env::var`)
///
/// This allows servers to get credentials from either source transparently.
pub fn resolve_credential(env_var: &str) -> Result<String, hkask_keystore::KeystoreError> {
    let keychain = hkask_keystore::Keychain::default();
    match keychain.retrieve_by_key(env_var) {
        Ok(val) => {
            tracing::debug!(
                credential = env_var,
                source = "keychain",
                "Credential resolved from OS keychain"
            );
            Ok(val)
        }
        Err(_) => match std::env::var(env_var) {
            Ok(val) => {
                tracing::debug!(
                    credential = env_var,
                    source = "env",
                    "Credential resolved from environment variable"
                );
                Ok(val)
            }
            Err(_) => Err(hkask_keystore::KeystoreError::NotFound(format!(
                "Credential '{}' not found in keychain or environment",
                env_var
            ))),
        },
    }
}

// =============================================================================
// emit_tool_span — CNS observability for tool invocations
// =============================================================================

/// Emit a CNS tool span for observability.
///
/// Called by tool methods to record invocation metadata. Uses the `tracing`
/// crate with `cns.tool` target so that CNS subscribers can capture it.
/// Also integrates with `hkask_cns::SpanEmitter` when available.
pub fn emit_tool_span(
    tool_name: &str,
    outcome: &str,
    duration_ms: u64,
    error_kind: Option<&McpErrorKind>,
) {
    let mut fields = serde_json::json!({
        "tool": tool_name,
        "outcome": outcome,
        "duration_ms": duration_ms,
    });
    if let Some(kind) = error_kind {
        fields["error_kind"] = serde_json::json!(kind.to_string());
    }
    tracing::info!(
        target: "cns.tool",
        tool = tool_name,
        outcome = outcome,
        duration_ms = duration_ms,
        error_kind = error_kind.map(|k| k.to_string()).as_deref().unwrap_or(""),
        "CNS tool span"
    );
}

// =============================================================================
// run_stdio_server — Common Server Bootstrap
// =============================================================================

/// Common bootstrap for hKask MCP server binaries.
///
/// Handles:
/// 1. Tracing subscriber initialization
/// 2. Credential requirement checks (keystore → env var fallback)
/// 3. Server construction via factory (only after credential checks pass)
/// 4. rmcp stdio serve
///
/// The factory pattern ensures server constructors that need credentials
/// only run AFTER credential availability is confirmed.
///
/// # Arguments
/// - `server_name` — Human-readable server name for logging (e.g., `"hkask-mcp-github"`)
/// - `version` — SemVer version string (use `env!("CARGO_PKG_VERSION")`)
/// - `server_factory` — Closure that constructs the server (called after credential checks)
/// - `credentials` — Declared credential requirements
///
/// # Errors
/// Returns an error if a required credential is missing or server construction fails.
pub async fn run_stdio_server<S, F>(
    server_name: &str,
    version: &str,
    server_factory: F,
    credentials: Vec<CredentialRequirement>,
) -> anyhow::Result<()>
where
    S: rmcp::ServiceExt<rmcp::RoleServer>,
    S: rmcp::Service<rmcp::RoleServer>,
    F: FnOnce() -> anyhow::Result<S>,
{
    // 1. Tracing initialization
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 2. Credential checks (keystore → env var)
    let mut missing_required = Vec::new();
    for cred in &credentials {
        match resolve_credential(&cred.env_var) {
            Ok(_) => {
                tracing::debug!(
                    credential = %cred.env_var,
                    "Credential resolved"
                );
            }
            Err(_) if cred.required => {
                tracing::error!(
                    credential = %cred.env_var,
                    description = %cred.description,
                    "Required credential not set — server cannot function"
                );
                missing_required.push(cred.env_var.clone());
            }
            Err(_) => {
                tracing::warn!(
                    credential = %cred.env_var,
                    description = %cred.description,
                    "Optional credential not set — server will operate with degraded functionality"
                );
            }
        }
    }

    if !missing_required.is_empty() {
        anyhow::bail!(
            "Missing required credentials: {}. Set them via environment variables or hkask-keystore.",
            missing_required.join(", ")
        );
    }

    // 3. Construct server (only after credential checks pass)
    let server = server_factory()?;

    // 4. Serve via rmcp stdio transport
    tracing::info!(
        server = server_name,
        version = version,
        "MCP server starting"
    );
    let service = server.serve(rmcp::transport::stdio());
    service.await?;
    Ok(())
}
