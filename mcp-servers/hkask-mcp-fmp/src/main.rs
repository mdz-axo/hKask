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

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

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

    async fn get_with_params(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, String> {
        let url = format!("{BASE_URL}{path}");
        let mut query: Vec<(&str, &str)> = params.to_vec();
        query.push(("apikey", &self.api_key));

        let resp = self
            .client
            .get(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API returned {status}: {body}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))
    }

    fn err_json(msg: &str) -> String {
        serde_json::json!({ "error": msg }).to_string()
    }
}

#[tool_router(server_handler)]
impl FmpServer {
    #[tool(description = "Ping FMP API")]
    async fn fmp_ping(&self) -> String {
        match self.get_with_params("/profile", &[("symbol", "AAPL")]).await {
            Ok(_) => serde_json::json!({
                "status": "ok",
                "message": "FMP API is reachable"
            })
            .to_string(),
            Err(e) => serde_json::json!({
                "status": "not_ok",
                "error": e
            })
            .to_string(),
        }
    }

    #[tool(description = "Get company profile")]
    async fn fmp_company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        match self.get_with_params("/profile", &[("symbol", &symbol)]).await {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get stock quote")]
    async fn fmp_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        match self.get_with_params("/quote", &[("symbol", &symbol)]).await {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get income statement")]
    async fn fmp_income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let limit_str = limit.unwrap_or(5).to_string();
        match self
            .get_with_params("/income-statement", &[("symbol", &symbol), ("limit", &limit_str)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get balance sheet")]
    async fn fmp_balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let limit_str = limit.unwrap_or(5).to_string();
        match self
            .get_with_params("/balance-sheet-statement", &[("symbol", &symbol), ("limit", &limit_str)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get cash flow statement")]
    async fn fmp_cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let limit_str = limit.unwrap_or(5).to_string();
        match self
            .get_with_params("/cash-flow-statement", &[("symbol", &symbol), ("limit", &limit_str)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get key metrics")]
    async fn fmp_key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let limit_str = limit.unwrap_or(5).to_string();
        match self
            .get_with_params("/key-metrics", &[("symbol", &symbol), ("limit", &limit_str)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get historical price data")]
    async fn fmp_historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        match self
            .get_with_params("/historical-price-full", &[("symbol", &symbol), ("from", &from), ("to", &to)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Search for symbols")]
    async fn fmp_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        let limit_str = limit.unwrap_or(10).to_string();
        match self
            .get_with_params("/search-name", &[("query", &query), ("limit", &limit_str)])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get analyst estimates")]
    async fn fmp_analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        match self
            .get_with_params("/analyst-estimates", &[("symbol", &symbol), ("period", "annual")])
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn fmp_dcf(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        match self.get_with_params("/discounted-cash-flow", &[("symbol", &symbol)]).await {
            Ok(v) => v.to_string(),
            Err(e) => Self::err_json(&e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = FmpServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-fmp started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
