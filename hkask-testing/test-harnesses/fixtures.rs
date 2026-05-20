use chrono::{DateTime, Utc};
use hkask_templates::{TemplateManifest, TemplateType};
use uuid::Uuid;

pub fn create_test_manifest(template_type: TemplateType) -> TemplateManifest {
    TemplateManifest {
        id: Uuid::new_v4(),
        name: format!("test-{}", template_type.as_str()),
        template_type,
        version: "0.1.0".to_string(),
        description: Some("Test manifest".to_string()),
        author: Some("test-author".to_string()),
        license: Some("MIT".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: vec!["test".to_string()],
        lexicon_terms: vec![],
        dependencies: vec![],
        contract: None,
        inference: None,
    }
}

pub fn create_test_id() -> Uuid {
    Uuid::new_v4()
}

pub fn create_test_timestamp() -> DateTime<Utc> {
    Utc::now()
}
