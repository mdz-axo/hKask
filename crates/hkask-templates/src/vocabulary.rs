
use hkask_types::ports::registry::RegistryEntry;

/// Known vocabulary terms — bootstrapped from manifest `lexicon_terms` across the skill corpus.
///
/// Terms are sorted alphabetically for binary-search lookup.
/// New terms should be added in sorted order.
const KNOWN_TERMS: &[&str] = &[
    "abduct",
    "accept",
    "acknowledge",
    "adapt",
    "affirm",
    "aggregate",
    "align",
    "amplify",
    "analogy",
    "analyze",
    "apply",
    "assert",
    "assess",
    "audit",
    "calibrate",
    "calibration",
    "catalog",
    "challenge",
    "clarify",
    "classify",
    "command",
    "compact",
    "compare",
    "compose",
    "compress",
    "confidence",
    "consent",
    "constrain",
    "contextualise",
    "contradiction",
    "converge",
    "create",
    "critique",
    "crystallize",
    "cultivate",
    "curate",
    "decide",
    "declare",
    "decompose",
    "deduce",
    "deepen",
    "deprecate",
    "design",
    "detect",
    "discriminate",
    "distill",
    "document",
    "elicit",
    "enforce",
    "escalate",
    "evaluate",
    "execute",
    "exercise",
    "explore",
    "extract",
    "fix",
    "flag",
    "gap",
    "ground",
    "improvise",
    "infer",
    "install",
    "instrument",
    "integrate",
    "inventory",
    "isolate",
    "iterate",
    "iteration",
    "map",
    "match",
    "measure",
    "migrate",
    "monitor",
    "observe",
    "orient",
    "parse",
    "plan",
    "predict",
    "prioritize",
    "probe",
    "prompt",
    "propose",
    "query",
    "rank",
    "recognize",
    "recommend",
    "reconcile",
    "redact",
    "reduce",
    "reference",
    "reflect",
    "regulate",
    "reject",
    "report",
    "reproduce",
    "request",
    "require",
    "resolve",
    "restore",
    "review",
    "revise",
    "route",
    "score",
    "search",
    "select",
    "sequence",
    "simplify",
    "specify",
    "structure",
    "suggest",
    "summarize",
    "synthesize",
    "target",
    "trace",
    "transform",
    "translate",
    "undertake",
    "update",
    "validate",
    "verify",
    "walk",
    "wire",
    "write",
];

/// Is `term` a known vocabulary term?
///
/// expect: "The system validates template contracts against the lexicon" [P3]
/// pre:  term may be any string
/// post: returns true if term is in KNOWN_TERMS
pub fn is_known(term: &str) -> bool {
    KNOWN_TERMS.binary_search(&term).is_ok()
}

/// Returns unknown terms from `terms` that are not in the vocabulary.
///
/// expect: "The system validates template contracts against the lexicon" [P3]
/// pre:  terms is a slice of declared lexicon terms
/// post: returns Vec of terms not found in KNOWN_TERMS
pub fn unrecognized(terms: &[String]) -> Vec<String> {
    terms.iter().filter(|t| !is_known(t)).cloned().collect()
}

/// Validate an entry's `lexicon_terms` against the known vocabulary.
/// Returns warnings for any unrecognized terms.
///
/// expect: "The system validates template contracts against the lexicon" [P3]
/// pre:  entry is a valid RegistryEntry
/// post: returns Vec of warning strings for unrecognized terms
pub fn validate_entry(entry: &RegistryEntry) -> Vec<String> {
    let mut warnings = Vec::new();
    let unknown = unrecognized(&entry.lexicon_terms);
    for term in &unknown {
        warnings.push(format!(
            "entry '{}' declares unknown lexicon term '{}'",
            entry.id, term
        ));
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    // contract: tpl-vocab-test-sorted
    // expect: "Template KNOWN_TERMS list must maintain alphabetical order" [P3]
    #[test]
    fn known_terms_are_sorted() {
        for w in KNOWN_TERMS.windows(2) {
            assert!(
                w[0] < w[1],
                "KNOWN_TERMS not sorted: '{}' >= '{}'",
                w[0],
                w[1]
            );
        }
    }

    // contract: tpl-vocab-test-duplicates
    // expect: "Template KNOWN_TERMS list must not contain duplicates" [P3]
    #[test]
    fn known_terms_no_duplicates() {
        for w in KNOWN_TERMS.windows(2) {
            assert_ne!(w[0], w[1], "Duplicate term: '{}'", w[0]);
        }
    }

    // contract: P3-tpl-vocab-test-known
    // expect: "Template validate_terms passes known terms" [P3]
    #[test]
    fn validate_known_terms_passes() {
        let terms: Vec<String> = vec!["compose", "verify", "classify"]
            .into_iter()
            .map(String::from)
            .collect();
        let unknown = unrecognized(&terms);
        assert!(
            unknown.is_empty(),
            "Known terms should not be flagged: {:?}",
            unknown
        );
    }

    // contract: P3-tpl-vocab-test-unknown
    // expect: "Template validate_terms flags unknown terms" [P3]
    #[test]
    fn validate_unknown_terms_flags() {
        let terms: Vec<String> = vec![
            "compose".into(),
            "nonsense_term_xyz".into(),
            "verify".into(),
        ];
        let unknown = unrecognized(&terms);
        assert_eq!(unknown, vec!["nonsense_term_xyz".to_string()]);
    }

    // contract: P3-tpl-vocab-test-all-known
    // expect: "Template all manifest-derived terms are known" [P3]
    #[test]
    fn all_bootstrapped_terms_are_known() {
        for term in KNOWN_TERMS {
            assert!(
                is_known(term),
                "Bootstrapped term '{}' not recognized",
                term
            );
        }
    }

    // contract: P3-tpl-vocab-test-empty
    // expect: "Template empty terms produce no warnings" [P3]
    #[test]
    fn empty_terms_no_warnings() {
        let unknown = unrecognized(&[]);
        assert!(unknown.is_empty());
    }
}
