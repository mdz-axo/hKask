---
title: "Open Questions — Multi-Pod Architecture (ζ Group) — Design Closure"
audience: [architects]
last_updated: 2026-06-19
version: "0.30.0"
status: "Design Closed"
domain: "Agent Pod Lifecycle"
principles: [P1, P4, P5, P6, P9, P11]
---

# Open Questions — Design Closure (v0.30.0)

Answers to the ζ-group design questions posed during the multi-pod architecture design session. Questions that have concrete implementation answers are marked **Resolved**. Questions requiring future work are marked **Deferred** with a trigger condition.

---

## ζ.1 — Cross-Pod A2A Protocol

**Question:** What is the minimal viable cross-pod A2A protocol that preserves OCAP gating?

**Status:** Deferred (trigger: cross-server deployment use case)

**Analysis:**
- Current A2A (`A2ARuntime`) assumes same-process agents. All agents register with a shared `ActivePods` registry and communicate via in-process channels.
- Cross-pod A2A requires a network boundary. The deployment model already specifies Matrix (Conduit homeserver) as the communication fabric.
- OCAP gating across pods: each pod carries its own `DelegationToken`. Cross-pod messages must carry an OCAP token that the receiving pod can verify against its `CapabilityChecker`.

**Recommended approach (when triggered):**
- Use Matrix as the A2A transport (already in deployment model).
- Each pod's `A2ARuntime` wraps a `MatrixTransport` for outbound messages.
- Inbound messages arrive at the pod's Matrix listener → OCAP verification → dispatch.
- OCAP tokens carried in message metadata, verified by the receiving pod's checker.

**Minimal viable protocol:**
```
Pod A → Matrix room → Pod B
  │                      │
  │ Message {             │
  │   sender_webid,       │
  │   ocap_token,         │
  │   payload             │
  │ }                     │
  └──────────────────────┘
                          │
                          ▼
                    CapabilityChecker::check(ocap_token, sender_webid, Tool, tool_name, Execute)
                          │
                          ├─ pass → dispatch to MCP tool
                          └─ fail → reject with 403
```

---

## ζ.2 — Pod Portability Across Servers

**Question:** Is exporting a SQLCipher file sufficient for "move my pod to another server"?

**Status:** Resolved — partially answered by current design.

**What transfers:**
- ✅ SQLCipher database file (`{pod}.db`) — contains all triples (episodic + semantic), embeddings, and metadata.
- ✅ Passphrase determinism — `derive_ocap_secret(webid)` produces the same passphrase on any server (same master key + same WebID → same key material, ADR-027).
- ✅ `.webid` sidecar file — enables passphrase re-derivation without re-authentication.
- ✅ Pod persona and capabilities — embedded in the database (AgentPod stores persona).

**What does NOT transfer:**
- ❌ CNS variety counters — temporal state; reset on import to zero (new server starts fresh CNS observation).
- ❌ Curator cursor state — stored in CuratorPod's `SemanticIndex`; the destination server's Curator starts from cursor 0 and replays.
- ❌ MCP server API keys — scoped to user, not server. User re-authenticates on the new server.
- ❌ Active A2A sessions — Matrix sessions are server-local; need re-registration.

**Import procedure (conceptual):**
```
kask pod export <pod_id> → produces {pod}.db + {pod}.webid
kask pod import <pod_id> {pod}.db {pod}.webid → opens with derived passphrase → activates
```

**Open:** CNS state portability. If variety counters are needed across servers, they should be stored in the database (persistent CNS state). Current CNS is in-memory only — counters reset on restart regardless. This is a CNS enhancement, not a pod portability issue.

---

## ζ.3 — Pod Lifecycle Across Containers

**Question:** If a pod IS a Docker container, does `kask pod activate` become `docker start {pod_id}`?

**Status:** Deferred (trigger: container deployment use case)

**Analysis:**
- Current lifecycle: `Populated → Registered → Activated → Deactivated`. Managed by `ActivePods` in-process.
- Container lifecycle: `Image built → Container created → Container started → Container stopped`.

**Mapping:**
| AgentPod State | Container Equivalent | CLI Command |
|----------------|---------------------|-------------|
| Populated | Image built (Dockerfile generated) | `kask pod export-container` |
| Registered | Container created (`docker create`) | `kask pod create-container` |
| Activated | Container started (`docker start`) | `kask pod start` or `docker start {pod_id}` |
| Deactivated | Container stopped (`docker stop`) | `kask pod stop` or `docker stop {pod_id}` |

**Complication:**
- `ActivePods` currently manages lifecycle in-process. Container-based pods are out-of-process.
- The "pod manager" role splits: `ActivePods` becomes a directory of running containers, not an in-process registry.
- CuratorSync still runs in-process (on the host), polling pod containers' exposed database volumes.

**Recommendation:** When container deployment is implemented, `ActivePods` becomes a thin status tracker that queries Docker/Podman for container state. `PodFactory::deploy` gains a `--container` flag that generates a Containerfile + build context instead of creating a database file directly.

---

## ζ.4 — Curator Aggregation Model

**Question:** Polling vs push for per-pod CNS aggregation?

**Status:** Resolved — polling model implemented in `CuratorSync`.

**Decision: Polling (1-second interval).**

**Rationale:**
- Polling handles restarts naturally — cursor-based catch-up replays missed triples.
- Push requires the Curator to be alive and reachable at write time — fragile.
- CNS events (`cns.semantic.published`) are emitted but serve as observability signals, not the sync trigger. The sync loop runs independently.
- The algedonic pathway (CNS → Curator) is already unidirectional; polling preserves this.

**Implementation:**
- `CuratorSync::run()` — `tokio::select!` loop with `sleep(1s)` + cancel channel.
- `CuratorSync::sync_pod()` — opens source pod with deterministic passphrase, queries triples since cursor, inserts into `SemanticIndex`.
- Cursor-based: on restart, cursor is 0 for all pods → full catch-up replay.
- Conflict: both triples stored → consumer decides.

**CNS aggregation for variety counters:**
- `CnsSpan::SemanticPublished` is emitted per-pod via `CnsRuntime::increment_variety`.
- The CuratorPod's own `CnsRuntime` does not aggregate other pods' variety counters.
- Full CNS aggregation (variety counters across pods) is a future Curator feature, not needed for semantic sync.

---

## ζ.5 — PodFactory Deletion Test

**Question:** Does `PodFactory` earn its existence if pods are created via `docker build`?

**Status:** Resolved — PodFactory survives essentialist G1 test.

**Essentialist G1 (Exist) test:**

1. **Delete PodFactory callers (ActivePods, API).** Does pod creation become impossible? **Yes** — no type constructs a `PodDeployment`. PodFactory earns existence.

2. **Delete PodFactory itself.** Does pod creation become simpler? **No** — the logic that creates the SQLCipher file, derives the passphrase, initializes the CNS runtime, creates memory adapters, and assembles the deployment must exist somewhere. PodFactory IS the minimal encapsulation of this logic.

3. **With container deployment (future).** PodFactory's `deploy()` method would gain `--output containerfile` mode. The factory generates the Containerfile + build context instead of creating the database directly. The factory remains the single constructor entry point — deep module discipline preserved (1 public method on PodFactory).

**Verdict:** PodFactory passes the deletion test. It is a deep module (high benefit: 1 call creates a fully-initialized `PodDeployment`; low cost: 1 public method, stateless).

---

## Summary

| ζ | Question | Status |
|---|----------|--------|
| ζ.1 | Cross-pod A2A protocol | Deferred (trigger: cross-server deployment) |
| ζ.2 | Pod portability | Resolved — DB file + deterministic passphrase = portable |
| ζ.3 | Pod lifecycle across containers | Deferred (trigger: container deployment) |
| ζ.4 | Curator aggregation model | Resolved — polling model (CuratorSync) |
| ζ.5 | PodFactory deletion test | Resolved — survives essentialist G1 |
