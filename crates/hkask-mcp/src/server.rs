//! MCP server scaffolding — shared helpers for hKask MCP server binaries.
//
//! WebID resolution order: `HKASK_WEBID` → `HKASK_AGENT_PERSONA` → anonymous.
//! No ambient authority — all identity and credentials flow through `ServerContext`.
//
//! ```rust,ignore
//! use hkask_mcp::server::{run_stdio_server, CredentialRequirement, ServerContext};
//
//! #[tokio::main]
//! async fn main() -> anyhow::`Result<()>` {
//!     run_stdio_server(
//!         "hkask-mcp-web",
//!         env!("CARGO_PKG_VERSION"),
//!         |ctx: ServerContext| {
//!             Ok(WebServer::new(ctx.webid))
//!         },
//!         vec![],
//!     ).await
//! }
//! ```

use hkask_types::McpErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use thiserror::Error;

/// Unified error type for hkask-mcp library operations.
///
/// Replaces `anyhow::Error` in all public APIs. Every variant carries
/// structured context suitable for CNS spans and operator diagnostics.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("{0} set but HKASK_DB_PASSPHRASE missing")]
    DatabasePassphrase(String),

    #[error(
        "Replicant '{replicant}' is not authenticated. Enter the replicant's passphrase in the hKask terminal."
    )]
    Auth { replicant: String },

    #[error(
        "Replicant '{replicant}' is not assigned to the {role} MCP role. Use 'kask pod assign {replicant} {role}' to grant this role."
    )]
    RoleAssignment { replicant: String, role: String },

    #[error("Unexpected {context} response: {detail}")]
    UnexpectedResponse { context: String, detail: String },

    #[error(
        "Missing required credentials: {missing}. Set them via environment variables or hkask-keystore."
    )]
    MissingCredentials { missing: String },

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

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
    /// Create a required credential declaration.
    ///
    /// pre:  env_var and description are non-empty
    /// post: returns CredentialDecl with required=true
    pub fn required(env_var: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
            description: description.into(),
            required: true,
        }
    }

    /// Declare an optional credential (allows degraded operation).
    /// Create an optional credential declaration.
    ///
    /// pre:  env_var and description are non-empty
    /// post: returns CredentialDecl with required=false
    pub fn optional(env_var: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
            description: description.into(),
            required: false,
        }
    }
}

/// Infrastructure capabilities detected at server startup.
///
/// Computed from environment and credential resolution results — not configured.
/// Servers use this to advertise available tools and report their operating mode.
///
/// Two operating modes emerge from capability detection:
/// - **Embedded** (hKask runtime): WebID is non-anonymous, keystore reachable,
///   persistence available, CNS consumes spans.
/// - **Standalone** (IDE): WebID is anonymous, keystore may be unavailable,
///   persistence unavailable, CNS spans go to stderr.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTier {
    /// Running as part of an hKask installation (vs standalone in an IDE).
    pub embedded: bool,
    /// OS keychain is reachable for secret resolution.
    pub keystore_available: bool,
    /// Persistent storage (database) is configured.
    pub persistence_available: bool,
}

impl CapabilityTier {
    /// Detect capabilities from resolved credentials and environment.
    /// Detect which credentials are available from resolved values.
    ///
    /// pre:  resolved_credentials is a valid map
    /// post: returns CredentialStatus with available/missing counts
    pub fn detect(resolved_credentials: &HashMap<String, String>) -> Self {
        let embedded = resolved_credentials.contains_key("HKASK_WEBID")
            || resolved_credentials.contains_key("HKASK_AGENT_PERSONA");
        let persistence_available = resolved_credentials.contains_key("HKASK_DB_PATH");
        let keystore_available = Self::probe_keystore();
        Self {
            embedded,
            keystore_available,
            persistence_available,
        }
    }

    /// Probe whether the OS keychain is reachable.
    ///
    /// Attempts a lightweight keychain read with a sentinel key.
    /// Returns `true` if the keychain responds (even with "not found"),
    /// `false` only if the platform keychain itself is broken/unavailable.
    fn probe_keystore() -> bool {
        match hkask_keystore::Keychain::default().retrieve_by_key("__hkask_capability_probe__") {
            Ok(_) => true,
            Err(hkask_keystore::KeychainError::NotFound(_)) => true,
            Err(hkask_keystore::KeychainError::Platform(_)) => false,
        }
    }

    /// CNS spans are meaningful only in embedded mode (consumed by hKask CNS).
    /// In standalone mode, spans go to stderr via the tracing subscriber.
    /// Check if CNS is available (all required credentials present).
    ///
    /// post: returns true iff all required credentials are available
    pub fn cns_available(&self) -> bool {
        self.embedded
    }
}

/// Server construction context. No ambient authority — all deps injected here.
pub struct ServerContext {
    pub credentials: HashMap<String, String>,

    pub adapters: crate::AdapterContainer,

    /// Resolved from HKASK_WEBID → HKASK_AGENT_PERSONA → anonymous.
    pub webid: hkask_types::WebID,

    /// Infrastructure capabilities detected at startup.
    pub capability_tier: CapabilityTier,
}

impl ServerContext {
    /// Looks up `db_env_var` and `HKASK_DB_PASSPHRASE`. Falls back to in-memory DB.
    /// Open a database using a credential env var for the passphrase.
    ///
    /// pre:  db_env_var is set and contains a valid passphrase
    /// post: returns opened Database
    pub fn open_database(&self, db_env_var: &str) -> Result<hkask_storage::Database, McpError> {
        use hkask_storage::open_database;
        match self.credentials.get(db_env_var) {
            Some(path) => {
                let passphrase = self
                    .credentials
                    .get("HKASK_DB_PASSPHRASE")
                    .ok_or_else(|| McpError::DatabasePassphrase(db_env_var.to_string()))?;
                Ok(open_database(path, passphrase).map_err(|e| anyhow::anyhow!("{e}"))?)
            }
            None => Ok(hkask_storage::Database::in_memory().map_err(|e| anyhow::anyhow!("{e}"))?),
        }
    }

    /// Like `open_database`, but passes DDL for custom tables (e.g. FTS5).
    /// Open a database with additional DDL extensions.
    ///
    /// pre:  db_env_var is set, extensions is valid SQL DDL
    /// post: returns opened Database with extensions applied
    pub fn open_database_with_extensions(
        &self,
        db_env_var: &str,
        extensions: &str,
    ) -> Result<hkask_storage::Database, McpError> {
        match self.credentials.get(db_env_var) {
            Some(path) => {
                let passphrase = self
                    .credentials
                    .get("HKASK_DB_PASSPHRASE")
                    .ok_or_else(|| McpError::DatabasePassphrase(db_env_var.to_string()))?;
                Ok(
                    hkask_storage::Database::open_with_extensions(path, passphrase, extensions)
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                )
            }
            None => Ok(
                hkask_storage::Database::in_memory_with_extensions(extensions)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
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
    /// Create a new tool span guard.
    ///
    /// pre:  tool_name is non-empty, caller is valid
    /// post: returns ToolSpanGuard with start time recorded
    pub fn new(tool_name: &str, caller: &hkask_types::WebID) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            start: Instant::now(),
            caller: *caller,
            emitted: false,
        }
    }

    /// Mark span as successful and return output.
    ///
    /// post: CNS tool span emitted with "ok" status
    /// post: returns output unchanged
    pub fn ok(mut self, output: String) -> String {
        self.emitted = true;
        let duration_ms = self.start.elapsed().as_millis() as u64;
        emit_tool_span_with_caller(&self.tool_name, "ok", duration_ms, None, Some(&self.caller));
        output
    }

    /// Mark span as error and return output.
    ///
    /// post: CNS tool span emitted with "error" status and error kind
    /// post: returns output unchanged
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
    /// Finish span with Ok JSON value.
    ///
    /// post: CNS tool span emitted with "ok" status
    /// post: returns JSON string of value
    pub fn ok_json(self, value: Value) -> String {
        self.ok(McpToolOutput::new(value).to_json_string())
    }

    /// Consume a `Result<Value, McpToolError>` — ok→`ok_json`, err→`error(…)`.
    /// Finish span with a Result.
    ///
    /// post: CNS tool span emitted with appropriate status
    /// post: returns JSON string of Ok value or error
    pub fn finish(self, result: Result<Value, McpToolError>) -> String {
        match result {
            Ok(value) => self.ok_json(value),
            Err(e) => self.error(e.kind, e.to_json_string()),
        }
    }

    /// Produces McpToolError wire format so clients can distinguish errors from successes.
    /// Finish span with an internal error.
    ///
    /// post: CNS tool span emitted with "error" status
    /// post: returns JSON error string
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
pub(crate) struct McpToolOutput {
    pub(crate) content: Value,
}

impl McpToolOutput {
    pub(crate) fn new(content: Value) -> Self {
        Self { content }
    }

    /// Serialize to JSON string for rmcp tool return value.
    pub(crate) fn to_json_string(&self) -> String {
        serde_json::to_string(&serde_json::json!({"content": &self.content})).unwrap_or_else(|e| {
            serde_json::json!({"content": format!("serialization error: {e}")}).to_string()
        })
    }
}

// McpToolError

/// Structured error from a tool dispatch, carrying semantic classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolError {
    pub kind: McpErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl McpToolError {
    /// Create a new McpToolError.
    ///
    /// pre:  kind is a valid McpErrorKind, message is non-empty
    /// post: returns McpToolError
    pub fn new(kind: McpErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }
    /// Create an internal error.
    ///
    /// post: returns McpToolError with Internal kind
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Internal, message)
    }
    /// Create a not-found error.
    ///
    /// post: returns McpToolError with NotFound kind
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::NotFound, message)
    }
    /// Create an invalid-argument error.
    ///
    /// post: returns McpToolError with InvalidArgument kind
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::InvalidArgument, message)
    }
    /// Create an unavailable error.
    ///
    /// post: returns McpToolError with Unavailable kind
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Unavailable, message)
    }
    /// Create a timeout error.
    ///
    /// post: returns McpToolError with Timeout kind
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::Timeout, message)
    }
    /// Create a permission-denied error.
    ///
    /// post: returns McpToolError with PermissionDenied kind
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::PermissionDenied, message)
    }
    /// Create a rate-limited error.
    ///
    /// post: returns McpToolError with RateLimited kind
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::RateLimited, message)
    }
    /// Create a failed-precondition error.
    ///
    /// post: returns McpToolError with FailedPrecondition kind
    pub fn failed_precondition(message: impl Into<String>) -> Self {
        Self::new(McpErrorKind::FailedPrecondition, message)
    }
    /// Serialize to JSON string for MCP wire format.
    ///
    /// post: returns JSON string with "error" object
    pub fn to_json_string(&self) -> String {
        serde_json::json!({"error": self.message, "kind": self.kind.to_string()}).to_string()
    }
}

impl std::fmt::Display for McpToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for McpToolError {}

/// Convenience: produce an internal error response for a named failed operation.
///
/// Combines `context` ("what failed") and `e` into a standard `{"error": "Failed to ...: ..."}` JSON
/// body, eliminating the repeated `span.internal_error(json!({...}))` pattern across servers.
/// Produce a JSON-RPC error response for internal tool errors.
///
/// pre:  message is non-empty
/// post: returns JSON string with error object
pub fn tool_internal_error(
    span: ToolSpanGuard,
    context: &str,
    e: impl std::fmt::Display,
) -> String {
    span.internal_error(serde_json::json!({"error": format!("Failed to {context}: {e}")}))
}

// Input validation — Shared sanitization for MCP tool parameters

/// Validate a string identifier.
/// Validate an identifier (tool name, server name, etc.).
///
/// pre:  name and value are non-empty, max_len > 0
/// post: returns Ok(()) if valid (non-empty, ≤max_len, alphanumeric+hyphen+underscore)
/// post: returns Err if invalid
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
/// Validate a tool URL (http/https only, no path traversal).
///
/// pre:  url is non-empty
/// post: returns Ok(()) if valid http/https URL
/// post: returns Err if invalid scheme or format
pub fn validate_tool_url(url: &str) -> Result<(), McpToolError> {
    crate::security::validate_url(url, &crate::security::UrlValidationConfig::default())
        .map_err(|e| McpToolError::invalid_argument(format!("URL validation failed: {e}")))
}

// classify_http_error — Shared HTTP Status → McpToolError mapping
/// Classify an HTTP error response into a structured `McpToolError`.
/// Classify an HTTP error response into an McpToolError.
///
/// pre:  service is non-empty, status is valid
/// post: returns McpToolError with appropriate kind based on status code
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

// api_get / api_post / api_put — Shared HTTP helpers

async fn http_req(
    client: &reqwest::Client,
    service: &str,
    method: &str,
    url: &str,
    payload: Option<&Value>,
) -> Result<Value, McpToolError> {
    let builder = match method {
        "GET" => client.get(url),
        "POST" => client.post(url).json(payload.unwrap_or(&Value::Null)),
        _ => client.put(url).json(payload.unwrap_or(&Value::Null)),
    };
    let resp = builder
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

pub async fn api_get(
    client: &reqwest::Client,
    service: &str,
    url: &str,
) -> Result<Value, McpToolError> {
    http_req(client, service, "GET", url, None).await
}
pub async fn api_post(
    client: &reqwest::Client,
    service: &str,
    url: &str,
    payload: &Value,
) -> Result<Value, McpToolError> {
    http_req(client, service, "POST", url, Some(payload)).await
}
pub async fn api_put(
    client: &reqwest::Client,
    service: &str,
    url: &str,
    payload: &Value,
) -> Result<Value, McpToolError> {
    http_req(client, service, "PUT", url, Some(payload)).await
}

// resolve_credential — Keystore-first credential resolution

/// Parse .env files and return key-value pairs without mutating the process environment.
/// Load .env file from the project root.
///
/// post: returns HashMap of env vars from .env file
/// post: returns empty map if .env not found
pub fn load_dotenv() -> HashMap<String, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    for path in [cwd.join(".env")].iter().chain(
        cwd.parent()
            .map(|p| vec![p.join(".env")])
            .unwrap_or_default()
            .iter(),
    ) {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut map = HashMap::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let (key, value) = (key.trim(), value.trim());
                    if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                        map.insert(key.into(), value.into());
                    }
                }
            }
            return map;
        }
    }
    HashMap::new()
}

/// Resolve a credential from env var or OS keychain.
///
/// pre:  env_var is non-empty
/// post: returns credential value from env or keychain
pub fn resolve_credential(env_var: &str) -> Result<String, hkask_keystore::KeystoreError> {
    match hkask_keystore::Keychain::default().retrieve_by_key(env_var) {
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
                "Credential '{}' not found",
                env_var
            ))),
        },
    }
}

// emit_tool_span — CNS observability for tool invocations

/// Emit a CNS tool span with caller identity (WebID) for observability.
fn emit_tool_span_with_caller(
    tool_name: &str,
    outcome: &str,
    duration_ms: u64,
    error_kind: Option<&McpErrorKind>,
    caller: Option<&hkask_types::WebID>,
) {
    tracing::info!(target: "cns.tool", tool = tool_name, outcome = outcome, duration_ms = duration_ms, error_kind = error_kind.map(|k| k.to_string()).as_deref().unwrap_or(""), caller = caller.map(|w| w.to_string()).as_deref().unwrap_or(""), "CNS tool span");
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
/// - `server_name` — Human-readable server name for logging (e.g., `"hkask-mcp-web"`)
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
) -> Result<(), McpError>
where
    S: rmcp::ServiceExt<rmcp::RoleServer> + rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> Result<S, McpError>,
{
    run_stdio_server_impl(server_name, version, server_factory, credentials, None).await
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
) -> Result<(), McpError>
where
    S: rmcp::ServiceExt<rmcp::RoleServer> + rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> Result<S, McpError>,
{
    run_stdio_server_impl(
        server_name,
        version,
        server_factory,
        credentials,
        Some(preloaded),
    )
    .await
}

/// Unified stdio server bootstrap — resolves credentials, constructs ServerContext,
/// and serves via rmcp stdio transport. Accepts optional preloaded credentials
/// for .env file injection (used by `run_stdio_server_with_preloaded`).
async fn run_stdio_server_impl<S, F>(
    server_name: &str,
    version: &str,
    server_factory: F,
    credentials: Vec<CredentialRequirement>,
    preloaded: Option<HashMap<String, String>>,
) -> Result<(), McpError>
where
    S: rmcp::ServiceExt<rmcp::RoleServer> + rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> Result<S, McpError>,
{
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut resolved = HashMap::new();
    let mut missing_required = Vec::new();
    for cred in &credentials {
        if let Some(ref pre) = preloaded
            && let Some(val) = pre.get(&cred.env_var)
        {
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
                tracing::error!(credential = %cred.env_var, description = %cred.description, "Required credential not set — server cannot function");
                missing_required.push(cred.env_var.clone());
            }
            Err(_) => {
                tracing::warn!(credential = %cred.env_var, description = %cred.description, "Optional credential not set — server will operate with degraded functionality")
            }
        }
    }
    if !missing_required.is_empty() {
        return Err(McpError::MissingCredentials {
            missing: missing_required.join(", "),
        });
    }

    let webid = if let Some(ref pre) = preloaded {
        if let Some(uuid_str) = pre.get("HKASK_WEBID") {
            hkask_types::WebID::from_str(uuid_str).unwrap_or_else(|_| {
                tracing::warn!(
                    "HKASK_WEBID set but invalid format — falling back to anonymous identity"
                );
                hkask_types::WebID::from_persona(b"anonymous")
            })
        } else if let Ok(uuid_str) = std::env::var("HKASK_WEBID") {
            hkask_types::WebID::from_str(&uuid_str).unwrap_or_else(|_| {
                tracing::warn!(
                    "HKASK_WEBID set but invalid format — falling back to anonymous identity"
                );
                hkask_types::WebID::from_persona(b"anonymous")
            })
        } else if let Some(persona) = pre.get("HKASK_AGENT_PERSONA") {
            hkask_types::WebID::from_persona(persona.as_bytes())
        } else if let Ok(persona) = std::env::var("HKASK_AGENT_PERSONA") {
            hkask_types::WebID::from_persona(persona.as_bytes())
        } else {
            tracing::warn!(
                "No HKASK_WEBID or HKASK_AGENT_PERSONA set — MCP server starting with anonymous identity. Set HKASK_WEBID for P12-compliant attribution."
            );
            hkask_types::WebID::from_persona(b"anonymous")
        }
    } else if let Ok(uuid_str) = std::env::var("HKASK_WEBID") {
        hkask_types::WebID::from_str(&uuid_str).unwrap_or_else(|_| {
            tracing::warn!(
                "HKASK_WEBID set but invalid format — falling back to anonymous identity"
            );
            hkask_types::WebID::from_persona(b"anonymous")
        })
    } else if let Ok(persona) = std::env::var("HKASK_AGENT_PERSONA") {
        hkask_types::WebID::from_persona(persona.as_bytes())
    } else {
        tracing::warn!(
            "No HKASK_WEBID or HKASK_AGENT_PERSONA set — MCP server starting with anonymous identity. Set HKASK_WEBID for P12-compliant attribution."
        );
        hkask_types::WebID::from_persona(b"anonymous")
    };

    tracing::info!(webid = %webid.redacted_display(), "Agent identity resolved");
    let capability_tier = CapabilityTier::detect(&resolved);
    tracing::info!(
        embedded = capability_tier.embedded,
        keystore = capability_tier.keystore_available,
        persistence = capability_tier.persistence_available,
        "Capability tier detected"
    );
    let ctx = ServerContext {
        credentials: resolved,
        adapters: crate::AdapterContainer::new(),
        webid,
        capability_tier,
    };
    let server = server_factory(ctx)?;
    tracing::info!(
        server = server_name,
        version = version,
        "MCP server starting"
    );
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    running
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse a `McpToolError::to_json_string()` output and extract fields.
    fn parse_error_json(json: &str) -> (String, String) {
        let v: serde_json::Value = serde_json::from_str(json).expect("error JSON should be valid");
        let error = v["error"]
            .as_str()
            .expect("should have 'error' field")
            .to_string();
        let kind = v["kind"]
            .as_str()
            .expect("should have 'kind' field")
            .to_string();
        (error, kind)
    }

    #[test]
    fn all_error_kinds_produce_correct_wire_format() {
        let cases = vec![
            (McpToolError::internal("internal bug"), "internal"),
            (McpToolError::unavailable("downstream down"), "unavailable"),
            (McpToolError::timeout("timed out"), "timeout"),
            (McpToolError::not_found("missing resource"), "not_found"),
            (
                McpToolError::invalid_argument("bad input"),
                "invalid_argument",
            ),
            (
                McpToolError::permission_denied("access denied"),
                "permission_denied",
            ),
            (
                McpToolError::rate_limited("too many requests"),
                "rate_limited",
            ),
            (
                McpToolError::failed_precondition("not initialized"),
                "failed_precondition",
            ),
        ];

        for (err, expected_kind) in cases {
            let json = err.to_json_string();
            let (error_msg, kind) = parse_error_json(&json);
            assert!(!error_msg.is_empty(), "error message should not be empty");
            assert_eq!(
                kind, expected_kind,
                "kind field should match McpErrorKind Display"
            );
            // Verify the JSON is valid and has exactly 2 top-level keys
            let v: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert!(
                v.as_object().unwrap().len() == 2,
                "error JSON should have exactly 2 fields (error + kind)"
            );
        }
    }

    #[test]
    fn error_wire_format_golden_strings() {
        // These exact JSON strings are the contract. Changing them breaks all clients.
        assert_eq!(
            McpToolError::internal("boom").to_json_string(),
            r#"{"error":"boom","kind":"internal"}"#
        );
        assert_eq!(
            McpToolError::not_found("gone").to_json_string(),
            r#"{"error":"gone","kind":"not_found"}"#
        );
        assert_eq!(
            McpToolError::invalid_argument("bad").to_json_string(),
            r#"{"error":"bad","kind":"invalid_argument"}"#
        );
        assert_eq!(
            McpToolError::permission_denied("nope").to_json_string(),
            r#"{"error":"nope","kind":"permission_denied"}"#
        );
        assert_eq!(
            McpToolError::unavailable("down").to_json_string(),
            r#"{"error":"down","kind":"unavailable"}"#
        );
        assert_eq!(
            McpToolError::timeout("late").to_json_string(),
            r#"{"error":"late","kind":"timeout"}"#
        );
        assert_eq!(
            McpToolError::rate_limited("wait").to_json_string(),
            r#"{"error":"wait","kind":"rate_limited"}"#
        );
        assert_eq!(
            McpToolError::failed_precondition("nope").to_json_string(),
            r#"{"error":"nope","kind":"failed_precondition"}"#
        );
    }

    #[test]
    fn error_kind_display_matches_wire_format() {
        for kind in &[
            McpErrorKind::Internal,
            McpErrorKind::Unavailable,
            McpErrorKind::Timeout,
            McpErrorKind::NotFound,
            McpErrorKind::InvalidArgument,
            McpErrorKind::PermissionDenied,
            McpErrorKind::RateLimited,
            McpErrorKind::FailedPrecondition,
        ] {
            let err = McpToolError::new(*kind, "test");
            let json = err.to_json_string();
            let (_, wire_kind) = parse_error_json(&json);
            assert_eq!(
                wire_kind,
                kind.to_string(),
                "wire format kind must match McpErrorKind Display"
            );
        }
    }

    #[test]
    fn classify_http_error_maps_status_codes() {
        use reqwest::StatusCode;

        // 401/403 → PermissionDenied
        let err = classify_http_error("TestSvc", StatusCode::UNAUTHORIZED, "unauthorized");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);
        let err = classify_http_error("TestSvc", StatusCode::FORBIDDEN, "forbidden");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);

        // 404 → NotFound
        let err = classify_http_error("TestSvc", StatusCode::NOT_FOUND, "missing");
        assert_eq!(err.kind, McpErrorKind::NotFound);

        // 422 → InvalidArgument
        let err = classify_http_error("TestSvc", StatusCode::UNPROCESSABLE_ENTITY, "bad schema");
        assert_eq!(err.kind, McpErrorKind::InvalidArgument);

        // 429 → RateLimited
        let err = classify_http_error("TestSvc", StatusCode::TOO_MANY_REQUESTS, "rate limited");
        assert_eq!(err.kind, McpErrorKind::RateLimited);

        // 502/503 → Unavailable
        let err = classify_http_error("TestSvc", StatusCode::BAD_GATEWAY, "bad gateway");
        assert_eq!(err.kind, McpErrorKind::Unavailable);
        let err = classify_http_error("TestSvc", StatusCode::SERVICE_UNAVAILABLE, "down");
        assert_eq!(err.kind, McpErrorKind::Unavailable);

        // Other 5xx → Unavailable
        let err = classify_http_error("TestSvc", StatusCode::INTERNAL_SERVER_ERROR, "boom");
        assert_eq!(err.kind, McpErrorKind::Unavailable);

        // Unknown → Internal
        let err = classify_http_error("TestSvc", StatusCode::OK, "unexpected");
        assert_eq!(err.kind, McpErrorKind::Internal);
    }

    // ── Capability Enforcement Tests ─────────────────────────────────────

    #[test]
    fn permission_denied_error_carries_message() {
        let err = McpToolError::permission_denied("agent lacks tool:execute capability");
        assert_eq!(err.kind, McpErrorKind::PermissionDenied);
        assert!(
            err.to_string()
                .contains("agent lacks tool:execute capability")
        );
        let json = err.to_json_string();
        assert!(json.contains("permission_denied"));
        assert!(json.contains("agent lacks tool:execute capability"));
    }

    #[test]
    fn failed_precondition_error_for_expired_token() {
        let err = McpToolError::failed_precondition("delegation token expired at 1000");
        assert_eq!(err.kind, McpErrorKind::FailedPrecondition);
        assert!(err.to_string().contains("delegation token expired"));
    }

    #[test]
    fn rate_limited_error_for_energy_budget_exceeded() {
        let err = McpToolError::rate_limited("energy budget exceeded for tool:execute");
        assert_eq!(err.kind, McpErrorKind::RateLimited);
        assert!(err.to_string().contains("energy budget exceeded"));
    }

    // ── Error Propagation Tests ───────────────────────────────────────────

    #[test]
    fn internal_error_propagates_with_context() {
        let err = McpToolError::internal("downstream inference engine returned 500");
        assert_eq!(err.kind, McpErrorKind::Internal);
        assert!(
            err.to_string()
                .contains("downstream inference engine returned 500")
        );
        // Verify JSON round-trip preserves error context
        let json = err.to_json_string();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "downstream inference engine returned 500");
        assert_eq!(parsed["kind"], "internal");
    }

    #[test]
    fn timeout_error_propagates_with_context() {
        let err = McpToolError::timeout("tool:execute timed out after 30s");
        assert_eq!(err.kind, McpErrorKind::Timeout);
        assert!(err.to_string().contains("tool:execute timed out after 30s"));
        let json = err.to_json_string();
        assert!(json.contains("timeout"));
        assert!(json.contains("tool:execute timed out after 30s"));
    }

    #[test]
    fn not_found_error_for_unknown_tool() {
        let err = McpToolError::not_found("unknown tool: none_such");
        assert_eq!(err.kind, McpErrorKind::NotFound);
        assert!(err.to_string().contains("unknown tool: none_such"));
    }

    // ── Tool Discovery Tests ──────────────────────────────────────────────

    #[test]
    fn validate_identifier_accepts_valid_names() {
        assert!(validate_identifier("tool_name", "web_search", 64).is_ok());
        assert!(validate_identifier("tool_name", "file_read", 64).is_ok());
        assert!(validate_identifier("tool_name", "my_tool_123", 64).is_ok());
        assert!(validate_identifier("tool_name", "a", 64).is_ok());
    }

    #[test]
    fn validate_identifier_rejects_invalid_names() {
        assert!(validate_identifier("tool_name", "", 64).is_err());
        assert!(validate_identifier("tool_name", "tool name", 64).is_err()); // space
    }

    #[test]
    fn validate_identifier_rejects_overly_long_names() {
        let long_name = "a".repeat(65);
        assert!(validate_identifier("tool_name", &long_name, 64).is_err());
    }

    #[test]
    fn validate_tool_url_accepts_valid_urls() {
        assert!(validate_tool_url("http://localhost:8080").is_ok());
        assert!(validate_tool_url("https://api.example.com/v1").is_ok());
    }

    #[test]
    fn validate_tool_url_rejects_invalid_urls() {
        assert!(validate_tool_url("not-a-url").is_err());
        assert!(validate_tool_url("").is_err());
    }
}
