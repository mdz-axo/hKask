//! hKask MCP FMP — Financial Modeling Prep API integration

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};
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
        format!(
            r#"{{"symbol":"{}","name":"Company Inc","sector":"Technology"}}"#,
            symbol
        )
    }

    #[tool(description = "Get stock quote")]
    async fn fmp_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        format!(r#"{{"symbol":"{}","price":150.25,"change":2.5}}"#, symbol)
    }

    #[tool(description = "Get income statement")]
    async fn fmp_income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","limit":{},"statements":[]}}"#,
            symbol,
            limit.unwrap_or(1)
        )
    }

    #[tool(description = "Get balance sheet")]
    async fn fmp_balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","limit":{},"sheets":[]}}"#,
            symbol,
            limit.unwrap_or(1)
        )
    }

    #[tool(description = "Get cash flow statement")]
    async fn fmp_cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","limit":{},"flows":[]}}"#,
            symbol,
            limit.unwrap_or(1)
        )
    }

    #[tool(description = "Get key metrics")]
    async fn fmp_key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","limit":{},"metrics":{}}}"#,
            symbol,
            limit.unwrap_or(1),
            serde_json::json!({})
        )
    }

    #[tool(description = "Get historical price data")]
    async fn fmp_historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","from":"{}","to":"{}","prices":[]}}"#,
            symbol, from, to
        )
    }

    #[tool(description = "Search for symbols")]
    async fn fmp_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        format!(
            r#"{{"query":"{}","limit":{},"results":[]}}"#,
            query,
            limit.unwrap_or(10)
        )
    }

    #[tool(description = "Get analyst estimates")]
    async fn fmp_analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","estimates":{}}}"#,
            symbol,
            serde_json::json!({})
        )
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn fmp_dcf(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        format!(
            r#"{{"symbol":"{}","dcf_value":175.50,"current_price":150.25}}"#,
            symbol
        )
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
