//! hKask MCP Markitdown — Document format conversion and OCR MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `markitdown_convert` — Detect format, extract text, OCR fallback for scanned docs
//! - `markitdown_detect_format` — Detect document format from path/extension
//! - `markitdown_ocr` — Explicitly OCR a document using local vision model
//!
//! # Environment Variables
//!
//! - `HKASK_OCR_MODEL` — Vision model for OCR (must be available in inference catalog).
//!   Use `inference_models` to discover available models. No default — must be set
//!   for OCR functionality. If unset, OCR requests return an error with guidance.
//! - `OM_BASE_URL` — Ollama base URL (default: "http://127.0.0.1:11434")
//!
//! # Architecture
//!
//! This server fills the OCR gap in `hkask-mcp-doc-knowledge`. For born-digital PDFs,
//! `pdf-extract` returns embedded text. For scanned/image-based PDFs, this server
//! falls back to vision OCR via the inference router, sending the PDF bytes (base64)
//! directly to a vision-capable model.
//!
//! No Python dependency — this is pure Rust, using the hkask-inference path.

use hkask_inference::{EmbeddingRouter, InferenceConfig};
use hkask_mcp_markitdown::tools::MarkitdownServer;
use hkask_types::ocr::ThresholdConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.markitdown", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-markitdown",
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
            };

            // Build embedding router for semantic cross-validation
            let embedding_router = EmbeddingRouter::new(inference_config.clone());

            MarkitdownServer::new(
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

    let auth = client.auth_query(replicant).await?;
    match auth {
        hkask_mcp::DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.markitdown", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
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

    let assignment = client.assignment_query(replicant, "markitdown").await?;
    match assignment {
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.markitdown", replicant = %replicant, "Replicant assigned to markitdown role");
        }
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the markitdown MCP role. Use 'kask replicant assign {} markitdown' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.markitdown", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
