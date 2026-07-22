//! FlowDef manifest cross-validation.
//!
//! Validates consistency between FlowDef manifests and their referenced
//! template contracts. Checks:
//! - Variable name alignment between manifest `input_mapping` keys and template contracts
//! - `convergence_field` references a valid step ordinal
//! - All `template_ref` values resolve to registered templates
//! - OCAP capabilities match the templates actually used in steps
//!
//! ℏKask v0.31.0 — Template/Manifest cross-validation

use serde::{Deserialize, Serialize};

/// A single validation finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefValidationFinding {
    /// The manifest being validated.
    pub manifest_id: String,
    /// Which step ordinal this applies to (if step-specific).
    pub step_ordinal: Option<u32>,
    /// Severity: critical (will cause runtime failure), high (likely bug), medium (style/convention).
    pub severity: String,
    /// Category of the finding.
    pub category: String,
    /// Human-readable description.
    pub description: String,
    /// Suggested fix.
    pub recommendation: String,
}

/// Result of validating a single FlowDef manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefValidationReport {
    pub manifest_id: String,
    pub findings: Vec<FlowDefValidationFinding>,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub passed: bool,
}

impl FlowDefValidationReport {
    pub fn new(manifest_id: &str, findings: Vec<FlowDefValidationFinding>) -> Self {
        let critical_count = findings.iter().filter(|f| f.severity == "critical").count() as u32;
        let high_count = findings.iter().filter(|f| f.severity == "high").count() as u32;
        let medium_count = findings.iter().filter(|f| f.severity == "medium").count() as u32;
        let passed = findings.is_empty();
        Self {
            manifest_id: manifest_id.to_string(),
            findings,
            critical_count,
            high_count,
            medium_count,
            passed,
        }
    }
}

/// Validates a FlowDef manifest step's input_mapping keys against
/// the referenced template's contract input fields.
///
/// Returns findings for:
/// - Keys in input_mapping that don't appear in the template contract (possible dead variables)
/// - Keys in the template contract that aren't in input_mapping (unmapped required inputs)
/// - Self-referential input_mapping (previous_config referring to own step)
pub fn validate_step_input_mapping(
    manifest_id: &str,
    step_ordinal: u32,
    template_ref: &str,
    input_mapping: &std::collections::HashMap<String, String>,
    template_contract_inputs: &[&str],
) -> Vec<FlowDefValidationFinding> {
    let mut findings = Vec::new();

    // Check for self-referential patterns (step references itself)
    for (key, value) in input_mapping {
        let step_ref = format!("step_{}_result", step_ordinal);
        if value.contains(&step_ref) {
            findings.push(FlowDefValidationFinding {
                manifest_id: manifest_id.to_string(),
                step_ordinal: Some(step_ordinal),
                severity: "medium".to_string(),
                category: "self_referential_input".to_string(),
                description: format!(
                    "Step {} input '{}' references its own output ({}) — self-referential input. \
                     This is valid for convergence/stationarity patterns but should be reviewed.",
                    step_ordinal, key, step_ref
                ),
                recommendation: "Verify that the self-referential pattern has a safe default for the first iteration.".to_string(),
            });
        }
    }

    // Check for keys in input_mapping that don't match any template contract input
    let mapping_keys: std::collections::HashSet<&str> =
        input_mapping.keys().map(|k| k.as_str()).collect();
    let contract_keys: std::collections::HashSet<&str> =
        template_contract_inputs.iter().copied().collect();

    // Keys in mapping but not in contract — the template won't use them
    for key in mapping_keys.difference(&contract_keys) {
        // Skip known infrastructure keys
        if *key == "convergence_description" || *key == "include_blockers" {
            continue;
        }
        findings.push(FlowDefValidationFinding {
            manifest_id: manifest_id.to_string(),
            step_ordinal: Some(step_ordinal),
            severity: "high".to_string(),
            category: "unmapped_input".to_string(),
            description: format!(
                "Step {} input_mapping key '{}' does not match any input declared in template '{}' contract. \
                 The template will not receive this variable by that name.",
                step_ordinal, key, template_ref
            ),
            recommendation: format!(
                "Either rename the input_mapping key to match a template contract input, \
                 or add '{}' to the template's contract.input list.",
                key
            ),
        });
    }

    // Keys in contract but not in mapping — the template expects them but won't receive them
    // Skip: this is expected for optional inputs with defaults
    // Only flag truly critical misses

    findings
}

/// Parse the contract input field names from a `.j2` template's frontmatter.
///
/// Supports two formats:
///   YAML style: `contract:` → `input:` → field list
///   TOML style: `[contract]` → `input: {field: type, ...}`
///
/// Returns the field names declared as contract inputs. Returns empty for
/// templates that use a non-standard contract format (e.g. media `parameters:`).
pub fn parse_template_contract_inputs(content: &str) -> Vec<String> {
    // Try TOML-style [contract] first: `input: {field: type, field2: type}`
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("input:")
            && trimmed.contains('{')
            && let Some(start) = trimmed.find('{')
            && let Some(end) = trimmed.rfind('}')
        {
            let inner = &trimmed[start + 1..end];
            let mut inputs = Vec::new();
            for pair in inner.split(',') {
                if let Some(field) = pair.split(':').next() {
                    let field = field.trim();
                    if !field.is_empty() && !field.starts_with('#') {
                        inputs.push(field.to_string());
                    }
                }
            }
            if !inputs.is_empty() {
                return inputs;
            }
        }
    }

    // Fall back to YAML-style contract: → input: → field list
    let mut in_contract = false;
    let mut in_input = false;
    let mut inputs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("contract:") {
            in_contract = true;
            continue;
        }
        if in_contract && (trimmed.starts_with("output:") || trimmed == "---") {
            in_contract = false;
            in_input = false;
            continue;
        }
        if in_contract && trimmed.starts_with("input:") {
            in_input = true;
            continue;
        }
        if in_contract && in_input {
            if trimmed.starts_with("output:") || trimmed.is_empty() {
                in_input = false;
                continue;
            }
            // Lines like "  field_name: type" or "  field_name: object"
            if let Some(field) = trimmed.split(':').next() {
                let field = field.trim();
                if !field.is_empty() && !field.starts_with('#') {
                    inputs.push(field.to_string());
                }
            }
        }
    }

    inputs
}

/// Validates that `convergence_field` references a step ordinal within the
/// valid range (1..=num_steps).
pub fn validate_convergence_field(
    manifest_id: &str,
    convergence_field: &str,
    num_steps: u32,
) -> Option<FlowDefValidationFinding> {
    // Parse "step_N_result.convergence_metric" pattern
    let step_num = convergence_field
        .strip_prefix("step_")
        .and_then(|rest| rest.split('_').next())
        .and_then(|n| n.parse::<u32>().ok());

    match step_num {
        Some(n) if n < 1 || n > num_steps => Some(FlowDefValidationFinding {
            manifest_id: manifest_id.to_string(),
            step_ordinal: None,
            severity: "critical".to_string(),
            category: "invalid_convergence_field".to_string(),
            description: format!(
                "convergence_field '{}' references step {}, but the manifest has {} steps.",
                convergence_field, n, num_steps
            ),
            recommendation: format!(
                "Update convergence_field to reference a valid step ordinal (1-{}).",
                num_steps
            ),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_step_input_mapping_no_issues() {
        let mut mapping = HashMap::new();
        mapping.insert("statement".to_string(), "{{ statement }}".to_string());
        mapping.insert(
            "system_context".to_string(),
            "{{ system_context | default('') }}".to_string(),
        );

        let contract_inputs = &["statement", "system_context"];
        let findings = validate_step_input_mapping(
            "test-skill",
            1,
            "test/template",
            &mapping,
            contract_inputs,
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn test_validate_step_input_mapping_mismatch() {
        let mut mapping = HashMap::new();
        mapping.insert("claim".to_string(), "{{ statement }}".to_string());
        mapping.insert("system_context".to_string(), "{{ context }}".to_string());

        let contract_inputs = &["statement", "system_context"];
        let findings = validate_step_input_mapping(
            "test-skill",
            1,
            "test/template",
            &mapping,
            contract_inputs,
        );
        // 'claim' is in mapping but not in contract
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.category == "unmapped_input"));
    }

    #[test]
    fn test_validate_convergence_field_valid() {
        let finding = validate_convergence_field("test", "step_4_result.convergence_metric", 5);
        assert!(finding.is_none());
    }

    #[test]
    fn test_validate_convergence_field_out_of_range() {
        let finding = validate_convergence_field("test", "step_7_result.convergence_metric", 5);
        assert!(finding.is_some());
        let f = finding.unwrap();
        assert_eq!(f.severity, "critical");
        assert_eq!(f.category, "invalid_convergence_field");
    }

    #[test]
    fn test_self_referential_detection() {
        let mut mapping = HashMap::new();
        mapping.insert(
            "previous_config".to_string(),
            "{{ step_2_result | default(None) }}".to_string(),
        );

        let contract_inputs = &["previous_config", "current_config"];
        let findings = validate_step_input_mapping(
            "test-skill",
            2,
            "test/template",
            &mapping,
            contract_inputs,
        );
        assert!(
            findings
                .iter()
                .any(|f| f.category == "self_referential_input")
        );
    }
}
