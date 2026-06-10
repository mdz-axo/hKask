//! hKask MCP Markitdown — Document format conversion and OCR MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `markitdown_convert` — Detect format, extract text, OCR fallback for scanned docs
//! - `markitdown_detect_format` — Detect document format from path/extension
//! - `markitdown_ocr` — Explicitly OCR a document using local vision model
//!
//! # Environment Variables
//!
//! - `HKASK_OCR_MODEL` — Vision model for OCR (must be available in Okapi catalog).
//!   Use `inference_models` to discover available models. No default — must be set
//!   for OCR functionality. If unset, OCR requests return an error with guidance.
//! - `OKAPI_BASE_URL` — Okapi API base URL (default: "http://127.0.0.1:11434")
//!
//! # Architecture
//!
//! This server fills the OCR gap in `hkask-mcp-doc-knowledge`. For born-digital PDFs,
//! `pdf-extract` returns embedded text. For scanned/image-based PDFs, this server
//! falls back to vision OCR via Okapi, sending the PDF bytes (base64) directly to
//! a vision-capable model.
//!
//! No Python dependency — this is pure Rust, using the Okapi inference path.

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
            let okapi_base_url = ctx
                .credentials
                .get("OKAPI_BASE_URL")
                .cloned()
                .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
            MarkitdownServer::new(ctx.webid, ocr_model, &okapi_base_url)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_OCR_MODEL",
                "Vision model for OCR (must exist in Okapi catalog). Required for OCR functionality.",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "OKAPI_BASE_URL",
                "Okapi API base URL (default: http://127.0.0.1:11434)",
            ),
        ],
    )
    .await
}
