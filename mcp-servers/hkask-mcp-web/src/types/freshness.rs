//! Freshness normalization per provider.

use crate::types::WebError;
use serde::{Deserialize, Serialize};

/// Normalized freshness values at the MCP boundary.
///
/// Each provider adapter translates these to its own parameter format.
/// This follows the Cockburn principle: the port defines the canonical model,
/// adapters translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Freshness {
    Day,
    Week,
    Month,
    Year,
}

impl std::str::FromStr for Freshness {
    type Err = WebError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" | "d" | "1d" | "past_day" | "past day" | "24h" => Ok(Freshness::Day),
            "week" | "w" | "1w" | "past_week" | "past week" | "7d" | "pw" => Ok(Freshness::Week),
            "month" | "m" | "1m" | "past_month" | "past month" | "30d" | "pm" => {
                Ok(Freshness::Month)
            }
            "year" | "y" | "1y" | "past_year" | "past year" | "365d" | "py" => Ok(Freshness::Year),
            _ => Err(WebError::BadArgs(format!(
                "Unknown freshness: {s}. Use: day, week, month, year"
            ))),
        }
    }
}

impl std::fmt::Display for Freshness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Freshness::Day => write!(f, "day"),
            Freshness::Week => write!(f, "week"),
            Freshness::Month => write!(f, "month"),
            Freshness::Year => write!(f, "year"),
        }
    }
}

/// Returns provider-generic key-value pairs for the given freshness value.
///
/// Each provider translates normalized freshness into its own parameter format:
/// - Brave: `freshness=pw` (past week)
/// - Tavily: `days=7`
/// - SerpAPI: `tbs=qdr:w`
pub fn normalize_freshness(freshness: &Freshness) -> Vec<(&'static str, String)> {
    match freshness {
        Freshness::Day => vec![("days", "1".to_string())],
        Freshness::Week => vec![("days", "7".to_string())],
        Freshness::Month => vec![("days", "30".to_string())],
        Freshness::Year => vec![("days", "365".to_string())],
    }
}

/// Map freshness to Brave's parameter format.
pub fn freshness_brave(freshness: &Freshness) -> String {
    match freshness {
        Freshness::Day => "pd".to_string(),
        Freshness::Week => "pw".to_string(),
        Freshness::Month => "pm".to_string(),
        Freshness::Year => "py".to_string(),
    }
}

/// Map freshness to SerpAPI's `tbs` parameter format.
pub fn freshness_serpapi(freshness: &Freshness) -> String {
    match freshness {
        Freshness::Day => "qdr:d".to_string(),
        Freshness::Week => "qdr:w".to_string(),
        Freshness::Month => "qdr:m".to_string(),
        Freshness::Year => "qdr:y".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FromStr round-trips ───────────────────────────────────────────────

    // P8 invariant: canonical names parse correctly
    #[test]
    fn freshness_parses_canonical_names() {
        assert_eq!("day".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("week".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("month".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("year".parse::<Freshness>(), Ok(Freshness::Year));
    }

    // P8 invariant: short aliases parse correctly
    #[test]
    fn freshness_parses_short_aliases() {
        assert_eq!("d".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("1d".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("24h".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("w".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("1w".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("7d".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("pw".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("m".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("1m".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("30d".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("pm".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("y".parse::<Freshness>(), Ok(Freshness::Year));
        assert_eq!("1y".parse::<Freshness>(), Ok(Freshness::Year));
        assert_eq!("365d".parse::<Freshness>(), Ok(Freshness::Year));
        assert_eq!("py".parse::<Freshness>(), Ok(Freshness::Year));
    }

    // P8 invariant: underscore and space variants parse
    #[test]
    fn freshness_parses_underscore_and_space_variants() {
        assert_eq!("past_day".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("past day".parse::<Freshness>(), Ok(Freshness::Day));
        assert_eq!("past_week".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("past week".parse::<Freshness>(), Ok(Freshness::Week));
        assert_eq!("past_month".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("past month".parse::<Freshness>(), Ok(Freshness::Month));
        assert_eq!("past_year".parse::<Freshness>(), Ok(Freshness::Year));
        assert_eq!("past year".parse::<Freshness>(), Ok(Freshness::Year));
    }

    // P8 invariant: invalid strings are rejected
    #[test]
    fn freshness_rejects_invalid_strings() {
        let err = " fortnight".parse::<Freshness>().unwrap_err();
        assert!(
            err.to_string().contains("Unknown freshness"),
            "invalid input must be rejected, got: {err}"
        );
        assert!(
            "century".parse::<Freshness>().is_err(),
            "unknown names must be rejected"
        );
        assert!(
            "".parse::<Freshness>().is_err(),
            "empty string must be rejected"
        );
    }

    // P8 invariant: Display round-trips with canonical names
    #[test]
    fn freshness_display_round_trips() {
        assert_eq!(Freshness::Day.to_string(), "day");
        assert_eq!(Freshness::Week.to_string(), "week");
        assert_eq!(Freshness::Month.to_string(), "month");
        assert_eq!(Freshness::Year.to_string(), "year");

        // Display output should parse back
        for variant in [
            Freshness::Day,
            Freshness::Week,
            Freshness::Month,
            Freshness::Year,
        ] {
            let s = variant.to_string();
            assert_eq!(
                s.parse::<Freshness>(),
                Ok(variant),
                "Display must round-trip"
            );
        }
    }

    // ── Provider mappings ─────────────────────────────────────────────────

    // P8 invariant: normalize_freshness returns correct day counts
    #[test]
    fn normalize_freshness_returns_day_counts() {
        let day = normalize_freshness(&Freshness::Day);
        assert_eq!(day, vec![("days", "1".to_string())]);

        let week = normalize_freshness(&Freshness::Week);
        assert_eq!(week, vec![("days", "7".to_string())]);

        let month = normalize_freshness(&Freshness::Month);
        assert_eq!(month, vec![("days", "30".to_string())]);

        let year = normalize_freshness(&Freshness::Year);
        assert_eq!(year, vec![("days", "365".to_string())]);
    }

    // P8 invariant: freshness_brave maps to Brave parameter format
    #[test]
    fn freshness_brave_maps_correctly() {
        assert_eq!(freshness_brave(&Freshness::Day), "pd");
        assert_eq!(freshness_brave(&Freshness::Week), "pw");
        assert_eq!(freshness_brave(&Freshness::Month), "pm");
        assert_eq!(freshness_brave(&Freshness::Year), "py");
    }

    // P8 invariant: freshness_serpapi maps to SerpAPI tbs parameter
    #[test]
    fn freshness_serpapi_maps_correctly() {
        assert_eq!(freshness_serpapi(&Freshness::Day), "qdr:d");
        assert_eq!(freshness_serpapi(&Freshness::Week), "qdr:w");
        assert_eq!(freshness_serpapi(&Freshness::Month), "qdr:m");
        assert_eq!(freshness_serpapi(&Freshness::Year), "qdr:y");
    }
}
