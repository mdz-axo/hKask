//! Template registry index
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Supports Prompt (WordAct), Process (FlowDef), and Cognition (KnowAct) templates.

use crate::ports::{Action, ManifestStep};
use crate::ports::{ProcessManifest, RegistryEntry, RegistryIndex};
use crate::ports::{Result, TemplateError};
use hkask_types::TemplateType;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

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
    /// Required capabilities for this template (R4: Capability Intersection)
    pub required_capabilities: Vec<String>,
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
            required_capabilities: vec![],
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

    /// Set required capabilities for this template (R4: Capability Intersection)
    pub fn with_required_capabilities(mut self, capabilities: Vec<&str>) -> Self {
        self.required_capabilities = capabilities.into_iter().map(String::from).collect();
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
    cache_valid: bool,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            cache_valid: true,
        }
    }

    /// Invalidate the registry cache (for hot-reload)
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
        self.templates.clear();
    }

    /// Check if cache is valid
    pub fn is_cache_valid(&self) -> bool {
        self.cache_valid
    }

    /// Reload registry from bootstrap (simulates reload from disk)
    pub fn reload(&mut self) {
        self.invalidate_cache();
        let fresh = Self::bootstrap();
        self.templates = fresh.templates;
        self.cache_valid = true;
    }

    /// Get the templates directory path
    ///
    /// Resolution order:
    /// 1. HKASK_TEMPLATES_PATH environment variable (if set)
    /// 2. <executable_dir>/registry/templates/ (default)
    /// 3. Fallback to ./registry/templates/ (development mode)
    pub fn get_templates_path() -> PathBuf {
        env::var("HKASK_TEMPLATES_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Try executable-relative path
                env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.join("registry/templates")))
                    .filter(|p| p.exists())
                    .unwrap_or_else(|| PathBuf::from("registry/templates"))
            })
    }

    /// Get full path to a template file
    pub fn get_template_path(template_id: &str) -> PathBuf {
        let base_path = Self::get_templates_path();
        // Convert template ID to filename (e.g., "prompt/selector" -> "prompt_selector.j2")
        let filename = template_id.replace('/', "_");
        base_path.join(format!("{}.j2", filename))
    }

    /// Validate that a template path is safe (no path traversal)
    pub fn validate_template_path(template_id: &str) -> Result<()> {
        // Reject absolute paths
        if template_id.starts_with('/') || template_id.starts_with('\\') {
            return Err(TemplateError::PathTraversal(format!(
                "Absolute path not allowed: {}",
                template_id
            )));
        }

        // Reject path traversal attempts
        if template_id.contains("..") {
            return Err(TemplateError::PathTraversal(format!(
                "Path traversal not allowed: {}",
                template_id
            )));
        }

        // Reject paths with null bytes
        if template_id.contains('\0') {
            return Err(TemplateError::PathTraversal(format!(
                "Null byte not allowed: {}",
                template_id
            )));
        }

        // Ensure path is normalized (no leading/trailing slashes)
        let normalized = template_id.trim_matches(|c| c == '/' || c == '\\');
        if normalized.is_empty() {
            return Err(TemplateError::PathTraversal(
                "Empty path not allowed".to_string(),
            ));
        }

        Ok(())
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
            .with_source(&Self::get_template_path("prompt/selector").to_string_lossy()),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/render",
                TemplateType::Prompt,
                "Prompt Render",
                "Renders prompt with context binding",
            )
            .with_lexicon(vec!["render", "compose", "format"])
            .with_source(&Self::get_template_path("prompt/render").to_string_lossy()),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/execute",
                TemplateType::Prompt,
                "Prompt Execute",
                "Executes rendered prompt via inference",
            )
            .with_lexicon(vec!["execute", "respond", "complete"])
            .with_source(&Self::get_template_path("prompt/execute").to_string_lossy()),
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
            .with_source(&Self::get_template_path("cognition/detect").to_string_lossy()),
        );

        registry.register(
            TemplateEntry::new(
                "cognition/calibrate",
                TemplateType::Cognition,
                "Calibration",
                "Calibrates agent responses to baseline",
            )
            .with_lexicon(vec!["calibrate", "baseline", "adjust"])
            .with_source(&Self::get_template_path("cognition/calibrate").to_string_lossy()),
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
            .with_source(&Self::get_template_path("process/memory/recall").to_string_lossy()),
        );

        registry.register(
            TemplateEntry::new(
                "process/dispatch",
                TemplateType::Process,
                "Dispatch",
                "Dispatches tool calls via ACP/MCP",
            )
            .with_lexicon(vec!["dispatch", "route", "invoke"])
            .with_source(&Self::get_template_path("process/dispatch").to_string_lossy()),
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

    fn get(&self, id: &str) -> Result<RegistryEntry> {
        // Validate path first (security)
        Self::validate_template_path(id)?;

        // Then check if template exists
        self.templates
            .get(id)
            .map(|e| e.as_registry_entry())
            .ok_or_else(|| TemplateError::NotFound(format!("Template '{}' not found", id)))
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
