---
title: "ADR-039: GitCasBundle Dual-Pointer Pattern — Trait Object + Concrete Adapter"
audience: [architects, developers]
last_updated: 2026-06-27
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# ADR-039: GitCasBundle Dual-Pointer Pattern

**Date:** 2026-06-27
**Status:** Superseded (2026-06-27).
**Related:** [ADR-036: gix Migration](ADR-036-gix-migration.md), [ADR-038: Eight-Repo CAS Design](ADR-038-eight-repo-cas-design.md)

## Context

`GitCasBundle` (in `hkask-api/src/git_cas.rs`) is the initialization point for git CAS infrastructure at API startup. It must provide two kinds of access to the git CAS adapter: (1) backup operations through the hexagonal `GitCASPort` trait (put_blob, get_blob, snapshot, verify, etc.), and (2) admin-level git operations like `resolve_ref` and `diff` that are not part of the backup contract but are needed by the API git route (template archive) and CLI git commands.

**Problem Statement:** How should `GitCasBundle` expose the git CAS adapter to both backup service code (which needs a trait object for testability) and surface-level admin code (which needs concrete methods not on the trait)?

**Stakeholders:** API surface code (git route, backup route), CLI admin commands, backup service

**Constraints:** Hexagonal architecture (ports as traits), testability via `MockGitCas`, no unnecessary dynamic dispatch in hot paths

## Decision

**`GitCasBundle` holds both `Arc<dyn GitCASPort>` (trait object for backup operations) AND `Arc<GixCasAdapter>` (concrete type for admin operations).** The two `Arc` pointers are backed by the same `GixCasAdapter` instance — `init_git_cas()` creates one adapter, clones the `Arc`, and coerces one clone to the trait object.

```rust
pub(crate) struct GitCasBundle {
    pub template_adapter: Arc<TemplateCrateLoader>,
    pub git_cas_port: Arc<dyn GitCASPort>,     // backup trait object
    pub gix_cas: Arc<GixCasAdapter>,            // concrete admin adapter
}
```

The trait object (`git_cas_port`) is passed to `BackupService::new(port, config)` and used by all backup routes (snapshot, restore, list, prune, verify). The concrete adapter (`gix_cas`) is used directly by the API git route for `resolve_ref("HEAD")` and the CLI for `diff` operations.

**Alternatives Considered:**

1. **Add `resolve_ref` and `diff` to `GitCASPort`** — Rejected because these are git-level introspection operations, not backup operations. Adding them would bloat the backup contract with concerns unrelated to snapshot/restore/verify. Every `GitCASPort` implementor (including `MockGitCas`) would need to implement git-ref resolution and diff computation — operations that have nothing to do with backup testing.

2. **Single trait object with downcasting** — Rejected because `Arc<dyn GitCASPort>::downcast_ref::<GixCasAdapter>()` requires the `Any` trait, adds runtime failure modes, and loses compile-time safety. The dual-pointer approach makes the distinction explicit at the type level.

3. **Separate admin trait (`GitAdminPort`)** — Rejected as over-engineering. `resolve_ref` and `diff` are two methods used in two call sites. A separate trait adds indirection without proportional benefit.

**Rationale:** The split prevents the backup contract from bloating with git-level operations while still allowing surface-level code to access concrete `gix` functionality. The concrete `Arc<GixCasAdapter>` reference also avoids dynamic dispatch overhead for `resolve_ref` and `diff`, which are called on every template archive request and every `kask git diff` invocation. The `GitCASPort` trait object enables testability — `BackupService` can be tested with `MockGitCas` without a real git repository.

## Consequences

### Positive

- Clean contract: `GitCASPort` contains only backup operations (7 methods); git introspection stays separate
- Testability: `BackupService` accepts `Arc<dyn GitCASPort>`, permitting `MockGitCas` in tests
- No dynamic dispatch in admin paths: `resolve_ref` and `diff` are direct calls on concrete type
- Type-level safety: admin code must explicitly reach for `gix_cas` — can't accidentally use from backup context
- Single backing instance: both pointers share the same `GixCasAdapter` — no resource duplication
- `MockGitCas` stays simple: doesn't need to implement git-ref resolution or diff computation

### Negative

- Two fields in `ApiState` where one might suffice — minor API surface area increase
- Callers must know which pointer to use: `git_cas_port` for backup, `gix_cas` for admin
- If more admin operations are added, the pattern may need to evolve into a dedicated admin trait

### Neutral

- Both `Arc` pointers are cheap (reference-count increment, no allocation)
- `init_git_cas()` returns `Result<GitCasBundle, ApiError>` rather than panicking (P4.1 compliance)
- The pattern is confined to `hkask-api` and `hkask-cli` — internal crates don't see the dual-pointer distinction

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P2.2** (Module extraction) | ✅ | `GitCasBundle` extracted from `ApiState::new()` into dedicated module |
| **P4.1** (Fallibility) | ✅ | `init_git_cas()` returns `Result`, not `expect()` — surfaced as `ApiError::Internal` |
| **C1** (Type worn before tailored) | ✅ | Two distinct `Arc` types worn by `ApiState` based on usage pattern |
| **C5** (Every error variant unique) | ✅ | CAS init failures → `ApiError::Internal`; git operation failures → `GitCasError` |

## Verification

```bash
# Verify GitCasBundle dual-pointer structure
grep -A10 "pub(crate) struct GitCasBundle" crates/hkask-api/src/git_cas.rs

# Verify backup routes use git_cas_port
grep "git_cas_port" crates/hkask-api/src/routes/backup.rs

# Verify git routes use gix_cas
grep "gix_cas" crates/hkask-api/src/routes/git.rs

# Verify BackupService accepts Arc<dyn GitCASPort>
grep "GitCASPort" crates/hkask-services-backup/src/lib.rs

# Run tests
cargo test -p hkask-api -- git
cargo test -p hkask-mcp -- git_cas
```

**Expected Results:**
- `GitCasBundle` has three fields: `template_adapter`, `git_cas_port`, `gix_cas`
- Backup routes reference `state.git_cas_port`; never `state.gix_cas`
- Git routes reference `state.gix_cas` for `resolve_ref`; never for backup operations
- `BackupService::new()` accepts `Arc<dyn GitCASPort>` — trait object, not concrete type
- All tests pass

## Related Documents

- [ADR-036: gix Migration](ADR-036-gix-migration.md) — The concrete `GixCasAdapter` behind both pointers
- [ADR-038: Eight-Repo CAS Design](ADR-038-eight-repo-cas-design.md) — Repo structure accessed by both pointers
- [`crates/hkask-api/src/git_cas.rs`](../../crates/hkask-api/src/git_cas.rs) — Production implementation
- [`crates/hkask-ports/src/git_cas/port.rs`](../../crates/hkask-ports/src/git_cas/port.rs) — `GitCASPort` trait definition

## References

[^hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture (Ports and Adapters).* https://alistair.cockburn.us/hexagonal-architecture/
[^fowler-di]: Fowler, M. (2004). *Inversion of Control Containers and the Dependency Injection pattern.* https://martinfowler.com/articles/injection.html

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-039 — v0.31.0*
