//! Multi-Okapi Failover System
//!
//! Provides automatic failover between multiple Okapi instances.
//! Configurable via OKAPI_FAILOVER_ENDPOINTS environment variable.

use crate::inference_port::{InferenceError, InferencePort, InferenceResult};
use hkask_types::LLMParameters;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Okapi endpoint with health tracking
#[derive(Clone)]
pub struct OkapiEndpoint {
    pub url: String,
    pub healthy: Arc<RwLock<bool>>,
    pub consecutive_failures: Arc<RwLock<u32>>,
}

impl OkapiEndpoint {
    pub fn new(url: String) -> Self {
        Self {
            url,
            healthy: Arc::new(RwLock::new(true)),
            consecutive_failures: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn mark_healthy(&self) {
        *self.healthy.write().await = true;
        *self.consecutive_failures.write().await = 0;
    }

    pub async fn mark_unhealthy(&self) {
        *self.healthy.write().await = false;
        let mut failures = self.consecutive_failures.write().await;
        *failures += 1;
    }

    pub async fn is_healthy(&self) -> bool {
        *self.healthy.read().await
    }
}

/// Multi-Okapi client with failover
pub struct MultiOkapiClient {
    endpoints: Vec<OkapiEndpoint>,
    current_index: Arc<RwLock<usize>>,
}

impl MultiOkapiClient {
    pub fn new(endpoints: Vec<String>) -> Self {
        let endpoints: Vec<OkapiEndpoint> = endpoints.into_iter().map(OkapiEndpoint::new).collect();

        Self {
            endpoints,
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    pub fn from_env() -> Self {
        let endpoints = std::env::var("OKAPI_FAILOVER_ENDPOINTS")
            .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Self::new(endpoints)
    }

    pub async fn get_healthy_endpoint(&self) -> Option<OkapiEndpoint> {
        let mut index = *self.current_index.read().await;
        let start_index = index;

        loop {
            if let Some(endpoint) = self.endpoints.get(index)
                && endpoint.is_healthy().await
            {
                return Some(endpoint.clone());
            }

            index = (index + 1) % self.endpoints.len();
            if index == start_index {
                // All endpoints unhealthy, return first anyway
                return self.endpoints.first().cloned();
            }
        }
    }

    pub async fn failover(&self) {
        let mut index = self.current_index.write().await;
        *index = (*index + 1) % self.endpoints.len();
        info!(target: "hkask.multi_okapi", "Failover to endpoint {}", *index);
    }

    pub async fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub async fn healthy_count(&self) -> usize {
        let mut count = 0;
        for endpoint in &self.endpoints {
            if endpoint.is_healthy().await {
                count += 1;
            }
        }
        count
    }
}

/// Multi-Okapi inference with failover
pub struct MultiOkapiInference {
    clients: Vec<Arc<dyn InferencePort + Send + Sync>>,
    multi_client: MultiOkapiClient,
}

impl MultiOkapiInference {
    pub fn new(
        clients: Vec<Arc<dyn InferencePort + Send + Sync>>,
        multi_client: MultiOkapiClient,
    ) -> Self {
        Self {
            clients,
            multi_client,
        }
    }

    pub async fn generate_with_failover(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        let mut last_error = None;

        for (i, client) in self.clients.iter().enumerate() {
            match client.generate(prompt, parameters).await {
                Ok(result) => {
                    // Mark endpoint healthy on success
                    if let Some(endpoint) = self.multi_client.endpoints.get(i) {
                        endpoint.mark_healthy().await;
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Mark endpoint unhealthy on failure
                    if let Some(endpoint) = self.multi_client.endpoints.get(i) {
                        endpoint.mark_unhealthy().await;
                    }
                    warn!(target: "hkask.multi_okapi", "Endpoint {} failed: {:?}", i, last_error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            InferenceError::Connection("All Okapi endpoints failed".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_okapi_client() {
        let client = MultiOkapiClient::new(vec![
            "http://127.0.0.1:11435".to_string(),
            "http://127.0.0.1:11436".to_string(),
            "http://127.0.0.1:11437".to_string(),
        ]);

        assert_eq!(client.endpoints.len(), 3);
    }
}
