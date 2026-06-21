//! Hetzner Cloud API client.
//!
//! Provides server and volume management for the Hetzner infrastructure layer.
//! Pod lifecycle (activate/deactivate) on Hetzner uses kubectl against the K3s
//! cluster; this client handles the IaaS layer (server provisioning, volume
//! management, object storage validation).
//!
//! API docs: https://docs.hetzner.cloud/

use reqwest::Client;
use serde::Deserialize;

const HCLOUD_API: &str = "https://api.hetzner.cloud/v1";

/// Client for the Hetzner Cloud REST API.
pub struct HetznerClient {
    client: Client,
    token: String,
}

/// Minimal Hetzner server representation.
#[derive(Debug, Default, Deserialize)]
pub struct HetznerServer {
    pub id: u64,
    pub name: String,
    pub status: String,
}

/// Minimal Hetzner volume representation.
#[derive(Debug, Default, Deserialize)]
pub struct HetznerVolume {
    pub id: u64,
    pub name: String,
    pub size: u32,
    pub location: String,
}

/// Hetzner API wraps everything in a standard envelope.
#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    #[serde(rename = "type")]
    _type: Option<String>,
    #[serde(default)]
    servers: Option<Vec<T>>,
    #[serde(default)]
    #[allow(dead_code)]
    volumes: Option<Vec<T>>,
    #[serde(default)]
    server: Option<T>,
    #[serde(default)]
    volume: Option<T>,
}

impl HetznerClient {
    /// Create a new Hetzner Cloud API client.
    ///
    /// `token` is a Read & Write API token from the Hetzner Console.
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// List all servers in the project.
    pub async fn list_servers(&self) -> Result<Vec<HetznerServer>, String> {
        let resp = self
            .client
            .get(format!("{HCLOUD_API}/servers"))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("Failed to list servers: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List servers failed ({status}): {body}"));
        }

        let envelope: ApiEnvelope<HetznerServer> =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(envelope.servers.unwrap_or_default())
    }

    /// Create a new server instance.
    pub async fn create_server(
        &self,
        name: &str,
        server_type: &str,
        image: &str,
        location: &str,
        ssh_keys: &[String],
    ) -> Result<HetznerServer, String> {
        let resp = self
            .client
            .post(format!("{HCLOUD_API}/servers"))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({
                "name": name,
                "server_type": server_type,
                "image": image,
                "location": location,
                "ssh_keys": ssh_keys,
                "start_after_create": true,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create server: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create server failed ({status}): {body}"));
        }

        let envelope: ApiEnvelope<HetznerServer> =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        envelope
            .server
            .ok_or_else(|| "Server not found in response".to_string())
    }

    /// Create a persistent volume.
    pub async fn create_volume(
        &self,
        name: &str,
        size_gb: u32,
        location: &str,
    ) -> Result<HetznerVolume, String> {
        let resp = self
            .client
            .post(format!("{HCLOUD_API}/volumes"))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({
                "name": name,
                "size": size_gb,
                "location": location,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create volume: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create volume failed ({status}): {body}"));
        }

        let envelope: ApiEnvelope<HetznerVolume> =
            resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        envelope
            .volume
            .ok_or_else(|| "Volume not found in response".to_string())
    }

    /// Validate that the Hetzner API token works.
    pub async fn validate_token(&self) -> Result<(), String> {
        // A simple server list is the lightest validation call
        let _servers = self.list_servers().await?;
        Ok(())
    }
}

/// Validate that Hetzner Object Storage is reachable with the given credentials.
///
/// Uses the S3-compatible API endpoint. This is the same validation pattern
/// as other S3-compatible storage, adapted for Hetzner's path-style addressing.
pub async fn validate_object_storage(
    endpoint: &str,
    bucket: &str,
    access_key: &str,
    _secret_key: &str,
) -> Result<(), String> {
    let client = Client::new();

    // Hetzner OS uses path-style: endpoint/bucket
    let url = format!("{endpoint}/{bucket}");

    let resp = client
        .head(&url)
        .header(
            "Authorization",
            format!(
                "AWS4-HMAC-SHA256 Credential={access_key}/20260620/auto/s3/aws4_request, SignedHeaders=host, Signature=UNSIGNED_VALIDATION_ONLY"
            ),
        )
        .send()
        .await
        .map_err(|e| format!("Cannot reach Hetzner Object Storage at {endpoint}: {e}"))?;

    match resp.status().as_u16() {
        200 | 403 => Ok(()), // 403 = valid creds, Litestream only needs PutObject/GetObject
        404 => Err(format!(
            "Bucket '{bucket}' not found at {endpoint}. Create it in the Hetzner Console first."
        )),
        status => {
            let body = resp.text().await.unwrap_or_default();
            Err(format!(
                "Object storage validation failed (HTTP {status}): {body}"
            ))
        }
    }
}
