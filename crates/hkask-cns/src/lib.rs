//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub(crate) mod algedonic; // Loop 6 subloop 6.4 — algedonic signal channel
pub mod api_metering; // API key metering — rate limits, CNS spans, alerts
pub mod calibrated_energy_estimator; // Loop 6 — self-regulating per-server gas estimator
pub mod circuit_breaker; // Loop 6 — regulation
pub mod composite_energy_estimator; // Composite routing: inference → token-based, others → table
pub mod cybernetics_loop; // Loop 6
pub(crate) mod dampener; // Loop 6 — regulation
pub mod energy; // Loop 6 — energy budgets (hJoules)
pub mod energy_budget_management; // Loop 6 — energy budget registration/reservation/settlement
pub mod governed_inference; // Loop 6 → inference call membrane
pub mod governed_tool; // Loop 6 → all tool invocation membranes
pub(crate) mod inference_estimator;
pub mod types; // Loop 6 → Inference energy estimation

pub mod curator_budget_policy;
pub mod dynamic_gas_table;
pub mod gas_report;
pub mod runtime; // Loop 6 — runtime
pub mod seam_watcher; // Public seam watcher — inventory, drift, CNS spans
pub mod set_points; // Loop 6 — set-points config & loaders
pub mod slo_manager; // Loop 6 — SLO evaluation, error budgets, breach escalation
pub(crate) mod snapshot_loop; // Loop 6 — scheduled CAS snapshots
pub(crate) mod table_energy_estimator; // Per-server energy cost table
pub mod wallet_budget; // Loop 6 — wallet-backed energy budgets (Phase 5)
pub(crate) mod wallet_energy_estimator; // Loop 6 — wallet-aware energy estimation (Phase 5)
pub mod wallet_gas_calibrator;
pub mod wallet_manager;
pub mod well;
pub use algedonic::RuntimeAlert;
pub use api_metering::{
    ApiMeter, ApiMeteringAlert, ApiRequestSpan, EndpointWeight, RateLimitStatus, endpoint_weight,
};
pub use calibrated_energy_estimator::{
    CalibratedEnergyEstimator, DEFAULT_CALIBRATION_INTERVAL, DEFAULT_INITIAL_LOOKBACK,
};
pub use circuit_breaker::CircuitBreaker;
pub use composite_energy_estimator::CompositeEnergyEstimator;
pub mod contract_events;
pub use contract_events::{
    emit_contract_accepted, emit_contract_proposed, emit_contract_rejected, emit_contract_violated,
};
pub use cybernetics_loop::CyberneticsLoop;
pub use dynamic_gas_table::DynamicGasTable;
pub use energy::{
    AgentGasStatus, DEFAULT_GAS_ALERT_THRESHOLD, GasBudget, GasCost, GasDelta, GasError,
};
pub use energy_budget_management::GasBudgetManager;
pub use gas_report::{AgentGasReport, AgentGasSummary, GasReport, GasTotals, ToolGasBreakdown};
pub use governed_inference::GovernedInference;
pub use governed_tool::{EnergyEstimator, GovernedTool};
pub use hkask_types::cns::QueueDepth;
pub use runtime::CnsRuntime;
pub use runtime::NoopEventSink;
pub use seam_watcher::{SeamDrift, SeamSummary, SeamWatcher};
pub use set_points::{
    DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD, DEFAULT_CONNECTOR_LATENCY_MAX_SECS,
    DEFAULT_ENERGY_MIN_REMAINING_RATIO, DEFAULT_ERROR_RATE_MAX, DEFAULT_MAX_ITERATIONS,
    DEFAULT_VARIETY_MAX_DEFICIT, SetPoints, SetPointsConfig, load_set_points,
};
pub use slo_manager::{SloDataPoint, SloDataProvider, SloManager, SloManagerError};
pub use snapshot_loop::{SnapshotLoop, SnapshotLoopConfig};
pub use types::curation::CurationThresholdConfig;
pub use types::loops::{
    CurationInput, CuratorDirective, CuratorHandle, ExperienceClassification, LoopAction,
};
pub use wallet_budget::WalletBackedBudget;
pub use wallet_energy_estimator::WalletEnergyEstimator;
pub use wallet_gas_calibrator::{
    DEFAULT_WALLET_CALIBRATION_INTERVAL, DEFAULT_WALLET_INITIAL_LOOKBACK, WalletGasCalibrator,
};
