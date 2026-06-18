//! Semantic Cross-Validation — Embedding-based similarity for dual-routed pages.
//!
//! Complements Levenshtein similarity with semantic comparison via
//! hkask-inference embedding router. Observation only (P4).

use hkask_inference::EmbeddingRouter;

/// Compute semantic similarity between two OCR results using embeddings.
///
/// Returns `None` if embedding generation fails or texts are empty.
/// Similarity is cosine in [0.0, 1.0].
pub async fn enrich_with_semantic(
    text_a: &str,
    text_b: &str,
    router: &EmbeddingRouter,
    model: &str,
) -> Option<f32> {
    if text_a.trim().is_empty() || text_b.trim().is_empty() {
        return None;
    }

    let embeddings = router
        .embed_sentences(model, &[text_a, text_b])
        .await
        .ok()?;

    if embeddings.len() < 2 {
        return None;
    }

    Some(cosine_similarity(&embeddings[0], &embeddings[1]))
}

/// Cosine similarity between two embedding vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // contract: ocr-semantic-01
    #[test]
    fn identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.001);
    }

    // contract: ocr-semantic-02
    #[test]
    fn orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 0.001);
    }

    // contract: ocr-semantic-03
    #[test]
    fn empty_vector() {
        let sim = cosine_similarity(&[], &[1.0]);
        assert_eq!(sim, 0.0);
    }

    // contract: ocr-semantic-04
    #[test]
    fn different_length_vectors() {
        let sim = cosine_similarity(&[1.0, 2.0], &[1.0]);
        assert_eq!(sim, 0.0);
    }
}
