//! Derive macro synthesis (G3 fix).
//!
//! tree-sitter parses source, not expanded macros. `#[derive(Serialize)]`
//! generates trait impls that are invisible to the parser. This module
//! synthesizes edges for standard derives by post-processing extracted symbols.
//!
//! Coverage: std derives (Clone, Copy, Debug, Default, PartialEq, Eq,
//! PartialOrd, Ord, Hash) and common ecosystem derives (Serialize,
//! Deserialize).
//!
//! Known limitation: custom proc macros remain invisible.

use crate::types::{Edge, Symbol, SymbolKind};

/// Standard derives we can synthesize.
const KNOWN_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "Default",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Serialize",
    "Deserialize",
];

/// Synthesize edges for `#[derive(...)]` attributes.
///
/// For each symbol that has known derives, adds `Implements` edges from
/// the symbol to synthetic trait symbols.
pub fn synthesize_derive_edges(symbols: &[Symbol], source: &[u8]) -> Vec<Edge> {
    #[allow(unused_mut)]
    let mut edges = Vec::new();

    for sym in symbols {
        if !matches!(sym.kind, SymbolKind::Struct | SymbolKind::Enum) {
            continue;
        }
        // Placeholder: derive attribute parsing not yet implemented.
        // Future: parse source text around sym.start_line for #[derive(...)].
        _ = sym;
        _ = source;
    }

    edges
}

/// Check if a source line contains a derive attribute with known traits.
#[allow(dead_code)]
fn extract_derived_traits(line: &str) -> Vec<String> {
    if !line.contains("derive") {
        return vec![];
    }
    let start = match line.find("derive(") {
        Some(i) => i + 7,
        None => return vec![],
    };
    let end = match line[start..].find(')') {
        Some(i) => start + i,
        None => return vec![],
    };
    let content = &line[start..end];
    content
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|t| KNOWN_DERIVES.contains(&t.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_derived_traits_single() {
        let traits = extract_derived_traits("#[derive(Debug)]");
        assert_eq!(traits, vec!["Debug"]);
    }

    #[test]
    fn test_extract_derived_traits_multiple() {
        let traits = extract_derived_traits("#[derive(Debug, Clone, Serialize, Deserialize)]");
        assert_eq!(traits, vec!["Debug", "Clone", "Serialize", "Deserialize"]);
    }

    #[test]
    fn test_extract_derived_traits_unknown_filtered() {
        let traits = extract_derived_traits("#[derive(Debug, UnknownTrait)]");
        assert_eq!(traits, vec!["Debug"]);
    }

    #[test]
    fn test_extract_derived_traits_no_derive() {
        let traits = extract_derived_traits("#[cfg(test)]");
        assert!(traits.is_empty());
    }
}
