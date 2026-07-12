---
title: "How-To Guides — Index"
audience: [developers, operators, users]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How-To Guides

Task-oriented procedures for achieving specific goals with hKask. Each guide answers "how do I achieve X?" with direct, imperative instructions.

## Installation & Configuration

| Guide | Goal |
|-------|------|
| [Install and Run](install-and-run.md) | Compile from source, install the `kask` binary, configure environment variables |
| [Configure Feature Gates](configure-feature-gates.md) | Enable/disable `matrix`, `communication`, `tui`, `api`, `hedera` features |
| [Configure Database Backend](configure-database-backend.md) | Switch between SQLite/SQLCipher and PostgreSQL |

## Agent Operations

| Guide | Goal |
|-------|------|
| [Create an Agent Pod](create-agent-pod.md) | Define and deploy an agent pod with storage, CNS, and tool bindings |
| [Invoke a Skill](invoke-a-skill.md) | Install, activate, and invoke a skill from CLI or API |
| [Use the REPL](use-repl.md) | Interactive agent session with slash-command dispatch |
| [Use the TUI](use-tui.md) | Terminal UI workspace with multi-window agent interface |

## CNS & Observability

| Guide | Goal |
|-------|------|
| [Read CNS Alerts](read-cns-alerts.md) | Interpret `cns.*` spans, variety counters, and algedonic alerts |
| [Run QA Pipeline](../user-guides/QA_GUIDE.md) | QA fuzz triage, mutation analysis, autonomous scripts |

## Security & Sovereignty

| Guide | Goal |
|-------|------|
| [Audit Sovereignty](audit-sovereignty.md) | Inspect OCAP delegation tokens, verify consent records, audit pod boundaries |
| [Configure Content Guard](configure-guard.md) | Set up content safety guard with classification policy |

## Skill Development

| Guide | Goal |
|-------|------|
| [Design a Skill](design-a-skill.md) | Create a PDCA skill with FlowDef manifest, convergence threshold, and gas budget |
| [Compose Skills](compose-skills.md) | Bundle multiple skills with cascade ordering |

## MCP Development

| Guide | Goal |
|-------|------|
| [Bootstrap an MCP Server](bootstrap-mcp-server.md) | Create a new MCP server using `mcp_server!` macro and `impl_tool_context!` |

## Deployment

| Guide | Goal |
|-------|------|
| [Setup Matrix Transport](setup-matrix-transport.md) | Configure Matrix homeserver integration for A2A communication |
| [Deploy on Kubernetes](deploy-k8s.md) | Deploy hKask on Kubernetes with Conduit sidecar |
| [Backup and Restore](backup-and-restore.md) | Backup SQLCipher database, keystore, and agent state |

## Kata & Coaching

| Guide | Goal |
|-------|------|
| [Run a Kata Cycle](run-kata-cycle.md) | Execute a Toyota Kata improvement cycle |
| [Use Kanban Boards](use-kanban.md) | Create boards, tasks, and WIP limits for agent coordination |

## Advanced

| Guide | Goal |
|-------|------|
| [Train a LoRA Adapter](train-lora-adapter.md) | Fine-tune a LoRA adapter for a replicant persona |
| [Train Qwen3.6 on RunPod](train-qwen36-unsloth-runpod.md) | Reasoning distillation on RunPod with Unsloth |
| [Train Rust Adapters on RunPod](train-rust-adapters-runpod.md) | Rust coding + analysis adapters on RunPod with Unsloth |
