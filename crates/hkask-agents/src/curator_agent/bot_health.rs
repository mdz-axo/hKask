//! Bot Health Evaluator — Evaluates agent health by comparing gas consumption
//! against budget allocations from the CNS runtime.
//!
//! REQ: BOT-HEALTH-001 — Mock CNS state with known budget values, feed gas events,
//! verify threshold behavior.

use chrono::{DateTime, Duration, Utc};
use hkask_cns::energy::AgentEnergyStatus;
use hkask_cns::gas_report::{AgentGasSummary, GasReport};
use hkask_types::WebID;
use std::sync::Arc;

use super::bot_metrics::BotHealthStatus;

pub(crate) const EVALUATION_WINDOW: Duration = Duration::hours(1);
const HEALTHY_THRESHOLD: f64 = 0.5;
const CRITICAL_THRESHOLD: f64 = 0.9;

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub healthy_threshold: f64,
    pub critical_threshold: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            healthy_threshold: HEALTHY_THRESHOLD,
            critical_threshold: CRITICAL_THRESHOLD,
        }
    }
}

pub struct BotHealthEvaluator {
    gas_report: Arc<GasReport>,
    cns: Arc<hkask_cns::runtime::CnsRuntime>,
    thresholds: HealthThresholds,
}

impl BotHealthEvaluator {
    pub fn new(
        gas_report: Arc<GasReport>,
        cns: Arc<hkask_cns::runtime::CnsRuntime>,
        thresholds: Option<HealthThresholds>,
    ) -> Self {
        Self {
            gas_report,
            cns,
            thresholds: thresholds.unwrap_or_default(),
        }
    }

    pub async fn evaluate(
        &self,
        agent: &WebID,
        now: DateTime<Utc>,
    ) -> Result<BotHealthStatus, hkask_types::InfrastructureError> {
        let since = now - EVALUATION_WINDOW;
        let until = now;
        let summary = self.gas_report.query_by_agent(agent, since, until)?;
        let budget = self.cns.agent_gas_status(agent).await;
        self.classify_health(&summary, budget.as_ref())
    }

    pub(crate) async fn evaluate_all(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<super::metacognition::BotStatusReport>, hkask_types::InfrastructureError> {
        let since = now - EVALUATION_WINDOW;
        let until = now;
        let summaries = self.gas_report.query_all_agents(since, until)?;
        let mut reports = Vec::with_capacity(summaries.len());
        for summary in &summaries {
            let budget = self.cns.agent_gas_status(&summary.agent).await;
            let status = self.classify_health(summary, budget.as_ref())?;
            let issues = Vec::new();
            reports.push(super::metacognition::BotStatusReport {
                bot_name: format!("{}", summary.agent),
                status,
                last_report: Some(now),
                issues,
            });
        }
        reports.sort_by_key(|r| match r.status {
            BotHealthStatus::Critical => 0u8,
            BotHealthStatus::Degraded => 1u8,
            BotHealthStatus::Healthy => 2u8,
        });
        Ok(reports)
    }

    fn classify_health(
        &self,
        summary: &AgentGasSummary,
        budget: Option<&AgentEnergyStatus>,
    ) -> Result<BotHealthStatus, hkask_types::InfrastructureError> {
        Self::classify_health_static(summary, budget, &self.thresholds)
    }

    /// Static version for testing — no self reference needed.
    pub(crate) fn classify_health_static(
        summary: &AgentGasSummary,
        budget: Option<&AgentEnergyStatus>,
        thresholds: &HealthThresholds,
    ) -> Result<BotHealthStatus, hkask_types::InfrastructureError> {
        let Some(budget) = budget else {
            return Ok(BotHealthStatus::Healthy);
        };
        let cap = budget.cap.as_raw();
        if cap == 0 {
            return Ok(BotHealthStatus::Healthy);
        }
        let ratio = summary.total_consumed as f64 / cap as f64;
        if ratio >= thresholds.critical_threshold {
            Ok(BotHealthStatus::Critical)
        } else if ratio >= thresholds.healthy_threshold {
            Ok(BotHealthStatus::Degraded)
        } else {
            Ok(BotHealthStatus::Healthy)
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that classify_health correctly thresholds consumption ratios.
    /// REQ: BOT-HEALTH-001 — verify threshold behavior.
    #[test]
    fn test_classify_healthy_below_threshold() {
        let thresholds = HealthThresholds::default();
        // Agent with 30% consumption of budget → Healthy
        let summary = AgentGasSummary {
            agent: WebID::default(),
            total_reserved: 0,
            total_consumed: 30,
            total_depleted: 0,
            tools: Vec::new(),
            window_start: Utc::now(),
            window_end: Utc::now(),
        };
        let budget = AgentEnergyStatus {
            cap: hkask_cns::energy::EnergyCost::from_raw(100),
            remaining: hkask_cns::energy::EnergyCost::from_raw(70),
            reserved: hkask_cns::energy::EnergyCost::ZERO,
            available: hkask_cns::energy::EnergyCost::from_raw(70),
            usage_ratio: 0.3,
            hard_limit: true,
            alert_threshold: 0.8,
        };
        let result =
            BotHealthEvaluator::classify_health_static(&summary, Some(&budget), &thresholds);
        assert_eq!(result, Ok(BotHealthStatus::Healthy));
    }

    /// Test that classify_health at boundary between Healthy and Degraded.
    #[test]
    fn test_classify_degraded_at_threshold() {
        let thresholds = HealthThresholds::default();
        // Agent with 50% consumption → Degraded (exactly at healthy_threshold)
        let summary = AgentGasSummary {
            agent: WebID::default(),
            total_reserved: 0,
            total_consumed: 50,
            total_depleted: 0,
            tools: Vec::new(),
            window_start: Utc::now(),
            window_end: Utc::now(),
        };
        let budget = AgentEnergyStatus {
            cap: hkask_cns::energy::EnergyCost::from_raw(100),
            remaining: hkask_cns::energy::EnergyCost::from_raw(50),
            reserved: hkask_cns::energy::EnergyCost::ZERO,
            available: hkask_cns::energy::EnergyCost::from_raw(50),
            usage_ratio: 0.5,
            hard_limit: true,
            alert_threshold: 0.8,
        };
        let result =
            BotHealthEvaluator::classify_health_static(&summary, Some(&budget), &thresholds);
        assert_eq!(result, Ok(BotHealthStatus::Degraded));
    }

    /// Test that classify_health at critical threshold.
    #[test]
    fn test_classify_critical_at_threshold() {
        let thresholds = HealthThresholds::default();
        // Agent with 90% consumption → Critical
        let summary = AgentGasSummary {
            agent: WebID::default(),
            total_reserved: 0,
            total_consumed: 90,
            total_depleted: 0,
            tools: Vec::new(),
            window_start: Utc::now(),
            window_end: Utc::now(),
        };
        let budget = AgentEnergyStatus {
            cap: hkask_cns::energy::EnergyCost::from_raw(100),
            remaining: hkask_cns::energy::EnergyCost::from_raw(10),
            reserved: hkask_cns::energy::EnergyCost::ZERO,
            available: hkask_cns::energy::EnergyCost::from_raw(10),
            usage_ratio: 0.9,
            hard_limit: true,
            alert_threshold: 0.8,
        };
        let result =
            BotHealthEvaluator::classify_health_static(&summary, Some(&budget), &thresholds);
        assert_eq!(result, Ok(BotHealthStatus::Critical));
    }

    /// Test that no budget means Healthy.
    #[test]
    fn test_classify_no_budget_is_healthy() {
        let thresholds = HealthThresholds::default();
        let summary = AgentGasSummary {
            agent: WebID::default(),
            total_reserved: 0,
            total_consumed: 999,
            total_depleted: 0,
            tools: Vec::new(),
            window_start: Utc::now(),
            window_end: Utc::now(),
        };
        let result = BotHealthEvaluator::classify_health_static(&summary, None, &thresholds);
        assert_eq!(result, Ok(BotHealthStatus::Healthy));
    }

    /// Test that zero budget cap is Healthy.
    #[test]
    fn test_classify_zero_cap_is_healthy() {
        let thresholds = HealthThresholds::default();
        let summary = AgentGasSummary {
            agent: WebID::default(),
            total_reserved: 0,
            total_consumed: 100,
            total_depleted: 0,
            tools: Vec::new(),
            window_start: Utc::now(),
            window_end: Utc::now(),
        };
        let budget = AgentEnergyStatus {
            cap: hkask_cns::energy::EnergyCost::ZERO,
            remaining: hkask_cns::energy::EnergyCost::ZERO,
            reserved: hkask_cns::energy::EnergyCost::ZERO,
            available: hkask_cns::energy::EnergyCost::ZERO,
            usage_ratio: 0.0,
            hard_limit: false,
            alert_threshold: 0.8,
        };
        let result =
            BotHealthEvaluator::classify_health_static(&summary, Some(&budget), &thresholds);
        assert_eq!(result, Ok(BotHealthStatus::Healthy));
    }
}
