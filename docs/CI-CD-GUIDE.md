# CI/CD and Installation Guide — hKask

## Overview

This document describes the CI/CD pipeline, installation process, and build configuration for hKask (ℏKask — "Planck's Constant of Agent Systems").

---

## Repository Structure

```
hKask/
├── .github/workflows/          # GitHub Actions workflows
│   ├── ci.yml                  # Main CI pipeline (format, lint, build, test)
│   ├── chaos-testing.yml       # Chaos engineering tests
│   ├── release.yml             # Release pipeline
│   └── docs.yml                # Documentation generation
├── scripts/
│   ├── install.sh              # Installation script for Linux
│   ├── build.sh                # Build script
│   └── chaos-injection.sh      # Chaos injection tests
├── .cargo/
│   └── config.toml             # Cargo configuration
├── rust-toolchain.toml         # Rust toolchain specification
├── Cargo.toml                  # Workspace manifest
└── crates/                     # Core crates (11 total)
```

---

## Installation

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/mdz-axolotl/hKask/main/scripts/install.sh | bash
```

### Manual Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/mdz-axolotl/hKask.git
   cd hKask
   ```

2. **Run the install script:**
   ```bash
   chmod +x scripts/install.sh
   ./scripts/install.sh
   ```

3. **Add to PATH:**
   ```bash
   export PATH="$HOME/.local/bin:$PATH"
   ```

### Supported Linux Distributions

The install script automatically detects and supports:

- **Debian/Ubuntu** (apt)
- **Fedora/RHEL** (dnf/yum)
- **Arch Linux** (pacman)
- **openSUSE** (zypper)
- **Alpine** (apk)

### System Requirements

**Minimum:**
- Rust 1.70+ (stable)
- 4GB RAM
- 2GB disk space
- GCC/Clang
- pkg-config
- libssl-dev
- libsqlite3-dev
- libclang-dev
- llvm-dev
- cmake
- protobuf-compiler

**Install dependencies manually (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libsqlite3-dev \
  libclang-dev \
  llvm-dev \
  cmake \
  git \
  curl \
  jq \
  protobuf-compiler \
  libprotobuf-dev
```

---

## Building

### Using the Build Script

```bash
# Debug build
./scripts/build.sh

# Release build
./scripts/build.sh --release

# Build with tests
./scripts/build.sh --all

# Build documentation
./scripts/build.sh --doc

# Clean build
./scripts/build.sh --clean
```

### Manual Build

```bash
# Check (fast)
cargo check --workspace

# Build
cargo build --workspace

# Release build
cargo build --release --workspace

# Build specific crate
cargo build --package hkask-cli

# Run tests
cargo test --workspace --lib

# Run clippy
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt
```

---

## CI/CD Workflows

### 1. CI Pipeline (`.github/workflows/ci.yml`)

**Triggers:** Push to `main`/`develop`, pull requests

**Jobs:**
- **Format Check** — Verifies code formatting with `cargo fmt`
- **Linting** — Runs `cargo clippy` with strict warnings
- **Build** — Compiles all workspace members
- **Unit Tests** — Runs `cargo test --workspace --lib`
- **Integration Tests** — Runs integration test suite
- **Line Budget Check** — Verifies ≤30,000 lines of Rust code
- **Security Audit** — Runs `cargo audit`
- **Dependency Check** — Runs `cargo outdated`
- **Release Build** — Builds production binary (main branch only)

**Features:**
- Parallel job execution where possible
- Caching for cargo registry and build artifacts
- Timeout limits to prevent hung builds
- Artifact upload for binaries and test results

### 2. Chaos Testing (`.github/workflows/chaos-testing.yml`)

**Triggers:** Schedule (daily 2 AM UTC), manual dispatch

**Jobs:**
- **Chaos Unit Tests** — Resilience and failover unit tests
- **Integration Tests** — Mock Okapi integration tests
- **Daily Chaos Tests** — Scheduled chaos injection (self-hosted)
- **Weekly Chaos Suite** — Full chaos test suite (self-hosted, Sundays)

**Test Categories:**
1. **Instance Failures** — Single/cascading instance termination
2. **Network Partitions** — Network isolation and latency injection
3. **Resource Exhaustion** — Memory pressure testing
4. **Circuit Breaker** — Trip and recovery testing
5. **Retry Policies** — Exponential backoff and exhaustion

**Requirements:**
- Self-hosted runners for daily/weekly tests
- Okapi instances (Docker containers)
- Tools: `stress-ng`, `iperf3`, `tc`, `jq`

### 3. Release Pipeline (`.github/workflows/release.yml`)

**Triggers:** Git tag push (`v*`), manual dispatch

**Jobs:**
- **Pre-Release Checks** — Format, lint, test, docs
- **Build Release** — Multi-architecture binaries:
  - `x86_64-unknown-linux-gnu`
  - `x86_64-unknown-linux-musl`
  - `aarch64-unknown-linux-gnu`
- **Create GitHub Release** — Generates release with assets
- **Publish to crates.io** — Optional crate publishing

**Release Assets:**
- Binary tarballs (`.tar.gz`)
- SHA256 checksums
- Changelog (auto-generated from Git)

### 4. Documentation (`.github/workflows/docs.yml`)

**Triggers:** Push to `main`/`develop`, PRs, manual dispatch

**Jobs:**
- **Generate Documentation** — `cargo doc --workspace`
- **Deploy to GitHub Pages** — Auto-deploy from main branch
- **Link Check** — Validates markdown links
- **Documentation Coverage** — Checks API documentation completeness

---

## Configuration Files

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = []
profile = "default"
```

**Purpose:** Ensures consistent Rust version across all developers and CI.

### `.cargo/config.toml`

Key configurations:

```toml
# Build profiles
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
debug = true
split-debuginfo = "unpacked"
panic = "unwind"

# Target-specific optimizations
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]
```

**Purpose:** Optimizes build performance and binary size.

---

## Code Quality Gates

### Pre-commit Checklist

Before committing code:

1. **Format:**
   ```bash
   cargo fmt
   ```

2. **Lint:**
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

3. **Test:**
   ```bash
   cargo test --workspace --lib
   ```

4. **Check:**
   ```bash
   cargo check --workspace
   ```

### CI Checks (Must Pass)

- ✓ Code formatting (`cargo fmt --check`)
- ✓ Linting (`cargo clippy -- -D warnings`)
- ✓ Build (`cargo build --workspace`)
- ✓ Unit tests (`cargo test --workspace --lib`)
- ✓ Line budget (≤30,000 lines)

### Optional Checks

- Security audit (`cargo audit`)
- Dependency updates (`cargo outdated`)
- Documentation generation (`cargo doc`)

---

## Troubleshooting

### Common Build Errors

**Error: `let chains are only allowed in Rust 2024 or later`**

**Fix:** The workspace uses Rust 2024 edition. Let chains work natively:

```rust
// ✅ Rust 2024 (works with let chains)
if let Some(x) = value && x > 0 {
    // ...
}
```

**Error: Missing system dependencies**

**Fix:** Install required packages for your distribution:

```bash
# Debian/Ubuntu
sudo apt-get install build-essential pkg-config libssl-dev libsqlite3-dev

# Fedora/RHEL
sudo dnf install gcc pkg-config openssl-devel sqlite-devel

# Arch
sudo pacman -S base-devel pkg-config openssl sqlite
```

**Error: Out of disk space during build**

**Fix:** Clean build artifacts:
```bash
cargo clean
```

### CI Failures

**Workflow fails on `self-hosted` runner:**

1. Ensure runner is online and has required tools
2. Check runner logs for specific errors
3. Verify Docker is running (for Okapi containers)

**Chaos tests fail:**

1. Check if Okapi instances are healthy
2. Verify network connectivity between containers
3. Review chaos injection logs in artifacts

---

## Architecture Decisions

### Why Rust 2024?

- **Latest Features:** Let chains, if-let chaining, improved macros
- **Performance:** Enhanced compiler optimizations
- **Security:** Latest security hardening
- **Ecosystem:** Full crate support by 2026
- **Compatibility:** All crates and dependencies support 2021
- **Features:** 2021 provides all needed features (async, macros, etc.)

### Why Single Workspace?

- **Consistency:** Shared dependencies and versions
- **Atomic commits:** Changes across crates in single commit
- **Testing:** Integration tests can access all crates

### Why Self-Hosted Runners for Chaos Tests?

- **Resource Requirements:** Chaos tests need multiple Okapi instances
- **Cost:** Long-running tests (3+ hours) are expensive on GitHub-hosted
- **Control:** Full control over system tools (`tc`, `stress-ng`)

### Why Not Windows/macOS?

- **Focus:** Linux-only simplifies CI/CD and testing
- **Target Deployment:** Server/cloud environments are Linux-based
- **Resource Constraints:** Cross-platform testing doubles CI costs

---

## Future Improvements

### Planned

1. **Binary Distribution:**
   - Package for Debian/Ubuntu (`.deb`)
   - Package for Fedora/RHEL (`.rpm`)
   - Homebrew formula for macOS (if needed)

2. **Container Images:**
   - Docker image for hKask runtime
   - Docker Compose for full stack (hKask + Okapi + Prometheus)

3. **Performance Benchmarks:**
   - Automated benchmark suite
   - Performance regression detection
   - Historical performance tracking

4. **Enhanced Security:**
   - SBOM (Software Bill of Materials) generation
   - Automated vulnerability scanning
   - Supply chain security (Sigstore/cosign)

### Considered (Not Implemented)

- ❌ Cross-platform builds (Windows/macOS)
- ❌ Nightly Rust features
- ❌ Custom CI runners (using standard GitHub Actions)
- ❌ Complex deployment pipelines (keep it simple)

---

## Contact and Support

- **Issues:** https://github.com/mdz-axolotl/hKask/issues
- **Discussions:** https://github.com/mdz-axolotl/hKask/discussions
- **Documentation:** https://mdz-axolotl.github.io/hKask/

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*As simple as possible, but no simpler.*
