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
/// contains patterns that must not appear in Curator output.
pub fn check_persona_constraints(
    output: &str,
    constraints: &PersonaConstraints,
) -> PersonaCheckResult {
    let mut violations = Vec::new();

    for pattern in &constraints.forbidden {
        // Case-insensitive substring check
        if output.to_lowercase().contains(&pattern.to_lowercase()) {
            // Find the actual matched text (case-insensitive)
            let lower_output = output.to_lowercase();
            let lower_pattern = pattern.to_lowercase();
            if let Some(pos) = lower_output.find(&lower_pattern) {
                let end = (pos + pattern.len()).min(output.len());
                let matched = output[pos..end].to_string();
                violations.push((pattern.clone(), matched));
            }
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
pub fn strip_forbidden_patterns(
    output: &str,
    constraints: &PersonaConstraints,
) -> (String, Vec<(String, String)>) {
    let check = check_persona_constraints(output, constraints);
    let mut cleaned = output.to_string();

    for (pattern, _) in &check.violations {
        // Case-insensitive replacement
        let lower = cleaned.to_lowercase();
        let lower_pattern = pattern.to_lowercase();
        if let Some(pos) = lower.find(&lower_pattern) {
            let end = (pos + pattern.len()).min(cleaned.len());
            cleaned = format!("{}{}", &cleaned[..pos], &cleaned[end..]);
        }
    }

    (cleaned, check.violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_constraints() -> PersonaConstraints {
        PersonaConstraints {
            tone: "professional".to_string(),
            verbosity: "low".to_string(),
            formatting: "markdown".to_string(),
            forbidden: vec![
                "Great!".to_string(),
                "Certainly".to_string(),
                "I'd be happy to".to_string(),
            ],
            required: vec![],
        }
    }

    #[test]
    fn clean_output_passes() {
        let constraints = test_constraints();
        let result = check_persona_constraints("The variety deficit is 150.", &constraints);
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn forbidden_pattern_detected() {
        let constraints = test_constraints();
        let result = check_persona_constraints("Certainly, the deficit is 150.", &constraints);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].0, "Certainly");
    }

    #[test]
    fn strip_removes_forbidden() {
        let constraints = test_constraints();
        let (cleaned, violations) =
            strip_forbidden_patterns("Certainly, the deficit is 150.", &constraints);
        assert_eq!(cleaned, ", the deficit is 150.");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn multiple_violations_detected() {
        let constraints = test_constraints();
        let result = check_persona_constraints("Great! I'd be happy to help.", &constraints);
        assert!(!result.passed);
        assert!(result.violations.len() >= 2);
    }
}
