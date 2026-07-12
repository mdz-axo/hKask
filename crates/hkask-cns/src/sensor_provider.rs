//! SensorProvider trait — pluggable metric sensors (Fermi Extractor pattern).
//!
//! Fermi's `Extractor` trait separates domain data extraction from the fitting
//! loop. SensorProvider applies the same pattern to hKask's regulation loop:
//! each metric gets its own `SensorProvider` implementation, registered with
//! a `SensorRegistry`. The `CyberneticsLoop::sense()` method walks the registry
//! instead of containing inline sensing logic.
//!
//! ## Why this lives in hkask-cns
//!
//! Sensor providers are CNS regulation infrastructure. They live alongside
//! `CyberneticsLoop`, `StagnationDetector`, and `SetPoints` in `hkask-cns`,
//! the crate responsible for homeostatic self-regulation.

use super::types::loops::{LoopId, Signal, SignalMetric};
use parking_lot::Mutex;
use std::sync::Arc;

/// A pluggable sensor that produces one kind of signal metric.
///
/// Each implementation senses a single `SignalMetric` from its data source.
/// Fermi pattern: the `Extractor` trait takes a domain payload and produces
/// a scalar; `SensorProvider` takes system state and produces an optional
/// `Signal`. If the sensor has nothing to report (metric is healthy),
/// it returns `None`.
#[async_trait::async_trait]
pub trait SensorProvider: Send + Sync {
    /// Sense the current state and produce a signal if the metric is
    /// in a reportable state. Returns `None` if nothing to report.
    async fn sense(&self) -> Option<Signal>;
}

/// Registry of sensor providers, walked by `sense()` each tick.
///
/// Providers are registered at construction time and executed in order.
/// Order doesn't matter — each provider independently decides whether
/// to emit a signal.
pub struct SensorRegistry {
    providers: Mutex<Vec<Arc<dyn SensorProvider>>>,
}

impl SensorRegistry {
    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub fn new() -> Self {
        Self {
            providers: Mutex::new(Vec::new()),
        }
    }

    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub fn register(&self, provider: Arc<dyn SensorProvider>) {
        self.providers.lock().push(provider);
    }

    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub async fn sense_all(&self, source: LoopId) -> Vec<Signal> {
        let providers: Vec<Arc<dyn SensorProvider>> = { self.providers.lock().clone() }; // Lock dropped here — no .await while holding it.
        let mut signals = Vec::new();
        for provider in &providers {
            if let Some(signal) = provider.sense().await {
                signals.push(signal);
            }
        }
        for s in &mut signals {
            s.source = source;
        }
        signals
    }
}

impl Default for SensorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// CONCRETE SENSOR PROVIDERS
// ═════════════════════════════════════════════════════════════════════════════

/// Senses energy budget remaining ratios across all agents.
///
/// Data source: `GasBudgetManager`. Produces a signal per agent.
pub(crate) struct EnergyBudgetSensor {
    budget_manager: Arc<tokio::sync::RwLock<super::energy_budget_management::GasBudgetManager>>,
    set_point: f64,
}

impl EnergyBudgetSensor {
    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub(crate) fn new(
        budget_manager: Arc<tokio::sync::RwLock<super::energy_budget_management::GasBudgetManager>>,
        set_point: f64,
    ) -> Self {
        Self {
            budget_manager,
            set_point,
        }
    }
}

#[async_trait::async_trait]
impl SensorProvider for EnergyBudgetSensor {
    async fn sense(&self) -> Option<Signal> {
        let statuses = self.budget_manager.read().await.all_agent_statuses().await;
        // Use the worst remaining ratio as the aggregate signal.
        let worst = statuses
            .iter()
            .map(|(_, s)| s.remaining.0 as f64 / s.cap.0.max(1) as f64)
            .fold(1.0, f64::min);
        Some(Signal::new(
            LoopId::Cybernetics, // placeholder — registry backfills
            SignalMetric::EnergyRemaining,
            worst,
            self.set_point,
        ))
    }
}

/// Senses variety deficit from the CNS runtime.
///
/// Data source: `CnsRuntime`. Produces a single aggregate signal.
pub(crate) struct VarietySensor {
    cns: Arc<tokio::sync::RwLock<super::runtime::CnsRuntime>>,
    set_point: f64,
}

impl VarietySensor {
    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    // Wired for future use when data sources are refactored.
    pub(crate) fn new(
        cns: Arc<tokio::sync::RwLock<super::runtime::CnsRuntime>>,
        set_point: f64,
    ) -> Self {
        Self { cns, set_point }
    }
}

#[async_trait::async_trait]
impl SensorProvider for VarietySensor {
    async fn sense(&self) -> Option<Signal> {
        let cns = self.cns.read().await;
        let health = cns.health().await;
        Some(Signal::new(
            LoopId::Cybernetics, // placeholder — registry backfills
            SignalMetric::VarietyDeficit,
            health.overall_deficit as f64,
            self.set_point,
        ))
    }
}

/// Senses wallet API key health from the gas budget manager.
pub(crate) struct WalletKeyHealthSensor {
    budget_manager: Arc<tokio::sync::RwLock<super::energy_budget_management::GasBudgetManager>>,
}

impl WalletKeyHealthSensor {
    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub(crate) fn new(
        budget_manager: Arc<tokio::sync::RwLock<super::energy_budget_management::GasBudgetManager>>,
    ) -> Self {
        Self { budget_manager }
    }
}

#[async_trait::async_trait]
impl SensorProvider for WalletKeyHealthSensor {
    async fn sense(&self) -> Option<Signal> {
        let key_alerts = self.budget_manager.read().await.wallet_key_alerts().await;
        if key_alerts.is_empty() {
            return None; // Healthy — nothing to report.
        }
        Some(Signal::new(
            LoopId::Cybernetics, // placeholder — registry backfills
            SignalMetric::WalletKeyHealth,
            1.0, // alert active
            0.0, // set-point: no alerts
        ))
    }
}

/// Senses tool reliability across all MCP tools.
pub(crate) struct ToolReliabilitySensor {
    tool_stats: Arc<crate::tool_stats::ToolStats>,
    threshold: f64,
}

impl ToolReliabilitySensor {
    /// expect: "The system provides pluggable metric sensing for the cybernetic regulation loop"
    pub(crate) fn new(tool_stats: Arc<crate::tool_stats::ToolStats>, threshold: f64) -> Self {
        Self {
            tool_stats,
            threshold,
        }
    }
}

#[async_trait::async_trait]
impl SensorProvider for ToolReliabilitySensor {
    async fn sense(&self) -> Option<Signal> {
        let alerts = self.tool_stats.reliability_alerts().await;
        if alerts.is_empty() {
            return None;
        }
        let worst = alerts
            .iter()
            .map(|a| a.success_probability)
            .fold(1.0, f64::min);
        Some(Signal::new(
            LoopId::Cybernetics, // placeholder — registry backfills
            SignalMetric::ToolReliability,
            worst,
            self.threshold,
        ))
    }
}
