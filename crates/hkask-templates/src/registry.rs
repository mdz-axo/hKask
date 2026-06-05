//! Template registry index
//!
//! Unified registry with template_type discriminator per architecture v0.22.0.
//! Template types align with hKask domains:
//! - **WordAct** — Jinja2 prompt templates ("what to say")
//! - **KnowAct** — Jinja2 cognition templates ("how to think")
//! - **FlowDef** — YAML process manifests ("what to do", including specifications)
//!
//! Rust is the loom. YAML/Jinja2 is the thread.

use crate::ports::{RegistryEntry, RegistryIndex, Result, TemplateError};
use hkask_types::ports::SkillRegistryIndex;
use hkask_types::{HLexicon, SYSTEM_MAX_RECURSION, Skill, TemplateType};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Unified template + skill registry
///
/// Templates are stored as `RegistryEntry` (the canonical type from `hkask_types::ports`).
/// Skills compose templates into coherent agent capabilities.
pub struct Registry {
    templates: HashMap<String, RegistryEntry>,
    skills: HashMap<String, Skill>,
    /// Optional hLexicon for validating lexicon_terms during registration.
    /// When set, `register()` logs warnings for terms not in the canonical vocabulary.
    hlexicon: Option<HLexicon>,
    cache_valid: bool,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            skills: HashMap::new(),
            hlexicon: None,
            cache_valid: true,
        }
    }

    /// Set the hLexicon for validating terms during registration.
    pub fn set_lexicon(&mut self, lexicon: HLexicon) {
        self.hlexicon = Some(lexicon);
    }

    /// Builder-style method to set the hLexicon.
    pub fn with_lexicon(mut self, lexicon: HLexicon) -> Self {
        self.hlexicon = Some(lexicon);
        self
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
    /// Maps `domain/name` to `registry/templates/<domain>/<name>.<ext>`
    /// where `<ext>` is determined by the template type.
    pub fn get_template_path(template_id: &str, template_type: TemplateType) -> PathBuf {
        let base_path = Self::get_templates_path();
        let ext = template_type.file_extension();
        base_path.join(format!("{}.{}", template_id, ext))
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
            .cloned()
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
            .cloned()
            .ok_or_else(|| TemplateError::NotFound(format!("Template '{}' not found", id)))
    }

    /// Register a template entry, validating against the hLexicon if set.
    ///
    /// Logs warnings for lexicon terms not in the canonical vocabulary
    /// and for entries where `cascade_level >= matroshka_limit`.
    pub fn register(&mut self, entry: RegistryEntry) {
        // Validate entry consistency
        let warnings = entry.validate();
        for warning in &warnings {
            tracing::warn!(target: "hkask.templates", "Registration warning: {}", warning);
        }

        // Validate lexicon terms against the hLexicon
        if let Some(ref lexicon) = self.hlexicon {
            let unknown = lexicon.validate(&entry.lexicon_terms);
            if !unknown.is_empty() {
                tracing::warn!(
                    target: "hkask.templates",
                    entry_id = %entry.id,
                    unknown_terms = ?unknown,
                    "Lexicon terms not in canonical vocabulary"
                );
            }
        }

        self.templates.insert(entry.id.clone(), entry);
    }

    pub fn get(&self, id: &str) -> Option<&RegistryEntry> {
        self.templates.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut RegistryEntry> {
        self.templates.get_mut(id)
    }

    pub fn by_type(&self, template_type: TemplateType) -> Vec<&RegistryEntry> {
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

    // ── Skill composition methods ──────────────────────────────────

    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.values().cloned().collect()
    }

    pub fn remove_skill(&mut self, id: &str) -> Option<Skill> {
        self.skills.remove(id)
    }

    pub fn register_skill(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn get_skill(&self, id: &str) -> Option<Skill> {
        self.skills.get(id).cloned()
    }

    pub fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill> {
        self.skills
            .values()
            .filter(|s| s.domain == domain)
            .cloned()
            .collect()
    }

    /// Find skills that reference a given template ID.
    pub fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill> {
        self.skills
            .values()
            .filter(|s| {
                s.word_act.as_deref() == Some(template_id)
                    || s.flow_def.as_deref() == Some(template_id)
                    || s.know_act.as_deref() == Some(template_id)
            })
            .cloned()
            .collect()
    }

    /// Bootstrap registry with core templates aligned to hKask domains.
    ///
    /// Template types use domain-aligned names:
    /// - WordAct (Jinja2 prompts) — "what to say"
    /// - KnowAct (Jinja2 cognition) — "how to think"
    /// - FlowDef (YAML manifests) — "what to do"
    pub fn bootstrap() -> Self {
        let mut registry = Self::new();
        let max_recursion = SYSTEM_MAX_RECURSION as u32;

        // ── WordAct templates (Jinja2 prompts — "what to say") ──────

        registry.register(RegistryEntry {
            id: "wordact/selector".into(),
            template_type: TemplateType::WordAct,
            name: "Template Selector".into(),
            lexicon_terms: vec![
                "recognize".into(),
                "classify".into(),
                "match".into(),
                "discriminate".into(),
            ],
            description: "Selects best-fit template for input context".into(),
            source_path: "registry/templates/wordact/selector.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "wordact/render".into(),
            template_type: TemplateType::WordAct,
            name: "Prompt Render".into(),
            lexicon_terms: vec!["render".into(), "compose".into(), "format".into()],
            description: "Renders prompt with context binding".into(),
            source_path: "registry/templates/wordact/render.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "wordact/execute".into(),
            template_type: TemplateType::WordAct,
            name: "Prompt Execute".into(),
            lexicon_terms: vec!["execute".into(), "respond".into(), "complete".into()],
            description: "Executes rendered prompt via inference".into(),
            source_path: "registry/templates/wordact/execute.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "composition/hemingway-style-synthesizer".into(),
            template_type: TemplateType::WordAct,
            name: "Hemingway Style Synthesizer".into(),
            lexicon_terms: vec!["compose".into(), "synthesize".into(), "write".into(), "edit".into(), "refine".into(), "render".into()],
            description: "Generate prose using Kansas City Star rules, Iceberg Theory, and Fish generative forms".into(),
            source_path: "registry/templates/composition/hemingway-style-synthesizer.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        // ── KnowAct templates (Jinja2 cognition — "how to think") ───

        registry.register(RegistryEntry {
            id: "knowact/detect".into(),
            template_type: TemplateType::KnowAct,
            name: "Drift Detection".into(),
            lexicon_terms: vec!["detect".into(), "drift".into(), "calibrate".into()],
            description: "Detects cognitive drift in agent behavior".into(),
            source_path: "registry/templates/knowact/detect.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "knowact/calibrate".into(),
            template_type: TemplateType::KnowAct,
            name: "Calibration".into(),
            lexicon_terms: vec!["calibrate".into(), "baseline".into(), "adjust".into()],
            description: "Calibrates agent responses to baseline".into(),
            source_path: "registry/templates/knowact/calibrate.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "knowact/prompt-strategy".into(),
            template_type: TemplateType::KnowAct,
            name: "Prompt Strategy Selection".into(),
            lexicon_terms: vec!["classify".into(), "select".into(), "frame".into()],
            description: "Keyword-based heuristic for prompt framing".into(),
            source_path: "registry/templates/knowact/prompt-strategy.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "knowact/ellipsis-analysis".into(),
            template_type: TemplateType::KnowAct,
            name: "Ellipsis Analysis".into(),
            lexicon_terms: vec![
                "read".into(),
                "detect".into(),
                "classify".into(),
                "calibrate".into(),
                "analyze".into(),
            ],
            description: "Bloom Method: detect meaning in gaps and omissions".into(),
            source_path: "registry/templates/knowact/ellipsis-analysis.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "knowact/falstaffian-perspective".into(),
            template_type: TemplateType::KnowAct,
            name: "Falstaffian Perspective".into(),
            lexicon_terms: vec![
                "calibrate".into(),
                "affirm".into(),
                "select".into(),
                "execute".into(),
                "verify".into(),
            ],
            description: "Multi-iteration perspective generation through semantic shape transforms"
                .into(),
            source_path: "registry/templates/knowact/falstaffian-perspective.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        // GML templates (KnowAct — allosteric reasoning)
        registry.register(RegistryEntry {
            id: "gml/recognize-ensemble".into(),
            template_type: TemplateType::KnowAct,
            name: "GML Recognize Ensemble".into(),
            lexicon_terms: vec!["recognize".into(), "discriminate".into(), "parse".into()],
            description: "Parse concept into states and ports".into(),
            source_path: "registry/templates/gml/recognize-ensemble.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "gml/bind-effector".into(),
            template_type: TemplateType::KnowAct,
            name: "GML Bind Effector".into(),
            lexicon_terms: vec!["analogy".into(), "infer".into(), "bind".into()],
            description: "Apply effector, infer state-shift".into(),
            source_path: "registry/templates/gml/bind-effector.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "gml/compute-equilibrium".into(),
            template_type: TemplateType::KnowAct,
            name: "GML Compute Equilibrium".into(),
            lexicon_terms: vec!["calculate".into(), "compare".into()],
            description: "Calculate R̄, n_H, distribution".into(),
            source_path: "registry/templates/gml/compute-equilibrium.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "gml/assess-coherence".into(),
            template_type: TemplateType::KnowAct,
            name: "GML Assess Coherence".into(),
            lexicon_terms: vec!["evaluate".into(), "reflect".into(), "calibrate".into()],
            description: "Evaluate network homeostasis".into(),
            source_path: "registry/templates/gml/assess-coherence.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "gml/reframe-concept".into(),
            template_type: TemplateType::KnowAct,
            name: "GML Reframe Concept".into(),
            lexicon_terms: vec!["abduct".into(), "generate".into(), "synthesize".into()],
            description: "Generate alternative frames".into(),
            source_path: "registry/templates/gml/reframe-concept.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        // ── FlowDef templates (YAML manifests — "what to do") ──────

        registry.register(RegistryEntry {
            id: "flowdef/dispatch".into(),
            template_type: TemplateType::FlowDef,
            name: "Dispatch".into(),
            lexicon_terms: vec!["dispatch".into(), "route".into(), "invoke".into()],
            description: "Dispatches tool calls via ACP/MCP".into(),
            source_path: "registry/templates/flowdef/dispatch.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "flowdef/memory/recall".into(),
            template_type: TemplateType::FlowDef,
            name: "Memory Recall".into(),
            lexicon_terms: vec!["recall".into(), "retrieve".into(), "remember".into()],
            description: "Recalls semantic/episodic memory triples".into(),
            source_path: "registry/templates/flowdef/memory_recall.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        // DDMVSS Specification templates (FlowDef — specification manifests)

        registry.register(RegistryEntry {
            id: "spec/goal-capture".into(),
            template_type: TemplateType::FlowDef,
            name: "Goal Capture".into(),
            lexicon_terms: vec!["specify".into(), "require".into(), "elicit".into()],
            description: "Elicit user intent as binding requirement".into(),
            source_path: "registry/templates/spec/goal-capture.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "spec/constraint-bind".into(),
            template_type: TemplateType::FlowDef,
            name: "Constraint Bind".into(),
            lexicon_terms: vec!["constrain".into(), "require".into()],
            description: "Attach OCAP boundaries to goals".into(),
            source_path: "registry/templates/spec/constraint-bind.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "spec/curate-collection".into(),
            template_type: TemplateType::FlowDef,
            name: "Curate Collection".into(),
            lexicon_terms: vec!["curate".into(), "cultivate".into()],
            description: "Evaluate collection coherence and completeness".into(),
            source_path: "registry/templates/spec/curate-collection.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "spec/reconcile-conflicts".into(),
            template_type: TemplateType::FlowDef,
            name: "Reconcile Conflicts".into(),
            lexicon_terms: vec!["reconcile".into()],
            description: "Resolve goal tensions without collapsing them".into(),
            source_path: "registry/templates/spec/reconcile-conflicts.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "spec/contextualise".into(),
            template_type: TemplateType::FlowDef,
            name: "Contextualise".into(),
            lexicon_terms: vec!["contextualise".into()],
            description: "Situate artifact within meaningful environment".into(),
            source_path: "registry/templates/spec/contextualise.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry.register(RegistryEntry {
            id: "spec/selector".into(),
            template_type: TemplateType::FlowDef,
            name: "Spec Selector".into(),
            lexicon_terms: vec!["recognize".into(), "match".into()],
            description: "Route input to best-fit specification template".into(),
            source_path: "registry/templates/spec/selector.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: max_recursion,
        });

        registry
    }

    /// One-time migration helper: infer nested path from flat-file convention.
    /// `registry/templates/knowact_calibrate.j2` -> `registry/templates/knowact/calibrate.j2`
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
            Some(t) => self.by_type(t).into_iter().cloned().collect(),
            None => self.templates.values().cloned().collect(),
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
        self.templates.get(id).cloned().ok_or_else(|| {
            hkask_types::ports::RegistryError::NotFound(format!("Template '{}' not found", id))
        })
    }
}

impl SkillRegistryIndex for Registry {
    fn register_skill(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    fn get_skill(&self, id: &str) -> Option<Skill> {
        self.skills.get(id).cloned()
    }

    fn list_skills(&self) -> Vec<Skill> {
        self.skills.values().cloned().collect()
    }

    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill> {
        self.skills
            .values()
            .filter(|s| s.domain == domain)
            .cloned()
            .collect()
    }

    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill> {
        self.skills
            .values()
            .filter(|s| {
                s.word_act.as_deref() == Some(template_id)
                    || s.flow_def.as_deref() == Some(template_id)
                    || s.know_act.as_deref() == Some(template_id)
            })
            .cloned()
            .collect()
    }

    fn remove_skill(&mut self, id: &str) -> Option<Skill> {
        self.skills.remove(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_builder_pattern() {
        let skill = Skill::new("research", TemplateType::KnowAct)
            .with_word_act("wordact/research/query")
            .with_flow_def("flowdef/research/pipeline")
            .with_know_act("knowact/research/calibrate")
            .with_cascade_order(vec![
                "wordact/research/query".into(),
                "knowact/research/calibrate".into(),
            ]);

        assert_eq!(skill.id, "research");
        assert_eq!(skill.domain, TemplateType::KnowAct);
        assert_eq!(skill.word_act.as_deref(), Some("wordact/research/query"));
        assert_eq!(skill.flow_def.as_deref(), Some("flowdef/research/pipeline"));
        assert_eq!(
            skill.know_act.as_deref(),
            Some("knowact/research/calibrate")
        );
        assert_eq!(skill.cascade_order.len(), 2);
    }

    #[test]
    fn skill_minimal_fields() {
        let skill = Skill::new("minimal", TemplateType::FlowDef);
        assert!(skill.word_act.is_none());
        assert!(skill.flow_def.is_none());
        assert!(skill.know_act.is_none());
        assert!(skill.cascade_order.is_empty());
    }

    #[test]
    fn register_and_retrieve_skill() {
        let mut registry = Registry::new();
        let skill =
            Skill::new("coding", TemplateType::WordAct).with_word_act("wordact/code/generate");
        registry.register_skill(skill);

        let retrieved = registry.get_skill("coding").unwrap();
        assert_eq!(retrieved.id, "coding");
        assert_eq!(retrieved.domain, TemplateType::WordAct);
        assert_eq!(retrieved.word_act.as_deref(), Some("wordact/code/generate"));

        assert!(registry.get_skill("nonexistent").is_none());
    }

    #[test]
    fn skills_by_domain() {
        let mut registry = Registry::new();
        registry.register_skill(Skill::new("research", TemplateType::KnowAct));
        registry.register_skill(Skill::new("summarize", TemplateType::KnowAct));
        registry.register_skill(Skill::new("deploy", TemplateType::FlowDef));

        let knowledge = registry.skills_by_domain(TemplateType::KnowAct);
        assert_eq!(knowledge.len(), 2);
        assert!(knowledge.iter().all(|s| s.domain == TemplateType::KnowAct));

        let engineering = registry.skills_by_domain(TemplateType::FlowDef);
        assert_eq!(engineering.len(), 1);
        assert_eq!(engineering[0].id, "deploy");

        let empty = registry.skills_by_domain(TemplateType::WordAct);
        assert!(empty.is_empty());
    }

    #[test]
    fn skills_referencing_template() {
        let mut registry = Registry::new();
        registry.register_skill(
            Skill::new("research", TemplateType::KnowAct)
                .with_word_act("wordact/research/query")
                .with_flow_def("flowdef/research/pipeline"),
        );
        registry.register_skill(
            Skill::new("audit", TemplateType::WordAct).with_word_act("wordact/research/query"), // same template
        );
        registry.register_skill(
            Skill::new("deploy", TemplateType::FlowDef).with_know_act("knowact/deploy/verify"),
        );

        let refs = registry.skills_referencing_template("wordact/research/query");
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|s| s.id == "research"));
        assert!(refs.iter().any(|s| s.id == "audit"));

        let process_refs = registry.skills_referencing_template("flowdef/research/pipeline");
        assert_eq!(process_refs.len(), 1);
        assert_eq!(process_refs[0].id, "research");

        let no_refs = registry.skills_referencing_template("nonexistent/template");
        assert!(no_refs.is_empty());
    }

    #[test]
    fn skill_serialization_roundtrip() {
        let skill = Skill::new("research", TemplateType::KnowAct)
            .with_word_act("wordact/research/query")
            .with_cascade_order(vec!["wordact/research/query".into()]);

        let json = serde_json::to_string(&skill).unwrap();
        let deserialized: Skill = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, skill.id);
        assert_eq!(deserialized.domain, skill.domain);
        assert_eq!(deserialized.word_act, skill.word_act);
        assert_eq!(deserialized.cascade_order, skill.cascade_order);
    }

    #[test]
    fn template_type_file_extensions() {
        assert_eq!(TemplateType::WordAct.file_extension(), "j2");
        assert_eq!(TemplateType::KnowAct.file_extension(), "j2");
        assert_eq!(TemplateType::FlowDef.file_extension(), "yaml");
    }

    #[test]
    fn template_type_infer_from_extension() {
        assert_eq!(
            TemplateType::infer_from_extension("j2"),
            Some(TemplateType::KnowAct)
        );
        assert_eq!(
            TemplateType::infer_from_extension("yaml"),
            Some(TemplateType::FlowDef)
        );
        assert_eq!(
            TemplateType::infer_from_extension("yml"),
            Some(TemplateType::FlowDef)
        );
        assert_eq!(TemplateType::infer_from_extension("txt"), None);
    }

    #[test]
    fn template_type_parse_str() {
        assert_eq!(
            TemplateType::parse_str("WordAct"),
            Some(TemplateType::WordAct)
        );
        assert_eq!(
            TemplateType::parse_str("KnowAct"),
            Some(TemplateType::KnowAct)
        );
        assert_eq!(
            TemplateType::parse_str("FlowDef"),
            Some(TemplateType::FlowDef)
        );
        assert_eq!(
            TemplateType::parse_str("wordact"),
            Some(TemplateType::WordAct)
        );
        assert_eq!(
            TemplateType::parse_str("knowact"),
            Some(TemplateType::KnowAct)
        );
        assert_eq!(
            TemplateType::parse_str("flowdef"),
            Some(TemplateType::FlowDef)
        );
        assert_eq!(TemplateType::parse_str("unknown"), None);
    }

    #[test]
    fn migrate_flat_to_nested() {
        assert_eq!(
            Registry::migrate_flat_to_nested("registry/templates/knowact_calibrate.j2"),
            Some("registry/templates/knowact/calibrate.j2".to_string())
        );
        assert_eq!(
            Registry::migrate_flat_to_nested("registry/templates/wordact_selector.j2"),
            Some("registry/templates/wordact/selector.j2".to_string())
        );
        assert_eq!(
            Registry::migrate_flat_to_nested("registry/templates/flowdef_dispatch.j2"),
            Some("registry/templates/flowdef/dispatch.j2".to_string())
        );
        // Non-matching paths return None
        assert_eq!(Registry::migrate_flat_to_nested("no_prefix.j2"), None);
    }

    #[test]
    fn registry_entry_validate_clean() {
        use hkask_types::ports::RegistryEntry;

        let entry = RegistryEntry {
            id: "wordact/render".into(),
            template_type: TemplateType::WordAct,
            name: "Prompt Render".into(),
            lexicon_terms: vec!["render".into()],
            description: "Renders prompt".into(),
            source_path: "registry/templates/wordact/render.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: 7,
        };
        let warnings = entry.validate();
        assert!(
            warnings.is_empty(),
            "Expected no warnings, got: {:?}",
            warnings
        );
    }

    #[test]
    fn registry_entry_validate_exhausted_nesting() {
        use hkask_types::ports::RegistryEntry;

        let entry = RegistryEntry {
            id: "wordact/deep".into(),
            template_type: TemplateType::WordAct,
            name: "Deep Nesting".into(),
            lexicon_terms: vec![],
            description: "".into(),
            source_path: "registry/templates/wordact/deep.j2".into(),
            required_capabilities: vec![],
            cascade_level: 7,
            matroshka_limit: 7,
        };
        let warnings = entry.validate();
        assert!(
            warnings.iter().any(|w| w.contains("nesting exhausted")),
            "Expected nesting exhausted warning, got: {:?}",
            warnings
        );
    }

    #[test]
    fn registry_entry_can_nest() {
        use hkask_types::ports::RegistryEntry;

        let entry = RegistryEntry {
            id: "test".into(),
            template_type: TemplateType::WordAct,
            name: "Test".into(),
            lexicon_terms: vec![],
            description: "".into(),
            source_path: "test.j2".into(),
            required_capabilities: vec![],
            cascade_level: 3,
            matroshka_limit: 7,
        };
        assert!(entry.can_nest());

        let exhausted = RegistryEntry {
            id: "test".into(),
            template_type: TemplateType::WordAct,
            name: "Test".into(),
            lexicon_terms: vec![],
            description: "".into(),
            source_path: "test.j2".into(),
            required_capabilities: vec![],
            cascade_level: 7,
            matroshka_limit: 7,
        };
        assert!(!exhausted.can_nest());
    }

    #[test]
    fn register_with_lexicon_validation() {
        use hkask_types::HLexicon;

        let mut registry = Registry::new();
        let lexicon = HLexicon::bootstrap();
        registry.set_lexicon(lexicon);

        // Register entry with all-known terms — should succeed
        let entry = RegistryEntry {
            id: "knowact/calibrate".into(),
            template_type: TemplateType::KnowAct,
            name: "Calibration".into(),
            lexicon_terms: vec!["calibrate".into(), "reflect".into()],
            description: "Calibrates agent".into(),
            source_path: "registry/templates/knowact/calibrate.j2".into(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: 7,
        };
        registry.register(entry);
        assert!(registry.get("knowact/calibrate").is_some());
    }

    #[test]
    fn skill_list_and_remove() {
        let mut registry = Registry::new();
        registry.register_skill(Skill::new("research", TemplateType::KnowAct));
        registry.register_skill(Skill::new("deploy", TemplateType::FlowDef));

        assert_eq!(registry.list_skills().len(), 2);

        let removed = registry.remove_skill("research");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "research");
        assert!(registry.get_skill("research").is_none());
        assert_eq!(registry.list_skills().len(), 1);
    }
}
