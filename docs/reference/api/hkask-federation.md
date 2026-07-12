---
title: "hkask-federation — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-federation — API Reference

**Purpose:** Cross-instance agent federation protocol. Enables agents running on different hKask instances to discover, connect, and synchronize state via CRDT-based replication.

## Public Modules

| Module | Purpose |
|--------|---------|
| `crdt` | Conflict-free Replicated Data Type implementation for agent state sync |
| `service` | Federation service — orchestrates link lifecycle and sync operations |
| `sync` | Sync protocol — message exchange and state reconciliation |
| `cns_span` | CNS span emission for federation events |

## Key Types

| Type | Description |
|------|-------------|
| `FederationDispatch` | Trait (from `hkask-ports`) — the port contract for federation operations |
| `FederationMessage` | A message exchanged between federated instances |
| `FederationResponse` | Response to a federation message |
| `FederationDispatchError` | Error type for federation dispatch failures |
| `FederationLink` | A connection between two federated instances |
| `LinkResolution` | Result of resolving a federation link (connected, pending, rejected) |
| `CrdtSyncResult` | Result of a CRDT sync operation — merged state |
| `ReplicaId` | Unique identifier for a federated replica (re-exported) |

## Key Functions

| Function | Signature |
|----------|-----------|
| `FederationDispatch::dispatch` | Sends a `FederationMessage` and returns a `FederationResponse` |
| `FederationDispatch::sync_crdt` | Synchronizes CRDT state with a peer replica |
| `FederationDispatch::resolve_link` | Resolves a federation link (discover → connect → verify) |

## Link Lifecycle

1. **Discover** — Instance A discovers Instance B via registry or manual configuration
2. **Connect** — A sends a link request; B accepts or rejects
3. **Sync** — CRDT state is synchronized between A and B
4. **Maintain** — Periodic sync keeps state consistent; link health is monitored

## CNS Integration

Federation events emit CNS spans for link establishment, sync operations, and error conditions. See `cns_span` module for span definitions.
