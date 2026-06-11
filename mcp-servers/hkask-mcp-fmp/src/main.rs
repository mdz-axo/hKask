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

use hkask_mcp::server::{McpToolError, ToolSpanGuard, classify_http_error, validate_identifier};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

mod analysis;

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
        return Err(classify_http_error("FMP", status, &body));
    }

    serde_json::from_str(&body)
        .map_err(|e| McpToolError::internal(format!("failed to parse response: {e}")))
}

fn validate_symbol(symbol: &str) -> Result<(), McpToolError> {
    validate_identifier("symbol", symbol, 16)
}

pub struct FmpServer {
    webid: WebID,
    client: reqwest::Client,
    api_key: String,
}

impl FmpServer {
    pub fn new(webid: WebID, api_key: String) -> Result<Self, anyhow::Error> {
        let client = reqwest::Client::new();
        Ok(Self {
            webid,
            client,
            api_key,
        })
    }
}

#[tool_router(server_handler)]
impl FmpServer {
    #[tool(description = "Ping FMP API")]
    async fn fmp_ping(&self) -> String {
        let span = ToolSpanGuard::new("fmp_ping", &self.webid);
        span.finish(
            fmp_get(
                &self.client,
                "/profile",
                &self.api_key,
                &[("symbol", "AAPL")],
            )
            .await
            .map(|_| {
                serde_json::json!({
                    "status": "ok",
                    "message": "FMP API is reachable"
                })
            }),
        )
    }

    #[tool(description = "Get company profile")]
    async fn fmp_company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_company_profile", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            fmp_get(
                &self.client,
                "/profile",
                &self.api_key,
                &[("symbol", &symbol)],
            )
            .await,
        )
    }

    #[tool(description = "Get stock quote")]
    async fn fmp_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_quote", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            fmp_get(
                &self.client,
                "/quote",
                &self.api_key,
                &[("symbol", &symbol)],
            )
            .await,
        )
    }

    #[tool(description = "Get income statement")]
    async fn fmp_income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_income_statement", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            fmp_get(
                &self.client,
                "/income-statement",
                &self.api_key,
                &[("symbol", &symbol), ("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get balance sheet")]
    async fn fmp_balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_balance_sheet", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            fmp_get(
                &self.client,
                "/balance-sheet-statement",
                &self.api_key,
                &[("symbol", &symbol), ("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get cash flow statement")]
    async fn fmp_cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_cash_flow_statement", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            fmp_get(
                &self.client,
                "/cash-flow-statement",
                &self.api_key,
                &[("symbol", &symbol), ("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get key metrics")]
    async fn fmp_key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_key_metrics", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            fmp_get(
                &self.client,
                "/key-metrics",
                &self.api_key,
                &[("symbol", &symbol), ("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get historical price data")]
    async fn fmp_historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_historical_price", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            fmp_get(
                &self.client,
                "/historical-price-full",
                &self.api_key,
                &[("symbol", &symbol), ("from", &from), ("to", &to)],
            )
            .await,
        )
    }

    #[tool(description = "Search for symbols")]
    async fn fmp_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_search", &self.webid);
        if query.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("query must not be empty").to_json_string(),
            );
        }
        let limit_str = limit.unwrap_or(10).to_string();
        span.finish(
            fmp_get(
                &self.client,
                "/search-name",
                &self.api_key,
                &[("query", &query), ("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get analyst estimates")]
    async fn fmp_analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_analyst_estimates", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            fmp_get(
                &self.client,
                "/analyst-estimates",
                &self.api_key,
                &[("symbol", &symbol), ("period", "annual")],
            )
            .await,
        )
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn fmp_dcf(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_dcf", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            fmp_get(
                &self.client,
                "/discounted-cash-flow",
                &self.api_key,
                &[("symbol", &symbol)],
            )
            .await,
        )
    }

    #[tool(
        description = "Analyze competitive moat using MAIA framework: gross margin stability and working capital market power signal"
    )]
    async fn fmp_moat_check(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fmp_moat_check", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }

        // Fetch 10 years of key metrics for gross margin stability analysis
        let limit = "10";
        let metrics_result = fmp_get(
            &self.client,
            "/key-metrics",
            &self.api_key,
            &[("symbol", &symbol), ("limit", limit)],
        )
        .await;

        let metrics = match metrics_result {
            Ok(v) => v,
            Err(e) => return span.error(e.kind, e.to_json_string()),
        };

        let gross_margins = analysis::extract_gross_margins(&metrics);
        if gross_margins.is_empty() {
            return span.ok_json(serde_json::json!({
                "symbol": symbol,
                "moat": "insufficient_data",
                "reason": "No gross margin data available for this symbol",
            }));
        }

        let margin_values: Vec<f64> = gross_margins.iter().map(|(_, m)| *m).collect();
        let stability = analysis::gross_margin_stability(&margin_values);

        let wc_data = analysis::extract_wc_days(&metrics);
        let (wc_spread, dpo, dso) = match wc_data {
            Some((dpo_val, dso_val)) => (
                analysis::working_capital_spread(dpo_val, dso_val),
                Some(dpo_val),
                Some(dso_val),
            ),
            None => (0.0, None, None),
        };

        let wc_label = analysis::wc_signal_label(wc_spread);
        let moat = analysis::classify_moat(stability, wc_spread, gross_margins.len());

        span.ok_json(serde_json::json!({
            "symbol": symbol,
            "moat": moat,
            "margin_stability": stability,
            "gross_margins": gross_margins,
            "working_capital": {
                "spread_days": wc_spread,
                "dpo": dpo,
                "dso": dso,
                "signal": wc_label,
            },
            "data_periods": gross_margins.len(),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-fmp",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let api_key = ctx
                .credentials
                .get("HKASK_FMP_API_KEY")
                .expect("required credential checked by run_stdio_server")
                .clone();
            FmpServer::new(ctx.webid, api_key)
        },
        vec![hkask_mcp::CredentialRequirement::required(
            "HKASK_FMP_API_KEY",
            "Financial Modeling Prep API key",
        )],
    )
    .await
}
