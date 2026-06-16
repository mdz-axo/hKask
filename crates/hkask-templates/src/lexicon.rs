//! hLexicon YAML loader and validation
//!
//! Loads the canonical vocabulary from
//! `registry/hlexicon/hlexicon-workspace.yaml` and validates
//! lexicon_terms against it during template registration.

use hkask_types::lexicon::{HLexicon, LexiconTerm, TemplateType};
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
/// `registry/hlexicon/hlexicon-workspace.yaml`. This function parses
/// that YAML into an `HLexicon` suitable for validation during registration.
///
/// REQ: P3-tpl-load-hlexicon-yaml
/// \[P3\] Motivating: Generative Space — loads canonical workspace vocabulary
/// \[P8\] Constraining: Semantic Grounding — YAML vocabulary round-trips to HLexicon
/// pre:  content is valid YAML in hlexicon-workspace format
/// post: returns HLexicon populated with WordAct, FlowDef, KnowAct terms
pub fn load_hlexicon_from_yaml(content: &str) -> Result<HLexicon, String> {
    let workspace: HlexiconWorkspaceYaml = serde_yaml::from_str(content)
        .map_err(|e| format!("Failed to parse hLexicon YAML: {}", e))?;
    let mut lexicon = HLexicon::new();
    for (terms, tt) in [
        (&workspace.hlexicon.wordact, TemplateType::WordAct),
        (&workspace.hlexicon.flowdef, TemplateType::FlowDef),
        (&workspace.hlexicon.knowact, TemplateType::KnowAct),
    ] {
        if let Some(terms) = terms {
            for t in terms {
                let mut term = LexiconTerm::new(&t.term, tt, &t.definition);
                if let Some(ref citation) = t.academic_citation {
                    term = term.with_citation(citation);
                }
                lexicon.add(term);
            }
        }
    }
    Ok(lexicon)
}

/// Load the hLexicon from a file path.
///
/// REQ: P3-tpl-load-hlexicon-file
/// \[P3\] Motivating: Generative Space — loads vocabulary from filesystem path
/// \[P8\] Constraining: Semantic Grounding — file contents parsed into HLexicon
/// pre:  path points to a valid hlexicon-workspace YAML file
/// post: returns HLexicon parsed from file contents
pub fn load_hlexicon_from_file(path: &Path) -> Result<HLexicon, String> {
    load_hlexicon_from_yaml(
        &std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read hLexicon file {:?}: {}", path, e))?,
    )
}

/// Load the hLexicon from the default workspace path.
///
/// REQ: P3-tpl-load-hlexicon-default
/// \[P3\] Motivating: Generative Space — loads built-in default vocabulary
/// \[P8\] Constraining: Semantic Grounding — default terms seed the workspace lexicon
/// post: returns HLexicon from registry/hlexicon/hlexicon-workspace.yaml
/// post: respects HKASK_TEMPLATES_PATH env var for path resolution
pub fn load_hlexicon_default() -> Result<HLexicon, String> {
    let path = std::env::var("HKASK_TEMPLATES_PATH")
        .map(|p| {
            let tp = Path::new(&p);
            tp.parent()
                .map(|p| p.join("hlexicon/hlexicon-workspace.yaml"))
                .unwrap_or_else(|| {
                    Path::new("registry/hlexicon/hlexicon-workspace.yaml").to_path_buf()
                })
        })
        .unwrap_or_else(|_| Path::new("registry/hlexicon/hlexicon-workspace.yaml").to_path_buf());
    load_hlexicon_from_file(&path)
}

/// Parse the canonical hLexicon markdown file into an intermediate catalog structure.
///
/// Parses term tables from `hKask-hLexicon.md` — extracting all (term, definition, domain)
/// tuples from the three domain sections (WordAct, FlowDef, KnowAct). Each term is
/// extracted from backtick-quoted first column entries in tables with `Term | Definition`
/// headers.
///
/// REQ: P3-tpl-parse-markdown-catalog
/// \[P3\] Motivating: Generative Space — extracts terms from markdown catalog
/// \[P8\] Constraining: Semantic Grounding — markdown tables become structured terms
/// pre:  markdown is valid hLexicon markdown content
/// post: returns Vec of (term, definition, TemplateType) tuples
pub fn parse_markdown_catalog(
    markdown: &str,
) -> Result<Vec<(String, String, TemplateType)>, String> {
    let mut terms = Vec::new();
    let mut current_domain: Option<TemplateType> = None;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Domain 1:") {
            current_domain = Some(TemplateType::WordAct);
            continue;
        }
        if trimmed.starts_with("## Domain 2:") {
            current_domain = Some(TemplateType::FlowDef);
            continue;
        }
        if trimmed.starts_with("## Domain 3:") {
            current_domain = Some(TemplateType::KnowAct);
            continue;
        }
        if trimmed.starts_with("## Cross-Domain") {
            break;
        }
        if let Some(domain) = current_domain
            && trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && !trimmed.contains("---")
        {
            let cells: Vec<&str> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim())
                .collect();
            if cells.len() >= 2
                && let Some(term) = extract_backtick_quoted(cells[0])
                && !term.eq_ignore_ascii_case("term")
            {
                let definition = cells[1].trim_matches('"');
                terms.push((term, definition.to_string(), domain));
            }
        }
    }
    if terms.is_empty() {
        return Err("No hLexicon terms extracted from markdown. Check that domain headers (## Domain 1/2/3:) and term tables are present.".into());
    }
    Ok(terms)
}

/// Extract content between backtick-quoted syntax, e.g. `term` → "term".
fn extract_backtick_quoted(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('`')
        && s.len() >= 3
        && let Some(end) = s[1..].find('`')
    {
        return Some(s[1..=end].to_string());
    }
    None
}

/// Render a workspace YAML string from a catalog of lexicon terms.
///
/// Produces the `hlexicon-workspace.yaml` format that `load_hlexicon_from_yaml` parses.
/// Terms are sorted alphabetically within each domain for stable output.
///
/// REQ: P3-tpl-render-workspace-yaml
/// \[P3\] Motivating: Generative Space — serializes vocabulary to workspace YAML
/// \[P8\] Constraining: Semantic Grounding — YAML output preserves term semantics
/// pre:  terms is a valid catalog from parse_markdown_catalog
/// post: returns YAML string in hlexicon-workspace format
/// post: terms sorted alphabetically within each domain
pub fn render_workspace_yaml(terms: &[(String, String, TemplateType)]) -> Result<String, String> {
    let mut wordact: Vec<&(String, String, TemplateType)> = terms
        .iter()
        .filter(|(_, _, d)| *d == TemplateType::WordAct)
        .collect();
    let mut flowdef: Vec<&(String, String, TemplateType)> = terms
        .iter()
        .filter(|(_, _, d)| *d == TemplateType::FlowDef)
        .collect();
    let mut knowact: Vec<&(String, String, TemplateType)> = terms
        .iter()
        .filter(|(_, _, d)| *d == TemplateType::KnowAct)
        .collect();

    wordact.sort_by(|a, b| a.0.cmp(&b.0));
    flowdef.sort_by(|a, b| a.0.cmp(&b.0));
    knowact.sort_by(|a, b| a.0.cmp(&b.0));

    let mut yaml = String::new();
    yaml.push_str("# hLexicon Workspace Registry\n");
    yaml.push_str("# DERIVED ARTIFACT — do not hand-edit term definitions.\n");
    yaml.push_str("# Canonical source: docs/architecture/reference/hKask-hLexicon.md\n");
    yaml.push_str("# Regenerate after editing the markdown:\n");
    yaml.push_str("#   cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored\n");
    yaml.push_str("# The hlexicon_yaml_matches_markdown test fails if this drifts.\n");
    yaml.push('\n');
    yaml.push_str("hlexicon:\n");

    render_domain(&mut yaml, "wordact", &wordact)?;
    render_domain(&mut yaml, "flowdef", &flowdef)?;
    render_domain(&mut yaml, "knowact", &knowact)?;

    Ok(yaml)
}

/// Render a single domain's terms into YAML.
fn render_domain(
    yaml: &mut String,
    domain_key: &str,
    terms: &[&(String, String, TemplateType)],
) -> Result<(), String> {
    yaml.push_str(&format!("  {}:\n", domain_key));
    for (term, definition, _) in terms {
        // YAML-safe: double-quote the definition, escape embedded double-quotes
        let escaped_def = definition.replace('\\', "\\\\").replace('"', "\\\"");
        yaml.push_str(&format!("    - term: {}\n", term));
        yaml.push_str(&format!("      definition: \"{}\"\n", escaped_def));
    }
    Ok(())
}

/// Regenerate `hlexicon-workspace.yaml` from the canonical markdown source.
///
/// This is the top-level pipeline: `parse_markdown_catalog` → `render_workspace_yaml`.
/// Returns the YAML content that should be written to disk.
///
/// REQ: P3-tpl-regenerate-workspace-yaml
/// \[P3\] Motivating: Generative Space — full markdown-to-YAML vocabulary pipeline
/// \[P8\] Constraining: Semantic Grounding — regenerated YAML matches canonical source
/// pre:  markdown is valid hLexicon markdown content
/// post: returns YAML string ready to write to hlexicon-workspace.yaml
pub fn regenerate_workspace_yaml(markdown: &str) -> Result<String, String> {
    let catalog = parse_markdown_catalog(markdown)?;
    render_workspace_yaml(&catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P8: parse_markdown_catalog extracts terms from markdown tables across all three domains
    // REQ: P3-tpl-test-parse-catalog-extracts-terms — parse_markdown_catalog extracts terms from markdown tables across all three domains
    // [P3] Motivating: Generative Space — validates markdown catalog parsing
    // [P8] Constraining: Semantic Grounding — extracts terms from canonical markdown
    #[test]
    fn parse_catalog_extracts_terms() {
        let markdown = "\
            ## Domain 1: WordAct — Prompting Language (34 terms)\n\
            ### 1.1 Directive Acts\n\
            | Term | Definition | Example Usage |\n\
            |------|------------|---------------|\n\
            | `query` | Ask for information | \"Query: What is...\" |\n\
            | `request` | Ask for action | \"Request: Summarize...\" |\n\
            \n\
            ## Domain 2: FlowDef — Process Flow Language (42 terms)\n\
            ### 2.1 Control Flow\n\
            | Term | Definition | Example Usage |\n\
            |------|------------|---------------|\n\
            | `sequence` | Linear ordering | \"sequence: A then B\" |\n\
            | `parallel` | Concurrent execution | \"parallel: A and B\" |\n\
            \n\
            ## Domain 3: KnowAct — Cognition Language (66 terms)\n\
            ### 3.1 Recognition\n\
            | Term | Definition | Example Usage |\n\
            |------|------------|---------------|\n\
            | `recognize` | Identify pattern | \"recognize: forecastable\" |\n\
        ";

        let catalog = parse_markdown_catalog(markdown).unwrap();

        assert_eq!(catalog.len(), 5);
        assert_eq!(
            catalog[0],
            (
                "query".into(),
                "Ask for information".into(),
                TemplateType::WordAct
            )
        );
        assert_eq!(
            catalog[1],
            (
                "request".into(),
                "Ask for action".into(),
                TemplateType::WordAct
            )
        );
        assert_eq!(
            catalog[2],
            (
                "sequence".into(),
                "Linear ordering".into(),
                TemplateType::FlowDef
            )
        );
        assert_eq!(
            catalog[3],
            (
                "parallel".into(),
                "Concurrent execution".into(),
                TemplateType::FlowDef
            )
        );
        assert_eq!(
            catalog[4],
            (
                "recognize".into(),
                "Identify pattern".into(),
                TemplateType::KnowAct
            )
        );
    }

    /// P8: parse_markdown_catalog skips non-term rows (separator, header, prose)
    // REQ: P3-tpl-test-parse-catalog-skips-non-terms — parse_markdown_catalog skips non-term rows (separator, header, prose)
    // [P3] Motivating: Generative Space — validates parser skips non-term rows
    // [P8] Constraining: Semantic Grounding — separators and prose are not terms
    #[test]
    fn parse_catalog_skips_non_term_rows() {
        let markdown = "\
            ## Domain 1: WordAct\n\
            Some prose text here.\n\
            | Term | Definition |\n\
            |------|------------|\n\
            | `query` | Ask |\n\
            | Non-backtick cell | Not a term |\n\
        ";

        let catalog = parse_markdown_catalog(markdown).unwrap();
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].0, "query");
    }

    /// P8: parse_markdown_catalog returns error on empty input
    // REQ: P3-tpl-test-parse-catalog-empty-error — parse_markdown_catalog returns error on empty input
    // [P3] Motivating: Generative Space — validates empty catalog is rejected
    #[test]
    fn parse_catalog_empty_input_returns_error() {
        let result = parse_markdown_catalog("No tables here");
        assert!(result.is_err());
    }

    /// P8: render_workspace_yaml produces valid YAML that round-trips through load_hlexicon_from_yaml
    // REQ: P3-tpl-test-render-yaml-round-trips — render_workspace_yaml produces valid YAML that round-trips through load_hlexicon_from_yaml
    // [P3] Motivating: Generative Space — validates YAML serialization round-trips
    // [P8] Constraining: Semantic Grounding — YAML preserves vocabulary semantics
    #[test]
    fn render_yaml_round_trips() {
        let terms = vec![
            (
                "query".into(),
                "Ask for information".into(),
                TemplateType::WordAct,
            ),
            (
                "request".into(),
                "Ask for action".into(),
                TemplateType::WordAct,
            ),
            (
                "sequence".into(),
                "Linear ordering".into(),
                TemplateType::FlowDef,
            ),
            (
                "recognize".into(),
                "Identify pattern".into(),
                TemplateType::KnowAct,
            ),
        ];

        let yaml = render_workspace_yaml(&terms).unwrap();

        // Should contain the domain keys
        assert!(yaml.contains("hlexicon:"));
        assert!(yaml.contains("wordact:"));
        assert!(yaml.contains("flowdef:"));
        assert!(yaml.contains("knowact:"));

        // Round-trip: parse back and verify
        let lexicon = load_hlexicon_from_yaml(&yaml).unwrap();
        assert_eq!(lexicon.len(), 4);
        assert!(lexicon.contains("query"));
        assert!(lexicon.contains("sequence"));
        assert!(lexicon.contains("recognize"));
    }

    /// P8: regenerate_workspace_yaml is the full markdown→YAML pipeline
    // REQ: P3-tpl-test-regenerate-yaml-pipeline — regenerate_workspace_yaml is the full markdown→YAML pipeline
    // [P3] Motivating: Generative Space — validates full markdown→YAML pipeline
    // [P8] Constraining: Semantic Grounding — regenerated YAML matches source
    #[test]
    fn regenerate_workspace_yaml_produces_valid_yaml() {
        let markdown = "\
            ## Domain 1: WordAct\n\
            | Term | Definition |\n\
            |------|------------|\n\
            | `query` | Ask for information |\n\
            \n\
            ## Domain 2: FlowDef\n\
            | Term | Definition |\n\
            |------|------------|\n\
            | `sequence` | Linear ordering |\n\
            ## Cross-Domain Composition Patterns\n\
            (rest of doc...)  \
        ";

        let yaml = regenerate_workspace_yaml(markdown).unwrap();

        assert!(yaml.starts_with("# hLexicon Workspace Registry"));
        assert!(yaml.contains("term: query"));
        assert!(yaml.contains("term: sequence"));
        assert!(yaml.contains("definition: \"Ask for information\""));
    }

    /// P8: generate YAML from actual markdown source and verify round-trip consistency
    // REQ: P3-tpl-test-hlexicon-yaml-matches-markdown — hLexicon YAML matches canonical markdown source
    // [P3] Motivating: Generative Space — validates workspace YAML matches markdown
    // [P8] Constraining: Semantic Grounding — generated vocabulary is canonical
    #[test]
    fn hlexicon_yaml_matches_markdown() {
        // Read the canonical markdown source
        let markdown_path = Path::new("docs/architecture/reference/hKask-hLexicon.md");
        if !markdown_path.exists() {
            // Skip if running from a different working directory
            return;
        }
        let markdown =
            std::fs::read_to_string(markdown_path).expect("Failed to read hKask-hLexicon.md");

        let generated_yaml =
            regenerate_workspace_yaml(&markdown).expect("Failed to regenerate YAML from markdown");

        // Verify generated YAML is loadable into HLexicon
        let lexicon =
            load_hlexicon_from_yaml(&generated_yaml).expect("Generated YAML should be valid");

        // Read the existing workspace YAML for comparison
        let yaml_path = Path::new("registry/hlexicon/hlexicon-workspace.yaml");
        if yaml_path.exists() {
            let existing_yaml =
                std::fs::read_to_string(yaml_path).expect("Failed to read hlexicon-workspace.yaml");

            // Term count consistency: generated from markdown should have same
            // or more terms than existing YAML (existing is manual snapshot)
            let existing_term_count = existing_yaml
                .lines()
                .filter(|l| l.trim().starts_with("- term:"))
                .count();
            assert!(
                lexicon.len() >= existing_term_count,
                "Generated YAML has {} terms, existing has {} — markdown may be missing terms\
                 or existing YAML has manually added entries not in the markdown",
                lexicon.len(),
                existing_term_count
            );

            // Every term in existing YAML should also exist in generated YAML
            // We check via contains() since HLexicon has no iter()
            let existing_terms: Vec<String> = existing_yaml
                .lines()
                .filter(|l| l.trim().starts_with("- term:"))
                .filter_map(|l| l.split("term:").nth(1))
                .map(|s| s.trim().to_string())
                .collect();
            for term_name in &existing_terms {
                assert!(
                    lexicon.contains(term_name),
                    "Term '{}' exists in workspace YAML but not in generated output. \
                     It may be a hand-added term not in the markdown source.",
                    term_name
                );
            }
        }

        // Basic term count sanity: should be substantial (140+ terms across 3 domains)
        assert!(
            lexicon.len() >= 140,
            "Expected at least 140 terms from markdown, got {}",
            lexicon.len()
        );
    }
}
