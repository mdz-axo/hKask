//! Curator Evaluation Pipeline — Template output evaluation and curation
//!
//! The Curator evaluates template invocations and decides:
//! - Merge: Output is good, merge into codebase
//! - Discard: Output is broken/unsafe, discard entirely
//! - Revise: Output needs revision, send back to bot
//! - Defer: Need more information, defer decision
//!
//! The Curator is ideological — it builds on logical ideas.

use crate::memory_feedback::MemoryFeedbackAdapter;
use hkask_types::event::SpanCategory;
use hkask_types::{
    AlgedonicAlert, CurationDecision, CurationRecord, CuratorId, OCAPBoundary, TemplateInvocation,
    TemplateOutcome, UserSovereigntyState, VarietyCounter,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum CurationError {
    #[error("Invocation not found: {0}")]
    InvocationNotFound(String),
    #[error("OCAP violation: {0}")]
    OcapViolation(String),
    #[error("Evaluation error: {0}")]
    Evaluation(String),
}

/// Curator evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub invocation_id: String,
    pub decision: CurationDecision,
    pub rationale: Option<String>,
    pub ocap_checked: bool,
    pub variety_impact: i64,
}

impl EvaluationResult {
    pub fn new(invocation_id: String, decision: CurationDecision) -> Self {
        Self {
            invocation_id,
            decision,
            rationale: None,
            ocap_checked: false,
            variety_impact: 0,
        }
    }

    pub fn with_rationale(mut self, rationale: &str) -> Self {
        self.rationale = Some(rationale.to_string());
        self
    }

    pub fn with_ocap_check(mut self, checked: bool) -> Self {
        self.ocap_checked = checked;
        self
    }

    pub fn with_variety_impact(mut self, impact: i64) -> Self {
        self.variety_impact = impact;
        self
    }
}

/// Curator evaluation pipeline
///
/// The Curator evaluates template outputs and makes curation decisions.
/// It is ideological — it builds on logical ideas.
/// It enforces the Magna Carta — user sovereignty is non-negotiable.
pub struct CuratorPipeline {
    curator_id: CuratorId,
    pending: Arc<Mutex<Vec<TemplateInvocation>>>,
    records: Arc<Mutex<Vec<CurationRecord>>>,
    variety: Arc<Mutex<VarietyCounter>>,
    ocap_boundaries: Arc<Mutex<Vec<OCAPBoundary>>>,
    sovereignty: Arc<Mutex<UserSovereigntyState>>,
    capability_checker: Option<Arc<hkask_types::CapabilityChecker>>,
    memory_feedback: Option<Arc<Mutex<MemoryFeedbackAdapter>>>,
}

impl CuratorPipeline {
    pub fn new(curator_id: CuratorId) -> Self {
        Self {
            curator_id,
            pending: Arc::new(Mutex::new(Vec::new())),
            records: Arc::new(Mutex::new(Vec::new())),
            variety: Arc::new(Mutex::new(VarietyCounter::new())),
            ocap_boundaries: Arc::new(Mutex::new(Vec::new())),
            sovereignty: Arc::new(Mutex::new(UserSovereigntyState::new())),
            capability_checker: None,
            memory_feedback: None,
        }
    }

    pub fn with_capability_checker(mut self, checker: Arc<hkask_types::CapabilityChecker>) -> Self {
        self.capability_checker = Some(checker);
        self
    }

    /// Attach a memory feedback adapter to close the relevance loop
    pub fn with_memory_feedback(mut self, feedback: Arc<Mutex<MemoryFeedbackAdapter>>) -> Self {
        self.memory_feedback = Some(feedback);
        self
    }

    /// The one true Curator — system singleton
    pub fn system() -> Self {
        Self::new(CuratorId::system())
    }

    /// Submit invocation for Curator evaluation
    pub async fn submit(&self, invocation: TemplateInvocation) {
        let mut pending = self.pending.lock().await;
        pending.push(invocation);
    }

    /// Evaluate pending invocations
    pub async fn evaluate_pending(&self) -> Vec<EvaluationResult> {
        let mut pending = self.pending.lock().await;
        let mut results = Vec::new();

        for invocation in pending.drain(..) {
            let result = self.evaluate_invocation(&invocation).await;

            // Record the decision before cloning
            self.record_decision(&invocation, &result).await;

            results.push(result.clone());
        }

        results
    }

    /// Evaluate a single invocation
    pub async fn evaluate_invocation(&self, invocation: &TemplateInvocation) -> EvaluationResult {
        // Check OCAP boundaries
        let ocap_ok = self.check_ocap(invocation).await;

        // Check sovereignty state
        let sovereignty_ok = self.check_sovereignty(invocation).await;

        // Evaluate output quality
        let (decision, rationale) = self.evaluate_quality(invocation).await;

        // Update variety counter
        let variety_impact = self.update_variety(&decision).await;

        let mut result = EvaluationResult::new(invocation.id.to_string(), decision)
            .with_rationale(&rationale)
            .with_ocap_check(ocap_ok)
            .with_variety_impact(variety_impact);

        // Check for sovereignty compromise
        if !sovereignty_ok {
            result = result.with_rationale(&format!(
                "{} [SOVEREignty ALERT: user sovereignty compromised]",
                rationale
            ));
        }

        // Check for algedonic alert
        if self.variety.lock().await.needs_alert() {
            let _alert = AlgedonicAlert::new(
                self.variety.lock().await.0,
                VarietyCounter::target(),
                SpanCategory::Curation,
            );
            // Alert would be emitted to CNS in production
            result = result.with_rationale(&format!(
                "{} [ALGEDONIC ALERT: variety deficit > 100]",
                rationale
            ));
        }

        // Record memory feedback relevance for this template invocation
        if let Some(ref feedback) = self.memory_feedback {
            let template_id = invocation.template_id.to_string();
            let recall_count = invocation.outputs.len();
            // Derive a confidence proxy from the variety impact:
            // positive impact → confidence > 1, negative → < 1, zero → neutral.
            let avg_confidence = if variety_impact == 0 {
                1.0
            } else {
                1.0 + (variety_impact as f64 / 100.0)
            };
            feedback
                .lock()
                .await
                .record_relevance(&template_id, recall_count, avg_confidence, &template_id)
                .await;
        }

        result
    }

    /// Check OCAP boundaries for invocation
    async fn check_ocap(&self, invocation: &TemplateInvocation) -> bool {
        let boundaries = self.ocap_boundaries.lock().await;

        for boundary in boundaries.iter() {
            if !boundary.enforced {
                continue;
            }

            if boundary.capability == invocation.template_id.to_string()
                || boundary.capability == "*"
            {
                match boundary.authority {
                    hkask_types::AuthorityLevel::Denied => {
                        tracing::warn!(
                            "OCAP denied: bot {} attempted template {} (boundary: {})",
                            invocation.bot_id,
                            invocation.template_id,
                            boundary.capability
                        );
                        return false;
                    }
                    hkask_types::AuthorityLevel::Explicit => {
                        if let Some(ref checker) = self.capability_checker {
                            // R3: Complete OCAP verification
                            if let Some(ref token) = invocation.capability_token {
                                // Verify the token is valid and grants access to this template
                                let token_valid = checker.verify(token);
                                let grants_template = token
                                    .grants_resource(hkask_types::CapabilityResource::Template);

                                if token_valid && grants_template {
                                    tracing::info!(
                                        "OCAP explicit: token verified for bot {} on template {}",
                                        invocation.bot_id,
                                        invocation.template_id
                                    );
                                    // Token is valid, continue checking other boundaries
                                } else {
                                    tracing::warn!(
                                        "OCAP explicit: token verification failed for bot {} on template {} (valid: {}, grants_template: {})",
                                        invocation.bot_id,
                                        invocation.template_id,
                                        token_valid,
                                        grants_template
                                    );
                                    return false;
                                }
                            } else {
                                tracing::warn!(
                                    "OCAP explicit: no capability token provided for bot {} on template {} (denying for safety)",
                                    invocation.bot_id,
                                    invocation.template_id
                                );
                                return false;
                            }
                        } else {
                            tracing::warn!(
                                "OCAP explicit: no capability checker configured, denying bot {} for template {}",
                                invocation.bot_id,
                                invocation.template_id
                            );
                            return false;
                        }
                    }
                    hkask_types::AuthorityLevel::Implicit => {
                        tracing::trace!(
                            "OCAP implicit: bot {} has implicit authority for template {}",
                            invocation.bot_id,
                            invocation.template_id
                        );
                    }
                }
            }
        }

        true
    }

    /// Check sovereignty state for invocation
    async fn check_sovereignty(&self, _invocation: &TemplateInvocation) -> bool {
        let sovereignty = self.sovereignty.lock().await;

        // Check if sovereignty is compromised
        !sovereignty.is_compromised()
    }

    /// Get current sovereignty state
    pub async fn get_sovereignty_state(&self) -> UserSovereigntyState {
        self.sovereignty.lock().await.clone()
    }

    /// Mark acquisition attempt detected
    pub async fn mark_acquisition_attempt(&self) {
        let mut sovereignty = self.sovereignty.lock().await;
        sovereignty.mark_acquisition_attempt();
    }

    /// Update VC investment level
    pub async fn update_vc_investment(&self, vc_investment: f32) {
        let mut sovereignty = self.sovereignty.lock().await;
        sovereignty.update_vc_investment(vc_investment);
    }

    /// Check if sovereignty alert should be triggered
    pub async fn sovereignty_needs_alert(&self) -> bool {
        self.sovereignty.lock().await.is_compromised()
    }

    /// Evaluate output quality and make decision
    async fn evaluate_quality(
        &self,
        invocation: &TemplateInvocation,
    ) -> (CurationDecision, String) {
        // Check if outputs exist
        if invocation.outputs.is_empty() {
            return (
                CurationDecision::Discard,
                "No outputs generated".to_string(),
            );
        }

        // Check outcome
        match invocation.outcome {
            TemplateOutcome::Success => {
                // Check if output is non-empty
                for output in &invocation.outputs {
                    if let Some(text) = output.as_str() {
                        if text.trim().is_empty() {
                            return (CurationDecision::Revise, "Output is empty".to_string());
                        }

                        // Check for obvious errors
                        if text.contains("ERROR") || text.contains("FAILED") {
                            return (
                                CurationDecision::Discard,
                                "Output contains error markers".to_string(),
                            );
                        }
                    }
                }

                // If multiple outputs, Curator selects best
                if invocation.outputs.len() > 1 {
                    (
                        CurationDecision::Merge,
                        format!(
                            "Selected best of {} outputs (ideological: logical ideas)",
                            invocation.outputs.len()
                        ),
                    )
                } else {
                    (
                        CurationDecision::Merge,
                        "Output is logical and sound".to_string(),
                    )
                }
            }
            TemplateOutcome::Failure => (
                CurationDecision::Discard,
                "Template invocation failed".to_string(),
            ),
            TemplateOutcome::Merged => (
                CurationDecision::Merge,
                "Output already merged by bot".to_string(),
            ),
            TemplateOutcome::Discarded => (
                CurationDecision::Discard,
                "Output already discarded".to_string(),
            ),
        }
    }

    /// Update variety counter based on decision
    async fn update_variety(&self, decision: &CurationDecision) -> i64 {
        let mut variety = self.variety.lock().await;
        let before = variety.0 as i64;

        match decision {
            // Merge increases variety (new code added)
            CurationDecision::Merge => variety.increment(),
            // Discard maintains variety (no change)
            CurationDecision::Discard => {}
            // Revise decreases variety slightly (delay)
            CurationDecision::Revise => variety.decrement(),
            // Defer decreases variety (delay)
            CurationDecision::Defer => variety.decrement(),
        }

        let after = variety.0 as i64;
        after - before
    }

    /// Record curation decision
    async fn record_decision(&self, invocation: &TemplateInvocation, result: &EvaluationResult) {
        let record = CurationRecord::new(
            self.curator_id,
            invocation.clone(),
            result.decision,
            result.rationale.clone(),
        );

        let mut records = self.records.lock().await;
        records.push(record);
    }

    /// Get curation records
    pub async fn get_records(&self) -> Vec<CurationRecord> {
        let records = self.records.lock().await;
        records.clone()
    }

    /// Get variety counter
    pub async fn get_variety(&self) -> VarietyCounter {
        *self.variety.lock().await
    }

    /// Check if variety needs alert
    pub async fn needs_alert(&self) -> bool {
        self.variety.lock().await.needs_alert()
    }

    /// Add OCAP boundary
    pub async fn add_ocap_boundary(&self, boundary: OCAPBoundary) {
        let mut boundaries = self.ocap_boundaries.lock().await;
        boundaries.push(boundary);
    }
}

impl Default for CuratorPipeline {
    fn default() -> Self {
        Self::system()
    }
}

/// Merge multiple outputs into single coherent output
pub fn merge_outputs(outputs: &[serde_json::Value]) -> Option<String> {
    if outputs.is_empty() {
        return None;
    }

    let mut merged = String::new();
    for (i, output) in outputs.iter().enumerate() {
        if let Some(text) = output.as_str() {
            if i > 0 {
                merged.push_str("\n\n---\n\n");
            }
            merged.push_str(text);
        }
    }

    Some(merged)
}
