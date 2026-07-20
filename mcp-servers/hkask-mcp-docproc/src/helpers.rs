//! Shared math and text helpers used across docproc tool modules.

/// Cosine similarity between two vectors. Consolidated from ocr/semantic.rs (C4).
/// Returns 0.0 if either vector is empty or dimensions mismatch.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
}

/// Approximate token-to-word conversion: 1 word ≈ 1.33 tokens.
/// So tokens ÷ 1.33 = words. This is the standard BPE ratio for English text.
pub(crate) fn tokens_to_words(tokens: usize) -> usize {
    ((tokens as f64) / 1.33) as usize
}

/// Compute (max_words, min_words) from (max_tokens, overlap_tokens).
/// `overlap_tokens` determines the minimum chunk size (hard floor below which
/// a buffer won't flush). Falls back to HkaskSettings::chunk_max_tokens() when
/// max_tokens is None.
pub(crate) fn chunk_word_bounds(
    max_tokens: Option<usize>,
    overlap_tokens: Option<usize>,
) -> (usize, usize) {
    let default_max = HkaskSettings::load().chunk_max_tokens();
    let max_w = tokens_to_words(max_tokens.unwrap_or(default_max));
    let min_w = tokens_to_words(overlap_tokens.unwrap_or(64)).max(max_w / 4);
    (max_w, min_w)
}

/// Serialize (entity_ref, text) pair slice into json.
pub(crate) fn serialize_passages(passages: &[(String, String)]) -> Vec<serde_json::Value> {
    passages
        .iter()
        .map(|(entity_ref, passage_text)| json!({"entity_ref": entity_ref, "text": passage_text}))
        .collect()
}

use crate::*;
