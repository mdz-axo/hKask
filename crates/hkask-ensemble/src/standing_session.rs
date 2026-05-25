//! Standing Ensemble Session — Bootstrap and lifecycle management
//!
//! The standing session is the persistent coordination channel where the 7R7 bots
//! report status and the Curator orchestrates metacognition.

use crate::chat::{ChatMessage, ChatParticipant, EnsembleChat, ParticipantRole};
use hkask_agents::ports::{MessageRecord, SessionRecord, StandingSessionPort};
use hkask_types::WebID;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum StandingSessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct StandingSessionConfig {
    pub session: SessionMetadata,
    pub participants: Vec<ParticipantEntry>,
    #[allow(dead_code)]
    pub rules: SessionRules,
    #[allow(dead_code)]
    pub bootstrap: BootstrapConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParticipantEntry {
    pub agent: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub role: String,
    #[allow(dead_code)]
    pub voting: bool,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionRules {
    #[allow(dead_code)]
    pub consensus_required: bool,
    #[allow(dead_code)]
    pub orchestration_model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    #[allow(dead_code)]
    pub auto_start: bool,
    pub initial_message: InitialMessage,
    pub initial_reports: Vec<InitialReport>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitialMessage {
    #[allow(dead_code)]
    pub from: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitialReport {
    pub from: String,
    pub content: String,
}

pub struct StandingSession {
    pub session_id: String,
    pub chat: EnsembleChat,
    pub participant_names: HashMap<WebID, String>,
    store: Option<Arc<dyn StandingSessionPort>>,
}

impl StandingSession {
    pub fn from_config(config: StandingSessionConfig) -> Self {
        let curator_webid = WebID::from_persona(b"Curator");
        let mut chat = EnsembleChat::new(curator_webid);
        let mut participant_names = HashMap::new();

        participant_names.insert(curator_webid, "Curator".to_string());

        for entry in &config.participants {
            if entry.agent == "Curator" {
                continue;
            }

            let webid = WebID::from_persona(entry.agent.as_bytes());
            let role = match entry.role.as_str() {
                "orchestrator" => ParticipantRole::Curator,
                _ => ParticipantRole::Custom(entry.agent.clone()),
            };

            let participant = ChatParticipant {
                webid,
                role,
                pod_id: None,
                capabilities: vec![],
            };

            chat.register_participant(participant);
            participant_names.insert(webid, entry.agent.clone());
        }

        info!(
            session_id = %config.session.id,
            participants = config.participants.len(),
            "Standing session created"
        );

        Self {
            session_id: config.session.id,
            chat,
            participant_names,
            store: None,
        }
    }

    pub fn with_store(mut self, store: Arc<dyn StandingSessionPort>) -> Self {
        self.store = Some(store);
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
            store
                .save_session(&record)
                .map_err(|e| StandingSessionError::Storage(e.to_string()))?;
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
            store
                .save_message(&record)
                .map_err(|e| StandingSessionError::Storage(e.to_string()))?;
            store
                .update_last_active(&self.session_id)
                .map_err(|e| StandingSessionError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    pub fn load_messages_from_storage(&mut self) -> Result<(), StandingSessionError> {
        if let Some(ref store) = self.store {
            let messages = store
                .get_messages(&self.session_id)
                .map_err(|e| StandingSessionError::Storage(e.to_string()))?;

            for stored in messages {
                let webid = WebID::from_string(&stored.from_webid);
                let mut msg = ChatMessage::new(webid, stored.content);
                msg.timestamp = chrono::DateTime::parse_from_rfc3339(&stored.timestamp)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                msg.template_id = stored.template_id;
                self.chat.add_message(msg);
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
        let initial_msg = ChatMessage::new(
            curator_webid,
            config.bootstrap.initial_message.content.clone(),
        );
        self.chat.add_message(initial_msg.clone());
        if let Err(e) = self.persist_message(&initial_msg) {
            tracing::warn!(target: "standing_session", error = %e, "Failed to persist initial message");
        }

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
            })
            .collect();

        StandingSessionStatus {
            session_id: self.session_id.clone(),
            participant_count: participants.len(),
            message_count: self.chat.get_history().len(),
            participants,
        }
    }
}

#[derive(Debug)]
pub struct StandingSessionStatus {
    pub session_id: String,
    pub participant_count: usize,
    pub message_count: usize,
    pub participants: Vec<ParticipantStatus>,
}

#[derive(Debug)]
pub struct ParticipantStatus {
    pub name: String,
    pub webid: String,
    pub role: String,
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

    const TEST_YAML: &str = r#"
session:
  id: test-standing-session
  name: Test Standing Session
  description: A test session

participants:
  - agent: Curator
    type: Replicant
    role: orchestrator
    voting: false
    description: Test curator

  - agent: test-bot-1
    type: Bot
    role: participant
    voting: true
    description: Test bot 1

  - agent: test-bot-2
    type: Bot
    role: participant
    voting: true
    description: Test bot 2

rules:
  consensus_required: false
  orchestration_model: curator_led

bootstrap:
  auto_start: true
  initial_message:
    from: Curator
    type: metacognition_update
    content: "Test session initialized"
  initial_reports:
    - from: test-bot-1
      content: "Bot 1 ready"
    - from: test-bot-2
      content: "Bot 2 ready"
"#;

    #[test]
    fn test_parse_config() {
        let config: StandingSessionConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        assert_eq!(config.session.id, "test-standing-session");
        assert_eq!(config.participants.len(), 3);
        assert_eq!(config.bootstrap.initial_reports.len(), 2);
    }

    #[test]
    fn test_from_config() {
        let config: StandingSessionConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        let session = StandingSession::from_config(config);

        assert_eq!(session.session_id, "test-standing-session");
        assert_eq!(session.participant_names.len(), 3);
        assert!(session.participant_names.values().any(|n| n == "Curator"));
        assert!(
            session
                .participant_names
                .values()
                .any(|n| n == "test-bot-1")
        );
        assert!(
            session
                .participant_names
                .values()
                .any(|n| n == "test-bot-2")
        );
    }

    #[test]
    fn test_post_initial_messages() {
        let config: StandingSessionConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        let mut session = StandingSession::from_config(config.clone());
        session.post_initial_messages(&config);

        let history = session.chat.get_history();
        assert_eq!(history.len(), 3);
        assert!(history[0].content.contains("Test session initialized"));
        assert!(history[1].content.contains("Bot 1 ready"));
        assert!(history[2].content.contains("Bot 2 ready"));
    }

    #[test]
    fn test_get_status() {
        let config: StandingSessionConfig = serde_yaml::from_str(TEST_YAML).unwrap();
        let mut session = StandingSession::from_config(config.clone());
        session.post_initial_messages(&config);

        let status = session.get_status();
        assert_eq!(status.session_id, "test-standing-session");
        assert_eq!(status.participant_count, 3);
        assert_eq!(status.message_count, 3);
    }

    #[test]
    fn test_bootstrap_from_file() {
        use std::io::Write;
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        write!(temp, "{}", TEST_YAML).unwrap();

        let session = bootstrap_standing_session(temp.path()).unwrap();
        assert_eq!(session.session_id, "test-standing-session");
        assert_eq!(session.chat.get_history().len(), 3);
    }
}
