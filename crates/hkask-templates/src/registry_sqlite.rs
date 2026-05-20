//! SQLite registry adapter
//!
//! Persistent template registry backed by SQLite.
//! Supports fast lookups, full-text search, and audit trail.

use crate::ports::{
    Action, ManifestStep, ProcessManifest, RegistryEntry, RegistryIndex, Result, TemplateError,
};
use crate::provenance::{ProvenanceManager, TemplateProvenance};
use hkask_types::TemplateType;
use rusqlite::{Connection, params};
use std::collections::HashMap;

/// SQLite-based registry index
pub struct SqliteRegistry {
    conn: Connection,
    templates: HashMap<String, RegistryEntry>,
    provenance: ProvenanceManager,
}

impl SqliteRegistry {
    /// Create new SQLite registry (in-memory or file-backed)
    pub fn new(path: Option<&str>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)
                .map_err(|e| TemplateError::Manifest(format!("Failed to open SQLite: {}", e)))?,
            None => Connection::open_in_memory().map_err(|e| {
                TemplateError::Manifest(format!("Failed to create in-memory SQLite: {}", e))
            })?,
        };

        let mut registry = Self {
            conn,
            templates: HashMap::new(),
            provenance: ProvenanceManager::new(),
        };

        // Initialize schema
        registry.init_schema()?;

        Ok(registry)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                template_type TEXT NOT NULL,
                description TEXT,
                source_path TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS lexicon_terms (
                template_id TEXT NOT NULL,
                term TEXT NOT NULL,
                PRIMARY KEY (template_id, term),
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE TABLE IF NOT EXISTS provenance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id TEXT NOT NULL,
                git_sha TEXT NOT NULL,
                modified_by TEXT NOT NULL,
                modified_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                branch TEXT,
                commit_message TEXT,
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE INDEX IF NOT EXISTS idx_templates_type ON templates(template_type);
            CREATE INDEX IF NOT EXISTS idx_lexicon_terms ON lexicon_terms(term);
            CREATE INDEX IF NOT EXISTS idx_provenance_template ON provenance(template_id);
            ",
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to init schema: {}", e)))?;

        Ok(())
    }

    /// Register a template in the registry
    pub fn register(
        &mut self,
        entry: RegistryEntry,
        provenance: Option<TemplateProvenance>,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| TemplateError::Manifest(format!("Failed to start transaction: {}", e)))?;

        // Insert template
        tx.execute(
            "INSERT OR REPLACE INTO templates (id, template_type, description, source_path, updated_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![entry.id, entry.template_type.as_str(), entry.description, entry.source_path],
        ).map_err(|e| TemplateError::Manifest(format!("Failed to insert template: {}", e)))?;

        // Delete existing lexicon terms
        tx.execute(
            "DELETE FROM lexicon_terms WHERE template_id = ?1",
            params![entry.id],
        )
        .map_err(|e| TemplateError::Manifest(format!("Failed to delete lexicon: {}", e)))?;

        // Insert lexicon terms
        for term in &entry.lexicon_terms {
            tx.execute(
                "INSERT INTO lexicon_terms (template_id, term) VALUES (?1, ?2)",
                params![entry.id, term],
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to insert lexicon: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| TemplateError::Manifest(format!("Failed to commit: {}", e)))?;

        // Record provenance
        if let Some(p) = provenance {
            self.provenance.record(p);
        }

        self.templates.insert(entry.id.clone(), entry);

        Ok(())
    }

    /// Load all templates from database into memory
    pub fn load_all(&mut self) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, template_type, description, source_path FROM templates")
            .map_err(|e| TemplateError::Manifest(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let template_type_str: String = row.get(1)?;
                let description: String = row.get(2)?;
                let source_path: String = row.get(3)?;

                let template_type =
                    TemplateType::parse_str(&template_type_str).unwrap_or(TemplateType::Prompt);

                Ok((id, template_type, description, source_path))
            })
            .map_err(|e| TemplateError::Manifest(format!("Failed to query: {}", e)))?;

        for row_result in rows {
            let (id, template_type, description, source_path) = row_result
                .map_err(|e| TemplateError::Manifest(format!("Failed to read row: {}", e)))?;

            // Load lexicon terms for this template
            let mut lexicon_stmt = self
                .conn
                .prepare("SELECT term FROM lexicon_terms WHERE template_id = ?1")
                .map_err(|e| {
                    TemplateError::Manifest(format!("Failed to prepare lexicon query: {}", e))
                })?;

            let lexicon_terms: Vec<String> = lexicon_stmt
                .query_map(params![id], |row| row.get(0))
                .map_err(|e| TemplateError::Manifest(format!("Failed to query lexicon: {}", e)))?
                .filter_map(|r| r.ok())
                .collect();

            let entry = RegistryEntry {
                id: id.clone(),
                template_type,
                lexicon_terms,
                description,
                source_path,
            };

            self.templates.insert(id, entry);
        }

        Ok(())
    }

    /// Search templates by lexicon term
    pub fn search_by_lexicon(&self, term: &str) -> Vec<RegistryEntry> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.id, t.template_type, t.description, t.source_path
             FROM templates t
             JOIN lexicon_terms l ON t.id = l.template_id
             WHERE l.term = ?1",
            )
            .unwrap(); // TODO: Handle error properly

        let rows = stmt
            .query_map(params![term], |row| {
                let id: String = row.get(0)?;
                let template_type_str: String = row.get(1)?;
                let description: String = row.get(2)?;
                let source_path: String = row.get(3)?;

                let template_type =
                    TemplateType::parse_str(&template_type_str).unwrap_or(TemplateType::Prompt);

                Ok((id, template_type, description, source_path))
            })
            .unwrap(); // TODO: Handle error properly

        let mut results = Vec::new();
        for row_result in rows {
            if let Ok((id, template_type, description, source_path)) = row_result {
                // Get lexicon terms for this template
                let mut lexicon_stmt = self
                    .conn
                    .prepare("SELECT term FROM lexicon_terms WHERE template_id = ?1")
                    .unwrap();

                let lexicon_terms: Vec<String> = lexicon_stmt
                    .query_map(params![id], |row| row.get(0))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();

                results.push(RegistryEntry {
                    id,
                    template_type,
                    lexicon_terms,
                    description,
                    source_path,
                });
            }
        }

        results
    }

    /// Get provenance for a template
    pub fn get_provenance(&self, template_id: &str) -> Option<&TemplateProvenance> {
        self.provenance.get_latest(template_id)
    }

    /// Get template count
    pub fn count(&self) -> usize {
        self.templates.len()
    }
}

impl RegistryIndex for SqliteRegistry {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
        match domain_hint {
            Some(t) => self
                .templates
                .values()
                .filter(|e| e.template_type == t)
                .cloned()
                .collect(),
            None => self.templates.values().cloned().collect(),
        }
    }

    fn get(&self, id: &str) -> Result<RegistryEntry> {
        self.templates.get(id).cloned().ok_or_else(|| {
            TemplateError::NotFound(format!("Template '{}' not found in SQLite registry", id))
        })
    }

    fn bootstrap_manifest(&self) -> Option<ProcessManifest> {
        Some(ProcessManifest {
            id: "registry/dispatch".to_string(),
            name: "Registry Dispatch".to_string(),
            description: "Bootstrap process for all registry resolution".to_string(),
            steps: vec![
                ManifestStep {
                    ordinal: 1,
                    action: Action::Select,
                    description: "Select best-fit template".to_string(),
                    template_ref: "prompt/selector".to_string(),
                    model_tier: Some("fast_local".to_string()),
                    mcp: Some("hkask-mcp-inference".to_string()),
                    renderer: Some("minijinja".to_string()),
                },
                ManifestStep {
                    ordinal: 2,
                    action: Action::Populate,
                    description: "Bind input to selected template".to_string(),
                    template_ref: "{{selected_template_id}}".to_string(),
                    model_tier: None,
                    mcp: None,
                    renderer: Some("minijinja".to_string()),
                },
                ManifestStep {
                    ordinal: 3,
                    action: Action::Execute,
                    description: "Execute template via model/tool".to_string(),
                    template_ref: "".to_string(),
                    model_tier: None,
                    mcp: Some("from_template_contract".to_string()),
                    renderer: None,
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_registry_new_in_memory() {
        let registry = SqliteRegistry::new(None).unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_sqlite_registry_register() {
        let mut registry = SqliteRegistry::new(None).unwrap();

        let entry = RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["test".to_string()],
            description: "Test template".to_string(),
            source_path: "test/template.j2".to_string(),
        };

        registry.register(entry, None).unwrap();

        assert_eq!(registry.count(), 1);
        assert!(registry.get("test/template").is_ok());
    }

    #[test]
    fn test_sqlite_registry_search_by_lexicon() {
        let mut registry = SqliteRegistry::new(None).unwrap();

        let entry1 = RegistryEntry {
            id: "prompt/selector".to_string(),
            template_type: TemplateType::Cognition,
            lexicon_terms: vec!["recognize".to_string(), "classify".to_string()],
            description: "Selector template".to_string(),
            source_path: "prompt/selector.j2".to_string(),
        };

        let entry2 = RegistryEntry {
            id: "prompt/render".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["render".to_string()],
            description: "Render template".to_string(),
            source_path: "prompt/render.j2".to_string(),
        };

        registry.register(entry1, None).unwrap();
        registry.register(entry2, None).unwrap();

        let results = registry.search_by_lexicon("recognize");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "prompt/selector");
    }

    #[test]
    fn test_sqlite_registry_list_by_type() {
        let mut registry = SqliteRegistry::new(None).unwrap();

        registry
            .register(
                RegistryEntry {
                    id: "prompt/1".to_string(),
                    template_type: TemplateType::Prompt,
                    lexicon_terms: vec![],
                    description: "Prompt 1".to_string(),
                    source_path: "prompt/1.j2".to_string(),
                },
                None,
            )
            .unwrap();

        registry
            .register(
                RegistryEntry {
                    id: "cognition/1".to_string(),
                    template_type: TemplateType::Cognition,
                    lexicon_terms: vec![],
                    description: "Cognition 1".to_string(),
                    source_path: "cognition/1.j2".to_string(),
                },
                None,
            )
            .unwrap();

        let prompts = registry.list(Some(TemplateType::Prompt));
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].id, "prompt/1");

        let cognitions = registry.list(Some(TemplateType::Cognition));
        assert_eq!(cognitions.len(), 1);
        assert_eq!(cognitions[0].id, "cognition/1");
    }
}
