// Auto-extracted inline tests for hkask-memory
// Extracted: Thu May 21 00:22:35 PDT 2026

// === From bayesian.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_high_confidence() {
        let result = BayesianOps::combine(0.9, 0.9);
        assert!(result > 0.9); // Combined should be higher
        assert!(result < 1.0);
    }

    #[test]
    fn test_combine_low_confidence() {
        let result = BayesianOps::combine(0.1, 0.1);
        assert!(result < 0.1); // Combined should be lower
    }

    #[test]
    fn test_combine_opposite() {
        let result = BayesianOps::combine(0.9, 0.1);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_retract() {
        let result = BayesianOps::retract(0.9, 0.5);
        assert!(result < 0.9);
    }

    #[test]
    fn test_join() {
        let confidences = vec![0.8, 0.7, 0.9];
        let result = BayesianOps::join(&confidences);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_decay() {
        let result = BayesianOps::decay(1.0, 0.1, 1.0);
        assert!(result < 1.0);
        assert!(result > 0.0);
    }

    #[test]
    fn test_weighted_average() {
        let confidences = vec![(0.5, 1.0), (1.0, 2.0)];
        let result = BayesianOps::weighted_average(&confidences);
        assert!((result - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_join_empty() {
        let result = BayesianOps::join(&[]);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn test_combine_extreme_values() {
        let result = BayesianOps::combine(1.0, 1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_combine_zero() {
        let result = BayesianOps::combine(0.0, 0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_decay_zero_time() {
        let result = BayesianOps::decay(1.0, 0.1, 0.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_weighted_average_empty() {
        let result = BayesianOps::weighted_average(&[]);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn test_weighted_average_zero_weights() {
        let result = BayesianOps::weighted_average(&[(0.5, 0.0), (1.0, 0.0)]);
        assert_eq!(result, 0.5);
    }
}

// === From episodic.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;
    use serde_json::json;

    fn create_test_memory() -> EpisodicMemory {
        let db = Database::in_memory().unwrap();
        EpisodicMemory::new(TripleStore::new(db.conn_arc()))
    }

    #[test]
    fn test_store_episodic() {
        let memory = create_test_memory();
        let owner = hkask_types::WebID::new();
        let perspective = hkask_types::WebID::new();
        let triple = Triple::new("event", "experienced", json!("Something happened"), owner)
            .with_perspective(perspective);

        memory.store(triple).unwrap();
    }

    #[test]
    fn test_query_for_perspective() {
        let memory = create_test_memory();
        let owner = hkask_types::WebID::new();
        let perspective1 = hkask_types::WebID::new();

        let t1 =
            Triple::new("event", "experienced", json!("E1"), owner).with_perspective(perspective1);

        memory.store(t1).unwrap();

        // Stub returns empty
        let results = memory.query_for("event", perspective1).unwrap();
        assert_eq!(results.len(), 0);
    }
}

// === From semantic.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
    use hkask_templates::SemanticMemory;
    use serde_json::json;

    fn create_test_memory() -> SemanticMemory {
        let db = Database::in_memory().unwrap();
        let conn = db.conn_arc();
        SemanticMemory::new(
            TripleStore::new(conn.clone()),
            EmbeddingStore::new(conn.clone()),
        )
    }

    #[test]
    fn test_store_and_query() {
        let memory = create_test_memory();
        let owner = hkask_types::WebID::new();
        let triple = Triple::new("concept", "definition", json!("A thing"), owner);

        // Store works
        memory.store(triple).unwrap();

        // Query returns empty (stub)
        let results = memory.query("concept").unwrap();
        assert_eq!(results.len(), 0);
    }
}
