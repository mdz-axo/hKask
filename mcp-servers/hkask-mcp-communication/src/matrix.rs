//! Embedded Matrix infrastructure for agent-to-agent communication.
//!
//! Conduit is embedded as a library dependency providing a lightweight, Rust-native
//! Matrix homeserver. Iamb provides a terminal-based Matrix client for human users.
//! Each hKask install hosts its own Conduit instance; federation is optional and
//! defaults to local-only for security isolation.
//!
//! In production, Conduit would be added as a cargo dependency:
//!   conduit = { git = "https://gitlab.com/famedly/conduit" }
//!
//! For now, this module defines the client API surface using Matrix protocol primitives
//! that would be backed by a proper Matrix SDK. The API is deliberately minimal:
//! - Room creation and invitations
//! - Message sending (text, structured JSON)
//! - Thread-based conversation management
//! - End-to-end encryption key management (via hkask-keystore)

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Matrix types ───────────────────────────────────────────────────────────

/// A Matrix room identifier (e.g., "!abc123:localhost").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomId(pub String);

impl RoomId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A Matrix user identifier (e.g., "@agent:localhost").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

impl UserId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A threaded conversation (Matrix room with thread support).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Matrix room ID.
    pub room_id: RoomId,
    /// Human-readable thread title.
    pub title: String,
    /// Participants in the thread.
    pub participants: Vec<UserId>,
    /// Whether this thread is monitored by an agent.
    pub monitored_by: Vec<UserId>,
    /// Whether this thread has been escalated.
    pub escalated: bool,
    /// Thread creation timestamp.
    pub created_at: i64,
}

/// A message in a Matrix room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMessage {
    /// Sender user ID.
    pub sender: UserId,
    /// Plain text body.
    pub body: String,
    /// Optional structured payload (JSON).
    pub structured: Option<serde_json::Value>,
    /// Message timestamp.
    pub timestamp: i64,
}

// ── Client errors ──────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum MatrixError {
    #[error("Matrix homeserver not available: {0}")]
    Unavailable(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Room error: {0}")]
    Room(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
}

// ── Matrix client ──────────────────────────────────────────────────────────

/// Thin client wrapper around the embedded Matrix homeserver.
///
/// In production, this would wrap a proper Matrix SDK client (e.g., matrix-sdk).
/// For now, all operations are stubs that demonstrate the API contract.
pub struct MatrixClient {
    /// Homeserver URL (e.g., "http://localhost:8008").
    homeserver_url: String,
    /// Whether the homeserver is available.
    available: bool,
}

impl Clone for MatrixClient {
    fn clone(&self) -> Self {
        Self {
            homeserver_url: self.homeserver_url.clone(),
            available: self.available,
        }
    }
}

impl MatrixClient {
    /// Create a new Matrix client pointed at the given homeserver URL.
    pub fn new(homeserver_url: &str) -> Self {
        Self {
            homeserver_url: homeserver_url.to_string(),
            available: false,
        }
    }

    /// Check homeserver health.
    pub async fn health_check(&mut self) -> Result<bool, MatrixError> {
        // Would perform GET /_matrix/client/versions to verify Conduit is running.
        self.available = true;
        tracing::info!(
            target: "cns.communication.matrix.health",
            url = %self.homeserver_url,
            "Matrix homeserver healthy"
        );
        Ok(true)
    }

    /// Register a new Matrix user.
    pub async fn register_user(
        &self,
        username: &str,
        _password: &str,
    ) -> Result<UserId, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available. Call health_check() first.".to_string(),
            ));
        }
        // Would call POST /_matrix/client/v3/register
        tracing::info!(
            target: "cns.communication.agent.registered",
            username = %username,
            homeserver = %self.homeserver_url,
            "Agent registered as Matrix user"
        );
        Ok(UserId::new(&format!("@{}:localhost", username)))
    }

    /// Login as an existing Matrix user.
    pub async fn login(&self, username: &str, _password: &str) -> Result<String, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        // Would call POST /_matrix/client/v3/login
        tracing::info!(target: "cns.communication.matrix.login", username = %username, "Matrix login");
        Ok("mock-access-token".to_string())
    }

    /// Create a new room (thread).
    pub async fn create_room(
        &self,
        name: &str,
        _topic: Option<&str>,
    ) -> Result<RoomId, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        let room_id = format!("!{}:localhost", uuid::Uuid::new_v4());
        tracing::info!(
            target: "cns.communication.thread.created",
            room_id = %room_id,
            name = %name,
            "Matrix room created"
        );
        Ok(RoomId::new(&room_id))
    }

    /// Invite a user to a room.
    pub async fn invite_user(&self, room_id: &RoomId, user_id: &UserId) -> Result<(), MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        tracing::info!(
            target: "cns.communication.agent.invited",
            room_id = %room_id.as_str(),
            user = %user_id.as_str(),
            "User invited to room"
        );
        Ok(())
    }

    /// Send a message to a room.
    pub async fn send_message(
        &self,
        room_id: &RoomId,
        _body: &str,
        _structured: Option<serde_json::Value>,
    ) -> Result<(), MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        tracing::info!(
            target: "cns.communication.message.sent",
            room_id = %room_id.as_str(),
            "Message sent"
        );
        Ok(())
    }

    /// List active rooms.
    pub async fn list_rooms(&self) -> Result<Vec<Thread>, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        // Would call GET /_matrix/client/v3/joined_rooms and resolve metadata.
        Ok(vec![])
    }

    /// Get messages from a room (with optional pagination).
    pub async fn get_messages(
        &self,
        _room_id: &RoomId,
        _limit: usize,
    ) -> Result<Vec<MatrixMessage>, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        // Would call GET /_matrix/client/v3/rooms/{roomId}/messages
        Ok(vec![])
    }

    /// Check for unread/flagged messages (for 7R7 moderation polling).
    pub async fn poll_unread(&self, _rooms: &[RoomId]) -> Result<Vec<MatrixMessage>, MatrixError> {
        if !self.available {
            return Err(MatrixError::Unavailable(
                "Homeserver not available".to_string(),
            ));
        }
        Ok(vec![])
    }
}

// ── Embedded homeserver lifecycle ──────────────────────────────────────────

/// Manages the Conduit homeserver lifecycle within the hKask process.
///
/// Conduit is started as a background task, bound to localhost, with
/// credentials sourced from `hkask-keystore`. The server is local-only
/// by default; federation requires explicit configuration.
pub struct EmbeddedHomeserver {
    matrix: MatrixClient,
}

impl EmbeddedHomeserver {
    /// Create and start the embedded Conduit homeserver.
    ///
    /// In production, this would spawn Conduit as a library call with
    /// config derived from hKask settings. For now, a health-check stub.
    pub async fn start(homeserver_url: &str) -> Result<Self, MatrixError> {
        let mut matrix = MatrixClient::new(homeserver_url);
        matrix.health_check().await?;
        tracing::info!(
            target: "cns.communication.server.started",
            url = %homeserver_url,
            "Embedded Matrix homeserver started"
        );
        Ok(Self { matrix })
    }

    /// Return the underlying client for tool operations.
    pub fn client(&self) -> MatrixClient {
        MatrixClient::new(&self.matrix.homeserver_url)
    }

    /// Health check.
    pub async fn healthy(&mut self) -> bool {
        self.matrix.health_check().await.unwrap_or(false)
    }
}
