use serde::{Deserialize, Serialize};

// ── OCR Result ────────────────────────────────────────────────────────────

/// The output of a single OCR backend invocation on one page.
///
/// Carries provenance metadata for verification and cross-validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// 0-based page index within the source document.
    pub page_index: usize,
    /// Which backend produced this result.
    pub backend: super::config::OcrBackend,
    /// Extracted text content.
    pub text: String,
    /// Backend-reported confidence [0.0, 1.0].
    pub confidence: f32,
    /// Wall-clock duration of the OCR invocation in milliseconds.
    pub duration_ms: u64,
    /// True if this result was produced by the fallback (second-attempt) path.
    pub was_fallback: bool,
}

// ── Cross-Validation ──────────────────────────────────────────────────────

/// Cross-validation data for a dual-routed page (Moderate tier + sampling).
///
/// Observation only — does not autonomously change routing (P4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossValidation {
    /// Page index that was dual-routed.
    pub page_index: usize,
    /// Normalized Levenshtein similarity [0.0, 1.0] between the two results.
    pub similarity: f32,
    /// Complexity tier at routing time.
    pub tier: super::config::ComplexityTier,
    /// First backend used.
    pub backend_a: super::config::OcrBackend,
    /// Second backend used.
    pub backend_b: super::config::OcrBackend,
    /// Confidence from backend A.
    pub confidence_a: f32,
    /// Confidence from backend B.
    pub confidence_b: f32,
    /// Semantic (embedding) similarity [0.0, 1.0] when available.
    /// Populated by `verify_semantic` if an embedding router is provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_similarity: Option<f32>,
}

// ── Pipeline Errors ───────────────────────────────────────────────────────

/// Errors that occur during pipeline execution. Collected per-page;
/// no error aborts the whole pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineError {
    /// Decimation (PDF → images) failed.
    DecimationFailed(String),
    /// All OCR backends exhausted for a page without success.
    OcrFailed {
        page_index: usize,
        backends_tried: Vec<super::config::OcrBackend>,
    },
    /// Assembly (results → text) failed.
    AssemblyFailed(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::DecimationFailed(msg) => write!(f, "decimation failed: {}", msg),
            PipelineError::OcrFailed {
                page_index,
                backends_tried,
            } => {
                write!(
                    f,
                    "OCR failed for page {} (tried: {})",
                    page_index,
                    backends_tried
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            PipelineError::AssemblyFailed(msg) => write!(f, "assembly failed: {}", msg),
        }
    }
}

// ── Verification Report ───────────────────────────────────────────────────

/// Post-pipeline verification checkpoint. `passed` is a computed field —
/// never settable by consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether the assembled page count matches the expected page count.
    pub page_count_match: bool,
    /// Percentage delta between estimated and actual word count.
    /// Positive = more words than expected; negative = fewer.
    pub word_count_delta_pct: f32,
    /// Indices of pages that produced zero text.
    pub empty_pages: Vec<usize>,
    /// Total number of pipeline errors across all pages.
    pub error_count: usize,
    /// Aggregate verification result. Derived from all checks.
    pub passed: bool,
    /// Page-level details: what happened to each page.
    pub page_details: Vec<PageVerificationDetail>,
}

impl VerificationReport {
    /// Compute `passed` from constituent checks.
    ///
    /// A report passes when: page count matches, no empty pages,
    /// zero errors, and word count delta is within ±50%.
    pub fn compute_passed(&mut self) {
        self.passed = self.page_count_match
            && self.empty_pages.is_empty()
            && self.error_count == 0
            && self.word_count_delta_pct.abs() <= 50.0;
    }

    /// Create a report and compute `passed` inline.
    pub fn new(
        page_count_match: bool,
        word_count_delta_pct: f32,
        empty_pages: Vec<usize>,
        error_count: usize,
        page_details: Vec<PageVerificationDetail>,
    ) -> Self {
        let mut report = Self {
            page_count_match,
            word_count_delta_pct,
            empty_pages,
            error_count,
            passed: false,
            page_details,
        };
        report.compute_passed();
        report
    }
}

/// Per-page verification detail for fine-grained diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageVerificationDetail {
    pub page_index: usize,
    pub word_count: usize,
    pub is_empty: bool,
    pub backend_used: Option<super::config::OcrBackend>,
    pub was_fallback: bool,
    pub error: Option<String>,
}

// ── Pipeline Outcome ──────────────────────────────────────────────────────

/// The single sealed output of the OCR pipeline.
///
/// No partial state escapes — consumers receive either a full
/// `PipelineOutcome` or a top-level error before the pipeline starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutcome {
    /// Per-page OCR results in page order.
    pub results: Vec<OcrResult>,
    /// Verification report computed after assembly.
    pub report: VerificationReport,
    /// Cross-validation data from dual-routed pages (calibration mode).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_validations: Vec<CrossValidation>,
    /// Pipeline errors collected across all pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<PipelineError>,
}
