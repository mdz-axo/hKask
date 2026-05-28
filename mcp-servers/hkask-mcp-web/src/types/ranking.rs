//! Search result ranking, deduplication, and date parsing.

use crate::types::{RRF_K, RankedResult, RerankSignal};
use chrono::Datelike;

pub fn rrf_score(ranks: &[usize]) -> f64 {
    ranks
        .iter()
        .map(|&r| 1.0 / (RRF_K as f64 + r as f64 + 1.0))
        .sum()
}

pub fn parse_age_to_days(age: &str) -> f64 {
    let lower = age.to_lowercase();
    let lower = lower.trim();

    if lower.is_empty() {
        return -1.0;
    }

    if let Some(rest) = lower.strip_suffix(" ago") {
        let rest = rest.trim();
        return parse_relative_age(rest);
    }

    if let Ok(days) = parse_iso_date_to_days(lower) {
        return days;
    }

    if let Some(rest) = lower.strip_prefix("published ") {
        if let Ok(days) = parse_iso_date_to_days(rest) {
            return days;
        }
        if let Some(rest2) = rest.strip_suffix(" ago") {
            return parse_relative_age(rest2.trim());
        }
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

pub fn apply_rerank(results: &mut [RankedResult], signal: RerankSignal) {
    match signal {
        RerankSignal::Recency => {
            for r in results.iter_mut() {
                if let Some(ref published) = r.published {
                    let days = parse_age_to_days(published);
                    if days >= 0.0 {
                        let boost = 1.0 / (1.0 + days / 7.0);
                        r.rrf_score += boost * 0.1;
                    }
                }
            }
        }
        RerankSignal::Semantic => {
            for r in results.iter_mut() {
                if let Some(score) = r.semantic_score {
                    r.rrf_score += score * 0.05;
                }
            }
        }
        RerankSignal::ContentQuality => {
            for r in results.iter_mut() {
                if r.content_preview.is_some() || r.extracted_content.is_some() {
                    r.rrf_score += 0.05;
                }
            }
        }
    }
    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn dedup_results(existing: &mut Vec<RankedResult>, incoming: Vec<RankedResult>) {
    for r in incoming {
        let key = r.url.to_lowercase();
        if let Some(idx) = existing.iter().position(|e| e.url.to_lowercase() == key) {
            if r.rrf_score > existing[idx].rrf_score {
                existing[idx] = r;
            }
        } else {
            existing.push(r);
        }
    }
    existing.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

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

    #[test]
    fn rrf_score_single_rank() {
        let score = rrf_score(&[0]);
        let expected = 1.0 / (RRF_K as f64 + 0.0 + 1.0);
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn rrf_score_multiple_ranks() {
        let score = rrf_score(&[0, 2]);
        let expected = 1.0 / (RRF_K as f64 + 1.0) + 1.0 / (RRF_K as f64 + 3.0);
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn rrf_score_agreement_beats_single_high_rank() {
        let agreement = rrf_score(&[5, 5]);
        let single = rrf_score(&[0]);
        assert!(agreement > single);
    }

    #[test]
    fn parse_age_to_days_relative() {
        let hours = parse_age_to_days("2 hours ago");
        assert!((hours - 2.0 / 24.0).abs() < 0.01);
        let days = parse_age_to_days("3 days ago");
        assert!((days - 3.0).abs() < 0.01);
        let weeks = parse_age_to_days("1 week ago");
        assert!((weeks - 7.0).abs() < 0.01);
    }

    #[test]
    fn parse_age_to_days_iso_date() {
        let days = parse_age_to_days("2024-01-15");
        assert!(days > 0.0);
    }

    #[test]
    fn recency_rerank_boosts_recent() {
        let mut results = vec![
            RankedResult {
                title: "old".into(),
                url: "http://old".into(),
                description: None,
                source: None,
                published: Some("1 year ago".into()),
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
            RankedResult {
                title: "recent".into(),
                url: "http://recent".into(),
                description: None,
                source: None,
                published: Some("1 day ago".into()),
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::Recency);
        assert_eq!(results[0].title, "recent");
    }

    #[test]
    fn semantic_rerank_boosts_high_scores() {
        let mut results = vec![
            RankedResult {
                title: "low".into(),
                url: "http://low".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: Some(0.1),
                extracted_content: None,
            },
            RankedResult {
                title: "high".into(),
                url: "http://high".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: None,
                semantic_score: Some(0.9),
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::Semantic);
        assert_eq!(results[0].title, "high");
    }

    #[test]
    fn content_quality_rerank_boosts_previews() {
        let mut results = vec![
            RankedResult {
                title: "no-preview".into(),
                url: "http://no".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(0),
                content_preview: None,
                semantic_score: None,
                extracted_content: None,
            },
            RankedResult {
                title: "with-preview".into(),
                url: "http://with".into(),
                description: None,
                source: None,
                published: None,
                rrf_score: 0.5,
                provider_count: 1,
                providers: vec!["test".into()],
                best_rank: Some(1),
                content_preview: Some("content".into()),
                semantic_score: None,
                extracted_content: None,
            },
        ];
        apply_rerank(&mut results, RerankSignal::ContentQuality);
        assert_eq!(results[0].title, "with-preview");
    }

    #[test]
    fn dedup_across_iterations() {
        let mut existing = vec![RankedResult {
            title: "t".into(),
            url: "http://x".into(),
            description: None,
            source: None,
            published: None,
            rrf_score: 0.3,
            provider_count: 1,
            providers: vec!["a".into()],
            best_rank: Some(0),
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }];
        let incoming = vec![RankedResult {
            title: "t2".into(),
            url: "http://X".into(),
            description: None,
            source: None,
            published: None,
            rrf_score: 0.5,
            provider_count: 1,
            providers: vec!["b".into()],
            best_rank: Some(0),
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1);
        assert!((existing[0].rrf_score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn normalize_date_bucket_unknown() {
        assert_eq!(normalize_date_bucket(""), "unknown");
        assert_eq!(normalize_date_bucket("gibberish"), "unknown");
    }

    #[test]
    fn normalize_date_bucket_today() {
        assert_eq!(normalize_date_bucket("1 hour ago"), "today");
        assert_eq!(normalize_date_bucket("2 hours ago"), "today");
    }

    #[test]
    fn normalize_date_bucket_this_week() {
        assert_eq!(normalize_date_bucket("3 days ago"), "this week");
    }

    #[test]
    fn normalize_date_bucket_this_month() {
        assert_eq!(normalize_date_bucket("2 weeks ago"), "this month");
    }

    #[test]
    fn normalize_date_bucket_older() {
        assert_eq!(normalize_date_bucket("2 months ago"), "older");
    }
}
