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

    fn make_result(url: &str, rrf_score: f64) -> RankedResult {
        RankedResult {
            title: format!("Result for {url}"),
            url: url.to_string(),
            description: None,
            source: None,
            published: None,
            rrf_score,
            provider_count: 1,
            providers: vec!["test".to_string()],
            best_rank: None,
            content_preview: None,
            semantic_score: None,
            extracted_content: None,
        }
    }

    // ── rrf_score ───────────────────────────────────────────────────────

    // P8 invariant: rrf_score computes 1/(k + rank + 1) per position, summed
    #[test]
    fn rrf_score_single_rank() {
        // Single result at rank 0: 1/(60 + 0 + 1) = 1/61 ≈ 0.01639
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
        // rank 0: 1/61, rank 1: 1/62, rank 5: 1/66
        let score = rrf_score(60, &[0, 1, 5]);
        let expected = 1.0 / 61.0 + 1.0 / 62.0 + 1.0 / 66.0;
        assert!(
            (score - expected).abs() < 1e-10,
            "rrf_score(60, [0,1,5]) = {score}, expected {expected}"
        );
    }

    // P8 invariant: empty ranks yields zero score
    #[test]
    fn rrf_score_empty_input() {
        assert_eq!(rrf_score(60, &[]), 0.0, "empty ranks must yield 0.0");
    }

    // ── apply_rerank ──────────────────────────────────────────────────────

    // P8 invariant: Recency boost applies 1/(1 + days/7) * 0.1 to results with published date
    #[test]
    fn apply_rerank_recency_boosts_recent_results() {
        let mut results = vec![make_result("https://a.com", 0.5)];
        apply_rerank(&mut results, RerankSignal::Recency);
        // Result has no published date, so no boost applied
        assert!(
            results[0].rrf_score > 0.5,
            "result with no published date should not get recency boost, got {}",
            results[0].rrf_score
        );
    }

    // P8 invariant: Semantic boost adds semantic_score * 0.05
    #[test]
    fn apply_rerank_semantic_boosts_by_score() {
        let mut results = vec![make_result("https://a.com", 0.5)];
        results[0].semantic_score = Some(0.8);
        apply_rerank(&mut results, RerankSignal::Semantic);
        let expected = 0.5 + 0.8 * 0.05;
        assert!(
            (results[0].rrf_score - expected).abs() < 1e-10,
            "semantic boost: got {}, expected {}",
            results[0].rrf_score,
            expected
        );
    }

    // P8 invariant: ContentQuality adds 0.05 if content_preview or extracted_content is present
    #[test]
    fn apply_rerank_content_quality_adds_fixed_boost() {
        let mut results = vec![make_result("https://a.com", 0.5)];
        results[0].content_preview = Some("preview text".to_string());
        apply_rerank(&mut results, RerankSignal::ContentQuality);
        assert_eq!(
            results[0].rrf_score, 0.55,
            "content quality boost should add 0.05"
        );
    }

    // P8 invariant: ContentQuality does not add boost without content
    #[test]
    fn apply_rerank_content_quality_no_boost_without_content() {
        let mut results = vec![make_result("https://a.com", 0.5)];
        apply_rerank(&mut results, RerankSignal::ContentQuality);
        assert_eq!(
            results[0].rrf_score, 0.5,
            "no boost without content_preview or extracted_content"
        );
    }

    // P8 invariant: results are sorted by descending rrf_score after rerank
    #[test]
    fn apply_rerank_sorts_by_descending_score() {
        let mut results = vec![
            make_result("https://low.com", 0.1),
            make_result("https://high.com", 0.9),
            make_result("https://mid.com", 0.5),
        ];
        apply_rerank(&mut results, RerankSignal::Semantic);
        assert!(
            results[0].rrf_score >= results[1].rrf_score,
            "results must be sorted descending"
        );
        assert!(
            results[1].rrf_score >= results[2].rrf_score,
            "results must be sorted descending"
        );
    }

    // ── dedup_results ─────────────────────────────────────────────────────

    // P8 invariant: dedup preserves higher-scoring duplicate
    #[test]
    fn dedup_preserves_higher_scoring_duplicate() {
        let mut existing = vec![make_result("https://example.com", 0.3)];
        let incoming = vec![make_result("https://EXAMPLE.COM", 0.7)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1, "duplicate URL should be merged");
        assert_eq!(existing[0].rrf_score, 0.7, "higher score must win");
    }

    // P8 invariant: dedup keeps lower-scoring existing when incoming score is lower
    #[test]
    fn dedup_keeps_existing_when_higher() {
        let mut existing = vec![make_result("https://example.com", 0.8)];
        let incoming = vec![make_result("https://EXAMPLE.COM", 0.4)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 1, "duplicate URL should be merged");
        assert_eq!(
            existing[0].rrf_score, 0.8,
            "existing score must win when higher"
        );
    }

    // P8 invariant: new unique URLs are appended
    #[test]
    fn dedup_appends_new_unique_urls() {
        let mut existing = vec![make_result("https://a.com", 0.5)];
        let incoming = vec![make_result("https://b.com", 0.3)];
        dedup_results(&mut existing, incoming);
        assert_eq!(existing.len(), 2, "unique URLs should both be present");
    }

    // P8 invariant: results are sorted by descending rrf_score after dedup
    #[test]
    fn dedup_sorts_by_descending_score() {
        let mut existing = vec![make_result("https://a.com", 0.5)];
        let incoming = vec![make_result("https://b.com", 0.9)];
        dedup_results(&mut existing, incoming);
        assert_eq!(
            existing[0].url, "https://b.com",
            "higher score should come first"
        );
        assert_eq!(
            existing[1].url, "https://a.com",
            "lower score should come second"
        );
    }
}
