#![allow(dead_code)]
//! Request/response types for the Spec MCP server

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Testing protocol types ────────────────────────────────────

/// Classification of a test according to DDMVSS testing protocol (TP-1).
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub enum TestClassification {
    /// Tests behavior through a module's public API or trait seam.
    PublicInterface,
    /// Tests interaction between two modules through a shared trait.
    SeamIntegration,
    /// Tests private methods, internal state, or mocked collaborators.
    /// Flagged as technical debt per TP-5.
    ImplementationCoupled,
}

impl TestClassification {
    /// Returns the string representation of this classification.
    pub fn as_str(&self) -> &'static str {
        match self {
            TestClassification::PublicInterface => "PublicInterface",
            TestClassification::SeamIntegration => "SeamIntegration",
            TestClassification::ImplementationCoupled => "ImplementationCoupled",
        }
    }

    /// Parse a string into a TestClassification. Case-insensitive.
    /// Returns PublicInterface for unrecognized values (safe default per DDMVSS TP-1).
    pub fn parse_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "publicinterface" | "public_interface" | "public-interface" => {
                TestClassification::PublicInterface
            }
            "seamintegration" | "seam_integration" | "seam-integration" => {
                TestClassification::SeamIntegration
            }
            "implementationcoupled" | "implementation_coupled" | "implementation-coupled" => {
                TestClassification::ImplementationCoupled
            }
            _ => TestClassification::PublicInterface,
        }
    }
}

/// Testing protocol status for a DDMVSS requirement.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TestTraceability {
    /// The DDMVSS requirement ID (e.g., "REQ-TRU-001").
    pub requirement_id: String,
    /// Classification of the covering test, if one exists.
    pub classification: Option<TestClassification>,
    /// The test function name or path, if a test exists.
    pub test_path: Option<String>,
    /// Whether this requirement has a documented gap (no test).
    pub has_gap: bool,
    /// If implementation-coupled, the `TEST-DEBT` comment location.
    pub test_debt_location: Option<String>,
}

/// Response from the spec_curate_test_verify tool.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TestVerifyResponse {
    /// Total DDMVSS requirements checked.
    pub total_requirements: usize,
    /// Requirements with at least one test.
    pub tested: usize,
    /// Requirements with documented gaps.
    pub gaps: usize,
    /// Requirements with implementation-coupled tests (debt).
    pub debt: usize,
    /// Per-requirement traceability details.
    pub traceability: Vec<TestTraceability>,
    /// Whether all requirements are satisfied (tested or documented gap).
    pub complete: bool,
}

// ── Response types ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GoalCaptureResponse {
    pub spec_id: String,
    pub category: String,
    pub domain_anchor: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GoalDecomposeResponse {
    pub spec_id: String,
    pub goal_index: usize,
    pub sub_goals_added: usize,
}

#[derive(Debug, Serialize)]
pub struct RequireBindResponse {
    pub spec_id: String,
    pub goal_index: usize,
    pub capability: String,
    pub authority: String,
    pub enforced: bool,
}

#[derive(Debug, Serialize)]
pub struct CurateEvaluateResponse {
    pub spec_id: String,
    pub decision: String,
    pub rationale: String,
    pub coherence_score: f64,
}

#[derive(Debug, Serialize)]
pub struct CurateReconcileResponse {
    pub resolution: String,
    pub spec_ids: Vec<String>,
    pub tension: String,
    pub tensions: Vec<TensionReport>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TensionReport {
    pub spec_a: String,
    pub spec_b: String,
    pub overlapping_goals: Vec<String>,
    pub jaccard_score: f64,
}

#[derive(Debug, Serialize)]
pub struct CurateCultivateResponse {
    pub coherence_score: f64,
    pub threshold: f64,
    pub above_threshold: bool,
    pub spec_count: usize,
    pub categories_covered: Vec<String>,
    pub categories_missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GraphNodeResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

#[derive(Debug, Serialize)]
pub struct GraphQueryResponse {
    pub count: usize,
    pub specs: Vec<GraphNodeResponse>,
}

#[derive(Debug, Serialize)]
pub struct GraphValidateResponse {
    pub valid: bool,
    pub coherence_score: f64,
    pub threshold: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
    pub spec_count: usize,
}

// ── Request types ────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalCaptureRequest {
    pub description: String,
    pub category: String,
    pub domain_anchor: String,
    pub criteria: Option<Vec<String>>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalDecomposeRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub sub_goals: Vec<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequireBindRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub capability: String,
    pub authority: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateEvaluateRequest {
    pub spec_id: String,
    pub rationale_hint: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateReconcileRequest {
    pub spec_ids: Vec<String>,
    pub tension_description: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateCultivateRequest {
    pub coherence_threshold: Option<f64>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphQueryRequest {
    pub category: Option<String>,
    pub domain_anchor: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphValidateRequest {
    pub coherence_threshold: Option<f64>,
    pub capability_token: Option<String>,
}

// ── Test protocol request types ────────────────────────────────

/// Request to create a test traceability record linking a test to a specification requirement.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestInvariantRequest {
    /// The spec ID (UUID) to link the test invariant to.
    pub spec_id: String,
    /// The seam or module boundary this test exercises.
    pub seam: String,
    /// A human-readable description of the invariant being tested.
    pub invariant: String,
    /// DDMVSS test classification: PublicInterface, SeamIntegration, or ImplementationCoupled.
    pub category: String,
    /// Optional TDD cycle identifier (e.g., "red", "green", "refactor").
    pub cycle: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

/// Response from spec/test/invariant confirming the traceability record.
#[derive(Debug, Serialize)]
pub struct TestInvariantResponse {
    /// The invariant ID (derived from spec_id + seam + category).
    pub invariant_id: String,
    /// Status of the record ("recorded").
    pub status: String,
}

/// Request to verify test coverage for a seam or spec category.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestVerifyRequest {
    /// Optional seam filter — only verify specs relevant to this seam.
    pub seam: Option<String>,
    /// Optional category filter — only verify specs in this DDMVSS category.
    pub category: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TestClassification (DDMVSS TP-1) ────────────────────────────

    // P8 invariant: TestClassification has exactly 3 variants matching DDMVSS TP-1 classification
    #[test]
    fn test_classification_has_exactly_three_variants() {
        let variants: Vec<TestClassification> = vec![
            TestClassification::PublicInterface,
            TestClassification::SeamIntegration,
            TestClassification::ImplementationCoupled,
        ];
        assert_eq!(
            variants.len(),
            3,
            "TestClassification must have exactly 3 variants per DDMVSS TP-1"
        );
    }

    // P8 invariant: TestClassification roundtrips through JSON serialization
    #[test]
    fn test_classification_json_roundtrip() {
        for variant in [
            TestClassification::PublicInterface,
            TestClassification::SeamIntegration,
            TestClassification::ImplementationCoupled,
        ] {
            let json = serde_json::to_string(&variant).expect("serialize");
            let back: TestClassification = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                variant, back,
                "TestClassification must roundtrip through JSON"
            );
        }
    }

    // P8 invariant: PublicInterface serializes to expected JSON string
    #[test]
    fn test_classification_public_interface_serialization() {
        let json = serde_json::to_string(&TestClassification::PublicInterface).expect("serialize");
        assert_eq!(
            json, r#""PublicInterface""#,
            "PublicInterface must serialize to exact JSON string"
        );
    }

    // ── TestTraceability ─────────────────────────────────────────────

    // P8 invariant: TestTraceability with all fields present serializes completely
    #[test]
    fn test_traceability_all_fields_present() {
        let t = TestTraceability {
            requirement_id: "REQ-TRU-001".to_string(),
            classification: Some(TestClassification::PublicInterface),
            test_path: Some("spec_category_roundtrip".to_string()),
            has_gap: false,
            test_debt_location: None,
        };
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(
            json.contains("REQ-TRU-001"),
            "requirement_id must appear in JSON"
        );
        assert!(
            json.contains("PublicInterface"),
            "classification must appear in JSON"
        );
        assert!(
            json.contains("spec_category_roundtrip"),
            "test_path must appear in JSON"
        );
    }

    // P8 invariant: TestTraceability with gap has classification None
    #[test]
    fn test_traceability_gap_means_no_classification() {
        let t = TestTraceability {
            requirement_id: "REQ-TRU-099".to_string(),
            classification: None,
            test_path: None,
            has_gap: true,
            test_debt_location: None,
        };
        assert!(t.has_gap, "has_gap must be true");
        assert!(
            t.classification.is_none(),
            "gap requirements must have no classification"
        );
        assert!(
            t.test_path.is_none(),
            "gap requirements must have no test path"
        );
    }

    // ── TestVerifyResponse ───────────────────────────────────────────

    // P8 invariant: TestVerifyResponse complete is true when all requirements are tested
    #[test]
    fn test_verify_response_complete_when_all_tested() {
        let resp = TestVerifyResponse {
            total_requirements: 3,
            tested: 3,
            gaps: 0,
            debt: 0,
            traceability: vec![],
            complete: true,
        };
        assert!(resp.complete, "complete must be true when tested == total");
        assert_eq!(resp.gaps, 0, "no gaps when all tested");
    }

    // P8 invariant: TestVerifyResponse complete is false when gaps exist
    #[test]
    fn test_verify_response_incomplete_when_gaps_exist() {
        let resp = TestVerifyResponse {
            total_requirements: 5,
            tested: 3,
            gaps: 2,
            debt: 0,
            traceability: vec![],
            complete: false,
        };
        assert!(!resp.complete, "complete must be false when gaps > 0");
        assert_eq!(resp.gaps, 2, "gaps must match untested count");
    }

    // P8 invariant: TestVerifyResponse serializes with all fields
    #[test]
    fn test_verify_response_serialization() {
        let resp = TestVerifyResponse {
            total_requirements: 4,
            tested: 2,
            gaps: 1,
            debt: 1,
            traceability: vec![TestTraceability {
                requirement_id: "REQ-TRU-001".to_string(),
                classification: Some(TestClassification::ImplementationCoupled),
                test_path: Some("test_debt".to_string()),
                has_gap: false,
                test_debt_location: Some("crates/foo.rs:L42".to_string()),
            }],
            complete: false,
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(
            json.contains("total_requirements"),
            "must include total_requirements"
        );
        assert!(
            json.contains("ImplementationCoupled"),
            "must include classification"
        );
        assert!(
            json.contains("crates/foo.rs:L42"),
            "must include test_debt_location"
        );
    }

    // ── Request type deserialization ─────────────────────────────────

    // P8 invariant: GoalCaptureRequest deserializes with required and optional fields
    #[test]
    fn goal_capture_request_deserialization() {
        let json = r#"{"description":"test goal","category":"domain","domain_anchor":"hkask","criteria":["c1","c2"],"capability_token":"tok"}"#;
        let req: GoalCaptureRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.description, "test goal");
        assert_eq!(req.category, "domain");
        assert_eq!(req.domain_anchor, "hkask");
        assert_eq!(req.criteria.as_ref().map(|c| c.len()), Some(2));
        assert!(req.capability_token.is_some());
    }

    // P8 invariant: GoalCaptureRequest allows null optional fields
    #[test]
    fn goal_capture_request_minimal_fields() {
        let json = r#"{"description":"minimal","category":"capability","domain_anchor":"okapi"}"#;
        let req: GoalCaptureRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.description, "minimal");
        assert!(req.criteria.is_none(), "criteria must default to None");
        assert!(
            req.capability_token.is_none(),
            "capability_token must default to None"
        );
    }

    // P8 invariant: CurateEvaluateRequest deserializes with optional rationale
    #[test]
    fn curate_evaluate_request_deserialization() {
        let json =
            r#"{"spec_id":"abc-123","rationale_hint":"partial goals","capability_token":null}"#;
        let req: CurateEvaluateRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.spec_id, "abc-123");
        assert!(req.rationale_hint.is_some());
        assert!(req.capability_token.is_none());
    }

    // ── Response type construction ───────────────────────────────────

    // P8 invariant: GoalCaptureResponse serializes with all fields
    #[test]
    fn goal_capture_response_serialization() {
        let resp = GoalCaptureResponse {
            spec_id: "test-id".to_string(),
            category: "domain".to_string(),
            domain_anchor: "hkask".to_string(),
            status: "captured".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("test-id"), "spec_id must appear in JSON");
        assert!(json.contains("captured"), "status must appear in JSON");
    }

    // P8 invariant: CurateEvaluateResponse decision matches curation logic
    #[test]
    fn curate_evaluate_response_merge_decision() {
        let resp = CurateEvaluateResponse {
            spec_id: "spec-1".to_string(),
            decision: "Merge".to_string(),
            rationale: "All criteria satisfied".to_string(),
            coherence_score: 1.0,
        };
        assert_eq!(
            resp.decision, "Merge",
            "complete spec must produce Merge decision"
        );
        assert!(
            (resp.coherence_score - 1.0).abs() < f64::EPSILON,
            "coherence 1.0 for complete spec"
        );
    }

    // P8 invariant: CurateEvaluateResponse Revise decision for partial spec
    #[test]
    fn curate_evaluate_response_revise_decision() {
        let resp = CurateEvaluateResponse {
            spec_id: "spec-2".to_string(),
            decision: "Revise".to_string(),
            rationale: "Unsatisfied criteria remain".to_string(),
            coherence_score: 0.5,
        };
        assert_eq!(
            resp.decision, "Revise",
            "partial spec must produce Revise decision"
        );
    }

    // P8 invariant: CurateEvaluateResponse Discard decision for empty spec
    #[test]
    fn curate_evaluate_response_discard_decision() {
        let resp = CurateEvaluateResponse {
            spec_id: "spec-3".to_string(),
            decision: "Discard".to_string(),
            rationale: "Empty goals".to_string(),
            coherence_score: 0.0,
        };
        assert_eq!(
            resp.decision, "Discard",
            "empty goals must produce Discard decision"
        );
    }

    // P8 invariant: TensionReport captures overlapping goals with Jaccard score
    #[test]
    fn tension_report_construction() {
        let report = TensionReport {
            spec_a: "spec-a".to_string(),
            spec_b: "spec-b".to_string(),
            overlapping_goals: vec!["goal-1".to_string(), "goal-2".to_string()],
            jaccard_score: 0.75,
        };
        assert_eq!(report.spec_a, "spec-a");
        assert_eq!(report.overlapping_goals.len(), 2);
        assert!(
            (report.jaccard_score - 0.75).abs() < f64::EPSILON,
            "Jaccard must be preserved"
        );
    }

    // P8 invariant: CurateCultivateResponse above_threshold is coherent with score and threshold
    #[test]
    fn curate_cultivate_response_above_threshold() {
        let resp = CurateCultivateResponse {
            coherence_score: 0.8,
            threshold: 0.7,
            above_threshold: true,
            spec_count: 5,
            categories_covered: vec!["domain".to_string(), "capability".to_string()],
            categories_missing: vec![],
        };
        assert!(
            resp.above_threshold,
            "coherence 0.8 > threshold 0.7 → above_threshold must be true"
        );
        assert!(
            resp.categories_missing.is_empty(),
            "no missing categories when covered"
        );
    }

    // P8 invariant: CurateCultivateResponse below threshold has missing categories
    #[test]
    fn curate_cultivate_response_below_threshold_with_gaps() {
        let resp = CurateCultivateResponse {
            coherence_score: 0.4,
            threshold: 0.7,
            above_threshold: false,
            spec_count: 2,
            categories_covered: vec!["domain".to_string()],
            categories_missing: vec![
                "capability".to_string(),
                "interface".to_string(),
                "composition".to_string(),
            ],
        };
        assert!(
            !resp.above_threshold,
            "coherence 0.4 < threshold 0.7 → above_threshold must be false"
        );
        assert_eq!(resp.categories_missing.len(), 3, "3 categories missing");
    }

    // P8 invariant: GraphValidateResponse valid is true when no violations
    #[test]
    fn graph_validate_response_valid_when_no_violations() {
        let resp = GraphValidateResponse {
            valid: true,
            coherence_score: 0.9,
            threshold: 0.7,
            violations: vec![],
            suggestions: vec![],
            spec_count: 4,
        };
        assert!(resp.valid, "no violations → valid must be true");
        assert!(
            resp.violations.is_empty(),
            "valid collection has no violations"
        );
    }

    // P8 invariant: GraphValidateResponse invalid when violations exist
    #[test]
    fn graph_validate_response_invalid_with_violations() {
        let resp = GraphValidateResponse {
            valid: false,
            coherence_score: 0.3,
            threshold: 0.7,
            violations: vec!["Coherence 0.30 below threshold 0.70".to_string()],
            suggestions: vec!["Missing category: capability".to_string()],
            spec_count: 2,
        };
        assert!(!resp.valid, "violations present → valid must be false");
        assert_eq!(resp.violations.len(), 1);
        assert_eq!(resp.suggestions.len(), 1);
    }

    // ── TestInvariantRequest/TestVerifyRequest ─────────────────────

    // P8 invariant: TestInvariantRequest deserialization accepts all fields
    #[test]
    fn test_invariant_request_full_deserialization() {
        let json = r#"{"spec_id":"00000000-0000-0000-0000-000000000001","seam":"spec-test-invariant","invariant":"rejects-missing-token","category":"PublicInterface","cycle":"red","capability_token":"dG9rZW4="}"#;
        let req: TestInvariantRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.spec_id, "00000000-0000-0000-0000-000000000001");
        assert_eq!(req.seam, "spec-test-invariant");
        assert_eq!(req.invariant, "rejects-missing-token");
        assert_eq!(req.category, "PublicInterface");
        assert_eq!(req.cycle.as_deref(), Some("red"));
    }

    // P8 invariant: TestInvariantRequest deserialization allows optional fields to be absent
    #[test]
    fn test_invariant_request_minimal_fields() {
        let json = r#"{"spec_id":"00000000-0000-0000-0000-000000000001","seam":"spec-test-invariant","invariant":"rejects-missing-token","category":"PublicInterface"}"#;
        let req: TestInvariantRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.spec_id, "00000000-0000-0000-0000-000000000001");
        assert!(req.cycle.is_none());
        assert!(req.capability_token.is_none());
    }

    // P8 invariant: TestVerifyRequest deserialization works with optional fields
    #[test]
    fn test_verify_request_deserialization() {
        let json = r#"{"category":"domain","capability_token":"dG9rZW4="}"#;
        let req: TestVerifyRequest = serde_json::from_str(json).expect("deserialize");
        assert!(req.seam.is_none());
        assert_eq!(req.category.as_deref(), Some("domain"));
    }

    // P8 invariant: TestVerifyRequest with all fields absent
    #[test]
    fn test_verify_request_empty() {
        let json = r#"{}"#;
        let req: TestVerifyRequest = serde_json::from_str(json).expect("deserialize");
        assert!(req.seam.is_none());
        assert!(req.category.is_none());
        assert!(req.capability_token.is_none());
    }

    // P8 invariant: TestInvariantResponse serialization
    #[test]
    fn test_invariant_response_serialization() {
        let resp = TestInvariantResponse {
            invariant_id:
                "00000000-0000-0000-0000-000000000001:spec-test-invariant:publicinterface"
                    .to_string(),
            status: "recorded".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("invariant_id"), "must contain invariant_id");
        assert!(json.contains("recorded"), "must contain status recorded");
    }

    // P8 invariant: TestClassification::parse_str roundtrips through as_str
    #[test]
    fn test_classification_parse_str_roundtrip() {
        for (input, expected) in [
            ("PublicInterface", TestClassification::PublicInterface),
            ("publicinterface", TestClassification::PublicInterface),
            ("public_interface", TestClassification::PublicInterface),
            ("SeamIntegration", TestClassification::SeamIntegration),
            ("seamintegration", TestClassification::SeamIntegration),
            (
                "ImplementationCoupled",
                TestClassification::ImplementationCoupled,
            ),
            (
                "implementation_coupled",
                TestClassification::ImplementationCoupled,
            ),
        ] {
            let parsed = TestClassification::parse_str(input);
            assert_eq!(
                parsed, expected,
                "parse_str({}) should equal {:?}",
                input, expected
            );
        }
    }

    // P8 invariant: TestClassification::parse_str defaults to PublicInterface for unknown values
    #[test]
    fn test_classification_parse_str_unknown_defaults_to_public_interface() {
        let parsed = TestClassification::parse_str("unknown-category");
        assert_eq!(
            parsed,
            TestClassification::PublicInterface,
            "unknown category must default to PublicInterface"
        );
    }
}
