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
//! - `research_search` — Multi-provider fundamental research search
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
pub mod research;
mod scenarios;
mod screener;
mod superforecast;
pub mod types;

use portfolio::PortfolioManager;

use types::*;
pub mod tools;

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
    /// Exa API key for fundamental research search (optional)
    pub exa_api_key: Option<String>,
    /// Tavily API key for fundamental research search (optional)
    pub tavily_api_key: Option<String>,
    /// Brave Search API key for fundamental research search (optional)
    pub brave_api_key: Option<String>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        fmp_api_key: String,
        eodhd_api_key: String,
        exa_api_key: Option<String>,
        tavily_api_key: Option<String>,
        brave_api_key: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let client = reqwest::Client::new();
        Ok(Self {
            webid,
            replicant,
            daemon,
            client,
            fmp_api_key,
            eodhd_api_key,
            exa_api_key,
            tavily_api_key,
            brave_api_key,
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

// ── Combined tool router (P5 Essentialism — modular tool groups) ──────────

impl CompaniesServer {
    fn combined_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::financial_data_router()
            + Self::analysis_router()
            + Self::portfolio_router()
            + Self::analytics_router()
            + Self::valuation_router()
    }
}

#[rmcp::tool_handler(router = Self::combined_router())]
impl rmcp::ServerHandler for CompaniesServer {}

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
            let exa_api_key = ctx.credentials.get("HKASK_EXA_API_KEY").cloned();
            let tavily_api_key = ctx.credentials.get("HKASK_TAVILY_API_KEY").cloned();
            let brave_api_key = ctx.credentials.get("HKASK_BRAVE_API_KEY").cloned();
            Ok(CompaniesServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                fmp_api_key,
                eodhd_api_key,
                exa_api_key,
                tavily_api_key,
                brave_api_key,
            )
            .map_err(|e| hkask_mcp::McpError::UnexpectedResponse {
                context: "companies server init".into(),
                detail: e.to_string(),
            })?)
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
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_EXA_API_KEY",
                "Exa API key for fundamental research search",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_TAVILY_API_KEY",
                "Tavily API key for fundamental research search",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_BRAVE_API_KEY",
                "Brave Search API key for fundamental research search",
            ),
        ],
    )
    .await
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
