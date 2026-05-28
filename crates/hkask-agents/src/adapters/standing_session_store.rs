//! Standing Session Store Adapter — Bridges hkask_storage::StandingSessionStore to StandingSessionPort

use crate::ports::{
    AcpSessionMessage, BotReport, MessageRecord, SessionRecord, StandingSessionPort,
    StandingSessionPortError,
};
use hkask_storage::StandingSessionStore;
use std::sync::Arc;

pub struct StandingSessionStoreAdapter {
    store: Arc<StandingSessionStore>,
}

impl StandingSessionStoreAdapter {
    pub fn new(store: Arc<StandingSessionStore>) -> Self {
        Self { store }
    }
}

impl StandingSessionPort for StandingSessionStoreAdapter {
    fn save_session(&self, session: &SessionRecord) -> Result<(), StandingSessionPortError> {
        let stored = hkask_storage::StoredSession {
            session_id: session.session_id.clone(),
            config_yaml: session.config_yaml.clone(),
            created_at: session.created_at.clone(),
            last_active: session.last_active.clone(),
        };
        self.store
            .save_session(&stored)
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn get_session(&self, session_id: &str) -> Result<SessionRecord, StandingSessionPortError> {
        self.store
            .get_session(session_id)
            .map(|s| SessionRecord {
                session_id: s.session_id,
                config_yaml: s.config_yaml,
                created_at: s.created_at,
                last_active: s.last_active,
            })
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn save_message(&self, message: &MessageRecord) -> Result<i64, StandingSessionPortError> {
        let stored = hkask_storage::StoredMessage {
            id: message.id,
            session_id: message.session_id.clone(),
            from_webid: message.from_webid.clone(),
            content: message.content.clone(),
            timestamp: message.timestamp.clone(),
            template_id: message.template_id.clone(),
        };
        self.store
            .save_message(&stored)
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn get_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<MessageRecord>, StandingSessionPortError> {
        self.store
            .get_messages(session_id)
            .map(|v| {
                v.into_iter()
                    .map(|m| MessageRecord {
                        id: m.id,
                        session_id: m.session_id,
                        from_webid: m.from_webid,
                        content: m.content,
                        timestamp: m.timestamp,
                        template_id: m.template_id,
                    })
                    .collect()
            })
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn update_last_active(&self, session_id: &str) -> Result<(), StandingSessionPortError> {
        self.store
            .update_last_active(session_id)
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn submit_bot_report(&self, report: &BotReport) -> Result<(), StandingSessionPortError> {
        let stored = hkask_storage::StoredMessage {
            id: 0,
            session_id: format!("bot-report-{}", report.bot_name),
            from_webid: report.bot_id.clone(),
            content: serde_json::to_string(report).unwrap_or_else(|_| report.bot_name.clone()),
            timestamp: report.timestamp.clone(),
            template_id: Some("bot_report".to_string()),
        };
        self.store
            .save_message(&stored)
            .map(|_| ())
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }

    fn get_bot_reports(
        &self,
        session_id: &str,
        bot_name: &str,
    ) -> Result<Vec<BotReport>, StandingSessionPortError> {
        let messages = self
            .store
            .get_messages(session_id)
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))?;
        let mut reports = Vec::new();
        for msg in messages {
            if msg.template_id.as_deref() == Some("bot_report") {
                if let Ok(report) = serde_json::from_str::<BotReport>(&msg.content) {
                    if report.bot_name == bot_name {
                        reports.push(report);
                    }
                }
            }
        }
        Ok(reports)
    }

    fn route_acp_message(
        &self,
        message: &AcpSessionMessage,
    ) -> Result<(), StandingSessionPortError> {
        let stored = hkask_storage::StoredMessage {
            id: 0,
            session_id: "system-coordination-standing-session".to_string(),
            from_webid: message.from_webid.clone(),
            content: serde_json::json!({
                "type": message.message_type,
                "target": message.target,
                "content": message.content,
            })
            .to_string(),
            timestamp: message.timestamp.clone(),
            template_id: Some("acp_message".to_string()),
        };
        self.store
            .save_message(&stored)
            .map(|_| ())
            .map_err(|e| StandingSessionPortError::Storage(e.to_string()))
    }
}
