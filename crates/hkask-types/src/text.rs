//! Text utilities — Cross-cutting infrastructure
//
//! Cryptographic hashing used across loops for content-addressing.
//
//! Shared primitive used across the three-layer DRY system:
//! - Layer 1: Memory recall dedup (hkask-memory)

/// Compute BLAKE3 hash of arbitrary data.
///
/// Returns a 32-byte hash suitable for exact deduplication.
/// BLAKE3 is cryptographically secure and extremely fast.
///
/// expect: "System types preserve semantic identity and are provenance-aware"
/// pre:  data is any byte slice, including empty
/// post: returns a deterministic 32-byte BLAKE3 hash; same input always
///       produces the same output
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}
