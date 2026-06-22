//! FederationSync — background CRDT sync loop for cross-server convergence.
//!
//! Full implementation deferred to Phase 1 completion.
//! CRDT types and transport infrastructure are ready.

use std::sync::Arc;
use std::time::Duration;

use hkask_ports::federation::{FederationSyncPort, FederationTransport};
use hkask_types::event::NuEventSink;
use tokio::sync::watch;

use crate::ReplicaId;

pub struct FederationSync {
    local_replica: ReplicaId,
    #[allow(dead_code)]
    transport: Arc<dyn FederationTransport>,
    #[allow(dead_code)]
    sync_port: Arc<dyn FederationSyncPort>,
    #[allow(dead_code)]
    event_sink: Arc<dyn NuEventSink>,
    interval: Duration,
}

#[allow(dead_code)]
struct PeerState {
    consecutive_failures: u64,
}

impl FederationSync {
    pub fn new(
        local_replica: ReplicaId,
        transport: Arc<dyn FederationTransport>,
        sync_port: Arc<dyn FederationSyncPort>,
        event_sink: Arc<dyn NuEventSink>,
    ) -> Self {
        Self {
            local_replica,
            transport,
            sync_port,
            event_sink,
            interval: Duration::from_secs(5),
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub async fn run(&self, mut cancel: watch::Receiver<bool>) {
        tracing::info!(
            target: "cns.federation.sync",
            replica = %self.local_replica,
            "FederationSync started"
        );
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.interval) => {
                    tracing::debug!(target: "cns.federation.sync", "tick");
                }
                _ = cancel.changed() => {
                    tracing::info!(target: "cns.federation.sync", "FederationSync stopped");
                    return;
                }
            }
        }
    }
}
