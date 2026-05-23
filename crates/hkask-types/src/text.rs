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
///
/// # Examples
///
/// ```
/// use hkask_types::estimate_tokens;
///
/// assert_eq!(estimate_tokens(""), 0);
/// assert_eq!(estimate_tokens("Hello world"), 3); // 11 chars / 4 = 2.75 → 3
/// ```
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Compute BLAKE3 hash of arbitrary data.
///
/// Returns a 32-byte hash suitable for exact deduplication.
/// BLAKE3 is cryptographically secure and extremely fast.
///
/// # Examples
///
/// ```
/// use hkask_types::blake3_hash;
///
/// let hash1 = blake3_hash(b"Hello world");
/// let hash2 = blake3_hash(b"Hello world");
/// assert_eq!(hash1, hash2);
///
/// let hash3 = blake3_hash(b"Hello World"); // different case
/// assert_ne!(hash1, hash3);
/// ```
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_single_char() {
        assert_eq!(estimate_tokens("a"), 1);
    }

    #[test]
    fn test_estimate_tokens_four_chars() {
        assert_eq!(estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_estimate_tokens_five_chars() {
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn test_estimate_tokens_hello_world() {
        assert_eq!(estimate_tokens("Hello world"), 3); // 11 chars / 4 = 2.75 → 3
    }

    #[test]
    fn test_blake3_hash_deterministic() {
        let h1 = blake3_hash(b"Hello world");
        let h2 = blake3_hash(b"Hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_blake3_hash_different_input() {
        let h1 = blake3_hash(b"Hello world");
        let h2 = blake3_hash(b"Hello World");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_blake3_hash_empty() {
        let h1 = blake3_hash(b"");
        let h2 = blake3_hash(b"");
        assert_eq!(h1, h2);
    }
}
