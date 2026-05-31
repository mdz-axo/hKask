//! Agent deliberation — Multi-agent response coordination
//!
//! Coordinates deliberation between multiple agents without consensus mechanisms.
//! Each agent provides independent response; Curator synthesizes.

use crate::chat::ChatParticipant;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

/// Deliberation session for coordinating multi-agent responses
pub struct DeliberationSession {
    session_id: String,
    participants: Vec<ChatParticipant>,
    responses: HashMap<WebID, AgentResponse>,
    status: DeliberationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
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

/// Deliberation request to send to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationRequest {
    pub query: String,
    pub context: Option<Value>,
    pub template_id: Option<String>,
    pub timeout_ms: u64,
}

impl DeliberationRequest {
    pub fn new(query: String) -> Self {
        Self {
            query,
            context: None,
            template_id: None,
            timeout_ms: 30000,
        }
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_template(mut self, template_id: String) -> Self {
        self.template_id = Some(template_id);
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
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
    pub fn new(session_id: String, curator_webid: WebID) -> Self {
        Self {
            session_id,
            participants: Vec::new(),
            responses: HashMap::new(),
            status: DeliberationStatus::Pending,
        }
    }

    /// Add a participant to deliberation
    pub fn add_participant(&mut self, participant: ChatParticipant) {
        self.participants.push(participant);
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

    /// Get session status
    pub fn status(&self) -> &DeliberationStatus {
        &self.status
    }

    /// Mark deliberation as in progress
    pub fn start(&mut self) {
        self.status = DeliberationStatus::InProgress;
    }

    /// Mark deliberation as completed
    pub fn complete(&mut self) {
        self.status = DeliberationStatus::Completed;
        info!("Deliberation session {} completed", self.session_id);
    }

    /// Cancel deliberation
    pub fn cancel(&mut self) {
        self.status = DeliberationStatus::Cancelled;
        info!("Deliberation session {} cancelled", self.session_id);
    }

    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Get response count
    pub fn response_count(&self) -> usize {
        self.responses.len()
    }
}

/// **Deprecated:** Use `SessionManager` instead.
///
/// `DeliberationCoordinator` has been collapsed into `SessionManager` which handles
/// both chat and deliberation sessions.
#[deprecated(
    since = "0.21.0",
    note = "Use `SessionManager` instead. DeliberationCoordinator has been collapsed into SessionManager."
)]
pub struct DeliberationCoordinator {
    sessions: HashMap<String, DeliberationSession>,
    curator_webid: WebID,
}

#[allow(deprecated)]
impl DeliberationCoordinator {
    /// Create new coordinator
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            sessions: HashMap::new(),
            curator_webid,
        }
    }

    /// Create a new deliberation session
    pub fn create_session(&mut self, session_id: &str) -> &mut DeliberationSession {
        use std::collections::hash_map::Entry;

        match self.sessions.entry(session_id.to_string()) {
            Entry::Vacant(entry) => {
                let session = DeliberationSession::new(session_id.to_string(), self.curator_webid);
                entry.insert(session)
            }
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }

    /// Get a deliberation session
    pub fn get_session(&self, session_id: &str) -> Option<&DeliberationSession> {
        self.sessions.get(session_id)
    }

    /// Get a deliberation session mutably
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut DeliberationSession> {
        self.sessions.get_mut(session_id)
    }

    /// Remove a deliberation session
    pub fn remove_session(&mut self, session_id: &str) -> Option<DeliberationSession> {
        self.sessions.remove(session_id)
    }

    /// List active sessions
    pub fn list_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[allow(deprecated)]
impl Default for DeliberationCoordinator {
    fn default() -> Self {
        Self::new(WebID::new())
    }
}
