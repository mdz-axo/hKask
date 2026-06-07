//! Agent deliberation — Multi-agent response coordination
//!
//! Coordinates deliberation between multiple agents without consensus mechanisms.
//! Each agent provides independent response; Curator synthesizes.
//!
//! Types live here in `hkask-ensemble` (not re-exported from `hkask-agents`)
//! so that ensemble depends only on `hkask-types`, respecting the Authority DAG.

use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deliberation session for coordinating multi-agent responses
pub struct DeliberationSession {
    session_id: String,
    responses: HashMap<WebID, AgentResponse>,
    status: DeliberationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeliberationStatus {
    Pending,
    InProgress,
}

/// Individual agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub agent_webid: WebID,
    pub content: String,
    pub confidence: f64,
    pub template_used: Option<String>,
    pub processing_time_ms: u64,
}

impl AgentResponse {
    pub fn new(agent_webid: WebID, content: String, confidence: f64) -> Self {
        Self {
            agent_webid,
            content,
            confidence,
            template_used: None,
            processing_time_ms: 0,
        }
    }

    pub fn with_template(mut self, template: String) -> Self {
        self.template_used = Some(template);
        self
    }

    pub fn with_processing_time(mut self, time_ms: u64) -> Self {
        self.processing_time_ms = time_ms;
        self
    }
}

/// Synthesized result from deliberation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationResult {
    pub session_id: String,
    pub synthesized_response: String,
    pub individual_responses: Vec<AgentResponse>,
    pub synthesis_method: String,
}

impl DeliberationSession {
    /// Create new deliberation session
    pub fn new(session_id: String, _curator_webid: WebID) -> Self {
        Self {
            session_id,
            responses: HashMap::new(),
            status: DeliberationStatus::Pending,
        }
    }

    /// Record an agent's response
    pub fn record_response(&mut self, response: AgentResponse) {
        self.responses.insert(response.agent_webid, response);
    }

    /// Get all responses
    pub fn get_responses(&self) -> &HashMap<WebID, AgentResponse> {
        &self.responses
    }

    /// Synthesize responses (simple concatenation, no consensus)
    pub fn synthesize(&self) -> DeliberationResult {
        let mut individual_responses: Vec<AgentResponse> =
            self.responses.values().cloned().collect();
        individual_responses.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let synthesized = individual_responses
            .iter()
            .map(|r| format!("[{}]: {}", r.agent_webid, r.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        DeliberationResult {
            session_id: self.session_id.clone(),
            synthesized_response: synthesized,
            individual_responses,
            synthesis_method: "concatenation".to_string(),
        }
    }

    /// Mark deliberation as in progress
    pub fn start(&mut self) {
        self.status = DeliberationStatus::InProgress;
    }

    /// Get response count
    pub fn response_count(&self) -> usize {
        self.responses.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AgentResponse ───────────────────────────────────────────────────────

    // P8 invariant: new response has default fields
    #[test]
    fn agent_response_new_has_defaults() {
        let webid = WebID::new();
        let resp = AgentResponse::new(webid.clone(), "hello".to_string(), 0.9);
        assert_eq!(resp.agent_webid, webid);
        assert_eq!(resp.content, "hello");
        assert!((resp.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(resp.template_used, None);
        assert_eq!(resp.processing_time_ms, 0);
    }

    // P8 invariant: builder methods set optional fields without clobbering base fields
    #[test]
    fn agent_response_builder_methods() {
        let resp = AgentResponse::new(WebID::new(), "test".to_string(), 0.5)
            .with_template("my_template".to_string())
            .with_processing_time(150);
        // Builder fields
        assert_eq!(resp.template_used, Some("my_template".to_string()));
        assert_eq!(resp.processing_time_ms, 150);
        // Base fields must survive chaining
        assert_eq!(resp.content, "test");
        assert!((resp.confidence - 0.5).abs() < f64::EPSILON);
    }

    // ── DeliberationSession ────────────────────────────────────────────────

    // P8 invariant: new session starts with zero responses
    #[test]
    fn deliberation_session_new_is_empty() {
        let session = DeliberationSession::new("s1".to_string(), WebID::new());
        assert_eq!(session.response_count(), 0);
        assert!(session.get_responses().is_empty());
    }

    // P8 invariant: record_response adds to responses map
    #[test]
    fn deliberation_session_record_response() {
        let mut session = DeliberationSession::new("s1".to_string(), WebID::new());
        let webid = WebID::new();
        session.record_response(AgentResponse::new(webid.clone(), "answer".to_string(), 0.8));
        assert_eq!(session.response_count(), 1);
        assert!(session.get_responses().contains_key(&webid));
    }

    // P8 invariant: same agent overwrites previous response
    #[test]
    fn deliberation_session_same_agent_overwrites() {
        let mut session = DeliberationSession::new("s1".to_string(), WebID::new());
        let webid = WebID::new();
        session.record_response(AgentResponse::new(webid.clone(), "first".to_string(), 0.8));
        session.record_response(AgentResponse::new(webid.clone(), "second".to_string(), 0.9));
        assert_eq!(session.response_count(), 1);
        assert_eq!(session.get_responses()[&webid].content, "second");
    }

    // P8 invariant: synthesize sorts by descending confidence
    #[test]
    fn deliberation_session_synthesize_sorts_by_confidence() {
        let mut session = DeliberationSession::new("s1".to_string(), WebID::new());
        let w1 = WebID::new();
        let w2 = WebID::new();
        session.record_response(AgentResponse::new(w1, "low".to_string(), 0.3));
        session.record_response(AgentResponse::new(w2, "high".to_string(), 0.9));
        let result = session.synthesize();
        assert_eq!(result.individual_responses[0].content, "high");
        assert_eq!(result.individual_responses[1].content, "low");
    }

    // P8 invariant: synthesize uses concatenation method
    #[test]
    fn deliberation_session_synthesize_method_is_concatenation() {
        let session = DeliberationSession::new("s1".to_string(), WebID::new());
        let result = session.synthesize();
        assert_eq!(result.synthesis_method, "concatenation");
    }

    // P8 invariant: synthesize preserves session_id
    #[test]
    fn deliberation_session_synthesize_preserves_session_id() {
        let session = DeliberationSession::new("test-session".to_string(), WebID::new());
        let result = session.synthesize();
        assert_eq!(result.session_id, "test-session");
    }

    // P8 invariant: synthesize with no responses produces empty output
    #[test]
    fn deliberation_session_synthesize_empty() {
        let session = DeliberationSession::new("s1".to_string(), WebID::new());
        let result = session.synthesize();
        assert!(result.synthesized_response.is_empty());
        assert!(result.individual_responses.is_empty());
    }
}
