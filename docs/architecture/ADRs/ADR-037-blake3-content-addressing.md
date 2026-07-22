---
title: "ADR-037: BLAKE3 for Content Addressing in Git CAS"
audience: [architects, developers]
last_updated: 2026-06-27
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle]
---

# ADR-037: BLAKE3 for Content Addressing

**Date:** 2026-06-27
**Status:** Active
**Related:** [ADR-036: gix Migration](ADR-036-gix-migration.md)

## Context

The git CAS backup system must address blob content independently of git's native object IDs. Git's SHA-1 OIDs are git-internal and tied to git's object format (type prefix + length + content). The CAS system needs a content hash that is: (1) independent of git's storage format, (2) usable as a flat-file filename on disk (blobs live in `cas/<hash>`), and (3) fast to compute for frequent integrity verification.

**Problem Statement:** Which hash algorithm should hKask use for content-addressed blob storage in its git CAS system?

**Stakeholders:** Backup service, integrity verification path, memory recall deduplication

**Constraints:** Must produce filenames safe for all filesystems (no `/`, null bytes); must be fast enough for real-time snapshot operations; must be a standard algorithm with strong collision resistance

## Decision

**Use BLAKE3 (32-byte output) for content addressing.** Blob content is hashed with `ContentHash::from_blake3(content)` and stored at `cas/<64-hex-chars>`. Git commit hashes remain SHA-1 (20-byte) as produced natively by `gix`.

**Alternatives Considered:**

1. **SHA-256 (32-byte)** — Rejected primarily on performance. BLAKE3 is ~3–10× faster than SHA-256 on modern CPUs due to SIMD acceleration (AVX-512, AVX2, NEON). Both produce 32-byte outputs with comparable collision resistance, but BLAKE3's speed advantage matters for frequent integrity verification (`verify()` re-hashes every blob) and `put_blob()` calls during snapshot operations.

2. **SHA-1 (20-byte, via gix)** — Rejected because tying content addressing to git's internal OID format couples the CAS system to git's object model. Git OIDs include a type prefix and length header — not a pure content hash. The CAS system needs format-independent addressing so blobs remain verifiable outside git context.

3. **XXH3 (8/16-byte)** — Rejected due to insufficient collision resistance. XXH3 is a non-cryptographic hash designed for hash tables, not content addressing. A collision would silently corrupt backup integrity.

**Rationale:** BLAKE3 produces a compact 32-byte hash suitable for filesystem filenames, with strong collision resistance (128-bit security level). It is the fastest cryptographic hash on modern hardware due to its inherent parallelism and SIMD optimization. The 32-byte output is identical in length to SHA-256 but computed significantly faster. Git's native SHA-1 (20-byte) is used for commit hashes by `gix` (as standard git requires), but blob content addressing uses BLAKE3 independently — the two hash layers serve different purposes: BLAKE3 for content deduplication and integrity verification, SHA-1 for git commit graph structure.

## Consequences

### Positive

- BLAKE3 is ~3–10× faster than SHA-256 on modern CPUs (SIMD-accelerated)
- 32-byte output fits cleanly in filesystem filenames (64 hex characters, no path separators)
- Content-addressed deduplication: identical content → identical hash → single blob on disk
- Integrity verification (`verify()`) re-hashes all blobs at scale without performance penalty
- Format-independent: BLAKE3 hashes remain valid outside git context
- Also used in memory recall deduplication (`recall_dedup::eav_hash`) for cross-system consistency

### Negative

- BLAKE3 is a newer algorithm (2020) with less institutional adoption than SHA-256
- Not FIPS-compliant — relevant only if hKask were deployed in FIPS-required environments
- Two hash families in one system (BLAKE3 + git's SHA-1) adds minor conceptual overhead

### Neutral

- `blake3` Rust crate is pure Rust with no C dependencies (consistent with ADR-036's pure-Rust constraint)
- `ContentHash` and `CommitHash` are distinct value types — no risk of hash confusion at the type level

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P5** (No feature flag) | ✅ | BLAKE3 is always compiled; no conditional feature gating |
| **C1** (Type worn before tailored) | ✅ | `ContentHash([u8; 32])` and `CommitHash([u8; 20])` are distinct newtypes |
| **C5** (Every error variant unique) | ✅ | `GitCasError::NotFound(hash)` for missing blobs, BLAKE3 mismatch in `verify()` |

## Verification

```bash
# Verify BLAKE3 is used in CAS operations
grep -r "from_blake3\|blake3::hash" crates/hkask-mcp/src/git_cas/ --include="*.rs"

# Verify ContentHash is 32 bytes
grep -A2 "pub struct ContentHash" crates/hkask-ports/src/git_cas/types.rs

# Verify BLAKE3 is also used in memory dedup
grep -r "blake3" crates/hkask-memory/src/recall_dedup.rs

# Run CAS tests
cargo test -p hkask-mcp -- git_cas
cargo test -p hkask-memory -- recall_dedup
```

**Expected Results:**
- `from_blake3` used in `put_blob`, `verify`, and `list_tree_recursive` within `GixCasAdapter`
- `ContentHash` wraps `[u8; 32]` — confirmed 32-byte BLAKE3 output
- Memory recall deduplication also uses BLAKE3 for `eav_hash` consistency

## Related Documents

- [ADR-036: gix Migration](ADR-036-gix-migration.md) — Pure-Rust git backend that calls BLAKE3
- ADR-038: Eight-Repo CAS Design (archived — superseded by pod-directory backup model)
- [`crates/hkask-ports/src/git_cas/types.rs`](../../../crates/hkask-ports/src/git_cas/types.rs) — `ContentHash` newtype

## References

[^blake3]: O'Connor, J. et al. (2020). *BLAKE3 — one function, fast everywhere.* https://github.com/BLAKE3-team/BLAKE3
[^sha256]: NIST. (2015). *FIPS PUB 180-4: Secure Hash Standard (SHS).* https://csrc.nist.gov/publications/detail/fips/180/4/final
[^aumasson]: Aumasson, J. et al. (2013). *BLAKE2: simpler, smaller, fast as MD5.* ACNS 2013.

---

*ℏKask v0.31.0 — A Sovereign Chat Client for Human Users — ADR-037 — v0.31.0*
