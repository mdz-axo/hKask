//! OCR Pipeline Types — Algebraic types for the hkask-mcp-docproc pipeline
//!
//! Every valid pipeline state is representable; every invalid state is
//! unrepresentable (P3). The sealed type hierarchy forms a closed set:
//! PDF → ComplexityScore → OcrBackend → OcrResult → VerificationReport → PipelineOutcome.
//!
//! # RDF core
//! ```text
//! (Page, hasComplexity, ComplexityScore)
//! (ComplexityScore, routesTo, OcrBackend)
//! (OcrBackend, produces, OcrResult)
//! (OcrResult, verifiedBy, VerificationReport)
//! (VerificationReport, reportsTo, CnsSpan)
//! ```

use serde::{Deserialize, Serialize};

// ── Complexity Tiers ──────────────────────────────────────────────────────

/// Complexity tier derived from pixel-density heuristics.
///
/// Determines backend routing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComplexityTier {
    /// Low visual complexity — text-only or near-empty pages.
    Simple,
    /// Moderate complexity — tables, mixed content, forms.
    Moderate,
    /// High complexity — photographs, dense graphics, handwritten text.
    Complex,
}

/// A scored complexity value for a single page image.
///
/// `tier` is derived from `value` via threshold dispatch and is the
/// authoritative routing signal; `value` is the raw heuristic score for
/// logging and threshold self-tuning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexityScore {
    /// Raw edge-density ratio [0.0, 1.0].
    pub value: f32,
    /// Threshold-derived tier.
    pub tier: ComplexityTier,
}

// ── OCR Backends ──────────────────────────────────────────────────────────

/// Exhaustive set of OCR backends. Each variant maps to a concrete
/// invocation path within the pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OcrBackend {
    /// Classical OCR via Tesseract (fast, best for text-only).
    Tesseract,
    /// Vision-language model OCR via hkask-inference router.
    /// The inner `String` is the model name (e.g., `maternion/LightOnOCR-2:1b`).
    LlmOcr(String),
}

impl OcrBackend {
    /// Human-readable label for logging and CNS spans.
    pub fn label(&self) -> &str {
        match self {
            OcrBackend::Tesseract => "tesseract",
            OcrBackend::LlmOcr(_) => "llm-ocr",
        }
    }
}

impl std::fmt::Display for OcrBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrBackend::Tesseract => write!(f, "tesseract"),
            OcrBackend::LlmOcr(model) => write!(f, "llm-ocr({})", model),
        }
    }
}

// ── OCR Result ────────────────────────────────────────────────────────────

/// The output of a single OCR backend invocation on one page.
///
/// Carries provenance metadata for verification and cross-validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// 0-based page index within the source document.
    pub page_index: usize,
    /// Which backend produced this result.
    pub backend: OcrBackend,
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
    pub tier: ComplexityTier,
    /// First backend used.
    pub backend_a: OcrBackend,
    /// Second backend used.
    pub backend_b: OcrBackend,
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
        backends_tried: Vec<OcrBackend>,
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
    pub backend_used: Option<OcrBackend>,
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

// ── CNS Span Payloads ─────────────────────────────────────────────────────

/// CNS span payload for OCR verification events.
///
/// Emitted after every pipeline run for homeostatic monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrVerificationSpan {
    pub total_pages: usize,
    pub error_count: usize,
    pub backend_distribution: Vec<BackendUsage>,
    pub duration_ms: u64,
    pub passed: bool,
}

/// Per-backend usage count for CNS reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendUsage {
    pub backend: OcrBackend,
    pub page_count: usize,
}

/// CNS span payload for cross-validation observations.
///
/// Emitted per dual-routed page. Observation only — no autonomous
/// routing change (P4: affirmative consent required for behavioral change).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrCrossValidationSpan {
    pub page_index: usize,
    pub similarity: f32,
    pub tier: ComplexityTier,
    pub backend_a: OcrBackend,
    pub backend_b: OcrBackend,
}

// ── Thresholds Module ─────────────────────────────────────────────────────

/// Configurable OCR complexity thresholds.
///
/// Values are loadable from `hkask-templates` registry for self-tuning
/// (P4: changes require affirmative consent, not autonomous mutation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Edge-density ratio below which a page is considered Simple.
    pub simple_max: f32,
    /// Edge-density ratio below which a page is considered Moderate.
    /// Values ≥ this threshold are Complex.
    pub moderate_max: f32,
    /// Dual-routing sampling rate for Moderate-tier pages [0.0, 1.0].
    pub moderate_sample_rate: f32,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            simple_max: 0.05,
            moderate_max: 0.15,
            moderate_sample_rate: 0.10,
        }
    }
}

impl ThresholdConfig {
    /// Classify an edge-density value into a complexity tier.
    pub fn classify(&self, edge_density: f32) -> ComplexityTier {
        if edge_density < self.simple_max {
            ComplexityTier::Simple
        } else if edge_density < self.moderate_max {
            ComplexityTier::Moderate
        } else {
            ComplexityTier::Complex
        }
    }
}

/// Legacy constants module — prefer `ThresholdConfig` for new code.
/// Retained for `DEFAULT_LLM_OCR_MODEL` which is referenced by routing.
pub mod thresholds {
    /// Default vision LLM model for OCR.
    /// Primary: olmOCR-2 on DeepInfra (cloud, fast GPUs, 82.4 on OlmOCR-Bench).
    /// Local fallback: LightOnOCR-2:1b on Ollama (83.2 on OlmOCR-Bench, 1B params).
    /// Override via `HKASK_OCR_MODEL` env var or `llm_model` pipeline parameter.
    pub const DEFAULT_LLM_OCR_MODEL: &str = "DI/allenai/olmOCR-2-7B-1025";
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ:ocr-type-01 — PipelineOutcome is constructible and serializable
    #[test]
    fn pipeline_outcome_roundtrip() {
        let outcome = PipelineOutcome {
            results: vec![OcrResult {
                page_index: 0,
                backend: OcrBackend::Tesseract,
                text: "Hello".into(),
                confidence: 0.95,
                duration_ms: 100,
                was_fallback: false,
            }],
            report: VerificationReport::new(true, 0.0, vec![], 0, vec![]),
            cross_validations: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let back: PipelineOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.results.len(), 1);
        assert!(back.report.passed);
    }

    // REQ:ocr-type-02 — VerificationReport::passed is derived, not settable
    #[test]
    fn verification_report_passed_is_computed() {
        let report = VerificationReport::new(false, 0.0, vec![], 0, vec![]);
        assert!(!report.passed, "page_count_match=false should fail");

        let report = VerificationReport::new(true, 0.0, vec![0], 0, vec![]);
        assert!(!report.passed, "empty page should fail");

        let report = VerificationReport::new(true, 0.0, vec![], 3, vec![]);
        assert!(!report.passed, "errors should fail");

        let report = VerificationReport::new(true, 60.0, vec![], 0, vec![]);
        assert!(!report.passed, "word delta >50% should fail");

        let report = VerificationReport::new(true, 10.0, vec![], 0, vec![]);
        assert!(report.passed, "clean report should pass");
    }

    // REQ:ocr-type-03 — ComplexityTier ordering
    #[test]
    fn complexity_tier_ordering() {
        assert!(ComplexityTier::Simple < ComplexityTier::Moderate);
        assert!(ComplexityTier::Moderate < ComplexityTier::Complex);
    }

    // REQ:ocr-type-04 — OcrBackend labels are stable
    #[test]
    fn ocr_backend_labels() {
        assert_eq!(OcrBackend::Tesseract.label(), "tesseract");
        assert_eq!(OcrBackend::LlmOcr("lighton".into()).label(), "llm-ocr");
        assert_eq!(OcrBackend::LlmOcr("gpt4".into()).label(), "llm-ocr");
    }

    // REQ:ocr-type-05 — PipelineError Display is meaningful
    #[test]
    fn pipeline_error_display() {
        let err = PipelineError::OcrFailed {
            page_index: 2,
            backends_tried: vec![OcrBackend::Tesseract, OcrBackend::LlmOcr("lighton".into())],
        };
        let display = err.to_string();
        assert!(display.contains("page 2"));
        assert!(display.contains("tesseract"));
        assert!(display.contains("lighton"));
    }

    // REQ:ocr-type-06 — CrossValidation is serializable
    #[test]
    fn cross_validation_roundtrip() {
        let cv = CrossValidation {
            page_index: 3,
            similarity: 0.87,
            tier: ComplexityTier::Moderate,
            backend_a: OcrBackend::Tesseract,
            backend_b: OcrBackend::LlmOcr("minicpm".into()),
            confidence_a: 0.92,
            confidence_b: 0.89,
            semantic_similarity: None,
        };
        let json = serde_json::to_string(&cv).unwrap();
        let back: CrossValidation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.similarity, 0.87);
    }
}
