//! Multi-Okapi Failover System (v1.1+ Future Work)
//!
//! This module provides multi-Okapi instance support with:
//! - Capability-based routing
//! - Health checking
//! - Automatic failover
//! - Load balancing
//!
//! **Status:** STUB - For v1.1+ implementation

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::adapters::OkapiHttpClient;
use crate::ports::OkapiCapabilities;

/// Okapi instance with health and capability tracking
#[derive(Clone)]
pub struct OkapiInstance {
    /// Instance endpoint URL
    pub endpoint: String,
    /// Current capabilities
    pub capabilities: OkapiCapabilities,
    /// Health status
    pub health: HealthStatus,
    /// Current load (0.0 - 1.0)
    pub load: f64,
    /// Last health check timestamp
    pub last_health_check: chrono::DateTime<chrono::Utc>,
}

impl OkapiInstance {
    /// Create new Okapi instance
    pub fn new(endpoint: String, capabilities: OkapiCapabilities) -> Self {
        Self {
            endpoint,
            capabilities,
            health: HealthStatus::Unknown,
            load: 0.0,
            last_health_check: chrono::Utc::now(),
        }
    }

    /// Check if instance is healthy and available
    pub fn is_available(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy { .. }) && self.load < 1.0
    }

    /// Update health status
    pub fn update_health(&mut self, health: HealthStatus) {
        self.health = health;
        self.last_health_check = chrono::Utc::now();
    }

    /// Update load
    pub fn update_load(&mut self, load: f64) {
        self.load = load.clamp(0.0, 1.0);
    }
}

/// Health status for Okapi instances
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Instance is healthy
    Healthy {
        /// Response time in milliseconds
        response_time_ms: u64,
        /// Consecutive successful health checks
        consecutive_successes: u32,
    },
    /// Instance is degraded (slow but responding)
    Degraded {
        /// Response time in milliseconds
        response_time_ms: u64,
        /// Reason for degraded status
        reason: String,
    },
    /// Instance is unhealthy
    Unhealthy {
        /// Last error message
        last_error: String,
        /// Consecutive failed health checks
        consecutive_failures: u32,
    },
    /// Health status unknown (not yet checked)
    Unknown,
}

impl HealthStatus {
    /// Check if status is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy { .. })
    }

    /// Check if status is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthStatus::Unhealthy { .. })
    }
}

/// Health checker for Okapi instances
#[derive(Clone)]
pub struct HealthChecker {
    check_interval: Duration,
    timeout: Duration,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(
        check_interval: Duration,
        timeout: Duration,
        _healthy_threshold: u32,
        _unhealthy_threshold: u32,
    ) -> Self {
        Self {
            check_interval,
            timeout,
        }
    }

    /// Check health of an Okapi instance
    pub async fn check_health(&self, endpoint: &str) -> Result<HealthStatus, String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| e.to_string())?;

        let start = std::time::Instant::now();

        match client
            .get(format!("{}/api/engine/status", endpoint))
            .send()
            .await
        {
            Ok(response) => {
                let response_time_ms = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    Ok(HealthStatus::Healthy {
                        response_time_ms,
                        consecutive_successes: 1,
                    })
                } else {
                    Ok(HealthStatus::Degraded {
                        response_time_ms,
                        reason: format!("HTTP {}", response.status()),
                    })
                }
            }
            Err(e) => Ok(HealthStatus::Unhealthy {
                last_error: e.to_string(),
                consecutive_failures: 1,
            }),
        }
    }
}

/// Capability-based router for multi-Okapi setup
pub struct CapabilityRouter {
    instances: Arc<RwLock<Vec<OkapiInstance>>>,
    health_checker: HealthChecker,
}

impl CapabilityRouter {
    /// Create new capability router
    pub fn new(instances: Vec<OkapiInstance>, health_checker: HealthChecker) -> Self {
        Self {
            instances: Arc::new(RwLock::new(instances)),
            health_checker,
        }
    }

    /// Add new Okapi instance
    pub async fn add_instance(&self, instance: OkapiInstance) {
        let mut instances = self.instances.write().await;
        instances.push(instance);
        info!("Added Okapi instance to router");
    }

    /// Remove instance by endpoint
    pub async fn remove_instance(&self, endpoint: &str) {
        let mut instances = self.instances.write().await;
        instances.retain(|inst| inst.endpoint != endpoint);
        info!("Removed Okapi instance from router: {}", endpoint);
    }

    /// Select best instance for required capabilities
    pub async fn select_instance(
        &self,
        required_capabilities: &OkapiCapabilities,
    ) -> Option<OkapiInstance> {
        let instances = self.instances.read().await;

        // Filter instances that have required capabilities and are healthy
        let mut candidates: Vec<&OkapiInstance> = instances
            .iter()
            .filter(|inst| {
                inst.is_available()
                    && self.has_required_capabilities(&inst.capabilities, required_capabilities)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Sort by load (prefer less loaded instances)
        candidates.sort_by(|a, b| {
            a.load
                .partial_cmp(&b.load)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return best candidate
        candidates.first().cloned().cloned()
    }

    /// Check if instance has required capabilities
    fn has_required_capabilities(
        &self,
        available: &OkapiCapabilities,
        required: &OkapiCapabilities,
    ) -> bool {
        // For now, just check runner type compatibility
        // In v1.1+, implement full capability matching
        available.runner_type == required.runner_type
            || (available.token_probs && required.token_probs)
            || (available.grammar_native && required.grammar_native)
    }

    /// Get all instances
    pub async fn get_instances(&self) -> Vec<OkapiInstance> {
        self.instances.read().await.clone()
    }

    /// Get instance count
    pub async fn instance_count(&self) -> usize {
        self.instances.read().await.len()
    }

    /// Start background health checking
    pub async fn start_health_checks(self: Arc<Self>) {
        let check_interval = self.health_checker.check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                let instances = self.instances.read().await.clone();
                for mut instance in instances {
                    match self.health_checker.check_health(&instance.endpoint).await {
                        Ok(health) => {
                            instance.update_health(health.clone());
                            if health.is_healthy() {
                                info!(
                                    "Okapi instance {} is healthy ({}ms)",
                                    instance.endpoint,
                                    if let HealthStatus::Healthy {
                                        response_time_ms, ..
                                    } = health
                                    {
                                        response_time_ms
                                    } else {
                                        0
                                    }
                                );
                            } else {
                                warn!("Okapi instance {} is unhealthy", instance.endpoint);
                            }
                        }
                        Err(e) => {
                            error!("Health check failed for {}: {}", instance.endpoint, e);
                            instance.update_health(HealthStatus::Unhealthy {
                                last_error: e,
                                consecutive_failures: 1,
                            });
                        }
                    }

                    // Update instance in the list
                    let mut instances = self.instances.write().await;
                    if let Some(existing) = instances
                        .iter_mut()
                        .find(|i| i.endpoint == instance.endpoint)
                    {
                        *existing = instance;
                    }
                }
            }
        });
    }
}

/// Multi-Okapi failover client
pub struct MultiOkapiClient {
    router: Arc<CapabilityRouter>,
    default_capabilities: OkapiCapabilities,
}

impl MultiOkapiClient {
    /// Create new multi-Okapi client
    pub fn new(router: Arc<CapabilityRouter>, default_capabilities: OkapiCapabilities) -> Self {
        Self {
            router,
            default_capabilities,
        }
    }

    /// Create HTTP client for selected instance
    pub async fn get_client(&self) -> Option<OkapiHttpClient> {
        if let Some(instance) = self
            .router
            .select_instance(&self.default_capabilities)
            .await
        {
            Some(OkapiHttpClient::new(&instance.endpoint))
        } else {
            None
        }
    }

    /// Get router reference
    pub fn router(&self) -> Arc<CapabilityRouter> {
        Arc::clone(&self.router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okapi_instance_creation() {
        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        let instance = OkapiInstance::new("http://localhost:11435".to_string(), capabilities);

        assert_eq!(instance.endpoint, "http://localhost:11435");
        assert_eq!(instance.health, HealthStatus::Unknown);
        assert_eq!(instance.load, 0.0);
    }

    #[test]
    fn test_health_status_transitions() {
        let mut health = HealthStatus::Unknown;
        assert!(!health.is_healthy());
        assert!(!health.is_unhealthy());

        health = HealthStatus::Healthy {
            response_time_ms: 50,
            consecutive_successes: 1,
        };
        assert!(health.is_healthy());

        health = HealthStatus::Unhealthy {
            last_error: "Connection refused".to_string(),
            consecutive_failures: 3,
        };
        assert!(health.is_unhealthy());
    }

    #[test]
    fn test_instance_availability() {
        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        let mut instance = OkapiInstance::new("http://localhost:11435".to_string(), capabilities);
        assert!(!instance.is_available()); // Unknown health

        instance.update_health(HealthStatus::Healthy {
            response_time_ms: 50,
            consecutive_successes: 1,
        });
        assert!(instance.is_available());

        instance.update_load(1.5); // Overloaded
        assert!(!instance.is_available());
    }

    #[tokio::test]
    async fn test_capability_router() {
        let health_checker =
            HealthChecker::new(Duration::from_secs(30), Duration::from_secs(5), 3, 3);

        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        let instance1 =
            OkapiInstance::new("http://localhost:11435".to_string(), capabilities.clone());
        let instance2 = OkapiInstance::new("http://localhost:11436".to_string(), capabilities);

        let router = CapabilityRouter::new(vec![instance1, instance2], health_checker);

        assert_eq!(router.instance_count().await, 2);

        let required = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: false,
            token_probs: true,
            grammar_native: false,
            advanced_sampling: false,
        };

        // No instances available yet (unknown health)
        let selected = router.select_instance(&required).await;
        assert!(selected.is_none());
    }
}
