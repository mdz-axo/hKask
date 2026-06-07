//! Search result ranking and date parsing utilities.
//!
//! Domain-agnostic functions for Reciprocal Rank Fusion scoring and
//! age/date parsing. Web-specific types (`RankedResult`, `dedup_results`,
//! `apply_rerank`) remain in `hkask-mcp-web`.

use chrono::Datelike;

/// Reciprocal Rank Fusion score for a set of rank positions.
///
/// `k` is the smoothing constant (commonly 60). Each rank position is
/// 0-based (rank 0 = first result).
pub fn rrf_score(k: u64, ranks: &[usize]) -> f64 {
    ranks
        .iter()
        .map(|&r| 1.0 / (k as f64 + r as f64 + 1.0))
        .sum()
}

/// Parse a human-readable age string into days.
///
/// Supports: "3 days ago", "2 weeks ago", ISO dates like "2024-01-15",
/// fuzzy dates like "Jan 15, 2024", and "published ..." prefixes.
/// Returns -1.0 for unparseable input.
pub fn parse_age_to_days(age: &str) -> f64 {
    let lower = age.to_lowercase();
    let lower = lower.trim();

    if lower.is_empty() {
        return -1.0;
    }

    // Strip "published" prefix first so that "published 3 days ago"
    // recurses into "3 days ago" instead of hitting the " ago" suffix
    // with "published 3 days" (which fails f64 parsing).
    if let Some(rest) = lower.strip_prefix("published ") {
        return parse_age_to_days(rest);
    }

    if let Some(rest) = lower.strip_suffix(" ago") {
        let rest = rest.trim();
        return parse_relative_age(rest);
    }

    if let Ok(days) = parse_iso_date_to_days(lower) {
        return days;
    }

    parse_fuzzy_date(lower)
}

fn parse_relative_age(rest: &str) -> f64 {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return -1.0;
    }
    let num: f64 = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => return -1.0,
    };
    match parts[1] {
        s if s.starts_with("second") => num / 86400.0,
        s if s.starts_with("minute") => num / 1440.0,
        s if s.starts_with("hour") => num / 24.0,
        s if s.starts_with("day") => num,
        s if s.starts_with("week") => num * 7.0,
        s if s.starts_with("month") => num * 30.0,
        s if s.starts_with("year") => num * 365.0,
        _ => -1.0,
    }
}

fn parse_iso_date_to_days(s: &str) -> Result<f64, ()> {
    let s = s.trim();
    if s.len() < 10 {
        return Err(());
    }
    let year: i32 = s.get(0..4).ok_or(())?.parse().map_err(|_| ())?;
    let month: i32 = s.get(5..7).ok_or(())?.parse().map_err(|_| ())?;
    let day: i32 = s.get(8..10).ok_or(())?.parse().map_err(|_| ())?;

    if !(2000..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(());
    }

    let now = chrono::Utc::now();
    let now_ordinal = now.ordinal0() as i32 + 1;
    let now_yday = now.year() * 366 + now_ordinal;

    let target_ordinal = ordinal_day(year, month, day);
    let target_yday = year * 366 + target_ordinal;

    let diff = now_yday - target_yday;
    if diff < 0 {
        return Ok(0.0);
    }
    Ok(diff as f64)
}

fn ordinal_day(year: i32, month: i32, day: i32) -> i32 {
    let days_in_months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut ordinal = day;
    for m in 1..month {
        ordinal += days_in_months[m as usize];
        if m == 2 && leap {
            ordinal += 1;
        }
    }
    ordinal
}

fn parse_fuzzy_date(s: &str) -> f64 {
    let parts: Vec<&str> = s.split(|c: char| !c.is_alphanumeric()).collect();
    let mut year: Option<i32> = None;
    let mut month: Option<i32> = None;
    let mut day: Option<i32> = None;
    let month_names = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];

    for part in parts {
        if part.is_empty() {
            continue;
        }
        if let Ok(n) = part.parse::<i32>() {
            if (2000..=2100).contains(&n) && year.is_none() {
                year = Some(n);
            } else if (1..=12).contains(&n) && month.is_none() {
                month = Some(n);
            } else if (1..=31).contains(&n) && day.is_none() {
                day = Some(n);
            }
        } else {
            let lower = part.to_lowercase();
            for (i, name) in month_names.iter().enumerate() {
                if lower.starts_with(name) {
                    month = Some((i + 1) as i32);
                    break;
                }
            }
        }
    }

    if let Some(y) = year {
        let m = month.unwrap_or(1);
        let d = day.unwrap_or(1);
        parse_iso_date_to_days(&format!("{y:04}-{m:02}-{d:02}")).unwrap_or(-1.0)
    } else {
        -1.0
    }
}

/// Classify a date string into a human-readable bucket.
///
/// Returns one of: "today", "this week", "this month", "older", "unknown".
pub fn normalize_date_bucket(published: &str) -> &'static str {
    let days = parse_age_to_days(published);
    if days < 0.0 {
        return "unknown";
    }
    if days < 1.0 {
        return "today";
    }
    if days < 7.0 {
        return "this week";
    }
    if days < 31.0 {
        return "this month";
    }
    "older"
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_age_to_days ─────────────────────────────────────────────────

    // P8 invariant: empty string returns -1.0 (unparseable)
    #[test]
    fn parse_age_empty_returns_negative() {
        assert_eq!(
            parse_age_to_days(""),
            -1.0,
            "empty string must be unparseable"
        );
    }

    // P8 invariant: relative age — "N days ago" parses to N
    #[test]
    fn parse_age_relative_days() {
        let result = parse_age_to_days("3 days ago");
        assert!(
            (result - 3.0).abs() < 1e-10,
            "3 days ago must parse to 3.0, got {result}"
        );
    }

    // P8 invariant: relative age — "N weeks ago" parses to N*7
    #[test]
    fn parse_age_relative_weeks() {
        let result = parse_age_to_days("2 weeks ago");
        assert!(
            (result - 14.0).abs() < 1e-10,
            "2 weeks ago must parse to 14.0, got {result}"
        );
    }

    // P8 invariant: relative age — "N months ago" parses to N*30
    #[test]
    fn parse_age_relative_months() {
        let result = parse_age_to_days("6 months ago");
        assert!(
            (result - 180.0).abs() < 1e-10,
            "6 months ago must parse to 180.0, got {result}"
        );
    }

    // P8 invariant: relative age — "N years ago" parses to N*365
    #[test]
    fn parse_age_relative_years() {
        let result = parse_age_to_days("1 year ago");
        assert!(
            (result - 365.0).abs() < 1e-10,
            "1 year ago must parse to 365.0, got {result}"
        );
    }

    // P8 invariant: relative age — hours and minutes convert correctly
    #[test]
    fn parse_age_relative_hours() {
        let result = parse_age_to_days("24 hours ago");
        assert!(
            (result - 1.0).abs() < 1e-10,
            "24 hours ago must parse to 1.0, got {result}"
        );
    }

    // P8 invariant: relative age — seconds convert to fractional days
    #[test]
    fn parse_age_relative_seconds() {
        let result = parse_age_to_days("86400 seconds ago");
        assert!(
            (result - 1.0).abs() < 1e-6,
            "86400 seconds ago must parse to ~1.0, got {result}"
        );
    }

    // P8 invariant: ISO date "YYYY-MM-DD" parses to correct day difference
    #[test]
    fn parse_age_iso_date() {
        let now = chrono::Utc::now();
        let three_days_ago = now - chrono::Duration::days(3);
        let date_str = three_days_ago.format("%Y-%m-%d").to_string();
        let result = parse_age_to_days(&date_str);
        assert!(
            (result - 3.0).abs() < 1.1,
            "ISO date 3 days ago must parse to ~3.0, got {result}"
        );
    }

    // P8 invariant: "published YYYY-MM-DD" prefix is stripped
    #[test]
    fn parse_age_published_prefix() {
        let now = chrono::Utc::now();
        let five_days_ago = now - chrono::Duration::days(5);
        let date_str = format!("published {}", five_days_ago.format("%Y-%m-%d"));
        let result = parse_age_to_days(&date_str);
        assert!(
            (result - 5.0).abs() < 1.1,
            "published prefix must be stripped, got {result}"
        );
    }

    // P8 invariant: "published N days ago" prefix is stripped and parses correctly
    #[test]
    fn parse_age_published_relative() {
        let result = parse_age_to_days("published 3 days ago");
        assert!(
            (result - 3.0).abs() < 1e-10,
            "published + relative must work, got {result}"
        );
    }

    // P8 invariant: fuzzy date "Jan 15, 2024" parses correctly
    #[test]
    fn parse_age_fuzzy_date() {
        let result = parse_age_to_days("Jan 1, 2020");
        // Jan 1, 2020 is well in the past — result must be positive
        assert!(
            result > 0.0,
            "fuzzy date must parse to positive days, got {result}"
        );
    }

    // P8 invariant: completely unparseable input returns -1.0
    #[test]
    fn parse_age_unparseable_returns_negative() {
        assert_eq!(
            parse_age_to_days("gibberish"),
            -1.0,
            "unparseable input must return -1.0"
        );
    }

    // ── normalize_date_bucket ──────────────────────────────────────────────

    // P8 invariant: "0 days ago" → "today"
    #[test]
    fn normalize_date_bucket_today() {
        assert_eq!(normalize_date_bucket("0 days ago"), "today");
    }

    // P8 invariant: "3 days ago" → "this week"
    #[test]
    fn normalize_date_bucket_this_week() {
        assert_eq!(normalize_date_bucket("3 days ago"), "this week");
    }

    // P8 invariant: "10 days ago" → "this month"
    #[test]
    fn normalize_date_bucket_this_month() {
        assert_eq!(normalize_date_bucket("10 days ago"), "this month");
    }

    // P8 invariant: "60 days ago" → "older"
    #[test]
    fn normalize_date_bucket_older() {
        assert_eq!(normalize_date_bucket("60 days ago"), "older");
    }

    // P8 invariant: unparseable → "unknown"
    #[test]
    fn normalize_date_bucket_unknown() {
        assert_eq!(normalize_date_bucket("gibberish"), "unknown");
    }

    // ── rrf_score ───────────────────────────────────────────────────────────

    // P8 invariant: single rank computes 1/(k + rank + 1)
    #[test]
    fn rrf_score_single_rank() {
        let score = rrf_score(60, &[0]);
        let expected = 1.0 / 61.0;
        assert!(
            (score - expected).abs() < 1e-10,
            "rrf_score(60, [0]) = {score}, expected {expected}"
        );
    }

    // P8 invariant: multiple ranks sum correctly
    #[test]
    fn rrf_score_multiple_ranks() {
        let score = rrf_score(60, &[0, 1, 5]);
        let expected = 1.0 / 61.0 + 1.0 / 62.0 + 1.0 / 66.0;
        assert!(
            (score - expected).abs() < 1e-10,
            "rrf_score(60, [0,1,5]) = {score}, expected {expected}"
        );
    }

    // P8 invariant: empty ranks yields zero
    #[test]
    fn rrf_score_empty() {
        assert_eq!(rrf_score(60, &[]), 0.0, "empty ranks must yield 0.0");
    }
}
