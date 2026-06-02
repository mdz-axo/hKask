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
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}
