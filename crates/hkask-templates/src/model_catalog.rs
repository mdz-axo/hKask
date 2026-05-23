//! Model Catalog Seeder — Pre-populate registry with approved open models
//!
//! Seeds the model registry with initial model catalog on first run.

use hkask_storage::{ModelCategory, ModelEntry, ModelRegistryStore, ModelStatus};
use tracing::info;

/// Seed the model registry with initial catalog
pub fn seed_model_catalog(registry: &ModelRegistryStore) -> Result<(), Box<dyn std::error::Error>> {
    let models = vec![
        // Instruct models
        ModelEntry {
            id: "ollama/llama-3.1-8b-instruct".to_string(),
            name: "Llama 3.1 8B Instruct".to_string(),
            category: ModelCategory::Instruct,
            provider: "ollama".to_string(),
            context_length: 8192,
            tokens_per_second: Some(50.0),
            energy_per_token: Some(0.001),
            capabilities: vec![
                "general".to_string(),
                "code".to_string(),
                "math".to_string(),
            ],
            recommended_for: vec!["prompt".to_string(), "instruct".to_string()],
            status: ModelStatus::Active,
        },
        ModelEntry {
            id: "ollama/mistral-7b-instruct".to_string(),
            name: "Mistral 7B Instruct".to_string(),
            category: ModelCategory::Instruct,
            provider: "ollama".to_string(),
            context_length: 8192,
            tokens_per_second: Some(60.0),
            energy_per_token: Some(0.0008),
            capabilities: vec!["general".to_string(), "fast".to_string()],
            recommended_for: vec!["prompt".to_string(), "fast".to_string()],
            status: ModelStatus::Active,
        },
        // Thinking models
        ModelEntry {
            id: "ollama/llama-3.1-70b-instruct".to_string(),
            name: "Llama 3.1 70B Instruct".to_string(),
            category: ModelCategory::Thinking,
            provider: "ollama".to_string(),
            context_length: 131072,
            tokens_per_second: Some(15.0),
            energy_per_token: Some(0.008),
            capabilities: vec![
                "reasoning".to_string(),
                "analysis".to_string(),
                "strategy".to_string(),
                "pattern_recognition".to_string(),
            ],
            recommended_for: vec!["analysis".to_string(), "thinking".to_string()],
            status: ModelStatus::Active,
        },
        // Categorization models
        ModelEntry {
            id: "ollama/llama-3.1-8b-categorization".to_string(),
            name: "Llama 3.1 8B Categorization".to_string(),
            category: ModelCategory::Categorization,
            provider: "ollama".to_string(),
            context_length: 8192,
            tokens_per_second: Some(80.0),
            energy_per_token: Some(0.0006),
            capabilities: vec!["classification".to_string(), "ranking".to_string()],
            recommended_for: vec!["categorization".to_string(), "ranking".to_string()],
            status: ModelStatus::Active,
        },
        // Embedding models
        ModelEntry {
            id: "ollama/nomic-embed-text".to_string(),
            name: "Nomic Embed Text".to_string(),
            category: ModelCategory::Embedding,
            provider: "ollama".to_string(),
            context_length: 8192,
            tokens_per_second: Some(100.0),
            energy_per_token: Some(0.0005),
            capabilities: vec!["embedding".to_string(), "semantic_search".to_string()],
            recommended_for: vec!["memory".to_string(), "embedding".to_string()],
            status: ModelStatus::Active,
        },
        ModelEntry {
            id: "ollama/all-minilm-l6-v2".to_string(),
            name: "all-MiniLM-L6-v2".to_string(),
            category: ModelCategory::Embedding,
            provider: "ollama".to_string(),
            context_length: 384,
            tokens_per_second: Some(150.0),
            energy_per_token: Some(0.0003),
            capabilities: vec!["embedding".to_string(), "fast".to_string()],
            recommended_for: vec!["memory".to_string(), "fast".to_string()],
            status: ModelStatus::Active,
        },
        // Specialist models
        ModelEntry {
            id: "ollama/codellama-7b".to_string(),
            name: "CodeLlama 7B".to_string(),
            category: ModelCategory::Specialist,
            provider: "ollama".to_string(),
            context_length: 16384,
            tokens_per_second: Some(45.0),
            energy_per_token: Some(0.001),
            capabilities: vec!["code".to_string(), "generation".to_string()],
            recommended_for: vec!["code".to_string(), "programming".to_string()],
            status: ModelStatus::Active,
        },
    ];

    for model in &models {
        registry.register(model)?;
        info!(
            target: "hkask.model_catalog",
            model_id = %model.id,
            category = %model.category.as_str(),
            "Registered model"
        );
    }

    info!(
        target: "hkask.model_catalog",
        count = models.len(),
        "Model catalog seeded"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::ModelRegistryStore;
    use rusqlite::Connection;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_registry() -> (ModelRegistryStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_models.db");
        let conn = Arc::new(Connection::open(db_path).unwrap());
        let registry = ModelRegistryStore::new(conn).unwrap();
        (registry, tmp)
    }

    #[test]
    fn test_seed_model_catalog() {
        let (registry, _tmp) = create_test_registry();
        seed_model_catalog(&registry).unwrap();

        // Verify models were registered
        let all_models = registry.list_all().unwrap();
        assert_eq!(all_models.len(), 7);

        // Verify categories
        let instruct_models = registry.list_by_category(&ModelCategory::Instruct).unwrap();
        assert_eq!(instruct_models.len(), 2);

        let thinking_models = registry.list_by_category(&ModelCategory::Thinking).unwrap();
        assert_eq!(thinking_models.len(), 1);

        let embedding_models = registry
            .list_by_category(&ModelCategory::Embedding)
            .unwrap();
        assert_eq!(embedding_models.len(), 2);
    }
}
