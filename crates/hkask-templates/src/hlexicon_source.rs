//! hLexicon source-of-truth alignment.
//!
//! Architectural lifecycle of the hLexicon (three file types, three lifecycles):
//!
//! 1. `docs/architecture/reference/hKask-hLexicon.md` — **canonical**, authored
//!    by humans. The single source of truth for the vocabulary.
//! 2. `registry/registries/hlexicon-workspace.yaml` — **derived data**. A
//!    committed artifact regenerated from the markdown. It has a data lifecycle:
//!    it can evolve and be customized (e.g. subsystem registries) in ways the
//!    compiled Rust cannot.
//! 3. Rust ([`hkask_types::lexicon`]) — **compiled types**, not user-editable.
//!
//! The markdown → YAML derivation is explicit and human-driven, never silent:
//! - [`parse_markdown_catalog`] reads the canonical markdown.
//! - [`render_workspace_yaml`] produces the derived YAML text.
//! - The `hlexicon_yaml_matches_markdown` test is a **consistency check**: it
//!   fails if the committed YAML and the markdown disagree, so the maintainer is
//!   asked to decide whether the markdown was corrupted (restore from git) or
//!   intentionally evolved (regenerate).
//! - The `regenerate_workspace_yaml` test is the **explicit, opt-in** regen step
//!   (`#[ignore]`d; run manually with `--ignored` only when you intend to update
//!   the YAML to reflect markdown evolution).

use hkask_types::lexicon::{Domain, HLexicon, LexiconTerm};

/// Canonical markdown catalog, embedded for the consistency check and the
/// explicit regeneration step. Only needed in test builds.
#[cfg(test)]
const CATALOG_MD: &str = include_str!("../../../docs/architecture/reference/hKask-hLexicon.md");

/// Committed derived YAML, embedded for the consistency check.
const WORKSPACE_YAML: &str = include_str!("../../../registry/registries/hlexicon-workspace.yaml");

/// Parse the canonical markdown catalog into ordered `(term, domain, definition)`.
///
/// Reads `## Domain N: <Name>` headings and the `| \`term\` | definition | ... |`
/// table rows beneath them, stopping at the alphabetical "Term Index" section so
/// each term is captured once. A term that appears in two domains (e.g.
/// `transform`) keeps its first/primary domain.
pub fn parse_markdown_catalog(md: &str) -> Vec<LexiconTerm> {
    let mut current: Option<Domain> = None;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();

    for line in md.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            if rest.starts_with("hLexicon Term Index") {
                break; // stop before the alphabetical index to avoid duplicates
            }
            current = parse_domain_heading(rest);
            continue;
        }
        let Some(domain) = current else { continue };
        if let Some((term, definition)) = parse_term_row(trimmed)
            && seen.insert(term.to_string())
        {
            out.push(LexiconTerm::new(term, domain, definition));
        }
    }
    out
}

/// Map a `Domain N: <Name> ...` heading to its [`Domain`].
fn parse_domain_heading(rest: &str) -> Option<Domain> {
    if !rest.starts_with("Domain ") {
        return None;
    }
    // "Domain 2: FlowDef — Process Flow Language (34 terms)" → "FlowDef"
    let after_colon = rest.split(':').nth(1)?.trim();
    let name = after_colon.split_whitespace().next()?;
    Domain::parse_str(name)
}

/// Parse a `| \`term\` | definition | example |` row into `(term, definition)`.
fn parse_term_row(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix('|')?;
    let mut cells = line.split('|');
    let term_cell = cells.next()?.trim();
    let term = term_cell.strip_prefix('`')?.strip_suffix('`')?;
    if term.is_empty() {
        return None;
    }
    let definition = cells.next()?.trim();
    Some((term, definition))
}

/// Render the derived workspace lexicon YAML from parsed terms.
///
/// Deterministic: terms are grouped by domain in markdown order, so regenerating
/// from an unchanged markdown yields byte-identical output.
pub fn render_workspace_yaml(terms: &[LexiconTerm]) -> String {
    let mut out = String::new();
    out.push_str("# hLexicon Workspace Registry\n");
    out.push_str("# DERIVED ARTIFACT — do not hand-edit term definitions.\n");
    out.push_str("# Canonical source: docs/architecture/reference/hKask-hLexicon.md\n");
    out.push_str(
        "# Regenerate after editing the markdown:\n\
         #   cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored\n",
    );
    out.push_str("# The hlexicon_yaml_matches_markdown test fails if this drifts.\n\n");
    out.push_str("hlexicon:\n");

    for (domain, key) in [
        (Domain::WordAct, "wordact"),
        (Domain::FlowDef, "flowdef"),
        (Domain::KnowAct, "knowact"),
    ] {
        out.push_str(&format!("  {key}:\n"));
        for t in terms.iter().filter(|t| t.domain == domain) {
            let def = t.definition.replace('"', "\\\"");
            out.push_str(&format!("    - term: {}\n", t.term));
            out.push_str(&format!("      definition: \"{def}\"\n"));
        }
    }
    out
}

/// Load the full canonical hLexicon from the committed workspace YAML.
///
/// This is the vocabulary template/specification validation should use in
/// production (as opposed to [`HLexicon::bootstrap`], a minimal fixture).
pub fn load_workspace_lexicon() -> Result<HLexicon, serde_yaml::Error> {
    parse_workspace_yaml(WORKSPACE_YAML)
}

fn parse_workspace_yaml(yaml: &str) -> Result<HLexicon, serde_yaml::Error> {
    #[derive(serde::Deserialize)]
    struct Entry {
        term: String,
        definition: String,
    }
    #[derive(serde::Deserialize)]
    struct Doc {
        hlexicon: std::collections::BTreeMap<String, Vec<Entry>>,
    }

    let doc: Doc = serde_yaml::from_str(yaml)?;
    let mut lexicon = HLexicon::new();
    for (domain_key, entries) in doc.hlexicon {
        let Some(domain) = Domain::parse_str(&domain_key) else {
            continue;
        };
        for e in entries {
            lexicon.add(LexiconTerm::new(&e.term, domain, &e.definition));
        }
    }
    Ok(lexicon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_parses_to_expected_counts() {
        let terms = parse_markdown_catalog(CATALOG_MD);
        let wordact = terms.iter().filter(|t| t.domain == Domain::WordAct).count();
        let flowdef = terms.iter().filter(|t| t.domain == Domain::FlowDef).count();
        let knowact = terms.iter().filter(|t| t.domain == Domain::KnowAct).count();
        // Catalog defines 87 term-slots (28/34/25) but `transform` is shared
        // between WordAct and FlowDef; it is keyed once under its first/primary
        // domain (WordAct), so the functional unique set is 28/33/25 = 86.
        assert_eq!(wordact, 28, "WordAct count drifted from catalog");
        assert_eq!(flowdef, 33, "FlowDef unique count drifted from catalog");
        assert_eq!(knowact, 25, "KnowAct count drifted from catalog");
        assert_eq!(
            terms.len(),
            86,
            "total unique term count drifted from catalog"
        );
    }

    /// Consistency check (rides `cargo test --workspace`).
    ///
    /// If this fails, the canonical markdown and the committed derived YAML
    /// disagree. Decide: was the markdown corrupted (restore from git), or
    /// intentionally evolved? If evolved, run the regeneration step:
    ///   cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored
    #[test]
    fn hlexicon_yaml_matches_markdown() {
        let expected = render_workspace_yaml(&parse_markdown_catalog(CATALOG_MD));
        assert_eq!(
            normalize(&expected),
            normalize(WORKSPACE_YAML),
            "hlexicon-workspace.yaml is out of sync with hKask-hLexicon.md — \
             regenerate intentionally or restore the markdown from git"
        );
    }

    /// Explicit, human-driven regeneration of the derived YAML. `#[ignore]`d so
    /// it never runs in normal CI; run it only when you intend to update the
    /// YAML to reflect an intentional markdown change:
    ///   cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored
    #[test]
    #[ignore = "explicit regeneration; run only to update the derived YAML"]
    fn regenerate_workspace_yaml() {
        let yaml = render_workspace_yaml(&parse_markdown_catalog(CATALOG_MD));
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../registry/registries/hlexicon-workspace.yaml"
        );
        std::fs::write(path, yaml).expect("write workspace lexicon yaml");
        eprintln!("regenerated {path}");
    }

    #[test]
    fn workspace_yaml_loads() {
        let lexicon = load_workspace_lexicon().expect("workspace lexicon parses");
        assert_eq!(lexicon.len(), 86);
        assert_eq!(
            lexicon.get("curate").map(|t| t.domain),
            Some(Domain::FlowDef)
        );
    }

    fn normalize(s: &str) -> String {
        s.replace("\r\n", "\n").trim_end().to_string()
    }
}
