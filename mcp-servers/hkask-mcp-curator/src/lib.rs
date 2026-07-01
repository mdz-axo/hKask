//! hkask-mcp-curator — Curator MCP server library.
//!
//! Exposes the Curator's regulatory surface as MCP tools:
//! system health, escalation management, CNS observability,
//! cross-pod semantic search, memory recall, spec drift detection,
//! and algedonic event history.

pub mod types;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use hkask_mcp::daemon::DaemonResponse;
use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_services_curator::CuratorService;

use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::NuEventSink;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use std::sync::Arc;

use types::*;

const SERVER_NAME: &str = "hkask-mcp-curator";

pub struct CuratorServer {
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    escalation_queue: Option<Arc<hkask_storage::EscalationQueue>>,
    nu_event_store: Option<Arc<hkask_storage::NuEventStore>>,
    episodic: Option<hkask_memory::EpisodicMemory>,
    semantic: Option<Arc<hkask_memory::SemanticMemory>>,
    token_registry: Option<Arc<dyn hkask_capability::TokenRegistry>>,
}

#[tool_router(server_handler)]
impl CuratorServer {
    // ── Liveness ───────────────────────────────────────────────────────

    #[tool(description = "Liveness check")]
    pub async fn curator_ping(&self, Parameters(_req): Parameters<PingRequest>) -> String {
        execute_tool(self, "curator_ping", async {
            Ok(json!({
                "status": "ok",
                "server": SERVER_NAME,
                "curator_webid": self.webid.to_string(),
                "replicant": self.replicant,
                "daemon_connected": self.daemon.is_some(),
                "stores": {
                    "escalation_queue": self.escalation_queue.is_some(),
                    "nu_event_store": self.nu_event_store.is_some(),
                    "episodic": self.episodic.is_some(),
                    "semantic": self.semantic.is_some(),
                }
            }))
        })
        .await
    }

    // ── Escalation Management ──────────────────────────────────────────

    #[tool(description = "List all pending escalations requiring review")]
    pub async fn curator_escalations(&self, Parameters(_req): Parameters<PingRequest>) -> String {
        execute_tool(self, "curator_escalations", async {
            let Some(ref queue) = self.escalation_queue else {
                return Err(McpToolError::permission_denied(
                    "EscalationQueue not available",
                ));
            };
            match CuratorService::list_escalations_direct(queue.as_ref()) {
                Ok(entries) => {
                    let serialized: Vec<serde_json::Value> = entries
                        .iter()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "template_id": e.template_id,
                                "bot_id": e.bot_id,
                                "output": e.output,
                                "confidence": e.confidence,
                                "retry_count": e.retry_count,
                                "error_context": e.error_context,
                                "created_at": e.created_at,
                                "status": e.status,
                                "resolved_at": e.resolved_at,
                                "resolved_by": e.resolved_by,
                            })
                        })
                        .collect();
                    Ok(json!({"count": serialized.len(), "escalations": serialized}))
                }
                Err(e) => Err(McpToolError::internal(format!("{e}"))),
            }
        })
        .await
    }

    #[tool(description = "Resolve an escalation by ID")]
    pub async fn curator_escalation_resolve(
        &self,
        Parameters(req): Parameters<EscalationResolveRequest>,
    ) -> String {
        execute_tool(self, "curator_escalation_resolve", async {
            let Some(ref queue) = self.escalation_queue else {
                return Err(McpToolError::permission_denied(
                    "EscalationQueue not available",
                ));
            };
            let Some(ref events_store) = self.nu_event_store else {
                return Err(McpToolError::permission_denied(
                    "NuEventStore not available",
                ));
            };
            let events: Arc<dyn NuEventSink> = Arc::clone(events_store) as Arc<dyn NuEventSink>;
            match CuratorService::resolve_direct(queue.as_ref(), &events, &req.id, &self.replicant)
            {
                Ok(()) => Ok(json!({"resolved": true, "id": req.id})),
                Err(e) => Err(McpToolError::internal(format!("{e}"))),
            }
        })
        .await
    }

    #[tool(description = "Dismiss an escalation as not actionable")]
    pub async fn curator_escalation_dismiss(
        &self,
        Parameters(req): Parameters<EscalationDismissRequest>,
    ) -> String {
        execute_tool(self, "curator_escalation_dismiss", async {
            let Some(ref queue) = self.escalation_queue else {
                return Err(McpToolError::permission_denied(
                    "EscalationQueue not available",
                ));
            };
            let Some(ref events_store) = self.nu_event_store else {
                return Err(McpToolError::permission_denied(
                    "NuEventStore not available",
                ));
            };
            let events: Arc<dyn NuEventSink> = Arc::clone(events_store) as Arc<dyn NuEventSink>;
            match CuratorService::dismiss_direct(queue.as_ref(), &events, &req.id, &self.replicant)
            {
                Ok(()) => Ok(json!({"dismissed": true, "id": req.id})),
                Err(e) => Err(McpToolError::internal(format!("{e}"))),
            }
        })
        .await
    }

    // ── System Health ──────────────────────────────────────────────────

    #[tool(description = "Run metacognition cycle — requires live daemon for CNS data")]
    pub async fn curator_health(&self, Parameters(_req): Parameters<PingRequest>) -> String {
        execute_tool(self, "curator_health", async {
            let Some(ref daemon) = self.daemon else {
                return Err(McpToolError::unavailable("Daemon not available"));
            };
            match daemon.curator_health_query(&self.replicant).await {
                Ok(DaemonResponse::CuratorHealthResponse { health }) => Ok(health),
                Ok(other) => Err(McpToolError::internal(format!(
                    "Bad daemon response: {:?}",
                    other
                ))),
                Err(e) => Err(McpToolError::internal(format!("Daemon query failed: {e}"))),
            }
        })
        .await
    }

    #[tool(description = "Live CNS status — variety per domain")]
    pub async fn curator_cns_status(
        &self,
        Parameters(req): Parameters<CnsStatusRequest>,
    ) -> String {
        execute_tool(self, "curator_cns_status", async {
            let Some(ref daemon) = self.daemon else {
                return Err(McpToolError::unavailable("Daemon not available"));
            };
            match daemon
                .cns_status_query(&self.replicant, req.domain.as_deref())
                .await
            {
                Ok(DaemonResponse::CnsStatusResponse { status }) => Ok(status),
                Ok(other) => Err(McpToolError::internal(format!(
                    "Bad daemon response: {:?}",
                    other
                ))),
                Err(e) => Err(McpToolError::internal(format!("Daemon query failed: {e}"))),
            }
        })
        .await
    }

    // ── Specification Curation ─────────────────────────────────────────

    #[tool(description = "Check specs for drift from registered verbs")]
    pub async fn curator_spec_drift(
        &self,
        Parameters(req): Parameters<SpecDriftRequest>,
    ) -> String {
        execute_tool(self, "curator_spec_drift", async {
            let Some(ref daemon) = self.daemon else {
                return Err(McpToolError::unavailable("Daemon not available"));
            };
            match daemon
                .spec_drift_query(&self.replicant, req.spec_id.as_deref())
                .await
            {
                Ok(DaemonResponse::SpecDriftResponse { drift }) => Ok(drift),
                Ok(other) => Err(McpToolError::internal(format!(
                    "Bad daemon response: {:?}",
                    other
                ))),
                Err(e) => Err(McpToolError::internal(format!("Daemon query failed: {e}"))),
            }
        })
        .await
    }

    // ── Memory & Learning ──────────────────────────────────────────────

    #[tool(description = "Query the Curator's semantic memory by entity name")]
    pub async fn curator_semantic_search(
        &self,
        Parameters(req): Parameters<SemanticSearchRequest>,
    ) -> String {
        execute_tool(self, "curator_semantic_search", async {
            let Some(ref semantic) = self.semantic else {
                return Err(McpToolError::permission_denied("SemanticMemory not available"));
            };
            match semantic.query_deduped(&req.query) {
                Ok(triples) => {
                    let limit = req.limit.unwrap_or(10);
                    let serialized: Vec<serde_json::Value> = triples
                        .iter()
                        .take(limit)
                        .map(|t| {
                            json!({
                                "entity": t.entity, "attribute": t.attribute,
                                "value": t.value, "confidence": t.confidence,
                            })
                        })
                        .collect();
                    Ok(json!({"count": serialized.len(), "total": triples.len(), "results": serialized}))
                }
                Err(e) => Err(McpToolError::internal(format!("Semantic recall failed: {e}"))),
            }
        })
        .await
    }

    #[tool(description = "Recall the Curator's episodic and semantic memory about an entity")]
    pub async fn curator_memory_recall(
        &self,
        Parameters(req): Parameters<MemoryRecallRequest>,
    ) -> String {
        execute_tool(self, "curator_memory_recall", async {
            let memory_type = req.memory_type.as_deref().unwrap_or("both");
            let mut result = json!({});

            if memory_type == "episodic" || memory_type == "both" {
                if let Some(ref ep) = self.episodic {
                    match ep.query_for_deduped(&req.entity, self.webid) {
                        Ok(triples) => {
                            let s: Vec<serde_json::Value> = triples
                                .iter()
                                .map(|t| {
                                    json!({
                                        "entity": t.entity, "attribute": t.attribute,
                                        "value": t.value, "confidence": t.confidence,
                                        "valid_from": t.temporal.valid_from.to_rfc3339(),
                                    })
                                })
                                .collect();
                            result["episodic"] = json!({"count": s.len(), "triples": s});
                        }
                        Err(e) => {
                            result["episodic"] = json!({"error": format!("{e}")});
                        }
                    }
                } else {
                    result["episodic"] = json!({"status": "unavailable"});
                }
            }
            if memory_type == "semantic" || memory_type == "both" {
                if let Some(ref sem) = self.semantic {
                    match sem.query_deduped(&req.entity) {
                        Ok(triples) => {
                            let s: Vec<serde_json::Value> = triples
                                .iter()
                                .map(|t| {
                                    json!({
                                        "entity": t.entity, "attribute": t.attribute,
                                        "value": t.value, "confidence": t.confidence,
                                    })
                                })
                                .collect();
                            result["semantic"] = json!({"count": s.len(), "triples": s});
                        }
                        Err(e) => {
                            result["semantic"] = json!({"error": format!("{e}")});
                        }
                    }
                } else {
                    result["semantic"] = json!({"status": "unavailable"});
                }
            }
            Ok(result)
        })
        .await
    }

    // ── Algedonic History ──────────────────────────────────────────────

    #[tool(description = "Read algedonic event log for a time window")]
    pub async fn curator_algedonic_log(
        &self,
        Parameters(req): Parameters<AlgedonicLogRequest>,
    ) -> String {
        execute_tool(self, "curator_algedonic_log", async {
            let Some(ref store) = self.nu_event_store else {
                return Err(McpToolError::permission_denied(
                    "NuEventStore not available",
                ));
            };
            let hours = req.hours.unwrap_or(24);
            let since = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
            match store.query_algedonic(since, 500) {
                Ok(events) => {
                    let s: Vec<serde_json::Value> = events
                        .iter()
                        .map(|e| {
                            json!({
                                "timestamp": e.timestamp.to_rfc3339(),
                                "span": e.span.path,
                                "phase": format!("{:?}", e.phase),
                                "observation": e.observation,
                            })
                        })
                        .collect();
                    Ok(json!({"window_hours": hours, "count": s.len(), "events": s}))
                }
                Err(e) => Err(McpToolError::internal(format!(
                    "Algedonic query failed: {e}"
                ))),
            }
        })
        .await
    }

    // ── CNS Query (for platform governance transparency) ────────────────

    #[tool(
        description = "Query CNS ν-events by namespace prefix within a time window. Returns structured event data for governance transparency reporting and consent auditing."
    )]
    pub async fn cns_query(&self, Parameters(req): Parameters<CnsQueryRequest>) -> String {
        execute_tool(self, "cns_query", async {
            let Some(ref store) = self.nu_event_store else {
                return Err(McpToolError::permission_denied(
                    "NuEventStore not available",
                ));
            };
            let window_secs = req.window_seconds.unwrap_or(3600);
            let limit = req.limit.unwrap_or(100) as u64;
            let since = chrono::Utc::now() - chrono::Duration::seconds(window_secs as i64);
            let config = hkask_storage::DecayConfig::default();

            let weighted = store
                .replay_weighted(since, limit, &config)
                .map_err(|e| McpToolError::internal(format!("CNS query failed: {e}")))?;
            let total_count = weighted.len();
            let filtered: Vec<serde_json::Value> = weighted
                .into_iter()
                .filter(|we| {
                    if let Some(ref ns) = req.namespace {
                        we.event.span.namespace.as_str().starts_with(ns)
                    } else {
                        true
                    }
                })
                .take(req.limit.unwrap_or(100))
                .map(|we| {
                    json!({
                        "timestamp": we.event.timestamp.to_rfc3339(),
                        "namespace": we.event.span.namespace.as_str(),
                        "path": we.event.span.path,
                        "phase": format!("{:?}", we.event.phase),
                        "weight": we.weight,
                        "observation": we.event.observation,
                    })
                })
                .collect();

            let namespace_info = req.namespace.as_deref().unwrap_or("all");
            CnsSpan::Tool {
                subsystem: hkask_types::cns::ToolSubsystem::Curator,
            }
            .emit("cns_query");

            Ok(json!({
                "namespace": namespace_info,
                "window_seconds": window_secs,
                "total_events": total_count,
                "filtered_count": filtered.len(),
                "events": filtered
            }))
        })
        .await
    }

    // ── Token Registry (for consent auditing) ───────────────────────────

    #[tool(
        description = "List all DelegationTokens within a time window. Supports filtering by issuer or recipient WebID. Returns structured token data for consent auditing and anomaly detection."
    )]
    pub async fn list_tokens(&self, Parameters(req): Parameters<TokenListRequest>) -> String {
        execute_tool(self, "list_tokens", async {
            let Some(ref registry) = self.token_registry else {
                return Err(McpToolError::permission_denied(
                    "TokenRegistry not available",
                ));
            };
            let window_secs = req.window_seconds.unwrap_or(86400);
            let since = chrono::Utc::now() - chrono::Duration::seconds(window_secs as i64);

            let tokens = if let Some(ref issuer) = req.issuer {
                let wid = WebID::from_string(issuer);
                registry.query_by_issuer(&wid, since)
            } else if let Some(ref recipient) = req.recipient {
                let wid = WebID::from_string(recipient);
                registry.query_by_recipient(&wid, since)
            } else {
                registry.query_all(since)
            }
            .map_err(|e| McpToolError::internal(format!("Token query failed: {e}")))?;

            let serialized: Vec<serde_json::Value> = tokens
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "resource": format!("{:?}", t.resource),
                        "resource_id": t.resource_id,
                        "action": format!("{:?}", t.action),
                        "delegated_from": t.delegated_from.to_string(),
                        "delegated_to": t.delegated_to.to_string(),
                        "expires_at": t.expires_at,
                        "attenuation_level": t.attenuation_level,
                    })
                })
                .collect();

            CnsSpan::Tool {
                subsystem: hkask_types::cns::ToolSubsystem::Curator,
            }
            .emit("list_tokens");

            Ok(json!({
                "window_seconds": window_secs,
                "count": serialized.len(),
                "tokens": serialized
            }))
        })
        .await
    }
}

impl hkask_mcp::server::ToolContext for CuratorServer {
    fn webid(&self) -> &hkask_types::WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── Server startup ─────────────────────────────────────────────────────

pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    hkask_mcp::run_server(
        SERVER_NAME,
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::server::ServerContext| {
            let (escalation_queue, nu_event_store, episodic, semantic) = open_curator_stores(&ctx);
            Ok(CuratorServer {
                webid: ctx.webid,
                replicant: replicant.clone(),
                daemon: daemon_client.clone(),
                escalation_queue,
                nu_event_store,
                episodic,
                semantic,
            })
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_CURATOR_DB",
                "Path to the Curator's SQLCipher database",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "SQLCipher encryption passphrase",
            ),
        ],
    )
    .await
}

#[allow(clippy::type_complexity)]
fn open_curator_stores(
    ctx: &hkask_mcp::server::ServerContext,
) -> (
    Option<Arc<hkask_storage::EscalationQueue>>,
    Option<Arc<hkask_storage::NuEventStore>>,
    Option<hkask_memory::EpisodicMemory>,
    Option<Arc<hkask_memory::SemanticMemory>>,
) {
    let curator_db_path = ctx
        .credentials
        .get("HKASK_CURATOR_DB")
        .cloned()
        .unwrap_or_else(|| {
            let p = hkask_types::agent_paths::agent_pod_db("curator");
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            p.to_string_lossy().to_string()
        });

    let db = match ctx.credentials.get("HKASK_DB_PASSPHRASE") {
        Some(pw) => match hkask_storage::open_or_repair(&curator_db_path, pw) {
            Ok(db) => Some(db),
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.curator", error = %e, "Failed to open curator DB");
                None
            }
        },
        None => {
            tracing::warn!(target: "hkask.mcp.curator", "HKASK_DB_PASSPHRASE not set");
            None
        }
    };
    let Some(db) = db else {
        return (None, None, None, None);
    };

    let conn = db.conn_arc();
    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
    let conn2 = db.conn_arc();
    let triple_store2 = hkask_storage::TripleStore::new(Arc::clone(&conn2));
    let embedding_store = hkask_storage::EmbeddingStore::new(conn2);
    let conn3 = db.conn_arc();

    let escalation_queue = match hkask_storage::EscalationQueue::new(Arc::clone(&conn)) {
        Ok(q) => Some(Arc::new(q)),
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.curator", error = %e, "Failed to create EscalationQueue");
            None
        }
    };
    let nu_event_store = Some(Arc::new(hkask_storage::NuEventStore::new(conn3)));
    let episodic = hkask_memory::EpisodicMemory::new(triple_store);
    let semantic = Arc::new(hkask_memory::SemanticMemory::new(
        triple_store2,
        embedding_store,
    ));

    (
        escalation_queue,
        nu_event_store,
        Some(episodic),
        Some(semantic),
    )
}
