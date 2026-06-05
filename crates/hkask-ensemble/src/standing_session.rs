//! Standing Ensemble Session — Bootstrap and lifecycle management
//!
//! The standing session is the persistent coordination channel where the R7 bots
//! report status and the Curator orchestrates metacognition.

use crate::chat::{ChatMessage, ChatParticipant, EnsembleChat, GasBudgetConfig, ParticipantRole};
use hkask_types::NuEventSink;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::ports::{MessageRecord, SessionRecord, StandingSessionPort};
use hkask_types::{R7BotIdentity, WebID, default_r7_bots};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum StandingSessionError {
    #[error("Session store error: {0}")]
    Storage(#[from] hkask_types::ports::SessionStoreError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StandingSessionConfig {
    pub session: SessionMetadata,
    pub participants: Vec<ParticipantEntry>,
    pub bootstrap: BootstrapConfig,
    #[serde(default)]
    pub gas: Option<GasSection>,
}

/// Gas budget section from standing-ensemble-session.yaml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GasSection {
    pub session_cap: Option<u64>,
    pub per_message_cost: Option<u64>,
    pub alert_threshold: Option<f64>,
    pub hard_limit: Option<bool>,
    pub per_bot_allocation: Option<u64>,
    pub curator_allocation: Option<u64>,
}

impl GasSection {
    /// Convert YAML gas section to GasBudgetConfig, applying defaults for missing fields
    pub fn to_config(&self) -> GasBudgetConfig {
        GasBudgetConfig {
            session_cap: self.session_cap.unwrap_or(150000),
            per_message_cost: self.per_message_cost.unwrap_or(100),
            alert_threshold: self.alert_threshold.unwrap_or(0.7),
            hard_limit: self.hard_limit.unwrap_or(true),
            per_bot_allocation: self.per_bot_allocation.unwrap_or(15000),
            curator_allocation: self.curator_allocation.unwrap_or(25000),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParticipantEntry {
    pub agent: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub role: String,
    pub description: String,
    /// Template domains this participant owns. Used to populate capabilities
    /// for R4 capability intersection checks. Curator has no domains.
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BootstrapConfig {
    pub initial_message: InitialMessage,
    pub initial_reports: Vec<InitialReport>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitialMessage {
    pub from: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitialReport {
    pub from: String,
    pub content: String,
}

pub struct StandingSession {
    pub session_id: String,
    pub description: String,
    pub chat: EnsembleChat,
    pub participant_names: HashMap<WebID, String>,
    pub participant_descriptions: HashMap<WebID, String>,
    store: Option<Arc<dyn StandingSessionPort>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl StandingSession {
    pub fn from_config(config: StandingSessionConfig) -> Self {
        let curator_webid = WebID::from_persona(b"Curator");
        let mut chat = EnsembleChat::new(curator_webid);
        let mut participant_names = HashMap::new();

        participant_names.insert(curator_webid, "Curator".to_string());

        let mut participant_descriptions = HashMap::new();

        participant_descriptions.insert(curator_webid, "".to_string());

        // Build R7 bot lookup for domain→capability resolution
        let r7_bots: HashMap<String, R7BotIdentity> = default_r7_bots()
            .iter()
            .map(|b| (b.id.clone(), b.clone()))
            .collect();

        for entry in &config.participants {
            if entry.agent == "Curator" {
                participant_descriptions.insert(
                    WebID::from_persona(entry.agent.as_bytes()),
                    entry.description.clone(),
                );
                continue;
            }

            let webid = WebID::from_persona(entry.agent.as_bytes());
            let role = match entry.role.as_str() {
                "orchestrator" => ParticipantRole::Curator,
                _ => ParticipantRole::Custom(entry.role.clone()),
            };

            // Load capabilities from domains declared in YAML.
            // If the agent is a known R7.x bot, also include its R7 bot capabilities.
            let mut capabilities: Vec<String> = entry.domains.clone();

            // If this entry matches an R7 bot identity, use its domains
            // (the YAML domains take precedence, then fall back to R7 bot defaults)
            if capabilities.is_empty()
                && let Some(r7_bot) = r7_bots.get(&entry.agent)
            {
                capabilities = r7_bot.domains.clone();
            }

            let participant = ChatParticipant {
                webid,
                role,
                pod_id: None,
                capabilities,
            };

            chat.register_participant(participant);
            participant_names.insert(webid, entry.agent.clone());
            participant_descriptions.insert(webid, entry.description.clone());
        }

        info!(
            session_id = %config.session.id,
            participants = config.participants.len(),
            "Standing session created"
        );

        Self {
            session_id: config.session.id,
            description: config.session.description.clone(),
            chat,
            participant_names,
            participant_descriptions,
            store: None,
            event_sink: None,
        }
    }

    pub fn with_store(mut self, store: Arc<dyn StandingSessionPort>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set gas budget configuration, forwarding it to the inner EnsembleChat
    pub fn with_gas_budget(mut self, config: GasBudgetConfig) -> Self {
        self.chat = self.chat.with_gas_budget(config);
        self
    }

    /// Set CNS event sink for span emission
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink.clone());
        self.chat = self.chat.with_event_sink(sink);
        self
    }

    pub fn persist_session(&self, config_yaml: &str) -> Result<(), StandingSessionError> {
        if let Some(ref store) = self.store {
            let now = chrono::Utc::now().to_rfc3339();
            let record = SessionRecord {
                session_id: self.session_id.clone(),
                config_yaml: config_yaml.to_string(),
                created_at: now.clone(),
                last_active: now,
            };
            store.save_session(&record)?;
        }
        Ok(())
    }

    pub fn persist_message(&self, message: &ChatMessage) -> Result<(), StandingSessionError> {
        if let Some(ref store) = self.store {
            let record = MessageRecord {
                id: 0,
                session_id: self.session_id.clone(),
                from_webid: message.from.to_string(),
                content: message.content.clone(),
                timestamp: message.timestamp.to_rfc3339(),
                template_id: message.template_id.clone(),
            };
            store.save_message(&record)?;
            store.update_last_active(&self.session_id)?;
        }
        if let Some(ref sink) = self.event_sink {
            let span = Span::new(
                SpanNamespace::new("cns.gas"),
                "standing_session.message_persisted",
            );
            let event = NuEvent::new(
                message.from,
                span,
                Phase::Act,
                serde_json::json!({
                    "from": message.from.to_string(),
                    "content_len": message.content.len(),
                    "template_id": message.template_id,
                    "session_id": self.session_id,
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(target: "cns.gas", error = %e, "Failed to persist message_persisted NuEvent");
            }
        }
        Ok(())
    }

    pub fn load_messages_from_storage(&mut self) -> Result<(), StandingSessionError> {
        if let Some(ref store) = self.store {
            let messages = store.get_messages(&self.session_id)?;

            for stored in messages {
                let webid: WebID = stored.from_webid.parse().map_err(|e| {
                    StandingSessionError::Storage(hkask_types::ports::SessionStoreError::Storage(
                        format!("Invalid WebID in stored message: {e}"),
                    ))
                })?;
                let mut msg = ChatMessage::new(webid, stored.content);
                msg.timestamp = chrono::DateTime::parse_from_rfc3339(&stored.timestamp)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                msg.template_id = stored.template_id;
                // Use add_restored_message to pre-register in dedup and skip gas
                self.chat.add_restored_message(msg);
            }

            info!(
                session_id = %self.session_id,
                messages = self.chat.get_history().len(),
                "Messages loaded from storage"
            );
        }
        Ok(())
    }

    pub fn post_initial_messages(&mut self, config: &StandingSessionConfig) {
        let curator_webid = *self.chat.curator();
        let initial_from = config
            .participants
            .iter()
            .find(|p| p.agent == config.bootstrap.initial_message.from)
            .map(|p| WebID::from_persona(p.agent.as_bytes()))
            .unwrap_or(curator_webid);
        let initial_msg = ChatMessage::new(
            initial_from,
            config.bootstrap.initial_message.content.clone(),
        );
        self.chat.add_message(initial_msg.clone());
        if let Err(e) = self.persist_message(&initial_msg) {
            tracing::warn!(target: "standing_session", error = %e, "Failed to persist initial message");
        }

        info!(
            session_id = %self.session_id,
            from = %config.bootstrap.initial_message.from,
            message_type = %config.bootstrap.initial_message.message_type,
            "Initial message posted"
        );

        for report in &config.bootstrap.initial_reports {
            let webid = WebID::from_persona(report.from.as_bytes());
            let msg = ChatMessage::new(webid, report.content.clone());
            self.chat.add_message(msg.clone());
            if let Err(e) = self.persist_message(&msg) {
                tracing::warn!(target: "standing_session", error = %e, "Failed to persist report message");
            }
        }

        info!(
            session_id = %self.session_id,
            messages = self.chat.get_history().len(),
            "Initial messages posted"
        );
    }

    pub fn get_status(&self) -> StandingSessionStatus {
        let participants: Vec<ParticipantStatus> = self
            .participant_names
            .iter()
            .map(|(webid, name)| ParticipantStatus {
                name: name.clone(),
                webid: webid.to_string(),
                role: self
                    .chat
                    .get_participants()
                    .get(webid)
                    .map(|p| format!("{:?}", p.role))
                    .unwrap_or_else(|| "unknown".to_string()),
                description: self
                    .participant_descriptions
                    .get(webid)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect();

        StandingSessionStatus {
            session_id: self.session_id.clone(),
            description: self.description.clone(),
            participant_count: participants.len(),
            message_count: self.chat.get_history().len(),
            participants,
        }
    }
}

#[derive(Debug)]
pub struct StandingSessionStatus {
    pub session_id: String,
    pub description: String,
    pub participant_count: usize,
    pub message_count: usize,
    pub participants: Vec<ParticipantStatus>,
}

#[derive(Debug)]
pub struct ParticipantStatus {
    pub name: String,
    pub webid: String,
    pub role: String,
    pub description: String,
}

pub fn load_standing_session_config(
    path: &Path,
) -> Result<StandingSessionConfig, StandingSessionError> {
    let content = std::fs::read_to_string(path)?;
    let config: StandingSessionConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

pub fn bootstrap_standing_session(path: &Path) -> Result<StandingSession, StandingSessionError> {
    let config = load_standing_session_config(path)?;
    let mut session = StandingSession::from_config(config.clone());
    if let Some(ref gas) = config.gas {
        session = session.with_gas_budget(gas.to_config());
    }
    session.post_initial_messages(&config);
    Ok(session)
}

pub fn bootstrap_standing_session_with_store(
    path: &Path,
    store: Arc<dyn StandingSessionPort>,
) -> Result<StandingSession, StandingSessionError> {
    let config = load_standing_session_config(path)?;
    let config_yaml = std::fs::read_to_string(path)?;

    // Check if session already exists in storage
    let session_exists = store.get_session(&config.session.id).is_ok();

    let mut session = StandingSession::from_config(config.clone()).with_store(store.clone());

    // Wire gas budget from YAML if present
    if let Some(ref gas) = config.gas {
        session = session.with_gas_budget(gas.to_config());
    }

    if session_exists {
        session.load_messages_from_storage()?;
        info!(
            session_id = %session.session_id,
            "Restored standing session from storage"
        );
    } else {
        session.persist_session(&config_yaml)?;
        session.post_initial_messages(&config);
        info!(
            session_id = %session.session_id,
            "Created new standing session with persistence"
        );
    }

    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_section_to_config_defaults() {
        let gas = GasSection {
            session_cap: None,
            per_message_cost: None,
            alert_threshold: None,
            hard_limit: None,
            per_bot_allocation: None,
            curator_allocation: None,
        };
        let config = gas.to_config();
        let default = GasBudgetConfig::default();
        assert_eq!(config.session_cap, default.session_cap);
        assert_eq!(config.per_message_cost, default.per_message_cost);
        assert!((config.alert_threshold - default.alert_threshold).abs() < f64::EPSILON);
        assert_eq!(config.hard_limit, default.hard_limit);
        assert_eq!(config.per_bot_allocation, default.per_bot_allocation);
        assert_eq!(config.curator_allocation, default.curator_allocation);
    }

    #[test]
    fn gas_section_to_config_custom() {
        let gas = GasSection {
            session_cap: Some(500000),
            per_message_cost: Some(200),
            alert_threshold: Some(0.9),
            hard_limit: Some(false),
            per_bot_allocation: Some(30000),
            curator_allocation: Some(50000),
        };
        let config = gas.to_config();
        assert_eq!(config.session_cap, 500000);
        assert_eq!(config.per_message_cost, 200);
        assert!((config.alert_threshold - 0.9).abs() < f64::EPSILON);
        assert!(!config.hard_limit);
        assert_eq!(config.per_bot_allocation, 30000);
        assert_eq!(config.curator_allocation, 50000);
    }

    #[test]
    fn gas_section_to_config_partial() {
        let gas = GasSection {
            session_cap: Some(999999),
            per_message_cost: None,
            alert_threshold: None,
            hard_limit: None,
            per_bot_allocation: None,
            curator_allocation: None,
        };
        let config = gas.to_config();
        let default = GasBudgetConfig::default();
        assert_eq!(config.session_cap, 999999);
        assert_eq!(config.per_message_cost, default.per_message_cost);
        assert!((config.alert_threshold - default.alert_threshold).abs() < f64::EPSILON);
        assert_eq!(config.hard_limit, default.hard_limit);
        assert_eq!(config.per_bot_allocation, default.per_bot_allocation);
        assert_eq!(config.curator_allocation, default.curator_allocation);
    }

    #[test]
    fn standing_session_config_parse_minimal_yaml() {
        let yaml = r#"
session:
  id: test-session
  name: Test
  description: A test session
participants:
  - agent: Curator
    type: replicant
    role: orchestrator
    description: The curator
bootstrap:
  initial_message:
    from: Curator
    type: greeting
    content: Hello
  initial_reports: []
"#;
        let config: StandingSessionConfig =
            serde_yaml::from_str(yaml).expect("failed to parse YAML");
        assert_eq!(config.session.id, "test-session");
        assert_eq!(config.session.name, "Test");
        assert_eq!(config.participants.len(), 1);
        assert_eq!(config.participants[0].agent, "Curator");
        assert_eq!(config.bootstrap.initial_message.from, "Curator");
        assert_eq!(config.bootstrap.initial_message.content, "Hello");
    }
}
