---
title: "ADR-038: Eight-Repo CAS Design — Isolated Git Repositories per Domain"
audience: [architects, developers]
last_updated: 2026-06-27
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle]
---

# ADR-038: Eight-Repo CAS Design

**Date:** 2026-06-27
**Status:** Superseded by pod-directory backup model (2026-06-27).
**Related:** [ADR-036: gix Migration](ADR-036-gix-migration.md), [ADR-037: BLAKE3 Content Addressing](ADR-037-blake3-content-addressing.md)

## Context

The git CAS backup system must store multiple categories of hKask state: agent registry data, semantic memory triples, CNS audit events, sovereignty consent records, goal specifications, session history, encrypted key material, and pod state. These categories have different snapshot cadences, retention needs, and security sensitivities.

**Problem Statement:** Should all hKask domain state be stored in a single git repository or in separate per-domain repositories?

**Stakeholders:** Backup service, CNS auditor, sovereignty subsystem, pod lifecycle management

**Constraints:** Headless (P3), security boundaries between encrypted and plaintext data, Magna Carta P1 (User Sovereignty) and P4 (Clear Boundaries)

## Decision

**Use 8 separate git repositories — one per `RepoId` variant — each with independent snapshot schedules, retention policies, and security boundaries.**

The repositories and their domain mappings:

| `RepoId` | Domain | Content |
|----------|--------|---------|
| `Registry` | Agent registry | Templates, personas, dispatch manifests |
| `Memory` | Semantic memory | Triples, knowledge graph |
| `CnsAudit` | CNS audit trail | ν-events, variety counters, algedonic alerts |
| `Sovereignty` | User sovereignty | Consent records, OCAP tokens |
| `GoalsSpecs` | Goals & specifications | Goal state, MDS specifications |
| `Sessions` | Session history | Conversation history for standing sessions |
| `Vault` | Encrypted key material | Master key material (encrypted at rest) |
| `Pods` | Agent pod state | `pod.db` snapshots for revert/spawn operations |

Each repo is identified by its `RepoId` variant in `hkask-ports::git_cas::types`. The `GixCasAdapter` resolves the on-disk path as `{HKASK_CAS_HOME}/{dir_name()}/`, creating a standard git repository with a `cas/` subdirectory for BLAKE3-addressed blobs.

**Alternatives Considered:**

1. **Single monolithic repository** — Rejected because it conflates security boundaries (Vault encrypted material would share a git object store with plaintext memory data), forces a single snapshot cadence on all domains (CNS audit needs frequent snapshots; Vault changes rarely), and creates coupling between retention policies (pruning old CNS events should not affect Sovereignty consent records).

2. **Per-artifact-type repositories (more granular than domain)** — Rejected as over-engineering. Eight repos already provide sufficient isolation. Further splitting (e.g., separate repos for templates vs personas within Registry) would increase operational complexity without proportional benefit.

**Rationale:** Domain isolation maps directly to hKask's architectural boundaries. Each repo's snapshot schedule can be tuned independently — `CnsAudit` snapshots every 30 seconds during active CNS traffic, while `Vault` snapshots only on key rotation events. Retention policies are per-repo: CNS audit data ages out after the configured retention window, while Sovereignty consent records are permanent. The `Vault` repo's git objects contain only encrypted material, so even if the git object store is inadvertently exposed, no plaintext key material leaks.

## Consequences

### Positive

- Domain isolation: each repo stores exactly one category of state — clean separation of concerns
- Independent snapshot schedules: `CnsAudit` can snapshot every 30s without overhead on `Vault`
- Independent retention policies: CNS events age out; Sovereignty records persist indefinitely
- Security boundary: `Vault` repo contains only encrypted data — safe even if the object store is read
- Per-repo integrity verification: `verify()` can be scoped to individual repos
- `RepoId::all()` iterates all 8 repos for bulk operations (full backup, full verify)

### Negative

- 8× repository initialization overhead on first run (one-time cost, <1s total)
- Cross-domain queries (e.g., "show CNS events for a specific goal") require joining data from separate repos
- More complex backup configuration: each repo's retention/schedule must be managed

### Neutral

- `RepoId` enum is extensible — new domains can add variants without breaking existing repos
- On-disk directory layout is deterministic: `~/.hkask/repos/{registry,memory,cns-audit,...}/`
- `GitCASPort` trait methods all take `&RepoId` — callers are always explicit about which repo

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (User Sovereignty) | ✅ | Sovereignty consent records isolated in their own repo — not mixed with operational data |
| **P4** (Clear Boundaries) | ✅ | Each `RepoId` variant maps to a single domain boundary |
| **P6** (Delete stubs) | ✅ | All 8 repos fully implemented; `RepoId::all()` returns exactly 8 variants |
| **C5** (Every error variant unique) | ✅ | `GitCasError::NotFound` includes repo context; repo-specific errors are traceable |

## Verification

```bash
# Verify 8 RepoId variants exist
grep -c "/// " crates/hkask-ports/src/git_cas/types.rs | head -1
rustc --edition 2021 -e 'fn main() { println!("{}", std::mem::variant_count::<hkask_ports::git_cas::RepoId>()); }' 2>/dev/null || \
  grep -A20 "pub enum RepoId" crates/hkask-ports/src/git_cas/types.rs | grep -c "///"

# Verify each variant has a dir_name
grep "=> " crates/hkask-ports/src/git_cas/types.rs | grep -c '"'

# Verify RepoId::all() returns 8 elements
grep -A10 "pub fn all" crates/hkask-ports/src/git_cas/types.rs | grep -c "RepoId::"

# Run CAS adapter tests
cargo test -p hkask-mcp -- git_cas
```

**Expected Results:**
- 8 `RepoId` variants: Registry, Memory, CnsAudit, Sovereignty, GoalsSpecs, Sessions, Vault, Pods
- 8 `dir_name()` mappings — each returns a distinct static string
- `RepoId::all()` returns exactly 8 elements in stable order
- All `GixCasAdapter` tests pass across all 8 repos

## Related Documents

- [ADR-036: gix Migration](ADR-036-gix-migration.md) — Pure-Rust git backend
- [ADR-037: BLAKE3 Content Addressing](ADR-037-blake3-content-addressing.md) — Content hash algorithm
- [ADR-039: GitCasBundle Dual Pointer](ADR-039-git-cas-bundle-dual-pointer.md) — How repos are accessed via dual pointers
- [`crates/hkask-ports/src/git_cas/types.rs`](../../crates/hkask-ports/src/git_cas/types.rs) — `RepoId` enum and dir_name

## References

[^evans-ddd]: Evans, E. (2004). *Domain-Driven Design: Tackling Complexity in the Heart of Software.* Addison-Wesley.
[^git-internals]: Chacon, S. & Straub, B. (2014). *Pro Git — Git Internals.* https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-038 — v0.31.0*
