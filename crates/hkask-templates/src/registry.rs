//! Template registry index
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Supports Prompt (WordAct), Process (FlowDef), and Cognition (KnowAct) templates.

use crate::ports::{Action, ManifestStep};
use crate::ports::{ProcessManifest, RegistryEntry, RegistryIndex};
use hkask_types::TemplateType;
use std::collections::HashMap;

/// Template registry entry
#[derive(Debug, Clone)]
pub struct TemplateEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub name: String,
    pub description: String,
    pub lexicon_terms: Vec<String>,
    pub source_path: String,
    pub cascade_level: u32,
    pub matroshka_limit: u32,
}

impl TemplateEntry {
    pub fn new(id: &str, template_type: TemplateType, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            template_type,
            name: name.to_string(),
            description: description.to_string(),
            lexicon_terms: vec![],
            source_path: format!("registry/templates/{}.j2", id.replace('/', "_")),
            cascade_level: 0,
            matroshka_limit: 7,
        }
    }

    pub fn with_lexicon(mut self, terms: Vec<&str>) -> Self {
        self.lexicon_terms = terms.into_iter().map(String::from).collect();
        self
    }

    pub fn with_source(mut self, path: &str) -> Self {
        self.source_path = path.to_string();
        self
    }

    pub fn with_cascade(mut self, level: u32) -> Self {
        self.cascade_level = level;
        self
    }

    pub fn with_matroshka_limit(mut self, limit: u32) -> Self {
        self.matroshka_limit = limit;
        self
    }

    pub fn as_registry_entry(&self) -> RegistryEntry {
        RegistryEntry {
            id: self.id.clone(),
            template_type: self.template_type,
            lexicon_terms: self.lexicon_terms.clone(),
            description: self.description.clone(),
            source_path: self.source_path.clone(),
        }
    }
}

/// Unified template registry
#[derive(Debug)]
pub struct Registry {
    templates: HashMap<String, TemplateEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn register(&mut self, entry: TemplateEntry) {
        self.templates.insert(entry.id.clone(), entry);
    }

    pub fn get(&self, id: &str) -> Option<&TemplateEntry> {
        self.templates.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut TemplateEntry> {
        self.templates.get_mut(id)
    }

    pub fn by_type(&self, template_type: TemplateType) -> Vec<&TemplateEntry> {
        self.templates
            .values()
            .filter(|t| t.template_type == template_type)
            .collect()
    }

    pub fn exists(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    pub fn ids(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.templates.len()
    }

    /// Bootstrap registry with hLexicon core templates
    pub fn bootstrap() -> Self {
        let mut registry = Self::new();

        // Core prompt templates (WordAct - what to say)
        registry.register(
            TemplateEntry::new(
                "prompt/selector",
                TemplateType::Prompt,
                "Template Selector",
                "Selects best-fit template for input context",
            )
            .with_lexicon(vec!["recognize", "classify", "match", "discriminate"])
            .with_source("registry/templates/prompt_selector.j2"),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/render",
                TemplateType::Prompt,
                "Template Renderer",
                "Renders Jinja2 template with bound variables",
            )
            .with_lexicon(vec!["compose", "bind", "render"])
            .with_source("registry/templates/prompt_render.j2"),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/execute",
                TemplateType::Prompt,
                "Template Executor",
                "Executes rendered template via model/tool",
            )
            .with_lexicon(vec!["invoke", "dispatch", "execute"])
            .with_source("registry/templates/prompt_execute.j2"),
        );

        // Core cognition templates (KnowAct - how to think)
        registry.register(
            TemplateEntry::new(
                "cognition/detect",
                TemplateType::Cognition,
                "Drift Detection",
                "Detects cognitive drift in agent behavior",
            )
            .with_lexicon(vec!["detect", "drift", "calibrate"])
            .with_source("registry/templates/cognition_detect.j2"),
        );

        registry.register(
            TemplateEntry::new(
                "cognition/calibrate",
                TemplateType::Cognition,
                "Calibration",
                "Calibrates agent responses to baseline",
            )
            .with_lexicon(vec!["calibrate", "baseline", "adjust"])
            .with_source("registry/templates/cognition_calibrate.j2"),
        );

        // Core process templates (FlowDef - what to do)
        registry.register(
            TemplateEntry::new(
                "process/memory/recall",
                TemplateType::Process,
                "Memory Recall",
                "Recalls semantic/episodic memory triples",
            )
            .with_lexicon(vec!["recall", "retrieve", "remember"])
            .with_source("registry/templates/process_recall.j2"),
        );

        registry.register(
            TemplateEntry::new(
                "process/dispatch",
                TemplateType::Process,
                "Dispatch",
                "Dispatches tool calls via ACP/MCP",
            )
            .with_lexicon(vec!["dispatch", "route", "invoke"])
            .with_source("registry/templates/process_dispatch.j2"),
        );

        registry
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryIndex for Registry {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
        match domain_hint {
            Some(t) => self
                .by_type(t)
                .into_iter()
                .map(|e| e.as_registry_entry())
                .collect(),
            None => self
                .templates
                .values()
                .map(|e| e.as_registry_entry())
                .collect(),
        }
    }

    fn get(&self, id: &str) -> Option<RegistryEntry> {
        self.templates.get(id).map(|e| e.as_registry_entry())
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
    fn test_registry_new() {
        let registry = Registry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = Registry::new();
        let entry = TemplateEntry::new("test-1", TemplateType::Prompt, "Test", "Test template");
        registry.register(entry);

        assert_eq!(registry.count(), 1);
        assert!(registry.exists("test-1"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = Registry::new();
        let entry = TemplateEntry::new("test-1", TemplateType::Prompt, "Test", "Test template");
        registry.register(entry);

        let retrieved = registry.get("test-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test");
    }

    #[test]
    fn test_registry_by_type() {
        let mut registry = Registry::new();
        registry.register(TemplateEntry::new(
            "p1",
            TemplateType::Prompt,
            "P1",
            "Prompt 1",
        ));
        registry.register(TemplateEntry::new(
            "p2",
            TemplateType::Prompt,
            "P2",
            "Prompt 2",
        ));
        registry.register(TemplateEntry::new(
            "c1",
            TemplateType::Cognition,
            "C1",
            "Cognition 1",
        ));

        let prompts = registry.by_type(TemplateType::Prompt);
        assert_eq!(prompts.len(), 2);

        let cognitions = registry.by_type(TemplateType::Cognition);
        assert_eq!(cognitions.len(), 1);
    }

    #[test]
    fn test_registry_bootstrap() {
        let registry = Registry::bootstrap();
        assert!(registry.count() > 0);
        assert!(registry.exists("prompt/selector"));
        assert!(registry.exists("cognition/detect"));
        assert!(registry.exists("process/memory/recall"));
    }

    #[test]
    fn test_template_entry_builder() {
        let entry = TemplateEntry::new("test", TemplateType::Prompt, "Test", "Desc")
            .with_lexicon(vec!["term1", "term2"])
            .with_cascade(2)
            .with_matroshka_limit(5);

        assert_eq!(entry.lexicon_terms, vec!["term1", "term2"]);
        assert_eq!(entry.cascade_level, 2);
        assert_eq!(entry.matroshka_limit, 5);
    }

    #[test]
    fn test_registry_as_index() {
        let registry = Registry::bootstrap();
        let entries = registry.list(None);
        assert!(!entries.is_empty());

        let prompt_entries = registry.list(Some(TemplateType::Prompt));
        assert!(!prompt_entries.is_empty());

        let entry = registry.get("prompt/selector");
        assert!(entry.is_some());
    }

    #[test]
    fn test_bootstrap_manifest() {
        let registry = Registry::bootstrap();
        let manifest = registry.bootstrap_manifest().unwrap();

        assert_eq!(manifest.steps.len(), 3);
        assert_eq!(manifest.steps[0].action, Action::Select);
        assert_eq!(manifest.steps[1].action, Action::Populate);
        assert_eq!(manifest.steps[2].action, Action::Execute);
    }
}
