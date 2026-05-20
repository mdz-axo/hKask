//! hKask MCP FMP — Financial Modeling Prep API for fundamental analysis

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    transport::stdio,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::Parameters;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use secrecy::Secret;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const FMP_API_BASE: &str = "https://financialmodelingprep.com/api/v3";

/// FMP server implementation
pub struct FmpServer {
    tool_router: ToolRouter<FmpServer>,
    client: Client,
    api_key: Option<Secret<String>>,
}

impl FmpServer {
    pub fn new() -> Self {
        let api_key = std::env::var("FMP_API_KEY").ok().map(Secret::new);
        let client = Client::builder().build().unwrap_or_default();

        Self {
            tool_router: Self::tool_router(),
            client,
            api_key,
        }
    }

    fn get_api_url(&self, endpoint: &str) -> String {
        if let Some(key) = &self.api_key {
            format!("{}/{}?apikey={}", FMP_API_BASE, endpoint, key.expose_secret())
        } else {
            format!("{}/{}", FMP_API_BASE, endpoint)
        }
    }
}

#[tool_router(server_handler)]
impl FmpServer {
    #[tool(description = "Ping the FMP server")]
    async fn fmp_ping(&self) -> String {
        serde_json::json!({
            "status": "ok",
            "server": "hkask-mcp-fmp",
            "version": SERVER_VERSION,
            "api_key_configured": self.api_key.is_some()
        }).to_string()
    }

    #[tool(description = "Get company profile for a stock symbol")]
    async fn fmp_company_profile(&self, symbol: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let url = self.get_api_url(&format!("profile/{}", symbol));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get stock quote for a symbol")]
    async fn fmp_quote(&self, symbol: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let url = self.get_api_url(&format!("quote/{}", symbol));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get income statement for a symbol")]
    async fn fmp_income_statement(&self, symbol: String, limit: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let limit = limit.unwrap_or(1);
        let url = self.get_api_url(&format!("income-statement/{}?limit={}", symbol, limit));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get balance sheet for a symbol")]
    async fn fmp_balance_sheet(&self, symbol: String, limit: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let limit = limit.unwrap_or(1);
        let url = self.get_api_url(&format!("balance-sheet-statement/{}?limit={}", symbol, limit));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get cash flow statement for a symbol")]
    async fn fmp_cash_flow_statement(&self, symbol: String, limit: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let limit = limit.unwrap_or(1);
        let url = self.get_api_url(&format!("cash-flow-statement/{}?limit={}", symbol, limit));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get key metrics for a symbol")]
    async fn fmp_key_metrics(&self, symbol: String, limit: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let limit = limit.unwrap_or(1);
        let url = self.get_api_url(&format!("key-metrics-ttm/{}?limit={}", symbol, limit));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get historical stock prices")]
    async fn fmp_historical_price(&self, symbol: String, from: Option<String>, to: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let from = from.unwrap_or_else(|| "2024-01-01".to_string());
        let to = to.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
        let url = self.get_api_url(&format!("historical-price-full/{}?from={}&to={}", symbol, from, to));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Search for stocks by name or symbol")]
    async fn fmp_search(&self, query: String, limit: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let limit = limit.unwrap_or(10);
        let url = self.get_api_url(&format!("search?query={}&limit={}", query, limit));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get analyst estimates for a symbol")]
    async fn fmp_analyst_estimates(&self, symbol: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let url = self.get_api_url(&format!("analyst-estimates/{}", symbol));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Get discounted cash flow (DCF) valuation")]
    async fn fmp_dcf(&self, symbol: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FMP_API_KEY not configured" }).to_string();
        }

        let url = self.get_api_url(&format!("dcf/{}", symbol));
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }
}

impl FmpServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = FmpServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-fmp MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
