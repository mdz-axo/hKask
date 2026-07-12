---
title: "hkask-cli — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-cli` is the command-line interface for hKask. The binary entry point is `main.rs` in this crate. The CLI is built on `clap` with a `Cli` struct carrying global flags and a `Commands` enum dispatching to subcommand handlers in the `commands` module. The binary name is `kask`.

## Public Modules

| Module | Purpose |
|---|---|
| `archival` | Tarball/compression archiver for backup and export |
| `cli` | Top-level `Cli` parser, `Commands` enum, action types, and markdown generator |
| `cloud` | Cloud gateway client support |
| `commands` | All subcommand handler modules (`chat`, `agent`, `pod`, `mcp`, `cns`, `sovereignty`, `goal`, `docs`, `git_cmd`, `backup_cmd`, `curator`, `federation`, `token`, `user`, `keystore`, `bundle`, `skill`, `style`, `kanban`, `adapter`, `kata`, `models`, `doctor`, `settings`, `consolidation`, `loops`, `daemon`, `test`, `web_search`, `serve`, `init`, `export_cmd`, `wallet`, `registry`, `repair`, `qa`, `matrix`) |
| `experience` | User experience utilities |
| `onboarding` | Interactive replicant onboarding flow |
| `onboarding_session` | Onboarding session state management |
| `repl_host` | REPL host infrastructure |
| `transcript_viewer` | TUI transcript viewer (feature-gated: `tui`) |

## Global Flags

Defined on the `Cli` struct:

| Flag | Short | Type | Purpose |
|---|---|---|---|
| `--verbose` | `-v` | `bool` | Enable verbose output |
| `--json-logs` | — | `bool` | Output logs as JSON for OpenTelemetry ingestion |
| `--registry` | `-r` | `Option<PathBuf>` | Registry database path (default: in-memory) |

## Subcommands

The `Commands` enum has 36 variants. Each maps to a handler in `commands/`:

| Subcommand | Purpose | Feature Gate |
|---|---|---|
| `Chat` | Curator chat interface (interactive by default). Flags: `--template`, `--input`, `--agent`, `--model`, `--tui` | — |
| `Template` | Template management (list, inspect, delete, register) | — |
| `Bot` | Bot capability management | — |
| `Pod` | Agent pod lifecycle (create, activate, deactivate, status, list) | — |
| `Mcp` | MCP server and tool management | — |
| `Cns` | CNS monitoring and health | — |
| `Sovereignty` | User sovereignty management (Magna Carta enforcement) | — |
| `Goal` | Goal coordination substrate (OCAP-gated, CNS-observed) | — |
| `Git` | Git archival and CAS actions | — |
| `Backup` | Snapshot, restore, list, prune, verify, config | — |
| `Docs` | Documentation generation | — |
| `Agent` | A2A agent registration and management | — |
| `Curator` | Curator governance and metacognition | — |
| `Federation` | Federation lifecycle — cross-server curator sync | — |
| `Token` | Token issuance and management | — |
| `Replicant` | Replicant identity management | — |
| `Keystore` | Keystore management (OS keychain) | — |
| `Bundle` | Skill bundle management (compose, apply, evolve) | — |
| `Skill` | Skill management (list, status, publish) | — |
| `Style` | Style operations — compose prose or embed corpora | — |
| `Kata` | Toyota Kata — list and inspect kata manifests | — |
| `Kanban` | Kanban board and task coordination | — |
| `Adapter` | Trained adapter lifecycle — deploy, infer, teardown | — |
| `Models` | List available LLM models | — |
| `Doctor` | Validate all configured providers and API keys | — |
| `Onboard` | Add a new replicant to an existing hKask installation | — |
| `Settings` | Manage REPL/CLI inference settings | — |
| `Consolidate` | Trigger episodic→semantic consolidation. Flags: `--agent`, `--limit` (default 100), `--confidence-floor`, `--max-semantic-triples`, `--passphrase` | — |
| `Loops` | Run the 6-loop regulation system | — |
| `Daemon` | Start daemon (Unix socket for MCP server auth + CNS monitoring) | — |
| `Test` | Run REQ-tagged contract tests. Flags: `--crate-name`, `--format` (default "text"), `--watch` | — |
| `WebSearch` | Web search. Args: `query`, `--max-results` (default 5) | — |
| `Serve` | Start HTTP API server. Flags: `--port` (default 3000), `--host` (default 127.0.0.1) | `api` |
| `Init` | Initialize hKask server configuration (interactive) | — |
| `Export` | Sovereignty export — create and migrate encrypted h_mem archives | — |
| `Wallet` | Wallet operations — balance, deposits, withdrawals, API keys | — |
| `List` | List artifacts in a registry (e.g., "styles", "bots") | — |
| `Rm` | Remove an artifact from a registry. Env: `HKASK_DB_PATH`, `HKASK_DB_PASSPHRASE` | — |
| `Transcript` | View a transcript bundle (TUI). Feature-gated: `tui` | `tui` |
| `Matrix` | Matrix messaging — sidecar deployment, agent registration, health checks | — |
| `Repair` | Repair encrypted databases. Flags: `--dry-run`, `--force` | — |
| `Qa` | QA operations — run autonomous test scripts | — |

## Environment Variables

| Variable | Purpose |
|---|---|
| `HKASK_DB_PATH` | Database path (used by `Rm` and stores) |
| `HKASK_DB_PASSPHRASE` | Database passphrase (used by `Rm`) |
| `HKASK_FUSION_JUDGE_MODEL` | Opt-in fusion model configuration |
| `HKASK_FUSION_PANEL_MODELS` | Opt-in fusion panel model list |
| `HKASK_MCP_HOST` | Replicant MCP host name (used by ACP) |
| `HKASK_GUARD_TOKEN_LIMIT` | Token limit override for content guard |

## Feature Flags

| Feature | Default | Purpose |
|---|---|---|
| `tui` | Yes | Enables `TranscripterViewer` (TUI) and `Transcript` subcommand. Pulls in `ratatui`, `crossterm`, `hkask-repl/tui` |
| `api` | Yes | Enables `Serve` subcommand and HTTP API server. Pulls in `hkask-api`, `axum` |

## Macros

### `block_on!`

Blocks on a tokio `Runtime`, exiting the process with code 1 on failure.

```text
block_on!($rt, $future, $msg)
```

Parameters: `$rt` — tokio Runtime reference; `$future` — future to block on; `$msg` — error message prefix.
