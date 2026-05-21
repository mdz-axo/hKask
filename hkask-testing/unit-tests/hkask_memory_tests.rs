//! Unit tests for hkask-memory crate
//! Migrated from inline tests in production code
//! Expanded to cover Bayesian operations

use hkask_memory::bayesian::BayesianOps;

mod bayesian_tests {
    use super::*;

    #[test]
    fn test_bayesian_ops_new() {
        let _ops = BayesianOps::new();
        assert!(true);
    }

    #[test]
    fn test_bayesian_combine_high_confidence() {
        let result = BayesianOps::combine(0.9, 0.9);
        assert!(result > 0.9);
        assert!(result < 1.0);
    }

    #[test]
    fn test_bayesian_combine_low_confidence() {
        let result = BayesianOps::combine(0.1, 0.1);
        assert!(result < 0.1);
    }

    #[test]
    fn test_bayesian_combine_opposite() {
        let result = BayesianOps::combine(0.9, 0.1);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_bayesian_combine_identical() {
        let result = BayesianOps::combine(0.5, 0.5);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_bayesian_combine_extreme() {
        let result = BayesianOps::combine(1.0, 0.5);
        assert!(result > 0.5);
    }

    #[test]
    fn test_bayesian_retract() {
        let result = BayesianOps::retract(0.9, 0.5);
        assert!(result < 0.9);
    }

    #[test]
    fn test_bayesian_join() {
        let confidences = vec![0.8, 0.9, 0.7];
        let result = BayesianOps::join(&confidences);
        assert!(result > 0.9);
    }

    #[test]
    fn test_bayesian_decay() {
        let result = BayesianOps::decay(1.0, 0.1, 1.0);
        assert!(result < 1.0);
        assert!(result > 0.0);
    }

    #[test]
    fn test_bayesian_weighted_average() {
        let confidences = vec![(0.8, 2.0), (0.6, 1.0)];
        let result = BayesianOps::weighted_average(&confidences);
        assert!(result > 0.6 && result < 0.8);
    }

    #[test]
    fn test_bayesian_combine_zero() {
        let result = BayesianOps::combine(0.0, 0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_bayesian_combine_one_zero() {
        let result = BayesianOps::combine(0.5, 0.0);
        // Combining with 0.0 results in 0.0 (no confidence)
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_bayesian_join_empty() {
        let confidences: Vec<f64> = vec![];
        let result = BayesianOps::join(&confidences);
        // Empty join returns default confidence
        assert_eq!(result, 0.5);
    }

    #[test]
    fn test_bayesian_join_single() {
        let confidences = vec![0.7];
        let result = BayesianOps::join(&confidences);
        assert_eq!(result, 0.7);
    }
}

mod memory_stub_tests {
    use hkask_storage::Triple;
    use hkask_types::WebID;
    use serde_json::json;

    #[test]
    fn test_triple_creation() {
        let owner = WebID::new();
        let triple = Triple::new("concept", "is_a", json!("animal"), owner);
        assert_eq!(triple.entity, "concept");
        assert_eq!(triple.attribute, "is_a");
    }

    #[test]
    fn test_triple_with_confidence() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_confidence(0.85);
        assert_eq!(triple.confidence, 0.85);
    }

    #[test]
    fn test_triple_with_visibility() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner)
            .with_visibility(hkask_types::Visibility::Public);
        assert_eq!(triple.visibility, hkask_types::Visibility::Public);
    }

    #[test]
    fn test_triple_is_semantic() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner);
        assert!(triple.is_semantic());
        assert!(!triple.is_episodic());
    }

    #[test]
    fn test_triple_is_episodic() {
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_perspective(perspective);
        assert!(triple.is_episodic());
        assert!(!triple.is_semantic());
    }

    #[test]
    fn test_triple_with_perspective() {
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_perspective(perspective.clone());
        assert_eq!(triple.perspective, Some(perspective));
    }

    #[test]
    fn test_triple_temporal_properties() {
        let owner = WebID::new();
        let triple = Triple::new("event", "time", json!("now"), owner);
        assert!(triple.valid_from <= chrono::Utc::now());
        assert!(triple.valid_to.is_none());
    }

    #[test]
    fn test_triple_confidence_default() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner);
        assert_eq!(triple.confidence, 1.0);
    }
}
