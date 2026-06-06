//! MCP server scaffolding — shared helpers for hKask MCP server binaries.
//
//! WebID resolution order: `HKASK_WEBID` → `HKASK_AGENT_PERSONA` → anonymous.
//! No ambient authority — all identity and credentials flow through `ServerContext`.
//
//! ```rust,ignore
//! use hkask_mcp::server::{run_stdio_server, CredentialRequirement, ServerContext};
//
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

use hkask_types::McpErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
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

/// Server construction context. No ambient authority — all deps injected here.
pub struct ServerContext {
    pub credentials: HashMap<String, String>,

    pub adapters: crate::AdapterContainer,

    /// Resolved from HKASK_WEBID → HKASK_AGENT_PERSONA → anonymous.
    pub webid: hkask_types::WebID,
}

impl ServerContext {
    /// Looks up `db_env_var` and `HKASK_DB_PASSPHRASE`. Falls back to in-memory DB.
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

    /// Like `open_database`, but passes DDL for custom tables (e.g. FTS5).
    pub fn open_database_with_extensions(
        &self,
        db_env_var: &str,
        extensions: &str,
    ) -> anyhow::Result<hkask_storage::Database> {
        match self.credentials.get(db_env_var) {
            Some(path) => {
                let passphrase = self.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                    anyhow::anyhow!("{} set but HKASK_DB_PASSPHRASE missing", db_env_var)
                })?;
                Ok(hkask_storage::Database::open_with_extensions(
                    path, passphrase, extensions,
                )?)
            }
            None => Ok(hkask_storage::Database::in_memory_with_extensions(
                extensions,
            )?),
        }
    }
}

/// RAII guard — emits CNS tool span on drop. Use `span.ok(output)` or `span.error(kind, output)`.
pub struct ToolSpanGuard {
    tool_name: String,
    start: Instant,
    caller: hkask_types::WebID,
    emitted: bool,
}

impl ToolSpanGuard {
    pub fn new(tool_name: &str, caller: &hkask_types::WebID) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            start: Instant::now(),
            caller: *caller,
            emitted: false,
        }
    }

    pub fn ok(mut self, output: String) -> String {
        self.emitted = true;
        let duration_ms = self.start.elapsed().as_millis() as u64;
        emit_tool_span_with_caller(&self.tool_name, "ok", duration_ms, None, Some(&self.caller));
        output
    }

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

    /// Equivalent to `self.ok(McpToolOutput::new(value).to_json_string())`.
    pub fn ok_json(self, value: Value) -> String {
        self.ok(McpToolOutput::new(value).to_json_string())
    }

    /// Produces McpToolError wire format so clients can distinguish errors from successes.
    pub fn internal_error(self, value: Value) -> String {
        let message = match value {
            Value::String(s) => s,
            other => other.to_string(),
        };
        self.error(
            McpErrorKind::Internal,
            McpToolError::internal(message).to_json_string(),
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

/// Tool result with optional observability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolOutput {
    pub content: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl McpToolOutput {
    pub fn new(content: Value) -> Self {
        Self {
            content,
            metadata: None,
        }
    }

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

// McpToolError

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

// Input validation — Shared sanitization for MCP tool parameters

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

// classify_http_error — Shared HTTP Status → McpToolError mapping

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

// api_get / api_post — Shared HTTP helpers

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

// resolve_credential — Keystore-first credential resolution

/// Resolve a credential value, trying hkask-keystore first, then env vars.
///
/// Resolution order:
/// 1. OS keychain via `hkask_keystore::Keychain` (key = env_var name)
/// 2. Environment variable (traditional `std::env::var`)
///
/// This allows servers to get credentials from either source transparently.
/// Parse .env files and return key-value pairs without mutating the process environment.
///
/// Searches for `.env` in the current directory, then the parent directory.
/// Only sets keys that are not already present in the environment (same semantics
/// as the original load_dotenv, but without the unsafe set_var).
///
/// This is the safe alternative to `unsafe std::env::set_var()` — the returned
/// map can be passed to `run_stdio_server_with_preloaded()` to inject values
/// into credential resolution without mutating global state.
pub fn load_dotenv() -> HashMap<String, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let paths = [cwd.join(".env")];
    let parent_paths = cwd
        .parent()
        .map(|p| vec![p.join(".env")])
        .unwrap_or_default();

    for path in paths.iter().chain(parent_paths.iter()) {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut map = HashMap::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                        map.insert(key.to_string(), value.to_string());
                    }
                }
            }
            return map;
        }
    }
    HashMap::new()
}

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

// emit_tool_span — CNS observability for tool invocations

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

// run_stdio_server — Common Server Bootstrap

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
        hkask_types::WebID::from_str(&uuid_str).unwrap_or_else(|_| hkask_types::WebID::new())
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

/// Like `run_stdio_server`, but with pre-resolved credentials from .env files.
///
/// Preloaded credentials take precedence over `resolve_credential()` results.
/// This allows .env file values to be injected without mutating the process
/// environment (no `unsafe set_var`).
pub async fn run_stdio_server_with_preloaded<S, F>(
    server_name: &str,
    version: &str,
    server_factory: F,
    credentials: Vec<CredentialRequirement>,
    preloaded: HashMap<String, String>,
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

    // 2. Credential checks — preloaded values override resolve_credential
    let mut resolved = HashMap::new();
    let mut missing_required = Vec::new();

    for cred in &credentials {
        if let Some(val) = preloaded.get(&cred.env_var) {
            tracing::debug!(credential = %cred.env_var, source = "preloaded", "Credential resolved from preloaded .env");
            resolved.insert(cred.env_var.clone(), val.clone());
            continue;
        }
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
    let webid = if let Some(uuid_str) = preloaded.get("HKASK_WEBID") {
        hkask_types::WebID::from_str(uuid_str).unwrap_or_else(|_| hkask_types::WebID::new())
    } else if let Ok(uuid_str) = std::env::var("HKASK_WEBID") {
        hkask_types::WebID::from_str(&uuid_str).unwrap_or_else(|_| hkask_types::WebID::new())
    } else if let Some(persona) = preloaded.get("HKASK_AGENT_PERSONA") {
        hkask_types::WebID::from_persona(persona.as_bytes())
    } else if let Ok(persona) = std::env::var("HKASK_AGENT_PERSONA") {
        hkask_types::WebID::from_persona(persona.as_bytes())
    } else {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── McpErrorKind ────────────────────────────────────────────────────────

    #[test]
    fn mcp_error_kind_is_retryable() {
        // Only Unavailable, Timeout, and RateLimited are retryable
        assert!(McpErrorKind::Unavailable.is_retryable());
        assert!(McpErrorKind::Timeout.is_retryable());
        assert!(McpErrorKind::RateLimited.is_retryable());
        // All others are not
        assert!(!McpErrorKind::Internal.is_retryable());
        assert!(!McpErrorKind::NotFound.is_retryable());
        assert!(!McpErrorKind::InvalidArgument.is_retryable());
        assert!(!McpErrorKind::PermissionDenied.is_retryable());
        assert!(!McpErrorKind::FailedPrecondition.is_retryable());
    }

    #[test]
    fn mcp_error_kind_requires_intervention() {
        assert!(McpErrorKind::PermissionDenied.requires_intervention());
        assert!(McpErrorKind::FailedPrecondition.requires_intervention());
        // All others do not require intervention
        assert!(!McpErrorKind::Internal.requires_intervention());
        assert!(!McpErrorKind::Unavailable.requires_intervention());
        assert!(!McpErrorKind::Timeout.requires_intervention());
        assert!(!McpErrorKind::NotFound.requires_intervention());
        assert!(!McpErrorKind::InvalidArgument.requires_intervention());
        assert!(!McpErrorKind::RateLimited.requires_intervention());
    }

    #[test]
    fn mcp_error_kind_display_snake_case() {
        assert_eq!(McpErrorKind::Internal.to_string(), "internal");
        assert_eq!(McpErrorKind::Unavailable.to_string(), "unavailable");
        assert_eq!(McpErrorKind::Timeout.to_string(), "timeout");
        assert_eq!(McpErrorKind::NotFound.to_string(), "not_found");
        assert_eq!(
            McpErrorKind::InvalidArgument.to_string(),
            "invalid_argument"
        );
        assert_eq!(
            McpErrorKind::PermissionDenied.to_string(),
            "permission_denied"
        );
        assert_eq!(McpErrorKind::RateLimited.to_string(), "rate_limited");
        assert_eq!(
            McpErrorKind::FailedPrecondition.to_string(),
            "failed_precondition"
        );
    }

    // ── McpToolError ────────────────────────────────────────────────────────

    #[test]
    fn mcp_tool_error_convenience_constructors() {
        let err = McpToolError::internal("boom");
        assert_eq!(err.kind, McpErrorKind::Internal);
        assert_eq!(err.message, "boom");
        assert!(err.details.is_none());

        let err = McpToolError::not_found("gone");
        assert_eq!(err.kind, McpErrorKind::NotFound);

        let err = McpToolError::invalid_argument("bad");
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);

        let err = McpToolError::unavailable("down");
        assert_eq!(err.kind, McpErrorKind::Unavailable);

        let err = McpToolError::timeout("slow");
        assert_eq!(err.kind, McpErrorKind::Timeout);

        let err = McpToolError::permission_denied("nope");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);

        let err = McpToolError::rate_limited("hold");
        assert_eq!(err.kind, McpErrorKind::RateLimited);

        let err = McpToolError::failed_precondition("not ready");
        assert_eq!(err.kind, McpErrorKind::FailedPrecondition);
    }

    #[test]
    fn mcp_tool_error_to_json_string_has_error_and_kind() {
        let err = McpToolError::internal("something broke");
        let json: Value = serde_json::from_str(&err.to_json_string()).unwrap();
        assert_eq!(json["error"], "something broke");
        assert_eq!(json["kind"], "internal");
    }

    #[test]
    fn mcp_tool_error_display_format() {
        let err = McpToolError::not_found("resource missing");
        assert_eq!(format!("{err}"), "[not_found] resource missing");
    }

    #[test]
    fn mcp_tool_error_new_with_kind_and_message() {
        let err = McpToolError::new(McpErrorKind::Timeout, "timed out after 30s");
        assert_eq!(err.kind, McpErrorKind::Timeout);
        assert_eq!(err.message, "timed out after 30s");
        assert!(err.details.is_none());
    }

    // ── McpToolOutput ───────────────────────────────────────────────────────

    #[test]
    fn mcp_tool_output_new_has_no_metadata() {
        let output = McpToolOutput::new(serde_json::json!({"result": 42}));
        assert!(output.metadata.is_none());
        assert_eq!(output.content["result"], 42);
    }

    #[test]
    fn mcp_tool_output_with_metadata() {
        let output = McpToolOutput::with_metadata(
            serde_json::json!("hello"),
            serde_json::json!({"source": "test"}),
        );
        assert_eq!(output.content, serde_json::json!("hello"));
        assert_eq!(output.metadata.as_ref().unwrap()["source"], "test");
    }

    #[test]
    fn mcp_tool_output_to_json_string_roundtrip() {
        let output = McpToolOutput::new(serde_json::json!({"count": 7}));
        let json: Value = serde_json::from_str(&output.to_json_string()).unwrap();
        assert_eq!(json["content"]["count"], 7);
        assert!(json.get("metadata").is_none());
    }

    #[test]
    fn mcp_tool_output_with_metadata_json_roundtrip() {
        let output = McpToolOutput::with_metadata(
            serde_json::json!("ok"),
            serde_json::json!({"latency_ms": 123}),
        );
        let json: Value = serde_json::from_str(&output.to_json_string()).unwrap();
        assert_eq!(json["content"], "ok");
        assert_eq!(json["metadata"]["latency_ms"], 123);
    }

    // ── CredentialRequirement ───────────────────────────────────────────────

    #[test]
    fn credential_requirement_required() {
        let req = CredentialRequirement::required("HKASK_TOKEN", "API token");
        assert_eq!(req.env_var, "HKASK_TOKEN");
        assert_eq!(req.description, "API token");
        assert!(req.required);
    }

    #[test]
    fn credential_requirement_optional() {
        let req = CredentialRequirement::optional("HKASK_DEBUG", "Enable debug logging");
        assert_eq!(req.env_var, "HKASK_DEBUG");
        assert_eq!(req.description, "Enable debug logging");
        assert!(!req.required);
    }

    // ── validate_identifier ─────────────────────────────────────────────────

    #[test]
    fn validate_identifier_accepts_alphanumeric() {
        assert!(validate_identifier("name", "hello123", 256).is_ok());
    }

    #[test]
    fn validate_identifier_accepts_underscore_dot_hyphen() {
        assert!(validate_identifier("id", "my_module.v2-beta", 256).is_ok());
    }

    #[test]
    fn validate_identifier_rejects_empty() {
        let err = validate_identifier("field", "", 256).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn validate_identifier_rejects_too_long() {
        let long = "a".repeat(257);
        let err = validate_identifier("field", &long, 256).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);
        assert!(err.message.contains("exceeds maximum length"));
    }

    #[test]
    fn validate_identifier_accepts_at_max_length() {
        let exact = "a".repeat(256);
        assert!(validate_identifier("field", &exact, 256).is_ok());
    }

    #[test]
    fn validate_identifier_rejects_max_length_plus_one() {
        let over = "a".repeat(257);
        assert!(validate_identifier("field", &over, 256).is_err());
    }

    #[test]
    fn validate_identifier_rejects_spaces() {
        let err = validate_identifier("name", "hello world", 256).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);
        assert!(err.message.contains("invalid characters"));
    }

    #[test]
    fn validate_identifier_rejects_special_chars() {
        assert!(validate_identifier("id", "test@symbol", 256).is_err());
        assert!(validate_identifier("id", "path/sep", 256).is_err());
        assert!(validate_identifier("id", "has:colon", 256).is_err());
    }

    // ── validate_tool_url (public SSRF gateway) ───────────────────────────

    #[test]
    fn validate_tool_url_accepts_https() {
        assert!(validate_tool_url("https://api.example.com/v1").is_ok());
    }

    #[test]
    fn validate_tool_url_rejects_ftp() {
        assert!(validate_tool_url("ftp://files.example.com").is_err());
    }

    #[test]
    fn validate_tool_url_rejects_private_ip() {
        assert!(validate_tool_url("http://10.0.0.1/internal").is_err());
    }

    #[test]
    fn validate_tool_url_rejects_loopback() {
        assert!(validate_tool_url("http://127.0.0.1/health").is_err());
    }

    // ── classify_http_error ─────────────────────────────────────────────────

    #[test]
    fn classify_http_error_permission_denied() {
        let err = classify_http_error("GitHub", reqwest::StatusCode::FORBIDDEN, "access denied");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);
        assert!(err.message.contains("GitHub"));
    }

    #[test]
    fn classify_http_error_not_found() {
        let err = classify_http_error("FMP", reqwest::StatusCode::NOT_FOUND, "missing");
        assert_eq!(err.kind, McpErrorKind::NotFound);
    }

    #[test]
    fn classify_http_error_invalid_argument() {
        let err = classify_http_error(
            "Test",
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "bad input",
        );
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);
    }

    #[test]
    fn classify_http_error_rate_limited() {
        let err = classify_http_error(
            "GitHub",
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "slow down",
        );
        assert_eq!(err.kind, McpErrorKind::RateLimited);
    }

    #[test]
    fn classify_http_error_unavailable_502() {
        let err = classify_http_error("FMP", reqwest::StatusCode::BAD_GATEWAY, "gateway timeout");
        assert_eq!(err.kind, McpErrorKind::Unavailable);
    }

    #[test]
    fn classify_http_error_unavailable_503() {
        let err = classify_http_error("FMP", reqwest::StatusCode::SERVICE_UNAVAILABLE, "down");
        assert_eq!(err.kind, McpErrorKind::Unavailable);
    }

    #[test]
    fn classify_http_error_internal_for_client_errors() {
        // 418 Teapot or other unclassified client errors → Internal
        let err = classify_http_error("Test", reqwest::StatusCode::IM_A_TEAPOT, "tea");
        assert_eq!(err.kind, McpErrorKind::Internal);
    }

    #[test]
    fn classify_http_error_unauthorized_is_permission_denied() {
        let err = classify_http_error("GitHub", reqwest::StatusCode::UNAUTHORIZED, "no auth");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);
    }
}
