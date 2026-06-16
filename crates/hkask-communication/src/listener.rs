//! 7R7 Communication Listener — polls Matrix rooms and emits CNS observation spans.
//!
//! The 7R7 bot is a passive listener. It polls Matrix rooms on a configurable
//! interval, receives messages, and emits CNS spans for observability. It does
//! NOT classify, escalate, moderate, or judge content. Those decisions belong
//! to the agent layer (Curator + skills + templates + LLM calls).
//!
//! Architecture:
//!   Matrix rooms → 7R7 poll → CNS span emission → agent layer (Curator)
//!
//! The communication server is a dumb pipe. CNS observes. Agents decide.

use crate::matrix::MatrixTransport;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

// ── 7R7 Listener ───────────────────────────────────────────────────────────

/// 7R7 communication listener — polls Matrix for messages, emits CNS spans.
///
/// This is a passive observer. It does not classify, escalate, or moderate.
/// Content decisions are made by the agent layer (Curator + skills + templates).
pub struct SevenR7Listener {
    /// Matrix transport for polling (Mutex-wrapped for shared &mut access).
    matrix: Arc<Mutex<MatrixTransport>>,
    /// Polling interval in seconds.
    poll_interval_secs: u64,
    /// Whether the listener is active.
    active: RwLock<bool>,
}

impl SevenR7Listener {
    /// Create a new 7R7 listener.
    ///
    /// REQ: COMM-019
    /// pre:  matrix is a valid MatrixTransport (authenticated)
    /// pre:  poll_interval_secs > 0
    /// post: returns SevenR7Listener with active=false
    pub fn new(matrix: Arc<Mutex<MatrixTransport>>, poll_interval_secs: u64) -> Self {
        Self {
            matrix,
            poll_interval_secs,
            active: RwLock::new(false),
        }
    }

    /// Start the polling loop.
    ///
    /// Spawns a background task that polls Matrix rooms on the configured
    /// interval and emits CNS observation spans for each message received.
    /// The agent layer (Curator) subscribes to these spans and decides what
    /// action to take.
    ///
    /// REQ: COMM-020
    /// pre:  matrix transport is authenticated
    /// post: background polling task spawned
    /// post: idempotent — calling start() on already-active listener is no-op
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let matrix = Arc::clone(&self.matrix);
        let interval = self.poll_interval_secs;

        tokio::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                timer.tick().await;

                // List known rooms
                let rooms = {
                    let transport = matrix.lock().await;
                    match transport.list_rooms().await {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.communication.listener",
                                error = %e,
                                "7R7 failed to list rooms"
                            );
                            continue;
                        }
                    }
                };

                // Poll each room for recent messages
                for room in &rooms {
                    let room_id = room.room_id.as_str();
                    let transport = matrix.lock().await;
                    match transport.get_messages(&room.room_id, 10).await {
                        Ok(messages) => {
                            for msg in &messages {
                                tracing::info!(
                                    target: "cns.communication.message.observed",
                                    room_id = %room_id,
                                    sender = %msg.sender.as_str(),
                                    body_len = %msg.body.len(),
                                    "7R7 observed message"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                target: "cns.communication.listener",
                                room_id = %room_id,
                                error = %e,
                                "7R7 failed to poll room"
                            );
                        }
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.communication.listener.started",
            interval_secs = %interval,
            "7R7 listener started"
        );
    }

    /// Stop the polling loop.
    ///
    /// REQ: COMM-021
    /// post: active flag set to false
    /// post: idempotent — calling stop() on already-stopped listener is no-op
    pub async fn stop(&self) {
        *self.active.write().await = false;
        tracing::info!(target: "cns.communication.listener.stopped", "7R7 listener stopped");
    }
}
