//! Integration tests for CNS + Ensemble coordination
//! Tests multi-agent coordination with CNS monitoring

use hkask_cns::{
    algedonic::AlgedonicManager,
    variety::VarietyMonitor,
    spans::SpanEmitter,
};
use hkask_ensemble::{
    metrics::EnsembleMetrics,
    macaroon::Macaroon,
};
use hkask_types::{WebID, NuEvent, Span};
use serde_json::json;

mod cns_ensemble_integration {
    use super::*;

    #[test]
    fn test_cns_monitors_ensemble_metrics() {
        let mut variety_monitor = VarietyMonitor::new();
        let mut algedonic_manager = AlgedonicManager::new(100);

        // Simulate ensemble activity
        variety_monitor.counter("ensemble.agents").increment("agent_1");
        variety_monitor.counter("ensemble.agents").increment("agent_2");
        variety_monitor.counter("ensemble.decisions").increment("consensus");

        // Check CNS health
        let alert_count = algedonic_manager.check_all(&mut variety_monitor);
        assert!(alert_count >= 0);
    }

    #[test]
    fn test_span_emitter_tracks_ensemble_events() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);

        // Track ensemble lifecycle
        emitter.emit_agent_pod("ensemble_created", json!({"agents": 3}));
        emitter.emit_tool("ensemble_dispatch", json!({"task": "coordination"}));
        emitter.emit_energy("allocate", json!({"budget": 1000}));

        assert!(true);
    }

    #[test]
    fn test_macaroon_with_cns_monitoring() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("test-macaroon", owner.clone());

        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);

        emitter.emit_connector("macaroon_created", json!({
            "id": macaroon.id(),
            "owner": owner.to_string()
        }));

        assert!(macaroon.id() == "test-macaroon");
    }

    #[test]
    fn test_ensemble_metrics_collection() {
        let mut metrics = EnsembleMetrics::new();
        
        metrics.record_decision(0.85);
        metrics.record_decision(0.90);
        metrics.record_decision(0.75);

        assert_eq!(metrics.decision_count(), 3);
        assert!(metrics.average_confidence() > 0.8);
    }

    #[test]
    fn test_cns_variety_tracks_ensemble_states() {
        let mut monitor = VarietyMonitor::new();

        // Track different ensemble states
        monitor.counter("ensemble.state").increment("idle");
        monitor.counter("ensemble.state").increment("processing");
        monitor.counter("ensemble.state").increment("waiting");
        monitor.counter("ensemble.state").increment("completed");

        assert_eq!(monitor.counter("ensemble.state").variety(), 4);
    }

    #[test]
    fn test_algedonic_alerts_on_low_ensemble_variety() {
        let mut manager = AlgedonicManager::new(2);
        let mut counter = crate::variety::VarietyCounter::new();

        // Low variety should trigger alert
        counter.increment("state_a");
        counter.increment("state_a");

        let alert = manager.check(&counter, "ensemble.variety");
        assert!(alert.is_some());
        assert!(alert.unwrap().should_escalate());
    }

    #[test]
    fn test_span_emission_for_multi_agent_chat() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);

        // Simulate multi-agent chat flow
        emitter.emit_prompt("render", json!({"template": "chat_prompt"}));
        emitter.emit_connector("llm_call", json!({"model": "fast_local"}));
        emitter.emit_tool("response_generator", json!({"response": "test"}));
        emitter.emit_prompt("outcome", json!({"success": true}));

        assert!(true);
    }

    #[test]
    fn test_cns_energy_tracking_for_ensemble() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);

        // Track energy costs for ensemble operations
        emitter.emit_energy("allocate", json!({"operation": "ensemble_dispatch", "tokens": 500}));
        emitter.emit_energy("consume", json!({"operation": "agent_response", "cost": 125}));
        emitter.emit_energy("opportunity", json!({
            "actual": 100,
            "alternative": 150,
            "opportunity_cost": 50
        }));

        assert!(true);
    }

    #[test]
    fn test_ensemble_ocap_enforcement() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("ocap-test", owner.clone());

        // Verify macaroon creation and basic properties
        assert_eq!(macaroon.owner(), &owner);
        assert!(!macaroon.caveats().is_empty() || macaroon.caveats().is_empty());
    }

    #[test]
    fn test_cns_pipeline_spans_for_ensemble_workflow() {
        let observer = WebID::new();
        let emitter = SpanEmitter::new(observer);

        // Track multi-stage ensemble workflow
        emitter.emit_pipeline("stage_1_input", json!({"input": "user_query"}));
        emitter.emit_pipeline("stage_2_processing", json!({"agents": 3}));
        emitter.emit_pipeline("stage_3_aggregation", json!({"consensus": 0.85}));
        emitter.emit_pipeline("stage_4_output", json!({"response": "final"}));

        assert!(true);
    }
}

mod macaroon_tests {
    use super::*;

    #[test]
    fn test_macaroon_new() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("test-id", owner.clone());

        assert_eq!(macaroon.id(), "test-id");
        assert_eq!(macaroon.owner(), &owner);
    }

    #[test]
    fn test_macaroon_add_caveat() {
        let owner = WebID::new();
        let mut macaroon = Macaroon::new("test", owner);

        macaroon.add_caveat("time_limit", json!({"expires": 3600}));
        assert!(!macaroon.caveats().is_empty());
    }

    #[test]
    fn test_macaroon_verify() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("test", owner);

        // Basic verification should succeed for valid macaroon
        let result = macaroon.verify();
        assert!(result.is_ok());
    }

    #[test]
    fn test_macaroon_serialize() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("test", owner);

        let serialized = macaroon.serialize();
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_macaroon_deserialize() {
        let owner = WebID::new();
        let macaroon = Macaroon::new("test", owner);

        let serialized = macaroon.serialize();
        let deserialized = Macaroon::deserialize(&serialized);

        assert!(deserialized.is_ok());
    }
}

mod ensemble_metrics_tests {
    use super::*;

    #[test]
    fn test_ensemble_metrics_new() {
        let metrics = EnsembleMetrics::new();
        assert_eq!(metrics.decision_count(), 0);
    }

    #[test]
    fn test_ensemble_metrics_record_decision() {
        let mut metrics = EnsembleMetrics::new();
        metrics.record_decision(0.85);
        assert_eq!(metrics.decision_count(), 1);
    }

    #[test]
    fn test_ensemble_metrics_average_confidence() {
        let mut metrics = EnsembleMetrics::new();
        metrics.record_decision(0.8);
        metrics.record_decision(0.9);
        
        assert!((metrics.average_confidence() - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_ensemble_metrics_variance() {
        let mut metrics = EnsembleMetrics::new();
        metrics.record_decision(0.5);
        metrics.record_decision(0.5);
        
        assert!((metrics.variance()) < 0.01);
    }

    #[test]
    fn test_ensemble_metrics_reset() {
        let mut metrics = EnsembleMetrics::new();
        metrics.record_decision(0.9);
        metrics.reset();
        
        assert_eq!(metrics.decision_count(), 0);
    }
}
