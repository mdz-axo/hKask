//! FIBO ontology mapping for hkask-mcp-companies.
//!
//! Maps FMP/EODHD API field names to FIBO (Financial Industry Business Ontology)
//! standard concepts. FIBO is the OMG standard for financial data — built by
//! Goldman Sachs, Citigroup, Bloomberg, the Fed, and others. We anchor to FIBO
//! rather than inventing our own taxonomy.
//!
//! Reference: https://spec.edmcouncil.org/fibo/
//!
//! Key FIBO modules used:
//! - fibo-fbc-fct-ra  — Financial Concepts: Financial Ratios (Release)
//! - fibo-sec-sec-ast — Securities: Security Assets (Release)
//! - fibo-be-le-corp  — Business Entities: Corporations (Release)
//! - fibo-fnd-gao-gao — Foundations: Geographies (Release)
//! - fibo-ind-ind-ind — Indices and Indicators: Weighted Averages
//!
//! # Shared Bridge Integration
//!
//! Uses [`hkask_bridge_dublincore`] for entity type classification
//! (e.g., `dctypes:Dataset` for financial data) and [`hkask_bridge_pko`]
//! for financial procedure classification.

use hkask_bridge_dublincore as dc;
use hkask_bridge_pko as pko;

/// A FIBO concept URI — the canonical identifier for a financial data concept.
pub type FiboConcept = &'static str;

// ── FIBO concept constants ──────────────────────────────────────────────

/// Business entity — the company itself.
pub const CORPORATION: FiboConcept = "fibo-be-le-corp:Corporation";
pub const LEGAL_NAME: FiboConcept = "fibo-fnd-utl-alias:legalName";
pub const TICKER_SYMBOL: FiboConcept = "fibo-sec-sec-lst:tickerSymbol";
pub const COUNTRY_OF_INCORPORATION: FiboConcept = "fibo-fnd-arr-arr:CountryOfIncorporation";
pub const INDUSTRY_SECTOR: FiboConcept = "fibo-fnd-gao-gao:IndustrySectorClassification";
pub const INDUSTRY_CLASSIFICATION: FiboConcept = "fibo-fnd-gao-gao:IndustryClassification";

/// Market data.
pub const MARKET_CAPITALIZATION: FiboConcept = "fibo-fbc-fct-ra:MarketCapitalization";

/// Valuation multiples.
pub const PRICE_EARNINGS_RATIO: FiboConcept = "fibo-fbc-fct-ra:PriceEarningsRatio";
pub const PRICE_TO_BOOK_RATIO: FiboConcept = "fibo-fbc-fct-ra:PriceToBookRatio";
pub const PRICE_TO_SALES_RATIO: FiboConcept = "fibo-fbc-fct-ra:PriceToSalesRatio";

/// Profitability.
pub const RETURN_ON_INVESTED_CAPITAL: FiboConcept = "fibo-fbc-fct-ra:ReturnOnInvestedCapital";
pub const RETURN_ON_EQUITY: FiboConcept = "fibo-fbc-fct-ra:ReturnOnEquity";
pub const RETURN_ON_ASSETS: FiboConcept = "fibo-fbc-fct-ra:ReturnOnAssets";
pub const GROSS_PROFIT_MARGIN: FiboConcept = "fibo-fbc-fct-ra:GrossProfitMargin";
pub const OPERATING_PROFIT_MARGIN: FiboConcept = "fibo-fbc-fct-ra:OperatingProfitMargin";
pub const NET_PROFIT_MARGIN: FiboConcept = "fibo-fbc-fct-ra:NetProfitMargin";

/// Leverage.
pub const DEBT_TO_EQUITY_RATIO: FiboConcept = "fibo-fbc-fct-ra:DebtToEquityRatio";
pub const FINANCIAL_LEVERAGE_RATIO: FiboConcept = "fibo-fbc-fct-ra:FinancialLeverageRatio";
pub const TOTAL_ASSETS: FiboConcept = "fibo-fbc-pas-fpas:TotalAssets";
pub const TOTAL_EQUITY: FiboConcept = "fibo-fbc-pas-fpas:TotalEquity";

/// Income.
pub const DIVIDEND_YIELD: FiboConcept = "fibo-fbc-fct-ra:DividendYield";
pub const REVENUE_GROWTH_RATE: FiboConcept = "fibo-fbc-fct-ra:RevenueGrowthRate";
pub const EPS_GROWTH_RATE: FiboConcept = "fibo-fbc-fct-ra:EarningsPerShareGrowthRate";

/// Portfolio concepts.
pub const PORTFOLIO: FiboConcept = "fibo-sec-sec-ast:Portfolio";
pub const SECURITY_HOLDING: FiboConcept = "fibo-sec-sec-ast:SecurityHolding";
pub const HOLDING_WEIGHT: FiboConcept = "fibo-sec-sec-ast:holdingWeight";
pub const WEIGHTED_AVERAGE: FiboConcept = "fibo-ind-ind-ind:WeightedAverage";

// ── FMP/EODHD field → FIBO concept mapping ──────────────────────────────

/// Map an FMP/EODHD API field name to its FIBO concept URI.
/// Returns None for fields not covered by FIBO (provider-specific metadata).
pub fn fmp_field_to_fibo(field: &str) -> Option<FiboConcept> {
    match field {
        // Profile
        "symbol" => Some(TICKER_SYMBOL),
        "companyName" => Some(LEGAL_NAME),
        "sector" => Some(INDUSTRY_SECTOR),
        "industry" => Some(INDUSTRY_CLASSIFICATION),
        "country" => Some(COUNTRY_OF_INCORPORATION),
        "mktCap" => Some(MARKET_CAPITALIZATION),

        // Valuation
        "peRatio" => Some(PRICE_EARNINGS_RATIO),
        "priceToBookRatio" => Some(PRICE_TO_BOOK_RATIO),
        "priceToSalesRatio" => Some(PRICE_TO_SALES_RATIO),

        // Profitability
        "roic" => Some(RETURN_ON_INVESTED_CAPITAL),
        "roe" => Some(RETURN_ON_EQUITY),
        "roa" => Some(RETURN_ON_ASSETS),
        "grossProfitMargin" => Some(GROSS_PROFIT_MARGIN),
        "operatingProfitMargin" => Some(OPERATING_PROFIT_MARGIN),
        "netProfitMargin" => Some(NET_PROFIT_MARGIN),

        // Leverage
        "debtToEquity" => Some(DEBT_TO_EQUITY_RATIO),
        "financialLeverage" => Some(FINANCIAL_LEVERAGE_RATIO),
        "totalAssets" => Some(TOTAL_ASSETS),
        "totalEquity" => Some(TOTAL_EQUITY),

        // Income / growth
        "dividendYield" => Some(DIVIDEND_YIELD),
        "revenueGrowth" => Some(REVENUE_GROWTH_RATE),
        "epsGrowth" => Some(EPS_GROWTH_RATE),

        // Not covered by FIBO (FMP/EODHD-specific metadata)
        _ => None,
    }
}

/// Portfolio characteristics aggregation — FIBO-defined concepts that can be
/// aggregated across holdings as weighted averages.
pub const PORTFOLIO_AGGREGATABLE_FIELDS: &[(&str, FiboConcept)] = &[
    ("peRatio", PRICE_EARNINGS_RATIO),
    ("priceToBookRatio", PRICE_TO_BOOK_RATIO),
    ("priceToSalesRatio", PRICE_TO_SALES_RATIO),
    ("roic", RETURN_ON_INVESTED_CAPITAL),
    ("roe", RETURN_ON_EQUITY),
    ("grossProfitMargin", GROSS_PROFIT_MARGIN),
    ("operatingProfitMargin", OPERATING_PROFIT_MARGIN),
    ("netProfitMargin", NET_PROFIT_MARGIN),
    ("debtToEquity", DEBT_TO_EQUITY_RATIO),
    ("dividendYield", DIVIDEND_YIELD),
    ("revenueGrowth", REVENUE_GROWTH_RATE),
    ("epsGrowth", EPS_GROWTH_RATE),
];

/// Portfolio categorical breakdown fields — FIBO-defined concepts that are
/// aggregated as category → weight maps (not numeric weighted averages).
pub const PORTFOLIO_CATEGORICAL_FIELDS: &[(&str, FiboConcept)] = &[
    ("sector", INDUSTRY_SECTOR),
    ("industry", INDUSTRY_CLASSIFICATION),
    ("country", COUNTRY_OF_INCORPORATION),
];

/// Compute FIBO-defined portfolio weighted average.
///
/// Per FIBO `fibo-ind-ind-ind:WeightedAverage`:
///   Σ(holding_weight × concept_value) for each holding in the portfolio.
pub fn fibo_weighted_average(values_by_weight: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = values_by_weight.iter().map(|(w, _)| w).sum();
    if total_weight <= 0.0 {
        return 0.0;
    }
    values_by_weight
        .iter()
        .map(|(weight, value)| weight * value)
        .sum::<f64>()
        / total_weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmp_to_fibo_maps_all_aggregatable_fields() {
        for (field, _concept) in PORTFOLIO_AGGREGATABLE_FIELDS {
            assert!(
                fmp_field_to_fibo(field).is_some(),
                "aggregatable field '{field}' must have FIBO mapping"
            );
        }
    }

    #[test]
    fn fmp_to_fibo_maps_all_categorical_fields() {
        for (field, _concept) in PORTFOLIO_CATEGORICAL_FIELDS {
            assert!(
                fmp_field_to_fibo(field).is_some(),
                "categorical field '{field}' must have FIBO mapping"
            );
        }
    }

    #[test]
    fn fibo_weighted_average_contract() {
        // 60% weight × 15 P/E + 40% weight × 25 P/E = 19.0 weighted P/E
        let values = [(0.60, 15.0), (0.40, 25.0)];
        let avg = fibo_weighted_average(&values);
        // Σ(weight × value) / Σ(weight) = (0.60×15 + 0.40×25) / (0.60+0.40)
        // = (9.0 + 10.0) / 1.0 = 19.0
        assert!((avg - 19.0).abs() < 0.001, "weighted P/E = 19.0");

        // Equal weights: (10 + 20 + 30) / 3 = 20
        let avg = fibo_weighted_average(&[(1.0, 10.0), (1.0, 20.0), (1.0, 30.0)]);
        assert!((avg - 20.0).abs() < 0.001, "equal weight avg = 20.0");
    }

    #[test]
    fn fibo_unknown_field_returns_none() {
        assert!(fmp_field_to_fibo("someFmpSpecificMetadata").is_none());
        assert!(fmp_field_to_fibo("").is_none());
    }

    #[test]
    fn portfolio_characteristics_fibo_contract() {
        // Simulated portfolio: 2 positions, known weights, known fundamentals.
        // AAPL: 60% weight, P/E=30, ROE=1.5, GrossMargin=0.44
        // MSFT: 40% weight, P/E=35, ROE=0.40, GrossMargin=0.69
        //
        // Weighted P/E:    0.60×30 + 0.40×35 = 18.0 + 14.0 = 32.0
        // Weighted ROE:    0.60×1.5 + 0.40×0.40 = 0.90 + 0.16 = 1.06
        // Weighted Margin: 0.60×0.44 + 0.40×0.69 = 0.264 + 0.276 = 0.54

        let aapl_weight = 0.60;
        let msft_weight = 0.40;

        let pe = fibo_weighted_average(&[(aapl_weight, 30.0), (msft_weight, 35.0)]);
        let roe = fibo_weighted_average(&[(aapl_weight, 1.5), (msft_weight, 0.40)]);
        let margin = fibo_weighted_average(&[(aapl_weight, 0.44), (msft_weight, 0.69)]);

        assert!((pe - 32.0).abs() < 0.01, "weighted P/E = 32.0");
        assert!((roe - 1.06).abs() < 0.01, "weighted ROE = 1.06");
        assert!((margin - 0.54).abs() < 0.01, "weighted margin = 0.54");
    }
}
