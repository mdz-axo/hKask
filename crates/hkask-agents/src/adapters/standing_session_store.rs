//! Standing Session Store Adapter — Bridges hkask_storage::StandingSessionStore to StandingSessionPort

use hkask_storage::StandingSessionStore;
use hkask_types::ports::{MessageRecord, SessionRecord, SessionStoreError, StandingSessionPort};
use std::sync::Arc;

pub(crate) struct StandingSessionStoreAdapter {
    store: Arc<StandingSessionStore>,
}

impl StandingSessionStoreAdapter {
    pub fn new(store: Arc<StandingSessionStore>) -> Self {
        Self { store }
    }
}

fn map_storage_err(e: hkask_storage::standing_session::StandingSessionError) -> SessionStoreError {
    match e {
        hkask_storage::standing_session::StandingSessionError::NotFound(id) => {
            SessionStoreError::NotFound(id)
        }
        hkask_storage::standing_session::StandingSessionError::Sealed(id) => {
            SessionStoreError::Sealed(id)
        }
        hkask_storage::standing_session::StandingSessionError::Infra(ie) => {
            SessionStoreError::Storage(ie.to_string())
        }
    }
}

impl StandingSessionPort for StandingSessionStoreAdapter {
    fn save_session(&self, session: &SessionRecord) -> Result<(), SessionStoreError> {
        // Derive key_version from the store's current version.
        let key_version = self.store.current_key_version().unwrap_or(1);
        let stored = hkask_storage::StoredSession {
            session_id: session.session_id.clone(),
            config_yaml: session.config_yaml.clone(),
            created_at: session.created_at.clone(),
            last_active: session.last_active.clone(),
            key_version,
            sealed: false,
        };
        self.store.save_session(&stored).map_err(map_storage_err)
    }

    fn get_session(&self, session_id: &str) -> Result<SessionRecord, SessionStoreError> {
        self.store
            .get_session(session_id)
            .map_err(map_storage_err)
            .map(|s| SessionRecord {
                session_id: s.session_id,
                config_yaml: s.config_yaml,
                created_at: s.created_at,
                last_active: s.last_active,
            })
    }

    fn save_message(&self, message: &MessageRecord) -> Result<i64, SessionStoreError> {
        let stored = hkask_storage::StoredMessage {
            id: message.id,
            session_id: message.session_id.clone(),
            from_webid: message.from_webid.clone(),
            content: message.content.clone(),
            timestamp: message.timestamp.clone(),
            template_id: message.template_id.clone(),
        };
        self.store.save_message(&stored).map_err(map_storage_err)
    }

    fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>, SessionStoreError> {
        self.store
            .get_messages(session_id)
            .map_err(map_storage_err)
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
    }

    fn update_last_active(&self, session_id: &str) -> Result<(), SessionStoreError> {
        self.store
            .update_last_active(session_id)
            .map_err(map_storage_err)
    }
}
