---
title: "Public Surface Justification — hkask-test-harness"
audience: [architects, developers]
last_updated: 2026-06-16
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-test-harness

**Crate:** `hkask-test-harness`
**Public items in lib.rs:** 42
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-test-harness` is the **shared test infrastructure crate** — it provides mocks, fixtures, and test utilities consumed by every other crate's integration tests. Its surface is large because it must cover the full breadth of hKask's domain:

1. **Database fixtures** — `TestDb` provides isolated SQLite databases for storage-layer tests across `hkask-storage`, `hkask-memory`, `hkask-templates`, and MCP servers.
2. **Keystore fixtures** — `TestKeystore` creates ephemeral key material for `hkask-keystore`, `hkask-wallet`, and `hkask-services` tests.
3. **Identity fixtures** — `TestWebId` generates deterministic and random WebIDs for sovereignty and consent tests.
4. **CNS mocks** — `MockCnsRuntime`, `MockCnsState`, `MockAlgedonicSignal`, and `MockToolState` provide controllable cybernetic state for `hkask-cns` and agent orchestration tests.
5. **Event/triple factories** — `test_event`, `test_triple`, `test_event_with_observer`, `test_triple_with_owner` produce valid CNS events and memory triples.
6. **Property-based strategies** — `strategies` module provides `proptest` strategies for generative testing across the workspace.

## Mitigations

- **Single consumer pattern:** Each public item is used by at least two other crates' test suites — no single-use exports.
- **Submodule organization:** `mocks/`, `strategies/`, and `schema/` separate concerns.
- **No production dependency:** `hkask-test-harness` is only a `[dev-dependencies]` entry — it never ships in release binaries.

## Deletion Test

Delete `hkask-test-harness` and every other crate must reimplement its own TestDb, TestKeystore, TestWebId, MockCnsRuntime, and proptest strategies. The complexity reappears scattered across 17+ crates. The crate earns its existence as shared test infrastructure.
