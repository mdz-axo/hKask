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

// G2 Justification: This module exposes 5 public items — ComplexityTier, ComplexityScore, OcrBackend, ThresholdConfig, and DEFAULT_LLM_OCR_MODEL. Each represents a distinct configuration concept.

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

// ── Thresholds Module ─────────────────────────────────────────────────────

/// Default vision LLM model for OCR.
/// Uses olmOCR-2 on DeepInfra (cloud, fast GPUs, 82.4 on OlmOCR-Bench).
/// Override via `HKASK_OCR_MODEL` env var or `llm_model` pipeline parameter.
pub const DEFAULT_LLM_OCR_MODEL: &str = "DI/allenai/olmOCR-2-7B-1025";

/// Configurable OCR complexity thresholds.
///
/// When `tuneable` is `true`, the CNS calibration system may suggest
/// adjustments based on accumulated cross-validation data (P4: human
/// approval required before any change takes effect).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Edge-density ratio below which a page is considered Simple.
    pub simple_max: f32,
    /// Edge-density ratio below which a page is considered Moderate.
    /// Values ≥ this threshold are Complex.
    pub moderate_max: f32,
    /// Dual-routing sampling rate for Moderate-tier pages [0.0, 1.0].
    pub moderate_sample_rate: f32,
    /// Whether CNS may suggest threshold adjustments based on observed accuracy.
    /// When `false`, thresholds are locked at configured values.
    #[serde(default = "default_tuneable")]
    pub tuneable: bool,
}

fn default_tuneable() -> bool {
    true
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            simple_max: 0.05,
            moderate_max: 0.15,
            moderate_sample_rate: 0.10,
            tuneable: true,
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
