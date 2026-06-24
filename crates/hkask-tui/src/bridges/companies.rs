//! CompaniesDataBridge — trait for financial data in the TUI.
//!
//! Provides the Companies window with live financial data from
//! hkask-mcp-companies (FMP + EODHD dual-provider). The MCP server
//! exposes 25+ tools for company profiles, stock quotes, financial
//! statements, portfolio management, and MAIA analysis.
//!
//! The bridge surfaces a focused subset (≤7 methods per deep-module
//! discipline) covering the four window sections: Search, Profile,
//! Financials, Portfolio.

use std::sync::Arc;

/// Summary of a company for TUI display (from company_profile tool).
#[derive(Debug, Clone)]
pub struct CompanySummary {
    pub symbol: String,
    pub name: String,
    pub exchange: Option<String>,
    pub industry: Option<String>,
    pub sector: Option<String>,
    pub market_cap: Option<f64>,
    pub description: Option<String>,
}

/// Key financial metrics for TUI display (from key_metrics + stock_quote).
#[derive(Debug, Clone)]
pub struct FinancialSummary {
    pub symbol: String,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub revenue_growth: Option<f64>,
}

/// Portfolio summary for TUI display (from portfolio_list).
#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub name: String,
    pub holdings: usize,
    pub created: Option<String>,
}

/// Trait for querying companies/financial subsystem state.
///
/// Implemented by the CLI via MCP tool dispatch to hkask-mcp-companies.
/// Degrades gracefully to placeholder text when the MCP server is not running
/// (Option<Arc<dyn Trait>> is None).
pub trait CompaniesDataBridge: Send + Sync {
    /// Search for a company by symbol or name.
    fn search(&self, query: &str) -> Vec<CompanySummary>;

    /// Get the last searched symbol.
    fn last_searched(&self) -> Option<String>;

    /// Get key financial metrics for the last searched symbol.
    fn financials(&self) -> Option<FinancialSummary>;

    /// List portfolios.
    fn portfolio_list(&self) -> Vec<PortfolioSummary>;
}

/// Mock implementation for TUI development and testing.
pub struct MockCompaniesBridge {
    pub results: Vec<CompanySummary>,
    pub last_query: Option<String>,
    pub financial: Option<FinancialSummary>,
    pub portfolios: Vec<PortfolioSummary>,
}

impl MockCompaniesBridge {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            last_query: None,
            financial: None,
            portfolios: Vec::new(),
        }
    }

    pub fn with_sample() -> Self {
        Self {
            results: vec![
                CompanySummary {
                    symbol: "AAPL".into(),
                    name: "Apple Inc.".into(),
                    exchange: Some("NASDAQ".into()),
                    industry: Some("Consumer Electronics".into()),
                    sector: Some("Technology".into()),
                    market_cap: Some(3_200_000_000_000.0),
                    description: Some(
                        "Designs, manufactures, and markets smartphones, personal computers, \
                         tablets, wearables, and accessories."
                            .into(),
                    ),
                },
                CompanySummary {
                    symbol: "MSFT".into(),
                    name: "Microsoft Corporation".into(),
                    exchange: Some("NASDAQ".into()),
                    industry: Some("Software—Infrastructure".into()),
                    sector: Some("Technology".into()),
                    market_cap: Some(3_100_000_000_000.0),
                    description: Some(
                        "Develops, licenses, and supports software, services, devices, and solutions."
                            .into(),
                    ),
                },
            ],
            last_query: Some("AAPL".into()),
            financial: Some(FinancialSummary {
                symbol: "AAPL".into(),
                price: Some(195.89),
                change_pct: Some(1.2),
                pe_ratio: Some(32.5),
                revenue_growth: Some(5.7),
            }),
            portfolios: vec![
                PortfolioSummary {
                    name: "Tech Holdings".into(),
                    holdings: 12,
                    created: Some("2024-01-15".into()),
                },
                PortfolioSummary {
                    name: "Dividend Growers".into(),
                    holdings: 8,
                    created: Some("2024-03-01".into()),
                },
            ],
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl CompaniesDataBridge for MockCompaniesBridge {
    fn search(&self, _query: &str) -> Vec<CompanySummary> {
        self.results.clone()
    }
    fn last_searched(&self) -> Option<String> {
        self.last_query.clone()
    }
    fn financials(&self) -> Option<FinancialSummary> {
        self.financial.clone()
    }
    fn portfolio_list(&self) -> Vec<PortfolioSummary> {
        self.portfolios.clone()
    }
}
