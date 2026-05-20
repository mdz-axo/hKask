//! Unit tests for hkask-cns crate
//! Migrated from inline tests in production code (git commit f9ed608)

use hkask_cns::{
    algedonic::{AlertSeverity, AlgedonicAlert, AlgedonicManager, CnsHealth},
    variety::VarietyCounter,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod algedonic_tests {
    use super::*;

    #[test]
    fn test_algedonic_alert_severity() {
        // Info level (deficit < threshold/2)
        let alert = AlgedonicAlert::new("test", 25, 100);
        assert_eq!(alert.severity, AlertSeverity::Info);
        assert!(!alert.should_escalate());

        // Warning level (threshold/2 <= deficit < threshold)
        let alert = AlgedonicAlert::new("test", 75, 100);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.should_escalate());

        // Critical level (deficit >= threshold)
        let alert = AlgedonicAlert::new("test", 150, 100);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.should_escalate());
    }

    #[test]
    fn test_algedonic_manager_check() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();

        // Low variety - should trigger alert
        counter.increment("state_a");
        counter.increment("state_a");

        let alert = manager.check(&counter, "test_domain");
        assert!(alert.is_some());
        // Note: deficit is based on variety count, not total count
    }

    #[test]
    fn test_algedonic_manager_escalation_callback() {
        let escalation_called = Arc::new(Mutex::new(false));
        let escalation_called_clone = Arc::clone(&escalation_called);

        let mut manager = AlgedonicManager::new(1).with_escalation_callback(move |_| {
            let mut called = escalation_called_clone.lock().unwrap();
            *called = true;
        });

        let mut counter = VarietyCounter::new();
        counter.increment("state_a");
        counter.increment("state_b");

        // Variety of 2 should exceed threshold of 1
        manager.check(&counter, "test");

        let called = *escalation_called.lock().unwrap();
        assert!(called);
    }

    #[test]
    fn test_cns_health() {
        let mut manager = AlgedonicManager::new(100);

        // Add some alerts - variety of 1 with deficit against u64::MAX will be critical
        let mut counter1 = VarietyCounter::new();
        counter1.increment("a");
        manager.check(&counter1, "domain1");

        let health = CnsHealth::check(&manager);
        // With threshold 100 and variety deficit >> 100, this should be critical
        assert!(!health.healthy);
        assert!(health.critical_count > 0);
    }

    #[test]
    fn test_alert_clear_old() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        assert_eq!(manager.alerts().len(), 1);

        // Clear with 0 duration should remove all
        manager.clear_old(Duration::from_secs(0));
        assert_eq!(manager.alerts().len(), 0);
    }
}
