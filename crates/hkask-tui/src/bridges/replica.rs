//! ReplicaDataBridge — trait for authorial style replicas in the TUI.

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ReplicaInfo {
    pub author: String,
    pub centroid_count: usize,
    pub status: String,
}

pub trait ReplicaDataBridge: Send + Sync {
    fn list_replicas(&self) -> Vec<ReplicaInfo>;
    fn replica_count(&self) -> usize;
}

pub struct MockReplicaBridge {
    pub replicas: Vec<ReplicaInfo>,
}
impl MockReplicaBridge {
    pub fn new() -> Self {
        Self { replicas: vec![] }
    }
    pub fn with_sample() -> Self {
        Self {
            replicas: vec![
                ReplicaInfo {
                    author: "Shakespeare".into(),
                    centroid_count: 12,
                    status: "built".into(),
                },
                ReplicaInfo {
                    author: "Austen".into(),
                    centroid_count: 8,
                    status: "built".into(),
                },
            ],
        }
    }
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}
impl ReplicaDataBridge for MockReplicaBridge {
    fn list_replicas(&self) -> Vec<ReplicaInfo> {
        self.replicas.clone()
    }
    fn replica_count(&self) -> usize {
        self.replicas.len()
    }
}
