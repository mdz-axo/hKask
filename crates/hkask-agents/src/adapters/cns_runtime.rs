//! CNS Runtime Adapter — Bridges CnsRuntime handles to domain-native types
//!
//! Maps `hkask_cns` types to the types defined in the ports module.
//!
//! # OCAP Handle Migration (Phase 2)
//!
//! This module provides both the legacy monolithic `CnsRuntimeAdapter` (deprecated)
//! and the new capability-disciplined handle types:
//!
//! - `CnsWriteAdapter` — wraps `CnsWriteHandle` (emit spans, increment variety)
//! - `CnsGovernReadAdapter` — wraps `CnsGovernReadHandle` (read variety, health, sovereignty)
//! - `CnsGovernWriteAdapter` — wraps `CnsGovernWriteHandle` (read + calibrate thresholds)
//! - `CnsAdminAdapter` — wraps `CnsAdminHandle` (reset alerts, subscribe)

use crate::ports::{AlertInfo, AlertLevel, HealthStatus};
use hkask_cns::{
    CnsAdminHandle, CnsGovernReadHandle, CnsGovernWriteHandle, CnsRuntime, CnsWriteHandle,
};
use hkask_types::WebID;
use std::sync::Arc;

fn map_severity(s: hkask_cns::AlertSeverity) -> AlertLevel {
    match s {
        hkask_cns::AlertSeverity::Info => AlertLevel::Info,
        hkask_cns::AlertSeverity::Warning => AlertLevel::Warning,
        hkask_cns::AlertSeverity::Critical => AlertLevel::Critical,
    }
}

fn map_alert(a: hkask_cns::RuntimeAlert) -> AlertInfo {
    AlertInfo {
        domain: a.domain,
        deficit: a.deficit,
        threshold: a.threshold,
        severity: map_severity(a.severity),
        escalated: a.escalated,
        message: a.message,
    }
}

fn map_health(h: hkask_cns::CnsHealth) -> HealthStatus {
    HealthStatus {
        overall_deficit: h.overall_deficit,
        critical_count: h.critical_count,
        warning_count: h.warning_count,
        healthy: h.healthy,
    }
}

// =============================================================================
// Legacy monolithic adapter — DEPRECATED, use handle-specific adapters below
// =============================================================================

/// Legacy monolithic CNS adapter.
///
/// **Deprecated:** Use `CnsWriteAdapter`, `CnsGovernReadAdapter`,
/// `CnsGovernWriteAdapter`, or `CnsAdminAdapter` instead.
/// These enforce OCAP discipline at the type level.
#[deprecated(
    note = "Use CnsWriteAdapter, CnsGovernReadAdapter, CnsGovernWriteAdapter, or CnsAdminAdapter instead"
)]
pub struct CnsRuntimeAdapter {
    runtime: Arc<CnsRuntime>,
}

#[allow(deprecated)]
impl CnsRuntimeAdapter {
    pub fn new(runtime: Arc<CnsRuntime>) -> Self {
        Self { runtime }
    }

    pub async fn health(&self) -> HealthStatus {
        map_health(self.runtime.health().await)
    }

    pub async fn variety(&self) -> Vec<(String, u64)> {
        self.runtime.variety().await
    }

    pub async fn alerts(&self) -> Vec<AlertInfo> {
        self.runtime
            .alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    pub async fn critical_alerts(&self) -> Vec<AlertInfo> {
        self.runtime
            .critical_alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.runtime.variety_for_domain(domain).await
    }

    /// Increment variety and check thresholds.
    /// Returns any algedonic alert if the threshold was crossed.
    pub async fn increment_and_check(&self, domain: &str, state_name: &str) -> Option<AlertInfo> {
        self.runtime.increment_variety(domain, state_name).await;
        self.runtime.check_variety(domain).await.map(map_alert)
    }
}

// =============================================================================
// OCAP-disciplined handle adapters
// =============================================================================

/// CNS Write Adapter — wraps `CnsWriteHandle`.
///
/// Used by inference and memory loops to emit spans and increment variety.
/// CANNOT reset alerts, subscribe, or process sovereignty events.
pub struct CnsWriteAdapter {
    handle: CnsWriteHandle,
}

impl CnsWriteAdapter {
    /// Create a write adapter from a CnsRuntime reference.
    pub fn new(runtime: Arc<CnsRuntime>, emitter: WebID) -> Self {
        Self {
            handle: runtime.write_handle(emitter),
        }
    }

    /// Create a write adapter from an existing CnsWriteHandle.
    pub fn from_handle(handle: CnsWriteHandle) -> Self {
        Self { handle }
    }

    /// The agent this adapter emits spans on behalf of.
    pub fn emitter(&self) -> &WebID {
        self.handle.emitter()
    }

    /// Increment variety counter and check thresholds.
    pub async fn increment_and_check(&self, domain: &str, state_name: &str) -> Option<AlertInfo> {
        self.handle
            .increment_and_check(domain, state_name)
            .await
            .map(map_alert)
    }

    /// Increment variety counter for domain.
    pub async fn increment_variety(&self, domain: &str, state_name: &str) {
        self.handle.increment_variety(domain, state_name).await
    }

    /// Check variety for a specific domain.
    pub async fn check_variety(&self, domain: &str) -> Option<AlertInfo> {
        self.handle.check_variety(domain).await.map(map_alert)
    }
}

/// CNS Governance Read Adapter — wraps `CnsGovernReadHandle`.
///
/// Used by Governance to read observability data for policy decisions.
/// CANNOT set expected variety, calibrate thresholds, or emit spans.
pub struct CnsGovernReadAdapter {
    handle: CnsGovernReadHandle,
}

impl CnsGovernReadAdapter {
    /// Create a governance read adapter from a CnsRuntime reference.
    pub fn new(runtime: Arc<CnsRuntime>, governor: WebID) -> Self {
        Self {
            handle: runtime.govern_read_handle(governor),
        }
    }

    /// Create a governance read adapter from an existing CnsGovernReadHandle.
    pub fn from_handle(handle: CnsGovernReadHandle) -> Self {
        Self { handle }
    }

    /// The agent performing governance reads.
    pub fn governor(&self) -> &WebID {
        self.handle.governor()
    }

    /// Get CNS health status.
    pub async fn health(&self) -> HealthStatus {
        map_health(self.handle.health().await)
    }

    /// Get variety counters for all domains.
    pub async fn variety(&self) -> Vec<(String, u64)> {
        self.handle.variety().await
    }

    /// Get variety counter for specific domain.
    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.handle.variety_for_domain(domain).await
    }

    /// Get all algedonic alerts.
    pub async fn alerts(&self) -> Vec<AlertInfo> {
        self.handle
            .alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    /// Get critical alerts only.
    pub async fn critical_alerts(&self) -> Vec<AlertInfo> {
        self.handle
            .critical_alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    /// Get total variety deficit across all domains.
    pub async fn total_deficit(&self) -> u64 {
        self.handle.total_deficit().await
    }
}

/// CNS Governance Write Adapter — wraps `CnsGovernWriteHandle`.
///
/// Used by Curation to adjust observability policy.
/// Can set expected variety and calibrate thresholds.
/// CANNOT emit spans or reset alerts.
pub struct CnsGovernWriteAdapter {
    handle: CnsGovernWriteHandle,
}

impl CnsGovernWriteAdapter {
    /// Create a governance write adapter from a CnsRuntime reference.
    pub fn new(runtime: Arc<CnsRuntime>, governor: WebID) -> Self {
        Self {
            handle: runtime.govern_write_handle(governor),
        }
    }

    /// Create a governance write adapter from an existing CnsGovernWriteHandle.
    pub fn from_handle(handle: CnsGovernWriteHandle) -> Self {
        Self { handle }
    }

    /// The agent performing governance writes (typically Curator).
    pub fn governor(&self) -> &WebID {
        self.handle.governor()
    }

    /// Get CNS health status.
    pub async fn health(&self) -> HealthStatus {
        map_health(self.handle.health().await)
    }

    /// Get variety counters for all domains.
    pub async fn variety(&self) -> Vec<(String, u64)> {
        self.handle.variety().await
    }

    /// Get variety counter for specific domain.
    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.handle.variety_for_domain(domain).await
    }

    /// Get all algedonic alerts.
    pub async fn alerts(&self) -> Vec<AlertInfo> {
        self.handle
            .alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    /// Get critical alerts only.
    pub async fn critical_alerts(&self) -> Vec<AlertInfo> {
        self.handle
            .critical_alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    /// Get total variety deficit across all domains.
    pub async fn total_deficit(&self) -> u64 {
        self.handle.total_deficit().await
    }

    /// Check all domains and return count of alerts generated.
    /// This is the calibration method — Curation uses it to evaluate
    /// whether thresholds need adjustment.
    pub async fn check_all(&self) -> usize {
        self.handle.check_all().await
    }

    /// Calibrate the algedonic threshold for a specific domain.
    ///
    /// This is the Curation loop's ADAPT subloop (5.3): adjust observability
    /// thresholds based on system evaluation.
    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        self.handle.calibrate_threshold(domain, new_threshold).await
    }

    /// Increment variety and check thresholds.
    pub async fn increment_and_check(&self, domain: &str, state_name: &str) -> Option<AlertInfo> {
        self.handle
            .increment_and_check(domain, state_name)
            .await
            .map(map_alert)
    }
}

/// CNS Admin Adapter — wraps `CnsAdminHandle`.
///
/// Used for operational maintenance: resetting alerts, clearing old
/// alert history, and subscribing event listeners.
/// CANNOT emit spans, check variety, or calibrate thresholds.
pub struct CnsAdminAdapter {
    handle: CnsAdminHandle,
}

impl CnsAdminAdapter {
    /// Create an admin adapter from a CnsRuntime reference.
    pub fn new(runtime: Arc<CnsRuntime>, admin: WebID) -> Self {
        Self {
            handle: runtime.admin_handle(admin),
        }
    }

    /// Create an admin adapter from an existing CnsAdminHandle.
    pub fn from_handle(handle: CnsAdminHandle) -> Self {
        Self { handle }
    }

    /// The administrator this adapter is scoped to.
    pub fn admin(&self) -> &WebID {
        self.handle.admin()
    }

    /// Reset all algedonic alerts.
    pub async fn reset_alerts(&self) {
        self.handle.reset_alerts().await
    }

    /// Clear old alerts (older than specified duration).
    pub async fn clear_old_alerts(&self, max_age: std::time::Duration) {
        self.handle.clear_old_alerts(max_age).await
    }
}
