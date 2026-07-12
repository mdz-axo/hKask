---
title: "How to Create an Agent Pod — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Create an Agent Pod

This guide covers writing an agent persona definition, deploying a pod, verifying its health, and managing pod lifecycle transitions.

## What Is a Pod?

A pod is a sovereign runtime container for a single agent (replicant or bot). Each pod has its own isolated storage (episodic + semantic memory), CNS runtime, MCP tool bindings, and capability token. Pods are registered with the A2A runtime for Matrix communication and can be activated for MCP access.

## Step 1: Write an Agent Persona YAML

Define your agent in a YAML file. The persona is parsed by `AgentPersona::from_yaml()` in `crates/hkask-agents/src/pod/types.rs`.

```yaml
agent:
  name: my-assistant
  type: replicant      # "bot" or "replicant"
  version: "0.1.0"

charter:
  description: "A general-purpose assistant for code review"
  editor: "alice"

capabilities:
  - tool:execute
  - skill:rust-review

rights:
  - read: registry/skills
  - write: episodic/my-assistant

responsibilities:
  - review_pull_requests
  - generate_reports

visibility:
  default: shared
  episodic_override: private

communication_posture:
  convergence_bias: 0.7
  invariant_traits:
    - precise
    - concise
```

**Validation rules** (enforced by `AgentPersona::validate_fields()`):
- `name`: 1–64 chars, alphanumeric, hyphens, and underscores only
- `agent_type`: must be `bot` or `replicant`
- `version`: 1–32 chars, non-empty
- `description`: max 1000 chars
- `editor`: 1–256 chars, non-empty
- `capabilities`: max 20, each ≤128 chars

## Step 2: Deploy the Pod

Use the `PodFactory` from `crates/hkask-agents/src/pod/deployment.rs`:

```rust
let factory = PodFactory::new(template_loader, consent, data_dir, db_provider);
let pod = factory.deploy(persona, pod_kind).await?;
```

Three `PodKind` variants determine isolation:
- **Curator** — singleton pod, owns the SemanticIndex, CNS aggregation
- **Team** — shared workspace for multiple bots
- **Replicant** (default) — per-user sovereign pod

Pods are persisted as files in `~/.config/hkask/pods/` with filename convention `<kind>/<name>.pod.yaml`.

## Step 3: Register and Activate

The pod lifecycle is linear: **Populated → Registered → Activated → Deactivated**.

```rust
// Register with A2A runtime
pod.register(&a2a_runtime).await?;  // Populated → Registered

// Activate for full capability
pod.activate()?;  // Registered → Activated
```

Registration mints a capability token. Activation grants MCP access and enables A2A communication. Agents are initially mutually exclusive between Chat and Server modes — set via `enter_chat_mode()` or `enter_server_mode()`.

## Step 4: Verify Pod Health

Check pod status through `ActivePods`:

```rust
let status = active_pods.get_pod_status(&pod_id).await?;
// PodStatusInfo { pod_id, name, state, webid, agent_type, template, pod_kind, created_at }

// List all active pods
let pods = active_pods.list_pods().await?;

// Check capabilities
let can_exec = active_pods.has_capability(&pod.webid(), "tool:execute").await;
```

## Step 5: Deactivate

```rust
pod.deactivate()?;  // Activated → Deactivated (terminal)
```

Deactivation revokes capabilities. The pod cannot be re-activated — create a new pod from the same persona if needed.

## Pod Isolation Model

Each pod gets:
- **Per-pod storage**: dedicated SQLCipher database, HMem stores, embedding storage
- **Per-pod CNS runtime**: isolated span namespace for observability
- **Per-pod tool bindings**: governed MCP tools with OCAP-gated access
- **WebID**: deterministically derived from the persona definition
