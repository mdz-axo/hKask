use hkask_rsolidity::contract;

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
#[contract(
    id = "P3-tpl-vocab-known — checks term membership via binary search",
    principle = "P3"
)]
pub fn is_known(term: &str) -> bool {
    KNOWN_TERMS.binary_search(&term).is_ok()
}

/// Returns unknown terms from `terms` that are not in the vocabulary.
///
#[contract(
    id = "P3-tpl-vocab-validate — validates terms against known vocabulary",
    principle = "P3"
)]
pub fn unrecognized(terms: &[String]) -> Vec<String> {
    terms.iter().filter(|t| !is_known(t)).cloned().collect()
}

/// Validate an entry's `lexicon_terms` against the known vocabulary.
/// Returns warnings for any unrecognized terms.
///
#[contract(
    id = "P3-tpl-vocab-validate-entry — validates entry lexicon_terms",
    principle = "P3"
)]
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
    #[test]
    fn known_terms_no_duplicates() {
        for w in KNOWN_TERMS.windows(2) {
            assert_ne!(w[0], w[1], "Duplicate term: '{}'", w[0]);
        }
    }

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

    #[test]
    fn empty_terms_no_warnings() {
        let unknown = unrecognized(&[]);
        assert!(unknown.is_empty());
    }
}
