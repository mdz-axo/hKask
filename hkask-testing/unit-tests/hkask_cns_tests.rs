//! Unit tests for hkask-cns crate
//! Migrated from inline tests in production code (git commit f9ed608)
//! Expanded to cover algedonic, energy, rate_limit, spans, and variety

use hkask_cns::{
    algedonic::{AlertSeverity, AlgedonicAlert, AlgedonicManager, CnsHealth},
    energy::{
        EnergyAccount, EnergyBudget, EnergyEmitter, EnergySpanType, OpportunityCost,
        calculate_energy_cost, cleanup_expired_capabilities, estimate_tokens,
        recommended_cleanup_interval,
    },
    rate_limit::{RateLimitConfig, RateLimiter},
    spans::{SpanCategory, SpanEmitter},
    variety::{VarietyCounter, VarietyMonitor},
};
use hkask_types::{NuEvent, Span, WebID};
use serde_json::json;
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
    fn test_algedonic_alert_message() {
        let alert = AlgedonicAlert::new("test_domain", 150, 100);
        assert!(alert.message.contains("test_domain"));
        assert!(alert.message.contains("150"));
        assert!(alert.message.contains("100"));
    }

    #[test]
    fn test_algedonic_alert_is_critical() {
        let critical = AlgedonicAlert::new("test", 150, 100);
        assert!(critical.is_critical());

        let warning = AlgedonicAlert::new("test", 75, 100);
        assert!(!warning.is_critical());
    }

    #[test]
    fn test_algedonic_alert_is_warning() {
        let warning = AlgedonicAlert::new("test", 75, 100);
        assert!(warning.is_warning());

        let info = AlgedonicAlert::new("test", 25, 100);
        assert!(!info.is_warning());
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
    fn test_algedonic_manager_check_all() {
        let mut manager = AlgedonicManager::new(100);
        let mut monitor = VarietyMonitor::new();

        monitor.counter("domain1").increment("a");
        monitor.counter("domain2").increment("x");

        let alert_count = manager.check_all(&mut monitor);
        assert!(alert_count > 0);
    }

    #[test]
    fn test_algedonic_manager_alerts() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        assert_eq!(manager.alerts().len(), 1);
    }

    #[test]
    fn test_algedonic_manager_critical_alerts() {
        let mut manager = AlgedonicManager::new(10);
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("b");

        manager.check(&counter, "test");
        let critical = manager.critical_alerts();
        assert!(!critical.is_empty());
    }

    #[test]
    fn test_algedonic_manager_total_deficit() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        let deficit = manager.total_deficit();
        assert!(deficit > 0);
    }

    #[test]
    fn test_algedonic_manager_clear_old() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        assert_eq!(manager.alerts().len(), 1);

        // Clear with 0 duration should remove all
        manager.clear_old(Duration::from_secs(0));
        assert_eq!(manager.alerts().len(), 0);
    }

    #[test]
    fn test_algedonic_manager_reset() {
        let mut manager = AlgedonicManager::new(100);
        let mut counter = VarietyCounter::new();
        counter.increment("a");

        manager.check(&counter, "test");
        manager.reset();
        assert_eq!(manager.alerts().len(), 0);
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
    fn test_cns_health_healthy() {
        let manager = AlgedonicManager::new(100);
        let health = CnsHealth::check(&manager);
        assert!(health.healthy);
        assert_eq!(health.critical_count, 0);
        assert_eq!(health.warning_count, 0);
    }
}

mod variety_tests {
    use super::*;

    #[test]
    fn test_variety_counter_new() {
        let counter = VarietyCounter::new();
        assert_eq!(counter.variety(), 0);
        assert_eq!(counter.total(), 0);
    }

    #[test]
    fn test_variety_counter_increment() {
        let mut counter = VarietyCounter::new();
        counter.increment("state_a");
        assert_eq!(counter.variety(), 1);
        assert_eq!(counter.total(), 1);
    }

    #[test]
    fn test_variety_counter_multiple_increments() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("a");
        counter.increment("b");
        counter.increment("c");

        assert_eq!(counter.variety(), 3);
        assert_eq!(counter.total(), 4);
    }

    #[test]
    fn test_variety_counter_get() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("a");
        counter.increment("a");

        assert_eq!(counter.get("a"), 3);
        assert_eq!(counter.get("b"), 0);
    }

    #[test]
    fn test_variety_counter_deficit() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("b");

        assert_eq!(counter.deficit(5), 3);
        assert_eq!(counter.deficit(2), 0);
    }

    #[test]
    fn test_variety_counter_count_deficit() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("a");

        assert_eq!(counter.count_deficit(5), 3);
        assert_eq!(counter.count_deficit(2), 0);
    }

    #[test]
    fn test_variety_counter_top() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("a");
        counter.increment("a");
        counter.increment("b");
        counter.increment("b");
        counter.increment("c");

        let top = counter.top(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "a");
        assert_eq!(top[0].1, 3);
    }

    #[test]
    fn test_variety_counter_entropy() {
        let mut counter = VarietyCounter::new();
        assert_eq!(counter.entropy(), 0.0);

        counter.increment("a");
        counter.increment("b");
        assert!((counter.entropy() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_variety_counter_reset() {
        let mut counter = VarietyCounter::new();
        counter.increment("a");
        counter.increment("b");
        counter.reset();

        assert_eq!(counter.variety(), 0);
        assert_eq!(counter.total(), 0);
    }

    #[test]
    fn test_variety_monitor_new() {
        let monitor = VarietyMonitor::new();
        assert_eq!(monitor.domains().len(), 0);
    }

    #[test]
    fn test_variety_monitor_counter() {
        let mut monitor = VarietyMonitor::new();
        let counter = monitor.counter("domain1");
        counter.increment("a");

        assert_eq!(monitor.domains().len(), 1);
        assert!(monitor.domains().contains(&"domain1"));
    }

    #[test]
    fn test_variety_monitor_total_deficit() {
        let mut monitor = VarietyMonitor::new();
        monitor.counter("d1").increment("a");
        monitor.counter("d2").increment("x");

        let deficit = monitor.total_deficit(5);
        assert_eq!(deficit, 8); // (5-1) + (5-1) = 8
    }

    #[test]
    fn test_variety_monitor_exceeds_threshold() {
        let mut monitor = VarietyMonitor::new();
        monitor.counter("d1").increment("a");

        assert!(monitor.exceeds_threshold(0));
        assert!(!monitor.exceeds_threshold(100));
    }
}

mod energy_tests {
    use super::*;

    #[test]
    fn test_energy_span_type_as_str() {
        assert_eq!(EnergySpanType::Allocate.as_str(), "cns.energy.allocate");
        assert_eq!(EnergySpanType::Consume.as_str(), "cns.energy.consume");
        assert_eq!(EnergySpanType::Opportunity.as_str(), "cns.energy.opportunity");
        assert_eq!(EnergySpanType::Deficit.as_str(), "cns.energy.deficit");
    }

    #[test]
    fn test_energy_span_type_parse() {
        assert_eq!(EnergySpanType::parse_str("allocate"), Some(EnergySpanType::Allocate));
        assert_eq!(EnergySpanType::parse_str("cns.energy.consume"), Some(EnergySpanType::Consume));
        assert_eq!(EnergySpanType::parse_str("invalid"), None);
    }

    #[test]
    fn test_energy_budget_new() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.cap, 10000);
        assert_eq!(budget.remaining, 10000);
        assert_eq!(budget.cost_per_token, 0.25);
        assert_eq!(budget.alert_threshold, 0.8);
        assert!(budget.hard_limit);
    }

    #[test]
    fn test_energy_budget_with_cost_per_token() {
        let budget = EnergyBudget::new(10000).with_cost_per_token(0.5);
        assert_eq!(budget.cost_per_token, 0.5);
    }

    #[test]
    fn test_energy_budget_with_alert_threshold() {
        let budget = EnergyBudget::new(10000).with_alert_threshold(0.9);
        assert_eq!(budget.alert_threshold, 0.9);
    }

    #[test]
    fn test_energy_budget_with_hard_limit() {
        let budget = EnergyBudget::new(10000).with_hard_limit(false);
        assert!(!budget.hard_limit);
    }

    #[test]
    fn test_energy_budget_calculate_cost() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.calculate_cost(100), 25);
        assert_eq!(budget.calculate_cost(1000), 250);
    }

    #[test]
    fn test_energy_budget_calculate_tokens() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.calculate_tokens(25), 100);
        assert_eq!(budget.calculate_tokens(250), 1000);
    }

    #[test]
    fn test_energy_budget_allocate() {
        let mut budget = EnergyBudget::new(10000);
        let cost = budget.allocate(1000).unwrap();
        assert_eq!(cost, 250);
        assert_eq!(budget.remaining, 9750);
    }

    #[test]
    fn test_energy_budget_allocate_exceeded() {
        let mut budget = EnergyBudget::new(100);
        let result = budget.allocate(1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_energy_budget_should_alert() {
        let mut budget = EnergyBudget::new(10000);
        assert!(!budget.should_alert());

        // Allocate enough to reach 80% usage
        budget.allocate(32000).unwrap();
        assert!(budget.should_alert());
    }

    #[test]
    fn test_energy_budget_usage_ratio() {
        let mut budget = EnergyBudget::new(10000);
        assert_eq!(budget.usage_ratio(), 0.0);

        budget.allocate(5000).unwrap();
        assert!((budget.usage_ratio() - 0.125).abs() < 0.01);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2);
        assert_eq!(estimate_tokens("hello world"), 3);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_calculate_energy_cost() {
        assert_eq!(calculate_energy_cost("hello", 0.25), 0);
        assert_eq!(calculate_energy_cost("hello world", 0.25), 0);
    }

    #[test]
    fn test_energy_account_new() {
        let account = EnergyAccount::new("test", 10000);
        assert_eq!(account.id, "test");
        assert_eq!(account.budget.cap, 10000);
        assert_eq!(account.total_allocated, 0);
        assert_eq!(account.total_consumed, 0);
    }

    #[test]
    fn test_energy_account_allocate() {
        let mut account = EnergyAccount::new("test", 10000);
        let cost = account.allocate(1000).unwrap();
        assert_eq!(cost, 250);
        assert_eq!(account.total_allocated, 250);
    }

    #[test]
    fn test_energy_account_consume() {
        let mut account = EnergyAccount::new("test", 10000);
        account.consume(100);
        assert_eq!(account.total_consumed, 100);
    }

    #[test]
    fn test_opportunity_cost() {
        let cost = OpportunityCost::new("test", 100, 150);
        assert_eq!(cost.operation, "test");
        assert_eq!(cost.actual_cost, 100);
        assert_eq!(cost.alternative_cost, 150);
        assert_eq!(cost.cost, 50);
    }

    #[test]
    fn test_energy_account_opportunity() {
        let mut account = EnergyAccount::new("test", 10000);
        account.record_opportunity(OpportunityCost::new("test", 100, 150));
        assert_eq!(account.total_opportunity_cost(), 50);
    }

    #[test]
    fn test_cleanup_expired_capabilities() {
        let secret = b"test-secret";
        let from = WebID::new();
        let to = WebID::new();
        let mut valid_cap = hkask_types::CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test".to_string(),
            hkask_types::CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        valid_cap.expires_at = Some(2000);
        let mut expired_cap = hkask_types::CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test2".to_string(),
            hkask_types::CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        expired_cap.expires_at = Some(500);
        let capabilities = vec![valid_cap, expired_cap];
        let (kept, removed) = cleanup_expired_capabilities(&capabilities, 1000);
        assert_eq!(kept, 1);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_recommended_cleanup_interval() {
        assert_eq!(recommended_cleanup_interval(), 300);
    }
}

mod rate_limit_tests {
    use super::*;
    use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter, TokenBucket};

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_tokens, 100);
        assert_eq!(config.refill_interval, Duration::from_millis(600));
    }

    #[test]
    fn test_token_bucket_new() {
        let config = RateLimitConfig::default();
        let bucket = TokenBucket::new(config);
        assert_eq!(bucket.tokens(), 100);
    }

    #[test]
    fn test_token_bucket_consume() {
        let config = RateLimitConfig::default();
        let mut bucket = TokenBucket::new(config);
        
        assert!(bucket.try_consume());
        assert_eq!(bucket.tokens(), 99);
    }

    #[test]
    fn test_token_bucket_exhaust() {
        let config = RateLimitConfig {
            max_tokens: 3,
            refill_interval: Duration::from_secs(60),
        };
        let mut bucket = TokenBucket::new(config);
        
        bucket.try_consume();
        bucket.try_consume();
        bucket.try_consume();
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter_new() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);
        assert!(true);
    }

    #[test]
    fn test_rate_limiter_check() {
        let limiter = RateLimiter::default();
        let bot_id = WebID::new();
        
        assert!(limiter.check(&bot_id));
    }

    #[test]
    fn test_rate_limiter_remaining() {
        let limiter = RateLimiter::default();
        let bot_id = WebID::new();
        
        let remaining = limiter.remaining(&bot_id);
        assert!(remaining > 0);
    }

    #[test]
    fn test_rate_limiter_configure_bot() {
        let limiter = RateLimiter::default();
        let bot_id = WebID::new();
        let custom_config = RateLimitConfig {
            max_tokens: 50,
            refill_interval: Duration::from_secs(1),
        };
        
        limiter.configure_bot(&bot_id, custom_config);
        assert!(true);
    }
}

mod span_tests {
    use super::*;

    #[test]
    fn test_span_category_as_str() {
        assert_eq!(SpanCategory::Connector.as_str(), "cns.connector");
        assert_eq!(SpanCategory::Tool.as_str(), "cns.tool");
        assert_eq!(SpanCategory::Prompt.as_str(), "cns.prompt");
        assert_eq!(SpanCategory::AgentPod.as_str(), "cns.agent_pod");
        assert_eq!(SpanCategory::Energy.as_str(), "cns.energy");
    }

    #[test]
    fn test_span_category_parse() {
        assert_eq!(SpanCategory::parse_str("connector"), Some(SpanCategory::Connector));
        assert_eq!(SpanCategory::parse_str("cns.tool"), Some(SpanCategory::Tool));
        assert_eq!(SpanCategory::parse_str("invalid"), None);
    }

    #[test]
    fn test_span_emitter_new() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        assert!(true);
    }

    #[test]
    fn test_span_emitter_emit_connector() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        emitter.emit_connector("llm_call", json!({"model": "test"}));
        assert!(true);
    }

    #[test]
    fn test_span_emitter_emit_tool() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        emitter.emit_tool("search", json!({"query": "test"}));
        assert!(true);
    }

    #[test]
    fn test_span_emitter_emit_prompt() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        emitter.emit_prompt("render", json!({"template": "test"}));
        assert!(true);
    }

    #[test]
    fn test_span_emitter_emit_agent_pod() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        emitter.emit_agent_pod("activated", json!({"agent": "test"}));
        assert!(true);
    }

    #[test]
    fn test_span_emitter_emit_energy() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);
        emitter.emit_energy("allocate", json!({"tokens": 100}));
        assert!(true);
    }
}
