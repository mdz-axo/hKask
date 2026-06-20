//! Fly.io Machines API client.
//!
//! Provides programmatic pod lifecycle management:
//!   create_app → create_volume → set_secrets → create_machine → (stop/start)
//!
//! API docs: https://fly.io/docs/machines/api/

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const FLY_API_HOST: &str = "https://api.machines.dev";

/// Client for the Fly.io Machines REST API.
pub struct FlyClient {
    client: Client,
    token: String,
}

/// Minimal Fly App representation.
#[derive(Debug, Deserialize)]
pub struct FlyApp {
    pub name: String,
    pub organization: Option<FlyOrg>,
}

#[derive(Debug, Deserialize)]
pub struct FlyOrg {
    pub slug: String,
}

/// Minimal Fly Volume representation.
#[derive(Debug, Deserialize)]
pub struct FlyVolume {
    pub id: String,
    pub name: String,
    pub size_gb: u32,
    pub region: String,
}

/// Machine state enum.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MachineState {
    Created,
    Started,
    Stopped,
    Destroyed,
}

/// Minimal Fly Machine representation.
#[derive(Debug, Deserialize)]
pub struct FlyMachine {
    pub id: String,
    pub name: String,
    pub state: MachineState,
    pub region: String,
}

/// Configuration for creating a Fly Machine.
#[derive(Debug, Serialize)]
pub struct MachineConfig {
    pub name: String,
    pub region: String,
    pub config: MachineSpec,
}

#[derive(Debug, Serialize)]
pub struct MachineSpec {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<MachineMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<MachineService>>,
    pub guest: MachineGuest,
}

#[derive(Debug, Serialize)]
pub struct MachineMount {
    pub volume: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct MachineService {
    pub protocol: String,
    pub internal_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<MachinePort>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MachinePort {
    pub port: u16,
    pub handlers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MachineGuest {
    pub cpu_kind: String,
    pub cpus: u32,
    pub memory_mb: u32,
}

impl FlyClient {
    /// Create a new Fly.io API client.
    ///
    /// `token` should be an org deploy token from `fly tokens create org`.
    /// Format: "FlyV1 fm2_..."
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Create a Fly App (namespace for machines, volumes, secrets).
    pub async fn create_app(&self, app_name: &str, org_slug: &str) -> Result<FlyApp, String> {
        let resp = self
            .client
            .post(format!("{FLY_API_HOST}/apps"))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({
                "app_name": app_name,
                "org_slug": org_slug,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create app '{app_name}': {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create app failed ({status}): {body}"));
        }

        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// Create a persistent volume for SQLite storage.
    pub async fn create_volume(
        &self,
        app_name: &str,
        volume_name: &str,
        region: &str,
        size_gb: u32,
    ) -> Result<FlyVolume, String> {
        let resp = self
            .client
            .post(format!("{FLY_API_HOST}/apps/{app_name}/volumes"))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({
                "name": volume_name,
                "region": region,
                "size_gb": size_gb,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create volume: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create volume failed ({status}): {body}"));
        }

        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// Create and start a Fly Machine (Firecracker microVM).
    pub async fn create_machine(
        &self,
        app_name: &str,
        config: &MachineConfig,
    ) -> Result<FlyMachine, String> {
        let resp = self
            .client
            .post(format!("{FLY_API_HOST}/apps/{app_name}/machines"))
            .header("Authorization", self.auth_header())
            .json(config)
            .send()
            .await
            .map_err(|e| format!("Failed to create machine: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create machine failed ({status}): {body}"));
        }

        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    /// Set secrets for an app (injected as env vars into machines).
    pub async fn set_secrets(
        &self,
        app_name: &str,
        secrets: &HashMap<String, String>,
    ) -> Result<(), String> {
        let resp = self
            .client
            .post(format!("{FLY_API_HOST}/apps/{app_name}/secrets"))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({ "values": secrets }))
            .send()
            .await
            .map_err(|e| format!("Failed to set secrets: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Set secrets failed ({status}): {body}"));
        }

        Ok(())
    }

    /// Stop a running Machine (preserves volume).
    pub async fn stop_machine(&self, app_name: &str, machine_id: &str) -> Result<(), String> {
        let resp = self
            .client
            .post(format!(
                "{FLY_API_HOST}/apps/{app_name}/machines/{machine_id}/stop"
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Failed to stop machine: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Stop machine failed ({status}): {body}"));
        }

        Ok(())
    }

    /// Start a stopped Machine.
    pub async fn start_machine(&self, app_name: &str, machine_id: &str) -> Result<(), String> {
        let resp = self
            .client
            .post(format!(
                "{FLY_API_HOST}/apps/{app_name}/machines/{machine_id}/start"
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Failed to start machine: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Start machine failed ({status}): {body}"));
        }

        Ok(())
    }

    /// List machines in an app.
    pub async fn list_machines(&self, app_name: &str) -> Result<Vec<FlyMachine>, String> {
        let resp = self
            .client
            .get(format!("{FLY_API_HOST}/apps/{app_name}/machines"))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Failed to list machines: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List machines failed ({status}): {body}"));
        }

        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }
}
