//! hKask MCP FMP — Value-added financial analysis (MAIA framework)
//!
//! Pure functions for computing investment analysis from FMP data.
//! No API calls, no async — these operate on already-fetched JSON values.

use serde_json::Value;

/// Gross margin stability score (0.0–1.0). Higher = more stable.
/// Uses coefficient of variation: lower CV → more stable → higher score.
/// Returns 1.0 for perfect stability, near 0.0 for high volatility.
pub fn gross_margin_stability(margins: &[f64]) -> f64 {
    if margins.len() < 2 {
        return 1.0;
    }
    let mean = margins.iter().sum::<f64>() / margins.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = margins.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / margins.len() as f64;
    let cv = variance.sqrt() / mean.abs();
    // Score: 1.0 / (1.0 + CV). CV of 0 → 1.0, CV of 1.0 → 0.5, CV of 10 → ~0.09
    (1.0 / (1.0 + cv)).clamp(0.0, 1.0)
}

/// Working capital moat signal: DPO − DSO in days.
/// Positive = customers pay faster than you pay suppliers (market power).
pub fn working_capital_spread(dpo_days: f64, dso_days: f64) -> f64 {
    dpo_days - dso_days
}

/// Classify the working capital signal.
pub fn wc_signal_label(spread: f64) -> &'static str {
    if spread > 30.0 {
        "strong_market_power"
    } else if spread > 0.0 {
        "moderate_market_power"
    } else if spread > -15.0 {
        "neutral"
    } else {
        "supplier_dominated"
    }
}

/// Overall moat classification from margin stability and working capital signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoatRating {
    Wide,
    Narrow,
    None,
    InsufficientData,
}

pub fn classify_moat(margin_stability: f64, wc_spread: f64, data_periods: usize) -> MoatRating {
    if data_periods < 3 {
        return MoatRating::InsufficientData;
    }
    let has_stable_margins = margin_stability > 0.7;
    let has_market_power = wc_spread > 0.0;

    if has_stable_margins && has_market_power {
        MoatRating::Wide
    } else if has_stable_margins || has_market_power {
        MoatRating::Narrow
    } else {
        MoatRating::None
    }
}

/// Extract gross margin values from FMP key-metrics JSON array.
/// Returns Vec of (year, grossProfitMargin) sorted by year ascending.
pub fn extract_gross_margins(metrics_json: &Value) -> Vec<(String, f64)> {
    let arr = match metrics_json.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    let mut margins: Vec<(String, f64)> = arr
        .iter()
        .filter_map(|entry| {
            let year = entry.get("calendarYear")?.as_str().unwrap_or("");
            let margin = entry.get("grossProfitMargin")?.as_f64()?;
            Some((year.to_string(), margin))
        })
        .collect();
    margins.sort_by(|a, b| a.0.cmp(&b.0));
    margins
}

/// Compute working capital days (DPO, DSO, DIO) from a set of balance sheet / income
/// statement pairs. Returns (dpo, dso) or None if data is insufficient.
///
/// FMP key-metrics provides daysOfPayablesOutstanding and daysOfSalesOutstanding.
pub fn extract_wc_days(metrics_json: &Value) -> Option<(f64, f64)> {
    let arr = metrics_json.as_array()?;
    let latest = arr.first()?;
    let dpo = latest.get("daysOfPayablesOutstanding")?.as_f64()?;
    let dso = latest.get("daysOfSalesOutstanding")?.as_f64()?;
    Some((dpo, dso))
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: FMP-MOAT — gross_margin_stability returns 1.0 for perfectly stable margins
    #[test]
    fn gross_margin_stability_perfect() {
        let score = gross_margin_stability(&[0.60, 0.60, 0.60, 0.60]);
        assert!((score - 1.0).abs() < 0.001);
    }

    // REQ: FMP-MOAT — gross_margin_stability returns lower score for volatile margins
    #[test]
    fn gross_margin_stability_volatile() {
        let score = gross_margin_stability(&[0.60, 0.20, 0.80, 0.10]);
        assert!(
            score < 0.75,
            "volatile margins should score lower than stable: {score}"
        );
        assert!(
            score > 0.0,
            "volatile margins should not score zero: {score}"
        );
    }

    // REQ: FMP-MOAT — gross_margin_stability handles single data point gracefully
    #[test]
    fn gross_margin_stability_single_point() {
        assert!((gross_margin_stability(&[0.60]) - 1.0).abs() < 0.001);
        assert!((gross_margin_stability(&[]) - 1.0).abs() < 0.001);
    }

    // REQ: FMP-MOAT — gross_margin_stability handles zero mean gracefully
    #[test]
    fn gross_margin_stability_zero_mean() {
        assert!((gross_margin_stability(&[0.0, 0.0, 0.0]) - 0.0).abs() < 0.001);
    }

    // REQ: FMP-MOAT — working_capital_spread computes DPO − DSO
    #[test]
    fn working_capital_spread_computation() {
        assert!((working_capital_spread(90.0, 30.0) - 60.0).abs() < 0.001);
        assert!((working_capital_spread(20.0, 40.0) - (-20.0)).abs() < 0.001);
    }

    // REQ: FMP-MOAT — wc_signal_label classifies spread correctly
    #[test]
    fn wc_signal_label_classification() {
        assert_eq!(wc_signal_label(60.0), "strong_market_power");
        assert_eq!(wc_signal_label(15.0), "moderate_market_power");
        assert_eq!(wc_signal_label(-5.0), "neutral");
        assert_eq!(wc_signal_label(-30.0), "supplier_dominated");
    }

    // REQ: FMP-MOAT — classify_moat returns Wide for both strong stability and positive spread
    #[test]
    fn classify_moat_wide() {
        assert_eq!(classify_moat(0.9, 40.0, 5), MoatRating::Wide);
    }

    // REQ: FMP-MOAT — classify_moat returns Narrow for partial moat signals
    #[test]
    fn classify_moat_narrow() {
        assert_eq!(classify_moat(0.9, -10.0, 5), MoatRating::Narrow);
        assert_eq!(classify_moat(0.3, 40.0, 5), MoatRating::Narrow);
    }

    // REQ: FMP-MOAT — classify_moat returns None for no moat signals
    #[test]
    fn classify_moat_none() {
        assert_eq!(classify_moat(0.3, -10.0, 5), MoatRating::None);
    }

    // REQ: FMP-MOAT — classify_moat returns InsufficientData with fewer than 3 periods
    #[test]
    fn classify_moat_insufficient_data() {
        assert_eq!(classify_moat(0.9, 40.0, 2), MoatRating::InsufficientData);
        assert_eq!(classify_moat(0.3, -10.0, 1), MoatRating::InsufficientData);
    }

    // REQ: FMP-MOAT — extract_gross_margins parses FMP key-metrics JSON
    #[test]
    fn extract_gross_margins_from_json() {
        let json = serde_json::json!([
            {"calendarYear": "2024", "grossProfitMargin": 0.45},
            {"calendarYear": "2023", "grossProfitMargin": 0.43},
            {"calendarYear": "2022", "grossProfitMargin": 0.44},
        ]);
        let margins = extract_gross_margins(&json);
        assert_eq!(margins.len(), 3);
        assert_eq!(margins[0].0, "2022");
        assert_eq!(margins[2].0, "2024");
        assert!((margins[1].1 - 0.44).abs() < 0.001);
    }

    // REQ: FMP-MOAT — extract_gross_margins handles empty/malformed JSON
    #[test]
    fn extract_gross_margins_empty() {
        assert!(extract_gross_margins(&serde_json::json!([])).is_empty());
        assert!(extract_gross_margins(&serde_json::json!({})).is_empty());
    }

    // REQ: FMP-MOAT — extract_wc_days extracts DPO and DSO from key-metrics
    #[test]
    fn extract_wc_days_from_json() {
        let json = serde_json::json!([
            {"calendarYear": "2024", "daysOfPayablesOutstanding": 90.0, "daysOfSalesOutstanding": 30.0},
        ]);
        let (dpo, dso) = extract_wc_days(&json).unwrap();
        assert!((dpo - 90.0).abs() < 0.001);
        assert!((dso - 30.0).abs() < 0.001);
    }

    // REQ: FMP-MOAT — extract_wc_days returns None for missing fields
    #[test]
    fn extract_wc_days_missing_fields() {
        assert!(extract_wc_days(&serde_json::json!([])).is_none());
        assert!(extract_wc_days(&serde_json::json!([{"year": "2024"}])).is_none());
    }
}
