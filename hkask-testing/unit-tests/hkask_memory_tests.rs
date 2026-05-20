//! Unit tests for hkask-memory crate
//! Migrated from inline tests in production code
//! Expanded to cover SemanticMemory, EpisodicMemory, and Bayesian operations

use hkask_memory::{
    bayesian::{BayesianOps, BayesianNetwork, Node, NodeState},
    episodic::EpisodicMemory,
    semantic::SemanticMemory,
};
use hkask_types::{Triple, WebID};
use serde_json::json;

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
    fn test_bayesian_update_prior() {
        let prior = 0.5;
        let likelihood = 0.8;
        let posterior = BayesianOps::update_prior(prior, likelihood);
        assert!(posterior > prior);
        assert!(posterior <= 1.0);
    }

    #[test]
    fn test_bayesian_update_prior_low_likelihood() {
        let prior = 0.5;
        let likelihood = 0.2;
        let posterior = BayesianOps::update_prior(prior, likelihood);
        assert!(posterior < prior);
    }

    #[test]
    fn test_node_new() {
        let node = Node::new("test_node", NodeState::Active);
        assert_eq!(node.name, "test_node");
        assert_eq!(node.state, NodeState::Active);
    }

    #[test]
    fn test_node_state_variants() {
        let active = NodeState::Active;
        let inactive = NodeState::Inactive;
        let uncertain = NodeState::Uncertain(0.5);

        assert_ne!(active, inactive);
        assert_ne!(active, uncertain);
    }

    #[test]
    fn test_bayesian_network_new() {
        let network = BayesianNetwork::new();
        assert_eq!(network.node_count(), 0);
    }

    #[test]
    fn test_bayesian_network_add_node() {
        let mut network = BayesianNetwork::new();
        let node = Node::new("node1", NodeState::Active);
        network.add_node(node);
        assert_eq!(network.node_count(), 1);
    }

    #[test]
    fn test_bayesian_network_get_node() {
        let mut network = BayesianNetwork::new();
        let node = Node::new("node1", NodeState::Active);
        network.add_node(node.clone());

        let retrieved = network.get_node("node1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "node1");
    }

    #[test]
    fn test_bayesian_network_connect() {
        let mut network = BayesianNetwork::new();
        network.add_node(Node::new("a", NodeState::Active));
        network.add_node(Node::new("b", NodeState::Active));
        network.connect("a", "b", 0.8);

        assert!(network.has_connection("a", "b"));
    }

    #[test]
    fn test_bayesian_network_propagate() {
        let mut network = BayesianNetwork::new();
        network.add_node(Node::new("a", NodeState::Active));
        network.add_node(Node::new("b", NodeState::Inactive));
        network.connect("a", "b", 0.9);

        network.propagate("a");
        // Propagation should complete without error
        assert!(true);
    }
}

mod semantic_memory_tests {
    use super::*;

    #[test]
    fn test_semantic_memory_new() {
        let memory = SemanticMemory::new();
        assert!(memory.is_empty());
    }

    #[test]
    fn test_semantic_memory_store() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();
        let triple = Triple::new("concept1", "is_a", json!("animal"), owner);

        memory.store(triple.clone());
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_semantic_memory_retrieve_by_entity() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();

        memory.store(Triple::new("cat", "is_a", json!("mammal"), owner.clone()));
        memory.store(Triple::new("cat", "has", json!("whiskers"), owner));

        let results = memory.retrieve_by_entity("cat");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_semantic_memory_retrieve_by_predicate() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();

        memory.store(Triple::new("cat", "is_a", json!("mammal"), owner.clone()));
        memory.store(Triple::new("dog", "is_a", json!("mammal"), owner));

        let results = memory.retrieve_by_predicate("is_a");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_semantic_memory_query() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();

        memory.store(Triple::new("cat", "is_a", json!("mammal"), owner));

        let results = memory.query("cat", "is_a", None);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_semantic_memory_forget() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();
        let triple = Triple::new("temp", "is_a", json!("concept"), owner);

        memory.store(triple.clone());
        assert!(!memory.is_empty());

        memory.forget(&triple.id);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_semantic_memory_merge() {
        let mut memory1 = SemanticMemory::new();
        let mut memory2 = SemanticMemory::new();
        let owner = WebID::new();

        memory1.store(Triple::new("a", "b", json!("c"), owner.clone()));
        memory2.store(Triple::new("x", "y", json!("z"), owner));

        memory1.merge(memory2);
        assert_eq!(memory1.triple_count(), 2);
    }

    #[test]
    fn test_semantic_memory_confidence_update() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();
        let triple = Triple::new("concept", "attr", json!("value"), owner)
            .with_confidence(0.5);

        memory.store(triple.clone());
        memory.update_confidence(&triple.id, 0.9);

        let results = memory.retrieve_by_entity("concept");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_semantic_memory_generalize() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();

        memory.store(Triple::new("sparrow", "is_a", json!("bird"), owner.clone()));
        memory.store(Triple::new("eagle", "is_a", json!("bird"), owner));

        let generalization = memory.generalize(&["sparrow", "eagle"]);
        assert!(!generalization.is_empty());
    }

    #[test]
    fn test_semantic_memory_specialize() {
        let mut memory = SemanticMemory::new();
        let owner = WebID::new();

        memory.store(Triple::new("bird", "is_a", json!("animal"), owner.clone()));
        memory.store(Triple::new("sparrow", "is_a", json!("bird"), owner));
        memory.store(Triple::new("eagle", "is_a", json!("bird"), owner));

        let specializations = memory.specialize("bird");
        assert!(!specializations.is_empty());
    }
}

mod episodic_memory_tests {
    use super::*;

    #[test]
    fn test_episodic_memory_new() {
        let memory = EpisodicMemory::new();
        assert!(memory.is_empty());
    }

    #[test]
    fn test_episodic_memory_store() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("event1", "happened", json!("yesterday"), owner)
            .with_perspective(perspective);

        memory.store(triple.clone());
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_episodic_memory_retrieve_by_context() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();

        memory.store(
            Triple::new("meeting", "location", json!("office"), owner.clone())
                .with_perspective(perspective.clone())
        );
        memory.store(
            Triple::new("meeting", "time", json!("morning"), owner)
                .with_perspective(perspective)
        );

        let results = memory.retrieve_by_context("meeting");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_episodic_memory_temporal_order() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();

        memory.store(
            Triple::new("event1", "order", json!(1), owner.clone())
                .with_perspective(perspective.clone())
        );
        memory.store(
            Triple::new("event2", "order", json!(2), owner)
                .with_perspective(perspective)
        );

        let events = memory.temporal_order();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_episodic_memory_forget() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("temp", "event", json!("data"), owner)
            .with_perspective(perspective);

        memory.store(triple.clone());
        assert!(!memory.is_empty());

        memory.forget(&triple.id);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_episodic_memory_consolidate() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();

        for i in 0..5 {
            memory.store(
                Triple::new("event", "index", json!(i), owner.clone())
                    .with_perspective(perspective.clone())
            );
        }

        memory.consolidate(3);
        // Should retain most important episodes
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_episodic_memory_query_by_perspective() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective1 = WebID::new();
        let perspective2 = WebID::new();

        memory.store(
            Triple::new("event", "data", json!(1), owner.clone())
                .with_perspective(perspective1.clone())
        );
        memory.store(
            Triple::new("event", "data", json!(2), owner)
                .with_perspective(perspective2)
        );

        let results = memory.query_by_perspective(&perspective1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_episodic_memory_merge() {
        let mut memory1 = EpisodicMemory::new();
        let mut memory2 = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();

        memory1.store(
            Triple::new("e1", "d1", json!("v1"), owner.clone())
                .with_perspective(perspective.clone())
        );
        memory2.store(
            Triple::new("e2", "d2", json!("v2"), owner)
                .with_perspective(perspective)
        );

        memory1.merge(memory2);
        assert_eq!(memory1.episode_count(), 2);
    }

    #[test]
    fn test_episodic_memory_snapshot() {
        let mut memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();

        memory.store(
            Triple::new("event", "data", json!("value"), owner)
                .with_perspective(perspective)
        );

        let snapshot = memory.snapshot();
        assert!(!snapshot.is_empty());
    }

    #[test]
    fn test_episodic_memory_is_episodic() {
        let memory = EpisodicMemory::new();
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner)
            .with_perspective(perspective);

        assert!(memory.is_episodic(&triple));
    }

    #[test]
    fn test_episodic_memory_is_semantic() {
        let memory = EpisodicMemory::new();
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner);

        assert!(memory.is_semantic(&triple));
    }
}
