//! Saliency scoring — how relevant is text to an agent's persona or memory?
//!
//! Extracted from `WordRankAlgorithm` so saliency is cleanly callable without
//! running the full compression pipeline. One public function:
//!
//! - `score_against_persona(text, persona_keywords)` → word-overlap score 0.0–1.0
//!
//! The persona scoring reuses `compute_word_frequencies` from WordRankAlgorithm.
//! Memory-based saliency is implemented inline by the MCP server (which has access
//! to the episodic/semantic stores) — see `condenser_score_saliency` tool.
//!
//! NOTE: `score_against_memory` was removed — it was a stub that always returned
//! 0.5 and was never wired by the MCP server. The MCP server implements memory
//! saliency inline because it owns the store handles.

use std::collections::HashMap;

/// Score how salient `text` is against a persona's keyword set.
///
/// Computes TF-IDF-like word overlap: what fraction of the persona's
/// keywords appear in the text, weighted by their frequency in the text.
/// Returns 0.0 (no overlap) to 1.0 (all keywords present with high weight).
///
/// `persona_keywords` should include the agent's charter description terms,
/// capability names, responsibility phrases, and invariant traits.
pub fn score_against_persona(text: &str, persona_keywords: &[&str]) -> f64 {
    if persona_keywords.is_empty() || text.is_empty() {
        return 0.5;
    }
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    let freq = compute_word_frequencies(&words);

    let mut total_weight: f64 = 0.0;
    let mut hits: usize = 0;

    for keyword in persona_keywords {
        let kw = keyword.to_lowercase();
        if text_lower.contains(&kw) {
            hits += 1;
            // Weight by how prominently the keyword appears (TF component)
            let weight = freq.get(kw.as_str()).copied().unwrap_or(0.1);
            total_weight += weight;
        }
    }

    if hits == 0 {
        return 0.0;
    }

    // Normalize: hits/total filtered through the average frequency weight
    let coverage = hits as f64 / persona_keywords.len() as f64;
    let avg_weight = total_weight / hits as f64;

    // Blend coverage and weight — both matter
    (coverage * 0.6 + avg_weight * 0.4).min(1.0)
}

/// Word frequency map — shared with WordRankAlgorithm.
/// Returns 0.0–1.0 normalized frequencies for words with length > 2.
fn compute_word_frequencies(words: &[&str]) -> HashMap<String, f64> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;
    for word in words {
        let w = word.to_lowercase();
        if w.len() > 2 {
            *freq.entry(w).or_insert(0) += 1;
            total += 1;
        }
    }
    if total == 0 {
        return HashMap::new();
    }
    freq.into_iter()
        .map(|(k, v)| (k, v as f64 / total as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_keywords_returns_neutral() {
        assert_eq!(score_against_persona("hello world", &[]), 0.5);
    }

    #[test]
    fn empty_text_returns_neutral() {
        assert_eq!(score_against_persona("", &["test"]), 0.5);
    }

    #[test]
    fn high_overlap_scores_high() {
        let keywords = &["monitor", "alert", "escalation", "curator"];
        let text =
            "The curator monitors alert channels and handles escalation of critical findings";
        let score = score_against_persona(text, keywords);
        assert!(score > 0.5, "expected high score, got {score}");
    }

    #[test]
    fn no_overlap_scores_zero() {
        let keywords = &["monitor", "alert", "escalation"];
        let text = "The weather is nice today and I had a good lunch";
        let score = score_against_persona(text, keywords);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn partial_overlap_scores_between() {
        let keywords = &["monitor", "alert", "escalation", "deploy"];
        let text = "The monitor detected an alert in the system";
        let score = score_against_persona(text, keywords);
        assert!(
            score > 0.0 && score < 1.0,
            "expected partial score, got {score}"
        );
    }
}
