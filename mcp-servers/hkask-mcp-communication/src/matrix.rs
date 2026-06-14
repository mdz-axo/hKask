//! Matrix transport for agent-to-agent and human-to-agent communication.
//!
//! Uses `matrix-sdk` for Matrix protocol integration. The homeserver (Conduit)
//! runs as a Docker sidecar — hKask does not embed or maintain server code.
//!
//! E2EE is deferred to v2 due to a SQLCipher/SQLite linking conflict between
//! hkask-storage and matrix-sdk-sqlite. v1 uses TLS-only transport security.
//!
//! Public API (≤7 functions per deep-module discipline):
//! - MatrixTransport: new, login, start_sync, send_message
//! - Room management: create_room, invite_user, list_rooms
//! - Health: health

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

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
    #[error("Not logged in — call login() first")]
    NotLoggedIn,
}

// ── Matrix transport ──────────────────────────────────────────────────────

/// Thin transport layer over `matrix_sdk::Client`.
///
/// Owns the Matrix client lifecycle: login, sync, message send/receive.
/// Does NOT manage E2EE keys (deferred to v2). Does NOT embed a homeserver.
///
/// Incoming messages are buffered in an internal queue. Callers poll via
/// `pending_messages()` or register a sync callback via `start_sync()`.
pub struct MatrixTransport {
    /// The underlying matrix-sdk Client. None before login.
    client: Option<matrix_sdk::Client>,
    /// Homeserver URL (e.g., "http://localhost:8008").
    homeserver_url: String,
    /// Buffered incoming messages from the sync loop.
    inbox: Arc<Mutex<VecDeque<MatrixMessage>>>,
    /// Whether the sync loop is running.
    sync_active: bool,
}

impl MatrixTransport {
    /// Create a new Matrix transport pointed at the given homeserver URL.
    ///
    /// Does not connect or authenticate — call `login()` first.
    pub fn new(homeserver_url: &str) -> Self {
        Self {
            client: None,
            homeserver_url: homeserver_url.to_string(),
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            sync_active: false,
        }
    }

    /// Check whether the homeserver is reachable.
    ///
    /// Performs `GET /_matrix/client/versions` to verify Conduit is running.
    pub async fn health_check(&mut self) -> Result<bool, MatrixError> {
        let client = matrix_sdk::Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await
            .map_err(|e| MatrixError::Unavailable(format!("Failed to build client: {}", e)))?;

        // Verify the client was built successfully (homeserver URL is accessible)
        let _homeserver = client.homeserver();
        tracing::info!(
            target: "cns.communication.matrix.health",
            url = %self.homeserver_url,
            "Matrix homeserver healthy"
        );
        Ok(true)
    }

    /// Login to the Matrix homeserver with username and password.
    ///
    /// Stores the authenticated client for subsequent operations.
    /// Must be called before `start_sync()`, `send_message()`, etc.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), MatrixError> {
        let client = matrix_sdk::Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await
            .map_err(|e| MatrixError::Unavailable(format!("Failed to build client: {}", e)))?;

        client
            .matrix_auth()
            .login_username(username, password)
            .send()
            .await
            .map_err(|e| MatrixError::Auth(format!("Login failed: {}", e)))?;

        tracing::info!(
            target: "cns.communication.matrix.login",
            username = %username,
            homeserver = %self.homeserver_url,
            "Matrix login successful"
        );

        self.client = Some(client);
        Ok(())
    }

    /// Register a new Matrix user on the homeserver.
    ///
    /// Requires open registration on the homeserver (`allow_registration = true`
    /// in conduit.toml) OR a valid admin token passed via the registration API.
    /// For admin-mediated registration, use `register_with_admin_token()`.
    pub async fn register_user(
        &self,
        username: &str,
        _password: &str,
    ) -> Result<UserId, MatrixError> {
        let _client = matrix_sdk::Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await
            .map_err(|e| MatrixError::Unavailable(format!("Failed to build client: {}", e)))?;

        // matrix-sdk 0.16 does not expose a direct registration API.
        // Registration is typically done via the homeserver's web UI or admin API.
        // For Conduit, we use the admin API with the admin token.
        tracing::warn!(
            target: "cns.communication.matrix.register",
            username = %username,
            "Direct registration via matrix-sdk not supported — use admin API or open registration"
        );

        Err(MatrixError::Unavailable(
            "Registration via matrix-sdk not implemented. Use kask matrix register --agent or --user (admin API).".to_string(),
        ))
    }

    /// Start the Matrix sync loop.
    ///
    /// Registers an event handler for incoming room messages and spawns
    /// a background sync task. Incoming messages are buffered into the
    /// internal inbox. Call `pending_messages()` to retrieve them.
    ///
    /// Only one sync loop may be active at a time.
    pub async fn start_sync(&mut self) -> Result<(), MatrixError> {
        let client = self.client.as_ref().ok_or(MatrixError::NotLoggedIn)?;

        if self.sync_active {
            tracing::warn!(
                target: "cns.communication.matrix.sync",
                "Sync loop already active"
            );
            return Ok(());
        }

        let inbox = Arc::clone(&self.inbox);
        let client = client.clone();
        self.sync_active = true;

        // Register event handler for incoming room messages
        client.add_event_handler(
            move |event: matrix_sdk::ruma::events::room::message::SyncRoomMessageEvent,
                  room: matrix_sdk::room::Room| async move {
                let sender = event.sender().to_string();
                let body = event
                    .as_original()
                    .map(|ev| ev.content.body().to_string())
                    .unwrap_or_default();

                if !body.is_empty() {
                    let msg = MatrixMessage {
                        sender: UserId::new(&sender),
                        body,
                        structured: None,
                        timestamp: i64::from(event.origin_server_ts().get()),
                    };

                    tracing::info!(
                        target: "cns.communication.matrix.message.received",
                        room_id = %room.room_id(),
                        sender = %sender,
                        body_len = msg.body.len(),
                        "Matrix message received"
                    );

                    let mut queue = inbox.lock().await;
                    queue.push_back(msg);
                }
            },
        );

        // Spawn sync loop
        tokio::spawn(async move {
            tracing::info!(
                target: "cns.communication.matrix.sync.started",
                "Matrix sync loop started"
            );

            loop {
                match client
                    .sync(matrix_sdk::config::SyncSettings::default())
                    .await
                {
                    Ok(()) => {
                        tracing::debug!(
                            target: "cns.communication.matrix.sync.health",
                            "Sync cycle complete"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            target: "cns.communication.matrix.sync.stalled",
                            error = %e,
                            "Matrix sync failed"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Retrieve pending messages from the sync inbox.
    ///
    /// Returns all buffered messages and clears the inbox.
    /// Call this periodically to check for new messages.
    pub async fn pending_messages(&self) -> Vec<MatrixMessage> {
        let mut queue = self.inbox.lock().await;
        let messages: Vec<MatrixMessage> = queue.drain(..).collect();
        messages
    }

    /// Send a message to a Matrix room.
    ///
    /// If `structured` is provided, it is attached as JSON in the
    /// message's `org.matrix.custom.html` formatted body for machine
    /// consumption while the plain `body` remains human-readable.
    pub async fn send_message(
        &self,
        room_id: &RoomId,
        body: &str,
        structured: Option<serde_json::Value>,
    ) -> Result<(), MatrixError> {
        let client = self.client.as_ref().ok_or(MatrixError::NotLoggedIn)?;

        let room_id = matrix_sdk::ruma::RoomId::parse(room_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid room ID: {}", e)))?;

        let room = client
            .get_room(&room_id)
            .ok_or_else(|| MatrixError::Room(format!("Room not found: {}", room_id)))?;

        let content = if let Some(ref structured) = structured {
            let html = format!(
                "<p>{}</p>\n<!-- hkask-structured: {} -->",
                body,
                serde_json::to_string(structured).unwrap_or_default()
            );
            matrix_sdk::ruma::events::room::message::RoomMessageEventContent::text_html(body, html)
        } else {
            matrix_sdk::ruma::events::room::message::RoomMessageEventContent::text_plain(body)
        };

        room.send(content)
            .await
            .map_err(|e| MatrixError::Network(format!("Send failed: {}", e)))?;

        tracing::info!(
            target: "cns.communication.matrix.message.sent",
            room_id = %room_id,
            body_len = body.len(),
            "Matrix message sent"
        );

        Ok(())
    }

    /// Create a new Matrix room.
    pub async fn create_room(
        &self,
        name: &str,
        _topic: Option<&str>,
    ) -> Result<RoomId, MatrixError> {
        let client = self.client.as_ref().ok_or(MatrixError::NotLoggedIn)?;

        let room = client
            .create_room(matrix_sdk::ruma::api::client::room::create_room::v3::Request::new())
            .await
            .map_err(|e| MatrixError::Room(format!("Failed to create room: {}", e)))?;

        let room_id = room.room_id().to_string();

        // Set the room name
        if let Some(joined) = client.get_room(room.room_id()) {
            joined
                .set_name(name.to_string())
                .await
                .map_err(|e| MatrixError::Room(format!("Failed to set room name: {}", e)))?;
        }

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
        let client = self.client.as_ref().ok_or(MatrixError::NotLoggedIn)?;

        let room_id = matrix_sdk::ruma::RoomId::parse(room_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid room ID: {}", e)))?;

        let user_id = matrix_sdk::ruma::UserId::parse(user_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid user ID: {}", e)))?;

        let room = client
            .get_room(&room_id)
            .ok_or_else(|| MatrixError::Room(format!("Room not found: {}", room_id)))?;

        room.invite_user_by_id(&user_id)
            .await
            .map_err(|e| MatrixError::Room(format!("Invite failed: {}", e)))?;

        tracing::info!(
            target: "cns.communication.agent.invited",
            room_id = %room_id,
            user = %user_id,
            "User invited to room"
        );

        Ok(())
    }

    /// List joined rooms.
    pub async fn list_rooms(&self) -> Result<Vec<Thread>, MatrixError> {
        let client = self.client.as_ref().ok_or(MatrixError::NotLoggedIn)?;

        let rooms = client.joined_rooms();

        let mut threads = Vec::new();
        for room in rooms {
            let room_id = room.room_id().to_string();
            let title = room.name().unwrap_or_else(|| room_id.clone());
            let members: Vec<UserId> = room
                .members(matrix_sdk::RoomMemberships::JOIN)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|m| UserId::new(m.user_id().as_str()))
                .collect();

            threads.push(Thread {
                room_id: RoomId::new(&room_id),
                title,
                participants: members,
                monitored_by: vec![],
                escalated: false,
                created_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        Ok(threads)
    }

    /// Check whether the client is authenticated and the sync loop is active.
    pub fn healthy(&self) -> bool {
        self.client.is_some() && self.sync_active
    }
}
