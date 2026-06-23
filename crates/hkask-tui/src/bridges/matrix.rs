//! MatrixDataBridge — trait for Matrix rooms and messages in the TUI.
//!
//! Provides the Matrix window with live room, message, and participant
//! data from hkask-communication / matrix-sdk.

use std::sync::Arc;

/// Connection status snapshot.
#[derive(Debug, Clone)]
pub struct MatrixConnectionStatus {
    pub connected: bool,
    pub homeserver: String,
    pub user_id: Option<String>,
}

/// Summary of a Matrix room.
#[derive(Debug, Clone)]
pub struct MatrixRoomSummary {
    pub id: String,
    pub title: String,
    pub member_count: usize,
    pub escalated: bool,
    pub last_active: String,
}

/// Summary of a single message.
#[derive(Debug, Clone)]
pub struct MatrixMessageSummary {
    pub sender: String,
    pub body: String,
    pub timestamp: String,
}

/// Trait for querying Matrix subsystem state.
pub trait MatrixDataBridge: Send + Sync {
    fn connection_status(&self) -> MatrixConnectionStatus;
    fn list_rooms(&self) -> Vec<MatrixRoomSummary>;
    fn recent_messages(&self, room_id: &str, limit: usize) -> Vec<MatrixMessageSummary>;
    fn room_count(&self) -> usize;
}

/// Mock implementation for TUI development and testing.
pub struct MockMatrixBridge {
    pub connected: bool,
    pub homeserver: String,
    pub user_id: Option<String>,
    pub rooms: Vec<MatrixRoomSummary>,
    pub messages: Vec<MatrixMessageSummary>,
}

impl MockMatrixBridge {
    pub fn new() -> Self {
        Self {
            connected: false,
            homeserver: "matrix.local".into(),
            user_id: None,
            rooms: Vec::new(),
            messages: Vec::new(),
        }
    }

    pub fn connected() -> Self {
        Self {
            connected: true,
            homeserver: "matrix.local".into(),
            user_id: Some("@agent:matrix.local".into()),
            rooms: vec![
                MatrixRoomSummary {
                    id: "!room1:matrix.local".into(),
                    title: "General".into(),
                    member_count: 3,
                    escalated: false,
                    last_active: "2026-06-23T10:00:00Z".into(),
                },
                MatrixRoomSummary {
                    id: "!room2:matrix.local".into(),
                    title: "Dev".into(),
                    member_count: 2,
                    escalated: false,
                    last_active: "2026-06-23T09:30:00Z".into(),
                },
                MatrixRoomSummary {
                    id: "!room3:matrix.local".into(),
                    title: "Alerts".into(),
                    member_count: 1,
                    escalated: true,
                    last_active: "2026-06-22T18:00:00Z".into(),
                },
            ],
            messages: vec![
                MatrixMessageSummary {
                    sender: "user".into(),
                    body: "Is the TUI ready?".into(),
                    timestamp: "10:00".into(),
                },
                MatrixMessageSummary {
                    sender: "agent".into(),
                    body: "Yes, 19 windows wired.".into(),
                    timestamp: "10:01".into(),
                },
                MatrixMessageSummary {
                    sender: "user".into(),
                    body: "Great, let's ship it.".into(),
                    timestamp: "10:02".into(),
                },
            ],
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl MatrixDataBridge for MockMatrixBridge {
    fn connection_status(&self) -> MatrixConnectionStatus {
        MatrixConnectionStatus {
            connected: self.connected,
            homeserver: self.homeserver.clone(),
            user_id: self.user_id.clone(),
        }
    }
    fn list_rooms(&self) -> Vec<MatrixRoomSummary> {
        self.rooms.clone()
    }
    fn recent_messages(&self, _room_id: &str, limit: usize) -> Vec<MatrixMessageSummary> {
        self.messages.iter().take(limit).cloned().collect()
    }
    fn room_count(&self) -> usize {
        self.rooms.len()
    }
}
