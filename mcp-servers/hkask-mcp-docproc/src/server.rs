//! Unified DocProc server — combines OCR pipeline with knowledge extraction,
//! triple extraction, embedding, QA generation, and caching.

use crate::ocr::llm_ocr::LlmOcrExecutor;
use crate::ocr::pipeline::OcrExecutor;
use crate::ocr::tesseract::TesseractExecutor;
use hkask_inference::{EmbeddingRouter, InferenceConfig, InferenceRouter};
use hkask_mcp::DaemonClient;
use hkask_types::ocr::{OcrBackend, OcrResult, ThresholdConfig};
use hkask_types::ports::CnsObserver;
use hkask_types::{LLMParameters, WebID};
use std::sync::Mutex;

/// Minimum word count from pdf-extract to consider text extraction successful
/// before falling back to OCR for scanned PDFs.
pub const OCR_FALLBACK_WORD_THRESHOLD: usize = 100;

/// System prompt for OCR vision requests.
const OCR_SYSTEM_PROMPT: &str =
    "Extract all text from this image. Output only the extracted text, nothing else.";

/// Default max tokens for OCR output.
pub fn default_ocr_max_tokens() -> u32 {
    8192
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct DocProcServer {
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<DaemonClient>,
    /// Configured OCR model (from HKASK_OCR_MODEL env var). None means OCR is unavailable.
    pub ocr_model: Option<String>,
    /// Inference configuration for the router.
    pub inference_config: InferenceConfig,
    /// OCR pipeline thresholds (loaded from settings.json).
    pub ocr_thresholds: ThresholdConfig,
    /// Embedding router for semantic cross-validation (None if unavailable).
    pub embedding_router: Option<EmbeddingRouter>,
    /// CNS observer for pipeline events → daemon → NuEventStore → CurationLoop.
    pub cns_observer: DocProcCnsObserver,
    /// Accumulated cross-validations across pipeline runs for threshold self-tuning.
    /// Cleared after a drift alert is emitted to avoid redundant suggestions.
    pub cv_accumulator: Mutex<Vec<hkask_types::ocr::CrossValidation>>,
    /// In-memory vector index for RAG query/retrieval. Passages indexed by `docproc_chunk`
    /// are stored here with their embeddings for cosine-similarity search via `docproc_query`.
    pub index: Mutex<Vec<IndexedPassage>>,
}

/// A passage stored in the in-memory vector index with its embedding.
#[derive(Debug, Clone)]
pub struct IndexedPassage {
    pub text: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}

impl DocProcServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        ocr_model: Option<String>,
        inference_config: InferenceConfig,
        ocr_thresholds: ThresholdConfig,
        embedding_router: Option<EmbeddingRouter>,
    ) -> anyhow::Result<Self> {
        let cns_observer = DocProcCnsObserver::new(daemon.clone(), &replicant);
        Ok(Self {
            webid,
            replicant,
            daemon,
            ocr_model,
            inference_config,
            ocr_thresholds,
            embedding_router,
            cns_observer,
            cv_accumulator: Mutex::new(Vec::new()),
            index: Mutex::new(Vec::new()),
        })
    }

    /// Check whether OCR capability is available.
    pub fn has_ocr(&self) -> bool {
        self.ocr_model.is_some()
    }

    /// Index passages into the in-memory vector store for later query.
    /// Embeds each passage text and stores it with metadata.
    /// Returns the number of passages indexed (0 if embedding router unavailable).
    pub async fn index_passages(&self, passages: &[(String, String)], source_label: &str) -> usize {
        let Some(ref emb_router) = self.embedding_router else {
            return 0;
        };

        let texts: Vec<&str> = passages.iter().map(|(_, t)| t.as_str()).collect();
        if texts.is_empty() {
            return 0;
        }

        let model_name = std::env::var("HKASK_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

        let vectors = match emb_router.embed_sentences(&model_name, &texts).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "hkask.mcp.docproc.index", error = %e, "Failed to embed passages for indexing");
                return 0;
            }
        };

        let mut index = self.index.lock().unwrap();
        for (i, ((entity_ref, passage_text), embedding)) in
            passages.iter().zip(vectors.into_iter()).enumerate()
        {
            index.push(IndexedPassage {
                text: passage_text.clone(),
                metadata: serde_json::json!({
                    "entity_ref": entity_ref,
                    "source": source_label,
                    "position": i,
                }),
                embedding,
            });
        }
        passages.len()
    }
}

// ── CNS Observer ─────────────────────────────────────────────────────────

/// CNS observer that implements the real `hkask_types::ports::CnsObserver` trait.
/// Routes pipeline events through the daemon to NuEventStore → CurationLoop.
pub struct DocProcCnsObserver {
    daemon: Option<DaemonClient>,
    replicant: String,
}

impl DocProcCnsObserver {
    pub fn new(daemon: Option<DaemonClient>, replicant: &str) -> Self {
        Self {
            daemon,
            replicant: replicant.to_string(),
        }
    }

    fn persist_span(&self, span_type: &str, data: serde_json::Value) {
        if let Some(ref daemon) = self.daemon {
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let span_name = span_type.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(
                        &replicant,
                        "ocr_pipeline",
                        span_name.as_str(),
                        &data,
                        Some(0.85),
                    )
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.docproc.cns", span = %span_name, "CNS span persisted");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", span = %span_name, response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", span = %span_name, error = %e, "Failed to persist CNS span");
                    }
                }
            });
        }
    }
}

#[async_trait::async_trait]
impl hkask_types::ports::CnsObserver for DocProcCnsObserver {
    fn interest_mask(&self) -> Vec<hkask_types::event::SpanNamespace> {
        vec![hkask_types::event::SpanNamespace::new("cns.pipeline")]
    }

    async fn on_event(&self, event: &hkask_types::event::NuEvent) {
        let span_name = event.span.namespace.short_name();
        self.persist_span(span_name, event.observation.clone());
    }

    async fn on_depletion(&self, _signal: &hkask_types::ports::DepletionSignal) {
        tracing::warn!(target: "hkask.mcp.docproc.cns", "CNS depletion signal received");
    }

    async fn on_backpressure(&self, _signal: &hkask_types::ports::BackpressureSignal) {
        tracing::warn!(target: "hkask.mcp.docproc.cns", "CNS backpressure signal received");
    }
}

// ── OcrExecutor implementation ───────────────────────────────────────────

#[async_trait::async_trait]
impl OcrExecutor for DocProcServer {
    fn is_available(&self, backend: &OcrBackend) -> bool {
        match backend {
            OcrBackend::Tesseract => TesseractExecutor::new().is_available(backend),
            OcrBackend::LlmOcr(_) => self.ocr_model.is_some(),
        }
    }

    async fn execute(
        &self,
        page_index: usize,
        backend: &OcrBackend,
        image: &image::DynamicImage,
        is_fallback: bool,
    ) -> Result<OcrResult, String> {
        match backend {
            OcrBackend::Tesseract => {
                TesseractExecutor::new()
                    .execute(page_index, backend, image, is_fallback)
                    .await
            }
            _ => {
                LlmOcrExecutor::new(self.inference_config.clone())
                    .execute(page_index, backend, image, is_fallback)
                    .await
            }
        }
    }
}

// ── Pipeline helpers ─────────────────────────────────────────────────────

impl DocProcServer {
    /// Persist pipeline outcome to daemon for CNS observability.
    pub async fn persist_pipeline_outcome(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
        if let Some(ref daemon) = self.daemon {
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let data = serde_json::json!({
                "total_pages": outcome.results.len(),
                "error_count": outcome.errors.len(),
                "verification_passed": outcome.report.passed,
                "page_count_match": outcome.report.page_count_match,
                "empty_pages": outcome.report.empty_pages,
                "word_count_delta_pct": outcome.report.word_count_delta_pct,
                "cross_validations": outcome.cross_validations.len(),
                "backend_distribution": outcome.results.iter()
                    .fold(std::collections::HashMap::new(), |mut acc, r| {
                        *acc.entry(r.backend.label().to_string()).or_insert(0) += 1;
                        acc
                    }),
            });
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(
                        &replicant,
                        "ocr_pipeline",
                        "verification",
                        &data,
                        Some(0.85),
                    )
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.docproc.cns", "Pipeline outcome persisted to daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", response = ?other, "Unexpected daemon response");
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.cns", error = %e, "Failed to persist pipeline outcome");
                    }
                }
            });
        }

        self.emit_pipeline_event(outcome).await;
        self.accumulate_and_check_drift(outcome);
    }

    /// Emit a CNS pipeline event through the CnsObserver.
    async fn emit_pipeline_event(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
        use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};

        let observation = serde_json::json!({
            "total_pages": outcome.results.len(),
            "error_count": outcome.errors.len(),
            "verification_passed": outcome.report.passed,
            "page_count_match": outcome.report.page_count_match,
            "empty_page_count": outcome.report.empty_pages.len(),
            "word_count_delta_pct": outcome.report.word_count_delta_pct,
            "cross_validation_count": outcome.cross_validations.len(),
            "mean_cross_validation_similarity": if outcome.cross_validations.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(outcome.cross_validations.iter()
                    .map(|cv| cv.similarity)
                    .sum::<f32>() / outcome.cross_validations.len() as f32)
            },
        });

        let event = NuEvent::new(
            self.webid,
            Span::new(SpanNamespace::new("cns.pipeline"), "ocr.verification"),
            Phase::Sense,
            observation,
            0,
        )
        .with_visibility("private");

        self.cns_observer.on_event(&event).await;
    }

    /// Accumulate cross-validations and check for threshold drift.
    fn accumulate_and_check_drift(&self, outcome: &hkask_types::ocr::PipelineOutcome) {
        use crate::ocr::calibration::{analyze_threshold_drift, emit_drift_alert};

        let mut acc = self.cv_accumulator.lock().unwrap();
        acc.extend(outcome.cross_validations.clone());

        let synthetic_outcome = hkask_types::ocr::PipelineOutcome {
            results: vec![],
            report: hkask_types::ocr::VerificationReport::new(true, 0.0, vec![], 0, vec![]),
            cross_validations: acc.clone(),
            errors: vec![],
        };

        if let Some(alert) = analyze_threshold_drift(&[synthetic_outcome], &self.ocr_thresholds) {
            emit_drift_alert(&alert);
            acc.clear();
        }
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    pub fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool, "input": input_summary, "outcome": outcome,
                "detail": detail, "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.docproc.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.docproc.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.docproc.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }

    /// Resolve OCR model: explicit override > HKASK_OCR_MODEL env.
    pub async fn resolve_ocr_model(&self, override_model: Option<&str>) -> Result<String, String> {
        let model = if let Some(m) = override_model
            && !m.is_empty()
        {
            m.to_string()
        } else {
            self.ocr_model.clone().ok_or_else(|| {
                "No OCR model configured. Set HKASK_OCR_MODEL env var to a vision-capable model, or pass the 'model' parameter. Use inference_models to discover available models.".to_string()
            })?
        };

        let router = InferenceRouter::new(self.inference_config.clone());
        let vision_models = router.list_vision_models().await;
        let is_vision = vision_models
            .iter()
            .any(|m| m.model == model || m.prefixed_name == model);

        if !is_vision {
            let all_models = router.list_models().await;
            let exists = all_models
                .iter()
                .any(|m| m.model == model || m.prefixed_name == model);
            if exists {
                return Err(format!(
                    "Model '{}' exists but may not support vision input. Use a vision-capable model (e.g., llava, minicpm-v, lighton). Run 'kask inference models' to list available models.",
                    model
                ));
            }
        }

        Ok(model)
    }

    /// Perform OCR by sending base64-encoded bytes to a vision model.
    pub async fn do_ocr(
        &self,
        file_bytes: &[u8],
        model: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        if file_bytes.is_empty() {
            return Err("File is empty".to_string());
        }

        let b64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, file_bytes);

        let router = InferenceRouter::new(self.inference_config.clone());

        let params = LLMParameters {
            temperature: 0.1,
            max_tokens,
            ..Default::default()
        };

        let result = router
            .generate_vision(OCR_SYSTEM_PROMPT, &[b64_data], &params, Some(model))
            .await
            .map_err(|e| format!("OCR inference failed: {}", e))?;

        Ok(result.text)
    }
}
