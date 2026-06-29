//! FederationSync — background CRDT sync loop for cross-server convergence.
//!
//! Uses RwLock-wrapped ORSet for interior mutability in async context.
//! Queries FederationLinkManager for linked peers each tick.
//! Emits CNS spans for merge events.

use std::sync::Arc;
use std::time::{Duration, Instant};

use hkask_ports::federation::{
    FederationDelta, FederationMessage, FederationSyncPort, FederationTransport,
};
use hkask_types::cns::CnsSpan;
use hkask_types::event::{CyclePhase, NuEvent, NuEventSink, Span, SpanNamespace};
use serde_json::json;
use tokio::sync::{RwLock, watch};

use crate::ReplicaId;
use crate::crdt::{FederationTripleKey, ORSet};
use crate::sync::FederationLinkManager;
use crate::sync::health_model::FederationHealthModel;
use crate::sync::payload_store::TriplePayloadStore;

pub struct FederationSync {
    local_replica: ReplicaId,
    semantic_set: RwLock<ORSet<FederationTripleKey>>,
    payload_store: RwLock<TriplePayloadStore>,
    transport: Arc<dyn FederationTransport>,
    sync_port: Arc<dyn FederationSyncPort>,
    /// Link manager — queried for linked peers each tick.
    link_manager: Arc<FederationLinkManager>,
    interval: Duration,
    event_sink: Arc<dyn NuEventSink>,
    health: RwLock<FederationHealthModel>,
}

impl FederationSync {
    pub fn new(
        local_replica: ReplicaId,
        transport: Arc<dyn FederationTransport>,
        sync_port: Arc<dyn FederationSyncPort>,
        link_manager: Arc<FederationLinkManager>,
        event_sink: Arc<dyn NuEventSink>,
    ) -> Self {
        Self {
            semantic_set: RwLock::new(ORSet::new(local_replica.clone())),
            payload_store: RwLock::new(TriplePayloadStore::new()),
            local_replica,
            transport,
            sync_port,
            link_manager,
            interval: Duration::from_secs(5),
            event_sink,
            health: RwLock::new(FederationHealthModel::new()),
        }
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
                    if let Err(e) = self.tick().await {
                        tracing::warn!(target: "cns.federation.sync", error = %e, "tick failed");
                    }
                }
                _ = cancel.changed() => {
                    tracing::info!(target: "cns.federation.sync", "stopped");
                    return;
                }
            }
        }
    }

    async fn tick(&self) -> Result<(), String> {
        // 1. Pull local public triples since last cursor
        let cursor = self.sync_port.cursor_for(&self.local_replica);
        let local_triples = self
            .sync_port
            .query_public_since(cursor, 1000)
            .map_err(|e| format!("query: {e}"))?;

        let local_added = local_triples.len() as u64;
        if local_added > 0 {
            let mut set = self.semantic_set.write().await;
            let mut store = self.payload_store.write().await;
            for t in &local_triples {
                let key =
                    FederationTripleKey::from_hash(compute_eav(&t.entity, &t.attribute, &t.value));
                set.add(key.clone());
                store.upsert(hkask_storage::Triple::new(
                    &t.entity,
                    &t.attribute,
                    t.value.clone(),
                    hkask_types::WebID::from_persona(b"local"),
                ));
            }
            self.sync_port
                .advance_cursor(&self.local_replica, cursor + local_added);
        }

        // 2. Sync with linked peers from the link manager
        let peers = self.link_manager.linked_peers().await;
        for peer in &peers {
            let vv: std::collections::HashMap<String, u64> = self
                .semantic_set
                .read()
                .await
                .version_vector()
                .iter()
                .map(|(r, c)| (r.clone(), *c))
                .collect();

            let start = Instant::now();
            let msg = FederationMessage::SyncRequest {
                version_vector: vv.clone(),
            };

            match self.transport.send(peer, msg).await {
                Ok(()) => match self.transport.recv().await {
                    Ok((_from, FederationMessage::SyncResponse { deltas, .. })) => {
                        let latency = start.elapsed().as_millis() as u64;
                        self.merge_deltas(&deltas).await;
                        self.emit_cns(
                            CnsSpan::FederationCrdtMerge,
                            json!({"from": peer, "triples_added": deltas.triples_added, "latency_ms": latency}),
                        );
                        {
                            let mut health = self.health.write().await;
                            health.observe_latency(latency);
                            health.observe_merge(deltas.triples_added);
                            health.observe_member_count(peers.len());
                        }
                    }
                    Ok((from, FederationMessage::InvitationRequest { .. })) => {
                        tracing::info!(
                            target: "cns.federation.sync",
                            from_replica = %from,
                            "Received federation invitation"
                        );
                    }
                    Ok((from, FederationMessage::InvitationResponse { accepted, .. })) => {
                        tracing::info!(
                            target: "cns.federation.sync",
                            from_replica = %from,
                            accepted = accepted,
                            "Received invitation response"
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(target: "cns.federation.sync", peer = %peer, error = %e, "sync recv failed");
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "cns.federation.sync", peer = %peer, error = %e, "sync send failed");
                }
            }
        }

        Ok(())
    }

    async fn merge_deltas(&self, deltas: &FederationDelta) {
        let mut set = self.semantic_set.write().await;
        let mut store = self.payload_store.write().await;
        for t in &deltas.triples {
            let key =
                FederationTripleKey::from_hash(compute_eav(&t.entity, &t.attribute, &t.value));
            set.add(key);
            store.upsert(hkask_storage::Triple::new(
                &t.entity,
                &t.attribute,
                t.value.clone(),
                hkask_types::WebID::from_persona(b"fed"),
            ));
        }
    }

    fn emit_cns(&self, span: CnsSpan, metadata: serde_json::Value) {
        let s = Span::new(SpanNamespace::from(span), "federation");
        let event = NuEvent::new(
            hkask_types::WebID::from_persona(b"curator"),
            s,
            CyclePhase::Act,
            metadata,
            0,
        );
        let _ = self.event_sink.persist(&event);
    }
}

fn compute_eav(entity: &str, attribute: &str, value: &serde_json::Value) -> [u8; 32] {
    let owner = hkask_types::WebID::from_persona(b"fed");
    let triple = hkask_storage::Triple::new(entity, attribute, value.clone(), owner);
    hkask_memory::recall_dedup::eav_hash(&triple)
}
