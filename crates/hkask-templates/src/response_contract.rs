//! Response contract assertions for cross-MVSS boundary verification.
//!
//! When an MCP tool call returns an unexpected response shape, this module
//! detects the drift and emits a `cns.spec.drift` event. The consumer's
//! spec governs — mismatches are surfaced for observability, not enforced
//! as hard errors.

use std::collections::HashSet;

/// Drift between a declared response contract and the actual response shape.
///
/// Uses Jaccard distance (0.0 = no drift, 1.0 = full drift) to quantify
/// mismatch magnitude, mirroring the T11 drift infrastructure in
/// `hkask-storage::spec_types::DriftReport` but with field-oriented naming
/// suited to JSON response verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseDrift {
    /// Jaccard distance between declared and actual fields (0.0 = no drift, 1.0 = full drift).
    pub drift_magnitude: f64,
    /// Required fields the contract declares but the response does not contain.
    pub missing_fields: Vec<String>,
    /// Fields present in the response but not declared in the contract.
    pub extra_fields: Vec<String>,
}

impl ResponseDrift {
    /// Whether any drift was detected.
    pub fn has_drift(&self) -> bool {
        self.drift_magnitude > 0.0
    }
}

/// A response contract defines the expected shape of MCP tool responses.
///
/// Each contract specifies which response fields are required and what
/// fields are expected but optional. This enables drift detection between
/// the consumer's specification and the actual response shape.
///
/// The contract is assertive, not restrictive: drift is surfaced for
/// observability (via `cns.spec.drift` span events), never enforced as
/// a hard error. The consumer's spec governs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseContract {
    /// The tool/server this contract applies to.
    pub tool_name: String,
    /// Required top-level fields in the response.
    pub required_fields: Vec<String>,
    /// Fields that are expected but optional (warning if missing, not error).
    pub optional_fields: Vec<String>,
}

impl ResponseContract {
    /// Create a new, empty contract for the given tool.
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            required_fields: vec![],
            optional_fields: vec![],
        }
    }

    /// Add a required field to the contract (builder pattern).
    pub fn with_required_field(mut self, field: &str) -> Self {
        self.required_fields.push(field.to_string());
        self
    }

    /// Add an optional field to the contract (builder pattern).
    pub fn with_optional_field(mut self, field: &str) -> Self {
        self.optional_fields.push(field.to_string());
        self
    }

    /// Assert the contract against a response. Returns a `ResponseDrift` describing
    /// any mismatches between the expected and actual response shape.
    ///
    /// This does NOT fail on drift — it surfaces mismatches for observability.
    /// The consumer's spec governs; this is a detection mechanism.
    pub fn assert_response(&self, response: &serde_json::Value) -> ResponseDrift {
        let actual_fields: HashSet<String> = match response {
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            _ => HashSet::new(),
        };

        let declared_fields: HashSet<String> = self
            .required_fields
            .iter()
            .chain(self.optional_fields.iter())
            .cloned()
            .collect();

        let missing: Vec<String> = self
            .required_fields
            .iter()
            .filter(|f| !actual_fields.contains(*f))
            .cloned()
            .collect();

        let extra: Vec<String> = actual_fields
            .iter()
            .filter(|f| !declared_fields.contains(*f))
            .cloned()
            .collect();

        // No declared expectations → no drift regardless of response shape.
        // An empty contract means "I don't care about the shape".
        let drift_magnitude = if declared_fields.is_empty() {
            0.0
        } else {
            let intersection_count = declared_fields.intersection(&actual_fields).count() as f64;
            let union_count = declared_fields.union(&actual_fields).count() as f64;
            if union_count == 0.0 {
                0.0
            } else {
                1.0 - (intersection_count / union_count)
            }
        };

        ResponseDrift {
            drift_magnitude: drift_magnitude.clamp(0.0, 1.0),
            missing_fields: missing,
            extra_fields: extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_contract_empty_contract_matches_anything() {
        let contract = ResponseContract::new("test_tool");
        let response = serde_json::json!({"arbitrary": "fields"});
        let report = contract.assert_response(&response);
        assert_eq!(report.drift_magnitude, 0.0); // no requirements = no drift
    }

    #[test]
    fn response_contract_detects_missing_required_fields() {
        let contract = ResponseContract::new("test_tool")
            .with_required_field("content")
            .with_required_field("model");
        let response = serde_json::json!({"content": "hello"});
        let report = contract.assert_response(&response);
        assert!(report.missing_fields.contains(&"model".to_string()));
        assert!(report.drift_magnitude > 0.0);
    }

    #[test]
    fn response_contract_detects_extra_fields() {
        let contract = ResponseContract::new("test_tool").with_required_field("content");
        let response = serde_json::json!({"content": "hello", "extra": "field"});
        let report = contract.assert_response(&response);
        assert!(report.extra_fields.contains(&"extra".to_string()));
    }

    #[test]
    fn response_contract_exact_match_has_zero_drift() {
        let contract = ResponseContract::new("test_tool")
            .with_required_field("content")
            .with_optional_field("model");
        let response = serde_json::json!({"content": "hello", "model": "gpt-4"});
        let report = contract.assert_response(&response);
        assert_eq!(report.drift_magnitude, 0.0);
        assert!(report.missing_fields.is_empty());
        assert!(report.extra_fields.is_empty());
    }

    #[test]
    fn response_contract_non_object_response_is_full_drift() {
        let contract = ResponseContract::new("test_tool").with_required_field("content");
        let response = serde_json::json!("just a string");
        let report = contract.assert_response(&response);
        assert_eq!(report.drift_magnitude, 1.0);
        assert!(report.missing_fields.contains(&"content".to_string()));
    }

    #[test]
    fn response_contract_optional_field_missing_is_not_missing() {
        let contract = ResponseContract::new("test_tool").with_optional_field("model");
        let response = serde_json::json!({});
        let report = contract.assert_response(&response);
        // optional fields are included in declared_fields but NOT reported as missing
        assert!(report.missing_fields.is_empty());
        // union = {model} ∩ {} = ∅, intersection = ∅, drift = 1 - 0/1 = 1.0
        // BUT: missing only counts required, and extra only counts non-declared actuals.
        // Since optional "model" is declared but not present, actual={}, declared={model},
        // intersection=0, union=1, drift=1.0, but missing_fields only tracks required.
        // This is correct: the contract surface reports structural drift but
        // missing_fields specifically tracks required field violations.
        assert!(report.missing_fields.is_empty());
    }

    #[test]
    fn response_drift_has_drift_method() {
        let no_drift = ResponseDrift {
            drift_magnitude: 0.0,
            missing_fields: vec![],
            extra_fields: vec![],
        };
        assert!(!no_drift.has_drift());

        let some_drift = ResponseDrift {
            drift_magnitude: 0.5,
            missing_fields: vec!["field".to_string()],
            extra_fields: vec![],
        };
        assert!(some_drift.has_drift());
    }
}
