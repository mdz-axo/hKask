//! Text utilities for token estimation and content hashing
//!
//! Shared primitives used across the three-layer DRY system:
//! - Layer 1: Memory recall dedup (hkask-memory)
//! - Layer 2: Session message dedup (hkask-ensemble)
//! - Layer 3: Prompt assembly dedup (hkask-templates)

/// Estimate token count for a string.
///
/// Uses a simple heuristic: ~4 characters per token (English approximation).
/// This is intentionally conservative — actual token counts depend on the
/// model's tokenizer. For precise counting, integrate with tiktoken or
/// the model's tokenizer directly.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Compute BLAKE3 hash of arbitrary data.
///
/// Returns a 32-byte hash suitable for exact deduplication.
/// BLAKE3 is cryptographically secure and extremely fast.
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}
