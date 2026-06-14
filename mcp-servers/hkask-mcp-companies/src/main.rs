//! hKask MCP Companies — Dual-provider company financial data (FMP + EODHD)
//!
//! Tools are provider-agnostic: each tool routes to FMP or EODHD based on
//! symbol characteristics, with automatic fallback. EODHD responses are
//! normalized to match FMP format so analysis functions work transparently.
//!
//! Tools:
//! - `ping` — API reachability check
//! - `company_profile` — Company profile by symbol
//! - `stock_quote` — Real-time stock quote
//! - `income_statement` — Income statements
//! - `balance_sheet` — Balance sheet statements
//! - `cash_flow_statement` — Cash flow statements
//! - `key_metrics` — Key financial metrics
//! - `historical_price` — Historical price data
//! - `symbol_search` — Symbol search
//! - `analyst_estimates` — Analyst estimates
//! - `dcf_analysis` — Discounted cash flow analysis
//! - `moat_check` — MAIA competitive moat analysis
//! - `management_scorecard` — MAIA CEO capital allocation scorecard
//! - `working_capital_cycle` — MAIA CFO working capital analysis
//! - `expectations_gap` — MAIA expectations gap analysis

use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_identifier};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

mod analysis;
mod providers;

use providers::companies_get;

// ── Request structs ─────────────────────────────────────────────────

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

// ── Validation ──────────────────────────────────────────────────────

fn validate_symbol(symbol: &str) -> Result<(), McpToolError> {
    // Allow exchange-qualified symbols (e.g., VOD.L, BMW.DE) for EODHD
    validate_identifier("symbol", symbol, 32)
}

// ── Server struct ──────────────────────────────────────────────────

pub struct CompaniesServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<DaemonClient>,
    client: reqwest::Client,
    fmp_api_key: String,
    eodhd_api_key: String,
}

impl CompaniesServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        fmp_api_key: String,
        eodhd_api_key: String,
    ) -> Result<Self, anyhow::Error> {
        let client = reqwest::Client::new();
        Ok(Self {
            webid,
            replicant,
            daemon,
            client,
            fmp_api_key,
            eodhd_api_key,
        })
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool,
                "input": input_summary,
                "outcome": outcome,
                "detail": detail,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.companies.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.companies.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.companies.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

// ── Tools ──────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl CompaniesServer {
    #[tool(description = "Ping company data APIs")]
    async fn ping(&self) -> String {
        let span = ToolSpanGuard::new("ping", &self.webid);
        span.finish(
            companies_get(
                &self.client,
                "company_profile",
                "AAPL",
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[],
            )
            .await
            .map(|_| {
                serde_json::json!({
                    "status": "ok",
                    "message": "Company data APIs are reachable"
                })
            }),
        )
    }

    #[tool(description = "Get company profile")]
    async fn company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("company_profile", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let result = companies_get(
            &self.client,
            "company_profile",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[],
        )
        .await;
        match &result {
            Ok(v) => {
                self.record_experience(
                    "company_profile",
                    &format!("symbol={}", symbol),
                    "success",
                    v.clone(),
                );
            }
            Err(e) => {
                self.record_experience(
                    "company_profile",
                    &format!("symbol={}", symbol),
                    "error",
                    serde_json::json!({"error": e.to_json_string()}),
                );
            }
        }
        span.finish(result)
    }

    #[tool(description = "Get stock quote")]
    async fn stock_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("stock_quote", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let result = companies_get(
            &self.client,
            "stock_quote",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[],
        )
        .await;
        match &result {
            Ok(v) => {
                self.record_experience(
                    "stock_quote",
                    &format!("symbol={}", symbol),
                    "success",
                    v.clone(),
                );
            }
            Err(e) => {
                self.record_experience(
                    "stock_quote",
                    &format!("symbol={}", symbol),
                    "error",
                    serde_json::json!({"error": e.to_json_string()}),
                );
            }
        }
        span.finish(result)
    }

    #[tool(description = "Get income statement")]
    async fn income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("income_statement", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            companies_get(
                &self.client,
                "income_statement",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get balance sheet")]
    async fn balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("balance_sheet", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            companies_get(
                &self.client,
                "balance_sheet",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get cash flow statement")]
    async fn cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("cash_flow_statement", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            companies_get(
                &self.client,
                "cash_flow_statement",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get key metrics")]
    async fn key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("key_metrics", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = limit.unwrap_or(5).to_string();
        span.finish(
            companies_get(
                &self.client,
                "key_metrics",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", &limit_str)],
            )
            .await,
        )
    }

    #[tool(description = "Get historical price data")]
    async fn historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("historical_price", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            companies_get(
                &self.client,
                "historical_price",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("from", &from), ("to", &to)],
            )
            .await,
        )
    }

    #[tool(description = "Search for symbols")]
    async fn symbol_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("symbol_search", &self.webid);
        if query.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("query must not be empty").to_json_string(),
            );
        }
        let limit_str = limit.unwrap_or(10).to_string();
        // Search is special: it doesn't use a symbol, it uses a query.
        // Route to FMP first (better US coverage), fall back to EODHD.
        let fmp_result =
            providers::fmp_search_get(&self.client, &query, &limit_str, &self.fmp_api_key).await;
        match fmp_result {
            Ok(v) => span.ok_json(v),
            Err(_) => {
                let eodhd_result = providers::eodhd_search_get(
                    &self.client,
                    &query,
                    &limit_str,
                    &self.eodhd_api_key,
                )
                .await;
                span.finish(eodhd_result)
            }
        }
    }

    #[tool(description = "Get analyst estimates")]
    async fn analyst_estimates(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("analyst_estimates", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            companies_get(
                &self.client,
                "analyst_estimates",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("period", "annual")],
            )
            .await,
        )
    }

    #[tool(description = "Get discounted cash flow analysis")]
    async fn dcf_analysis(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("dcf_analysis", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        span.finish(
            companies_get(
                &self.client,
                "dcf_analysis",
                &symbol,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[],
            )
            .await,
        )
    }

    #[tool(
        description = "Analyze competitive moat using MAIA framework: gross margin stability and working capital market power signal"
    )]
    async fn moat_check(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("moat_check", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }

        // Fetch 10 years of key metrics for gross margin stability analysis
        let limit = "10";
        let metrics_result = companies_get(
            &self.client,
            "key_metrics",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", limit)],
        )
        .await;

        let metrics = match metrics_result {
            Ok(v) => v,
            Err(e) => {
                self.record_experience(
                    "moat_check",
                    &format!("symbol={}", symbol),
                    "error",
                    serde_json::json!({"error": e.to_json_string()}),
                );
                return span.error(e.kind, e.to_json_string());
            }
        };

        let gross_margins = analysis::extract_gross_margins(&metrics);
        if gross_margins.is_empty() {
            let output = serde_json::json!({
                "symbol": symbol,
                "moat": "insufficient_data",
                "reason": "No gross margin data available for this symbol",
            });
            self.record_experience(
                "moat_check",
                &format!("symbol={}", symbol),
                "insufficient_data",
                output.clone(),
            );
            return span.ok_json(output);
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

        let output = serde_json::json!({
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
        });
        self.record_experience(
            "moat_check",
            &format!("symbol={}", symbol),
            "success",
            output.clone(),
        );
        span.ok_json(output)
    }

    #[tool(
        description = "CEO capital allocation scorecard (MAIA framework): rates how well management allocates capital by comparing returns on capital vs invested capital over time"
    )]
    async fn management_scorecard(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("management_scorecard", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }

        let limit = "10";
        let metrics_result = companies_get(
            &self.client,
            "key_metrics",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", limit)],
        )
        .await;

        let bs_result = companies_get(
            &self.client,
            "balance_sheet",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", limit)],
        )
        .await;

        let (metrics, balance_sheets) = match (metrics_result, bs_result) {
            (Ok(m), Ok(b)) => (m, b),
            (Err(e), _) | (_, Err(e)) => return span.error(e.kind, e.to_json_string()),
        };

        let roic_values = analysis::extract_roic(&metrics);
        let capital_values = analysis::extract_invested_capital(&balance_sheets);
        let roic_nums: Vec<f64> = roic_values.iter().map(|(_, v)| *v).collect();
        let capital_nums: Vec<f64> = capital_values.iter().map(|(_, v)| *v).collect();

        let rating = analysis::ceo_capital_allocation_score(&roic_nums, &capital_nums);

        span.ok_json(serde_json::json!({
            "symbol": symbol,
            "ceo_rating": rating,
            "returns_on_capital": roic_values,
            "invested_capital": capital_values,
            "data_periods": roic_nums.len(),
            "framework": "MAIA: Good = decreasing capital with improving returns, OR increasing capital with improving returns. Bad = increasing capital with decreasing returns.",
        }))
    }

    #[tool(
        description = "Working capital cycle analysis (MAIA CFO scorecard): tracks days payable, days sales outstanding, and cash conversion cycle over time"
    )]
    async fn working_capital_cycle(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("working_capital_cycle", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let limit_str = (limit.unwrap_or(10) as usize).min(40).to_string();

        let result = companies_get(
            &self.client,
            "key_metrics",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", &limit_str)],
        )
        .await;

        let metrics = match result {
            Ok(v) => v,
            Err(e) => return span.error(e.kind, e.to_json_string()),
        };

        // Extract working capital days per period
        let arr = match metrics.as_array() {
            Some(a) => a,
            None => return span.ok_json(serde_json::json!({"symbol": symbol, "error": "no data"})),
        };

        let periods: Vec<serde_json::Value> = arr
            .iter()
            .filter_map(|entry| {
                let year = entry.get("calendarYear")?.as_str().unwrap_or("");
                let period = entry.get("period").and_then(|p| p.as_str()).unwrap_or("");
                let dpo = entry.get("daysOfPayablesOutstanding")?.as_f64()?;
                let dso = entry.get("daysOfSalesOutstanding")?.as_f64()?;
                let dio = entry
                    .get("daysOfInventoryOutstanding")
                    .and_then(|v| v.as_f64());
                let ccc = entry.get("cashConversionCycle").and_then(|v| v.as_f64());
                Some(serde_json::json!({
                    "year": year,
                    "period": period,
                    "dpo": dpo,
                    "dso": dso,
                    "dio": dio,
                    "spread": dpo - dso,
                    "cash_conversion_cycle": ccc,
                }))
            })
            .collect();

        // MAIA CFO score: consistency of working capital management
        let spreads: Vec<f64> = periods
            .iter()
            .filter_map(|p| p.get("spread")?.as_f64())
            .collect();
        let spread_stability = analysis::gross_margin_stability(&spreads);

        let cfo_rating = if spread_stability > 0.8 {
            "stable"
        } else if spread_stability > 0.5 {
            "moderate"
        } else {
            "volatile"
        };

        span.ok_json(serde_json::json!({
            "symbol": symbol,
            "cfo_working_capital_rating": cfo_rating,
            "spread_stability": spread_stability,
            "periods": periods,
            "data_points": periods.len(),
            "framework": "MAIA CFO scorecard: stability of working capital management through economic conditions. The level is structural; consistency is management skill.",
        }))
    }

    #[tool(
        description = "Expectations gap analysis (MAIA valuation framework): reverse-engineers market-implied expectations and compares to analyst consensus"
    )]
    async fn expectations_gap(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("expectations_gap", &self.webid);
        if let Err(e) = validate_symbol(&symbol) {
            return span.error(e.kind, e.to_json_string());
        }

        // Parallel fetch: profile (for P/E, P/B, P/S) + analyst estimates
        let profile_result = companies_get(
            &self.client,
            "company_profile",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[],
        )
        .await;

        let estimates_result = companies_get(
            &self.client,
            "analyst_estimates",
            &symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("period", "annual")],
        )
        .await;

        let (profile_arr, estimates_arr) = match (profile_result, estimates_result) {
            (Ok(p), Ok(e)) => (p, e),
            (Err(e), _) | (_, Err(e)) => return span.error(e.kind, e.to_json_string()),
        };

        // Extract market multiples from profile
        let profile = profile_arr.as_array().and_then(|a| a.first());
        let profile_data: Option<(f64, f64, f64, f64)> = match profile {
            Some(p) => {
                let eps = p.get("eps").and_then(|v| v.as_f64()).unwrap_or(-1.0);
                let price = p.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let bv = p
                    .get("bookValuePerShare")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(-1.0);
                let sales_per_share = p
                    .get("revenuePerShare")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(-1.0);
                if price > 0.0 && eps > 0.0 && bv > 0.0 && sales_per_share > 0.0 {
                    Some((price / eps, price / bv, price / sales_per_share, price))
                } else {
                    None
                }
            }
            None => None,
        };

        // Extract analyst growth estimates
        let estimates = estimates_arr.as_array();
        let analyst_growth: Option<Vec<serde_json::Value>> = estimates.and_then(|arr| {
            let items: Vec<serde_json::Value> = arr
                .iter()
                .filter_map(|e| {
                    let year = e.get("date").and_then(|v| v.as_str()).unwrap_or("");
                    let revenue_growth =
                        e.get("estimatedRevenueGrowth").and_then(|v| v.as_f64())?;
                    let eps_growth = e.get("estimatedEpsGrowth").and_then(|v| v.as_f64())?;
                    Some(serde_json::json!({
                        "year": year,
                        "estimated_revenue_growth": revenue_growth,
                        "estimated_eps_growth": eps_growth,
                    }))
                })
                .collect();
            if items.is_empty() { None } else { Some(items) }
        });

        let (pe, pb, ps, price) = match profile_data {
            Some((pe, pb, ps, price)) => (pe, pb, ps, price),
            None => {
                return span.ok_json(serde_json::json!({
                    "symbol": symbol,
                    "error": "insufficient data for expectations gap analysis",
                }));
            }
        };

        // MAIA valuation insight
        let market_implied_expensive = pe > 25.0 || ps > 5.0;
        let market_implied_cheap = pe < 12.0 && ps < 2.0;

        span.ok_json(serde_json::json!({
            "symbol": symbol,
            "current_price": price,
            "market_multiples": {
                "price_to_earnings": pe,
                "price_to_book": pb,
                "price_to_sales": ps,
            },
            "market_sentiment": if market_implied_expensive { "high_expectations" } else if market_implied_cheap { "low_expectations" } else { "moderate_expectations" },
            "analyst_estimates": analyst_growth,
            "framework": "MAIA expectations investing: compare market-implied expectations (price multiples) against analyst consensus. Low market expectations + reasonable analyst growth = potential opportunity. High market expectations = setup for disappointment.",
        }))
    }
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.companies", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-companies",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let fmp_api_key = ctx
                .credentials
                .get("HKASK_FMP_API_KEY")
                .expect("required credential checked by run_stdio_server")
                .clone();
            let eodhd_api_key = ctx
                .credentials
                .get("HKASK_EODHD_API_KEY")
                .expect("required credential checked by run_stdio_server")
                .clone();
            CompaniesServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                fmp_api_key,
                eodhd_api_key,
            )
        },
        vec![
            hkask_mcp::CredentialRequirement::required(
                "HKASK_FMP_API_KEY",
                "Financial Modeling Prep API key",
            ),
            hkask_mcp::CredentialRequirement::required(
                "HKASK_EODHD_API_KEY",
                "EOD Historical Data (EODHD) API key",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = DaemonClient::new();

    let auth = client.auth_query(replicant).await?;
    match auth {
        DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.companies", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
        }
        DaemonResponse::AuthResponse {
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

    let assignment = client.assignment_query(replicant, "companies").await?;
    match assignment {
        DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.companies", replicant = %replicant, "Replicant assigned to companies role");
        }
        DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the companies MCP role. Use 'kask pod assign {} companies' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.companies", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
