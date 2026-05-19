//! hLexicon — Canonical vocabulary for hKask

use serde::{Deserialize, Serialize};

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
}

/// Domain for hLexicon terms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Domain {
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
}

/// hLexicon term — canonical vocabulary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiconTerm {
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
pub struct HLexicon {
    pub terms: Vec<LexiconTerm>,
}

impl HLexicon {
    pub fn new() -> Self {
        Self { terms: Vec::new() }
    }

    pub fn add(&mut self, term: LexiconTerm) {
        self.terms.push(term);
    }

    pub fn get(&self, term: &str) -> Option<&LexiconTerm> {
        self.terms.iter().find(|t| t.term == term)
    }

    pub fn validate(&self, terms: &[String]) -> Vec<String> {
        terms
            .iter()
            .filter(|t| self.get(t).is_none())
            .cloned()
            .collect()
    }
}
