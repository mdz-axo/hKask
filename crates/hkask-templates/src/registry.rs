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
use hkask_types::ports::{BundleRegistryIndex, SkillRegistryIndex};
use hkask_types::{HLexicon, SYSTEM_MAX_RECURSION, Skill, TemplateType};
use std::collections::HashMap;

/// Unified template + skill registry
///
/// Thin in-memory wrapper (read-through cache) around `SqliteRegistry`.
/// Not a separate API surface — both `Registry` and `SqliteRegistry` implement
/// the same three index traits (`RegistryIndex`, `SkillRegistryIndex`,
/// `BundleRegistryIndex`). `Registry` loads from the filesystem on startup
/// and caches entries in HashMaps; `SqliteRegistry` provides the persistent
/// backing store. The two are always used in tandem: `Registry` for fast
/// lookups, `SqliteRegistry` for durability.
///
/// Templates are stored as `RegistryEntry` (the canonical type from `hkask_types::ports`).
/// Skills compose templates into coherent agent capabilities.
/// Bundles compose multiple skills into orchestrated process flows.
pub struct Registry {
    templates: HashMap<String, RegistryEntry>,
    skills: HashMap<String, Skill>,
    /// Bundle manifests — composed skill bundles
    bundles: HashMap<String, hkask_types::BundleManifest>,
    /// Optional hLexicon for validating lexicon_terms during registration.
    /// When set, `register()` logs warnings for terms not in the canonical vocabulary.
    hlexicon: Option<HLexicon>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            skills: HashMap::new(),
            bundles: HashMap::new(),
            hlexicon: None,
        }
    }

    /// Set the hLexicon for validating terms during registration.
    pub fn set_lexicon(&mut self, lexicon: HLexicon) {
        self.hlexicon = Some(lexicon);
    }

    /// Invalidate the registry cache (for hot-reload)
    pub(crate) fn invalidate_cache(&mut self) {
        self.templates.clear();
    }

    /// Reload registry from bootstrap (simulates reload from disk)
    pub fn reload(&mut self) {
        self.invalidate_cache();
        let fresh = Self::bootstrap();
        self.templates = fresh.templates;
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

    pub(crate) fn by_type(&self, template_type: TemplateType) -> Vec<&RegistryEntry> {
        self.templates
            .values()
            .filter(|t| t.template_type == template_type)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.templates.len()
    }

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

    /// Register a bundle manifest.
    pub fn register_bundle(&mut self, bundle: hkask_types::BundleManifest) {
        self.bundles.insert(bundle.id.clone(), bundle);
    }

    /// Retrieve a bundle manifest by ID.
    pub fn get_bundle(&self, id: &str) -> Option<&hkask_types::BundleManifest> {
        self.bundles.get(id)
    }

    /// List all bundle manifests.
    pub fn list_bundles(&self) -> Vec<&hkask_types::BundleManifest> {
        self.bundles.values().collect()
    }

    /// Remove a bundle manifest by ID.
    pub fn remove_bundle(&mut self, id: &str) -> Option<hkask_types::BundleManifest> {
        self.bundles.remove(id)
    }

    /// Find an existing bundle that contains exactly the given set of skills.
    /// Returns the first exact match, if any.
    pub fn find_bundle_by_skills(
        &self,
        skill_ids: &[String],
    ) -> Option<&hkask_types::BundleManifest> {
        let target: std::collections::HashSet<&str> =
            skill_ids.iter().map(|s| s.as_str()).collect();
        self.bundles.values().find(|b| {
            let bundle_skills: std::collections::HashSet<&str> =
                b.skills.iter().map(|s| s.id.as_str()).collect();
            bundle_skills == target
        })
    }

    /// Data-driven template definitions for [`bootstrap()`].
    ///
    /// Each tuple is: `(id, template_type, name, lexicon_terms, description, source_path)`.
    /// `required_capabilities` is always `[]`, `cascade_level` is always `0`,
    /// and `matroshka_limit` is set to `SYSTEM_MAX_RECURSION` at runtime.
    #[allow(clippy::type_complexity)]
    const BOOTSTRAP_TEMPLATES: &[(&str, TemplateType, &str, &[&str], &str, &str)] = &[
        (
            "wordact/selector",
            TemplateType::WordAct,
            "Template Selector",
            &["recognize", "classify", "match", "discriminate"],
            "Selects best-fit template for input context",
            "registry/templates/wordact/selector.j2",
        ),
        (
            "wordact/render",
            TemplateType::WordAct,
            "Prompt Render",
            &["render", "compose", "format"],
            "Renders prompt with context binding",
            "registry/templates/wordact/render.j2",
        ),
        (
            "wordact/execute",
            TemplateType::WordAct,
            "Prompt Execute",
            &["execute", "respond", "complete"],
            "Executes rendered prompt via inference",
            "registry/templates/wordact/execute.j2",
        ),
        (
            "composition/hemingway-style-synthesizer",
            TemplateType::WordAct,
            "Hemingway Style Synthesizer",
            &["compose", "synthesize", "write", "edit", "refine", "render"],
            "Generate prose using Kansas City Star rules, Iceberg Theory, Fish generative forms, and embedding-based exemplar retrieval",
            "registry/templates/composition/hemingway-style-synthesizer.j2",
        ),
        (
            "knowact/detect",
            TemplateType::KnowAct,
            "Drift Detection",
            &["detect", "drift", "calibrate"],
            "Detects cognitive drift in agent behavior",
            "registry/templates/knowact/detect.j2",
        ),
        (
            "knowact/calibrate",
            TemplateType::KnowAct,
            "Calibration",
            &["calibrate", "baseline", "adjust"],
            "Calibrates agent responses to baseline",
            "registry/templates/knowact/calibrate.j2",
        ),
        (
            "knowact/prompt-strategy",
            TemplateType::KnowAct,
            "Prompt Strategy Selection",
            &["classify", "select", "frame"],
            "Keyword-based heuristic for prompt framing",
            "registry/templates/knowact/prompt-strategy.j2",
        ),
        (
            "knowact/ellipsis-analysis",
            TemplateType::KnowAct,
            "Ellipsis Analysis",
            &["read", "detect", "classify", "calibrate", "analyze"],
            "Bloom Method: detect meaning in gaps and omissions",
            "registry/templates/knowact/ellipsis-analysis.j2",
        ),
        (
            "knowact/falstaffian-perspective",
            TemplateType::KnowAct,
            "Falstaffian Perspective",
            &["calibrate", "affirm", "select", "execute", "verify"],
            "Multi-iteration perspective generation through semantic shape transforms",
            "registry/templates/knowact/falstaffian-perspective.j2",
        ),
        // GML templates (KnowAct — allosteric reasoning)
        (
            "gml/recognize-ensemble",
            TemplateType::KnowAct,
            "GML Recognize Ensemble",
            &["recognize", "discriminate", "parse"],
            "Parse concept into states and ports",
            "registry/templates/gml/recognize-ensemble.j2",
        ),
        (
            "gml/bind-effector",
            TemplateType::KnowAct,
            "GML Bind Effector",
            &["analogy", "infer", "bind"],
            "Apply effector, infer state-shift",
            "registry/templates/gml/bind-effector.j2",
        ),
        (
            "gml/compute-equilibrium",
            TemplateType::KnowAct,
            "GML Compute Equilibrium",
            &["calculate", "compare"],
            "Calculate R\u{0304}, n_H, distribution",
            "registry/templates/gml/compute-equilibrium.j2",
        ),
        (
            "gml/assess-coherence",
            TemplateType::KnowAct,
            "GML Assess Coherence",
            &["evaluate", "reflect", "calibrate"],
            "Evaluate network homeostasis",
            "registry/templates/gml/assess-coherence.j2",
        ),
        (
            "gml/reframe-concept",
            TemplateType::KnowAct,
            "GML Reframe Concept",
            &["abduct", "generate", "synthesize"],
            "Generate alternative frames",
            "registry/templates/gml/reframe-concept.j2",
        ),
        // ── Coding Guidelines templates (KnowAct — Karpathy behavioral guardrails) ──
        (
            "coding-guidelines/guidelines-assess",
            TemplateType::KnowAct,
            "Coding Guidelines Assess",
            &["assess", "orient", "detect", "classify", "discriminate"],
            "Assess a coding task against four behavioral principles before implementation",
            "registry/templates/coding-guidelines/guidelines-assess.j2",
        ),
        (
            "coding-guidelines/guidelines-apply",
            TemplateType::KnowAct,
            "Coding Guidelines Apply",
            &["constrain", "apply", "regulate", "simplify", "specify"],
            "Generate constrained implementation directives with guardrails against over-engineering",
            "registry/templates/coding-guidelines/guidelines-apply.j2",
        ),
        (
            "coding-guidelines/guidelines-verify",
            TemplateType::KnowAct,
            "Coding Guidelines Verify",
            &["verify", "evaluate", "discriminate", "calibrate", "audit"],
            "Verify implementation against four principles with compliance scoring",
            "registry/templates/coding-guidelines/guidelines-verify.j2",
        ),
        // ── Handoff templates (KnowAct/WordAct — session context transfer) ──
        (
            "handoff/handoff-compact",
            TemplateType::KnowAct,
            "Handoff Compact",
            &["compact", "distill", "extract", "summarize", "crystallize"],
            "Compress session context into structured summary for handoff",
            "registry/templates/handoff/handoff-compact.j2",
        ),
        (
            "handoff/handoff-artifacts",
            TemplateType::KnowAct,
            "Handoff Artifacts",
            &["catalog", "reference", "classify", "detect", "redact"],
            "Catalog artifacts by reference and detect sensitive data for redaction",
            "registry/templates/handoff/handoff-artifacts.j2",
        ),
        (
            "handoff/handoff-skills-suggest",
            TemplateType::KnowAct,
            "Handoff Skills Suggest",
            &["suggest", "match", "prioritize", "analyze", "recommend"],
            "Suggest relevant skills and extract open questions for next session",
            "registry/templates/handoff/handoff-skills-suggest.j2",
        ),
        (
            "handoff/handoff-compose",
            TemplateType::WordAct,
            "Handoff Compose",
            &["compose", "synthesize", "structure", "redact", "document"],
            "Assemble final handoff document with redaction and skill suggestions",
            "registry/templates/handoff/handoff-compose.j2",
        ),
        (
            "flowdef/dispatch",
            TemplateType::FlowDef,
            "Dispatch",
            &["dispatch", "route", "invoke"],
            "Dispatches tool calls via ACP/MCP",
            "registry/templates/flowdef/dispatch.j2",
        ),
        (
            "flowdef/memory/recall",
            TemplateType::FlowDef,
            "Memory Recall",
            &["recall", "retrieve", "remember"],
            "Recalls semantic/episodic memory triples",
            "registry/templates/flowdef/memory_recall.j2",
        ),
        // DDMVSS Specification templates (FlowDef — specification manifests)
        (
            "spec/goal-capture",
            TemplateType::FlowDef,
            "Goal Capture",
            &["specify", "require", "elicit"],
            "Elicit user intent as binding requirement",
            "registry/templates/spec/goal-capture.j2",
        ),
        (
            "spec/constraint-bind",
            TemplateType::FlowDef,
            "Constraint Bind",
            &["constrain", "require"],
            "Attach OCAP boundaries to goals",
            "registry/templates/spec/constraint-bind.j2",
        ),
        (
            "spec/curate-collection",
            TemplateType::FlowDef,
            "Curate Collection",
            &["curate", "cultivate"],
            "Evaluate collection coherence and completeness",
            "registry/templates/spec/curate-collection.j2",
        ),
        (
            "spec/reconcile-conflicts",
            TemplateType::FlowDef,
            "Reconcile Conflicts",
            &["reconcile"],
            "Resolve goal tensions without collapsing them",
            "registry/templates/spec/reconcile-conflicts.j2",
        ),
        (
            "spec/contextualise",
            TemplateType::FlowDef,
            "Contextualise",
            &["contextualise"],
            "Situate artifact within meaningful environment",
            "registry/templates/spec/contextualise.j2",
        ),
        (
            "spec/selector",
            TemplateType::FlowDef,
            "Spec Selector",
            &["recognize", "match"],
            "Route input to best-fit specification template",
            "registry/templates/spec/selector.j2",
        ),
        (
            "skill-bundler/compose-bundle",
            TemplateType::FlowDef,
            "Compose Skill Bundle",
            &[
                "compose",
                "decompose",
                "reconcile",
                "curate",
                "sequence",
                "evaluate",
                "calibrate",
            ],
            "Analyze a set of skills and produce a FlowDef bundle manifest with conflicts, complementarities, and cascade steps",
            "registry/templates/skill-bundler/compose-bundle.j2",
        ),
        (
            "skill-bundler/apply-bundle",
            TemplateType::FlowDef,
            "Apply Skill Bundle",
            &[
                "sequence",
                "curate",
                "orient",
                "reconcile",
                "evaluate",
                "calibrate",
            ],
            "Apply an existing bundle to a session interaction following cascade order and conflict resolutions",
            "registry/templates/skill-bundler/apply-bundle.j2",
        ),
        (
            "skill-bundler/evolve-bundle",
            TemplateType::FlowDef,
            "Evolve Skill Bundle",
            &["compose", "reconcile", "calibrate", "cultivate", "evaluate"],
            "Re-compose a bundle when skills have evolved, preserving unchanged configuration and updating what has changed",
            "registry/templates/skill-bundler/evolve-bundle.j2",
        ),
    ];

    /// Bootstrap registry with core templates aligned to hKask domains.
    ///
    /// Template types use domain-aligned names:
    /// - WordAct (Jinja2 prompts) — "what to say"
    /// - KnowAct (Jinja2 cognition) — "how to think"
    /// - FlowDef (YAML manifests) — "what to do"
    pub fn bootstrap() -> Self {
        let mut registry = Self::new();
        let max_recursion = SYSTEM_MAX_RECURSION as u32;

        for &(id, ttype, name, terms, desc, path) in Self::BOOTSTRAP_TEMPLATES {
            registry.register(RegistryEntry {
                id: id.into(),
                template_type: ttype,
                name: name.into(),
                lexicon_terms: terms.iter().map(|s| s.to_string()).collect(),
                description: desc.into(),
                source_path: path.into(),
                required_capabilities: vec![],
                cascade_level: 0,
                matroshka_limit: max_recursion,
            });
        }

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

impl BundleRegistryIndex for Registry {
    fn register_bundle(&mut self, bundle: hkask_types::BundleManifest) {
        self.bundles.insert(bundle.id.clone(), bundle);
    }

    fn get_bundle(&self, id: &str) -> Option<hkask_types::BundleManifest> {
        self.bundles.get(id).cloned()
    }

    fn list_bundles(&self) -> Vec<hkask_types::BundleManifest> {
        self.bundles.values().cloned().collect()
    }

    fn remove_bundle(&mut self, id: &str) -> Option<hkask_types::BundleManifest> {
        self.bundles.remove(id)
    }

    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<hkask_types::BundleManifest> {
        let target: std::collections::HashSet<&str> =
            skill_ids.iter().map(|s| s.as_str()).collect();
        self.bundles
            .values()
            .find(|b| {
                let bundle_skills: std::collections::HashSet<&str> =
                    b.skills.iter().map(|s| s.id.as_str()).collect();
                bundle_skills == target
            })
            .cloned()
    }
}
