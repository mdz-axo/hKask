//! Matrix client integration via `matrix-sdk`.
//!
//! Wraps `matrix_sdk::Client` to provide the communication server's tool surface:
//! room creation, invitations, message sending, thread management.
//!
//! The homeserver is expected to run as a Docker sidecar (Conduit) on the same
//! machine. See `scripts/conduit-docker.sh` for setup instructions.
//!
//! Architecture:
//!   hKask communication server → matrix_sdk::Client → Conduit (Docker sidecar)
//!
//! Federation is disabled by default. All rooms are local-only.

use matrix_sdk::{
    Client as MatrixSdkClient,
    ruma::{
        OwnedRoomId, OwnedUserId,
        api::client::{
            membership::invite_user::v3::{InvitationRecipient, Request as InviteUserRequest},
            room::create_room::v3::Request as CreateRoomRequest,
        },
        events::room::message::RoomMessageEventContent,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use url::Url;

// ── Matrix types (re-exported for tool surface) ────────────────────────────

/// A Matrix room identifier (e.g., "!abc123:localhost").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomIdStr(pub String);

impl RoomIdStr {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A Matrix user identifier (e.g., "@agent:localhost").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserIdStr(pub String);

impl UserIdStr {
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
    pub room_id: RoomIdStr,
    /// Human-readable thread title.
    pub title: String,
    /// Participants in the thread.
    pub participants: Vec<UserIdStr>,
    /// Whether this thread is monitored by an agent.
    pub monitored_by: Vec<UserIdStr>,
    /// Whether this thread has been escalated.
    pub escalated: bool,
    /// Thread creation timestamp.
    pub created_at: i64,
}

/// A message in a Matrix room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMessage {
    /// Sender user ID.
    pub sender: UserIdStr,
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
    #[error("SDK error: {0}")]
    Sdk(String),
}

// ── Matrix client ──────────────────────────────────────────────────────────

/// Wrapper around `matrix_sdk::Client` providing the communication server's
/// tool surface.
///
/// Manages login state and provides convenience methods for room creation,
/// invitations, and message sending. Thread safety via `Arc<RwLock<>>` on
/// the underlying SDK client.
pub struct MatrixClient {
    /// The underlying matrix-sdk client.
    client: Arc<RwLock<Option<MatrixSdkClient>>>,
    /// Homeserver URL.
    homeserver_url: String,
    /// Whether we're logged in.
    logged_in: RwLock<bool>,
    /// Current session user ID (if logged in).
    user_id: RwLock<Option<OwnedUserId>>,
}

impl Clone for MatrixClient {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            homeserver_url: self.homeserver_url.clone(),
            logged_in: RwLock::new(false),
            user_id: RwLock::new(None),
        }
    }
}

impl MatrixClient {
    /// Create a new Matrix client pointed at the given homeserver URL.
    pub fn new(homeserver_url: &str) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            homeserver_url: homeserver_url.to_string(),
            logged_in: RwLock::new(false),
            user_id: RwLock::new(None),
        }
    }

    /// Build the underlying SDK client and verify homeserver connectivity.
    ///
    /// Must be called before any other operations. Performs a GET
    /// `/_matrix/client/versions` to verify the server is reachable.
    pub async fn connect(&self) -> Result<(), MatrixError> {
        let url = Url::parse(&self.homeserver_url)
            .map_err(|e| MatrixError::Unavailable(format!("Invalid homeserver URL: {}", e)))?;

        let sdk_client = MatrixSdkClient::new(url).await.map_err(|e| {
            MatrixError::Unavailable(format!("Failed to connect to homeserver: {}", e))
        })?;

        // Verify server is reachable by checking supported versions
        let _versions = sdk_client.unstable_features().await.map_err(|_| {
            MatrixError::Unavailable("Homeserver did not respond to version check".to_string())
        })?;

        *self.client.write().await = Some(sdk_client);

        tracing::info!(
            target: "cns.communication.matrix.connected",
            url = %self.homeserver_url,
            "Matrix homeserver connected"
        );
        Ok(())
    }

    /// Get a reference to the underlying SDK client.
    async fn sdk(&self) -> Result<MatrixSdkClient, MatrixError> {
        let guard = self.client.read().await;
        guard.clone().ok_or_else(|| {
            MatrixError::Unavailable("Not connected. Call connect() first.".to_string())
        })
    }

    /// Register a new Matrix user on the homeserver.
    ///
    /// Conduit allows registration without email/token verification when
    /// `allow_registration` is enabled (our Docker setup enables this).
    pub async fn register_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserIdStr, MatrixError> {
        let sdk = self.sdk().await?;
        let full_username = format!("@{}:localhost", username);

        // We use the raw send() method to POST /_matrix/client/v3/register
        // matrix-sdk 0.16: RegistrationKind is private, so we construct the request
        // with direct field assignment and use m.login.dummy auth.
        use matrix_sdk::ruma::api::client::account::register::v3::Request as RegisterRequest;
        let mut request = RegisterRequest::new();
        request.username = Some(username.to_string());
        request.password = Some(password.to_string());
        request.initial_device_display_name = Some("hKask Agent".to_string());
        // Use m.login.dummy for registration without auth (Conduit allows this)
        request.auth = None;

        sdk.send(request)
            .await
            .map_err(|e| MatrixError::Auth(format!("Registration failed: {}", e)))?;

        // After registration, log in to get an access token
        let user_id_owned = OwnedUserId::try_from(full_username.as_str())
            .map_err(|e| MatrixError::Auth(format!("Invalid user ID: {}", e)))?;

        sdk.matrix_auth()
            .login_username(&user_id_owned, password)
            .send()
            .await
            .map_err(|e| MatrixError::Auth(format!("Login after registration failed: {}", e)))?;

        *self.logged_in.write().await = true;
        *self.user_id.write().await = Some(user_id_owned.clone());

        tracing::info!(
            target: "cns.communication.agent.registered",
            username = %username,
            homeserver = %self.homeserver_url,
            "Agent registered and logged in as Matrix user"
        );
        Ok(UserIdStr::new(&full_username))
    }

    /// Login as an existing Matrix user.
    pub async fn login(&self, username: &str, password: &str) -> Result<(), MatrixError> {
        let sdk = self.sdk().await?;
        let user_id = OwnedUserId::try_from(username)
            .map_err(|e| MatrixError::Auth(format!("Invalid user ID: {}", e)))?;

        sdk.matrix_auth()
            .login_username(&user_id, password)
            .send()
            .await
            .map_err(|e| MatrixError::Auth(format!("Login failed: {}", e)))?;

        *self.logged_in.write().await = true;
        *self.user_id.write().await = Some(user_id);

        tracing::info!(
            target: "cns.communication.matrix.login",
            username = %username,
            "Matrix login successful"
        );
        Ok(())
    }

    /// Create a new room (thread).
    pub async fn create_room(
        &self,
        name: &str,
        topic: Option<&str>,
    ) -> Result<RoomIdStr, MatrixError> {
        let sdk = self.sdk().await?;
        let mut request = CreateRoomRequest::new();
        request.name = Some(name.to_string());
        request.topic = topic.map(|t| t.to_string());

        let room = sdk
            .create_room(request)
            .await
            .map_err(|e| MatrixError::Room(format!("Failed to create room: {}", e)))?;

        let room_id = room.room_id().to_string();
        tracing::info!(
            target: "cns.communication.thread.created",
            room_id = %room_id,
            name = %name,
            "Matrix room created"
        );
        Ok(RoomIdStr::new(&room_id))
    }

    /// Invite a user to a room.
    pub async fn invite_user(
        &self,
        room_id: &RoomIdStr,
        user_id: &UserIdStr,
    ) -> Result<(), MatrixError> {
        let sdk = self.sdk().await?;
        let room_id_owned = OwnedRoomId::try_from(room_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid room ID: {}", e)))?;
        let user_id_owned = OwnedUserId::try_from(user_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid user ID: {}", e)))?;

        let request = InviteUserRequest::new(
            room_id_owned,
            InvitationRecipient::UserId {
                user_id: user_id_owned,
            },
        );
        sdk.send(request)
            .await
            .map_err(|e| MatrixError::Room(format!("Failed to invite user: {}", e)))?;

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
        room_id: &RoomIdStr,
        body: &str,
        _structured: Option<serde_json::Value>,
    ) -> Result<(), MatrixError> {
        let sdk = self.sdk().await?;
        let room_id_owned = OwnedRoomId::try_from(room_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid room ID: {}", e)))?;

        // Get or join the room
        let room = match sdk.get_room(&room_id_owned) {
            Some(r) => r,
            None => {
                // Join the room if we're not already in it
                sdk.send(
                    matrix_sdk::ruma::api::client::membership::join_room_by_id::v3::Request::new(
                        room_id_owned.clone(),
                    ),
                )
                .await
                .map_err(|e| MatrixError::Room(format!("Failed to join room: {}", e)))?;
                sdk.get_room(&room_id_owned)
                    .ok_or_else(|| MatrixError::Room("Room not found after join".to_string()))?
            }
        };

        let content = RoomMessageEventContent::text_plain(body);
        room.send(content)
            .await
            .map_err(|e| MatrixError::Room(format!("Failed to send message: {}", e)))?;

        tracing::info!(
            target: "cns.communication.message.sent",
            room_id = %room_id.as_str(),
            "Message sent"
        );
        Ok(())
    }

    /// List active rooms.
    pub async fn list_rooms(&self) -> Result<Vec<Thread>, MatrixError> {
        let sdk = self.sdk().await?;
        let rooms = sdk.rooms();
        let mut threads = Vec::new();

        for room in rooms {
            let room_id = room.room_id().to_string();
            let name = room.name().unwrap_or_else(|| room_id.clone());
            let _topic = room.topic();

            // Collect joined members
            let members = room
                .members(matrix_sdk::RoomMemberships::JOIN)
                .await
                .map_err(|e| MatrixError::Room(format!("Failed to list members: {}", e)))?;
            let participants: Vec<UserIdStr> = members
                .iter()
                .map(|m| UserIdStr::new(&m.user_id().to_string()))
                .collect();

            threads.push(Thread {
                room_id: RoomIdStr::new(&room_id),
                title: name,
                participants,
                monitored_by: vec![],
                escalated: false,
                created_at: chrono::Utc::now().timestamp(),
            });
        }
        Ok(threads)
    }

    /// Get messages from a room (with optional pagination).
    pub async fn get_messages(
        &self,
        room_id: &RoomIdStr,
        limit: usize,
    ) -> Result<Vec<MatrixMessage>, MatrixError> {
        let sdk = self.sdk().await?;
        let room_id_owned = OwnedRoomId::try_from(room_id.as_str())
            .map_err(|e| MatrixError::Room(format!("Invalid room ID: {}", e)))?;

        let room = sdk
            .get_room(&room_id_owned)
            .ok_or_else(|| MatrixError::Room(format!("Room not found: {}", room_id.as_str())))?;

        // Get recent messages from the timeline
        // matrix-sdk 0.16: room.messages() returns a Messages struct with a `chunk` Vec.
        let result = room
            .messages(matrix_sdk::room::MessagesOptions::backward())
            .await
            .map_err(|e| MatrixError::Room(format!("Failed to get messages: {}", e)))?;

        let messages: Vec<MatrixMessage> = result
            .chunk
            .into_iter()
            .take(limit)
            .filter_map(|ev| {
                // TimelineEvent wraps a raw JSON event. Parse and extract fields.
                let raw_str = ev.raw().json().get();
                let parsed: serde_json::Value = serde_json::from_str(raw_str).ok()?;
                let sender = parsed
                    .get("sender")
                    .and_then(|s| s.as_str())
                    .map(|s| UserIdStr::new(s))
                    .unwrap_or_else(|| UserIdStr::new("unknown"));
                let body = parsed
                    .get("content")
                    .and_then(|c| c.get("body"))
                    .and_then(|b| b.as_str())
                    .unwrap_or("")
                    .to_string();
                let timestamp = parsed
                    .get("origin_server_ts")
                    .and_then(|t| t.as_i64())
                    .unwrap_or(0);
                Some(MatrixMessage {
                    sender,
                    body,
                    structured: None,
                    timestamp,
                })
            })
            .collect();

        Ok(messages)
    }

    /// Check for unread/flagged messages (for 7R7 moderation polling).
    pub async fn poll_unread(
        &self,
        rooms: &[RoomIdStr],
    ) -> Result<Vec<MatrixMessage>, MatrixError> {
        let mut all_messages = Vec::new();
        for room_id in rooms {
            match self.get_messages(room_id, 20).await {
                Ok(msgs) => all_messages.extend(msgs),
                Err(_) => continue,
            }
        }
        Ok(all_messages)
    }
}

// ── Conduit sidecar ────────────────────────────────────────────────────────

/// Manages the connection to a Conduit Docker sidecar.
///
/// Conduit runs as a separate Docker container on the same machine.
/// This struct holds the URL and provides a `connect()` method that
/// builds the `MatrixClient` and verifies connectivity.
///
/// Setup: see `scripts/conduit-docker.sh` for the Docker Compose
/// configuration that starts Conduit on localhost:8008.
pub struct ConduitSidecar {
    matrix: MatrixClient,
}

impl ConduitSidecar {
    /// Connect to a Conduit instance at the given URL.
    ///
    /// Builds the SDK client, verifies server reachability, and returns
    /// a ready-to-use `ConduitSidecar`.
    pub async fn connect(homeserver_url: &str) -> Result<Self, MatrixError> {
        let matrix = MatrixClient::new(homeserver_url);
        matrix.connect().await?;

        tracing::info!(
            target: "cns.communication.server.started",
            url = %homeserver_url,
            "Connected to Conduit sidecar"
        );
        Ok(Self { matrix })
    }

    /// Return the underlying client for tool operations.
    pub fn client(&self) -> MatrixClient {
        self.matrix.clone()
    }
}
