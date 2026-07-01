---
title: "ADR-036: gix (Pure-Rust Gitoxide) as Git Backend for Content-Addressed Storage"
audience: [architects, developers]
last_updated: 2026-06-27
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle]
---

# ADR-036: gix (Pure-Rust Gitoxide) as Git Backend

**Date:** 2026-06-27
**Status:** Active

## Context

The backup system migrated from an archive-based model (flat file trees with ad-hoc deduplication) to a content-addressed storage (CAS) model built on git. This required a programmatic git library — one that could create blobs, build trees, commit snapshots, and resolve refs without shelling out to a `git` CLI binary.

Two Rust git libraries exist: `gix` (gitoxide, pure Rust) and `git2-rs` (libgit2 C bindings).

**Problem Statement:** Which git library should power hKask's CAS backup system — pure Rust `gix` or C-binding `git2-rs`?

**Stakeholders:** Backup service implementers, CLI/API surface code, cross-compilation users

**Constraints:** Headless-only (P3, §5), no CLI subprocess dependency, musl-target compatibility for static builds

## Decision

**Use `gix` crate v0.81 (gitoxide) — pure Rust, no C dependencies, no CLI subprocess.**

The backup system uses `gix` for all git operations within `GixCasAdapter` (`crates/hkask-mcp/src/git_cas/gix_adapter/`): initializing repositories, writing blob objects, building trees, creating commits, resolving refs, and computing diffs. Snapshot strategy reads files from a `cas/<blake3-hash>` directory, writes each as a git blob object, builds a tree from blob OIDs, and commits — no index file is needed.

**Alternatives Considered:**

1. **`git2-rs` (libgit2 C bindings)** — Rejected because it introduces a C build dependency (cmake, C compiler, OpenSSL) that breaks static musl cross-compilation and violates the principle of a pure-Rust dependency tree. The C ABI also introduces potential memory-safety boundary issues.

2. **CLI subprocess (`std::process::Command("git")`)** — Rejected because it violates P3's headless requirement: the system must not depend on external tooling at runtime. A missing `git` binary would silently break snapshot/restore operations.

**Rationale:** `gix` eliminates the C toolchain dependency entirely, enabling static musl builds for containerized deployments. It provides a safe Rust API for all required git operations with no external binary dependency. The migration from the old archive model to git CAS was driven by the need for automatic deduplication (git's content-addressed object store naturally deduplicates identical blobs) and versioned snapshots (every `snapshot()` call produces an immutable commit with parent tracking).

## Consequences

### Positive

- Pure Rust dependency tree — no C compiler, cmake, or OpenSSL required
- Musl static builds work without cross-compilation toolchain gymnastics
- Headless-compliant: no CLI `git` binary dependency (P3, §5)
- Git-native deduplication: identical blobs share a single object automatically
- Versioned snapshot history via commit DAG with parent tracking
- Safe Rust API — no `unsafe` FFI boundary for memory-critical operations

### Negative

- `gix` is younger than libgit2; API surface may evolve across major versions
- Fewer ecosystem examples and tutorials compared to `git2-rs`
- `gix` v0.81 does not cover 100% of git's feature surface (not needed for CAS operations)

### Neutral

- Both libraries can coexist — `gix` does not conflict with any `git2-rs` usage elsewhere
- Repository format is standard git; `gix`-created repos are readable by any git client
- Snapshot strategy (no index file) means CAS repos are lightweight but not suitable for interactive git workflows

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P3** (§5, Headless) | ✅ | No CLI subprocess — all git operations via `gix` crate API |
| **P5** (No feature flag without activator) | ✅ | `gix` is always compiled; no conditional compilation |
| **P6** (Delete stubs, don't publish) | ✅ | `GixCasAdapter` is a complete implementation — no `todo!()` or `unimplemented!()` |

## Verification

```bash
# Verify gix is the only git dependency
grep -r "git2\|libgit2" Cargo.toml Cargo.lock | wc -l

# Verify no CLI git subprocess calls
grep -r 'Command::new("git")' crates/ --include="*.rs" | wc -l

# Verify GixCasAdapter compiles and tests pass
cargo test -p hkask-mcp -- git_cas
```

**Expected Results:**
- Zero references to `git2` or `libgit2` in dependency tree
- Zero CLI git subprocess invocations
- All `GixCasAdapter` tests pass (put/get blob, snapshot, verify, list_tree, resolve_ref, diff)

## Related Documents

- [ADR-037: BLAKE3 Content Addressing](ADR-037-blake3-content-addressing.md) — BLAKE3 used for content hashing within CAS
- ADR-038: Eight-Repo CAS Design (archived — superseded by pod-directory backup model)
- [`crates/crates/hkask-mcp/src/git_cas/gix_adapter/`](../../crates/crates/hkask-mcp/src/git_cas/gix_adapter/) — Production implementation

## References

[^gix]: Byron, S. et al. (2024). *gitoxide — An idiomatic, lean, fast & safe pure Rust implementation of Git.* https://github.com/GitoxideLabs/gitoxide
[^libgit2]: Vicent, C. et al. (2024). *libgit2 — a portable, pure C implementation of the Git core methods.* https://libgit2.org
[^musl]: musl libc. *A new standard library to power a new generation of Linux-based devices.* https://musl.libc.org

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-036 — v0.31.0*
