---
title: "hKask Dependency Policy"
audience: [developers, maintainers, agents]
last_updated: 2026-05-22
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [lifecycle]
---


# hKask Dependency Policy

**Purpose:** Ensure hKask runs on the most recent stable versions of Rust and dependencies while maintaining build stability.

**Related:** [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md)

---

## 1. Rust Edition Policy

### 1.1 Current Edition

**Rust 2024** (toolchain 1.91+)

### 1.2 Edition Update Cadence

| Event | Action |
|-------|--------|
| **New Rust edition released** | Update within 3 months of stable release |
| **New toolchain version** | Update `rust-toolchain.toml` within 1 month |
| **MSRV (Minimum Supported Rust Version)** | Track latest stable minus 2 versions [^semver] |

### 1.3 Rationale

- **Security:** Newer editions include security hardening
- **Performance:** Compiler optimizations improve over time
- **Developer Experience:** New syntax features reduce boilerplate
- **Ecosystem Alignment:** Most crates target recent editions

---

## 2. Dependency Update Policy

### 2.1 Update Cadence

| Dependency Type | Update Frequency | Method |
|-----------------|------------------|--------|
| **Patch versions** (`1.0.x` → `1.0.y`) | Weekly | Automated (`cargo update`) |
| **Minor versions** (`1.x` → `1.y`) | Monthly | Manual review + test |
| **Major versions** (`1.x` → `2.x`) | Per-release | Breaking change audit |

### 2.2 Automated Updates

**Tool:** `cargo-outdated` + `dependabot` (or `renovate`) [^dependabot]

```bash
# Check for outdated dependencies
cargo outdated

# Update to latest compatible versions
cargo update

# Update to latest versions (may change Cargo.lock significantly)
cargo update --aggressive
```

### 2.3 Manual Review Required

Major version updates require:

1. **Changelog review** — Identify breaking changes
2. **Migration guide** — Follow crate's migration instructions
3. **Test suite** — All tests must pass
4. **Clippy** — No new warnings introduced
5. **Performance** — No significant regressions

---

## 3. Dependency Selection Criteria

### 3.1 Required Properties

| Property | Requirement | Verification |
|----------|-------------|--------------|
| **Active maintenance** | Commits within 6 months | GitHub activity |
| **Documentation** | API docs + examples | `cargo doc` builds |
| **Test coverage** | Tests present + passing | CI status |
| **Security** | No known CVEs | `cargo audit` [^slsa] |
| **License** | MIT, Apache-2.0, BSD | `cargo-license` |

### 3.2 Preferred Properties

| Property | Preference | Rationale |
|----------|------------|-----------|
| **Pure Rust** | Prefer over FFI | Easier cross-compilation |
| **Async support** | Tokio-native | Ecosystem alignment |
| **`no_std` capable** | Bonus | Future embedded use |
| **Workspace member** | Monorepo preferred | Easier debugging |

### 3.3 Prohibited Properties

| Property | Prohibition | Rationale |
|----------|-------------|-----------|
| **Abandoned** | No commits >12 months | Security risk |
| **Known CVEs** | Unpatched vulnerabilities | Security risk |
| **Copyleft licenses** | GPL, AGPL, etc. | Incompatible with MIT |
| **Excessive dependencies** | >50 transitive deps | Build bloat |

---

## 4. Version Pinning Strategy

### 4.1 Cargo.toml Version Specifiers

| Specifier | Meaning | Use Case |
|-----------|---------|----------|
| `^1.2.3` | `>=1.2.3, <2.0.0` | Default for stable crates [^semver] |
| `~1.2.3` | `>=1.2.3, <1.3.0` | Conservative updates |
| `=1.2.3` | Exactly this version | Critical dependencies |
| `>=1.2.3` | Any compatible version | Flexible crates |

### 4.2 Cargo.lock Policy

**Committed to Git:** Yes

**Rationale:**
- Reproducible builds across machines
- CI/CD consistency
- Debugging specific version issues

### 4.3 When to Pin Exactly

Pin with `=` for:

- **Critical security crates** (cryptographic libraries)
- **Known-breaking patterns** in newer versions
- **Workspace-internal crates** (inter-crate dependencies)

---

## 5. Security Scanning

### 5.1 Automated Scanning

**Tool:** `cargo-audit`

**Frequency:** Every CI run

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit
```

### 5.2 Response to Vulnerabilities

| Severity | Response Time | Action |
|----------|---------------|--------|
| **Critical** | <24 hours | Immediate patch or workaround |
| **High** | <1 week | Schedule patch in next sprint |
| **Medium** | <1 month | Include in next release |
| **Low** | <3 months | Monitor, patch when convenient |

### 5.3 Vulnerability Response Workflow

```mermaid
flowchart TD
    A[cargo-audit detects CVE] --> B{Severity?}
    B -->|Critical| C[Immediate patch]
    B -->|High| D[Schedule within week]
    B -->|Medium| E[Include in next release]
    B -->|Low| F[Monitor]
    
    C --> G[Update dependency]
    D --> G
    E --> G
    F --> G
    
    G --> H[Run test suite]
    H --> I{Tests pass?}
    I -->|Yes| J[Commit + deploy]
    I -->|No| K[Workaround or fork]
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DEP-001
verified_date: 2026-05-24
verified_against: Cargo.toml workspace.dependencies
status: VERIFIED
-->

---

## 6. Build Reproducibility [^reproducible-builds]

### 6.1 Toolchain Pinning

**File:** `rust-toolchain.toml`

**Contents:**
```toml
[toolchain]
channel = "1.91"
components = ["rustfmt", "clippy", "rust-src"]
targets = []
profile = "default"
```

### 6.2 Docker/Container Builds

**Base Image:** `rust:1.91-slim` (or equivalent)

**Rationale:** Matches toolchain version exactly

### 6.3 CI/CD Consistency

All CI jobs must:

1. Use `rust-toolchain.toml` version
2. Build from clean `Cargo.lock`
3. Run `cargo audit` before tests

---

## 7. Dependency Audit Checklist

Before adding a new dependency:

- [ ] **Active maintenance** — Commits within 6 months
- [ ] **Documentation** — API docs build successfully
- [ ] **Tests** — Test suite present and passing
- [ ] **Security** — `cargo audit` shows no CVEs
- [ ] **License** — MIT, Apache-2.0, or BSD
- [ ] **Alternatives** — Evaluated 2+ alternatives
- [ ] **Necessity** — Cannot be implemented with existing deps
- [ ] **Size** — Transitive deps <50

---

## 8. References

- **Rust Edition Guide:** <https://doc.rust-lang.org/edition-guide/>
- **Cargo Versioning:** <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- **cargo-audit:** <https://github.com/rustsec/rustsec>
- **cargo-outdated:** <https://github.com/kbknapp/cargo-outdated>

[^semver]: Preston-Werner, T. (2013). *Semantic Versioning 2.0.0*. https://semver.org/
[^dependabot]: GitHub. (2024). *About Dependabot version updates*. https://docs.github.com/en/code-security/dependabot
[^slsa]: OpenSSF. (2024). *SLSA: Supply-chain Levels for Software Artifacts*. https://slsa.dev/
[^reproducible-builds]: Reproducible Builds Project. (2024). *Reproducible Builds*. https://reproducible-builds.org/

---

*This policy ensures hKask remains current with the Rust ecosystem while maintaining build stability and security.*

**Review Cadence:** Quarterly (aligned with Rust release cycle)
