---
title: "hkask-ledger — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-ledger — API Reference

**Purpose:** Double-entry accounting ledger for immutable financial transactions. Records every energy deposit, withdrawal, and API key issuance as permanent ledger entries.

## Key Types

| Type | Description |
|------|-------------|
| `Ledger` | The ledger itself — persists transactions immutably |
| `LedgerTransaction` | A single transaction with timestamp, description, and postings |
| `Posting` | A single entry in a transaction — debits one account, credits another |
| `AccountBalance` | Balance snapshot for a single account |
| `DateRange` | Inclusive date range for queries (start → end) |
| `QueryFilter` | Filter criteria for ledger queries (date range, account, transaction type) |
| `LedgerError` | Ledger-specific error type |

## Key Functions

| Function | Signature |
|----------|-----------|
| `Ledger::record` | Records a new transaction with postings (idempotent by transaction hash) |
| `Ledger::balance` | Returns the current balance for an account |
| `Ledger::query` | Queries transactions matching a filter |
| `Ledger::verify` | Verifies that all postings balance (sum of debits = sum of credits) |

## Double-Entry Model

Every `LedgerTransaction` contains at least two `Posting` entries:
- A **debit** (removes from one account)
- A **credit** (adds to another account)

The sum of all debit amounts must equal the sum of all credit amounts. The ledger enforces this invariant at record time.

## Idempotency

Transactions are keyed by their content hash. Recording the same transaction twice is a no-op — the ledger returns the existing entry rather than duplicating.

## CNS Integration

Ledger events emit CNS spans for deposit confirmation and withdrawal settlement. Used by the gas budget system to track energy consumption against wallet balances.
