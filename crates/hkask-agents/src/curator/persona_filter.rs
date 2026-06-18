//! Persona constraint filter — enforces Curator behavioral constraints at inference boundary.
//!
//! The Curator persona's forbidden patterns (preamble, emoji, filler words like
//! "Great", "Certainly") are documented in `PersonaConstraints` but were never
//! enforced at runtime. This filter checks model output against the constraints
//! and reports violations.

use hkask_types::PersonaConstraints;

/// Result of checking model output against persona constraints.
#[derive(Debug, Clone)]
pub struct PersonaCheckResult {
    /// Whether the output passes all constraints.
    pub passed: bool,
    /// List of violations found (pattern → matched text).
    pub violations: Vec<(String, String)>,
}

/// Check model output against persona constraints.
///
/// Returns a `PersonaCheckResult` indicating whether the output passes and
/// listing any violations. The `forbidden` field of `PersonaConstraints`
/// \[NORMATIVE\] contains patterns that must not appear in Curator output. (P3 — Generative Space).
///
/// REQ: P9-agt-curator-persona-check
/// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
/// \[P9\] Motivating: Homeostatic Self-Regulation — persona filter prevents harmful output
/// \[P4\] Constraining: Clear Boundaries — forbidden patterns are explicit
/// pre:  `output` is a valid UTF-8 string (may be empty); `constraints`
///       is a valid `PersonaConstraints` with a non-empty `forbidden` list.
/// post: Returns a `PersonaCheckResult` with `passed = true` if no
///       forbidden patterns are found (case-insensitive substring match);
///       `passed = false` with a list of `(pattern, matched_text)`
///       violations otherwise. Does not panic on non-ASCII input.
pub fn check_persona_constraints(
    output: &str,
    constraints: &PersonaConstraints,
) -> PersonaCheckResult {
    let mut violations = Vec::new();

    for pattern in &constraints.forbidden {
        // Case-insensitive substring check
        let lower_output = output.to_lowercase();
        let lower_pattern = pattern.to_lowercase();
        if let Some(pos) = lower_output.find(&lower_pattern) {
            // Safe: pos and lower_pattern.len() are both byte indices into lower_output.
            let end = (pos + lower_pattern.len()).min(lower_output.len());
            let matched = lower_output[pos..end].to_string();
            violations.push((pattern.clone(), matched));
        }
    }

    PersonaCheckResult {
        passed: violations.is_empty(),
        violations,
    }
}

/// Strip forbidden patterns from model output.
///
/// Replaces each forbidden pattern occurrence with an empty string.
/// Returns the cleaned output and a list of violations that were stripped.
///
/// REQ: P9-agt-curator-persona-strip
/// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
/// \[P9\] Motivating: Homeostatic Self-Regulation — stripping reduces harm while preserving utility
/// pre:  `output` is a valid UTF-8 string; `constraints` is a valid
///       `PersonaConstraints` with a non-empty `forbidden` list.
/// post: Returns `(cleaned_output, violations)` where `cleaned_output`
///       has all forbidden patterns removed (case-insensitive, first
///       occurrence only per pattern) and `violations` lists what was
///       stripped. Does not panic on non-ASCII input.
pub fn strip_forbidden_patterns(
    output: &str,
    constraints: &PersonaConstraints,
) -> (String, Vec<(String, String)>) {
    let check = check_persona_constraints(output, constraints);
    let mut cleaned = output.to_string();

    for (pattern, _) in &check.violations {
        // Case-insensitive replacement. Persona constraint patterns are
        // ASCII, so to_lowercase() preserves byte length and positions
        // map directly between lowercased and original strings.
        let lower = cleaned.to_lowercase();
        let lower_pattern = pattern.to_lowercase();
        if let Some(pos) = lower.find(&lower_pattern) {
            let end = (pos + lower_pattern.len()).min(lower.len());
            cleaned = format!("{}{}", &cleaned[..pos], &cleaned[end..]);
        }
    }

    (cleaned, check.violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::PersonaConstraints;

    fn constraints(forbidden: &[&str]) -> PersonaConstraints {
        PersonaConstraints {
            forbidden: forbidden.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    // REQ: P4-agt-persona-filter-non-ascii-check-test — non-ASCII output does not panic on byte-boundary check
    /// expect: "Agent interactions are gated by OCAP boundaries" [P4]
    #[test]
    fn check_does_not_panic_on_non_ascii_output() {
        // 'é' is 2 bytes in UTF-8. The forbidden pattern "great" is ASCII.
        // Before fix, pos from to_lowercase() on a mixed-byte string was used
        // to index the original string, causing a byte-boundary panic.
        let c = constraints(&["great"]);
        let output = "C'est très Great de vous voir";
        // Must not panic; violation should be detected.
        let result = check_persona_constraints(output, &c);
        assert!(
            !result.passed,
            "should detect 'great' in mixed UTF-8 output"
        );
    }

    // REQ: P4-agt-persona-filter-non-ascii-strip-test — strip does not panic on non-ASCII output
    /// expect: "Agent interactions are gated by OCAP boundaries" [P4]
    #[test]
    fn strip_does_not_panic_on_non_ascii_output() {
        let c = constraints(&["great"]);
        let output = "C'est très Great de vous voir";
        let (cleaned, violations) = strip_forbidden_patterns(output, &c);
        assert!(!violations.is_empty(), "should report violation");
        assert!(!cleaned.contains("Great"), "should strip the pattern");
    }

    // REQ: P4-agt-persona-filter-ascii-detect-test — ASCII output: clean detection and stripping
    /// expect: "Agent interactions are gated by OCAP boundaries" [P4]
    #[test]
    fn check_detects_ascii_forbidden_pattern() {
        let c = constraints(&["Great", "Certainly"]);
        let result = check_persona_constraints("Great! Certainly.", &c);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 2);
    }

    // REQ: P4-agt-persona-filter-clean-test — no false positives on clean output
    /// expect: "Agent interactions are gated by OCAP boundaries" [P4]
    #[test]
    fn check_passes_clean_output() {
        let c = constraints(&["Great", "Certainly"]);
        let result = check_persona_constraints("The result is ready.", &c);
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }
}
