//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Starts an MCP server over stdio exposing 9 tools:
//! - `docproc_convert` — Extract text from documents with OCR fallback
//! - `docproc_ocr` — Explicit OCR using vision model
//! - `docproc_chunk` — Chunk text or documents into passages (single or multi-tier), auto-indexes
//! - `docproc_extract_triples` — Extract RDF triples from text via LLM
//! - `docproc_embed` — Generate embedding vectors for passages or triples
//! - `docproc_generate_qa` — Generate QA pairs from text via LLM
//! - `docproc_cache` — Cache processed text for reference
//! - `docproc_query` — Search indexed passages by natural language query, optionally generate answer
//! - `docproc_clear_index` — Reset the vector index for a new document set
//!
//! # Environment Variables
//!
//! - `HKASK_OCR_MODEL` — Vision model for OCR (must be available in inference catalog).
//!   Use `inference_models` to discover available models. No default — must be set
//!   for OCR functionality. If unset, OCR requests return an error with guidance.
//! - `OM_BASE_URL` — Ollama base URL (default: "http://127.0.0.1:11434")

use hkask_inference::{EmbeddingRouter, InferenceConfig};
use hkask_mcp_docproc::server::DocProcServer;
use hkask_types::ocr::ThresholdConfig;

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

            DocProcServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                ocr_model,
                inference_config,
                ocr_thresholds,
                Some(embedding_router),
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
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "docproc", &[]).await?;
    tracing::info!(target: "hkask.mcp.docproc", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
