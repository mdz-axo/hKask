//! hLexicon — Loop 2b (Semantic Memory): canonical vocabulary
//
//! The hLexicon is the shared public vocabulary stored in semantic memory.
//! Curation (Loop 5) curates terms; Inference (Loop 1) uses them for prompting.
//
//! The canonical vocabulary is authored in
//! `docs/architecture/reference/hKask-hLexicon.md` (the single source of truth)
//! and derived into the workspace lexicon registry
//! `registry/hlexicon/hlexicon-workspace.yaml`. Loading the full vocabulary
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
    /// REQ: TYP-212
    /// pre:  self is a valid TemplateType variant
    /// post: returns the canonical PascalCase string ("WordAct", "KnowAct", "FlowDef")
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "WordAct",
            TemplateType::KnowAct => "KnowAct",
            TemplateType::FlowDef => "FlowDef",
        }
    }

    /// REQ: TYP-213
    /// pre:  s is a string in PascalCase or lowercase ("WordAct"/"wordact", "KnowAct"/"knowact", "FlowDef"/"flowdef")
    /// post: returns Some(TemplateType) if s matches a known variant; None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "WordAct" | "wordact" => Some(TemplateType::WordAct),
            "KnowAct" | "knowact" => Some(TemplateType::KnowAct),
            "FlowDef" | "flowdef" => Some(TemplateType::FlowDef),
            _ => None,
        }
    }

    /// REQ: TYP-214
    /// pre:  self is a valid TemplateType variant
    /// post: returns the file extension: "j2" for WordAct/KnowAct, "yaml" for FlowDef
    pub fn file_extension(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "j2",
            TemplateType::KnowAct => "j2",
            TemplateType::FlowDef => "yaml",
        }
    }

    /// REQ: TYP-215
    /// pre:  self is a valid TemplateType variant
    /// post: returns the MDS specification name: WordAct→"Prompt", KnowAct→"Cognition", FlowDef→"Process"
    pub fn as_spec_name(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "Prompt",
            TemplateType::KnowAct => "Cognition",
            TemplateType::FlowDef => "Process",
        }
    }

    /// REQ: TYP-216
    /// pre:  ext is a file extension string (e.g. "j2", "yaml", "yml")
    /// post: returns Some(KnowAct) for "j2", Some(FlowDef) for "yaml"/"yml"; None for unknown extensions
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

/// MDS Category — the five categories from the Minimal Domain Specification.
///
/// Shared across `hkask-types` (lexicon) and `hkask-storage` (spec_types).
/// When a `LexiconTerm` carries a category, the mapping from 87 terms → 5
/// categories becomes formally verifiable via `spec/graph/coherence`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MdsCategory {
    Domain,
    Composition,
    Trust,
    Lifecycle,
    Curation,
}

impl MdsCategory {
    /// REQ: TYP-217
    /// pre:  self is a valid MdsCategory variant
    /// post: returns the lowercase category string ("domain", "composition", "trust", "lifecycle", "curation")
    pub fn as_str(&self) -> &'static str {
        match self {
            MdsCategory::Domain => "domain",
            MdsCategory::Composition => "composition",
            MdsCategory::Trust => "trust",
            MdsCategory::Lifecycle => "lifecycle",
            MdsCategory::Curation => "curation",
        }
    }
}

/// hLexicon term — canonical vocabulary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiconTerm {
    pub term: String,
    pub domain: TemplateType,
    pub definition: String,
    pub academic_citation: Option<String>,
    /// MDS category this term belongs to. When set, enables formal
    /// verification that all 87 hLexicon terms map to valid MDS categories
    /// via `spec/graph/coherence`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mds_category: Option<MdsCategory>,
}

impl LexiconTerm {
    /// REQ: TYP-218
    /// pre:  term is non-empty, domain is a valid TemplateType, definition is non-empty
    /// post: returns LexiconTerm with academic_citation=None, mds_category=None
    pub fn new(term: &str, domain: TemplateType, definition: &str) -> Self {
        Self {
            term: term.to_string(),
            domain,
            definition: definition.to_string(),
            academic_citation: None,
            mds_category: None,
        }
    }

    /// REQ: TYP-219
    /// pre:  citation is a non-empty string
    /// post: returns self with academic_citation set to Some(citation.to_string())
    pub fn with_citation(mut self, citation: &str) -> Self {
        self.academic_citation = Some(citation.to_string());
        self
    }

    /// REQ: TYP-220
    /// pre:  cat is a valid MdsCategory variant
    /// post: returns self with mds_category set to Some(cat)
    pub fn with_mds_category(mut self, cat: MdsCategory) -> Self {
        self.mds_category = Some(cat);
        self
    }
}

/// hLexicon — Collection of canonical terms
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HLexicon {
    terms: HashMap<String, LexiconTerm>,
}

impl HLexicon {
    /// REQ: TYP-221
    /// post: returns an empty HLexicon
    pub fn new() -> Self {
        Self {
            terms: HashMap::new(),
        }
    }

    /// REQ: TYP-222
    /// pre:  term is a valid LexiconTerm with a non-empty term field
    /// post: inserts term into the lexicon keyed by term.term; replaces existing entry if term already present
    pub fn add(&mut self, term: LexiconTerm) {
        self.terms.insert(term.term.clone(), term);
    }

    /// REQ: TYP-223
    /// pre:  term is a non-empty string key
    /// post: returns Some(&LexiconTerm) if term exists in lexicon; None otherwise
    pub fn get(&self, term: &str) -> Option<&LexiconTerm> {
        self.terms.get(term)
    }

    /// REQ: TYP-224
    /// pre:  term is a non-empty string key
    /// post: returns true if term exists in lexicon; false otherwise
    pub fn contains(&self, term: &str) -> bool {
        self.terms.contains_key(term)
    }

    /// REQ: TYP-225
    /// pre:  terms is a slice of String keys to validate
    /// post: returns Vec<String> of terms not found in the lexicon (empty if all present)
    pub fn validate(&self, terms: &[String]) -> Vec<String> {
        terms
            .iter()
            .filter(|t| !self.contains(t))
            .cloned()
            .collect()
    }

    /// REQ: TYP-226
    /// post: returns the number of terms in the lexicon
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// REQ: TYP-227
    /// post: returns true if the lexicon contains no terms; false otherwise
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// REQ: TYP-228
    /// post: returns a bootstrap HLexicon with 17 minimal startup terms covering KnowAct, FlowDef, and WordAct domains
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

        // KnowAct terms — verification and diagnosis (test program)
        lexicon.add(LexiconTerm::new(
            "diagnose",
            TemplateType::KnowAct,
            "Construct a feedback loop to identify root causes",
        ));
        lexicon.add(LexiconTerm::new(
            "verify",
            TemplateType::KnowAct,
            "Mechanically check whether behavioral tests exist for a seam",
        ));

        // FlowDef terms — test program and skill workflows
        lexicon.add(LexiconTerm::new(
            "trace",
            TemplateType::FlowDef,
            "Execute a tracer-bullet test cycle (RED→GREEN for one invariant)",
        ));
        lexicon.add(LexiconTerm::new(
            "deepen",
            TemplateType::FlowDef,
            "Extract a smaller interface from a shallow module to create a testable seam",
        ));
        lexicon.add(LexiconTerm::new(
            "register",
            TemplateType::FlowDef,
            "Record a skill-to-MDS mapping as a SpecArtifact",
        ));
        lexicon.add(LexiconTerm::new(
            "handoff",
            TemplateType::FlowDef,
            "Transfer session context to a fresh agent for continuity",
        ));

        lexicon
    }
}
