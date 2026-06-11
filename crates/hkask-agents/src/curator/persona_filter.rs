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
pub fn strip_forbidden_patterns(
    output: &str,
    constraints: &PersonaConstraints,
) -> (String, Vec<(String, String)>) {
    let check = check_persona_constraints(output, constraints);
    let mut cleaned = output.to_string();

    for (pattern, _) in &check.violations {
        // Case-insensitive replacement using lowercased indices
        let lower = cleaned.to_lowercase();
        let lower_pattern = pattern.to_lowercase();
        if let Some(pos) = lower.find(&lower_pattern) {
            let end = (pos + lower_pattern.len()).min(lower.len());
            // Map lowercased byte positions back to original via char walk.
            // Since to_lowercase() on ASCII changes no byte length, this
            // is a direct position map for persona constraint patterns.
            let (orig_pos, orig_end) = if lower.len() == cleaned.len() {
                (pos, end)
            } else {
                // Fallback: rebuild string without the matched range
                let mut result = String::with_capacity(cleaned.len());
                let mut byte_offset = 0;
                for c in cleaned.chars() {
                    if byte_offset < pos || byte_offset >= end {
                        result.push(c);
                    }
                    byte_offset += c.len_utf8();
                }
                cleaned = result;
                continue;
            };
            cleaned = format!("{}{}", &cleaned[..orig_pos], &cleaned[orig_end..]);
        }
    }

    (cleaned, check.violations)
}
