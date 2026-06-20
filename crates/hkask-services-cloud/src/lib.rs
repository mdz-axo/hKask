pub mod fly;
pub mod hetzner;
pub mod tigris;

/// Configuration for deploying a pod to cloud infrastructure.
pub struct DeployConfig {
    pub fly_token: String,
    pub org_slug: String,
    pub container_registry: String,
    pub version: String,
    pub matrix_url: String,
    pub litestream_bucket: String,
    pub litestream_endpoint: String,
    pub litestream_region: String,
    pub litestream_access_key: String,
    pub litestream_secret_key: String,
    pub litestream_force_path: String,
    pub keystore_passphrase: String,
    pub base_url: String,
}

impl DeployConfig {
    /// Build from environment variables with sensible defaults.
    pub fn from_env(pod_id: &str) -> Self {
        let app_name = format!("hkask-pod-{pod_id}");
        Self {
            fly_token: std::env::var("FLY_API_TOKEN").unwrap_or_default(),
            org_slug: std::env::var("FLY_ORG_SLUG").unwrap_or_else(|_| "personal".to_string()),
            container_registry: std::env::var("CONTAINER_REGISTRY")
                .unwrap_or_else(|_| "ghcr.io/mdz-axo/hkask".to_string()),
            version: std::env::var("HKASK_VERSION").unwrap_or_else(|_| "0.30.0".to_string()),
            matrix_url: std::env::var("HKASK_MATRIX_URL")
                .unwrap_or_else(|_| "http://hkask-conduit.internal:8008".to_string()),
            litestream_bucket: std::env::var("LITESTREAM_BUCKET").unwrap_or_default(),
            litestream_endpoint: std::env::var("LITESTREAM_ENDPOINT").unwrap_or_default(),
            litestream_region: std::env::var("LITESTREAM_REGION")
                .unwrap_or_else(|_| "auto".to_string()),
            litestream_access_key: std::env::var("LITESTREAM_ACCESS_KEY_ID").unwrap_or_default(),
            litestream_secret_key: std::env::var("LITESTREAM_SECRET_ACCESS_KEY")
                .unwrap_or_default(),
            litestream_force_path: std::env::var("LITESTREAM_FORCE_PATH_STYLE")
                .unwrap_or_else(|_| "false".to_string()),
            keystore_passphrase: std::env::var("HKASK_KEYSTORE_PASSPHRASE").unwrap_or_default(),
            base_url: std::env::var("HKASK_BASE_URL")
                .unwrap_or_else(|_| format!("https://{app_name}.fly.dev")),
        }
    }
}
