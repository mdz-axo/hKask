---
title: "CNS Architecture — Responsibility Clusters and Coupling"
audience: [architects, developers]
last_updated: 2026-07-11
version: "0.31.0"
status: "Active"
domain: "Cybernetics"
mds_categories: [composition, lifecycle, curation]
---

# CNS Architecture — Responsibility Clusters and Coupling

This class diagram maps the internal structure of `hkask-cns` after the StorageGuard extraction and WalletBudgetPort introduction. The CNS crate previously conflated seven distinct responsibilities; two have been addressed:

- **StorageGuard** — extracted to `hkask-storage-guard` crate (2026-07-11)
- **Wallet coupling** — resolved via `WalletBudgetPort` trait in `hkask-ports` (2026-07-11). CNS no longer depends on `hkask-wallet`; it depends on the abstract port.

Remaining responsibility clusters still in CNS:

1. **Core Regulation** — `CyberneticsLoop`, `CnsRuntime`, `SetPoints`, `Dampener`, `StagnationDetector`
2. **Energy/Gas** — `GasBudgetManager`, `GasBudget`, `GasCost`, `GasReport`
3. **Wallet Budget** — `WalletBackedBudget`, `WalletGasCalibrator` (now via `WalletBudgetPort`, not concrete)
4. **SLO** — `SloManager`, `SloDataPoint`, `SloDataProvider` (candidate for extraction)
5. **Seam Watching** — `SeamWatcher`, `SeamDrift`, `SeamSummary` (candidate for extraction)
6. **Spans** — `AcpSpan`, `ClassifySpan`, `ContractSpan`, `InfraSpan`, `QaSpan`, `SloSpan`, `SeamSpan` (candidate for extraction)

See also: [flowchart-cns-homeostatic-loop.md](flowchart-cns-homeostatic-loop.md) for the sense→compare→compute→act cycle, and [flowchart-cns-regulation.md](flowchart-cns-regulation.md) for regulation policy dispatch.

```mermaid
classDiagram
    class CyberneticsLoop {
        +cns: Arc~RwLock~CnsRuntime~~
        +gas_budget_manager: Arc~RwLock~GasBudgetManager~~
        +well_manager: Arc~RwLock~WellManager~~
        +wallet_manager: Option~Arc~dyn WalletBudgetPort~~
        +set_points: SetPoints
        +dampener: Arc~Dampener~~
        +stagnation_detector: Arc~StagnationDetector~~
        +sensor_registry: Arc~SensorRegistry~~
        +sense_compare_compute_act()
    }

    class CnsRuntime {
        +variety_monitor: VarietyMonitor
        +regulation_cycle()
    }

    class SetPoints {
        +variety_max_deficit: u32
        +error_rate_max: f64
        +energy_min_remaining_ratio: f64
        +communication_backpressure: u64
    }

    class GasBudgetManager {
        +register_agent()
        +reserve()
        +settle()
        +get_status()
    }

    class GasBudget {
        +cap: u64
        +remaining: u64
    }

    class WalletBackedBudget {
        +wallet_manager: Arc~dyn WalletBudgetPort~~
        +spend()
        +draw_from_well()
    }

    class WalletGasCalibrator {
        +wallet_manager: Arc~dyn WalletBudgetPort~~
        +calibrate()
    }

    class SloManager {
        +evaluate()
        +check_breach()
    }

    class SeamWatcher {
        +inventory()
        +detect_drift()
    }

    class GovernedTool~P~ {
        +port: P
        +energy_estimator: Box~dyn EnergyEstimator~~
        +circuit_breaker: CircuitBreaker
        +invoke()
    }

    class SensorRegistry {
        +sensors: Vec~Box~dyn SensorProvider~~
        +read_all()
    }

    class CircuitBreaker {
        +state: CircuitState
        +record_success()
        +record_failure()
        +allow()
    }

    class ToolStats {
        +cost_distribution: CostDistribution
        +reliability: Beta
    }

    %% Core regulation cluster
    CyberneticsLoop --> CnsRuntime : owns
    CyberneticsLoop --> SetPoints : reads
    CyberneticsLoop --> GasBudgetManager : owns
    CyberneticsLoop --> SensorRegistry : owns
    CyberneticsLoop --> CircuitBreaker : uses

    %% Energy cluster
    GasBudgetManager --> GasBudget : manages

    %% Wallet coupling — now via port trait (hexagonal)
    CyberneticsLoop --> WalletBackedBudget : owns
    CyberneticsLoop --> WalletGasCalibrator : owns
    WalletBackedBudget --> WalletBudgetPort : port dep
    WalletGasCalibrator --> WalletBudgetPort : port dep

    %% SLO cluster
    CyberneticsLoop --> SloManager : owns

    %% Seam watching — observability, not regulation
    SeamWatcher ..> CyberneticsLoop : "candidate for extraction"

    %% Tool governance
    GovernedTool --> ToolStats : reads
    GovernedTool --> CircuitBreaker : uses

    %% Port trait — implemented by hkask-wallet, consumed by CNS
    class WalletBudgetPort {
        <<interface>>
        +gas_to_rjoules(gas) RJoule
        +get_encumbrance(key_id) Option
        +can_afford(wallet_id, cost) bool
        +get_balance(wallet_id) Result
        +consume(key_id, rj) Result
        +settle_rjoules(wallet_id, reserved, actual) Result
        +gas_per_rjoule() u64
        +set_gas_per_rjoule(rate)
        +emit_key_alert(key_id, exhausted, expired)
        +get_api_key(key_id) Option
    }

    class StorageGuardLoop {
        +config: StorageGuardConfig
        +walk_dir()
        +enforce_limits()
    }

    %% StorageGuard — extracted to hkask-storage-guard
    StorageGuardLoop ..> HkaskLoop : implements (in hkask-storage-guard)
```

## Coupling Analysis — Current State

| Edge | Type | Status |
|------|------|--------|
| `CyberneticsLoop → WalletBudgetPort` | Port trait (hexagonal) | ✅ Resolved — CNS depends on abstract port, not concrete `WalletManager` |
| `WalletBackedBudget → WalletBudgetPort` | Port trait | ✅ Resolved — uses `Arc<dyn WalletBudgetPort>` |
| `WalletGasCalibrator → WalletBudgetPort` | Port trait | ✅ Resolved — uses `Arc<dyn WalletBudgetPort>` |
| `StorageGuardLoop` | Extracted | ✅ Resolved — now in `hkask-storage-guard` crate |
| `SeamWatcher` in CNS | Wrong crate | ⬜ Candidate for extraction — seam observability is not regulation |
| `SloManager` in CNS | Wrong crate | ⬜ Candidate for extraction — SLO evaluation is independent of the cybernetic loop |

## Completed Extractions

| Step | Extract | Target Crate | Status |
|------|---------|-------------|--------|
| 2a | `StorageGuardLoop` + `StorageGuardConfig` | `hkask-storage-guard` | ✅ Done 2026-07-11 |
| — | `WalletBudgetPort` trait | `hkask-ports` | ✅ Done 2026-07-11 |

## Remaining Extraction Candidates

| Step | Extract | Target | Lines | Independence |
|------|---------|--------|-------|-------------|
| 2b | `SloManager` + `SloDataPoint` + `SloDataProvider` | new slo crate | ~1000 | Depends only on `CnsObserver` port |
| 2c | `SeamWatcher` + `SeamDrift` + `SeamSummary` + `SeamTypes` | new seam crate | ~900 | Fully independent — observability |
| 2d | Span types (`AcpSpan`, `ClassifySpan`, `ContractSpan`, `InfraSpan`, `QaSpan`, `SloSpan`, `SeamSpan`) | new cns-spans crate | ~800 | Depends on `hkask-types::event` |
| 2e | Energy/gas (`GasBudget`, `GasCost`, `GasBudgetManager`, `GasReport`, `DynamicGasTable`) | new energy crate | ~1800 | Depends on `hkask-types` |
| 2f | Estimators (`CalibratedEnergyEstimator`, `CompositeEnergyEstimator`, `TableEnergyEstimator`, `InferenceEstimator`) | new energy-estimators crate | ~1300 | Depends on energy crate |

After extraction, CNS core retains: `CyberneticsLoop`, `CnsRuntime`, `Algedonic`, `Dampener`, `RegulationPolicy`, `GovernedTool`, `GovernedInference`, `SensorProvider`, `SetPoints`, `StrategyEvaluator`, `SystemSimulator`, `ToolStats`, `WalletBackedBudget`, `WalletGasCalibrator` — all cohesive regulation logic.

<!-- DIAGRAM_ALIGNMENT
id: DIAG-IC-012
verified_date: 2026-07-11
verified_against: crates/hkask-cns/src/cybernetics_loop.rs, crates/hkask-cns/src/runtime.rs, crates/hkask-cns/src/wallet_budget.rs, crates/hkask-cns/src/wallet_gas_calibrator.rs, crates/hkask-cns/src/slo_manager.rs, crates/hkask-cns/src/seam_watcher.rs, crates/hkask-cns/src/governed_tool.rs, crates/hkask-storage-guard/src/lib.rs, crates/hkask-ports/src/wallet_budget_port.rs
status: VERIFIED
-->
