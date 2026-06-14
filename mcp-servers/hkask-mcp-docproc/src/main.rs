//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Starts an MCP server over stdio exposing 8 tools:
//! - `docproc_convert` — Extract text from documents with OCR fallback
//! - `docproc_ocr` — Explicit OCR using vision model
//! - `docproc_chunk` — Chunk text or documents into passages (single or multi-tier)
//! - `docproc_extract_triples` — Extract RDF triples from text via LLM
//! - `docproc_embed` — Generate embedding vectors for passages or triples
//! - `docproc_generate_qa` — Generate QA pairs from text via LLM
//! - `docproc_store_qa` — Store QA items with provenance
//! - `docproc_cache` — Cache processed text for reference
//!
//! # Environment Variables
//!
//! - `HKASK_OCR_MODEL` — Vision model for OCR (must be available in inference catalog).
//!   Use `inference_models` to discover available models. No default — must be set
//!   for OCR functionality. If unset, OCR requests return an error with guidance.
//! - `OM_BASE_URL` — Ollama base URL (default: "http://127.0.0.1:11434")
//! - `HKASK_MEMORY_DB` — Path to per-agent memory database for QA storage
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database

use hkask_inference::{EmbeddingRouter, InferenceConfig};
use hkask_mcp_docproc::server::DocProcServer;
use hkask_types::ocr::ThresholdConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.docproc", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-docproc",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let ocr_model = ctx
                .credentials
                .get("HKASK_OCR_MODEL")
                .cloned();
            let inference_config = InferenceConfig::from_env();

            // OCR thresholds from env vars with sensible defaults
            let ocr_thresholds = ThresholdConfig {
                simple_max: std::env::var("HKASK_OCR_SIMPLE_MAX")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.05),
                moderate_max: std::env::var("HKASK_OCR_MODERATE_MAX")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.15),
                moderate_sample_rate: std::env::var("HKASK_OCR_SAMPLE_RATE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.10),
                tuneable: std::env::var("HKASK_OCR_TUNEABLE")
                    .ok()
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(true),
            };

            // Build embedding router for semantic cross-validation
            let embedding_router = EmbeddingRouter::new(inference_config.clone());

            // Build semantic memory if configured
            let semantic = match ctx.credentials.get("HKASK_MEMORY_DB") {
                Some(path) => {
                    let passphrase =
                        ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                            anyhow::anyhow!("HKASK_MEMORY_DB set but HKASK_DB_PASSPHRASE missing")
                        })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
                    let conn = db.conn_arc();
                    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                    let embedding_store = hkask_storage::EmbeddingStore::new(conn);
                    Some(hkask_memory::SemanticMemory::new(
                        triple_store,
                        embedding_store,
                    ))
                }
                None => None,
            };

            DocProcServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                ocr_model,
                inference_config,
                ocr_thresholds,
                Some(embedding_router),
                semantic,
            )
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_OCR_MODEL",
                "Vision model for OCR (must exist in inference catalog). Required for OCR functionality.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OM_BASE_URL",
                "Ollama base URL (default: http://127.0.0.1:11434).",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_MEMORY_DB",
                "Path to per-agent memory database for QA storage (in-memory if absent)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the database (required if HKASK_MEMORY_DB is set)",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();

    let auth = client.auth_query(replicant).await?;
    match auth {
        hkask_mcp::DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.docproc", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
        }
        hkask_mcp::DaemonResponse::AuthResponse {
            authenticated: false,
            action: Some(ref action),
            ..
        } if action == "prompt_user" => {
            anyhow::bail!(
                "Replicant '{}' is not authenticated. Enter the replicant's passphrase in the hKask terminal.",
                replicant
            );
        }
        other => anyhow::bail!("Unexpected auth response: {:?}", other),
    }

    let assignment = client.assignment_query(replicant, "docproc").await?;
    match assignment {
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.docproc", replicant = %replicant, "Replicant assigned to docproc role");
        }
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the docproc MCP role. Use 'kask pod assign {} docproc' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.docproc", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
