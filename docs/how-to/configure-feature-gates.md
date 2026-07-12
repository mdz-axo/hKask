---
title: "How to Configure Feature Gates — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Configure Feature Gates

This guide covers which compile-time feature flags exist in hKask, what functionality each gates, and how to enable or disable them for your build.

## Feature Flags Overview

hKask uses two crates with feature gates controlling subsystem availability:

| Crate | Feature | Default | Gates |
|-------|---------|---------|-------|
| `hkask-cli` | `tui` | **on** | Terminal UI subsystem |
| `hkask-cli` | `api` | **on** | REST API and web server |
| `hkask-communication` | `matrix` | **on** | Matrix protocol transport |

The `hkask-cli` crate also declares a `communication` feature in `Cargo.toml` at the workspace level, used as the matrix feature gate that selects between `hkask-communication/matrix` and `hedera` transports.

The `hedera` transport pathway is **declared but not yet implemented** — the feature gate placeholder exists for future Hashgraph integration.

## What Each Feature Gates

### `tui` (Default: enabled)

Pulls in `ratatui`, `crossterm`, and `hkask-repl/tui`. Disabling this strips the terminal UI entirely — the `kask` binary will be CLI-only. The TUI provides the multi-window workspace with chat, CNS monitor, kanban, backup, registry, matrix, pods, and other subsystem views.

**Disable with:**
```bash
cargo build --no-default-features -p hkask-cli
```

Or in `Cargo.toml`:
```toml
[dependencies]
hkask-cli = { path = "crates/hkask-cli", default-features = false, features = ["api"] }
```

### `api` (Default: enabled)

Pulls in `hkask-api` and `axum` for the REST API server. Disabling this strips the HTTP API surface — the binary will only have CLI and (if enabled) TUI interfaces. This is the `kask serve --api` backend.

**Disable with:**
```bash
cargo build --no-default-features -p hkask-cli --features tui
```

### `matrix` (Default: enabled in `hkask-communication`)

Pulls in `matrix-sdk` for Matrix protocol transport. This enables agent-to-agent (A2A) communication via Matrix rooms, the 7R7 listener, and the agent registry. Disabling this feature removes the entire communication subsystem.

**Disable with:**
```bash
cargo build --no-default-features -p hkask-communication
```

## Verifying Your Feature Configuration

After building, you can check which features were active:

```bash
# Show the resolved features for the kask binary
cargo tree -p hkask-cli -e features | head -20
```

For a minimal build (CLI only, no TUI, no API):
```bash
cargo build -p hkask-cli --no-default-features
```

## Feature Interaction Notes

- The `tui` feature depends on `hkask-repl/tui` — you cannot have the TUI without the REPL.
- The `api` feature depends on `axum` — it pulls in the HTTP server runtime.
- `hkask-communication`'s `matrix` feature is independent of `hkask-cli`'s TUI/API features.
- Stripping `tui` but keeping `api` is valid — you get a headless server with REST API.
- Stripping both `tui` and `api` gives you a pure CLI tool for scripting and automation.
