//! hLexicon YAML loader and validation
//!
//! Loads the canonical vocabulary from
//! `registry/registries/hlexicon-workspace.yaml` and validates
//! lexicon_terms against it during template registration.

use hkask_types::{HLexicon, LexiconTerm, TemplateType};
use std::path::Path;

/// Intermediate YAML deserialization structure matching the workspace YAML format:
///
/// ```yaml
/// hlexicon:
///   wordact:
///     - term: query
///       definition: "Ask for information"
///   flowdef:
///     - term: sequence
///       definition: "Linear ordering"
///   knowact:
///     - term: recognize
///       definition: "Identify pattern"
/// ```
#[derive(Debug, serde::Deserialize)]
struct HlexiconWorkspaceYaml {
    hlexicon: HlexiconDomains,
}

#[derive(Debug, serde::Deserialize)]
struct HlexiconDomains {
    wordact: Option<Vec<LexiconTermYaml>>,
    flowdef: Option<Vec<LexiconTermYaml>>,
    knowact: Option<Vec<LexiconTermYaml>>,
}

#[derive(Debug, serde::Deserialize)]
struct LexiconTermYaml {
    term: String,
    definition: String,
    #[serde(default)]
    academic_citation: Option<String>,
}

/// Load the hLexicon from the workspace YAML file.
///
/// The canonical vocabulary is authored in
/// `docs/architecture/reference/hKask-hLexicon.md` and derived into
/// `registry/registries/hlexicon-workspace.yaml`. This function parses
/// that YAML into an `HLexicon` suitable for validation during registration.
pub fn load_hlexicon_from_yaml(content: &str) -> Result<HLexicon, String> {
    let workspace: HlexiconWorkspaceYaml = serde_yaml::from_str(content)
        .map_err(|e| format!("Failed to parse hLexicon YAML: {}", e))?;

    let mut lexicon = HLexicon::new();

    if let Some(terms) = workspace.hlexicon.wordact {
        for t in terms {
            let mut term = LexiconTerm::new(&t.term, TemplateType::WordAct, &t.definition);
            if let Some(ref citation) = t.academic_citation {
                term = term.with_citation(citation);
            }
            lexicon.add(term);
        }
    }

    if let Some(terms) = workspace.hlexicon.flowdef {
        for t in terms {
            let mut term = LexiconTerm::new(&t.term, TemplateType::FlowDef, &t.definition);
            if let Some(ref citation) = t.academic_citation {
                term = term.with_citation(citation);
            }
            lexicon.add(term);
        }
    }

    if let Some(terms) = workspace.hlexicon.knowact {
        for t in terms {
            let mut term = LexiconTerm::new(&t.term, TemplateType::KnowAct, &t.definition);
            if let Some(ref citation) = t.academic_citation {
                term = term.with_citation(citation);
            }
            lexicon.add(term);
        }
    }

    Ok(lexicon)
}

/// Load the hLexicon from a file path.
pub fn load_hlexicon_from_file(path: &Path) -> Result<HLexicon, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read hLexicon file {:?}: {}", path, e))?;
    load_hlexicon_from_yaml(&content)
}

/// Load the hLexicon from the default workspace YAML path.
///
/// Resolution order:
/// 1. `HKASK_TEMPLATES_PATH` env var parent + `registries/hlexicon-workspace.yaml`
/// 2. `registry/registries/hlexicon-workspace.yaml` (relative to CWD)
pub fn load_hlexicon_default() -> Result<HLexicon, String> {
    let path = std::env::var("HKASK_TEMPLATES_PATH")
        .map(|p| {
            let templates_path = Path::new(&p);
            templates_path
                .parent()
                .map(|p| p.join("registries/hlexicon-workspace.yaml"))
                .unwrap_or_else(|| {
                    Path::new("registry/registries/hlexicon-workspace.yaml").to_path_buf()
                })
        })
        .unwrap_or_else(|_| Path::new("registry/registries/hlexicon-workspace.yaml").to_path_buf());

    load_hlexicon_from_file(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_hlexicon_from_yaml_valid() {
        let yaml = r#"
hlexicon:
  wordact:
    - term: query
      definition: "Ask for information"
    - term: instruct
      definition: "Give step-by-step direction"
  knowact:
    - term: recognize
      definition: "Identify pattern"
    - term: calibrate
      definition: "Tune accuracy"
  flowdef:
    - term: sequence
      definition: "Linear ordering"
"#;
        let lexicon = load_hlexicon_from_yaml(yaml).unwrap();
        assert!(lexicon.contains("query"));
        assert!(lexicon.contains("recognize"));
        assert!(lexicon.contains("sequence"));
        assert_eq!(lexicon.len(), 5);

        // Verify domain assignment
        let query_term = lexicon.get("query").unwrap();
        assert_eq!(query_term.domain, TemplateType::WordAct);

        let recognize_term = lexicon.get("recognize").unwrap();
        assert_eq!(recognize_term.domain, TemplateType::KnowAct);

        let sequence_term = lexicon.get("sequence").unwrap();
        assert_eq!(sequence_term.domain, TemplateType::FlowDef);
    }

    #[test]
    fn load_hlexicon_validates_terms() {
        let yaml = r#"
hlexicon:
  wordact:
    - term: query
      definition: "Ask for information"
  knowact:
    - term: recognize
      definition: "Identify pattern"
"#;
        let lexicon = load_hlexicon_from_yaml(yaml).unwrap();

        // Known terms should validate cleanly
        let unknown = lexicon.validate(&["query".into(), "recognize".into()]);
        assert!(unknown.is_empty());

        // Unknown terms should be flagged
        let unknown = lexicon.validate(&["query".into(), "unknown_term".into()]);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0], "unknown_term");
    }

    #[test]
    fn load_hlexicon_empty_domains() {
        let yaml = r#"
hlexicon:
  wordact: []
  knowact: []
  flowdef: []
"#;
        let lexicon = load_hlexicon_from_yaml(yaml).unwrap();
        assert!(lexicon.is_empty());
    }
}
