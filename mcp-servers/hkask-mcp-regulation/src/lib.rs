//! MCP server for hkask-regulation — regulation span history query tools.
//!
//! Exposes two tools for reading regulation regulation record history from the persistent
//! `RegulationArchive`:
//! - `reg_query_spans` — query events by span_category prefix within a time window
//! - `reg_span_stats`  — aggregate counts by span_category
//!
//! These tools are the runtime telemetry surface that the
//! `runtime-posture-monitor` skill consumes to observe `reg.guard.*`,
//! `reg.regulation`, and `hkask.*` performative spans.
//!
//! The stored `span_category` column holds the short name (e.g. "guard.input",
//! "regulation", "gas") — i.e. the `SpanNamespace::short_name()` with the
//! `reg.` prefix stripped. Callers pass the full `reg.*` namespace (e.g.
//! "reg.guard"); the server strips the `reg.` prefix before querying so the
//! `LIKE 'prefix%'` predicate hits the index on `(span_category, phase)`.

#![allow(unused_crate_dependencies)]

use hkask_storage::database::sqlite::SqliteDriver;
use hkask_mcp_server::DaemonClient;
use hkask_mcp_server::run_server;
use hkask_mcp_server::server::{McpToolError, execute_tool};
use hkask_storage::RegulationArchive;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

const SERVER_NAME: &str = "hkask-mcp-regulation";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

hkask_mcp_server::mcp_server!(
    pub struct RegulationServer {
        regulation_store: Option<Arc<RegulationArchive>>,
    }
);

// ── Request types ─────────────────────────────────────────────────

/// Request for `reg_query_spans`.
///
/// `namespace` is the full canonical regulation namespace prefix (e.g. "reg.guard",
/// "reg.outcome", "hkask"). The server strips the `reg.` prefix before
/// querying the `span_category` column, which stores short names.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuerySpansRequest {
    /// Canonical namespace prefix (e.g. "reg.guard", "reg.outcome", "hkask").
    /// Empty string is rejected with `invalid_argument`.
    namespace: String,
    /// Lookback window in hours (default 1.0).
    #[serde(default = "default_since_hours")]
    since_hours: f64,
    /// Maximum number of events to return (default 100).
    #[serde(default = "default_limit")]
    limit: u64,
}

fn default_since_hours() -> f64 {
    1.0
}

fn default_limit() -> u64 {
    100
}

/// Request for `reg_span_stats`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SpanStatsRequest {
    /// Canonical namespace prefix (e.g. "reg.guard", "reg.outcome", "hkask").
    /// Empty string is rejected with `invalid_argument`.
    namespace: String,
    /// Lookback window in hours (default 1.0).
    #[serde(default = "default_since_hours")]
    since_hours: f64,
}

// ── Tools ──────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl RegulationServer {
    #[tool(
        description = "Query regulation record history by namespace prefix within a time window. Returns events ordered by timestamp ASC. Use 'reg.guard' for guard violations, 'reg.outcome' for regulation events, 'hkask' for performative telemetry."
    )]
    pub async fn reg_query_spans(&self, Parameters(req): Parameters<QuerySpansRequest>) -> String {
        execute_tool(self, "reg_query_spans", async {
            let namespace = req.namespace.trim();
            if namespace.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "namespace must be a non-empty string (e.g. \"reg.guard\", \"reg.outcome\", \"hkask\")",
                ));
            }
            let Some(ref store) = self.regulation_store else {
                return Err(McpToolError::permission_denied(
                    "RegulationArchive not available — set HKASK_DB_PATH and HKASK_DB_PASSPHRASE",
                ));
            };
            let since = chrono::Utc::now()
                - chrono::Duration::seconds((req.since_hours * 3600.0) as i64);
            // The stored span_category column holds the short name (e.g. "guard.input",
            // "regulation", "gas"). Strip the "reg." prefix so LIKE 'prefix%' hits the
            // (span_category, phase) index.
            let short_prefix = strip_reg_prefix(namespace);
            let events = store
                .query_by_namespace(short_prefix, since, req.limit)
                .map_err(|e| McpToolError::internal(format!("Regulation query failed: {e}")))?;
            let count = events.len();
            let serialized: Vec<serde_json::Value> = events
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "timestamp": e.timestamp.to_rfc3339(),
                        "observer_webid": e.observer_webid.to_string(),
                        "namespace": e.span.namespace.as_str(),
                        "path": e.span.path,
                        "phase": e.phase.as_str(),
                        "observation": e.observation,
                        "regulation": e.regulation,
                        "outcome": e.outcome,
                        "recursion_depth": e.recursion_depth,
                        "parent_event": e.parent_event.map(|id| id.to_string()),
                        "visibility": e.visibility,
                    })
                })
                .collect();
            Ok(serde_json::json!({
                "namespace": namespace,
                "since": since.to_rfc3339(),
                "limit": req.limit,
                "count": count,
                "events": serialized,
            }))
        })
        .await
    }

    #[tool(
        description = "Aggregate regulation regulation record counts by exact span_category within a namespace prefix and time window. Returns a JSON object mapping each span_category to its count, ordered by count DESC."
    )]
    pub async fn reg_span_stats(&self, Parameters(req): Parameters<SpanStatsRequest>) -> String {
        execute_tool(self, "reg_span_stats", async {
            let namespace = req.namespace.trim();
            if namespace.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "namespace must be a non-empty string (e.g. \"reg.guard\", \"reg.outcome\", \"hkask\")",
                ));
            }
            let Some(ref store) = self.regulation_store else {
                return Err(McpToolError::permission_denied(
                    "RegulationArchive not available — set HKASK_DB_PATH and HKASK_DB_PASSPHRASE",
                ));
            };
            let since = chrono::Utc::now()
                - chrono::Duration::seconds((req.since_hours * 3600.0) as i64);
            let short_prefix = strip_reg_prefix(namespace);
            let stats = store
                .query_span_stats(short_prefix, since)
                .map_err(|e| McpToolError::internal(format!("Regulation stats query failed: {e}")))?;
            let total: u64 = stats.iter().map(|(_, c)| *c).sum();
            let mut categories: HashMap<String, u64> = HashMap::new();
            for (cat, cnt) in stats {
                categories.insert(cat, cnt);
            }
            Ok(serde_json::json!({
                "namespace": namespace,
                "since": since.to_rfc3339(),
                "total_events": total,
                "categories": categories,
            }))
        })
        .await
    }
}

/// Strip the `reg.` prefix from a namespace so it matches the short-name
/// `span_category` column. Non-`reg.` namespaces (e.g. `hkask`) are returned
/// as-is so callers can query performative telemetry too.
fn strip_reg_prefix(namespace: &str) -> &str {
    namespace.strip_prefix("reg.").unwrap_or(namespace)
}

// ── Server startup ─────────────────────────────────────────────────────

/// Open the RegulationArchive from the configured database.
///
/// Follows the curator pattern: read `HKASK_DB_PATH` (or fall back to the
/// regulation pod database path) and `HKASK_DB_PASSPHRASE` from credentials/env.
/// Returns `None` (graceful degradation) when the database cannot be opened —
/// the tools then return `permission_denied` so callers see a clear message.
fn open_regulation_store(ctx: &hkask_mcp_server::server::ServerContext) -> Option<Arc<RegulationArchive>> {
    let db_path = ctx
        .credentials
        .get("HKASK_DB_PATH")
        .cloned()
        .or_else(|| std::env::var("HKASK_DB_PATH").ok())
        .unwrap_or_else(|| {
            let p = hkask_types::agent_paths::userpod_pod_db("regulation");
            let resolved = hkask_types::agent_paths::resolve_under_data_dir(&p);
            if let Some(parent) = resolved.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            resolved.to_string_lossy().to_string()
        });

    let passphrase = ctx
        .credentials
        .get("HKASK_DB_PASSPHRASE")
        .cloned()
        .or_else(|| std::env::var("HKASK_DB_PASSPHRASE").ok());
    let passphrase = match passphrase {
        Some(pw) => pw,
        None => {
            tracing::warn!(
                target: "hkask.mcp.regulation",
                "HKASK_DB_PASSPHRASE not set — RegulationArchive unavailable"
            );
            return None;
        }
    };

    let db = match hkask_storage::open_or_repair(&db_path, &passphrase) {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.regulation",
                error = %e,
                path = %db_path,
                "Failed to open regulation database"
            );
            return None;
        }
    };
    let pool = match db.sqlite_pool() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.regulation",
                error = %e,
                "Failed to get SQLite pool"
            );
            return None;
        }
    };
    let driver: Arc<dyn hkask_storage::database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    Some(Arc::new(RegulationArchive::from_driver(driver)))
}

pub async fn run(
    userpod: String,
    daemon_client: Option<DaemonClient>,
) -> Result<(), hkask_mcp_server::McpError> {
    run_server(
        SERVER_NAME,
        SERVER_VERSION,
        |ctx: hkask_mcp_server::server::ServerContext| {
            let regulation_store = open_regulation_store(&ctx);
            Ok(RegulationServer::new(
                ctx.webid,
                userpod.clone(),
                daemon_client.clone(),
                regulation_store,
            ))
        },
        vec![
            hkask_mcp_server::CredentialRequirement::optional(
                "HKASK_DB_PATH",
                "Path to the SQLCipher database holding the reg_records table",
            ),
            hkask_mcp_server::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "SQLCipher encryption passphrase",
            ),
        ],
    )
    .await
}
