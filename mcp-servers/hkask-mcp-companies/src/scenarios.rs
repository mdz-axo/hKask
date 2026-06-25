//! Schwartz 2×2 scenario planning for company valuation.
//!
//! Two axes (default: revenue growth × profit margin) produce four scenarios.
//! Each scenario runs through the DCF model to produce a range of intrinsic values.
//!
//! Reference: Schwartz, "The Art of the Long View"

use crate::dcf::{self, CompanyFundamentals, DcfConfig, DcfResult};

// ── 2×2 Matrix ────────────────────────────────────────────────────────────

/// A scenario quadrant in the 2×2 matrix.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: &'static str,
    pub description: &'static str,
    /// Multiplier applied to the primary axis (e.g., revenue growth).
    pub axis1_multiplier: f64,
    /// Multiplier applied to the secondary axis (e.g., profit margin).
    pub axis2_multiplier: f64,
}

/// Scenario axis definition.
#[derive(Debug, Clone)]
pub struct ScenarioAxis {
    /// Human-readable name.
    pub name: &'static str,
    /// FIBO concept (for ontology anchoring).
    pub fibo_concept: &'static str,
    /// Baseline value pulled from company fundamentals.
    pub baseline: f64,
    /// "High" end of the range (multiplier applied to baseline).
    pub high_multiplier: f64,
    /// "Low" end of the range.
    pub low_multiplier: f64,
}

/// Schwartz 2×2 scenario matrix.
#[derive(Debug, Clone)]
pub struct ScenarioMatrix {
    pub axis1: ScenarioAxis,
    pub axis2: ScenarioAxis,
    pub scenarios: [Scenario; 4],
}

impl ScenarioMatrix {
    /// Build a 2×2 matrix around revenue growth (axis1) and profit margin (axis2).
    pub fn growth_x_margin(hist_revenue_growth: f64, fcf_margin: f64) -> Self {
        let axis1 = ScenarioAxis {
            name: "Revenue Growth",
            fibo_concept: "fibo-fbc-fct-ra:RevenueGrowthRate",
            baseline: hist_revenue_growth,
            high_multiplier: 1.5,
            low_multiplier: 0.5,
        };
        let axis2 = ScenarioAxis {
            name: "Profit Margin",
            fibo_concept: "fibo-fbc-fct-ra:NetProfitMargin",
            baseline: fcf_margin,
            high_multiplier: 1.2,
            low_multiplier: 0.8,
        };

        let scenarios = [
            // High growth, high margin — bull case
            Scenario {
                name: "Bull Case",
                description: "Strong revenue growth with expanding margins. The company executes well in a favorable environment.",
                axis1_multiplier: axis1.high_multiplier,
                axis2_multiplier: axis2.high_multiplier,
            },
            // High growth, low margin — land grab
            Scenario {
                name: "Land Grab",
                description: "Revenue grows fast but margins compress. The company invests aggressively for market share at the expense of profitability.",
                axis1_multiplier: axis1.high_multiplier,
                axis2_multiplier: axis2.low_multiplier,
            },
            // Low growth, high margin — cash cow
            Scenario {
                name: "Cash Cow",
                description: "Slow revenue growth with strong margins. The company harvests its position, generating steady cash flow.",
                axis1_multiplier: axis1.low_multiplier,
                axis2_multiplier: axis2.high_multiplier,
            },
            // Low growth, low margin — bear case
            Scenario {
                name: "Bear Case",
                description: "Weak revenue and compressed margins. Competitive pressure or macro headwinds erode both top and bottom line.",
                axis1_multiplier: axis1.low_multiplier,
                axis2_multiplier: axis2.low_multiplier,
            },
        ];

        ScenarioMatrix {
            axis1,
            axis2,
            scenarios,
        }
    }
}

// ── Scenario projection ────────────────────────────────────────────────────

/// A DCF result under a specific scenario.
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub scenario: Scenario,
    pub dcf_result: DcfResult,
    pub applied_growth: f64,
    pub applied_margin: f64,
}

/// Run DCF under all four scenarios and return the range of intrinsic values.
pub fn run_scenario_analysis(
    fundamentals: &CompanyFundamentals,
    matrix: &ScenarioMatrix,
    dcf_config: &DcfConfig,
) -> Result<Vec<ScenarioResult>, String> {
    let mut results = Vec::with_capacity(4);

    for scenario in &matrix.scenarios {
        let applied_growth = matrix.axis1.baseline * scenario.axis1_multiplier;
        let applied_margin = (matrix.axis2.baseline * scenario.axis2_multiplier).clamp(0.01, 0.80);

        let scenario_fundamentals = CompanyFundamentals {
            ttm_revenue: fundamentals.ttm_revenue,
            ttm_fcf: fundamentals.ttm_revenue * applied_margin,
            fcf_margin: applied_margin,
            hist_revenue_growth: applied_growth,
            shares_outstanding: fundamentals.shares_outstanding,
            current_price: fundamentals.current_price,
            market_cap: fundamentals.market_cap,
        };

        let dcf_result = dcf::run_dcf(&scenario_fundamentals, dcf_config)?;

        results.push(ScenarioResult {
            scenario: scenario.clone(),
            dcf_result,
            applied_growth,
            applied_margin,
        });
    }

    Ok(results)
}

/// Summarize scenario results with range and dispersion.
pub fn scenario_summary(results: &[ScenarioResult]) -> ScenarioSummary {
    let intrinsics: Vec<f64> = results
        .iter()
        .map(|r| r.dcf_result.intrinsic_per_share)
        .collect();
    let min_val = intrinsics.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = intrinsics.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg_val = intrinsics.iter().sum::<f64>() / intrinsics.len() as f64;
    let current_price = results
        .first()
        .map(|r| r.dcf_result.current_price)
        .unwrap_or(0.0);

    let upside = if current_price > 0.0 {
        (max_val - current_price) / current_price
    } else {
        0.0
    };
    let downside = if current_price > 0.0 {
        (current_price - min_val) / current_price
    } else {
        0.0
    };
    let range_spread = if avg_val > 0.0 {
        (max_val - min_val) / avg_val
    } else {
        0.0
    };

    ScenarioSummary {
        intrinsic_range: (min_val, max_val),
        intrinsic_average: avg_val,
        current_price,
        upside_pct: upside,
        downside_pct: downside,
        range_spread_pct: range_spread,
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioSummary {
    pub intrinsic_range: (f64, f64),
    pub intrinsic_average: f64,
    pub current_price: f64,
    pub upside_pct: f64,
    pub downside_pct: f64,
    pub range_spread_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fundamentals() -> CompanyFundamentals {
        CompanyFundamentals {
            ttm_revenue: 100_000.0,
            ttm_fcf: 15_000.0,
            fcf_margin: 0.15,
            hist_revenue_growth: 0.08,
            shares_outstanding: 1_000.0,
            current_price: 150.0,
            market_cap: 150_000.0,
        }
    }

    #[test]
    fn four_scenarios_produce_four_results() {
        let f = sample_fundamentals();
        let matrix = ScenarioMatrix::growth_x_margin(f.hist_revenue_growth, f.fcf_margin);
        let results = run_scenario_analysis(&f, &matrix, &DcfConfig::default()).unwrap();
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn bull_case_highest_intrinsic() {
        let f = sample_fundamentals();
        let matrix = ScenarioMatrix::growth_x_margin(f.hist_revenue_growth, f.fcf_margin);
        let results = run_scenario_analysis(&f, &matrix, &DcfConfig::default()).unwrap();

        let bull = &results[0];
        let bear = &results[3];
        assert!(
            bull.dcf_result.intrinsic_per_share > bear.dcf_result.intrinsic_per_share,
            "bull ({:.2}) > bear ({:.2})",
            bull.dcf_result.intrinsic_per_share,
            bear.dcf_result.intrinsic_per_share
        );
    }

    #[test]
    fn summary_computes_range_correctly() {
        let f = sample_fundamentals();
        let matrix = ScenarioMatrix::growth_x_margin(f.hist_revenue_growth, f.fcf_margin);
        let results = run_scenario_analysis(&f, &matrix, &DcfConfig::default()).unwrap();
        let summary = scenario_summary(&results);

        assert!(summary.intrinsic_range.0 > 0.0);
        assert!(summary.intrinsic_range.1 >= summary.intrinsic_range.0);
        assert!(summary.intrinsic_average >= summary.intrinsic_range.0);
        assert!(summary.intrinsic_average <= summary.intrinsic_range.1);
        assert!(summary.range_spread_pct >= 0.0);
    }
}
