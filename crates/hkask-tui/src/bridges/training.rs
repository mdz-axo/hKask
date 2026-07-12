//! TrainingDataBridge — trait for training/adapter data in the TUI.
//!
//! Provides the Training window with live adapter, deployment, and
//! session data from hkask-mcp-training / hkask-adapter.

use std::sync::Arc;

/// Summary of a LoRA adapter.
#[derive(Debug, Clone)]
pub struct AdapterSummary {
    pub name: String,
    pub base_model: String,
    pub version: String,
    pub size_bytes: u64,
    pub expertise: String,
}

/// Summary of a deployment endpoint.
#[derive(Debug, Clone)]
pub struct DeploymentSummary {
    pub adapter_name: String,
    pub provider: String,
    pub status: String,
}

/// Trait for querying training subsystem state.
pub trait TrainingDataBridge: Send + Sync {
    fn adapter_list(&self) -> Vec<AdapterSummary>;
    fn deployment_list(&self) -> Vec<DeploymentSummary>;
    fn session_count(&self) -> usize;
    fn adapter_count(&self) -> usize;
}

/// Mock implementation for TUI development and testing.
pub struct MockTrainingBridge {
    pub adapters: Vec<AdapterSummary>,
    pub deployments: Vec<DeploymentSummary>,
    pub sessions: usize,
}

impl MockTrainingBridge {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
            deployments: Vec::new(),
            sessions: 0,
        }
    }

    pub fn with_sample_data() -> Self {
        Self {
            adapters: vec![AdapterSummary {
                name: "pragmatic-semantics".into(),
                base_model: "llama-3.1-8b".into(),
                version: "v1".into(),
                size_bytes: 38_000_000,
                expertise: "Epistemic classification".into(),
            }],
            deployments: vec![DeploymentSummary {
                adapter_name: "pragmatic-semantics".into(),
                provider: "together".into(),
                status: "active".into(),
            }],
            sessions: 3,
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl TrainingDataBridge for MockTrainingBridge {
    fn adapter_list(&self) -> Vec<AdapterSummary> {
        self.adapters.clone()
    }
    fn deployment_list(&self) -> Vec<DeploymentSummary> {
        self.deployments.clone()
    }
    fn session_count(&self) -> usize {
        self.sessions
    }
    fn adapter_count(&self) -> usize {
        self.adapters.len()
    }
}
