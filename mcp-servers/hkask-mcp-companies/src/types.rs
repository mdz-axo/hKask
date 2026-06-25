//! Request types for hkask-mcp-companies MCP tools.
//!
//! Extracted from main.rs — these are the tool input structs that derive
//! Deserialize + JsonSchema for MCP parameter deserialization.

use schemars::JsonSchema;
use serde::Deserialize;

// ── Financial data request structs ──────────────────────────────────

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

// ── Portfolio request structs ─────────────────────────────────────

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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResultFeedbackRequest {
    /// Which tool produced the result being rated
    pub tool: String,
    /// The query that was used (symbol, portfolio name, search query, etc.)
    pub query: String,
    /// 1–5 satisfaction score (5 = exceeded expectations, 1 = completely missed)
    /// Omit if you just want to leave comments without a score.
    pub score: Option<u8>,
    /// Free-text comments about what worked, what didn't, or what was missing.
    /// Omit if you just want to leave a score without comments.
    #[serde(default)]
    pub comments: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DcfValuationRequest {
    pub symbol: String,
    /// Stage 1 years (1–3, default 3)
    pub stage1_years: Option<u8>,
    /// Stage 2 years (2–7, default 7)
    pub stage2_years: Option<u8>,
    /// Discount rate / WACC (0.0–0.30, default 0.10)
    pub discount_rate: Option<f64>,
    /// Terminal growth rate (0.0–0.10, default 0.025)
    pub terminal_growth: Option<f64>,
    /// Terminal method: "perpetuity" or "multiple"
    pub terminal_method: Option<String>,
    /// Exit multiple (only used when terminal_method = "multiple", default 15.0)
    pub terminal_multiple: Option<f64>,
    /// Projection frequency: "annual" or "quarterly" (default "annual")
    pub frequency: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReverseDcfRequest {
    pub symbol: String,
    /// Stage 1 years (1–3, default 3)
    pub stage1_years: Option<u8>,
    /// Stage 2 years (2–7, default 7)
    pub stage2_years: Option<u8>,
    /// Discount rate / WACC (0.0–0.30, default 0.10)
    pub discount_rate: Option<f64>,
    /// Terminal growth rate (0.0–0.10, default 0.025)
    pub terminal_growth: Option<f64>,
    /// Projection frequency: "annual" or "quarterly" (default "annual")
    pub frequency: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScenarioAnalysisRequest {
    pub symbol: String,
    /// Primary axis: "revenue_growth" or "profit_margin" (default "revenue_growth")
    pub axis1: Option<String>,
    /// Secondary axis: "revenue_growth" or "profit_margin" (default "profit_margin")
    pub axis2: Option<String>,
    /// Override the high multiplier for axis1 (default 1.5)
    pub axis1_high: Option<f64>,
    /// Override the low multiplier for axis1 (default 0.5)
    pub axis1_low: Option<f64>,
    /// Override the high multiplier for axis2 (default 1.2)
    pub axis2_high: Option<f64>,
    /// Override the low multiplier for axis2 (default 0.8)
    pub axis2_low: Option<f64>,
    /// Discount rate (default 0.10)
    pub discount_rate: Option<f64>,
    /// Terminal growth rate (default 0.025)
    pub terminal_growth: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalibrateForecastRequest {
    pub symbol: String,
    /// Your estimate of future revenue growth rate (0.0–1.0).
    /// If omitted, runs Fermi decomposition with default sub-questions.
    pub growth_estimate: Option<f64>,
    /// Your confidence in the growth estimate (0.0–1.0). Default 0.6.
    pub growth_confidence: Option<f64>,
    /// Your estimate of future profit margin (0.0–1.0).
    /// If omitted, runs Fermi decomposition with default sub-questions.
    pub margin_estimate: Option<f64>,
    /// Your confidence in the margin estimate (0.0–1.0). Default 0.6.
    pub margin_confidence: Option<f64>,
    /// Override individual Fermi sub-questions for growth.
    /// Each entry: { "estimate": 0.0-1.0, "confidence": 0.0-1.0 }.
    /// Must provide exactly 4 if overriding. Omitted questions use defaults.
    #[serde(default)]
    pub growth_fermi_overrides: Vec<FermiOverride>,
    /// Override individual Fermi sub-questions for margin.
    #[serde(default)]
    pub margin_fermi_overrides: Vec<FermiOverride>,
    /// Reference class for outside view (e.g., "S&P 500 large-cap tech").
    /// Default: "S&P 500 large-cap, 2015-2025"
    pub reference_class: Option<String>,
    /// Number of reference cases for outside view calibration.
    /// Higher N = more weight on base rate. Default: 500.
    pub reference_count: Option<u64>,
    /// Stage 1 years (1–3, default 3)
    pub stage1_years: Option<u8>,
    /// Stage 2 years (2–7, default 7)
    pub stage2_years: Option<u8>,
    /// Discount rate / WACC (default 0.10)
    pub discount_rate: Option<f64>,
    /// Terminal growth rate (default 0.025)
    pub terminal_growth: Option<f64>,
}

/// Override for a single Fermi sub-question estimate.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FermiOverride {
    /// Index of the sub-question to override (0-3).
    pub index: usize,
    /// New estimate (0.0–1.0).
    pub estimate: f64,
    /// New confidence (0.0–1.0).
    pub confidence: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ForecastRecordRequest {
    pub symbol: String,
    /// When the forecast was made (YYYY-MM-DD)
    pub forecast_date: String,
    /// When the outcome was measured (YYYY-MM-DD)
    pub outcome_date: String,
    /// Actual revenue growth rate over the forecast period (e.g., 0.07 = 7%)
    pub actual_growth: f64,
    /// Actual FCF margin at outcome date (e.g., 0.21 = 21%)
    pub actual_margin: f64,
    /// Actual valuation multiple at outcome date (optional)
    pub actual_multiple: Option<f64>,
}
