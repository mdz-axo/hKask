//! Async credential resolution with rotation support.

use hkask_mcp::server::resolve_credential;

use crate::types::WebError;

/// Async trait for resolving credentials with rotation support.
///
/// The production implementation wraps `resolve_credential()`. A future
/// implementation can call `hkask-keystore` for key rotation without
/// restarting the server.
#[async_trait::async_trait]
pub trait CredentialResolver: Send + Sync {
    async fn get_credential(&self, name: &str) -> Result<String, WebError>;
}

/// Production credential resolver that reads from environment / .env files.
///
/// Note: Initial credentials at server construction time come from `ctx.credentials`
/// (resolved by `run_stdio_server_with_preloaded` from keystore, env vars, and .env files).
/// This resolver is used by `ProviderPool` for runtime credential refresh only.
pub struct EnvCredentialResolver;

#[async_trait::async_trait]
impl CredentialResolver for EnvCredentialResolver {
    async fn get_credential(&self, name: &str) -> Result<String, WebError> {
        resolve_credential(name).map_err(|e| {
            WebError::ProviderUnavailable(format!("Credential '{}' unavailable: {}", name, e))
        })
    }
}
