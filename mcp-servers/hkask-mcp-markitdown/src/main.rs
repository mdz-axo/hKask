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
//! - `OKAPI_BASE_URL` — Legacy; maps to `OM_BASE_URL` if unset
//!
//! # Architecture
//!
//! This server fills the OCR gap in `hkask-mcp-doc-knowledge`. For born-digital PDFs,
//! `pdf-extract` returns embedded text. For scanned/image-based PDFs, this server
//! falls back to vision OCR via the inference router, sending the PDF bytes (base64)
//! directly to a vision-capable model.
//!
//! No Python dependency — this is pure Rust, using the hkask-inference path.

use hkask_inference::InferenceConfig;
use hkask_mcp_markitdown::tools::MarkitdownServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-markitdown",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let ocr_model = ctx
                .credentials
                .get("HKASK_OCR_MODEL")
                .cloned();
            let inference_config = InferenceConfig::from_env();
            MarkitdownServer::new(ctx.webid, ocr_model, &inference_config.ollama_base_url)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_OCR_MODEL",
                "Vision model for OCR (must exist in inference catalog). Required for OCR functionality.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OM_BASE_URL",
                "Ollama base URL (default: http://127.0.0.1:11434). Also reads OKAPI_BASE_URL as legacy fallback.",
            ),
        ],
    )
    .await
}
