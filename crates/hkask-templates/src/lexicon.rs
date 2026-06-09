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

/// Parse the canonical hLexicon markdown file into an intermediate catalog structure.
///
/// FocusingAssumption FA-Co1: Minimal stub — full implementation deferred.
/// This function will parse the term tables from `hKask-hLexicon.md` and
/// produce a structured catalog that `render_workspace_yaml` can convert to YAML.
pub fn parse_markdown_catalog(_markdown: &str) -> Result<Vec<LexiconTerm>, String> {
    todo!(
        "spec_ahead: parse_markdown_catalog — markdown-to-YAML derivation pipeline not yet implemented. See hKask-hLexicon.md and FocusingAssumption FA-Co1."
    )
}

/// Render a workspace YAML string from a catalog of lexicon terms.
///
/// FocusingAssumption FA-Co1: Minimal stub — full implementation deferred.
/// This function will serialize the catalog terms into the `hlexicon-workspace.yaml`
/// format that `load_hlexicon_from_yaml` parses.
pub fn render_workspace_yaml(_terms: &[LexiconTerm]) -> Result<String, String> {
    todo!(
        "spec_ahead: render_workspace_yaml — markdown-to-YAML derivation pipeline not yet implemented. See hKask-hLexicon.md and FocusingAssumption FA-Co1."
    )
}

/// Regenerate `hlexicon-workspace.yaml` from the canonical markdown source.
///
/// FocusingAssumption FA-Co1: Minimal stub — full implementation deferred.
/// This is the top-level pipeline: parse_markdown_catalog → render_workspace_yaml → write to disk.
pub fn regenerate_workspace_yaml(_markdown: &str) -> Result<String, String> {
    todo!(
        "spec_ahead: regenerate_workspace_yaml — markdown-to-YAML derivation pipeline not yet implemented. See hKask-hLexicon.md and FocusingAssumption FA-Co1."
    )
}
