//! MCP server scaffolding — shared helpers for hKask MCP server binaries.
//!
//! Each server uses `rmcp`'s `#[tool]` + `#[tool_router(server_handler)]` macros
//! for the wire protocol. This module provides:
//!
//! - `CredentialRequirement` — declarative credential needs (bridges to keystore)
//! - `ServerContext` — resolved credentials, WebID identity, shared infrastructure (no ambient authority)
//! - `McpToolError` — structured errors with `McpErrorKind` classification
//! - `McpToolOutput` — structured output with optional metadata
//! - `run_stdio_server()` — common main() bootstrap (tracing, credential check, WebID, rmcp serve)
//! - `classify_http_error()` — HTTP status → McpToolError mapping
//! - `api_get()` / `api_post()` — shared HTTP helpers with automatic error classification
//! - `resolve_credential()` — credential resolution via hkask-keystore with env var fallback
//!
//! ## WebID Resolution
//!
//! `run_stdio_server` resolves the calling agent's identity from environment
//! variables (no ambient authority inside tool handlers):
//!
//! 1. `HKASK_WEBID` — direct UUID string (highest precedence)
//! 2. `HKASK_AGENT_PERSONA` — deterministic derivation via `WebID::from_persona`
//! 3. Anonymous — `WebID::new()` (random UUID, for unauthenticated callers)
//!
//! The resolved `WebID` is available as `ctx.webid` in the factory closure
//! OCAP gating, and CNS span attribution.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use hkask_mcp::server::{run_stdio_server, CredentialRequirement, ServerContext};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     run_stdio_server(
//!         "hkask-mcp-github",
//!         env!("CARGO_PKG_VERSION"),
//!         |ctx: ServerContext| {
//!             let token = ctx.credentials.get("HKASK_GITHUB_TOKEN")
//!                 .expect("credential checked by run_stdio_server")
//!                 .clone();
//!             Ok(GithubServer::new(token))
//!         },
//!         vec![CredentialRequirement::required("HKASK_GITHUB_TOKEN", "GitHub PAT")],
//!     ).await
//! }
//! ```
//!
//! The factory closure receives a `ServerContext` containing resolved credentials,
//! an adapter container, and the calling agent's `WebID` —
//! no ambient authority via `std::env::var`. All configuration and identity flows
//! through the context.

use hkask_types::McpErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

/// A credential that an MCP server requires to function.
///
/// Servers declare these; the runtime resolves them from `hkask-keystore`
/// and passes them into the `ServerContext` at server construction time.
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
// ServerContext — Resolved dependencies for server construction
// =============================================================================

/// Context provided to the server factory by `run_stdio_server`.
///
/// Contains all resolved dependencies. Servers receive this context
/// rather than reading `std::env::var` or global singletons —
/// no ambient authority. Credentials are resolved once at bootstrap
/// via `resolve_credential` (keystore → env var) and injected here.
///
/// The `webid` field carries the calling agent's identity, derived
/// from `HKASK_WEBID` (direct UUID) or `HKASK_AGENT_PERSONA` (deterministic
/// derivation via `WebID::from_persona`). If neither is set, an anonymous
/// WebID is generated. This enables energy budget enforcement, OCAP gating, and CNS
/// attribution without ambient authority.
pub struct ServerContext {
    /// Resolved credential values, keyed by env var name.
    /// Only credentials declared in the `CredentialRequirement` list are present.
    /// Required credentials are guaranteed to be present; optional ones
    /// may be absent (check with ctx.credentials.get("KEY")).
    pub credentials: HashMap<String, String>,

    /// Adapter container for shared adapters (GitCAS, etc.).
    pub adapters: crate::AdapterContainer,

    /// Identity of the calling agent.
    ///
    /// Resolved from (in order of precedence):
    /// 1. `HKASK_WEBID` — direct UUID string
    /// 2. `HKASK_AGENT_PERSONA` — deterministic derivation via `WebID::from_persona`
    /// 3. Anonymous — `WebID::new()` (random UUID)
    ///
    /// Use this for energy budget enforcement, OCAP gating, and CNS span attribution.
    pub webid: hkask_types::WebID,
}

impl ServerContext {
    /// Open a database from credentials resolved in this context.
    ///
    /// Looks up `db_env_var` and `HKASK_DB_PASSPHRASE` from resolved credentials.
    /// If `db_env_var` is absent, falls back to an in-memory database.
    ///
    /// # Errors
    ///
    /// Returns an error if `HKASK_DB_PASSPHRASE` is absent when `db_env_var` is present,
    /// or if the database fails to open.
    pub fn open_database(&self, db_env_var: &str) -> anyhow::Result<hkask_storage::Database> {
        use hkask_storage::open_database;
        match self.credentials.get(db_env_var) {
            Some(path) => {
                let passphrase = self.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                    anyhow::anyhow!("{} set but HKASK_DB_PASSPHRASE missing", db_env_var)
                })?;
                Ok(open_database(path, passphrase)?)
            }
            None => Ok(hkask_storage::Database::in_memory()?),
        }
    }
}

// =============================================================================
// ToolSpanGuard — RAII guard for automatic CNS tool span emission
// =============================================================================

/// RAII guard that automatically emits a CNS tool span when dropped.
///
/// This eliminates the need for manual `emit_tool_span` / `emit_tool_span_with_caller`
/// calls at every exit point in a tool handler. Instead, create the guard at the
/// start of a tool method, and it will emit the span when the guard is consumed.
///
/// # Usage
///
/// ```rust,ignore
/// async fn my_tool(&self, ...) -> String {
///     let span = ToolSpanGuard::new("my:tool", &self.webid);
///     // ... tool logic ...
///     span.ok(McpToolOutput::new(json!({...})).to_json_string())
/// }
/// ```
///
/// For error paths, use `span.error(kind, output)` which sets the error kind
/// and returns the output string. If the guard is dropped without calling `ok()`
/// or `error()`, it emits a span with outcome `"dropped"` (indicating a bug).
pub struct ToolSpanGuard {
    tool_name: String,
    start: Instant,
    caller: hkask_types::WebID,
    emitted: bool,
}

impl ToolSpanGuard {
    /// Create a new tool span guard with timing starting now.
    ///
    /// The `caller` parameter is the calling agent's WebID, used for
    /// CNS attribution in the emitted span.
    pub fn new(tool_name: &str, caller: &hkask_types::WebID) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            start: Instant::now(),
            caller: *caller,
            emitted: false,
        }
    }

    /// Mark the tool invocation as successful and return the output string.
    ///
    /// Emits a CNS span with outcome `"ok"` and no error kind.
    pub fn ok(mut self, output: String) -> String {
        self.emitted = true;
        let duration_ms = self.start.elapsed().as_millis() as u64;
        emit_tool_span_with_caller(&self.tool_name, "ok", duration_ms, None, Some(&self.caller));
        output
    }

    /// Mark the tool invocation as an error and return the output string.
    ///
    /// Emits a CNS span with outcome `"error"` and the given error kind.
    pub fn error(mut self, kind: McpErrorKind, output: String) -> String {
        self.emitted = true;
        let duration_ms = self.start.elapsed().as_millis() as u64;
        emit_tool_span_with_caller(
            &self.tool_name,
            "error",
            duration_ms,
            Some(&kind),
            Some(&self.caller),
        );
        output
    }

    /// Return a success response with a JSON value.
    ///
    /// Equivalent to `self.ok(McpToolOutput::new(value).to_json_string())`.
    pub fn ok_json(self, value: Value) -> String {
        self.ok(McpToolOutput::new(value).to_json_string())
    }

    /// Return an internal error response with a JSON value.
    ///
    /// Equivalent to `self.error(McpErrorKind::Internal, McpToolOutput::new(value).to_json_string())`.
    pub fn internal_error(self, value: Value) -> String {
        self.error(
            McpErrorKind::Internal,
            McpToolOutput::new(value).to_json_string(),
        )
    }
}

impl Drop for ToolSpanGuard {
    fn drop(&mut self) {
        if !self.emitted {
            // Guard dropped without calling ok() or error() — emit a warning span
            let duration_ms = self.start.elapsed().as_millis() as u64;
            emit_tool_span_with_caller(
                &self.tool_name,
                "dropped",
                duration_ms,
                None,
                Some(&self.caller),
            );
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
    ///
    /// External API boundary rate limiter — protects MCP servers from external
    /// client DoS, distinct from internal energy budget tracking.
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::RateLimited, message)
    }

    /// Create a failed-precondition error (server not initialized, feature disabled).
    pub fn failed_precondition(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::FailedPrecondition, message)
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
// Input validation — Shared sanitization for MCP tool parameters
// =============================================================================

/// Validate a string identifier (owner, repo, symbol, etc.).
///
/// Rejects empty strings, strings longer than `max_len`, and strings
/// containing characters outside the allowed set `[a-zA-Z0-9_.-]`.
/// This prevents injection in URL paths and query parameters.
pub fn validate_identifier(name: &str, value: &str, max_len: usize) -> Result<(), McpToolError> {
    if value.is_empty() {
        return Err(McpToolError::invalid_argument(format!(
            "{name} must not be empty"
        )));
    }
    if value.len() > max_len {
        return Err(McpToolError::invalid_argument(format!(
            "{name} exceeds maximum length of {max_len} (got {})",
            value.len()
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return Err(McpToolError::invalid_argument(format!(
            "{name} contains invalid characters (allowed: alphanumeric, _, ., -)"
        )));
    }
    Ok(())
}

/// Validate a URL parameter against SSRF protection rules.
///
/// Delegates to `hkask_mcp::validate_url()` with the default (strict) config.
/// Use this for any tool that accepts a user-provided URL.
pub fn validate_tool_url(url: &str) -> Result<(), McpToolError> {
    crate::security::validate_url(url, &crate::security::UrlValidationConfig::default())
        .map_err(|e| McpToolError::invalid_argument(format!("URL validation failed: {e}")))
}

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
pub fn classify_http_error(service: &str, status: reqwest::StatusCode, body: &str) -> McpToolError {
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

/// Perform an authenticated PUT request with automatic error classification.
///
/// On success, parses the response body as JSON. On failure, classifies
/// the HTTP status using `classify_http_error()`.
pub async fn api_put(
    client: &reqwest::Client,
    service: &str,
    url: &str,
    payload: &Value,
) -> Result<Value, McpToolError> {
    let resp = client
        .put(url)
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
/// Emit a CNS tool span with caller identity (WebID) for observability.
///
/// Like `emit_tool_span`, but includes the calling agent's identity in the
/// span. Use this in servers that have access to `self.webid` for full
/// CNS attribution — it records *who* called the tool, not just *what* happened.
///
/// For servers that don't yet store a `webid`, `emit_tool_span` still works
/// and omits the caller field.
pub fn emit_tool_span_with_caller(
    tool_name: &str,
    outcome: &str,
    duration_ms: u64,
    error_kind: Option<&McpErrorKind>,
    caller: Option<&hkask_types::WebID>,
) {
    let mut fields = serde_json::json!({
        "tool": tool_name,
        "outcome": outcome,
        "duration_ms": duration_ms,
    });
    if let Some(kind) = error_kind {
        fields["error_kind"] = serde_json::json!(kind.to_string());
    }
    if let Some(webid) = caller {
        fields["caller"] = serde_json::json!(webid.to_string());
    }
    tracing::info!(
        target: "cns.tool",
        tool = tool_name,
        outcome = outcome,
        duration_ms = duration_ms,
        error_kind = error_kind.map(|k| k.to_string()).as_deref().unwrap_or(""),
        caller = caller.map(|w| w.to_string()).as_deref().unwrap_or(""),
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
/// 2. Credential requirement checks (keystore → env var)
/// 3. WebID resolution (HKASK_WEBID → HKASK_AGENT_PERSONA → anonymous)
/// 4. Server construction via factory (only after credential checks pass)
/// 5. rmcp stdio serve
///
/// The factory pattern ensures server constructors that need credentials
/// only run AFTER credential availability is confirmed. The factory receives
/// a `ServerContext` containing resolved credentials and shared infrastructure
/// — no ambient authority via `std::env::var`.
///
/// # Arguments
/// - `server_name` — Human-readable server name for logging (e.g., `"hkask-mcp-github"`)
/// - `version` — SemVer version string (use `env!("CARGO_PKG_VERSION")`)
/// - `server_factory` — Closure that constructs the server, receiving a `ServerContext`
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
    F: FnOnce(ServerContext) -> anyhow::Result<S>,
{
    // 1. Tracing initialization
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 2. Credential checks (keystore → env var)
    let mut resolved = HashMap::new();
    let mut missing_required = Vec::new();

    for cred in &credentials {
        match resolve_credential(&cred.env_var) {
            Ok(val) => {
                tracing::debug!(credential = %cred.env_var, "Credential resolved");
                resolved.insert(cred.env_var.clone(), val);
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

    // 3. Resolve calling agent identity (WebID)
    let webid = if let Ok(uuid_str) = std::env::var("HKASK_WEBID") {
        // Direct UUID — highest precedence
        hkask_types::WebID::from_string(&uuid_str)
    } else if let Ok(persona) = std::env::var("HKASK_AGENT_PERSONA") {
        // Deterministic derivation from persona name
        hkask_types::WebID::from_persona(persona.as_bytes())
    } else {
        // Anonymous caller — random UUID
        hkask_types::WebID::new()
    };

    tracing::info!(
        webid = %webid.redacted_display(),
        "Agent identity resolved"
    );

    // 4. Build server context (no ambient authority)
    let ctx = ServerContext {
        credentials: resolved,
        adapters: crate::AdapterContainer::new(),
        webid,
    };

    // 5. Construct server (only after credential checks pass)
    let server = server_factory(ctx)?;

    // 6. Serve via rmcp stdio transport
    tracing::info!(
        server = server_name,
        version = version,
        "MCP server starting"
    );
    let service = server.serve(rmcp::transport::stdio());
    service.await?;
    Ok(())
}
