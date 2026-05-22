# CI/CD Test Compilation Gate

## Purpose
Ensure all test files in `hkask-testing` compile before merge to prevent broken test debt accumulation.

## Gate Implementation

### Pre-Merge Check
```bash
# Must pass before merge
cargo test -p hkask-testing --all-targets --no-run
```

### CI Step (GitHub Actions)
```yaml
- name: Test Compilation Gate
  run: cargo test -p hkask-testing --all-targets --no-run
```

## Test Categories

| Category | Location | Budget Impact | Gate |
|----------|----------|---------------|------|
| **Unit Tests** | `hkask-testing/unit-tests/` | Excluded | ✅ Required |
| **Integration Tests** | `hkask-testing/integration-tests/` | Excluded | ✅ Required |
| **Inline Tests** | `crates/*/src/**/*.rs` | **Counted** | ❌ Prohibited |

## Rules

1. **No inline `#[cfg(test)]` modules** in production crates
2. **All test code** must be in `hkask-testing` crate
3. **Test files** must compile independently
4. **Broken tests** must be fixed or removed within 24 hours

## Enforcement

- Pre-commit hook: `cargo test -p hkask-testing --all-targets --no-run`
- CI gate: Blocks merge on test compilation failure
- Code review: Reject PRs with inline test modules

## Rationale

Test code in production crates:
- Counts toward 30k LOC budget
- Creates coupling between test/production boundaries
- Makes test maintenance harder
- Blurs hexagonal architecture boundaries

Test code in `hkask-testing`:
- Excluded from LOC budget
- Clear separation of concerns
- Easier to maintain and extend
- Follows hexagonal architecture

---
*Document generated: 2026-05-22*
*Part of hKask Test Architecture Integrity (Phase 1)*