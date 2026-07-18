use hkask_ports::registry::RegistryEntry;

/// Known vocabulary terms — bootstrapped from manifest `lexicon_terms` across the skill corpus.
///
/// Terms are sorted alphabetically for binary-search lookup.
/// New terms should be added in sorted order.
const KNOWN_TERMS: &[&str] = &[
    "abduct",
    "accept",
    "acknowledge",
    "act",
    "adapt",
    "adjust",
    "admit",
    "advise",
    "affirm",
    "aggregate",
    "agree",
    "alert",
    "align",
    "amplify",
    "analogy",
    "analyze",
    "anchor",
    "answer",
    "apply",
    "assemble",
    "assert",
    "assess",
    "audit",
    "backup",
    "baseline",
    "batch",
    "bind",
    "branch",
    "budget",
    "build",
    "calculate",
    "calibrate",
    "calibration",
    "capability",
    "capture",
    "catalog",
    "categorize",
    "chain",
    "challenge",
    "charter",
    "check",
    "choice",
    "cite",
    "claim",
    "clarify",
    "classify",
    "coach",
    "collect",
    "combine",
    "command",
    "commit",
    "commoditize",
    "compact",
    "compare",
    "complete",
    "complexity",
    "component",
    "compose",
    "compress",
    "compute",
    "condition",
    "confidence",
    "configure",
    "consent",
    "consistency",
    "consolidate",
    "constrain",
    "construct",
    "context",
    "contextualise",
    "contradiction",
    "converge",
    "coordinate",
    "correlate",
    "corroborate",
    "counterfactual",
    "coverage",
    "create",
    "critique",
    "crystallize",
    "cultivate",
    "curate",
    "dead_code",
    "decide",
    "declare",
    "decompose",
    "deduce",
    "deepen",
    "defend",
    "defer",
    "delegate",
    "deliver",
    "dependency",
    "deprecate",
    "derive",
    "describe",
    "design",
    "detach",
    "detect",
    "diagnose",
    "dialogue",
    "discover",
    "discriminate",
    "dispatch",
    "distill",
    "divergence",
    "diversify",
    "divest",
    "document",
    "drift",
    "elicit",
    "eliminate",
    "embed",
    "emit",
    "encode",
    "encourage",
    "endure",
    "enforce",
    "enumerate",
    "escalate",
    "estimate",
    "evaluate",
    "evolution",
    "evolve",
    "execute",
    "exercise",
    "expect",
    "expectation",
    "experiment",
    "explain",
    "explore",
    "export",
    "extract",
    "fact",
    "falsify",
    "feedback",
    "finalize",
    "find",
    "fix",
    "flag",
    "focus",
    "formalize",
    "format",
    "frame",
    "gap",
    "gate",
    "generate",
    "goal",
    "ground",
    "guide",
    "habit",
    "handoff",
    "heal",
    "hunt",
    "hypothesise",
    "hypothesize",
    "identify",
    "identity",
    "impact",
    "import",
    "improve",
    "improvise",
    "index",
    "infer",
    "install",
    "instruct",
    "instrument",
    "integrate",
    "interpret",
    "intervene",
    "inventory",
    "invert",
    "invest",
    "invoke",
    "isolate",
    "iterate",
    "iteration",
    "justify",
    "label",
    "learn",
    "link",
    "list",
    "locate",
    "map",
    "match",
    "maturity",
    "measure",
    "merge",
    "migrate",
    "monitor",
    "movement",
    "multi_step",
    "mutate",
    "normalize",
    "observe",
    "obstacle",
    "ontology",
    "organize",
    "orient",
    "package",
    "parse",
    "partition",
    "perceive",
    "persona",
    "perspective",
    "pipeline",
    "plan",
    "populate",
    "position",
    "practice",
    "predict",
    "present",
    "principal",
    "principles",
    "prioritize",
    "probe",
    "produce",
    "prompt",
    "propose",
    "quality",
    "query",
    "question",
    "rank",
    "read",
    "reason",
    "recall",
    "recognize",
    "recombine",
    "recommend",
    "reconcile",
    "record",
    "recover",
    "redact",
    "reduce",
    "reference",
    "refine",
    "reflect",
    "regulate",
    "reject",
    "relinquish",
    "remember",
    "remind",
    "render",
    "renounce",
    "report",
    "reproduce",
    "request",
    "require",
    "resolve",
    "respond",
    "restore",
    "retrieve",
    "retry",
    "reverse",
    "review",
    "revise",
    "rotate",
    "route",
    "routine",
    "rule_out",
    "sample",
    "scaffold",
    "scenario",
    "score",
    "search",
    "select",
    "sequence",
    "serialize",
    "shape",
    "simplify",
    "simulate",
    "slice",
    "specify",
    "standardize",
    "strategize",
    "structure",
    "suggest",
    "summarize",
    "surface",
    "survey",
    "switch",
    "synthesize",
    "tag",
    "target",
    "taxonomize",
    "tension",
    "test",
    "topology",
    "trace",
    "track",
    "transform",
    "transient",
    "transition",
    "translate",
    "traverse",
    "tune",
    "undertake",
    "update",
    "validate",
    "value_chain",
    "variance",
    "vectorize",
    "verify",
    "voice",
    "walk",
    "weigh",
    "weight",
    "wire",
    "workflow",
    "write",
];

/// Is `term` a known vocabulary term?
///
/// expect: "The system validates template contracts against the lexicon"
/// pre:  term may be any string
/// post: returns true if term is in KNOWN_TERMS
pub fn is_known(term: &str) -> bool {
    KNOWN_TERMS.binary_search(&term).is_ok()
}

/// Returns unknown terms from `terms` that are not in the vocabulary.
///
/// expect: "The system validates template contracts against the lexicon"
/// pre:  terms is a slice of declared lexicon terms
/// post: returns Vec of terms not found in KNOWN_TERMS
pub fn unrecognized(terms: &[String]) -> Vec<String> {
    terms.iter().filter(|t| !is_known(t)).cloned().collect()
}

/// Validate an entry's `lexicon_terms` against the known vocabulary.
/// Returns warnings for any unrecognized terms.
///
/// expect: "The system validates template contracts against the lexicon"
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
