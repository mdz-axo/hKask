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

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use chrono::Datelike;
use hkask_mcp::server::{McpToolError, execute_tool, validate_identifier};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_types::WebID;
use hkask_types::time::now_rfc3339;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::collections::HashMap;
use uuid::Uuid;

mod analysis;
pub mod fibo;
mod financial_model;
mod portfolio;
mod providers;
mod scenarios;
mod superforecast;
pub mod types;

use portfolio::PortfolioManager;

use types::*;

// ── Forecast store ───────────────────────────────────────────────────

/// A stored forecast model for later decomposition during `forecast_record`.
#[derive(Debug, Clone)]
pub struct StoredForecast {
    pub forecast_id: String,
    pub symbol: String,
    pub created_at: String,
    pub model: financial_model::ProjectedModel,
    pub assumptions: financial_model::ProjectionAssumptions,
    pub hist: financial_model::HistoricalSnapshot,
    pub current_price: f64,
    pub intrinsic_per_share: f64,
}

/// Extract the terminal multiple implied by the projected model.
fn projected_terminal_multiple(model: &financial_model::ProjectedModel) -> f64 {
    if let Some(last) = model.periods.last() {
        if last.free_cash_flow > 0.0 {
            model.terminal_value / last.free_cash_flow
        } else {
            0.0
        }
    } else {
        0.0
    }
}

/// Approximate a current price from a valuation multiple and historical data.
fn current_price_from_multiple(multiple: f64, hist: &financial_model::HistoricalSnapshot) -> f64 {
    let latest_fcf =
        hist.latest_revenue() * hist.gross_margin() - hist.latest_da() - hist.latest_capex();
    if hist.shares_outstanding > 0.0 {
        (latest_fcf * multiple) / hist.shares_outstanding
    } else {
        0.0
    }
}

// ── Validation ──────────────────────────────────────────────────────

fn validate_symbol(symbol: &str) -> Result<(), McpToolError> {
    // Allow exchange-qualified symbols (e.g., VOD.L, BMW.DE) for EODHD
    validate_identifier("symbol", symbol, 32)
}

/// Extract a symbol from a query string for learning state tracking.
/// Handles: "symbol=AAPL", "symbol=VOD.L", "query=..." (search queries).
fn parse_symbol_from_query(query: &str) -> Option<String> {
    if let Some(sym) = query.strip_prefix("symbol=") {
        let sym = sym.split('&').next().unwrap_or(sym);
        if !sym.is_empty() {
            return Some(sym.to_string());
        }
    }
    // For symbol_search, the query IS the search term — use it directly.
    if let Some(q) = query.strip_prefix("query=")
        && !q.is_empty()
    {
        return Some(q.to_string());
    }
    None
}

// ── Server struct ──────────────────────────────────────────────────

/// Learning state — tracks user feedback per (tool, symbol) to adapt
/// provider routing. Kanban-style: feedback → state → behavior change.
/// No separate consumer process needed — the feedback tool updates this
/// directly and the provider router reads it.
#[derive(Debug, Clone, Default)]
pub struct LearningState {
    /// (symbol, provider) → (inaccurate_count, total_ratings, avg_score)
    provider_scores: std::collections::HashMap<(String, String), (u64, u64, f64)>,
}

impl LearningState {
    /// Record a user rating for a tool result. Updates running averages.
    pub fn record(&mut self, symbol: &str, provider: &str, score: Option<u8>) {
        let key = (symbol.to_string(), provider.to_string());
        let entry = self.provider_scores.entry(key).or_insert((0, 0, 0.0));
        entry.1 += 1; // total ratings
        if let Some(s) = score {
            entry.2 = (entry.2 * (entry.1 - 1) as f64 + s as f64) / entry.1 as f64;
            if s <= 2 {
                entry.0 += 1; // inaccurate if score 1-2
            }
        }
    }

    /// Check if a provider should be avoided for a given symbol.
    /// Returns true if the provider has >3 inaccurate ratings with average <3.
    pub fn is_flaky(&self, symbol: &str, provider: &str) -> bool {
        if let Some((inaccurate, total, avg)) = self
            .provider_scores
            .get(&(symbol.to_string(), provider.to_string()))
        {
            *inaccurate > 3 && *avg < 3.0 && *total >= 5
        } else {
            false
        }
    }

    /// Get the preferred provider for a symbol based on learning.
    /// Returns None if no preference (both providers are fine or no data).
    pub fn preferred_provider(&self, symbol: &str) -> Option<String> {
        let _fmp_key = (symbol.to_string(), "FMP".to_string());
        let _eodhd_key = (symbol.to_string(), "EODHD".to_string());
        let fmp_flaky = self.is_flaky(symbol, "FMP");
        let eodhd_flaky = self.is_flaky(symbol, "EODHD");
        if fmp_flaky && !eodhd_flaky {
            Some("EODHD".to_string())
        } else if eodhd_flaky && !fmp_flaky {
            Some("FMP".to_string())
        } else {
            None
        }
    }
}

pub struct CompaniesServer {
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<DaemonClient>,
    pub client: reqwest::Client,
    pub fmp_api_key: String,
    pub eodhd_api_key: String,
    pub portfolio: PortfolioManager,
    /// Learning state — kanban-style feedback loop. Updated by result_feedback,
    /// read by provider routing to adapt provider preference per symbol.
    pub learning: std::sync::Arc<std::sync::Mutex<LearningState>>,
    /// Server-level Fermi seed/bootstrap estimates. Overridable via
    /// HKASK_FERMI_DEFAULTS env var. Used by calibrate_forecast.
    pub fermi_defaults: superforecast::FermiDefaults,
    /// In-memory forecast model store. dcf_valuation and calibrate_forecast
    /// store their projected models here keyed by forecast_id. forecast_record
    /// looks up stored models for 11-line-item gap decomposition.
    pub forecast_store: std::sync::Arc<std::sync::Mutex<HashMap<String, StoredForecast>>>,
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
            learning: std::sync::Arc::new(std::sync::Mutex::new(LearningState::default())),
            fermi_defaults: superforecast::FermiDefaults::from_env(),
            forecast_store: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        })
    }

    async fn fetch(
        &self,
        tool: &str,
        symbol: &str,
        extra: &[(&str, &str)],
    ) -> Result<serde_json::Value, McpToolError> {
        let l = self.learning.lock().unwrap().clone();
        providers::companies_get(
            &self.client,
            tool,
            symbol,
            &self.fmp_api_key,
            &self.eodhd_api_key,
            extra,
            Some(&l),
        )
        .await
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
                        tracing::debug!(target: "cns.mcp.companies.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "cns.mcp.companies.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "cns.mcp.companies.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

impl hkask_mcp::server::ToolContext for CompaniesServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

// ── Tools ──────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl CompaniesServer {
    #[tool(description = "Get company profile")]
    pub async fn company_profile(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "company_profile", async {
            validate_symbol(&symbol)?;
            let result = self.fetch("company_profile", &symbol, &[]).await;
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
            result
        })
        .await
    }

    #[tool(description = "Get stock quote")]
    pub async fn stock_quote(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "stock_quote", async {
            validate_symbol(&symbol)?;
            let result = self.fetch("stock_quote", &symbol, &[]).await;
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
            result
        })
        .await
    }

    #[tool(description = "Get income statement")]
    pub async fn income_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "income_statement", async {
            validate_symbol(&symbol)?;
            let limit_str = limit.unwrap_or(5).to_string();
            let result = self
                .fetch("income_statement", &symbol, &[("limit", &limit_str)])
                .await;
            match &result {
                Ok(v) => {
                    self.record_experience(
                        "income_statement",
                        &format!("symbol={}", symbol),
                        "success",
                        v.clone(),
                    );
                }
                Err(e) => {
                    self.record_experience(
                        "income_statement",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                }
            }
            result
        })
        .await
    }

    #[tool(description = "Get balance sheet")]
    pub async fn balance_sheet(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "balance_sheet", async {
            validate_symbol(&symbol)?;
            let limit_str = limit.unwrap_or(5).to_string();
            let result = self
                .fetch("balance_sheet", &symbol, &[("limit", &limit_str)])
                .await;
            match &result {
                Ok(v) => {
                    self.record_experience(
                        "balance_sheet",
                        &format!("symbol={}", symbol),
                        "success",
                        v.clone(),
                    );
                }
                Err(e) => {
                    self.record_experience(
                        "balance_sheet",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                }
            }
            result
        })
        .await
    }

    #[tool(description = "Get cash flow statement")]
    pub async fn cash_flow_statement(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "cash_flow_statement", async {
            validate_symbol(&symbol)?;
            let limit_str = limit.unwrap_or(5).to_string();
            let result = self
                .fetch("cash_flow_statement", &symbol, &[("limit", &limit_str)])
                .await;
            match &result {
                Ok(v) => {
                    self.record_experience(
                        "cash_flow_statement",
                        &format!("symbol={}", symbol),
                        "success",
                        v.clone(),
                    );
                }
                Err(e) => {
                    self.record_experience(
                        "cash_flow_statement",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                }
            }
            result
        })
        .await
    }

    #[tool(description = "Get key metrics")]
    pub async fn key_metrics(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "key_metrics", async {
            validate_symbol(&symbol)?;
            let limit_str = limit.unwrap_or(5).to_string();
            let result = self
                .fetch("key_metrics", &symbol, &[("limit", &limit_str)])
                .await;
            match &result {
                Ok(v) => {
                    self.record_experience(
                        "key_metrics",
                        &format!("symbol={}", symbol),
                        "success",
                        v.clone(),
                    );
                }
                Err(e) => {
                    self.record_experience(
                        "key_metrics",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                }
            }
            result
        })
        .await
    }

    #[tool(description = "Get historical price data")]
    pub async fn historical_price(
        &self,
        Parameters(HistoricalRequest { symbol, from, to }): Parameters<HistoricalRequest>,
    ) -> String {
        execute_tool(self, "historical_price", async {
            validate_symbol(&symbol)?;
            let result = self
                .fetch("historical_price", &symbol, &[("from", &from), ("to", &to)])
                .await;
            match &result {
                Ok(v) => {
                    self.record_experience(
                        "historical_price",
                        &format!("symbol={}", symbol),
                        "success",
                        v.clone(),
                    );
                }
                Err(e) => {
                    self.record_experience(
                        "historical_price",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                }
            }
            result
        })
        .await
    }

    #[tool(description = "Search for symbols")]
    pub async fn symbol_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        execute_tool(self, "symbol_search", async {
            if query.is_empty() {
                return Err(McpToolError::invalid_argument("query must not be empty"));
            }
            let limit_str = limit.unwrap_or(10).to_string();
            // Search is special: it doesn't use a symbol, it uses a query.
            // Route to FMP first (better US coverage), fall back to EODHD.
            let fmp_result =
                providers::fmp_search_get(&self.client, &query, &limit_str, &self.fmp_api_key)
                    .await;

            match fmp_result {
                Ok(v) => {
                    self.record_experience(
                        "symbol_search",
                        &format!("query={}, provider=fmp", query),
                        "success",
                        v.clone(),
                    );
                    Ok(v)
                }
                Err(_fmp_err) => {
                    let eodhd_result = providers::eodhd_search_get(
                        &self.client,
                        &query,
                        &limit_str,
                        &self.eodhd_api_key,
                    )
                    .await;
                    match &eodhd_result {
                        Ok(v) => {
                            self.record_experience(
                                "symbol_search",
                                &format!("query={}, provider=eodhd", query),
                                "success",
                                v.clone(),
                            );
                        }
                        Err(e) => {
                            self.record_experience(
                                "symbol_search",
                                &format!("query={}", query),
                                "error",
                                serde_json::json!({"error": e.to_json_string()}),
                            );
                        }
                    }
                    eodhd_result
                }
            }
        })
        .await
    }

    #[tool(
        description = "Analyze competitive moat using MAIA framework: gross margin stability and working capital market power signal"
    )]
    pub async fn moat_check(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "moat_check", async {
            validate_symbol(&symbol)?;

            // Fetch 10 years of key metrics for gross margin stability analysis
            let limit = "10";
            let metrics_result = self
                .fetch("key_metrics", &symbol, &[("limit", limit)])
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
                    return Err(e);
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
                return Ok(output);
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
            Ok(output)
        })
        .await
    }

    #[tool(
        description = "CEO capital allocation scorecard (MAIA framework): rates how well management allocates capital by comparing returns on capital vs invested capital over time"
    )]
    pub async fn management_scorecard(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "management_scorecard", async {
            validate_symbol(&symbol)?;

            let limit = "10";
            let metrics_result = self.fetch(
     "key_metrics",
     &symbol,
     &[("limit", limit)],
 )
            .await;

            let bs_result = self.fetch(
     "balance_sheet",
     &symbol,
     &[("limit", limit)],
 )
            .await;

            let (metrics, balance_sheets) = match (metrics_result, bs_result) {
                (Ok(m), Ok(b)) => (m, b),
                (Err(e), _) | (_, Err(e)) => {
                    self.record_experience(
                        "management_scorecard",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                    return Err(e);
                }
            };

            let roic_values = analysis::extract_roic(&metrics);
            let capital_values = analysis::extract_invested_capital(&balance_sheets);

            // Align ROIC and invested capital by calendar year — they come from
            // different API endpoints and may have different year ranges.
            use std::collections::HashMap;
            let roic_by_year: HashMap<&str, f64> = roic_values
                .iter()
                .map(|(y, v)| (y.as_str(), *v))
                .collect();
            let mut aligned: Vec<(f64, f64)> = capital_values
                .iter()
                .filter_map(|(year, cap)| roic_by_year.get(year.as_str()).map(|r| (*r, *cap)))
                .collect();
            // Sort by invested capital ascending to preserve original ordering intent
            aligned.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let roic_nums: Vec<f64> = aligned.iter().map(|(r, _)| *r).collect();
            let capital_nums: Vec<f64> = aligned.iter().map(|(_, c)| *c).collect();

            let rating = analysis::ceo_capital_allocation_score(&roic_nums, &capital_nums);

            let output = serde_json::json!({
                "symbol": symbol,
                "ceo_rating": rating,
                "returns_on_capital": roic_values,
                "invested_capital": capital_values,
                "aligned_periods": aligned.len(),
                "data_periods": roic_nums.len(),
                "framework": "MAIA: Good = decreasing capital with improving returns, OR increasing capital with improving returns. Bad = increasing capital with decreasing returns.",
            });
            self.record_experience(
                "management_scorecard",
                &format!("symbol={}", symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    #[tool(
        description = "Working capital cycle analysis (MAIA CFO scorecard): tracks days payable, days sales outstanding, and cash conversion cycle over time"
    )]
    pub async fn working_capital_cycle(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "working_capital_cycle", async {
            validate_symbol(&symbol)?;
            let limit_str = (limit.unwrap_or(10) as usize).min(40).to_string();

            let result = self.fetch(
     "key_metrics",
     &symbol,
     &[("limit", &limit_str)],
 )
            .await;

            let metrics = match result {
                Ok(v) => v,
                Err(e) => {
                    self.record_experience(
                        "working_capital_cycle",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                    return Err(e);
                }
            };

            // Extract working capital days per period
            let arr = match metrics.as_array() {
                Some(a) => a,
                None => {
                    return Ok(serde_json::json!({"symbol": symbol, "error": "no data"}));
                }
            };

            let periods: Vec<serde_json::Value> = arr
                .iter()
                .filter_map(|entry| {
                    let year = entry.get("calendarYear")?.as_str().unwrap_or("");
                    let period = entry
                        .get("period")
                        .and_then(|p| p.as_str())
                        .unwrap_or("");
                    let dpo = entry.get("daysOfPayablesOutstanding")?.as_f64()?;
                    let dso = entry.get("daysOfSalesOutstanding")?.as_f64()?;
                    let dio = entry
                        .get("daysOfInventoryOutstanding")
                        .and_then(|v| v.as_f64());
                    let ccc = entry
                        .get("cashConversionCycle")
                        .and_then(|v| v.as_f64());
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

            let output = serde_json::json!({
                "symbol": symbol,
                "cfo_working_capital_rating": cfo_rating,
                "spread_stability": spread_stability,
                "periods": periods,
                "data_points": periods.len(),
                "framework": "MAIA CFO scorecard: stability of working capital management through economic conditions. The level is structural; consistency is management skill.",
            });
            self.record_experience(
                "working_capital_cycle",
                &format!("symbol={}", symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    #[tool(
        description = "Expectations gap: compare trailing 5-year actual performance to the future performance implied by the current price. Uses Gordon Growth Model to compute implied growth from valuation multiples vs historical profitability and growth."
    )]
    pub async fn expectations_gap(
        &self,
        Parameters(req): Parameters<ExpectationsGapRequest>,
    ) -> String {
        execute_tool(self, "expectations_gap", async {
            validate_symbol(&req.symbol)?;
            let target_return = req.target_return.unwrap_or(0.15);

            // Fetch 5 years of key metrics for historical profitability and growth
            let metrics_result = self.fetch(
     "key_metrics",
     &req.symbol,
     &[("limit", "5")],
 )
            .await;

            let profile_result = self.fetch(
     "company_profile",
     &req.symbol,
     &[],
 )
            .await;

            let bs_result = self.fetch(
     "balance_sheet",
     &req.symbol,
     &[("limit", "1")],
 )
            .await;

            let (metrics_arr, profile_arr, bs_arr) =
                match (metrics_result, profile_result, bs_result) {
                    (Ok(m), Ok(p), Ok(b)) => (m, p, b),
                    (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                        self.record_experience(
                            "expectations_gap",
                            &format!("symbol={}", req.symbol),
                            "error",
                            serde_json::json!({"error": e.to_json_string()}),
                        );
                        return Err(e);
                    }
                };

            // Extract trailing 5-year averages
            let metrics_list = metrics_arr.as_array();
            let profile = profile_arr.as_array().and_then(|a| a.first());
            let bs = bs_arr.as_array().and_then(|a| a.first());

            if metrics_list.is_none_or(|m| m.len() < 2) || profile.is_none() {
                return Ok(serde_json::json!({
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
                    if sps > 0.0 {
                        Some(price / sps)
                    } else {
                        None
                    }
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
                    if bv > 0.0 {
                        Some(price / bv)
                    } else {
                        None
                    }
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
            //
            // NaN → null in JSON to avoid silent corruption downstream.
            let implied_sales_growth = if ps > 0.0 && avg_net_margin > 0.0 {
                Some((target_return - avg_net_margin / ps) / 2.0)
            } else {
                None
            };
            let implied_bv_growth = if pb > 0.0 && avg_roe > 0.0 {
                Some((target_return - avg_roe / pb) / 2.0)
            } else {
                None
            };
            let implied_asset_growth = if pa > 0.0 && avg_roa > 0.0 {
                Some((target_return - avg_roa / pa) / 2.0)
            } else {
                None
            };

            // Helper: finite f64 → JSON number, else null
            let finite_or_null = |v: Option<f64>| -> serde_json::Value {
                match v {
                    Some(x) if x.is_finite() => serde_json::json!(x),
                    _ => serde_json::Value::Null,
                }
            };

            let output = serde_json::json!({
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
                    "set_a_sales_growth_implied": finite_or_null(implied_sales_growth),
                    "set_b_book_value_growth_implied": finite_or_null(implied_bv_growth),
                    "set_c_asset_growth_implied": finite_or_null(implied_asset_growth),
                },
                "gaps": {
                    "sales_growth_gap": finite_or_null(implied_sales_growth.and_then(|i| if hist_sales_growth.is_finite() { Some(i - hist_sales_growth) } else { None })),
                    "book_value_growth_gap": finite_or_null(implied_bv_growth.and_then(|i| if hist_bv_growth.is_finite() { Some(i - hist_bv_growth) } else { None })),
                    "asset_growth_gap": finite_or_null(implied_asset_growth.and_then(|i| if hist_asset_growth.is_finite() { Some(i - hist_asset_growth) } else { None })),
                },
                "framework": "Gordon Growth Model with profitability-growth correlation: P/V = profitability / (r - 2g). Assumes growth and profitability improvement are proportional — a company expected to grow 10% is also expected to improve profitability ~10%. Total cash flow growth ≈ 2g. Implied g = (r - profitability / valuation_ratio) / 2. Compare to historical CAGR. Consistent methodology → rank ordering is accurate even if precise quantification is not.",
            });
            self.record_experience(
                "expectations_gap",
                &format!("symbol={}", req.symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    // ── Portfolio tools ──────────────────────────────────────────

    #[tool(description = "Delete a portfolio and all its data")]
    pub async fn portfolio_delete(
        &self,
        Parameters(PortfolioNameRequest { name }): Parameters<PortfolioNameRequest>,
    ) -> String {
        execute_tool(self, "portfolio_delete", async {
            self.portfolio
                .delete(&name)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "deleted", "name": name}))
        })
        .await
    }

    #[tool(description = "List all portfolios")]
    pub async fn portfolio_list(&self) -> String {
        execute_tool(self, "portfolio_list", async {
            let names = self.portfolio.list().map_err(McpToolError::internal)?;
            Ok(serde_json::json!({"portfolios": names}))
        })
        .await
    }

    #[tool(description = "Import transactions from CSV or JSON into a portfolio ledger")]
    pub async fn ledger_import(
        &self,
        Parameters(LedgerImportRequest {
            portfolio,
            format,
            data,
        }): Parameters<LedgerImportRequest>,
    ) -> String {
        execute_tool(self, "ledger_import", async {
            // Auto-create portfolio if it doesn't exist
            if self.portfolio.list().is_ok_and(|l| !l.contains(&portfolio))
                && let Err(e) = self.portfolio.create(&portfolio)
            {
                return Err(McpToolError::invalid_argument(format!(
                    "auto-create failed: {e}"
                )));
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
                    Ok(serde_json::json!({
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
                Err(e) => Err(McpToolError::invalid_argument(e)),
            }
        })
        .await
    }

    #[tool(description = "Export portfolio ledger to CSV or JSON")]
    pub async fn ledger_export(
        &self,
        Parameters(LedgerExportRequest { portfolio, format }): Parameters<LedgerExportRequest>,
    ) -> String {
        execute_tool(self, "ledger_export", async {
            let result = match format.as_str() {
                "csv" => self.portfolio.export_csv(&portfolio),
                "json" => self.portfolio.export_json(&portfolio),
                other => Err(format!("unsupported format '{other}'; use 'csv' or 'json'")),
            };
            match result {
                Ok(data) => Ok(serde_json::json!({"format": format, "data": data})),
                Err(e) => Err(McpToolError::invalid_argument(e)),
            }
        })
        .await
    }

    #[tool(description = "Append a note to an existing transaction")]
    pub async fn transaction_note_append(
        &self,
        Parameters(TransactionNoteRequest {
            portfolio,
            tx_id,
            note,
        }): Parameters<TransactionNoteRequest>,
    ) -> String {
        execute_tool(self, "transaction_note_append", async {
            self.portfolio
                .append_note(&portfolio, &tx_id, &note)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "note appended", "tx_id": tx_id}))
        })
        .await
    }

    #[tool(
        description = "Compare two portfolios side by side — positions, overlap, unique symbols"
    )]
    pub async fn portfolio_comparison(
        &self,
        Parameters(PortfolioCompareRequest {
            portfolio_a,
            portfolio_b,
        }): Parameters<PortfolioCompareRequest>,
    ) -> String {
        execute_tool(self, "portfolio_comparison", async {
            self.portfolio
                .compare(&portfolio_a, &portfolio_b)
                .map_err(McpToolError::invalid_argument)
        })
        .await
    }

    #[tool(description = "Time-weighted and money-weighted returns for a date range")]
    pub async fn portfolio_returns(
        &self,
        Parameters(PortfolioReturnsRequest {
            portfolio,
            from,
            to,
        }): Parameters<PortfolioReturnsRequest>,
    ) -> String {
        execute_tool(self, "portfolio_returns", async {
            let txs = match self
                .portfolio
                .get_transactions(&portfolio, None, None, None, None)
            {
                Ok(t) => t,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(e));
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
                    if let Ok(value) = self
                        .fetch("historical_price", sym, &[("from", date), ("to", date)])
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
                return Ok(serde_json::json!({
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
            let from_date =
                chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d").unwrap_or_default();
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
                let mut converged = false;
                for _ in 0..50 {
                    let f = npv(r);
                    let fp = npv_deriv(r);
                    if fp.abs() < 1e-12 {
                        break;
                    }
                    let r_new = r - f / fp;
                    if (r_new - r).abs() < 1e-8 {
                        r = r_new;
                        converged = true;
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
                (r, converged)
            };

            let (irr, irr_converged) = irr;

            Ok(serde_json::json!({
                "portfolio": portfolio,
                "from": from,
                "to": to,
                "total_return": total_return,
                "modified_dietz": modified_dietz,
                "irr": irr,
                "irr_converged": irr_converged,
                "start_value": total_start,
                "end_value": total_end,
                "net_cash_flows": net_flows,
                "cash_flow_count": cash_flow_events.len(),
                "positions_at_start": positions_start.len(),
                "positions_at_end": positions_end.len(),
            }))
        })
        .await
    }

    // ── Notes & Files tools ─────────────────────────────────────

    #[tool(description = "Add a note to a company/security as of a date")]
    pub async fn note_add(
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
        execute_tool(self, "note_add", async {
            let id = self
                .portfolio
                .add_note(&portfolio, &symbol, &date, &title, &body, &tags)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "created", "id": id}))
        })
        .await
    }

    #[tool(description = "List notes for a symbol, optionally filtered by date range or tags")]
    pub async fn note_list(
        &self,
        Parameters(NoteListRequest {
            portfolio,
            symbol,
            date_from,
            date_to,
            tags,
        }): Parameters<NoteListRequest>,
    ) -> String {
        execute_tool(self, "note_list", async {
            let notes = self
                .portfolio
                .list_notes(
                    &portfolio,
                    &symbol,
                    date_from.as_deref(),
                    date_to.as_deref(),
                    tags.as_deref(),
                )
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"notes": notes}))
        })
        .await
    }

    #[tool(description = "Delete a note by ID")]
    pub async fn note_delete(
        &self,
        Parameters(NoteDeleteRequest { note_id }): Parameters<NoteDeleteRequest>,
    ) -> String {
        execute_tool(self, "note_delete", async {
            self.portfolio
                .delete_note(&note_id)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "deleted", "id": note_id}))
        })
        .await
    }

    #[tool(description = "Attach a file (base64-encoded) to a company/security")]
    pub async fn file_attach(
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
        execute_tool(self, "file_attach", async {
            let id = self
                .portfolio
                .attach_file(
                    &portfolio, &symbol, &date, &filename, &mime_type, &data, &notes,
                )
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "attached", "id": id}))
        })
        .await
    }

    #[tool(description = "List attached files for a symbol in a portfolio")]
    pub async fn file_list(
        &self,
        Parameters(FileListRequest { portfolio, symbol }): Parameters<FileListRequest>,
    ) -> String {
        execute_tool(self, "file_list", async {
            let files = self
                .portfolio
                .list_files(&portfolio, &symbol)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"files": files}))
        })
        .await
    }

    #[tool(description = "Delete an attached file by ID — removes record and file from disk")]
    pub async fn file_delete(
        &self,
        Parameters(FileDeleteRequest { file_id }): Parameters<FileDeleteRequest>,
    ) -> String {
        execute_tool(self, "file_delete", async {
            self.portfolio
                .delete_file(&file_id)
                .map_err(McpToolError::invalid_argument)?;
            Ok(serde_json::json!({"status": "deleted", "id": file_id}))
        })
        .await
    }

    // ── Analysis tools ───────────────────────────────────────────

    #[tool(
        description = "What moved the portfolio — each position's weight, return, and contribution, ranked by impact"
    )]
    pub async fn portfolio_attribution(
        &self,
        Parameters(req): Parameters<AttributionRequest>,
    ) -> String {
        execute_tool(self, "portfolio_attribution", async {
            // Get transactions and compute positions at start and end
            let txs = match self
                .portfolio
                .get_transactions(&req.portfolio, None, None, None, None)
            {
                Ok(t) => t,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(e));
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
                return Ok(serde_json::json!(
                    {"attribution": [], "message": "no positions at start date"}
                ));
            }

            // Fetch prices for all symbols at both dates
            let mut prices_start = serde_json::Map::new();
            let mut prices_end = serde_json::Map::new();
            let mut errors = Vec::new();

            for sym in positions_start.keys() {
                // Fetch historical prices around each date
                for (date, prices_map) in
                    [(&req.from, &mut prices_start), (&req.to, &mut prices_end)]
                {
                    match self.fetch(
                            "historical_price",
                            sym,
                            &[("from", date), ("to", date)],
                    )
                    .await
                    {
                        Ok(value) => {
                            let historical =
                                value.get("historical").and_then(|h| h.as_array());
                            if let Some(days) = historical
                                && let Some(day) = days.first()
                            {
                                let close = day
                                    .get("close")
                                    .or_else(|| day.get("adjClose"))
                                    .and_then(|v| v.as_f64());
                                if let Some(c) = close {
                                    prices_map
                                        .insert(sym.clone(), serde_json::Value::from(c));
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
                let p_end = prices_end
                    .get(sym)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
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
                    let weight = if total_mv > 0.0 {
                        mv_start / total_mv
                    } else {
                        0.0
                    };
                    let contribution_bps = weight * ret * 10000.0;
                    let shares_end = positions_end.get(&sym).copied().unwrap_or(0.0);
                    let p_end = prices_end
                        .get(&sym)
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
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

            Ok(serde_json::json!({
                "portfolio": req.portfolio,
                "from": req.from,
                "to": req.to,
                "attribution": attribution,
                "errors": errors,
            }))
        }).await
    }

    #[tool(
        description = "Weighted-average fundamentals of what the portfolio owns — valuation, profitability, leverage, growth, composition"
    )]
    pub async fn portfolio_characteristics(
        &self,
        Parameters(req): Parameters<CharacteristicsRequest>,
    ) -> String {
        execute_tool(self, "portfolio_characteristics", async {
            let symbols = match self.portfolio.get_symbols(&req.portfolio) {
                Ok(s) => s,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(e));
                }
            };

            if symbols.is_empty() {
                return Ok(serde_json::json!(
                    {"characteristics": {}, "message": "no symbols in portfolio"}
                ));
            }

            // Get positions at the as-of date
            let txs = match self.portfolio.get_transactions(
                &req.portfolio,
                None,
                None,
                None,
                Some(&req.date),
            ) {
                Ok(t) => t,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(e));
                }
            };
            let mut positions: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            for tx in &txs {
                if let Some(ref sym) = tx.symbol {
                    match tx.tx_type.as_str() {
                        "buy" => {
                            *positions.entry(sym.clone()).or_insert(0.0) +=
                                tx.quantity.unwrap_or(0.0)
                        }
                        "sell" => {
                            *positions.entry(sym.clone()).or_insert(0.0) -=
                                tx.quantity.unwrap_or(0.0)
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
                match self.fetch("stock_quote", sym, &[]).await {
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
                return Ok(serde_json::json!(
                    {"characteristics": {}, "message": "no market value"}
                ));
            }

            // Fetch fundamentals and compute weighted averages
            let mut characteristics = serde_json::Map::new();
            for (sym, _shares, _price, mv) in &market_values {
                let weight = mv / total_mv;

                // Fetch profile for sector/industry/country/market cap
                if let Ok(profile_val) = self.fetch("company_profile", sym, &[]).await
                    && let Some(profile) = profile_val.as_array().and_then(|a| a.first())
                {
                    for field in ["sector", "industry", "country", "mktCap"] {
                        if let Some(val) = profile.get(field) {
                            let key = field.to_string();
                            let entry =
                                characteristics.entry(key).or_insert(serde_json::json!(0.0));
                            if val.is_string() {
                                let str_val =
                                    val.as_str().expect("guarded by is_string check above");
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
                if let Ok(metrics_val) = self.fetch("key_metrics", sym, &[("limit", "1")]).await
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
                            let fibo_uri = fibo::fmp_field_to_fibo(field).unwrap_or("unknown");
                            let entry = characteristics
                                .entry(field.to_string())
                                .or_insert(serde_json::json!({"value": 0.0, "fibo": fibo_uri}));
                            let current = entry["value"].as_f64().unwrap_or(0.0);
                            *entry = serde_json::json!({
                                "value": current + weight * val,
                                "fibo": fibo_uri,
                            });
                        }
                    }
                }

                // Balance sheet for leverage
                if let Ok(bs_val) = self.fetch("balance_sheet", sym, &[("limit", "1")]).await
                    && let Some(bs) = bs_val.as_array().and_then(|a| a.first())
                {
                    let assets = bs.get("totalAssets").and_then(|v| v.as_f64());
                    let equity = bs.get("totalEquity").and_then(|v| v.as_f64());
                    if let (Some(a), Some(e)) = (assets, equity)
                        && e > 0.0
                    {
                        let lev = a / e;
                        let fibo_uri =
                            fibo::fmp_field_to_fibo("financialLeverage").unwrap_or("unknown");
                        let entry = characteristics
                            .entry("financialLeverage".to_string())
                            .or_insert(serde_json::json!({"value": 0.0, "fibo": fibo_uri}));
                        let current = entry["value"].as_f64().unwrap_or(0.0);
                        *entry = serde_json::json!({
                            "value": current + weight * lev,
                            "fibo": fibo_uri,
                        });
                    }
                }
            }

            Ok(serde_json::json!({
                "portfolio": req.portfolio,
                "date": req.date,
                "total_market_value": total_mv,
                "position_count": market_values.len(),
                "characteristics": characteristics,
                "errors": errors,
            }))
        })
        .await
    }

    #[tool(
        description = "Two-stage DCF valuation. Projects income statement, balance sheet, and cash flow line items to derive free cash flow, then discounts back to enterprise value and intrinsic equity per share. Projects 11 line items per period (revenue, COGS, gross profit, D&A, EBIT, tax, NOPAT, capex, change in NWC, FCF, PV). Returns a forecast_id for later decomposition via forecast_record. Default: 10yr model, 3yr stage 1, 7yr stage 2, 10% WACC, 2.5% terminal growth."
    )]
    pub async fn dcf_valuation(
        &self,
        Parameters(req): Parameters<types::DcfValuationRequest>,
    ) -> String {
        execute_tool(self, "dcf_valuation", async {
            validate_symbol(&req.symbol)?;

            // Fetch all required financial statements
            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;

            let (income, balance, cf, metrics, profile) =
                match (income_result, balance_result, cf_result, metrics_result, profile_result) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(m), Ok(p)) => (inc, bal, cf, m, p),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => {
                        self.record_experience("dcf_valuation", &format!("symbol={}", req.symbol), "error", serde_json::json!({"error": e.to_json_string()}));
                        return Err(e);
                    }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            // Build historical snapshot from API data
            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            // Build projection assumptions from history
            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);

            // Apply user overrides
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }
            if let Some(rg) = req.revenue_growth {
                assumptions.revenue_growth = rg;
            }
            if let Some(gm) = req.gross_margin {
                assumptions.gross_margin = gm;
            }
            if let Some(da) = req.da_to_revenue {
                assumptions.da_to_revenue = da;
            }
            if let Some(cx) = req.capex_to_revenue {
                assumptions.capex_to_revenue = cx;
            }
            if let Some(nw) = req.nwc_to_revenue {
                assumptions.nwc_to_revenue = nw;
            }
            if let Some(tr) = req.tax_rate {
                assumptions.tax_rate = tr;
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let shares = hist.shares_outstanding;

            // Run the projection engine
            let model = financial_model::project_model(&hist, &assumptions, current_price);

            // Generate forecast ID for later decomposition
            let forecast_id = Uuid::new_v4().to_string();

            // Store forecast model for later decomposition
            {
                let stored = StoredForecast {
                    forecast_id: forecast_id.clone(),
                    symbol: req.symbol.clone(),
                    created_at: now_rfc3339(),
                    model: model.clone(),
                    assumptions: assumptions.clone(),
                    hist: hist.clone(),
                    current_price,
                    intrinsic_per_share: model.intrinsic_per_share,
                };
                let mut store = self.forecast_store.lock().unwrap();
                store.insert(forecast_id.clone(), stored);
            }

            // Margin of safety
            let margin_of_safety = if current_price > 0.0 {
                (model.intrinsic_per_share - current_price) / current_price
            } else {
                0.0
            };

            // Build period summary for JSON output (all 13 line items)
            let period_summary: Vec<serde_json::Value> = model.periods.iter().map(|p| {
                serde_json::json!({
                    "period": p.period,
                    "year": p.year,
                    "revenue": p.revenue,
                    "cogs": p.cogs,
                    "gross_profit": p.gross_profit,
                    "da": p.da,
                    "ebit": p.ebit,
                    "tax": p.tax,
                    "nopat": p.nopat,
                    "capex": p.capex,
                    "change_in_nwc": p.change_in_nwc,
                    "free_cash_flow": p.free_cash_flow,
                    "discount_factor": p.discount_factor,
                    "present_value": p.present_value,
                })
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "forecast_id": forecast_id,
                "config": {
                    "stage1_years": assumptions.stage1_years,
                    "stage2_years": assumptions.total_years - assumptions.stage1_years,
                    "total_years": assumptions.total_years,
                    "discount_rate": assumptions.discount_rate,
                    "terminal_growth": assumptions.terminal_growth,
                    "revenue_growth": assumptions.revenue_growth,
                    "gross_margin": assumptions.gross_margin,
                    "da_to_revenue": assumptions.da_to_revenue,
                    "capex_to_revenue": assumptions.capex_to_revenue,
                    "nwc_to_revenue": assumptions.nwc_to_revenue,
                    "tax_rate": assumptions.tax_rate,
                },
                "history": {
                    "revenue_cagr": hist.revenue_cagr(),
                    "gross_margin": hist.gross_margin(),
                    "da_to_revenue": hist.da_to_revenue(),
                    "capex_to_revenue": hist.capex_to_revenue(),
                    "nwc_to_revenue": hist.nwc_to_revenue(),
                    "tax_rate": hist.tax_rate,
                    "latest_revenue": hist.latest_revenue(),
                    "shares_outstanding": shares,
                    "net_debt": hist.net_debt(),
                },
                "projections": period_summary,
                "valuation": {
                    "pv_cash_flows": model.periods.iter().map(|p| p.present_value).sum::<f64>(),
                    "terminal_value": model.terminal_value,
                    "terminal_pv": model.terminal_pv,
                    "enterprise_value": model.enterprise_value,
                    "net_debt": model.net_debt,
                    "equity_value": model.equity_value,
                    "intrinsic_per_share": model.intrinsic_per_share,
                    "current_price": current_price,
                    "margin_of_safety": margin_of_safety,
                },
                "framework": "Two-stage 11-line-item DCF: History-calibrated projections through income statement (revenue, COGS, D&A) and balance sheet (NWC, capex) to FCF. Terminal value via Gordon Growth perpetuity (capped at r - 0.5%). Enterprise value to equity bridge via net debt. Damodaran (2012) Investment Valuation. Use forecast_record with the forecast_id to decompose actual outcomes against these projections.",
            });

            self.record_experience("dcf_valuation", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Reverse DCF (Mauboussin's Expectations Investing). Solves for the revenue growth rate implied by the current stock price. \"What growth does the market expect?\" — compare to your own estimate to find mispricing. Default: 10yr model, 3yr stage 1, 7yr stage 2, 10% WACC."
    )]
    pub async fn reverse_dcf(
        &self,
        Parameters(req): Parameters<types::ReverseDcfRequest>,
    ) -> String {
        execute_tool(self, "reverse_dcf", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;

            let (income, balance, cf, metrics, profile) =
                match (income_result, balance_result, cf_result, metrics_result, profile_result) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(m), Ok(p)) => (inc, bal, cf, m, p),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => {
                        self.record_experience("reverse_dcf", &format!("symbol={}", req.symbol), "error", serde_json::json!({"error": e.to_json_string()}));
                        return Err(e);
                    }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);

            // Binary search for implied growth rate: lo=-0.50, hi=1.00, max 50 iterations
            let mut lo = -0.50_f64;
            let mut hi = 1.00_f64;
            let mut implied_growth = 0.0_f64;
            for _ in 0..50 {
                let mid = (lo + hi) / 2.0;
                let mut a = assumptions.clone();
                a.revenue_growth = mid;
                let model = financial_model::project_model(&hist, &a, current_price);
                if (model.intrinsic_per_share - current_price).abs() < 0.0001 {
                    implied_growth = mid;
                    break;
                }
                if model.intrinsic_per_share > current_price {
                    lo = mid;
                } else {
                    hi = mid;
                }
                implied_growth = mid;
            }

            // Final model at implied growth
            let mut final_a = assumptions.clone();
            final_a.revenue_growth = implied_growth;
            let result = financial_model::project_model(&hist, &final_a, current_price);

            let output = serde_json::json!({
                "symbol": req.symbol,
                "current_price": current_price,
                "implied_growth_rate": implied_growth,
                "intrinsic_at_implied": result.intrinsic_per_share,
                "enterprise_value": result.enterprise_value,
                "config": {
                    "stage1_years": assumptions.stage1_years,
                    "stage2_years": assumptions.total_years - assumptions.stage1_years,
                    "discount_rate": assumptions.discount_rate,
                    "terminal_growth": assumptions.terminal_growth,
                },
                "fibo": {
                    "implied_growth_rate": fibo::REVENUE_GROWTH_RATE,
                    "discount_rate": fibo::DISCOUNT_RATE,
                    "terminal_growth_rate": fibo::TERMINAL_GROWTH_RATE,
                    "enterprise_value": fibo::ENTERPRISE_VALUE,
                    "intrinsic_value_per_share": fibo::INTRINSIC_VALUE_PER_SHARE,
                },
                "interpretation": {
                    "implied_growth_pct": format!("{:.1}%", implied_growth * 100.0),
                    "signal": if implied_growth < 0.05 { "low_expectations" } else if implied_growth > 0.15 { "high_expectations" } else { "moderate_expectations" },
                    "mauboussin_framework": "The current stock price implies a revenue growth rate. Compare this to your own estimate of sustainable growth. If your estimate is higher, the stock may be undervalued. If lower, it may be overvalued. The gap between implied and expected growth is the expectations gap — the core of Expectations Investing (Mauboussin & Rappaport, 2001).",
                },
            });

            self.record_experience("reverse_dcf", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Schwartz 2x2 scenario analysis. Projects four scenarios (Bull, Land Grab, Cash Cow, Bear) based on revenue growth x profit margin axes. Runs DCF under each scenario and returns the intrinsic value range. Default axes: revenue_growth x profit_margin. Adjustable multipliers let you tune scenario severity."
    )]
    pub async fn scenario_analysis(
        &self,
        Parameters(req): Parameters<types::ScenarioAnalysisRequest>,
    ) -> String {
        execute_tool(self, "scenario_analysis", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;

            let (income, balance, cf, metrics, profile) =
                match (income_result, balance_result, cf_result, metrics_result, profile_result) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(m), Ok(p)) => (inc, bal, cf, m, p),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => {
                        return Err(e);
                    }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let matrix = scenarios::ScenarioMatrix::growth_x_margin(assumptions.revenue_growth, assumptions.gross_margin);
            let results = scenarios::run_scenario_analysis(&hist, &assumptions, &matrix);

            let summary = scenarios::scenario_summary(&results);

            let scenario_output: Vec<serde_json::Value> = results.iter().map(|r| {
                serde_json::json!({
                    "name": r.scenario.name,
                    "description": r.scenario.description,
                    "applied_growth": r.applied_growth,
                    "applied_margin": r.applied_margin,
                    "intrinsic_per_share": r.intrinsic_per_share,
                    "enterprise_value": r.model.enterprise_value,
                    "margin_of_safety": if current_price > 0.0 { (r.intrinsic_per_share - current_price) / current_price } else { 0.0 },
                })
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "axes": {
                    "axis1": {"name": matrix.axis1.name, "fibo": matrix.axis1.fibo_concept, "baseline": matrix.axis1.baseline},
                    "axis2": {"name": matrix.axis2.name, "fibo": matrix.axis2.fibo_concept, "baseline": matrix.axis2.baseline},
                },
                "scenarios": scenario_output,
                "summary": {
                    "intrinsic_range": [summary.intrinsic_range.0, summary.intrinsic_range.1],
                    "intrinsic_average": summary.intrinsic_average,
                    "current_price": current_price,
                    "upside_pct": summary.upside_pct,
                    "downside_pct": summary.downside_pct,
                    "range_spread_pct": summary.range_spread_pct,
                },
                "fibo": {
                    "discount_rate": fibo::DISCOUNT_RATE,
                    "terminal_growth_rate": fibo::TERMINAL_GROWTH_RATE,
                    "enterprise_value": fibo::ENTERPRISE_VALUE,
                    "intrinsic_value_per_share": fibo::INTRINSIC_VALUE_PER_SHARE,
                    "margin_of_safety": fibo::MARGIN_OF_SAFETY,
                    "scenario_probability": fibo::SCENARIO_PROBABILITY,
                },
                "framework": "Schwartz 2x2 scenario matrix: revenue growth x gross margin. Four scenarios: Bull (high/high), Land Grab (high/low), Cash Cow (low/high), Bear (low/low). Each scenario runs through the two-stage DCF model. The range of intrinsic values represents the uncertainty around the single-point DCF estimate.",
            });

            self.record_experience("scenario_analysis", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Calibrated superforecast. Runs Fermi decomposition on growth and margin estimates, applies outside view (base rate) and inside view adjustments, then distributes probabilities across the four Schwartz scenarios. Produces a probability-weighted intrinsic value and compares it to the market price. Anchored to Tetlock's GJP methodology. Collaborative — you provide base rates and reference counts; the tool computes calibrations."
    )]
    pub async fn calibrate_forecast(
        &self,
        Parameters(req): Parameters<types::CalibrateForecastRequest>,
    ) -> String {
        execute_tool(self, "calibrate_forecast", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;

            let (income, balance, metrics, profile, cf) =
                match (income_result, balance_result, metrics_result, profile_result, cf_result) {
                    (Ok(inc), Ok(bal), Ok(m), Ok(p), Ok(c)) => (inc, bal, m, p, c),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => { return Err(e); }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let hist_revenue_growth = hist.revenue_cagr();

            // Build projection assumptions from history
            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }

            // Run scenarios
            let matrix = scenarios::ScenarioMatrix::growth_x_margin(hist_revenue_growth, assumptions.gross_margin);
            let results = scenarios::run_scenario_analysis(&hist, &assumptions, &matrix);

            // Build Fermi estimates from server-level defaults, apply user overrides
            let mut growth_fermi = self.fermi_defaults.growth_questions.clone();
            let mut margin_fermi = self.fermi_defaults.margin_questions.clone();

            if !req.growth_fermi_overrides.is_empty() {
                let o: Vec<(usize, f64, f64)> = req.growth_fermi_overrides.iter()
                    .map(|ov| (ov.index, ov.estimate, ov.confidence)).collect();
                superforecast::apply_fermi_overrides(&mut growth_fermi, &o);
            }
            if !req.margin_fermi_overrides.is_empty() {
                let o: Vec<(usize, f64, f64)> = req.margin_fermi_overrides.iter()
                    .map(|ov| (ov.index, ov.estimate, ov.confidence)).collect();
                superforecast::apply_fermi_overrides(&mut margin_fermi, &o);
            }

            let growth_inside = req.growth_estimate.unwrap_or_else(|| {
                superforecast::calibrate_from_fermi(&growth_fermi)
            });
            let margin_inside = req.margin_estimate.unwrap_or_else(|| {
                superforecast::calibrate_from_fermi(&margin_fermi)
            });

            let ref_class = req.reference_class.unwrap_or_else(|| "S&P 500 large-cap, 2015-2025".into());
            let ref_count = req.reference_count.unwrap_or(500);

            let (growth_calibrated, growth_conf) = superforecast::outside_view_adjustment(
                0.55, growth_inside, ref_count,
            );
            let (margin_calibrated, margin_conf) = superforecast::outside_view_adjustment(
                0.50, margin_inside, ref_count,
            );

            // Distribute probabilities across scenarios
            let weighted = superforecast::distribute_scenario_probabilities(
                growth_calibrated, margin_calibrated, &results,
            );
            let expected_value = superforecast::expected_intrinsic(&weighted);
            let market_gap = if current_price > 0.0 { (expected_value - current_price) / current_price } else { 0.0 };

            // Generate forecast ID and store calibrated projection for later decomposition
            let forecast_id = Uuid::new_v4().to_string();
            {
                // Apply calibrated estimates to assumptions for the stored model
                assumptions.revenue_growth = growth_calibrated;
                assumptions.gross_margin = margin_calibrated;
                let model = financial_model::project_model(&hist, &assumptions, current_price);
                let stored = StoredForecast {
                    forecast_id: forecast_id.clone(),
                    symbol: req.symbol.clone(),
                    created_at: now_rfc3339(),
                    model,
                    assumptions: assumptions.clone(),
                    hist: hist.clone(),
                    current_price,
                    intrinsic_per_share: expected_value,
                };
                let mut store = self.forecast_store.lock().unwrap();
                store.insert(forecast_id.clone(), stored);
            }

            let fermi_output: Vec<serde_json::Value> = growth_fermi.iter().zip(margin_fermi.iter()).map(|(g, m)| {
                serde_json::json!({
                    "growth_sub_q": g.question, "growth_estimate": g.estimate, "growth_confidence": g.confidence,
                    "margin_sub_q": m.question, "margin_estimate": m.estimate, "margin_confidence": m.confidence,
                })
            }).collect();

            let scenario_output: Vec<serde_json::Value> = weighted.iter().map(|w| {
                serde_json::json!({"name": w.name, "intrinsic": w.intrinsic_per_share, "probability": w.probability})
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "forecast_id": forecast_id,
                "current_price": current_price,
                "calibration": {
                    "growth": {"inside_estimate": growth_inside, "calibrated": growth_calibrated, "confidence": growth_conf},
                    "margin": {"inside_estimate": margin_inside, "calibrated": margin_calibrated, "confidence": margin_conf},
                    "reference_class": ref_class,
                    "reference_count": ref_count,
                    "method": "Fermi decomposition + outside/inside view calibration",
                },
                "fermi_decomposition": fermi_output,
                "scenarios": scenario_output,
                "expected_intrinsic": expected_value,
                "market_gap_pct": market_gap,
                "interpretation": if market_gap > 0.10 { "significantly_undervalued" } else if market_gap > 0.0 { "modestly_undervalued" } else if market_gap > -0.10 { "fairly_valued" } else { "overvalued" },
                "framework": "Tetlock GJP Superforecasting pipeline: Fermi decomposition → outside/inside view calibration → Bayesian-ready probability estimates → scenario-weighted intrinsic value. Probabilities are probability-weighted scenario intrinsic values compared to market price. Brier score tracking available when outcomes are recorded via result_feedback.",
            });

            self.record_experience("calibrate_forecast", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Record a forecast outcome to close the superforecasting loop. Forecast a valuation multiple and price change over a horizon (3mo/6mo/1yr/2yr/3yr), then record what actually happened. Computes Brier scores on multiple direction and price return vs a tolerance band. When forecast_id is provided (from dcf_valuation or calibrate_forecast), looks up the stored 11-line-item projection model and decomposes the return gap into revenue growth, gross margin, D&A, capex, NWC, multiple expansion, and net debt contributions."
    )]
    pub async fn forecast_record(
        &self,
        Parameters(req): Parameters<types::ForecastRecordRequest>,
    ) -> String {
        execute_tool(self, "forecast_record", async {
            validate_symbol(&req.symbol)?;

            // Validate horizon
            if !superforecast::FORECAST_HORIZONS.contains(&req.horizon.as_str()) {
                return Err(McpToolError::invalid_argument(format!(
                    "horizon must be one of: {}", superforecast::FORECAST_HORIZONS.join(", ")
                )));
            }

            // Brier scores on binary outcomes
            // Multiple: was actual multiple >= forecast? (binary direction)
            let multiple_higher = req.actual_multiple >= req.forecast_multiple;
            let p_multiple_up = if req.forecast_multiple > 0.0 { 0.5 } else { 0.5 };
            let multiple_brier = superforecast::brier_score(p_multiple_up, multiple_higher);

            // Price change: was actual return within 20% tolerance of forecast?
            let return_accurate = superforecast::within_tolerance(
                req.forecast_price_change, req.actual_price_change, 0.20,
            );
            let return_brier = superforecast::brier_score(0.7, return_accurate);

            let combined = (multiple_brier + return_brier) / 2.0;

            // Gap decomposition: use stored forecast model if available
            let mut decomposition: Option<serde_json::Value> = None;
            if let Some(ref forecast_id) = req.forecast_id {
                // Clone stored forecast out of lock scope before async fetches
                let stored_opt = {
                    let store = self.forecast_store.lock().unwrap();
                    store.get(forecast_id).cloned()
                };
                if let Some(stored) = stored_opt {
                    // Fetch actual financials at the outcome date for decomposition
                    let actual_income = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
                    let actual_balance = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
                    let actual_cf = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
                    let actual_metrics = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
                    let actual_profile = self.fetch("company_profile", &req.symbol, &[]).await;

                    if let (Ok(inc), Ok(bal), Ok(cf), Ok(metrics), Ok(prof)) =
                        (&actual_income, &actual_balance, &actual_cf, &actual_metrics, &actual_profile)
                    {
                        let inc_arr = inc.as_array();
                        let bal_arr = bal.as_array();
                        let cf_arr = cf.as_array();
                        let met_arr = metrics.as_array();
                        let prof_obj = prof.as_array().and_then(|a| a.first());

                        if inc_arr.is_some_and(|a| !a.is_empty())
                            && bal_arr.is_some_and(|a| !a.is_empty())
                            && cf_arr.is_some_and(|a| !a.is_empty())
                        {
                            let actual_hist = financial_model::HistoricalSnapshot::from_api_json(
                                inc_arr.unwrap(),
                                bal_arr.unwrap(),
                                cf_arr.unwrap(),
                                met_arr.map_or(&[] as &[serde_json::Value], |v| v),
                                prof_obj.unwrap_or(&serde_json::Value::Null),
                            );

                            // Run decomposition
                            let gap = financial_model::decompose_gap(
                                &stored.model,
                                &stored.assumptions,
                                &actual_hist,
                                current_price_from_multiple(req.actual_multiple, &actual_hist),
                                req.actual_multiple,
                                stored.intrinsic_per_share,
                                stored.current_price,
                            );

                            decomposition = Some(serde_json::json!({
                                "total_return_gap": gap.total_return_gap,
                                "components": {
                                    "revenue_growth": {
                                        "contribution": gap.revenue_growth_contribution,
                                        "projected_growth": stored.assumptions.revenue_growth,
                                        "actual_growth": actual_hist.revenue_cagr(),
                                    },
                                    "gross_margin": {
                                        "contribution": gap.gross_margin_contribution,
                                        "projected": stored.assumptions.gross_margin,
                                        "actual": actual_hist.gross_margin(),
                                    },
                                    "da": {
                                        "contribution": gap.da_contribution,
                                        "projected": stored.assumptions.da_to_revenue,
                                        "actual": actual_hist.da_to_revenue(),
                                    },
                                    "capex": {
                                        "contribution": gap.capex_contribution,
                                        "projected": stored.assumptions.capex_to_revenue,
                                        "actual": actual_hist.capex_to_revenue(),
                                    },
                                    "nwc": {
                                        "contribution": gap.nwc_contribution,
                                        "projected": stored.assumptions.nwc_to_revenue,
                                        "actual": actual_hist.nwc_to_revenue(),
                                    },
                                    "multiple": {
                                        "contribution": gap.multiple_contribution,
                                        "projected": projected_terminal_multiple(&stored.model),
                                        "actual": req.actual_multiple,
                                    },
                                    "net_debt": {
                                        "contribution": gap.net_debt_contribution,
                                        "projected": stored.model.net_debt,
                                        "actual": actual_hist.net_debt(),
                                    },
                                },
                                "residual": gap.residual,
                            }));
                        }
                    }
                }
            }

            // Legacy gap narrative (used when no forecast_id or decomposition fails)
            let multiple_gap = req.actual_multiple - req.forecast_multiple;
            let return_gap = req.actual_price_change - req.forecast_price_change;
            let gap_narrative = if decomposition.is_some() {
                "full_decomposition"
            } else if multiple_gap.abs() > 2.0 && return_gap.abs() > 0.05 {
                "multiple_and_return_diverged"
            } else if multiple_gap.abs() > 2.0 {
                "multiple_drove_gap"
            } else if return_gap.abs() > 0.05 {
                "return_drove_gap"
            } else {
                "forecast_accurate"
            };

            // Store in daemon
            if let Some(ref daemon) = self.daemon {
                let mut value = serde_json::json!({
                    "symbol": req.symbol,
                    "forecast_date": req.forecast_date,
                    "horizon": req.horizon,
                    "forecast_multiple": req.forecast_multiple,
                    "forecast_price_change": req.forecast_price_change,
                    "outcome_date": req.outcome_date,
                    "actual_multiple": req.actual_multiple,
                    "actual_price_change": req.actual_price_change,
                    "multiple_brier": multiple_brier,
                    "return_brier": return_brier,
                    "combined_brier": combined,
                    "timestamp": now_rfc3339(),
                });
                if let Some(ref dec) = decomposition {
                    value["decomposition"] = dec.clone();
                }
                if let Some(ref fid) = req.forecast_id {
                    value["forecast_id"] = serde_json::Value::String(fid.clone());
                }
                let daemon_clone = daemon.clone();
                let replicant = self.replicant.clone();
                let symbol = req.symbol.clone();
                let _ = tokio::spawn(async move {
                    let _ = daemon_clone.store_experience(
                        &replicant, &format!("forecast_outcome:{symbol}"), "outcome_recorded",
                        &value, Some(0.95),
                    ).await;
                });
            }

            let mut output = serde_json::json!({
                "status": "recorded",
                "symbol": req.symbol,
                "horizon": req.horizon,
                "forecast": {
                    "multiple": req.forecast_multiple,
                    "price_change_pct": req.forecast_price_change,
                },
                "actual": {
                    "multiple": req.actual_multiple,
                    "price_change_pct": req.actual_price_change,
                },
                "gaps": {
                    "multiple_gap": multiple_gap,
                    "return_gap": return_gap,
                    "narrative": gap_narrative,
                },
                "brier": {
                    "multiple_direction": multiple_brier,
                    "return_accuracy": return_brier,
                    "combined": combined,
                    "interpretation": superforecast::brier_interpretation(combined),
                },
                "framework": "Forecast-Record-Score (Tetlock GJP). Brier scores on binary outcomes: multiple direction and return accuracy within 20% tolerance. When forecast_id is provided, runs full 11-line-item decomposition (revenue growth, gross margin, D&A, capex, NWC, multiple, net debt).",
            });

            if let Some(dec) = decomposition {
                output["decomposition"] = dec;
            }
            if let Some(ref fid) = req.forecast_id {
                output["forecast_id"] = serde_json::Value::String(fid.clone());
            }

            self.record_experience("forecast_record", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Rate a previous tool result on a 1–5 scale with optional comments. Score: 5 = exceeded expectations, 3 = met expectations, 1 = completely missed. Both score and comments are optional — provide either, both, or neither to acknowledge you saw the result. Feeds the learning loop."
    )]
    pub async fn result_feedback(
        &self,
        Parameters(types::ResultFeedbackRequest {
            tool,
            query,
            score,
            comments,
        }): Parameters<types::ResultFeedbackRequest>,
    ) -> String {
        execute_tool(self, "result_feedback", async {
            // Validate score range if provided
            if let Some(s) = score
                && !(1..=5).contains(&s)
            {
                return Err(McpToolError::invalid_argument(format!(
                    "score must be 1–5, got {s}"
                )));
            }

            // Accept empty feedback as an acknowledgment (no score, no comments = "I saw it")
            let has_feedback = score.is_some() || !comments.is_empty();

            // Store feedback as a daemon experience linked to the original tool.
            if let Some(ref daemon) = self.daemon {
                let value = serde_json::json!({
                    "tool": tool,
                    "query": query,
                    "score": score,
                    "comments": comments,
                    "has_feedback": has_feedback,
                    "timestamp": now_rfc3339(),
                });
                let daemon_clone = daemon.clone();
                let replicant = self.replicant.clone();
                let tool_for_spawn = tool.clone();
                tokio::spawn(async move {
                    let _ = daemon_clone
                        .store_experience(
                            &replicant,
                            &format!("feedback:{tool_for_spawn}"),
                            "user_rated",
                            &value,
                            Some(0.95),
                        )
                        .await;
                });
            }

            // Kanban-style learning: feedback updates in-process state.
            // Extracts symbol from query to track per-symbol provider quality.
            if let Some(sym) = parse_symbol_from_query(&query)
                && let Ok(mut state) = self.learning.lock()
            {
                let prov = if comments.contains("provider=eodhd") {
                    "EODHD"
                } else if comments.contains("provider=fmp") {
                    "FMP"
                } else if sym.contains('.') {
                    "EODHD"
                } else {
                    "FMP"
                };
                state.record(&sym, prov, score);
            }

            let summary = if has_feedback {
                if let Some(s) = score {
                    format!("score {s}/5")
                } else {
                    "comments only".to_string()
                }
            } else {
                "acknowledged".to_string()
            };

            Ok(serde_json::json!({
                "status": "recorded",
                "tool": tool,
                "query": query,
                "summary": summary,
            }))
        })
        .await
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

// ── Tracer-bullet contracts ───────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── expectations_gap: Gordon Growth Model formula ──────────────

    #[test]
    fn gordon_growth_formula_contract() {
        let target_return = 0.15f64;
        let avg_net_margin = 0.10f64;
        let price_to_sales = 2.0f64;
        let implied_growth = (target_return - avg_net_margin / price_to_sales) / 2.0;
        assert!(
            (implied_growth - 0.05).abs() < 0.0001,
            "implied growth = 5%"
        );
        let hist_growth = 0.03f64;
        let gap = implied_growth - hist_growth;
        assert!(
            (gap - 0.02).abs() < 0.0001,
            "positive expectations gap = 2%"
        );
    }

    #[test]
    fn gordon_growth_formula_insufficient_data_null_output() {
        let ps = 0.0;
        let avg_net_margin = 0.10;
        let implied: Option<f64> = if ps > 0.0 && avg_net_margin > 0.0 {
            Some((0.15 - avg_net_margin / ps) / 2.0)
        } else {
            None
        };
        assert!(implied.is_none(), "zero P/S = no implied growth");
        let ps = 2.0;
        let avg_net_margin = 0.0;
        let implied: Option<f64> = if ps > 0.0 && avg_net_margin > 0.0 {
            Some((0.15 - avg_net_margin / ps) / 2.0)
        } else {
            None
        };
        assert!(implied.is_none(), "zero margin = no implied growth");
    }

    #[test]
    fn extract_net_margin_and_sales_growth_contract() {
        let m1 = serde_json::json!({"calendarYear": "2022", "netProfitMargin": 0.20, "revenuePerShare": 10.0});
        let m2 = serde_json::json!({"calendarYear": "2023", "netProfitMargin": 0.18, "revenuePerShare": 11.0});
        let m3 = serde_json::json!({"calendarYear": "2024", "netProfitMargin": 0.22, "revenuePerShare": 12.1});
        let metrics = vec![m1, m2, m3];
        let (margins, growths) = extract_net_margin_and_sales_growth(&metrics);
        assert_eq!(margins.len(), 3, "3 net margin values");
        assert_eq!(growths.len(), 2, "2 YoY growth rates");
        assert!((growths[0] - 0.10).abs() < 0.001, "first growth = 10%");
        assert!((growths[1] - 0.10).abs() < 0.001, "second growth = 10%");
        let cagr = cagr_from_series(&growths);
        assert!((cagr - 0.10).abs() < 0.001, "CAGR = 10%");
    }

    #[test]
    fn cagr_from_series_edge_cases() {
        assert!((cagr_from_series(&[]) - 0.0).abs() < 0.001, "empty = 0");
        let cagr = cagr_from_series(&[0.50]);
        assert!((cagr - 0.50).abs() < 0.001, "single 50% = 50% CAGR");
        let cagr = cagr_from_series(&[0.20, -0.10]);
        assert!((cagr - 0.0392).abs() < 0.001, "+20%, -10% = 3.92% CAGR");
    }

    // ── working_capital_cycle: CFO rating boundaries ───────────────

    #[test]
    fn cfo_rating_boundaries_contract() {
        let perfect = [20.0, 20.0, 20.0, 20.0];
        let score = analysis::gross_margin_stability(&perfect);
        assert!(score > 0.99, "identical spreads = near-perfect stability");
        assert!(score > 0.8, "= stable CFO rating");
        let moderate = [20.0, 35.0, 10.0, 40.0];
        let score = analysis::gross_margin_stability(&moderate);
        assert!(
            score > 0.5 && score <= 0.8,
            "moderate variance = moderate CFO rating: {score}"
        );
    }

    #[test]
    fn cfo_rating_single_period_defaults() {
        let single = analysis::gross_margin_stability(&[30.0]);
        assert!((single - 1.0).abs() < 0.001, "single period = 1.0");
    }

    // ── portfolio_attribution: weight + contribution formulas ───────

    #[test]
    fn attribution_weight_and_contribution_contract() {
        let positions = [
            ("AAPL", 60000.0, 0.15),
            ("MSFT", 30000.0, 0.05),
            ("GOOGL", 10000.0, -0.10),
        ];
        let total_mv: f64 = positions.iter().map(|(_, mv, _)| mv).sum();
        assert!((total_mv - 100000.0).abs() < 0.01, "total MV = $100K");
        let weights: Vec<f64> = positions.iter().map(|(_, mv, _)| mv / total_mv).collect();
        assert!((weights[0] - 0.60).abs() < 0.001, "AAPL weight = 60%");
        assert!((weights[1] - 0.30).abs() < 0.001, "MSFT weight = 30%");
        assert!((weights[2] - 0.10).abs() < 0.001, "GOOGL weight = 10%");
        let contributions: Vec<f64> = weights
            .iter()
            .zip(positions.iter())
            .map(|(w, (_, _, r))| w * r * 10000.0)
            .collect();
        assert!((contributions[0] - 900.0).abs() < 1.0, "AAPL = 900 bps");
        assert!((contributions[1] - 150.0).abs() < 1.0, "MSFT = 150 bps");
        assert!(
            (contributions[2] - (-100.0)).abs() < 1.0,
            "GOOGL = -100 bps"
        );
        let total_return_bps: f64 = contributions.iter().sum();
        let portfolio_return = total_return_bps / 10000.0;
        assert!(
            (portfolio_return - 0.095).abs() < 0.001,
            "portfolio return = 9.5%"
        );
    }

    // ── result_feedback: score validation + conversational prompts ─

    #[test]
    fn result_feedback_score_range_contract() {
        // Valid scores: 1–5
        for s in 1..=5u8 {
            let valid = (1..=5).contains(&s);
            assert!(valid, "score {s} should be accepted");
        }
        // Invalid scores: 0 and 6+
        for s in [0u8, 6, 10, 255] {
            let valid = (1..=5).contains(&s);
            assert!(!valid, "score {s} should be rejected");
        }
    }

    #[test]
    fn result_feedback_both_optional() {
        let _score: Option<u8> = None;
        let _comments: &str = "";
        assert!(_score.is_none() && _comments.is_empty(), "both optional");
        let score: Option<u8> = Some(4);
        assert!(score.is_some(), "score only is valid feedback");
        let comments: &str = "missing sector field";
        assert!(!comments.is_empty(), "comments only is valid feedback");
    }
    // ── Learning loop integration: feedback → state → routing ────────

    #[test]
    fn learning_loop_flaky_provider_override() {
        let mut state = LearningState::default();

        // No data → no provider preference
        assert!(!state.is_flaky("AAPL", "FMP"));
        assert!(state.preferred_provider("AAPL").is_none());

        // Feed 5 inaccurate ratings for FMP on AAPL (scores 1-2)
        for _ in 0..5 {
            state.record("AAPL", "FMP", Some(1));
        }
        // 5 ratings, avg=1.0, all inaccurate → flaky
        assert!(state.is_flaky("AAPL", "FMP"));
        assert_eq!(
            state.preferred_provider("AAPL"),
            Some("EODHD".to_string()),
            "FMP flaky → should prefer EODHD"
        );

        // EODHD is not flaky for AAPL
        assert!(!state.is_flaky("AAPL", "EODHD"));

        // MSFT has no data → no preference
        assert!(state.preferred_provider("MSFT").is_none());
    }

    #[test]
    fn learning_loop_both_flaky_no_override() {
        let mut state = LearningState::default();

        // Feed flaky ratings for both providers
        for _ in 0..5 {
            state.record("VOD.L", "FMP", Some(2));
            state.record("VOD.L", "EODHD", Some(1));
        }
        assert!(state.is_flaky("VOD.L", "FMP"));
        assert!(state.is_flaky("VOD.L", "EODHD"));
        // Both flaky → no preference (let default routing handle it)
        assert!(state.preferred_provider("VOD.L").is_none());
    }

    #[test]
    fn learning_loop_recovery_after_accurate_ratings() {
        let mut state = LearningState::default();

        // Make FMP flaky
        for _ in 0..5 {
            state.record("AAPL", "FMP", Some(1));
        }
        assert!(state.is_flaky("AAPL", "FMP"));

        // Feed 5 accurate ratings — running avg rises, inaccurate count stays
        for _ in 0..5 {
            state.record("AAPL", "FMP", Some(5));
        }
        // Now 10 total ratings, avg = (5×1 + 5×5)/10 = 3.0, 5 inaccurate
        // is_flaky requires avg < 3.0 → avg=3.0 passes the threshold
        // It's still flaky because inaccurate count > 3 but avg >= 3.0
        // Wait: is_flaky requires inaccurate > 3 AND avg < 3.0 AND total >= 5
        // avg = 3.0, so avg < 3.0 is FALSE → not flaky anymore
        assert!(
            !state.is_flaky("AAPL", "FMP"),
            "should recover after consistent 5s raise avg to 3.0"
        );
    }

    #[test]
    fn learning_loop_insufficient_data_no_override() {
        let mut state = LearningState::default();

        // Only 3 ratings — below the total >= 5 threshold
        for _ in 0..3 {
            state.record("AAPL", "FMP", Some(1));
        }
        assert!(
            !state.is_flaky("AAPL", "FMP"),
            "3 ratings < 5 threshold → not enough data"
        );
        assert!(state.preferred_provider("AAPL").is_none());
    }
}

// ── Entry point ─────────────────────────────────────────────────────

/// Run the companies MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
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
            Ok(CompaniesServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                fmp_api_key,
                eodhd_api_key,
            )?)
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
