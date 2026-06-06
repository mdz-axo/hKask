//! hLexicon — Loop 2b (Semantic Memory): canonical vocabulary
//
//! The hLexicon is the shared public vocabulary stored in semantic memory.
//! Curation (Loop 5) curates terms; Inference (Loop 1) uses them for prompting.
//
//! The canonical vocabulary is authored in
//! `docs/architecture/reference/hKask-hLexicon.md` (the single source of truth)
//! and derived into the workspace lexicon registry
//! `registry/registries/hlexicon-workspace.yaml`. Loading the full vocabulary
//! from that YAML lives in `hkask-templates` (which owns lexicon validation and
//! already depends on a YAML parser); this crate provides the plain types and a
//! minimal [`HLexicon::bootstrap`] fixture.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template type discriminator — aligned with hKask domains.
///
/// Each variant corresponds to a domain in the architecture and a file format:
/// - **WordAct**: Jinja2 prompt templates — "what to say" — `.j2`
/// - **KnowAct**: Jinja2 cognition templates — "how to think" — `.j2`
/// - **FlowDef**: YAML process manifests — "what to do" — `.yaml`
///
/// Specifications are FlowDef manifests that define constraints; they are not
/// a separate type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TemplateType {
    /// Jinja2 prompt templates — "what to say"
    WordAct,
    /// Jinja2 cognition templates — "how to think"
    KnowAct,
    /// YAML process manifests — "what to do"
    FlowDef,
}

impl TemplateType {
    /// Return the canonical domain-aligned string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "WordAct",
            TemplateType::KnowAct => "KnowAct",
            TemplateType::FlowDef => "FlowDef",
        }
    }

    /// Parse a domain-aligned template type string.
    /// Accepts PascalCase and lowercase forms of WordAct, KnowAct, FlowDef.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "WordAct" | "wordact" => Some(TemplateType::WordAct),
            "KnowAct" | "knowact" => Some(TemplateType::KnowAct),
            "FlowDef" | "flowdef" => Some(TemplateType::FlowDef),
            _ => None,
        }
    }

    /// Return the file extension for templates of this type.
    ///
    /// - WordAct → `.j2` (Jinja2 prompt)
    /// - KnowAct → `.j2` (Jinja2 cognition)
    /// - FlowDef → `.yaml` (YAML manifest)
    pub fn file_extension(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "j2",
            TemplateType::KnowAct => "j2",
            TemplateType::FlowDef => "yaml",
        }
    }

    /// Infer template type from a file extension.
    ///
    /// - `.j2` → KnowAct (Jinja2 cognition is the more general Jinja2 type;
    ///   WordAct is disambiguated by path convention or manifest metadata)
    /// - `.yaml` / `.yml` → FlowDef
    pub fn infer_from_extension(ext: &str) -> Option<Self> {
        match ext {
            "j2" => Some(TemplateType::KnowAct),
            "yaml" | "yml" => Some(TemplateType::FlowDef),
            _ => None,
        }
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// hLexicon term — canonical vocabulary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiconTerm {
    pub term: String,
    pub domain: TemplateType,
    pub definition: String,
    pub academic_citation: Option<String>,
}

impl LexiconTerm {
    pub fn new(term: &str, domain: TemplateType, definition: &str) -> Self {
        Self {
            term: term.to_string(),
            domain,
            definition: definition.to_string(),
            academic_citation: None,
        }
    }

    pub fn with_citation(mut self, citation: &str) -> Self {
        self.academic_citation = Some(citation.to_string());
        self
    }
}

/// hLexicon — Collection of canonical terms
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HLexicon {
    terms: HashMap<String, LexiconTerm>,
}

impl HLexicon {
    pub fn new() -> Self {
        Self {
            terms: HashMap::new(),
        }
    }

    pub fn add(&mut self, term: LexiconTerm) {
        self.terms.insert(term.term.clone(), term);
    }

    pub fn get(&self, term: &str) -> Option<&LexiconTerm> {
        self.terms.get(term)
    }

    pub fn contains(&self, term: &str) -> bool {
        self.terms.contains_key(term)
    }

    pub fn validate(&self, terms: &[String]) -> Vec<String> {
        terms
            .iter()
            .filter(|t| !self.contains(t))
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Create the default bootstrap hLexicon.
    ///
    /// This is a minimal startup subset (17 terms), NOT the full vocabulary.
    /// The full canonical vocabulary is authored in
    /// `docs/architecture/reference/hKask-hLexicon.md` and loaded from
    /// `registry/registries/hlexicon-workspace.yaml` by `hkask-templates`. This
    /// fixture is retained for lightweight tests and seeds; domain assignments
    /// here MUST match the catalog's domain classification.
    pub fn bootstrap() -> Self {
        let mut lexicon = Self::new();

        // KnowAct terms — pattern recognition (catalog §3.1 Recognition)
        lexicon.add(LexiconTerm::new(
            "recognize",
            TemplateType::KnowAct,
            "Identify and classify input patterns",
        ));
        lexicon.add(LexiconTerm::new(
            "classify",
            TemplateType::KnowAct,
            "Assign category or type to input",
        ));
        lexicon.add(LexiconTerm::new(
            "discriminate",
            TemplateType::KnowAct,
            "Distinguish between similar patterns",
        ));

        // FlowDef terms — MVSDD pipeline steps (select → populate → execute)
        lexicon.add(LexiconTerm::new(
            "select",
            TemplateType::FlowDef,
            "Choose best-fit template from registry",
        ));
        lexicon.add(LexiconTerm::new(
            "populate",
            TemplateType::FlowDef,
            "Bind input data to template fields",
        ));
        lexicon.add(LexiconTerm::new(
            "execute",
            TemplateType::FlowDef,
            "Invoke target model or tool",
        ));

        // KnowAct terms — reflective cognition (catalog §3.4 Metacognition)
        lexicon.add(LexiconTerm::new(
            "reflect",
            TemplateType::KnowAct,
            "Analyze outcomes for patterns",
        ));
        lexicon.add(LexiconTerm::new(
            "calibrate",
            TemplateType::KnowAct,
            "Adjust confidence based on outcomes",
        ));
        lexicon.add(LexiconTerm::new(
            "improve",
            TemplateType::KnowAct,
            "Propose template revisions",
        ));

        // WordAct terms — speech acts of specification
        lexicon.add(LexiconTerm::new(
            "specify",
            TemplateType::WordAct,
            "Define a binding constraint or intent",
        ));
        lexicon.add(LexiconTerm::new(
            "require",
            TemplateType::WordAct,
            "State a non-negotiable condition",
        ));
        lexicon.add(LexiconTerm::new(
            "constrain",
            TemplateType::WordAct,
            "Limit the solution space",
        ));

        // FlowDef terms — process of composition
        lexicon.add(LexiconTerm::new(
            "curate",
            TemplateType::FlowDef,
            "Select, contextualise, and integrate artifacts",
        ));
        lexicon.add(LexiconTerm::new(
            "elicit",
            TemplateType::FlowDef,
            "Draw out latent goals or requirements",
        ));
        lexicon.add(LexiconTerm::new(
            "reconcile",
            TemplateType::FlowDef,
            "Resolve conflicts between goals or requirements",
        ));

        // KnowAct terms — cognitive acts of curation
        lexicon.add(LexiconTerm::new(
            "contextualise",
            TemplateType::KnowAct,
            "Situate an artifact within its meaningful environment",
        ));
        lexicon.add(LexiconTerm::new(
            "cultivate",
            TemplateType::KnowAct,
            "Nurture growth and coherence over time",
        ));

        lexicon
    }
}
