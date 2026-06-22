//! In-memory transport for unit testing — no Matrix dependency.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use hkask_ports::federation::{FederationMessage, FederationTransport, FederationTransportError};
use tokio::sync::Mutex;

use crate::ReplicaId;

pub struct Inner {
    queues: HashMap<(ReplicaId, ReplicaId), VecDeque<FederationMessage>>,
    partitions: std::collections::HashSet<ReplicaId>,
}

pub struct InMemoryFederationTransport {
    inner: Arc<Mutex<Inner>>,
    local_replica: ReplicaId,
}

impl InMemoryFederationTransport {
    pub fn new() -> Arc<Mutex<Inner>> {
        Arc::new(Mutex::new(Inner {
            queues: HashMap::new(),
            partitions: std::collections::HashSet::new(),
        }))
    }

    pub fn for_replica(shared: &Arc<Mutex<Inner>>, replica: ReplicaId) -> Self {
        Self {
            inner: Arc::clone(shared),
            local_replica: replica,
        }
    }
}

#[async_trait::async_trait]
impl FederationTransport for InMemoryFederationTransport {
    async fn send(
        &self,
        peer: &ReplicaId,
        message: FederationMessage,
    ) -> Result<(), FederationTransportError> {
        let mut inner = self.inner.lock().await;
        if inner.partitions.contains(peer) || inner.partitions.contains(&self.local_replica) {
            return Err(FederationTransportError::PeerPartitioned(peer.clone()));
        }
        inner
            .queues
            .entry((self.local_replica.clone(), peer.clone()))
            .or_default()
            .push_back(message);
        Ok(())
    }

    async fn recv(&self) -> Result<(ReplicaId, FederationMessage), FederationTransportError> {
        for _ in 0..10 {
            let mut inner = self.inner.lock().await;
            for ((from, to), queue) in inner.queues.iter_mut() {
                if *to == self.local_replica {
                    if let Some(msg) = queue.pop_front() {
                        return Ok((from.clone(), msg));
                    }
                }
            }
            drop(inner);
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        Err(FederationTransportError::Transport("no messages".into()))
    }

    fn simulate_partition(&self, peer: &ReplicaId) {
        let inner = self.inner.clone();
        let peer = peer.clone();
        tokio::task::block_in_place(|| {
            let mut guard = inner.blocking_lock();
            guard.partitions.insert(peer);
        });
    }

    fn heal_partition(&self, peer: &ReplicaId) {
        let inner = self.inner.clone();
        let peer = peer.clone();
        tokio::task::block_in_place(|| {
            let mut guard = inner.blocking_lock();
            guard.partitions.remove(&peer);
        });
    }
}
