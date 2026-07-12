---
title: "How to Install and Run hKask — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Install and Run hKask

**Goal:** Compile hKask from source, verify the installation, initialize a profile, and configure the environment.

hKask is a Rust workspace of 45 crates, 15 MCP servers, and ~864 tests. It compiles to a single `kask` binary.

---

## 1. Prerequisites

Install the Rust toolchain via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

You need the **stable** toolchain for regular builds and **nightly** for dependency hygiene checks:

```bash
rustup toolchain install stable
rustup toolchain install nightly
```

Additional system dependencies:

| Package | Required for |
|---------|-------------|
| `git` | Clone the repository |
| `build-essential` (Linux) / Xcode CLI tools (macOS) | C linker for native dependencies (SQLCipher) |
| `pkg-config`, `libssl-dev` | OpenSSL bindings |
| `libsqlcipher-dev` | Encrypted SQLite storage (bundled by default, but system library is preferred when available) |

On Debian/Ubuntu:

```bash
sudo apt install git build-essential pkg-config libssl-dev libsqlcipher-dev
```

---

## 2. Clone and Build

```bash
git clone https://github.com/mdz-axo/hKask.git
cd hKask
```

Build the release binary:

```bash
cargo build --release
```

Expected output: a single binary at `target/release/kask`. The full workspace build compiles 45 crates and 15 MCP servers.

For a faster development build (no optimizations):

```bash
cargo build
```

---

## 3. Verify Installation

Confirm the binary works and reports the correct version:

```bash
./target/release/kask --version
# Expected: kask 0.31.0
```

Run the health check (standalone CNS runtime; no agent service needed):

```bash
./target/release/kask cns health
```

Expected output:

```
CNS Health Status
=================

Runtime Status:
  • Healthy: true
  • Overall variety deficit: 0
  • Critical alerts: 0
  • Warning alerts: 0

Variety Counter Summary:
  • No variety data recorded

Active Algedonic Alerts:
  • No active alerts

Energy Budget Status:
  • Model: Energy tracking (subsumes rate limiting)
  • Status: OPERATIONAL
```

---

## 4. Initialize User Profile

Run the interactive server initialization:

```bash
./target/release/kask init
```

This walks you through:

1. **Master passphrase** (minimum 8 characters) — stored in the OS keychain
2. **Data directory** — defaults to `/var/lib/hkask`, creates the directory
3. **Domain name** — defaults to `localhost` (for TLS and OAuth redirects)
4. **OAuth: GitHub** — Client ID and Client Secret (stored in OS keychain)
5. **Config file** — written to `~/.config/hkask/config.json`
6. **Systemd unit** — generated at `~/.config/hkask/hkask.service` for auto-start on boot

After initialization:

```bash
# Set your domain (required if not using localhost):
export HKASK_DOMAIN=your-domain.com

# Start the server:
kask serve
```

---

## 5. Environment Variables

hKask reads configuration from environment variables. The canonical index is at `docs/user-guides/ENVIRONMENT.md`. Here are the essential ones:

### Inference Provider API Keys

All inference keys follow the two-letter provider prefix + `_API_KEY`:

```bash
export DI_API_KEY="your-deepinfra-key"       # DeepInfra
export TG_API_KEY="your-together-key"         # Together AI
export FA_API_KEY="your-fal-key"              # Fal.ai
export OR_API_KEY="your-openrouter-key"       # OpenRouter
export KC_API_KEY="your-kilocode-key"         # KiloCode
```

### Search and External APIs

```bash
export HKASK_BRAVE_API_KEY="your-brave-key"       # Brave Search
export HKASK_TAVILY_API_KEY="your-tavily-key"     # Tavily
export HKASK_FIRECRAWL_API_KEY="your-fc-key"      # Firecrawl
export HKASK_EXA_API_KEY="your-exa-key"           # Exa
export HKASK_SERPAPI_API_KEY="your-serpapi-key"   # SerpApi
```

### Core Configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `HKASK_DEFAULT_MODEL` | Default inference model | Provider-dependent |
| `HKASK_DEFAULT_PROVIDER` | Default inference provider | First configured |
| `HKASK_DB_PATH` | SQLite database path | `~/.local/share/hkask/hkask.db` |
| `HKASK_DB_PASSPHRASE` | SQLCipher passphrase | From OS keychain |
| `HKASK_DOMAIN` | Server domain for TLS | `localhost` |
| `HKASK_PROJECT_ROOT` | Project root for skill discovery | Current directory |
| `HKASK_REPLICANT_NAME` | Replicant name for skill publishing | `git user.name` or `"local"` |
| `HKASK_REPLICANT_PERSONA` | Persona-based WebID resolution | Not set |
| `HKASK_WEBID` | User's WebID | Generated from persona |
| `HKASK_MASTER_KEY` | Master encryption key | From OS keychain |
| `HKASK_TUI` | Force TUI mode (`=1`) | Off |
| `HKASK_FUSION_DISABLED` | Disable fusion mode | Enabled |

### Guard Configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `HKASK_GUARD_TOKEN_LIMIT` | Maximum input token budget | `32000` |

---

## 6. Common Build Issues and Fixes

### `error: linker 'cc' not found`

Install the C toolchain:

```bash
# Debian/Ubuntu
sudo apt install build-essential

# macOS
xcode-select --install
```

### `error: failed to run custom build command for 'libsqlite3-sys'`

SQLCipher needs `libsqlcipher-dev` or uses the bundled version. To force the bundled build:

```bash
# The workspace Cargo.toml already specifies bundled-sqlcipher for rusqlite.
# If you see linker errors for system sqlcipher, ensure the feature is active:
cargo build --release --features rusqlite/bundled-sqlcipher
```

### Build too slow

Build only the CLI binary (not all MCP servers):

```bash
cargo build --release -p hkask-cli
```

Use `mold` linker for faster linking:

```bash
# Install mold first, then:
mold -run cargo build --release
```

### `error: unused crate dependency` on nightly

This is intentional — CI enforces dependency hygiene. Fix by removing unused deps or adding `#[allow(unused_crate_dependencies)]` if intentional. Run:

```bash
RUSTFLAGS="-D unused_crate_dependencies" cargo +nightly check --workspace
```

### Disk space during build

The full workspace builds ~193K LOC across 45 crates. Ensure at least 10 GB free disk space in `target/`. Clean up with:

```bash
cargo clean
```

---

## Next Steps

- Start chatting: `kask chat`
- Launch the TUI: `kask chat --tui` (or `HKASK_TUI=1 kask chat`)
- Read CNS alerts: [Read CNS Alerts](read-cns-alerts.md)
- Start the server: `kask serve`
- Verify the build: `kask cns health`
