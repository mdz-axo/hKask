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

// ── Writing Excellence assessment ───────────────────────────────

/// Score for one Writing Excellence dimension (Hopper, Lovelace, Schriver, Gentle).
/// Per WRITING_EXCELLENCE.md §3: 3 of 4 passing is the publication standard.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub struct WritingExcellenceScore {
    /// Hopper test (Accessibility): Can a zero-context reader accomplish the task?
    pub hopper: bool,
    /// Lovelace test (Precision): Can a reader write a correct test from the spec alone?
    pub lovelace: bool,
    /// Schriver test (Findability): Can a reader find their answer within 30 seconds?
    pub schriver: bool,
    /// Gentle test (Agent-correctness): Would an AI agent consuming this doc behave correctly?
    pub gentle: bool,
}

impl WritingExcellenceScore {
    /// Number of dimensions passing.
    pub fn passes(&self) -> usize {
        let mut n = 0;
        if self.hopper {
            n += 1;
        }
        if self.lovelace {
            n += 1;
        }
        if self.schriver {
            n += 1;
        }
        if self.gentle {
            n += 1;
        }
        n
    }

    /// Whether the document meets the publication standard (3 of 4).
    pub fn meets_publication_standard(&self) -> bool {
        self.passes() >= 3
    }
}

/// Request for a Writing Excellence assessment on a specification document.
/// Per WRITING_EXCELLENCE.md §3: the 4-perspective test is part of the
/// DDMVSS curation process.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WritingExcellenceRequest {
    /// The spec ID to assess.
    pub spec_id: String,
    /// Assessor-provided scores for each dimension.
    pub scores: WritingExcellenceScore,
    /// Optional assessor notes explaining the scores.
    pub notes: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

/// Response from spec/curate/writing-excellence.
#[derive(Debug, Serialize)]
pub struct WritingExcellenceResponse {
    /// The spec ID that was assessed.
    pub spec_id: String,
    /// Number of dimensions passing (0–4).
    pub dimensions_passing: usize,
    /// Whether the document meets the publication standard (3 of 4).
    pub meets_publication_standard: bool,
    /// Whether the document is below minimum quality (1 of 4 blocks publication).
    pub blocks_publication: bool,
    /// The individual dimension scores.
    pub scores: WritingExcellenceScore,
}

// ── Completeness domain ──────────────────────────────────────

/// Domain of completeness assessment for curation decisions.
/// Specification: the spec document is internally complete.
/// Implementation: the code that satisfies the spec is complete.
/// These are orthogonal — a spec can be complete even if no code implements it.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub enum CompletenessDomain {
    /// The specification document is complete as a specification
    Specification,
    /// The code that satisfies the specification is complete
    Implementation,
}

impl Default for CompletenessDomain {
    fn default() -> Self {
        CompletenessDomain::Specification
    }
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
    /// Whether the specification document is complete (all criteria satisfied)
    pub specification_completeness: bool,
    /// Implementation status, included only when CompletenessDomain::Implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_status: Option<String>,
    /// Writing Excellence assessment (4-perspective test per WRITING_EXCELLENCE.md).
    /// Included when the caller provides a writing_excellence assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writing_excellence: Option<WritingExcellenceScore>,
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
    pub completeness_domain: Option<CompletenessDomain>,
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
    pub completeness_domain: Option<CompletenessDomain>,
    /// Writing Excellence 4-perspective assessment. When provided, the evaluation
    /// includes Writing Excellence results in the response and the curation
    /// decision accounts for publication standard (3 of 4 passing).
    pub writing_excellence: Option<WritingExcellenceScore>,
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
    pub completeness_domain: Option<CompletenessDomain>,
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
