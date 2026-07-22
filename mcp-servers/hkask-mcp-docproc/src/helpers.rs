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

/// Chunk a `DocStructure` into passages, respecting heading boundaries.
///
/// Groups blocks under their nearest preceding heading. Each group becomes
/// one or more passages via `SemanticMemory::chunk_text`. When a group exceeds
/// `max_words`, it is split at sentence boundaries within the group. When a
/// group is smaller than `min_words`, it is merged with the next group if
/// possible (to avoid tiny chunks).
///
/// Falls back to flat `chunk_text` when the structure has no headings.
pub(crate) fn chunk_structure(
    structure: &hkask_types::document::DocStructure,
    entity_ref_prefix: &str,
    min_words: usize,
    max_words: usize,
    boundary: &str,
) -> Vec<(String, String)> {
    use hkask_types::document::Block;

    // Collect all blocks across pages, tracking heading starts.
    let blocks: Vec<&Block> = structure.iter_blocks().collect();

    // If no headings, flatten to text and use the standard chunker.
    let has_headings = blocks.iter().any(|b| b.is_heading());
    if !has_headings {
        let flat_text = structure.text();
        return SemanticMemory::chunk_text(
            &flat_text,
            entity_ref_prefix,
            min_words,
            max_words,
            boundary,
        );
    }

    // Group blocks by section (each heading starts a new section).
    let mut sections: Vec<(String, String)> = Vec::new(); // (heading_text, body_text)
    let mut current_heading = String::new();
    let mut current_body = String::new();

    for block in &blocks {
        match block {
            Block::Heading { text, .. } => {
                // Flush previous section
                if !current_body.trim().is_empty() || !current_heading.is_empty() {
                    sections.push((current_heading.clone(), current_body.clone()));
                }
                current_heading = text.clone();
                current_body.clear();
            }
            _ => {
                let block_text = block.text();
                if !current_body.is_empty() {
                    current_body.push_str("\n\n");
                }
                current_body.push_str(&block_text);
            }
        }
    }
    // Flush final section
    if !current_body.trim().is_empty() || !current_heading.is_empty() {
        sections.push((current_heading.clone(), current_body.clone()));
    }

    // Chunk each section, prepending the heading as context.
    let mut passages = Vec::new();
    for (idx, (heading, body)) in sections.iter().enumerate() {
        if body.trim().is_empty() {
            continue;
        }
        // Prepend heading to body so each chunk knows its section.
        let section_text = if heading.is_empty() {
            body.clone()
        } else {
            format!("{heading}\n\n{body}")
        };
        let section_ref = format!("{entity_ref_prefix}:sec{idx}");
        let section_passages =
            SemanticMemory::chunk_text(&section_text, &section_ref, min_words, max_words, boundary);
        passages.extend(section_passages);
    }
    passages
}

use crate::*;
