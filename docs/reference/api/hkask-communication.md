---
title: "hkask-communication — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-communication — API Reference

**Purpose:** Core Matrix transport, agent registry, and 7R7 listener for agent-to-agent communication.

## Public Modules (feature-gated)

| Module | Purpose | Feature |
|--------|---------|---------|
| `agent_registration` | Agent registration and discovery on Matrix | `matrix` |
| `listener` | 7R7 message listener for incoming agent messages | `matrix` |
| `matrix` | Matrix protocol integration (homeserver communication) | `matrix` |

## Key Types

| Type | Description |
|------|-------------|
| `AgentRegistry` | Registry mapping agent IDs to Matrix room/user identifiers. Supports record, resolve, deregister, and monitor operations. |
| `MatrixTransport` | Transport layer for sending and receiving messages via Matrix |
| `AgentRegistrationError` | Error type for agent registration failures |
| `MatrixError` | Error type for Matrix protocol failures |

## Key Functions

| Function | Signature |
|----------|-----------|
| `AgentRegistry::record` | Registers an agent with its Matrix identity |
| `AgentRegistry::resolve` | Resolves an agent ID to its Matrix room/user |
| `AgentRegistry::deregister` | Removes an agent from the registry |
| `AgentRegistry::monitor` | Returns a stream of registry change events |
| `AgentRegistry::watchers` | Returns registered watchers for an agent |

## Features

| Feature | Effect |
|---------|--------|
| `matrix` (default) | Enables Matrix protocol integration — `agent_registration`, `listener`, `matrix` modules |
| `matrix` disabled | Crate provides only type definitions (RoomId, UserId, Thread, MatrixMessage) |

## Integration Tests

19 integration tests in `crates/hkask-communication/tests/integration_test.rs`:
- Type tests: RoomId, UserId, Thread, MatrixMessage (7 tests)
- Error tests: MatrixError, AgentRegistrationError (4 tests)
- AgentRegistry tests: record, resolve, deregister, monitor, watchers (8 tests)

MatrixTransport tests require a running Conduit homeserver (Docker sidecar) and are deferred.
