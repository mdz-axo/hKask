use async_trait::async_trait;
use hkask_templates::ports::{TemplateRepository, TemplateError};
use hkask_templates::TemplateManifest;
use std::collections::HashMap;
use uuid::Uuid;

pub struct MockTemplateRepository {
    templates: HashMap<Uuid, TemplateManifest>,
}

impl MockTemplateRepository {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn with_templates(templates: Vec<TemplateManifest>) -> Self {
        Self {
            templates: templates.into_iter().map(|t| (t.id, t)).collect(),
        }
    }
}

impl Default for MockTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TemplateRepository for MockTemplateRepository {
    async fn get(&self, id: Uuid) -> Result<TemplateManifest, TemplateError> {
        self.templates
            .get(&id)
            .cloned()
            .ok_or_else(|| TemplateError::NotFound(format!("Template {} not found", id)))
    }

    async fn list(&self) -> Result<Vec<TemplateManifest>, TemplateError> {
        Ok(self.templates.values().cloned().collect())
    }

    async fn insert(&mut self, manifest: TemplateManifest) -> Result<(), TemplateError> {
        self.templates.insert(manifest.id, manifest);
        Ok(())
    }

    async fn delete(&mut self, id: Uuid) -> Result<(), TemplateError> {
        self.templates
            .remove(&id)
            .ok_or_else(|| TemplateError::NotFound(format!("Template {} not found", id)))?;
        Ok(())
    }
}
