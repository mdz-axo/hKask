//! Web-search-specific ranking: deduplication and re-ranking.
//!
//! Domain-agnostic utilities (`rrf_score`, `parse_age_to_days`,
//! `normalize_date_bucket`) live in `hkask_memory::ranking`.

use hkask_memory::ranking::parse_age_to_days;

use crate::types::{RankedResult, RerankSignal};

pub use hkask_memory::ranking::{normalize_date_bucket, rrf_score};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_result(url: &str, score: f64) -> RankedResult {
        RankedResult {
            title: format!("Title {}", url),
            url: url.to_string(),
            description: None,
            published: None,
            rrf_score: score,
            provider_count: 1,
            providers: vec!["test".into()],
            best_rank: None,
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }
    }

    // REQ: CNS-WEB-RANKING — dedup_results merges incoming into existing, deduplicating by URL
    #[test]
    fn dedup_results_merges_disjoint() {
        let mut existing = vec![mk_result("a.com", 0.9)];
        let incoming = vec![mk_result("b.com", 0.8)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].url, "a.com");
        assert_eq!(existing[1].url, "b.com");
    }

    // REQ: CNS-WEB-RANKING — dedup_results keeps higher-scoring duplicate
    #[test]
    fn dedup_results_keeps_higher_score() {
        let mut existing = vec![mk_result("a.com", 0.9)];
        let incoming = vec![mk_result("a.com", 0.5)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1);
        assert!((existing[0].rrf_score - 0.9).abs() < 0.001);
    }

    // REQ: CNS-WEB-RANKING — dedup_results replaces with higher-scoring incoming duplicate
    #[test]
    fn dedup_results_upgrades_lower_score() {
        let mut existing = vec![mk_result("a.com", 0.5)];
        let incoming = vec![mk_result("a.com", 0.9)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1);
        assert!((existing[0].rrf_score - 0.9).abs() < 0.001);
    }

    // REQ: CNS-WEB-RANKING — dedup_results sorts by rrf_score descending
    #[test]
    fn dedup_results_sorts_by_score() {
        let mut existing = vec![];
        let incoming = vec![
            mk_result("a.com", 0.5),
            mk_result("b.com", 0.9),
            mk_result("c.com", 0.3),
        ];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing[0].url, "b.com");
        assert_eq!(existing[1].url, "a.com");
        assert_eq!(existing[2].url, "c.com");
    }

    // REQ: CNS-WEB-RANKING — apply_rerank boosts recency by age
    #[test]
    fn apply_rerank_recency_boosts() {
        let mut results = vec![mk_result("old.com", 0.5), mk_result("new.com", 0.5)];
        results[1].published = Some("2026-06-10T00:00:00Z".into());
        apply_rerank(&mut results, RerankSignal::Recency);
        assert!(results[0].rrf_score > 0.5, "recent result should get boost");
    }
}
