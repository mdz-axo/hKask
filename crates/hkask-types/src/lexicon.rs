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

/// Template type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TemplateType {
    Prompt,
    Process,
    Cognition,
}

impl TemplateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::Prompt => "Prompt",
            TemplateType::Process => "Process",
            TemplateType::Cognition => "Cognition",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Prompt" | "prompt" => Some(TemplateType::Prompt),
            "Process" | "process" => Some(TemplateType::Process),
            "Cognition" | "cognition" => Some(TemplateType::Cognition),
            _ => None,
        }
    }
}

/// Domain for hLexicon terms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) enum Domain {
    WordAct,
    FlowDef,
    KnowAct,
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::WordAct => "WordAct",
            Domain::FlowDef => "FlowDef",
            Domain::KnowAct => "KnowAct",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "WordAct" | "wordact" => Some(Domain::WordAct),
            "FlowDef" | "flowdef" => Some(Domain::FlowDef),
            "KnowAct" | "knowact" => Some(Domain::KnowAct),
            _ => None,
        }
    }
}

/// hLexicon term — canonical vocabulary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LexiconTerm {
    pub term: String,
    pub domain: Domain,
    pub definition: String,
    pub academic_citation: Option<String>,
}

impl LexiconTerm {
    pub fn new(term: &str, domain: Domain, definition: &str) -> Self {
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
pub(crate) struct HLexicon {
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

    pub fn terms(&self) -> impl Iterator<Item = &LexiconTerm> {
        self.terms.values()
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
            Domain::KnowAct,
            "Identify and classify input patterns",
        ));
        lexicon.add(LexiconTerm::new(
            "classify",
            Domain::KnowAct,
            "Assign category or type to input",
        ));
        lexicon.add(LexiconTerm::new(
            "discriminate",
            Domain::KnowAct,
            "Distinguish between similar patterns",
        ));

        // FlowDef terms — MVSDD pipeline steps (select → populate → execute)
        lexicon.add(LexiconTerm::new(
            "select",
            Domain::FlowDef,
            "Choose best-fit template from registry",
        ));
        lexicon.add(LexiconTerm::new(
            "populate",
            Domain::FlowDef,
            "Bind input data to template fields",
        ));
        lexicon.add(LexiconTerm::new(
            "execute",
            Domain::FlowDef,
            "Invoke target model or tool",
        ));

        // KnowAct terms — reflective cognition (catalog §3.4 Metacognition)
        lexicon.add(LexiconTerm::new(
            "reflect",
            Domain::KnowAct,
            "Analyze outcomes for patterns",
        ));
        lexicon.add(LexiconTerm::new(
            "calibrate",
            Domain::KnowAct,
            "Adjust confidence based on outcomes",
        ));
        lexicon.add(LexiconTerm::new(
            "improve",
            Domain::KnowAct,
            "Propose template revisions",
        ));

        // SpecCure — WordAct (Speech Acts of Specification)
        lexicon.add(LexiconTerm::new(
            "specify",
            Domain::WordAct,
            "Define a binding constraint or intent",
        ));
        lexicon.add(LexiconTerm::new(
            "require",
            Domain::WordAct,
            "State a non-negotiable condition",
        ));
        lexicon.add(LexiconTerm::new(
            "constrain",
            Domain::WordAct,
            "Limit the solution space",
        ));

        // SpecCure — FlowDef (Process of Composition)
        lexicon.add(LexiconTerm::new(
            "curate",
            Domain::FlowDef,
            "Select, contextualise, and integrate artifacts",
        ));
        lexicon.add(LexiconTerm::new(
            "elicit",
            Domain::FlowDef,
            "Draw out latent goals or requirements",
        ));
        lexicon.add(LexiconTerm::new(
            "reconcile",
            Domain::FlowDef,
            "Resolve conflicts between goals or requirements",
        ));

        // SpecCure — KnowAct (Cognitive Acts of Curation)
        lexicon.add(LexiconTerm::new(
            "contextualise",
            Domain::KnowAct,
            "Situate an artifact within its meaningful environment",
        ));
        lexicon.add(LexiconTerm::new(
            "cultivate",
            Domain::KnowAct,
            "Nurture growth and coherence over time",
        ));

        lexicon
    }
}
