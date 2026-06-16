//! hKask MCP Companies — Dual-provider company financial data (FMP + EODHD)
//!
//! Tools are provider-agnostic: each tool routes to FMP or EODHD based on
//! symbol characteristics, with automatic fallback. EODHD responses are
//! normalized to match FMP format so analysis functions work transparently.
//!
//! Financial data tools:
//! - `company_profile` — Company profile by symbol
//! - `stock_quote` — Real-time stock quote
//! - `income_statement` — Income statements
//! - `balance_sheet` — Balance sheet statements
//! - `cash_flow_statement` — Cash flow statements
//! - `key_metrics` — Key financial metrics
//! - `historical_price` — Historical price data
//! - `symbol_search` — Symbol search
//! - `moat_check` — MAIA competitive moat analysis
//! - `management_scorecard` — MAIA CEO capital allocation scorecard
//! - `working_capital_cycle` — MAIA CFO working capital analysis
//! - `expectations_gap` — Gordon Growth Model: implied vs historical growth
//!
//! Portfolio tools:
//! - `ledger_import` — Import CSV/JSON (auto-creates portfolio)
//! - `ledger_export` — Export CSV/JSON
//! - `portfolio_list` — List all portfolios
//! - `portfolio_delete` — Delete a portfolio
//! - `transaction_note_append` — Annotate a transaction
//! - `note_add` — Add a research note to a security
//! - `note_list` — List notes with optional date/tag filtering
//! - `note_delete` — Delete a note
//! - `file_attach` — Attach a file (base64) to a security
//! - `file_list` — List attached files for a security
//! - `file_delete` — Delete an attached file
//! - `portfolio_attribution` — What moved the portfolio
//! - `portfolio_characteristics` — Weighted-average fundamentals
//! - `portfolio_comparison` — Side-by-side comparison
//! - `portfolio_returns` — TWR and IRR for any date range

use chrono::Datelike;
use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_identifier};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_types::time::now_rfc3339;
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

mod analysis;
mod portfolio;
mod providers;

use portfolio::PortfolioManager;
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

// ── Portfolio request structs ──────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PortfolioNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransactionNoteRequest {
    pub portfolio: String,
    pub tx_id: String,
    pub note: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LedgerImportRequest {
    pub portfolio: String,
    pub format: String, // "csv" or "json"
    pub data: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LedgerExportRequest {
    pub portfolio: String,
    pub format: String, // "csv" or "json"
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PortfolioCompareRequest {
    pub portfolio_a: String,
    pub portfolio_b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttributionRequest {
    pub portfolio: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CharacteristicsRequest {
    pub portfolio: String,
    pub date: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExpectationsGapRequest {
    pub symbol: String,
    pub target_return: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PortfolioReturnsRequest {
    pub portfolio: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteAddRequest {
    pub portfolio: String,
    pub symbol: String,
    pub date: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteListRequest {
    pub portfolio: String,
    pub symbol: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteDeleteRequest {
    pub note_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileAttachRequest {
    pub portfolio: String,
    pub symbol: String,
    pub date: String,
    pub filename: String,
    pub mime_type: String,
    /// Base64-encoded file content
    pub data: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileListRequest {
    pub portfolio: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileDeleteRequest {
    pub file_id: String,
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
    portfolio: PortfolioManager,
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
            portfolio: PortfolioManager::new(),
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
                "timestamp": now_rfc3339(),
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
        description = "Expectations gap: compare trailing 5-year actual performance to the future performance implied by the current price. Uses Gordon Growth Model to compute implied growth from valuation multiples vs historical profitability and growth."
    )]
    async fn expectations_gap(
        &self,
        Parameters(req): Parameters<ExpectationsGapRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("expectations_gap", &self.webid);
        if let Err(e) = validate_symbol(&req.symbol) {
            return span.error(e.kind, e.to_json_string());
        }
        let target_return = req.target_return.unwrap_or(0.15);

        // Fetch 5 years of key metrics for historical profitability and growth
        let metrics_result = companies_get(
            &self.client,
            "key_metrics",
            &req.symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", "5")],
        )
        .await;

        let profile_result = companies_get(
            &self.client,
            "company_profile",
            &req.symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[],
        )
        .await;

        let bs_result = companies_get(
            &self.client,
            "balance_sheet",
            &req.symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            &[("limit", "1")],
        )
        .await;

        let (metrics_arr, profile_arr, bs_arr) = match (metrics_result, profile_result, bs_result) {
            (Ok(m), Ok(p), Ok(b)) => (m, p, b),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                return span.error(e.kind, e.to_json_string());
            }
        };

        // Extract trailing 5-year averages
        let metrics_list = metrics_arr.as_array();
        let profile = profile_arr.as_array().and_then(|a| a.first());
        let bs = bs_arr.as_array().and_then(|a| a.first());

        if metrics_list.is_none_or(|m| m.len() < 2) || profile.is_none() {
            return span.ok_json(serde_json::json!({
                "symbol": req.symbol,
                "error": "insufficient data — need at least 2 years of metrics and a profile",
            }));
        }

        let metrics = metrics_list
            .as_ref()
            .expect("guarded by is_none_or check above");

        // ── Set A: Net Margins, Sales Growth, P/Sales ──
        let (net_margins, sales_growths) = extract_net_margin_and_sales_growth(metrics);
        let avg_net_margin: f64 = if !net_margins.is_empty() {
            net_margins.iter().sum::<f64>() / net_margins.len() as f64
        } else {
            0.0
        };
        let hist_sales_growth: f64 = if sales_growths.len() >= 2 {
            cagr_from_series(&sales_growths)
        } else {
            0.0
        };
        let ps = profile
            .and_then(|p| {
                let price = p.get("price").and_then(|v| v.as_f64())?;
                let sps = p.get("revenuePerShare").and_then(|v| v.as_f64())?;
                if sps > 0.0 { Some(price / sps) } else { None }
            })
            .unwrap_or(0.0);

        // ── Set B: ROE, Book Value Growth, P/Book ──
        let (roes, bv_growths) = extract_roe_and_bv_growth(metrics);
        let avg_roe: f64 = if !roes.is_empty() {
            roes.iter().sum::<f64>() / roes.len() as f64
        } else {
            0.0
        };
        let hist_bv_growth: f64 = if bv_growths.len() >= 2 {
            cagr_from_series(&bv_growths)
        } else {
            0.0
        };
        let pb = profile
            .and_then(|p| {
                let price = p.get("price").and_then(|v| v.as_f64())?;
                let bv = p.get("bookValuePerShare").and_then(|v| v.as_f64())?;
                if bv > 0.0 { Some(price / bv) } else { None }
            })
            .unwrap_or(0.0);

        // ── Set C: ROA, Asset Growth, P/Assets ──
        let (roas, asset_growths) = extract_roa_and_asset_growth(metrics);
        let avg_roa: f64 = if !roas.is_empty() {
            roas.iter().sum::<f64>() / roas.len() as f64
        } else {
            0.0
        };
        let hist_asset_growth: f64 = if asset_growths.len() >= 2 {
            cagr_from_series(&asset_growths)
        } else {
            0.0
        };
        let market_cap = profile
            .and_then(|p| p.get("mktCap").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let total_assets = bs
            .and_then(|b| b.get("totalAssets").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let pa = if total_assets > 0.0 {
            market_cap / total_assets
        } else {
            0.0
        };

        // ── Gordon Growth Model: implied growth from valuation ──
        // Assumes profitability and growth are correlated — a company expected
        // to grow 10% is also expected to improve profitability ~10%.
        // Total cash flow growth ≈ 2g, so: P/V = profitability / (r - 2g)
        // Rearranging: g = (r - profitability / (P/V)) / 2
        let implied_sales_growth = if ps > 0.0 && avg_net_margin > 0.0 {
            (target_return - avg_net_margin / ps) / 2.0
        } else {
            f64::NAN
        };
        let implied_bv_growth = if pb > 0.0 && avg_roe > 0.0 {
            (target_return - avg_roe / pb) / 2.0
        } else {
            f64::NAN
        };
        let implied_asset_growth = if pa > 0.0 && avg_roa > 0.0 {
            (target_return - avg_roa / pa) / 2.0
        } else {
            f64::NAN
        };

        span.ok_json(serde_json::json!({
            "symbol": req.symbol,
            "target_return": target_return,
            "historical": {
                "years": metrics.len(),
                "set_a_income": {
                    "avg_net_margin": avg_net_margin,
                    "hist_sales_growth": hist_sales_growth,
                    "price_to_sales": ps,
                },
                "set_b_balance": {
                    "avg_roe": avg_roe,
                    "hist_book_value_growth": hist_bv_growth,
                    "price_to_book": pb,
                },
                "set_c_assets": {
                    "avg_roa": avg_roa,
                    "hist_asset_growth": hist_asset_growth,
                    "price_to_assets": pa,
                },
            },
            "implied": {
                "set_a_sales_growth_implied": implied_sales_growth,
                "set_b_book_value_growth_implied": implied_bv_growth,
                "set_c_asset_growth_implied": implied_asset_growth,
            },
            "gaps": {
                "sales_growth_gap": if implied_sales_growth.is_finite() && hist_sales_growth.is_finite() { implied_sales_growth - hist_sales_growth } else { f64::NAN },
                "book_value_growth_gap": if implied_bv_growth.is_finite() && hist_bv_growth.is_finite() { implied_bv_growth - hist_bv_growth } else { f64::NAN },
                "asset_growth_gap": if implied_asset_growth.is_finite() && hist_asset_growth.is_finite() { implied_asset_growth - hist_asset_growth } else { f64::NAN },
            },
            "framework": "Gordon Growth Model with profitability-growth correlation: P/V = profitability / (r - 2g). Assumes growth and profitability improvement are proportional — a company expected to grow 10% is also expected to improve profitability ~10%. Total cash flow growth ≈ 2g. Implied g = (r - profitability / valuation_ratio) / 2. Compare to historical CAGR. Consistent methodology → rank ordering is accurate even if precise quantification is not.",
        }))
    }

    // ── Portfolio tools ──────────────────────────────────────────

    #[tool(description = "Delete a portfolio and all its data")]
    async fn portfolio_delete(
        &self,
        Parameters(PortfolioNameRequest { name }): Parameters<PortfolioNameRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("portfolio_delete", &self.webid);
        match self.portfolio.delete(&name) {
            Ok(()) => span.ok_json(serde_json::json!({"status": "deleted", "name": name})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "List all portfolios")]
    async fn portfolio_list(&self) -> String {
        let span = ToolSpanGuard::new("portfolio_list", &self.webid);
        match self.portfolio.list() {
            Ok(names) => span.ok_json(serde_json::json!({"portfolios": names})),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Import transactions from CSV or JSON into a portfolio ledger")]
    async fn ledger_import(
        &self,
        Parameters(LedgerImportRequest {
            portfolio,
            format,
            data,
        }): Parameters<LedgerImportRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ledger_import", &self.webid);
        // Auto-create portfolio if it doesn't exist
        if self.portfolio.list().is_ok_and(|l| !l.contains(&portfolio))
            && let Err(e) = self.portfolio.create(&portfolio)
        {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("auto-create failed: {e}")).to_json_string(),
            );
        }
        let result = match format.as_str() {
            "csv" => self.portfolio.import_csv(&portfolio, &data),
            "json" => self.portfolio.import_json(&portfolio, &data),
            other => Err(format!("unsupported format '{other}'; use 'csv' or 'json'")),
        };
        match result {
            Ok(ids) => {
                // Auto-validate after import
                let validation = self.portfolio.validate(&portfolio).unwrap_or_else(|e| {
                    portfolio::ValidationReport {
                        valid: false,
                        transaction_count: ids.len(),
                        positions: vec![],
                        cash_balance: 0.0,
                        issues: vec![e],
                    }
                });
                span.ok_json(serde_json::json!({
                    "status": "imported",
                    "count": ids.len(),
                    "validation": {
                        "valid": validation.valid,
                        "positions": validation.positions.len(),
                        "cash": validation.cash_balance,
                        "issues": validation.issues,
                    }
                }))
            }
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Export portfolio ledger to CSV or JSON")]
    async fn ledger_export(
        &self,
        Parameters(LedgerExportRequest { portfolio, format }): Parameters<LedgerExportRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ledger_export", &self.webid);
        let result = match format.as_str() {
            "csv" => self.portfolio.export_csv(&portfolio),
            "json" => self.portfolio.export_json(&portfolio),
            other => Err(format!("unsupported format '{other}'; use 'csv' or 'json'")),
        };
        match result {
            Ok(data) => span.ok_json(serde_json::json!({"format": format, "data": data})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Append a note to an existing transaction")]
    async fn transaction_note_append(
        &self,
        Parameters(TransactionNoteRequest {
            portfolio,
            tx_id,
            note,
        }): Parameters<TransactionNoteRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("transaction_note_append", &self.webid);
        match self.portfolio.append_note(&portfolio, &tx_id, &note) {
            Ok(()) => span.ok_json(serde_json::json!({"status": "note appended", "tx_id": tx_id})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Compare two portfolios side by side — positions, overlap, unique symbols"
    )]
    async fn portfolio_comparison(
        &self,
        Parameters(PortfolioCompareRequest {
            portfolio_a,
            portfolio_b,
        }): Parameters<PortfolioCompareRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("portfolio_comparison", &self.webid);
        match self.portfolio.compare(&portfolio_a, &portfolio_b) {
            Ok(report) => span.ok_json(report),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Time-weighted and money-weighted returns for a date range")]
    async fn portfolio_returns(
        &self,
        Parameters(PortfolioReturnsRequest {
            portfolio,
            from,
            to,
        }): Parameters<PortfolioReturnsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("portfolio_returns", &self.webid);

        let txs = match self
            .portfolio
            .get_transactions(&portfolio, None, None, None, None)
        {
            Ok(t) => t,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e).to_json_string(),
                );
            }
        };

        // ── Compute positions at from and to ─────────────────────
        let mut positions_start: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let mut positions_end: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let mut cash_start = 0.0f64;
        let mut cash_end = 0.0f64;

        // Collect cash flow dates for TWR sub-periods
        let mut cash_flow_events: Vec<(String, f64)> = Vec::new();

        for tx in &txs {
            // Cash accounting
            let cf_amount = match tx.tx_type.as_str() {
                "deposit" => tx.amount.unwrap_or(0.0),
                "withdrawal" => -tx.amount.unwrap_or(0.0),
                "buy" => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    let price = tx.price.unwrap_or(0.0);
                    let comm = tx.commission.unwrap_or(0.0);
                    -(qty * price + comm)
                }
                "sell" => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    let price = tx.price.unwrap_or(0.0);
                    let comm = tx.commission.unwrap_or(0.0);
                    qty * price - comm
                }
                "dividend" => tx.amount.unwrap_or(0.0),
                _ => 0.0,
            };

            if tx.date <= from {
                cash_start += cf_amount;
            }
            if tx.date <= to {
                cash_end += cf_amount;
            }

            // Collect deposit/withdrawal events in (from, to] for TWR sub-periods
            if tx.date > from
                && tx.date <= to
                && (tx.tx_type == "deposit" || tx.tx_type == "withdrawal")
            {
                let amt = match tx.tx_type.as_str() {
                    "deposit" => tx.amount.unwrap_or(0.0),
                    "withdrawal" => -tx.amount.unwrap_or(0.0),
                    _ => 0.0,
                };
                cash_flow_events.push((tx.date.clone(), amt));
            }

            // Position accounting
            if let Some(ref sym) = tx.symbol {
                let qty = tx.quantity.unwrap_or(0.0);
                if tx.date <= from {
                    match tx.tx_type.as_str() {
                        "buy" => *positions_start.entry(sym.clone()).or_insert(0.0) += qty,
                        "sell" => *positions_start.entry(sym.clone()).or_insert(0.0) -= qty,
                        _ => {}
                    }
                }
                if tx.date <= to {
                    match tx.tx_type.as_str() {
                        "buy" => *positions_end.entry(sym.clone()).or_insert(0.0) += qty,
                        "sell" => *positions_end.entry(sym.clone()).or_insert(0.0) -= qty,
                        _ => {}
                    }
                }
            }
        }

        // Retain only positive positions at start
        positions_start.retain(|_, v| *v > 0.0001);

        // Fetch prices for all symbols at from and to
        let all_symbols: Vec<String> = positions_start
            .keys()
            .chain(positions_end.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        let mut prices_at: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        // Try price_cache first, then API
        for date in [&from, &to] {
            let key_prefix = format!("{date}:");
            for sym in &all_symbols {
                // Check cache
                if let Ok(cached) = self.portfolio.get_prices(&portfolio, sym, date, date)
                    && let Some((_, close, _)) = cached.first()
                {
                    prices_at.insert(format!("{key_prefix}{sym}"), *close);
                    continue;
                }
                // Fall back to API
                if let Ok(value) = companies_get(
                    &self.client,
                    "historical_price",
                    sym,
                    &self.fmp_api_key,
                    &self.eodhd_api_key,
                    &[("from", date), ("to", date)],
                )
                .await
                    && let Some(days) = value.get("historical").and_then(|h| h.as_array())
                    && let Some(day) = days.first()
                    && let Some(close) = day
                        .get("close")
                        .or_else(|| day.get("adjClose"))
                        .and_then(|v| v.as_f64())
                {
                    prices_at.insert(format!("{key_prefix}{sym}"), close);
                }
            }
        }

        // ── Compute market values ─────────────────────────────────
        let mv_at = |positions: &std::collections::HashMap<String, f64>, date: &str| -> f64 {
            positions
                .iter()
                .map(|(sym, shares)| {
                    let price = prices_at
                        .get(&format!("{date}:{sym}"))
                        .copied()
                        .unwrap_or(0.0);
                    shares * price
                })
                .sum()
        };

        let mv_start = mv_at(&positions_start, &from);
        let mv_end = mv_at(&positions_end, &to);
        let total_start = mv_start + cash_start;
        let total_end = mv_end + cash_end;

        if total_start <= 0.0 {
            return span.ok_json(serde_json::json!({
                "error": "portfolio has zero or negative starting value",
                "from": from,
                "to": to,
            }));
        }

        let net_flows: f64 = cash_flow_events.iter().map(|(_, amt)| amt).sum();

        // ── Total return ──────────────────────────────────────────
        let total_return = (total_end - total_start - net_flows) / total_start;

        // ── Modified Dietz (approximate TWR) ──────────────────────
        let to_date = chrono::NaiveDate::parse_from_str(&to, "%Y-%m-%d").unwrap_or_default();
        let from_date = chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d").unwrap_or_default();
        let period_days = (to_date - from_date).num_days().max(1) as f64;

        let weighted_flows: f64 = cash_flow_events
            .iter()
            .map(|(date_str, amt)| {
                let cf_date =
                    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap_or_default();
                let days_remaining = (to_date - cf_date).num_days().max(0) as f64;
                let weight = days_remaining / period_days;
                amt * weight
            })
            .sum();

        let modified_dietz = if (total_start + weighted_flows).abs() > 0.0001 {
            (total_end - total_start - net_flows) / (total_start + weighted_flows)
        } else {
            total_return
        };

        // ── IRR via Newton's method ───────────────────────────────
        // Treat this as solving NPV(r) = 0 where:
        // cash flows = [-total_start at from, each external CF, +total_end at to]
        let irr = {
            let from_days = from_date.num_days_from_ce();
            let mut cfs: Vec<(f64, f64)> = vec![(-total_start, from_days as f64)];
            for (date_str, amt) in &cash_flow_events {
                let cf_date =
                    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap_or_default();
                let days = (cf_date.num_days_from_ce() - from_days) as f64;
                cfs.push((*amt, days));
            }
            let to_days = (to_date.num_days_from_ce() - from_days) as f64;
            cfs.push((total_end, to_days));

            // Newton's method: r_{n+1} = r_n - NPV(r_n) / NPV'(r_n)
            let npv = |r: f64| -> f64 {
                cfs.iter()
                    .map(|(cf, days)| cf / (1.0 + r).powf(days / 365.0))
                    .sum()
            };
            let npv_deriv = |r: f64| -> f64 {
                cfs.iter()
                    .map(|(cf, days)| -cf * (days / 365.0) / (1.0 + r).powf(days / 365.0 + 1.0))
                    .sum()
            };

            let mut r = 0.1; // initial guess: 10%
            for _ in 0..50 {
                let f = npv(r);
                let fp = npv_deriv(r);
                if fp.abs() < 1e-12 {
                    break;
                }
                let r_new = r - f / fp;
                if (r_new - r).abs() < 1e-8 {
                    r = r_new;
                    break;
                }
                r = r_new;
                if r < -0.99 {
                    r = -0.5; // reset if diving below -100%
                }
                if r > 10.0 {
                    r = 1.0; // cap at 100% and continue
                }
            }
            r
        };

        span.ok_json(serde_json::json!({
            "portfolio": portfolio,
            "from": from,
            "to": to,
            "total_return": total_return,
            "modified_dietz": modified_dietz,
            "irr": irr,
            "start_value": total_start,
            "end_value": total_end,
            "net_cash_flows": net_flows,
            "cash_flow_count": cash_flow_events.len(),
            "positions_at_start": positions_start.len(),
            "positions_at_end": positions_end.len(),
        }))
    }

    // ── Notes & Files tools ─────────────────────────────────────

    #[tool(description = "Add a note to a company/security as of a date")]
    async fn note_add(
        &self,
        Parameters(NoteAddRequest {
            portfolio,
            symbol,
            date,
            title,
            body,
            tags,
        }): Parameters<NoteAddRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("note_add", &self.webid);
        match self
            .portfolio
            .add_note(&portfolio, &symbol, &date, &title, &body, &tags)
        {
            Ok(id) => span.ok_json(serde_json::json!({"status": "created", "id": id})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "List notes for a symbol, optionally filtered by date range or tags")]
    async fn note_list(
        &self,
        Parameters(NoteListRequest {
            portfolio,
            symbol,
            date_from,
            date_to,
            tags,
        }): Parameters<NoteListRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("note_list", &self.webid);
        match self.portfolio.list_notes(
            &portfolio,
            &symbol,
            date_from.as_deref(),
            date_to.as_deref(),
            tags.as_deref(),
        ) {
            Ok(notes) => span.ok_json(serde_json::json!({"notes": notes})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Delete a note by ID")]
    async fn note_delete(
        &self,
        Parameters(NoteDeleteRequest { note_id }): Parameters<NoteDeleteRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("note_delete", &self.webid);
        match self.portfolio.delete_note(&note_id) {
            Ok(()) => span.ok_json(serde_json::json!({"status": "deleted", "id": note_id})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Attach a file (base64-encoded) to a company/security")]
    async fn file_attach(
        &self,
        Parameters(FileAttachRequest {
            portfolio,
            symbol,
            date,
            filename,
            mime_type,
            data,
            notes,
        }): Parameters<FileAttachRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("file_attach", &self.webid);
        match self.portfolio.attach_file(
            &portfolio, &symbol, &date, &filename, &mime_type, &data, &notes,
        ) {
            Ok(id) => span.ok_json(serde_json::json!({"status": "attached", "id": id})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "List attached files for a symbol in a portfolio")]
    async fn file_list(
        &self,
        Parameters(FileListRequest { portfolio, symbol }): Parameters<FileListRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("file_list", &self.webid);
        match self.portfolio.list_files(&portfolio, &symbol) {
            Ok(files) => span.ok_json(serde_json::json!({"files": files})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    #[tool(description = "Delete an attached file by ID — removes record and file from disk")]
    async fn file_delete(
        &self,
        Parameters(FileDeleteRequest { file_id }): Parameters<FileDeleteRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("file_delete", &self.webid);
        match self.portfolio.delete_file(&file_id) {
            Ok(()) => span.ok_json(serde_json::json!({"status": "deleted", "id": file_id})),
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e).to_json_string(),
            ),
        }
    }

    // ── Analysis tools ───────────────────────────────────────────

    #[tool(
        description = "What moved the portfolio — each position's weight, return, and contribution, ranked by impact"
    )]
    async fn portfolio_attribution(
        &self,
        Parameters(req): Parameters<AttributionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("portfolio_attribution", &self.webid);

        // Get transactions and compute positions at start and end
        let txs = match self
            .portfolio
            .get_transactions(&req.portfolio, None, None, None, None)
        {
            Ok(t) => t,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e).to_json_string(),
                );
            }
        };

        // Compute positions at from_date and to_date
        let mut positions_start: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let mut positions_end: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for tx in &txs {
            if let Some(ref sym) = tx.symbol {
                if tx.date <= req.from {
                    match tx.tx_type.as_str() {
                        "buy" => {
                            *positions_start.entry(sym.clone()).or_insert(0.0) +=
                                tx.quantity.unwrap_or(0.0)
                        }
                        "sell" => {
                            *positions_start.entry(sym.clone()).or_insert(0.0) -=
                                tx.quantity.unwrap_or(0.0)
                        }
                        _ => {}
                    }
                }
                if tx.date <= req.to {
                    match tx.tx_type.as_str() {
                        "buy" => {
                            *positions_end.entry(sym.clone()).or_insert(0.0) +=
                                tx.quantity.unwrap_or(0.0)
                        }
                        "sell" => {
                            *positions_end.entry(sym.clone()).or_insert(0.0) -=
                                tx.quantity.unwrap_or(0.0)
                        }
                        _ => {}
                    }
                }
            }
        }

        // Only include symbols with non-zero position at start
        positions_start.retain(|_, v| *v > 0.0001);
        if positions_start.is_empty() {
            return span.ok_json(
                serde_json::json!({"attribution": [], "message": "no positions at start date"}),
            );
        }

        // Fetch prices for all symbols at both dates
        let mut prices_start = serde_json::Map::new();
        let mut prices_end = serde_json::Map::new();
        let mut errors = Vec::new();

        for sym in positions_start.keys() {
            // Fetch historical prices around each date
            for (date, prices_map) in [(&req.from, &mut prices_start), (&req.to, &mut prices_end)] {
                match companies_get(
                    &self.client,
                    "historical_price",
                    sym,
                    &self.fmp_api_key,
                    &self.eodhd_api_key,
                    &[("from", date), ("to", date)],
                )
                .await
                {
                    Ok(value) => {
                        let historical = value.get("historical").and_then(|h| h.as_array());
                        if let Some(days) = historical
                            && let Some(day) = days.first()
                        {
                            let close = day
                                .get("close")
                                .or_else(|| day.get("adjClose"))
                                .and_then(|v| v.as_f64());
                            if let Some(c) = close {
                                prices_map.insert(sym.clone(), serde_json::Value::from(c));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("{sym}@{date}: {}", e.to_json_string()));
                    }
                }
            }
        }

        // Build attribution table
        let mut rows = Vec::new();
        for (sym, shares) in &positions_start {
            let p_start = prices_start
                .get(sym)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let p_end = prices_end.get(sym).and_then(|v| v.as_f64()).unwrap_or(0.0);
            if p_start <= 0.0 {
                continue;
            }
            let security_return = (p_end - p_start) / p_start;
            let mv_start = shares * p_start;
            rows.push((sym.clone(), mv_start, security_return));
        }

        let total_mv: f64 = rows.iter().map(|(_, mv, _)| mv).sum();
        let mut attribution: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(sym, mv_start, ret)| {
                let weight = if total_mv > 0.0 { mv_start / total_mv } else { 0.0 };
                let contribution_bps = weight * ret * 10000.0;
                let shares_end = positions_end.get(&sym).copied().unwrap_or(0.0);
                let p_end = prices_end.get(&sym).and_then(|v| v.as_f64()).unwrap_or(0.0);
                serde_json::json!({
                    "symbol": sym,
                    "weight_start_pct": (weight * 100.0),
                    "weight_end_pct": if total_mv > 0.0 { shares_end * p_end / total_mv * 100.0 } else { 0.0 },
                    "security_return_pct": (ret * 100.0),
                    "contribution_bps": contribution_bps,
                    "gain_loss": mv_start * ret,
                })
            })
            .collect();

        // Sort by absolute contribution
        attribution.sort_by(|a, b| {
            let ca = a["contribution_bps"].as_f64().unwrap_or(0.0).abs();
            let cb = b["contribution_bps"].as_f64().unwrap_or(0.0).abs();
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });

        span.ok_json(serde_json::json!({
            "portfolio": req.portfolio,
            "from": req.from,
            "to": req.to,
            "attribution": attribution,
            "errors": errors,
        }))
    }

    #[tool(
        description = "Weighted-average fundamentals of what the portfolio owns — valuation, profitability, leverage, growth, composition"
    )]
    async fn portfolio_characteristics(
        &self,
        Parameters(req): Parameters<CharacteristicsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("portfolio_characteristics", &self.webid);

        let symbols = match self.portfolio.get_symbols(&req.portfolio) {
            Ok(s) => s,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e).to_json_string(),
                );
            }
        };

        if symbols.is_empty() {
            return span.ok_json(
                serde_json::json!({"characteristics": {}, "message": "no symbols in portfolio"}),
            );
        }

        // Get positions at the as-of date
        let txs =
            match self
                .portfolio
                .get_transactions(&req.portfolio, None, None, None, Some(&req.date))
            {
                Ok(t) => t,
                Err(e) => {
                    return span.error(
                        McpErrorKind::InvalidArgument,
                        McpToolError::invalid_argument(e).to_json_string(),
                    );
                }
            };
        let mut positions: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for tx in &txs {
            if let Some(ref sym) = tx.symbol {
                match tx.tx_type.as_str() {
                    "buy" => {
                        *positions.entry(sym.clone()).or_insert(0.0) += tx.quantity.unwrap_or(0.0)
                    }
                    "sell" => {
                        *positions.entry(sym.clone()).or_insert(0.0) -= tx.quantity.unwrap_or(0.0)
                    }
                    _ => {}
                }
            }
        }
        positions.retain(|_, v| *v > 0.0001);

        // Fetch prices and market values
        let mut market_values = Vec::new();
        let mut errors = Vec::new();
        for sym in positions.keys() {
            match companies_get(
                &self.client,
                "stock_quote",
                sym,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[],
            )
            .await
            {
                Ok(value) => {
                    let price = value
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|q| q.get("price").and_then(|p| p.as_f64()))
                        .unwrap_or(0.0);
                    let shares = positions.get(sym).copied().unwrap_or(0.0);
                    market_values.push((sym.clone(), shares, price, shares * price));
                }
                Err(e) => {
                    errors.push(format!("{sym} quote: {}", e.to_json_string()));
                }
            }
        }

        let total_mv: f64 = market_values.iter().map(|(_, _, _, mv)| mv).sum();
        if total_mv <= 0.0 {
            return span
                .ok_json(serde_json::json!({"characteristics": {}, "message": "no market value"}));
        }

        // Fetch fundamentals and compute weighted averages
        let mut characteristics = serde_json::Map::new();
        for (sym, _shares, _price, mv) in &market_values {
            let weight = mv / total_mv;

            // Fetch profile for sector/industry/country/market cap
            if let Ok(profile_val) = companies_get(
                &self.client,
                "company_profile",
                sym,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[],
            )
            .await
                && let Some(profile) = profile_val.as_array().and_then(|a| a.first())
            {
                for field in ["sector", "industry", "country", "mktCap"] {
                    if let Some(val) = profile.get(field) {
                        let key = field.to_string();
                        let entry = characteristics.entry(key).or_insert(serde_json::json!(0.0));
                        if val.is_string() {
                            let str_val = val.as_str().expect("guarded by is_string check above");
                            let sub = characteristics
                                .entry(format!("{field}_breakdown"))
                                .or_insert(serde_json::json!({}));
                            if let Some(sub_map) = sub.as_object_mut() {
                                let e = sub_map
                                    .entry(str_val.to_string())
                                    .or_insert(serde_json::json!(0.0));
                                *e = serde_json::json!(e.as_f64().unwrap_or(0.0) + weight);
                            }
                        } else if let Some(num) = val.as_f64() {
                            *entry =
                                serde_json::json!(entry.as_f64().unwrap_or(0.0) + weight * num);
                        }
                    }
                }
            }

            // Fetch key metrics for profitability/valuation
            if let Ok(metrics_val) = companies_get(
                &self.client,
                "key_metrics",
                sym,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", "1")],
            )
            .await
                && let Some(metrics) = metrics_val.as_array().and_then(|a| a.first())
            {
                for field in [
                    "peRatio",
                    "priceToBookRatio",
                    "priceToSalesRatio",
                    "roic",
                    "roe",
                    "grossProfitMargin",
                    "operatingProfitMargin",
                    "netProfitMargin",
                    "debtToEquity",
                    "dividendYield",
                    "revenueGrowth",
                    "epsGrowth",
                ] {
                    if let Some(val) = metrics.get(field).and_then(|v| v.as_f64()) {
                        let entry = characteristics
                            .entry(field.to_string())
                            .or_insert(serde_json::json!(0.0));
                        *entry = serde_json::json!(entry.as_f64().unwrap_or(0.0) + weight * val);
                    }
                }
            }

            // Balance sheet for leverage
            if let Ok(bs_val) = companies_get(
                &self.client,
                "balance_sheet",
                sym,
                &self.fmp_api_key,
                &self.eodhd_api_key,
                &[("limit", "1")],
            )
            .await
                && let Some(bs) = bs_val.as_array().and_then(|a| a.first())
            {
                let assets = bs.get("totalAssets").and_then(|v| v.as_f64());
                let equity = bs.get("totalEquity").and_then(|v| v.as_f64());
                if let (Some(a), Some(e)) = (assets, equity)
                    && e > 0.0
                {
                    let lev = a / e;
                    let entry = characteristics
                        .entry("financialLeverage".to_string())
                        .or_insert(serde_json::json!(0.0));
                    *entry = serde_json::json!(entry.as_f64().unwrap_or(0.0) + weight * lev);
                }
            }
        }

        span.ok_json(serde_json::json!({
            "portfolio": req.portfolio,
            "date": req.date,
            "total_market_value": total_mv,
            "position_count": market_values.len(),
            "characteristics": characteristics,
            "errors": errors,
        }))
    }
}

// ── Expectations gap helpers ─────────────────────────────────────

fn extract_net_margin_and_sales_growth(metrics: &[serde_json::Value]) -> (Vec<f64>, Vec<f64>) {
    let mut margins = Vec::new();
    let mut revenues: Vec<(String, f64)> = Vec::new();
    for m in metrics {
        if let Some(npm) = m.get("netProfitMargin").and_then(|v| v.as_f64()) {
            margins.push(npm);
        }
        if let Some(rev) = m.get("revenuePerShare").and_then(|v| v.as_f64()) {
            let year = m.get("calendarYear").and_then(|v| v.as_str()).unwrap_or("");
            revenues.push((year.to_string(), rev));
        }
    }
    revenues.sort_by(|a, b| a.0.cmp(&b.0));
    let growths: Vec<f64> = revenues
        .windows(2)
        .filter_map(|w| {
            if w[0].1 > 0.0 {
                Some((w[1].1 - w[0].1) / w[0].1)
            } else {
                None
            }
        })
        .collect();
    (margins, growths)
}

fn extract_roe_and_bv_growth(metrics: &[serde_json::Value]) -> (Vec<f64>, Vec<f64>) {
    let mut roes = Vec::new();
    let mut bvs: Vec<(String, f64)> = Vec::new();
    for m in metrics {
        if let Some(roe) = m.get("roe").and_then(|v| v.as_f64()) {
            roes.push(roe);
        }
        if let Some(bv) = m.get("bookValuePerShare").and_then(|v| v.as_f64()) {
            let year = m.get("calendarYear").and_then(|v| v.as_str()).unwrap_or("");
            bvs.push((year.to_string(), bv));
        }
    }
    bvs.sort_by(|a, b| a.0.cmp(&b.0));
    let growths: Vec<f64> = bvs
        .windows(2)
        .filter_map(|w| {
            if w[0].1 > 0.0 {
                Some((w[1].1 - w[0].1) / w[0].1)
            } else {
                None
            }
        })
        .collect();
    (roes, growths)
}

fn extract_roa_and_asset_growth(metrics: &[serde_json::Value]) -> (Vec<f64>, Vec<f64>) {
    let mut roas = Vec::new();
    let mut assets: Vec<(String, f64)> = Vec::new();
    for m in metrics {
        if let Some(roa) = m.get("roa").and_then(|v| v.as_f64()) {
            roas.push(roa);
        }
        if let Some(ta) = m.get("totalAssets").and_then(|v| v.as_f64()) {
            let year = m.get("calendarYear").and_then(|v| v.as_str()).unwrap_or("");
            assets.push((year.to_string(), ta));
        }
    }
    assets.sort_by(|a, b| a.0.cmp(&b.0));
    let growths: Vec<f64> = assets
        .windows(2)
        .filter_map(|w| {
            if w[0].1 > 0.0 {
                Some((w[1].1 - w[0].1) / w[0].1)
            } else {
                None
            }
        })
        .collect();
    (roas, growths)
}

fn cagr_from_series(yoy_growths: &[f64]) -> f64 {
    if yoy_growths.is_empty() {
        return 0.0;
    }
    let product: f64 = yoy_growths.iter().map(|g| 1.0 + g).product();
    product.powf(1.0 / yoy_growths.len() as f64) - 1.0
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
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "companies", &[]).await?;
    tracing::info!(target: "hkask.mcp.companies", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
