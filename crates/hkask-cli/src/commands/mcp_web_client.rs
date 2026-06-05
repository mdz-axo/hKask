//! MCP client for style corpus embedding.
//!
//! Spawns `hkask-mcp-web` as a child process via rmcp stdio transport
//! and calls `web_extract` to download public domain texts with proper
//! User-Agent, rate limiting, caching, and SSRF validation — all
//! provided by the web MCP server infrastructure.

use anyhow::{Context, Result};
use rmcp::model::CallToolRequestParams;
use rmcp::service::ServiceExt;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use serde_json::json;
use std::path::Path;
use tokio::process::Command;

/// MCP client that wraps `hkask-mcp-web`'s `web_extract` tool.
///
/// Spawns the web server as a child process, connects via rmcp stdio
/// transport, and calls `web_extract` for each URL. The server handles:
/// - Proper User-Agent (`hkask-mcp-web/<version>`)
/// - Rate limiting (per-tool rate limiter)
/// - Response caching (in-memory LRU cache)
/// - SSRF validation (URL validation at tool and provider boundaries)
/// - Error classification (HTTP status → McpToolError mapping)
pub struct McpWebClient {
    /// The rmcp client handle. None means not yet started.
    client: Option<rmcp::service::client::DynClient>,
}

impl McpWebClient {
    /// Create a new client (not yet connected).
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Start the `hkask-mcp-web` server and connect via stdio transport.
    ///
    /// The server binary must be on PATH or specified via
    /// `HKASK_MCP_WEB_BIN` environment variable.
    pub async fn start(&mut self) -> Result<()> {
        let bin_path =
            std::env::var("HKASK_MCP_WEB_BIN").unwrap_or_else(|_| "hkask-mcp-web".to_string());

        let client = ()
            .into_dyn()
            .serve(TokioChildProcess::new(Command::new(&bin_path).configure(
                |cmd| {
                    // Inherit environment for API keys (Brave, Tavily, etc.)
                    // The MCP server reads its own env vars at startup.
                    cmd.env("HKASK_MCP_WEB_QUIET", "1");
                },
            )?))
            .await
            .with_context(|| format!("Failed to start MCP web server: {}", bin_path))?;

        // Wait for initialization handshake
        let server_info = client.peer_info();
        tracing::info!(
            target: "embed_corpus.mcp",
            server = ?server_info,
            "Connected to hkask-mcp-web"
        );

        self.client = Some(client);
        Ok(())
    }

    /// Extract content from a URL via the `web_extract` tool.
    ///
    /// Routes through `hkask-mcp-web`'s ProviderPool:
    /// - RawFetchProvider for plain text / simple HTML
    /// - FirecrawlProvider for JS-heavy pages (if FIRECRAWL_API_KEY set)
    /// - BrowserbaseProvider for browser rendering (if BROWSERBASE_API_KEY set)
    ///
    /// For Project Gutenberg plain text files, RawFetchProvider is the
    /// default provider. It sets proper User-Agent and handles plain
    /// text responses correctly.
    pub async fn web_extract(&self, url: &str) -> Result<String> {
        let client = self
            .client
            .as_ref()
            .context("MCP web client not started — call start() first")?;

        let result = client
            .call_tool(
                CallToolRequestParams::new("web_extract").with_arguments(
                    json!({
                        "url": url,
                        "format": "markdown",
                        "main_content_only": true
                    })
                    .as_object()
                    .context("Failed to build web_extract arguments")?
                    .clone(),
                ),
            )
            .await
            .with_context(|| format!("web_extract call failed for {}", url))?;

        // Parse the MCP tool result to extract the content field.
        // The web_extract tool returns an ExtractOutput JSON:
        // { "url": "...", "format": "markdown", "content": "...", "metadata": {...} }
        let content = parse_extract_result(&result, url)?;
        Ok(content)
    }

    /// Shut down the MCP server process.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            client.cancel().await?;
        }
        Ok(())
    }
}

/// Parse the MCP tool call result from `web_extract`.
///
/// The MCP protocol returns tool results as a list of content items
/// (text, image, etc.). For `web_extract`, the result is a single text
/// content containing a JSON object with a `content` field.
fn parse_extract_result(result: &rmcp::model::CallToolResult, url: &str) -> Result<String> {
    // The result contains a list of content items.
    // For web_extract, we expect a single text content with the extracted text.
    if result.content.is_empty() {
        anyhow::bail!("web_extract returned empty content for {}", url);
    }

    let first_content = &result.content[0];

    // Try to extract text from the content item
    let raw_text = match first_content {
        rmcp::model::RawContent::Text(text_content) => &text_content.text,
        rmcp::model::RawContent::Image(_) => {
            anyhow::bail!(
                "web_extract returned image content for {} (expected text)",
                url
            );
        }
        rmcp::model::RawContent::Resource(_) => {
            anyhow::bail!(
                "web_extract returned resource content for {} (expected text)",
                url
            );
        }
        _ => {
            anyhow::bail!("web_extract returned unknown content type for {}", url);
        }
    };

    // The raw text might be a JSON object from the ExtractOutput,
    // or it might be the raw extracted text directly.
    // Try parsing as JSON first; if that fails, return the raw text.
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(raw_text) {
        if let Some(content) = json_value.get("content").and_then(|c| c.as_str()) {
            return Ok(content.to_string());
        }
        // JSON but no "content" field — return the full text
        return Ok(raw_text.clone());
    }

    // Not JSON — this is the raw extracted text (plain text from Gutenberg)
    Ok(raw_text.clone())
}

/// Download text from a URL via the `hkask-mcp-web` MCP server.
///
/// Uses file-based caching: if a cached file exists at `cache_dir/{slug}.txt`,
/// returns the cached content. Otherwise, calls `web_extract` via MCP
/// and caches the result.
///
/// This function handles the full lifecycle: start server → extract → cache → shutdown.
/// For batch downloads, prefer creating an `McpWebClient` once and calling
/// `web_extract` for each URL, then shutting down at the end.
pub async fn download_via_mcp(url: &str, slug: &str, cache_dir: &Path) -> Result<String> {
    let cache_path = cache_dir.join(format!("{}.txt", slug));

    // Check cache first
    if cache_path.exists() {
        tracing::info!(target: "embed_corpus.mcp", slug = %slug, "Using cached download");
        return std::fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read cache {}", cache_path.display()));
    }

    // Start MCP server, extract, cache, shutdown
    let mut client = McpWebClient::new();
    client.start().await?;

    let content = client.web_extract(url).await?;

    // Cache the result
    if let Err(e) = std::fs::write(&cache_path, &content) {
        tracing::warn!(
            target: "embed_corpus.mcp",
            path = %cache_path.display(),
            error = %e,
            "Failed to cache download"
        );
    }

    client.shutdown().await?;

    Ok(content)
}
