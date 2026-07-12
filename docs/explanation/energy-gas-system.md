---
title: "Energy & Gas System — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Energy & Gas System

## Why hKask Has an Economic Layer

Agent platforms face a fundamental problem: autonomous loops can consume resources without bound. An agent that retries failed operations indefinitely, or a skill that iterates without converging, becomes a resource sink. hKask addresses this with an explicit economic layer — gas budgets, energy tracking, and wallet accounting — that makes resource consumption visible, measurable, and governable.

This is not cryptocurrency speculation. Gas is a dimensionless unit of computational work, analogous to Ethereum gas: it bounds execution and prevents infinite loops. rJoule is the unit of inference energy — the scarce resource that powers LLM calls. Together they form a dual-currency system: gas for compute, rJoules for inference.

## GasBudget: The Entry Point

The `GasBudget` struct (`crates/hkask-cns/src/energy.rs`) is the primary enforcement mechanism. Every skill and agent carries a budget with a `cap`, a `remaining` balance, a `reserved` hold, and a `replenish_rate`. Fields are private; invariants like `remaining + reserved ≤ cap` are enforced structurally — you cannot construct a `GasBudget` that violates them.

```rust
pub struct GasBudget {
    cap: GasCost,
    remaining: GasCost,
    replenish_rate: GasCost,
    alert_threshold: f64,
    hard_limit: bool,
    reserved: GasCost,
    priority: f64,
    last_reservation: Option<chrono::DateTime<chrono::Utc>>,
}
```

The hold-settle pattern is the core mechanism:

1. **Reserve** — `budget.reserve(gas)` allocates gas from `remaining` to `reserved`. Returns `GasError::BudgetExceeded` if the hard limit is breached. The reservation is timestamped for stale-reservation detection.
2. **Execute** — The tool or operation runs, consuming resources.
3. **Settle** — `budget.settle(reserved, actual)` releases the reservation and deducts actual cost from remaining. If actual < reserved, the difference is refunded — preventing gas leaks from overestimation.

Stale reservations older than 300 seconds (`RESERVATION_TIMEOUT_SECS`) are auto-released. This prevents leaked reservations from permanently reducing available budget. An `AgentGasStatus` snapshot (`cap`, `remaining`, `reserved`, `available`, `usage_ratio`) is derived from any budget for observability.

## The Well: Source of All Gas

Gas doesn't materialize from nowhere. A `Well` (`crates/hkask-cns/src/well.rs`) is the sole source of new gas and rJoules entering the system. One default Well per installation. Wells produce gas and rJoules on a replenishment schedule — `gas_rate` and `rjoule_rate` — and agents draw from Wells to fill their wallets.

```rust
pub struct Well {
    pub config: WellConfig,
    pub gas_available: GasCost,
    pub rjoule_available: u64,
}
```

`WellManager` manages multiple Wells with dampening logic: when the default Well is exhausted, it emits an algedonic alert, but `was_already_exhausted` prevents re-alerting every tick. Wells persist their state via JSON serialization so gas production survives restarts.

## Wallets: Per-Agent Balances

Where the Well is the source, the `WalletManager` (`crates/hkask-cns/src/wallet_manager.rs`) is distribution. Backed by SQLite via `WalletStore`, it creates per-agent wallets on replicant registration, manages deposits and spending, and auto-draws from the Well when balances run low.

```rust
pub async fn spend(&self, agent: &WebID, amount: GasCost) -> Result<GasCost, GasError> {
    // If balance insufficient, auto-draw from Well
    // Then debit the wallet
}
```

The `WalletBackedBudget` (`crates/hkask-cns/src/wallet_budget.rs`) extends this for paid agents. Unlike standard `GasBudget` which replenishes from a dimensionless pool, `WalletBackedBudget` converts gas costs to rJoules and debits a real wallet. It always has `hard_limit = true` — rJoules represent real value. When an API key is attached, it checks encumbrance (allocated rJoule reservation) rather than raw wallet balance.

## RJoule: The Currency of Inference

rJoule is defined in `hkask-wallet-types`. The conversion rate is configured via `GAS_PER_RJOULE` — 250,000 gas cycles = 1 rJoule, reflecting the cost differential between cheap local compute and expensive LLM inference. This rate is runtime-adjustable; the CNS calibration loop in `CalibratedEnergyEstimator` updates it based on observed settlement data.

The `hkask-wallet` crate (`crates/hkask-wallet/src/lib.rs`) provides the full wallet infrastructure: multi-chain support via `ChainPort` trait (Hedera port is feature-gated), self-custody via HKDF-derived keys, API key issuance, price feeds for fee estimation, and zeroizing wrappers on all secret key material.

## The Ledger: Double-Entry Accounting

`hkask-ledger` (`crates/hkask-ledger/src/lib.rs`) provides immutable double-entry accounting backed by SQLite. Three domain ledgers (cost, crypto, securities) share this crate with separate database files. Every transaction is composed of `Posting` entries — source account, destination account, asset, amount — and transactions are committed with a `reference` for idempotency.

```rust
pub struct Posting {
    pub source: String,
    pub destination: String,
    pub asset: String,
    pub amount: i64,  // in asset's smallest integer unit
}
```

Three invariants are structurally enforced: idempotency (same reference, identical postings → no-op; different postings → `IdempotencyConflict`), double-entry (postings must sum to zero), and immutability (committed transactions are never modified or deleted).

## CNS Feedback Loop

The economic layer doesn't exist in isolation — it feeds directly into CNS regulation. `cns.gas` spans emit on every reserve, settle, consume, and reset operation. `GasReport` (`crates/hkask-cns/src/gas_report.rs`) queries these events and produces per-agent `AgentGasSummary` reports with per-tool breakdowns (`ToolGasBreakdown`: reserved, consumed, depleted, invocations).

`CalibratedEnergyEstimator` (`crates/hkask-cns/src/calibrated_energy_estimator.rs`) closes the Good Regulator feedback loop. It wraps `CompositeEnergyEstimator` and keeps its per-server table in sync with observed `cns.gas.settled` events via `DynamicGasTable`. A background calibration task refreshes estimates every 5 minutes, adjusting by exponential moving average. This is P9 (Homeostatic Self-Regulation) at work: estimates are continuously validated against reality and adjusted to minimize prediction error.

## Provider Intelligence

`EnergyEstimator` (`crates/hkask-cns/src/governed_tool.rs`) is the trait that enables cost-aware routing. Different tool categories have different cost models: `InferenceEnergyEstimator` estimates by token count; `TableEnergyEstimator` uses flat per-server costs. The `GovernedTool` membrane — which wraps every tool invocation — checks OCAP authority, reserves gas, invokes the tool, settles actual cost, and emits CNS spans. This is where the economic layer becomes the enforcement layer.
