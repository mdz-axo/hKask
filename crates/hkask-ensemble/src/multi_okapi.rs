//! Multi-Okapi Failover — Simplified
//!
//! Minimal stub for multi-Okapi support. Full implementation deferred to v1.1.
//! Configuration-driven via multi-okapi.yaml manifest.
//! ℏKask v0.21.2

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multi-Okapi configuration (loaded from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiOkapiConfig {
    pub instances: Vec<OkapiInstanceConfig>,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub failover: FailoverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiInstanceConfig {
    pub id: String,
    pub endpoint: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub strategy: String,
    #[serde(default)]
    pub load_balance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthCheckConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
}

fn default_interval() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailoverConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_retries: u32,
}

/// Health status (simplified)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Okapi instance (simplified)
#[derive(Debug, Clone)]
pub struct OkapiInstance {
    pub id: String,
    pub endpoint: String,
    pub health: HealthStatus,
}

impl OkapiInstance {
    pub fn new(id: String, endpoint: String) -> Self {
        Self {
            id,
            endpoint,
            health: HealthStatus::Unknown,
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy)
    }
}

/// Multi-Okapi manager (stub)
pub struct MultiOkapiManager {
    config: MultiOkapiConfig,
    instances: HashMap<String, OkapiInstance>,
}

impl MultiOkapiManager {
    pub fn new(config: MultiOkapiConfig) -> Self {
        let instances = config
            .instances
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    OkapiInstance::new(c.id.clone(), c.endpoint.clone()),
                )
            })
            .collect();
        Self { config, instances }
    }

    pub fn get_healthy_instance(&self) -> Option<&OkapiInstance> {
        self.instances.values().find(|i| i.is_available())
    }

    pub fn update_health(&mut self, id: &str, health: HealthStatus) {
        if let Some(instance) = self.instances.get_mut(id) {
            instance.health = health;
        }
    }
}

/// Load config from YAML
pub fn load_multi_okapi_config(yaml_path: &str) -> Result<MultiOkapiConfig, &'static str> {
    let content =
        std::fs::read_to_string(yaml_path).map_err(|_| "Failed to read Multi-Okapi config")?;

    serde_yaml::from_str(&content).map_err(|_| "Failed to parse Multi-Okapi config")
}
