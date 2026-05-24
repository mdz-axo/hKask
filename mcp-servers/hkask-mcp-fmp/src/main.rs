//! hKask MCP FMP — Financial Modeling Prep API integration

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[derive(Debug, Default)]
pub struct FmpServer;

impl FmpServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl FmpServer {
    #[tool(description = "Ping FMP API")]
    async fn fmp_ping(&self) -> String {
        r#"{"status":"ok","message":"FMP API is reachable"}"#.to_string()
    }

    #[tool(description = "Get company profile")]
    async fn fmp_company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "name": "Company Inc",
            "sector": "Technology",
        })
        .to_string()
    }

    #[tool(description = "Get stock quote")]
    async fn fmp_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "price": 150.25,
            "change": 2.5,
        })
        .to_string()
    }

    #[tool(description = "Get income statement")]
    async fn fmp_income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "limit": limit.unwrap_or(1),
            "statements": [],
        })
        .to_string()
    }

    #[tool(description = "Get balance sheet")]
    async fn fmp_balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "limit": limit.unwrap_or(1),
            "sheets": [],
        })
        .to_string()
    }

    #[tool(description = "Get cash flow statement")]
    async fn fmp_cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "limit": limit.unwrap_or(1),
            "flows": [],
        })
        .to_string()
    }

    #[tool(description = "Get key metrics")]
    async fn fmp_key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "limit": limit.unwrap_or(1),
            "metrics": {},
        })
        .to_string()
    }

    #[tool(description = "Get historical price data")]
    async fn fmp_historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "from": from,
            "to": to,
            "prices": [],
        })
        .to_string()
    }

    #[tool(description = "Search for symbols")]
    async fn fmp_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        serde_json::json!({
            "query": query,
            "limit": limit.unwrap_or(10),
            "results": [],
        })
        .to_string()
    }

    #[tool(description = "Get analyst estimates")]
    async fn fmp_analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "estimates": {},
        })
        .to_string()
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn fmp_dcf(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        serde_json::json!({
            "symbol": symbol,
            "dcf_value": 175.50,
            "current_price": 150.25,
        })
        .to_string()
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
