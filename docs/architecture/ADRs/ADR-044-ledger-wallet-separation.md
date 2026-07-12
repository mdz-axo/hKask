---
title: "ADR-044: Ledger-Wallet Separation"
audience: [architects, developers]
last_updated: 2026-07-03
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-044: Ledger-Wallet Separation

**Date:** 2026-07-03
**Status:** Active

## Context

`hkask-ledger` is a standalone double-entry accounting library with zero hKask
dependencies. It provides `Ledger`, `LedgerTransaction`, `Posting`, and
`AccountBalance` types for immutable audit trails.

`hkask-wallet` provides self-custody multi-chain deposits, API key issuance, and
rJoule balance management. It tracks transactions via `hkask-storage::wallet`.

During the v0.31.0 architecture audit, the question arose whether the ledger
should integrate with the wallet system so that gas consumption produces
ledger entries.

**Problem Statement:** Should `hkask-wallet` integrate with `hkask-ledger` for
immutable transaction recording, or should they remain separate concerns?

## Decision

**Chosen Approach:** Keep `hkask-ledger` and `hkask-wallet` as separate,
independently-deployable crates with distinct concerns.

**Current ledger consumers:**
- `hkask-services-runtime` (`provider_intel.rs`) — tracks per-provider energy
  costs (gas/rJoule consumption) as immutable ledger entries for cost
  attribution and billing audit.
- `hkask-mcp-companies` (`portfolio.rs`) — tracks financial portfolio positions
  as double-entry postings for fundamental analysis.

**Wallet transaction tracking** uses `hkask-storage::wallet` (SQLite) for
deposit/withdrawal/spend records with rJoule denomination. This is a
separate domain from the ledger's double-entry accounting.

**Alternatives Considered:**
1. **Integrate wallet with ledger** — Rejected. Wallet transactions are
   single-entry rJoule records; ledger is double-entry for multi-asset
   accounting. Forcing single-entry wallet records into double-entry would
   add complexity without benefit. The wallet is a self-custody settlement
   layer; the ledger is an accounting layer.
2. **Remove ledger from workspace** — Rejected. Two active consumers
   demonstrate the ledger earns its keep.

## Consequences

### Positive
- Wallet and ledger remain independently testable
- Wallet's `WalletStore` is optimized for rJoule deposits/spends (single-entry)
- Ledger's `Ledger` is optimized for multi-asset double-entry with idempotency
- No forced coupling between settlement and accounting layers

### Negative
- Wallet gas consumption cannot be automatically reconciled with ledger entries
- Cross-domain audit (wallet spend → ledger entry) requires application-layer
  coordination, not infrastructure

### Neutral
- If a future consumer needs wallet-to-ledger reconciliation, a
  dedicated service bridge can connect them without changing either crate's
  API

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (No trait without two consumers) | ✅ | Ledger has 2 consumers (runtime, companies) |
| **P5** (Essentialism) | ✅ | Ledger's scope is narrowly defined: immutable double-entry accounting |
| **P7** (Prefer deletion over deprecation) | ✅ | Kept as active, not deprecated |

## Verification

```bash
# Verify ledger consumers
grep -rn "hkask_ledger" crates/ mcp-servers/ --include="*.rs" | grep -v "/tests/" | grep -v "hkask-ledger/"
# Expected: 2+ consumers in runtime and companies

# Verify wallet does NOT depend on ledger
grep "hkask-ledger" crates/hkask-wallet/Cargo.toml crates/hkask-services-wallet/Cargo.toml
# Expected: no matches
```

## References

[^fowler-po]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture.* Addison-Wesley. — Double-entry accounting pattern for immutable audit trails.
[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design.* Addison-Wesley. — Bounded context separation: wallet (settlement) and ledger (accounting) are distinct bounded contexts.
