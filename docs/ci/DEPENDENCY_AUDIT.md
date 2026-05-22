# CI/CD Dependency Audit Gate

## Purpose
Prevent dependency creep by detecting unused dependencies before merge.

## Tools

### cargo-udeps (Unused Dependencies)
```bash
# Install
cargo install cargo-udeps

# Run audit
cargo udeps --workspace --all-targets
```

### cargo-deny (Security & Licensing)
```bash
# Install
cargo install cargo-deny

# Run security audit
cargo deny check advisories

# Run license check
cargo deny check licenses

# Run ban check (blocked crates)
cargo deny check bans
```

## CI Step (GitHub Actions)

```yaml
- name: Dependency Audit
  run: |
    cargo install cargo-udeps
    cargo udeps --workspace --all-targets --exclude hkask-testing
    
- name: Security Audit
  run: |
    cargo install cargo-audit
    cargo audit
```

## Workflow

### Before Adding Dependency
1. Check if workspace already provides equivalent
2. Document justification in `docs/DEPENDENCY_DECISIONS.md`
3. Add to `[workspace.dependencies]` first
4. Update consuming crates to use workspace dep

### Before Removing Dependency
1. Verify zero usage with `cargo udeps`
2. Check all crates with `grep -r "use <crate>" crates/`
3. Remove from `[workspace.dependencies]`
4. Remove from individual crate `Cargo.toml`
5. Run `cargo check --workspace` to verify

## Allowed Deviations

| Scenario | Action | Approval |
|----------|--------|----------|
| **Build tools** | Allow in dev-dependencies | Auto |
| **Security patches** | Immediate update required | Auto |
| **Performance critical** | Document benchmark | Human |
| **Platform-specific** | Mark with cfg | Human |

## Enforcement

- **Weekly**: Automated `cargo udeps` report
- **Pre-merge**: Block on unused deps in production
- **Monthly**: Review `cargo-deny` advisories

---
*Document generated: 2026-05-22*
*Part of hKask Dependency Governance (Phase 2)*