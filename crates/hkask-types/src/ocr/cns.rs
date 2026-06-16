use serde::{Deserialize, Serialize};

// ── CNS Span Payloads ─────────────────────────────────────────────────────

/// CNS span payload for OCR verification events.
///
/// Emitted after every pipeline run for homeostatic monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct OcrVerificationSpan {
    pub total_pages: usize,
    pub error_count: usize,
    pub backend_distribution: Vec<BackendUsage>,
    pub duration_ms: u64,
    pub passed: bool,
}

/// Per-backend usage count for CNS reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct BackendUsage {
    pub backend: super::config::OcrBackend,
    pub page_count: usize,
}

/// CNS span payload for cross-validation observations.
///
/// Emitted per dual-routed page. Observation only — no autonomous
/// routing change (P4: affirmative consent required for behavioral change).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct OcrCrossValidationSpan {
    pub page_index: usize,
    pub similarity: f32,
    pub tier: super::config::ComplexityTier,
    pub backend_a: super::config::OcrBackend,
    pub backend_b: super::config::OcrBackend,
}
