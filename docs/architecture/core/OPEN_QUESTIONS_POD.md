---
title: "Pod Architecture — Open Questions"
audience: [architects]
last_updated: 2026-06-18
version: "0.29.0"
status: "Open — design space exploration"
domain: "Agent Pod Lifecycle"
---

# Pod Architecture — Open Questions

Questions raised by the Solid Pod isomorphism that require future design work. These are not blockers for Phase 1 (coexistence) but must be resolved before the migration is complete.

---

## ζ.1 — Pod-to-Pod Communication (Cross-Pod A2A)

**Current state (v0.29.0):** `PodManager` has been deleted. Each pod now has its own `PodDeployment` with per-pod storage, CNS, and MCP bindings. The `ActivePods` registry tracks active deployments. `PodRegistry` provides filesystem-based discovery.

**Options:**

| Transport | Pros | Cons |
|-----------|------|------|
| **Matrix (Conduit)** | Already in deployment model. Built-in federation. OCAP tokens as Matrix device IDs? | Adds Conduit as hard dependency. Matrix protocol overhead. |
| **gRPC** | High performance. Bidirectional streaming. Strong typing via protobuf. | New infrastructure. No federation story. |
| **mpsc channels over TCP** | Simple. Matches existing CNS channel pattern. | No authentication, discovery, or routing. |
| **WebSocket + JSON** | Browser-compatible. Matches existing xterm.js WebSocket pattern. | No built-in federation. |

**Key constraint:** Cross-pod A2A must preserve OCAP gating. The capability token that works within a pod must extend across pod boundaries. This implies a capability-bearing protocol — not just any message transport.

**Question:** What is the minimal viable cross-pod A2A protocol that preserves OCAP gating?

---

## ζ.2 — Pod Portability Across Servers

**Current state:** The backup model exports a SQLCipher file ("Backup as portable archive. Encrypted SQLCipher file. Export from one server, upload to another"). The `derive_ocap_secret(webid)` function is deterministic: same master key + same WebID → same pod key material, independent of server.

**Open sub-questions:**

1. **CNS state transferability:** Variety counters are temporal (60-second sliding windows). Do they transfer to a new server? Transferring them would produce stale state; resetting them would lose behavioral history. *Proposed answer:* Reset variety counters on migration. The CNS adapts quickly (60-second window). Curator's historical records remain in the pod's database.

2. **MCP server API keys:** API keys are scoped to the user, not the server. When a pod moves from server A to server B, the inference API key moves with the pod (it's the user's key). *Proposed answer:* Yes, keys travel with the pod. The pod's persona YAML or keystore entry carries the API key reference.

3. **What about DNS?** A Solid Pod has a URL (`https://user.example/pod/`). An AgentPod has a database file. What is the AgentPod's addressable identity? *Proposed answer:* The pod's WebID is the addressable identity. The server is just the host — any server can host any pod, as long as the WebID is consistent.

---

## ζ.3 — Pod Lifecycle Across Containers

**Current state (v0.29.0):** Pod lifecycle is managed by `ActivePods` (runtime registry) and `PodFactory` (constructor). Each `PodDeployment` holds its own `AgentPod` with lifecycle state.

**Target:** Pod IS a Docker/Podman container. Starting/stopping the pod IS starting/stopping the container.

**Questions:**

| Current (ActivePods) | Containerized (Proposed) | Open Question |
|---------------------|--------------------------|---------------|
| `kask pod activate <id>` | `docker start <pod_id>` | Should `kask pod activate` become a wrapper around Docker? Or should the container model be an alternative deployment mode? |
| `kask pod deactivate <id>` | `docker stop <pod_id>` | Same question. Does this simplify or complicate the lifecycle state machine? |
| `PodLifecycleState::Activated` | Container running | The state machine already has Populated → Registered → Activated → Deactivated. Does "Activated" mean "container running"? Or is "Activated" a logical state independent of the container's process state? |
| `PodLifecycleState::Deactivated` | Container stopped | Is Deactivated terminal? Containers can be restarted. Should the pod lifecycle allow reactivation from Deactivated? |

**Proposed answer:** `PodLifecycleState` remains a logical state. The container is an implementation detail of the "Activated" state. `kask pod activate` could start a container; `kask pod deactivate` could stop it. But `PodLifecycleState::Deactivated` should remain terminal — a deactivated pod has its capabilities revoked. Restarting the container would require re-registration (Populated → Registered → Activated).

---

## ζ.4 — Curator Aggregation Model

**Current state:** `CnsRuntime` is server-global (one per process). Per-pod CNS means N CNS runtimes, one per pod. The Curator (VSM S4 — Intelligence) must aggregate across all pods.

**Two models:**

| Model | Description | Pros | Cons |
|-------|-------------|------|------|
| **Poll (Curator pulls)** | Curator queries each pod's CNS runtime for variety counters, alerts, health status | Simple. No push infrastructure. Curator controls sampling rate. | Polling overhead scales with N pods. Delayed detection of critical alerts. |
| **Push (Pods emit)** | Each pod's CNS runtime pushes spans to a shared Curator channel (existing `mpsc` or `tokio::broadcast`) | Real-time. Already matches existing CNS→Curator `mpsc` pattern. | Shared channel means per-pod isolation is weakened at the aggregation point. |

**Key constraint:** The algedonic pathway is unidirectional (CNS → Curator). Per-pod CNS must preserve this — each pod emits its own algedonic alerts; the Curator reads them all. The per-pod boundary means a pod's CNS should not be able to observe another pod's CNS state directly.

**Question:** Should the Curator poll each pod's CNS (stronger isolation, weaker real-time) or should pods push to a shared channel (weaker isolation, stronger real-time)?

---

## ζ.5 — Essentialist Deletion Test on PodFactory

**G1 (Exist):** If pods are created via `docker build` from a template and `kask pod export-container` produces a Dockerfile — is `PodFactory` a Rust type at all, or is it a CLI command that shells out to Docker?

**Two futures:**

| Future | PodFactory Role | Verdict |
|--------|----------------|---------|
| **In-process pods** | `PodFactory::deploy()` constructs `PodDeployment` in-process. Pod runs as a thread/task within the server. | **KEEP.** PodFactory is the canonical constructor. |
| **Containerized pods** | `kask pod export-container` generates Dockerfile + build context. Pod runs as a Docker container. `PodFactory` becomes a CLI command generator, not a Rust type constructor. | **DELETE.** Replace with `kask pod export-container`. |

**Current Phase 1 design** assumes in-process pods (PodFactory constructs PodDeployment in-process). If the container model becomes the primary deployment mode, PodFactory's `deploy()` method may become unnecessary — replaced by `kask pod export-container` → `docker build` → `docker run`.

**Question:** Is PodFactory a necessary intermediate step (in-process pods first, containers later) or should we skip directly to container-native deployment?
