---
title: "Federation Model — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
last-verified-against: "3d1a876f"
---

# Federation Model

**Purpose:** Explain hKask's cross-instance federation protocol — how agents running on different hKask instances discover each other, synchronize state, and communicate as if they shared a single namespace.

## Why Federation?

hKask instances are sovereign by design (P1). Each instance owns its data, its agents, and its delegation boundaries. But agents need to communicate across instances — a Curator on instance A may need to coordinate with a skill-execution agent on instance B.

Federation solves this without violating sovereignty. Instances remain independent; they choose to federate. The protocol is opt-in, consent-bound, and revocable — consistent with P2 (Affirmative Consent).

## The FederationDispatch Contract

The `FederationDispatch` trait (in `hkask-ports`) defines the port contract:

```rust
pub trait FederationDispatch {
    fn dispatch(&self, message: FederationMessage)
        -> Result<FederationResponse, FederationDispatchError>;
    fn sync_crdt(&self, peer: ReplicaId)
        -> Result<CrdtSyncResult, FederationDispatchError>;
    fn resolve_link(&self, link: FederationLink)
        -> Result<LinkResolution, FederationDispatchError>;
}
```

This trait lives in the ports layer — instances can swap federation implementations without changing the agents that use them.

## CRDT-Based Sync

Federation uses Conflict-free Replicated Data Types (CRDTs) for state synchronization. This design exists because CRDTs have a key property: **any two replicas that have seen the same set of updates will converge to the same state, regardless of the order in which they received those updates.** This means:

- No consensus protocol needed (no leader election, no voting)
- Instances can go offline and resync without conflict resolution
- Network partitions are tolerated — each side continues operating independently

The `crdt` module in `hkask-federation` implements the CRDT data structures used for agent registry synchronization.

## Link Lifecycle

Federation links follow a four-phase lifecycle:

1. **Discover** — Instance A discovers Instance B via a configured registry URL or manual peer list. Discovery is mutual — both sides must know about each other.

2. **Connect** — A sends a link request containing its `ReplicaId` and public key. B validates the request against its consent policy and accepts or rejects. This step implements P2: federation is opt-in and consent-bound.

3. **Sync** — Once linked, the instances exchange CRDT state. The first sync is a full state transfer; subsequent syncs are incremental (delta-based). The sync protocol ensures eventual consistency.

4. **Maintain** — Periodic heartbeat syncs keep state current. Link health is monitored: if a peer is unreachable for a configurable threshold, the link is marked degraded and a CNS span is emitted.

## Merged Registries

When instances federate, their agent registries merge. A Curator on instance A can discover agents on instance B as if they were local. However, the merge is **namespace-aware** — agent IDs are qualified by their home instance to prevent collisions.

This is implemented as a CRDT merge of the registry state. Each instance contributes its own agents; the merged registry is the union. Conflicts (same agent ID on two instances) are resolved by `ReplicaId` precedence — the instance with the lexicographically lower `ReplicaId` wins.

## CNS Integration

Federation events emit CNS spans for observability:

- `cns.federation.link.established` — New federation link created
- `cns.federation.link.degraded` — Peer unreachable, link health degraded
- `cns.federation.link.terminated` — Link explicitly terminated
- `cns.federation.sync.completed` — CRDT sync finished successfully
- `cns.federation.sync.failed` — Sync error (will retry)

These spans feed the CNS homeostatic loop: if federation links degrade across the board, the CNS can escalate to the Curator for investigation.

## Sovereignty Guarantees

Federation does not weaken hKask's sovereignty guarantees:

- **No ambient sharing:** An instance only federates with peers it explicitly configures.
- **Revocable:** Any instance can terminate a federation link at any time — the CRDT state on the local side continues operating independently.
- **OCAP-preserving:** Federation messages pass through the OCAP membrane. A federated agent cannot access tools or data that its capability tokens don't authorize.
- **P4-compliant:** Each instance's pod boundary remains its OCAP enforcement perimeter. Federation does not create cross-pod access paths.

## Relationship to Other Subsystems

| Subsystem | Federation Role |
|-----------|----------------|
| **CNS** | Monitors link health, emits federation spans |
| **Registry** | Merged agent registries via CRDT sync |
| **Curator** | Receives federation escalations (degraded links, sync failures) |
| **A2A** | Agent-to-agent messages route through federation when agents are on different instances |
| **Consent** | Federation links require mutual consent (P2) |
