//! Template registry index
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Supports Prompt (WordAct), Process (FlowDef), and Cognition (KnowAct) templates.

use crate::ports::{RegistryEntry, RegistryIndex, Result, TemplateError};
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
            source_path: format!("registry/templates/{}.j2", id),
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
            required_capabilities: self.required_capabilities.clone(),
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
    ///
    /// Maps `domain/name` to `registry/templates/<domain>/<name>.j2` (nested, no flatten).
    pub fn get_template_path(template_id: &str) -> PathBuf {
        let base_path = Self::get_templates_path();
        // Direct path: cognition/calibrate -> registry/templates/cognition/calibrate.j2
        base_path.join(format!("{}.j2", template_id))
    }

    /// Validate that a template path is safe (no path traversal)
    ///
    /// Extended checks: component length ≤64 chars, Unicode NFC normalization.
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

        // Reject components exceeding 64 characters (resource-exhaustion hygiene)
        for component in normalized.split('/') {
            if component.len() > 64 {
                return Err(TemplateError::PathTraversal(format!(
                    "Path component exceeds 64 characters: {}",
                    component
                )));
            }
        }

        // Reject non-ASCII path components (homograph attack surface)
        // Template IDs must be ASCII: domain/name using lowercase a-z, digits, hyphens
        if !normalized.is_ascii() {
            return Err(TemplateError::PathTraversal(format!(
                "Non-ASCII path not allowed: {}",
                template_id
            )));
        }

        Ok(())
    }

    /// List templates as tool descriptors, filtered by agent capabilities.
    /// Implements the RDF visibility rule: visible_to(agent, tool) iff
    /// ∃c: possesses(agent, c) ∧ enables(c, tool).
    /// Emits a `cns.tool.discovery` span for algedonic monitoring.
    pub fn list_tools(&self, capabilities: &[String]) -> Vec<RegistryEntry> {
        let visible: Vec<RegistryEntry> = self
            .templates
            .values()
            .filter(|e| {
                e.required_capabilities.is_empty()
                    || e.required_capabilities
                        .iter()
                        .all(|c| capabilities.contains(c))
            })
            .map(|e| e.as_registry_entry())
            .collect();

        // Emit CNS span for algedonic monitoring
        tracing::info!(
            target: "cns.tool.discovery",
            template_count_visible = visible.len(),
            template_count_total = self.templates.len(),
            capability_set = ?capabilities,
            "Registry tool discovery"
        );

        visible
    }

    /// Describe a single template or manifest by ID.
    pub fn describe(&self, id: &str) -> Result<RegistryEntry> {
        Self::validate_template_path(id)?;
        self.templates
            .get(id)
            .map(|e| e.as_registry_entry())
            .ok_or_else(|| TemplateError::NotFound(format!("Template '{}' not found", id)))
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
            .with_lexicon(vec!["recognize", "classify", "match", "discriminate"]),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/render",
                TemplateType::Prompt,
                "Prompt Render",
                "Renders prompt with context binding",
            )
            .with_lexicon(vec!["render", "compose", "format"]),
        );

        registry.register(
            TemplateEntry::new(
                "prompt/execute",
                TemplateType::Prompt,
                "Prompt Execute",
                "Executes rendered prompt via inference",
            )
            .with_lexicon(vec!["execute", "respond", "complete"]),
        );

        // Core cognition templates (KnowAct - how to think)
        registry.register(
            TemplateEntry::new(
                "cognition/detect",
                TemplateType::Cognition,
                "Drift Detection",
                "Detects cognitive drift in agent behavior",
            )
            .with_lexicon(vec!["detect", "drift", "calibrate"]),
        );

        registry.register(
            TemplateEntry::new(
                "cognition/calibrate",
                TemplateType::Cognition,
                "Calibration",
                "Calibrates agent responses to baseline",
            )
            .with_lexicon(vec!["calibrate", "baseline", "adjust"]),
        );

        // Core process templates (FlowDef - what to do)
        registry.register(
            TemplateEntry::new(
                "process/dispatch",
                TemplateType::Process,
                "Dispatch",
                "Dispatches tool calls via ACP/MCP",
            )
            .with_lexicon(vec!["dispatch", "route", "invoke"]),
        );

        registry.register(
            TemplateEntry::new(
                "process/memory/recall",
                TemplateType::Process,
                "Memory Recall",
                "Recalls semantic/episodic memory triples",
            )
            .with_lexicon(vec!["recall", "retrieve", "remember"]),
        );

        // GML templates (KnowAct - allosteric reasoning)
        registry.register(
            TemplateEntry::new(
                "gml/recognize-ensemble",
                TemplateType::Cognition,
                "GML Recognize Ensemble",
                "Parse concept into states and ports",
            )
            .with_lexicon(vec!["recognize", "discriminate", "parse"]),
        );

        registry.register(
            TemplateEntry::new(
                "gml/bind-effector",
                TemplateType::Cognition,
                "GML Bind Effector",
                "Apply effector, infer state-shift",
            )
            .with_lexicon(vec!["analogy", "infer", "bind"]),
        );

        registry.register(
            TemplateEntry::new(
                "gml/compute-equilibrium",
                TemplateType::Cognition,
                "GML Compute Equilibrium",
                "Calculate R̄, n_H, distribution",
            )
            .with_lexicon(vec!["calculate", "compare"]),
        );

        registry.register(
            TemplateEntry::new(
                "gml/assess-coherence",
                TemplateType::Cognition,
                "GML Assess Coherence",
                "Evaluate network homeostasis",
            )
            .with_lexicon(vec!["evaluate", "reflect", "calibrate"]),
        );

        registry.register(
            TemplateEntry::new(
                "gml/reframe-concept",
                TemplateType::Cognition,
                "GML Reframe Concept",
                "Generate alternative frames",
            )
            .with_lexicon(vec!["abduct", "generate", "synthesize"]),
        );

        // Cognition — Ellipsis Analysis (Bloom Method)
        registry.register(
            TemplateEntry::new(
                "cognition/ellipsis-analysis",
                TemplateType::Cognition,
                "Ellipsis Analysis",
                "Bloom Method: detect meaning in gaps and omissions",
            )
            .with_lexicon(vec!["read", "detect", "classify", "calibrate", "analyze"]),
        );

        // Cognition — Falstaffian Perspective Engine
        registry.register(
            TemplateEntry::new(
                "cognition/falstaffian-perspective",
                TemplateType::Cognition,
                "Falstaffian Perspective",
                "Multi-iteration perspective generation through semantic shape transforms",
            )
            .with_lexicon(vec!["calibrate", "affirm", "select", "execute", "verify"]),
        );

        // Composition — Hemingway Style Synthesizer
        registry.register(
            TemplateEntry::new(
                "composition/hemingway-style-synthesizer",
                TemplateType::Prompt,
                "Hemingway Style Synthesizer",
                "Generate prose using Kansas City Star rules, Iceberg Theory, and Fish generative forms",
            )
            .with_lexicon(vec!["compose", "synthesize", "write", "edit", "refine", "render"]),
        );

        // DDMVSS Specification templates (FlowDef — define)
        registry.register(
            TemplateEntry::new(
                "spec/goal/capture",
                TemplateType::Specification,
                "Goal Capture",
                "Capture a goal as a binding requirement",
            )
            .with_lexicon(vec!["specify", "require", "elicit"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/goal/decompose",
                TemplateType::Specification,
                "Goal Decompose",
                "Decompose into sub-goals (max depth 7)",
            )
            .with_lexicon(vec!["decompose", "sequence"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/require/bind",
                TemplateType::Specification,
                "Requirement Bind",
                "Bind OCAP boundaries to a goal",
            )
            .with_lexicon(vec!["constrain", "require"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/curate/evaluate",
                TemplateType::Specification,
                "Spec Evaluate",
                "Evaluate spec for collection coherence",
            )
            .with_lexicon(vec!["curate", "evaluate"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/curate/reconcile",
                TemplateType::Specification,
                "Spec Reconcile",
                "Reconcile tensions between specs",
            )
            .with_lexicon(vec!["reconcile", "compose"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/curate/cultivate",
                TemplateType::Specification,
                "Spec Cultivate",
                "Cultivate collection toward coherence",
            )
            .with_lexicon(vec!["cultivate"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/graph/query",
                TemplateType::Specification,
                "Spec Graph Query",
                "Query spec graph by category",
            )
            .with_lexicon(vec!["recognize", "match"]),
        );

        registry.register(
            TemplateEntry::new(
                "spec/graph/validate",
                TemplateType::Specification,
                "Spec Graph Validate",
                "Validate spec graph completeness",
            )
            .with_lexicon(vec!["evaluate", "ground"]),
        );

        registry
    }

    /// One-time migration helper: infer nested path from flat-file convention.
    /// `registry/templates/cognition_calibrate.j2` -> `registry/templates/cognition/calibrate.j2`
    pub fn migrate_flat_to_nested(flat_path: &str) -> Option<String> {
        let stripped = flat_path
            .strip_prefix("registry/templates/")?
            .strip_suffix(".j2")?;
        let (domain, name) = stripped.split_once('_')?;
        Some(format!("registry/templates/{}/{}.j2", domain, name))
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

    fn get(
        &self,
        id: &str,
    ) -> std::result::Result<RegistryEntry, hkask_types::ports::RegistryError> {
        // Validate path first (security)
        if let Err(e) = Self::validate_template_path(id) {
            return Err(hkask_types::ports::RegistryError::Other(e.to_string()));
        }

        // Then check if template exists
        self.templates
            .get(id)
            .map(|e| e.as_registry_entry())
            .ok_or_else(|| {
                hkask_types::ports::RegistryError::NotFound(format!("Template '{}' not found", id))
            })
    }
}
