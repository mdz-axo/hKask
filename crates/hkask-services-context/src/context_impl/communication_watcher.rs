//! Communication watcher — polls NuEventStore for communication CNS events
//! and forwards them to the curation inbox as `CurationInput::Communication`.
//!
//! This bridges the gap between the 7R7 listener (which persists NuEvents) and
//! the CurationLoop (which reads from the inbox). Without this watcher, Matrix
//! messages would be stored but never routed to the Curator for action.
//!
//! Pattern mirrors `DefaultSpecCurator`: periodic poll → typed event → inbox.
//!
//! Architecture:
//!   7R7 → NuEventStore → CommunicationWatcher → CurationInput::Communication
//!                                         → CurationLoop.sense()
//!                                         → CuratorAgent (LLM decides response)

use hkask_cns::types::loops::CurationInput;
use hkask_cns::types::loops::channels::CommunicationEvent;
use hkask_storage::nu_event_store::NuEventStore;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, MissedTickBehavior};

/// Polling interval in seconds — how often to check for new Matrix activity.
const POLL_INTERVAL_SECS: u64 = 30;

/// Spawn a background task that watches for communication CNS events
/// and routes them to the curation inbox.
///
/// Non-blocking: if the inbox channel is full or closed, events are logged
/// and dropped. The system continues without Matrix awareness — graceful
/// degradation per P3 (Generative Space).
///
/// expect: "Agents communicate through user-owned channels"
/// \[P1\] Motivating: User Sovereignty — Matrix activity is user-owned
/// pre:  `store` is a live NuEventStore with the nu_events schema
/// pre:  `inbox_tx` is a valid UnboundedSender (receiver is the CurationLoop)
/// post: background task spawned that polls every POLL_INTERVAL_SECS
pub fn spawn_communication_watcher(
    store: Arc<NuEventStore>,
    inbox_tx: mpsc::UnboundedSender<CurationInput>,
) {
    tokio::spawn(async move {
        // Track the last-seen event timestamp to avoid re-processing.
        // Start from "now" so we only see events produced after startup.
        let mut last_seen = chrono::Utc::now();

        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let since = last_seen;
            match store.query_algedonic(since, 100) {
                Ok(events) => {
                    // Filter to only communication events (by span_category prefix).
                    let comm_events: Vec<_> = events
                        .iter()
                        .filter(|e| {
                            let cat = e.span.namespace.short_name();
                            cat.starts_with("communication.")
                        })
                        .collect();

                    for event in &comm_events {
                        let ce = CommunicationEvent {
                            span_category: event.span.namespace.short_name().to_string(),
                            span_path: event.span.as_str().to_string(),
                            observation: event.observation.clone(),
                            observed_at: event.timestamp.to_rfc3339(),
                        };

                        match inbox_tx.send(CurationInput::Communication(ce)) {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::warn!(
                                    target: "cns.communication.watcher",
                                    error = %e,
                                    "Curation inbox closed — communication watcher will retry"
                                );
                            }
                        }
                    }

                    // Advance cursor past the last seen event
                    if let Some(latest) = comm_events.last() {
                        last_seen = latest.timestamp;
                    } else {
                        // No communication events found — still advance to now
                        // so we don't repeat the same empty window.
                        last_seen = chrono::Utc::now();
                    }

                    if !comm_events.is_empty() {
                        tracing::info!(
                            target: "cns.communication.watcher",
                            count = comm_events.len(),
                            "Forwarded communication events to curation inbox"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.communication.watcher",
                        error = %e,
                        "Failed to query NuEventStore for communication events"
                    );
                }
            }
        }
    });

    tracing::info!(
        target: "cns.communication.watcher",
        interval_secs = POLL_INTERVAL_SECS,
        "Communication watcher spawned — Matrix activity will be forwarded to curation"
    );
}
