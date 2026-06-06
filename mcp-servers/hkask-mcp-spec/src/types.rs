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
