//! Request/response types for the Spec MCP server — MDS §3 tool surface.
//!
//! Six tools: capture, decompose, writing-quality, graph/query, graph/coherence, replica/rewrite.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Writing Quality assessment ─────────────────────────────────

/// Score for one Writing Quality dimension (Hopper, Lovelace, Schriver, Gentle).
/// Per MDS §3: `spec/require/writing-quality` — 3 of 4 passing is the publication standard.
/// These are heuristic booleans; for embedding-based scores see `DimensionScore`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub struct WritingQualityScore {
    /// Hopper test (Accessibility): Can a zero-context reader accomplish the task?
    pub hopper: bool,
    /// Lovelace test (Precision): Can a reader write a correct test from the spec alone?
    pub lovelace: bool,
    /// Schriver test (Findability): Can a reader find their answer within 30 seconds?
    pub schriver: bool,
    /// Gentle test (Agent-correctness): Would an AI agent consuming this doc behave correctly?
    pub gentle: bool,
}

/// Per-dimension embedding-based score from replica comparison.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct DimensionScore {
    /// Dimension name: "gentle", "schriver", "hopper", "lovelace", or "composite".
    pub dimension: String,
    /// Centroid entity ref (e.g., "style:gentle-lovelace:gentle-centroid").
    pub centroid_ref: String,
    /// Cosine distance from document embedding to dimension centroid.
    /// Lower = stronger alignment. ≤0.4 is the publication threshold.
    pub cosine_distance: f64,
    /// Qualitative label: "strong" (≤0.2), "aligned" (≤0.4), "divergent" (>0.4).
    pub qualitative: String,
    /// Number of passages used to compute this centroid.
    pub passage_count: usize,
}

impl WritingQualityScore {
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

// ── Response types ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GoalCaptureResponse {
    pub goal_id: String,
    pub requirements: Vec<String>,
    pub ocap_boundaries: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GoalDecomposeResponse {
    pub sub_goals: Vec<String>,
    pub dependencies: Vec<DependencyEdge>,
}

/// A dependency between sub-goals: `from` must complete before `to`.
#[derive(Debug, Serialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct WritingQualityResponse {
    pub dimensions_passing: usize,
    pub meets_publication_standard: bool,
    /// Replica persona used for embedding-based validation (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replica_persona: Option<String>,
    /// Per-dimension embedding-based scores from replica comparison.
    /// Only populated when `replica_persona` is set and DB credentials provided.
    /// Each entry has cosine_distance (lower = stronger alignment, ≤0.4 = passing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension_scores: Option<Vec<DimensionScore>>,
    /// The dimension with the highest cosine distance (weakest alignment).
    /// When set, this is the recommended target for `spec_replica_rewrite`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weakest_dimension: Option<String>,
    /// Pre-built rewrite prompt for the weakest dimension.
    /// Can be passed directly to `spec_replica_rewrite` as the `passage` parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub category: String,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Serialize)]
pub struct GraphPath {
    pub nodes: Vec<String>,
    pub length: usize,
}

#[derive(Debug, Serialize)]
pub struct GraphQueryResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub paths: Vec<GraphPath>,
}

#[derive(Debug, Serialize)]
pub struct GraphCoherenceResponse {
    pub coherence_score: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

// ── Request types ──────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalCaptureRequest {
    /// Natural-language description of the goal.
    pub description: String,
    /// Domain context (bounded-context name, existing specs, verb inventory hints).
    pub context: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalDecomposeRequest {
    /// Goal ID to decompose (from a prior capture).
    pub goal_id: String,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

/// Request to assess a spec's writing quality via the 4-perspective test.
/// Per MDS §3: `spec/require/writing-quality` — server assesses, caller does not
/// provide scores. Optional `notes` for assessor context.
/// Optional `replica_persona` enables embedding-based validation via the replica
/// server (e.g., "gentle-lovelace") as a supplement to heuristic assessment.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WritingQualityRequest {
    /// The spec ID to assess.
    pub spec_id: String,
    /// Optional assessor notes providing context for the assessment.
    pub notes: Option<String>,
    /// Optional replica persona for embedding-based validation
    /// (e.g., "gentle-lovelace"). When set with db_path and db_passphrase,
    /// the server embeds the spec content and computes per-dimension cosine
    /// distances against the persona's centroids. Distances ≤0.4 count as passing.
    #[serde(default)]
    pub replica_persona: Option<String>,
    /// Database path for embedding-based validation (required with replica_persona).
    #[serde(default)]
    pub db_path: Option<String>,
    /// Database passphrase for embedding-based validation (required with replica_persona).
    #[serde(default)]
    pub db_passphrase: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphQueryRequest {
    /// Search query for spec graph traversal.
    pub query: String,
    /// Maximum traversal depth (default 3).
    pub depth: Option<u8>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphCoherenceRequest {
    /// Collection or domain anchor to assess coherence against.
    pub collection_id: Option<String>,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

// ── Replica Rewrite types ──────────────────────────────────────

/// Request to rewrite a passage or document using the Gentle Lovelace
/// replica persona. The replica retrieves exemplar passages from the
/// target dimension's centroid and generates improved prose.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReplicaRewriteRequest {
    /// The passage or document text to rewrite.
    pub passage: String,
    /// Which Gentle Lovelace dimension to optimize for:
    /// "composite", "Gentle", "Schriver", "Hopper", or "Lovelace".
    #[serde(default = "default_dimension")]
    pub dimension: String,
    /// Optional document type for context-sensitive weighting
    /// (specification, guide, reference, adr, plan, status).
    #[serde(default)]
    pub document_type: Option<String>,
    /// Path to the per-agent semantic database (where the Gentle Lovelace
    /// embeddings and centroids are stored).
    pub db_path: String,
    /// Passphrase for opening the database.
    pub db_passphrase: String,
    /// OCAP capability token for authorization.
    pub capability_token: Option<String>,
}

fn default_dimension() -> String {
    "composite".to_string()
}

/// Result of a replica-style rewrite.
#[derive(Debug, Serialize)]
pub struct ReplicaRewriteResponse {
    /// The rewritten passage or document text.
    pub rewritten: String,
    /// The dimension that was optimized for.
    pub dimension: String,
    /// Number of exemplar passages used in the rewrite.
    pub exemplar_count: usize,
    /// Cosine distance from rewritten prose to target centroid (if validated).
    pub centroid_distance: Option<f64>,
    /// Elapsed rewrite time in milliseconds.
    pub elapsed_ms: u64,
}

// ── Contract Audit (contract/audit) ─────────────────────────────

/// Request: discover uncontracted public functions in a crate.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContractAuditRequest {
    /// Crate name (e.g., "hkask-cns"). If omitted, audits all crates.
    pub crate_name: Option<String>,
    /// Workspace root path. If omitted, defaults to HKASK_WORKSPACE_ROOT or cwd.
    pub workspace_root: Option<String>,
}

/// Response: per-crate contract coverage summary.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ContractAuditResponse {
    /// Per-crate audit results.
    pub crates: Vec<CrateCoverage>,
    /// Workspace-wide totals.
    pub totals: AuditTotals,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CrateCoverage {
    pub crate_name: String,
    pub total_pub_fns: usize,
    pub contracted: usize,
    pub coverage_pct: f64,
    pub uncontracted: Vec<UncontractedFn>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UncontractedFn {
    pub function_name: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AuditTotals {
    pub total_pub_fns: usize,
    pub contracted: usize,
    pub coverage_pct: f64,
    pub uncontracted_total: usize,
}
