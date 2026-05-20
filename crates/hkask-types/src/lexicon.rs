//! hLexicon — Canonical vocabulary for hKask

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

    /// Create default hLexicon with bootstrap terms
    pub fn bootstrap() -> Self {
        let mut lexicon = Self::new();

        // WordAct terms (Prompt templates)
        lexicon.add(LexiconTerm::new(
            "recognize",
            Domain::WordAct,
            "Identify and classify input patterns",
        ));
        lexicon.add(LexiconTerm::new(
            "classify",
            Domain::WordAct,
            "Assign category or type to input",
        ));
        lexicon.add(LexiconTerm::new(
            "discriminate",
            Domain::WordAct,
            "Distinguish between similar patterns",
        ));

        // FlowDef terms (Process templates)
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

        // KnowAct terms (Cognition templates)
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

        lexicon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(TemplateType::Prompt.as_str(), "Prompt");
        assert_eq!(TemplateType::Process.as_str(), "Process");
        assert_eq!(TemplateType::Cognition.as_str(), "Cognition");
    }

    #[test]
    fn test_template_type_from_str() {
        assert_eq!(
            TemplateType::parse_str("Prompt"),
            Some(TemplateType::Prompt)
        );
        assert_eq!(
            TemplateType::parse_str("process"),
            Some(TemplateType::Process)
        );
        assert_eq!(TemplateType::parse_str("COGNITION"), None);
    }

    #[test]
    fn test_domain_as_str() {
        assert_eq!(Domain::WordAct.as_str(), "WordAct");
        assert_eq!(Domain::FlowDef.as_str(), "FlowDef");
        assert_eq!(Domain::KnowAct.as_str(), "KnowAct");
    }

    #[test]
    fn test_lexicon_term_new() {
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term");
        assert_eq!(term.term, "test");
        assert_eq!(term.domain, Domain::WordAct);
        assert!(term.academic_citation.is_none());
    }

    #[test]
    fn test_lexicon_term_with_citation() {
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term")
            .with_citation("Smith et al. 2024");
        assert_eq!(
            term.academic_citation,
            Some("Smith et al. 2024".to_string())
        );
    }

    #[test]
    fn test_hlexicon_add_and_get() {
        let mut lexicon = HLexicon::new();
        let term = LexiconTerm::new("test", Domain::WordAct, "A test term");
        lexicon.add(term.clone());

        let retrieved = lexicon.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().term, "test");
    }

    #[test]
    fn test_hlexicon_contains() {
        let mut lexicon = HLexicon::new();
        lexicon.add(LexiconTerm::new("test", Domain::WordAct, "A test term"));

        assert!(lexicon.contains("test"));
        assert!(!lexicon.contains("missing"));
    }

    #[test]
    fn test_hlexicon_validate() {
        let mut lexicon = HLexicon::new();
        lexicon.add(LexiconTerm::new("known", Domain::WordAct, "Known term"));

        let terms = vec!["known".to_string(), "unknown".to_string()];
        let invalid = lexicon.validate(&terms);

        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], "unknown");
    }

    #[test]
    fn test_hlexicon_bootstrap() {
        let lexicon = HLexicon::bootstrap();
        assert!(lexicon.len() > 0);
        assert!(lexicon.contains("recognize"));
        assert!(lexicon.contains("select"));
        assert!(lexicon.contains("reflect"));
    }
}
