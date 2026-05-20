//! CNS Composition Observer
//!
//! Implements pattern observation for Claude Skill → hKask translation.
//! Monitors translation success rate, energy cost variance, CNS variety counters,
//! and security violations blocked.
//!
//! **Metrics:**
//! - Translation success rate
//! - Energy cost variance (source vs. target)
//! - CNS variety counters (template diversity, manifest complexity)
//! - Security violations blocked (OCAP enforcement)
//!
//! **Feedback Loop:**
//! - CNS monitors → algedonic alerts on variety deficit >100
//! - Calibration prompts → adjust translation rules
//! - Human escalation → Curator review on persistent deficits

use crate::algedonic::{AlgedonicAlert, AlertSeverity};
use crate::spans::energy::EnergyAccount;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Composition observer metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionMetrics {
    /// Total translation attempts
    pub total_attempts: u64,
    /// Successful translations
    pub successful_translations: u64,
    /// Failed translations
    pub failed_translations: u64,
    /// Energy cost variance (source vs. target)
    pub energy_cost_variance: f64,
    /// Template diversity counter
    pub template_diversity: u64,
    /// Manifest complexity counter
    pub manifest_complexity: u64,
    /// Security violations blocked
    pub security_violations_blocked: u64,
    /// Average translation time (ms)
    pub avg_translation_time_ms: f64,
}

impl CompositionMetrics {
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            successful_translations: 0,
            failed_translations: 0,
            energy_cost_variance: 0.0,
            template_diversity: 0,
            manifest_complexity: 0,
            security_violations_blocked: 0,
            avg_translation_time_ms: 0.0,
        }
    }

    /// Calculate translation success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.successful_translations as f64 / self.total_attempts as f64
        }
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }

    /// Record successful translation
    pub fn record_success(&mut self, translation_time_ms: u64) {
        self.total_attempts += 1;
        self.successful_translations += 1;
        self.update_avg_time(translation_time_ms);
    }

    /// Record failed translation
    pub fn record_failure(&mut self, translation_time_ms: u64) {
        self.total_attempts += 1;
        self.failed_translations += 1;
        self.update_avg_time(translation_time_ms);
    }

    /// Update average translation time
    fn update_avg_time(&mut self, new_time_ms: u64) {
        let total = self.total_attempts as f64;
        let prev_avg = self.avg_translation_time_ms;
        self.avg_translation_time_ms = prev_avg + ((new_time_ms as f64 - prev_avg) / total);
    }

    /// Record security violation
    pub fn record_security_violation(&mut self) {
        self.security_violations_blocked += 1;
    }

    /// Update template diversity
    pub fn update_template_diversity(&mut self, count: u64) {
        self.template_diversity = count;
    }

    /// Update manifest complexity
    pub fn update_manifest_complexity(&mut self, count: u64) {
        self.manifest_complexity = count;
    }

    /// Update energy cost variance
    pub fn update_energy_variance(&mut self, source_cost: u64, target_cost: u64) {
        let diff = (target_cost as i64 - source_cost as i64).abs() as f64;
        let avg = (source_cost as f64 + target_cost as f64) / 2.0;
        if avg > 0.0 {
            self.energy_cost_variance = diff / avg;
        }
    }
}

impl Default for CompositionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Variety counter for CNS monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarietyCounter {
    pub entity_type: String,
    pub count: u64,
    pub deficit: u64,
    pub threshold: u64,
    pub alert_triggered: bool,
}

impl VarietyCounter {
    pub fn new(entity_type: &str, threshold: u64) -> Self {
        Self {
            entity_type: entity_type.to_string(),
            count: 0,
            deficit: 0,
            threshold,
            alert_triggered: false,
        }
    }

    /// Increment counter
    pub fn increment(&mut self) {
        self.count += 1;
        self.check_deficit();
    }

    /// Check for variety deficit
    pub fn check_deficit(&mut self) {
        if self.count < self.threshold {
            self.deficit = self.threshold - self.count;
            self.alert_triggered = self.deficit > self.threshold;
        } else {
            self.deficit = 0;
            self.alert_triggered = false;
        }
    }

    /// Get variety deficit
    pub fn deficit(&self) -> u64 {
        self.deficit
    }

    /// Check if alert should be triggered
    pub fn should_alert(&self) -> bool {
        self.alert_triggered
    }
}

/// Composition observer state
#[derive(Debug, Clone)]
pub struct CompositionObserverState {
    pub metrics: CompositionMetrics,
    pub variety_counters: HashMap<String, VarietyCounter>,
    pub energy_account: EnergyAccount,
    pub algedonic_threshold: u64,
    pub total_variety_deficit: u64,
}

impl CompositionObserverState {
    pub fn new(algedonic_threshold: u64) -> Self {
        let mut variety_counters = HashMap::new();
        variety_counters.insert("template".to_string(), VarietyCounter::new("template", 100));
        variety_counters.insert("manifest".to_string(), VarietyCounter::new("manifest", 100));
        variety_counters.insert("lexicon".to_string(), VarietyCounter::new("lexicon", 100));

        Self {
            metrics: CompositionMetrics::new(),
            variety_counters,
            energy_account: EnergyAccount::new("composition", 10000),
            algedonic_threshold,
            total_variety_deficit: 0,
        }
    }

    /// Calculate total variety deficit
    pub fn calculate_total_deficit(&mut self) -> u64 {
        self.total_variety_deficit = self
            .variety_counters
            .values()
            .map(|vc| vc.deficit())
            .sum();
        self.total_variety_deficit
    }

    /// Check if algedonic alert should be triggered
    pub fn should_trigger_algedonic(&self) -> bool {
        self.total_variety_deficit > self.algedonic_threshold
    }
}

/// Composition observer with cybernetic feedback
pub struct CompositionObserver {
    observer_webid: WebID,
    state: Arc<RwLock<CompositionObserverState>>,
}

impl CompositionObserver {
    pub fn new(observer_webid: WebID, algedonic_threshold: u64) -> Self {
        Self {
            observer_webid,
            state: Arc::new(RwLock::new(CompositionObserverState::new(algedonic_threshold))),
        }
    }

    /// Get observer state
    pub async fn state(&self) -> CompositionObserverState {
        self.state.read().await.clone()
    }

    /// Record translation success
    pub async fn record_success(&self, translation_time_ms: u64) {
        let mut state = self.state.write().await;
        state.metrics.record_success(translation_time_ms);
        if let Some(counter) = state.variety_counters.get_mut("template") {
            counter.increment();
        }
        state.calculate_total_deficit();
    }

    /// Record translation failure
    pub async fn record_failure(&self, translation_time_ms: u64, reason: &str) {
        let mut state = self.state.write().await;
        state.metrics.record_failure(translation_time_ms);
        tracing::warn!(target: "cns.composition", reason = reason, "Translation failed");
        state.calculate_total_deficit();
    }

    /// Record security violation
    pub async fn record_security_violation(&self, violation_type: &str) {
        let mut state = self.state.write().await;
        state.metrics.record_security_violation();
        tracing::warn!(target: "cns.composition", violation_type = violation_type, "Security violation blocked");
    }

    /// Update energy cost variance
    pub async fn update_energy_variance(&self, source_cost: u64, target_cost: u64) {
        let mut state = self.state.write().await;
        state.metrics.update_energy_variance(source_cost, target_cost);
    }

    /// Update template diversity
    pub async fn update_template_diversity(&self, count: u64) {
        let mut state = self.state.write().await;
        state.metrics.update_template_diversity(count);
        if let Some(counter) = state.variety_counters.get_mut("template") {
            counter.count = count;
            counter.check_deficit();
        }
        state.calculate_total_deficit();
    }

    /// Update manifest complexity
    pub async fn update_manifest_complexity(&self, count: u64) {
        let mut state = self.state.write().await;
        state.metrics.update_manifest_complexity(count);
        if let Some(counter) = state.variety_counters.get_mut("manifest") {
            counter.count = count;
            counter.check_deficit();
        }
        state.calculate_total_deficit();
    }

    /// Check and generate algedonic alert if needed
    pub async fn check_algedonic(&self) -> Option<AlgedonicAlert> {
        let mut state = self.state.write().await;
        let total_deficit = state.calculate_total_deficit();

        if state.should_trigger_algedonic() {
            let severity = if total_deficit > state.algedonic_threshold * 2 {
                AlertSeverity::Critical
            } else if total_deficit > state.algedonic_threshold * 3 / 2 {
                AlertSeverity::High
            } else {
                AlertSeverity::Medium
            };

            let alert = AlgedonicAlert::new(
                "composition_variety_deficit",
                severity,
                &format!(
                    "Variety deficit {} exceeds threshold {}",
                    total_deficit, state.algedonic_threshold
                ),
                "Curator",
            );

            tracing::error!(target: "cns.composition", alert = ?alert, "Algedonic alert triggered");
            Some(alert)
        } else {
            None
        }
    }

    /// Get translation success rate
    pub async fn success_rate(&self) -> f64 {
        self.state.read().await.metrics.success_rate()
    }

    /// Get energy cost variance
    pub async fn energy_variance(&self) -> f64 {
        self.state.read().await.metrics.energy_cost_variance
    }

    /// Get total variety deficit
    pub async fn total_variety_deficit(&self) -> u64 {
        self.state.read().await.total_variety_deficit
    }

    /// Generate calibration prompt based on metrics
    pub async fn generate_calibration_prompt(&self) -> String {
        let state = self.state.read().await;
        let metrics = &state.metrics;

        let mut prompt = String::from("Composition calibration recommendations:\n\n");

        if metrics.success_rate() < 0.8 {
            prompt.push_str("- Translation success rate is below 80%. Review translation rules.\n");
        }

        if metrics.energy_cost_variance > 0.5 {
            prompt.push_str("- Energy cost variance is high (>50%). Optimize template efficiency.\n");
        }

        if state.total_variety_deficit > 0 {
            prompt.push_str(&format!(
                "- Variety deficit detected ({}). Increase template/manifest diversity.\n",
                state.total_variety_deficit
            ));
        }

        if metrics.security_violations_blocked > 0 {
            prompt.push_str(&format!(
                "- {} security violations blocked. Review OCAP configuration.\n",
                metrics.security_violations_blocked
            ));
        }

        if prompt.len() == "Composition calibration recommendations:\n\n".len() {
            prompt.push_str("No calibration needed. System operating within parameters.");
        }

        prompt
    }
}

impl Clone for CompositionObserver {
    fn clone(&self) -> Self {
        Self {
            observer_webid: self.observer_webid,
            state: Arc::clone(&self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_composition_metrics_new() {
        let metrics = CompositionMetrics::new();
        assert_eq!(metrics.total_attempts, 0);
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_composition_metrics_success_rate() {
        let mut metrics = CompositionMetrics::new();
        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_failure(150);

        assert_eq!(metrics.total_attempts, 3);
        assert_eq!(metrics.successful_translations, 2);
        assert_eq!(metrics.failed_translations, 1);
        assert!((metrics.success_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_variety_counter() {
        let mut counter = VarietyCounter::new("template", 100);
        assert_eq!(counter.count, 0);
        assert_eq!(counter.deficit(), 100);

        for _ in 0..50 {
            counter.increment();
        }
        assert_eq!(counter.count, 50);
        assert_eq!(counter.deficit(), 50);

        for _ in 0..60 {
            counter.increment();
        }
        assert_eq!(counter.count, 110);
        assert_eq!(counter.deficit(), 0);
        assert!(!counter.should_alert());
    }

    #[tokio::test]
    async fn test_composition_observer_state() {
        let mut state = CompositionObserverState::new(100);
        assert_eq!(state.variety_counters.len(), 3);
        assert_eq!(state.algedonic_threshold, 100);
    }

    #[tokio::test]
    async fn test_composition_observer() {
        let observer = CompositionObserver::new(WebID::new(), 100);

        observer.record_success(100).await;
        observer.record_success(200).await;
        observer.record_failure(150, "test error").await;

        assert!((observer.success_rate().await - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_algedonic_trigger() {
        let observer = CompositionObserver::new(WebID::new(), 100);

        // Manually set high variety deficit
        {
            let mut state = observer.state.write().await;
            if let Some(counter) = state.variety_counters.get_mut("template") {
                counter.deficit = 150;
                counter.alert_triggered = true;
            }
            state.total_variety_deficit = 150;
        }

        let alert = observer.check_algedonic().await;
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, AlertSeverity::Medium);
    }

    #[tokio::test]
    async fn test_calibration_prompt() {
        let observer = CompositionObserver::new(WebID::new(), 100);
        let prompt = observer.generate_calibration_prompt().await;
        assert!(prompt.contains("Composition calibration recommendations"));
    }

    #[tokio::test]
    async fn test_energy_variance() {
        let observer = CompositionObserver::new(WebID::new(), 100);
        observer.update_energy_variance(1000, 1200).await;
        assert!((observer.energy_variance().await - 0.181).abs() < 0.01);
    }
}