//! hKask MCP FMP — Financial Modeling Prep API integration
//!
//! Uses the FMP /stable/ API endpoints (v3 endpoints deprecated Aug 2025).
//! Tools:
//! - `fmp_ping` — API reachability check
//! - `fmp_company_profile` — Company profile by symbol
//! - `fmp_quote` — Real-time stock quote
//! - `fmp_income_statement` — Income statements
//! - `fmp_balance_sheet` — Balance sheet statements
//! - `fmp_cash_flow_statement` — Cash flow statements
//! - `fmp_key_metrics` — Key financial metrics
//! - `fmp_historical_price` — Historical price data
//! - `fmp_search` — Symbol search
//! - `fmp_analyst_estimates` — Analyst estimates
//! - `fmp_dcf` — Discounted cash flow analysis

use hkask_mcp::server::{CredentialRequirement, McpToolError, McpToolOutput, run_stdio_server};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::time::Instant;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const BASE_URL: &str = "https://financialmodelingprep.com/stable";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolRequest {
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolLimitRequest {
    pub symbol: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HistoricalRequest {
    pub symbol: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u32>,
}

fn classify_api_error(status: reqwest::StatusCode, body: &str) -> McpToolError {
    let msg = format!("FMP API returned {}: {}", status, body.trim());
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

async fn fmp_get(
    client: &reqwest::Client,
    path: &str,
    api_key: &str,
    params: &[(&str, &str)],
) -> Result<Value, McpToolError> {
    let url = format!("{BASE_URL}{path}");
    let mut query: Vec<(&str, &str)> = params.to_vec();
    query.push(("apikey", api_key));

    let resp = client
        .get(&url)
        .query(&query)
        .send()
        .await
        .map_err(|e| McpToolError::unavailable(format!("request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_api_error(status, &body));
    }

    serde_json::from_str(&body)
        .map_err(|e| McpToolError::internal(format!("failed to parse response: {e}")))
}

#[derive(Debug)]
pub struct FmpServer {
    client: reqwest::Client,
    api_key: String,
}

impl Default for FmpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl FmpServer {
    pub fn new() -> Self {
        let api_key = std::env::var("HKASK_FMP_API_KEY")
            .expect("HKASK_FMP_API_KEY environment variable must be set");
        let client = reqwest::Client::new();
        Self { client, api_key }
    }
}

#[tool_router(server_handler)]
impl FmpServer {
    #[tool(description = "Ping FMP API")]
    async fn fmp_ping(&self) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/profile",
            &self.api_key,
            &[("symbol", "AAPL")],
        )
        .await
        {
            Ok(_) => McpToolOutput::with_timing(
                serde_json::json!({
                    "status": "ok",
                    "message": "FMP API is reachable"
                }),
                start,
            )
            .to_json_string(),
            Err(e) => McpToolOutput::with_timing(
                serde_json::json!({
                    "status": "not_ok",
                    "error": e.message,
                }),
                start,
            )
            .to_json_string(),
        }
    }

    #[tool(description = "Get company profile")]
    async fn fmp_company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/profile",
            &self.api_key,
            &[("symbol", &symbol)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get stock quote")]
    async fn fmp_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/quote",
            &self.api_key,
            &[("symbol", &symbol)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get income statement")]
    async fn fmp_income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let start = Instant::now();
        let limit_str = limit.unwrap_or(5).to_string();
        match fmp_get(
            &self.client,
            "/income-statement",
            &self.api_key,
            &[("symbol", &symbol), ("limit", &limit_str)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get balance sheet")]
    async fn fmp_balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let start = Instant::now();
        let limit_str = limit.unwrap_or(5).to_string();
        match fmp_get(
            &self.client,
            "/balance-sheet-statement",
            &self.api_key,
            &[("symbol", &symbol), ("limit", &limit_str)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get cash flow statement")]
    async fn fmp_cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let start = Instant::now();
        let limit_str = limit.unwrap_or(5).to_string();
        match fmp_get(
            &self.client,
            "/cash-flow-statement",
            &self.api_key,
            &[("symbol", &symbol), ("limit", &limit_str)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get key metrics")]
    async fn fmp_key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let start = Instant::now();
        let limit_str = limit.unwrap_or(5).to_string();
        match fmp_get(
            &self.client,
            "/key-metrics",
            &self.api_key,
            &[("symbol", &symbol), ("limit", &limit_str)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get historical price data")]
    async fn fmp_historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/historical-price-full",
            &self.api_key,
            &[("symbol", &symbol), ("from", &from), ("to", &to)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Search for symbols")]
    async fn fmp_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        let start = Instant::now();
        let limit_str = limit.unwrap_or(10).to_string();
        match fmp_get(
            &self.client,
            "/search-name",
            &self.api_key,
            &[("query", &query), ("limit", &limit_str)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get analyst estimates")]
    async fn fmp_analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/analyst-estimates",
            &self.api_key,
            &[("symbol", &symbol), ("period", "annual")],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn fmp_dcf(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let start = Instant::now();
        match fmp_get(
            &self.client,
            "/discounted-cash-flow",
            &self.api_key,
            &[("symbol", &symbol)],
        )
        .await
        {
            Ok(v) => McpToolOutput::with_timing(v, start).to_json_string(),
            Err(e) => e.to_json_string(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-fmp",
        SERVER_VERSION,
        FmpServer::new(),
        vec![CredentialRequirement::required(
            "HKASK_FMP_API_KEY",
            "Financial Modeling Prep API key",
        )],
    )
    .await
}
