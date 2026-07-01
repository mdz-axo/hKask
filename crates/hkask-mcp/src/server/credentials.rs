//! Credential resolution — .env loading and keystore-first credential lookup.

use std::collections::HashMap;

/// Parse .env files and return key-value pairs without mutating the process environment.
/// Load .env file from the project root.
///
/// post: returns HashMap of env vars from .env file
/// post: returns empty map if .env not found
#[must_use]
pub fn load_dotenv() -> HashMap<String, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    for path in [cwd.join(".env")].iter().chain(
        cwd.parent()
            .map(|p| vec![p.join(".env")])
            .unwrap_or_default()
            .iter(),
    ) {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut map = HashMap::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let (key, value) = (key.trim(), value.trim());
                    if !key.is_empty() && !value.is_empty() && std::env::var(key).is_err() {
                        map.insert(key.into(), value.into());
                    }
                }
            }
            return map;
        }
    }
    HashMap::new()
}

/// Routes known credential names through the proper hkask keystore resolvers.
///
/// For unrecognized credential names, falls back to keychain lookup by env var name
/// and then environment variable lookup.
///
/// pre:  env_var is non-empty
/// post: returns credential value from the appropriate resolution chain
#[must_use = "result must be used"]
pub fn resolve_credential(env_var: &str) -> Result<String, hkask_keystore::KeystoreError> {
    match env_var {
        "HKASK_DB_PASSPHRASE" => {
            let bytes = hkask_keystore::keychain::resolve_db_passphrase()?;
            Ok(hex::encode(&*bytes))
        }
        "HKASK_OCAP_SECRET" => {
            let bytes = hkask_keystore::keychain::get_or_create_ocap_secret()?;
            Ok(hex::encode(&*bytes))
        }
        "HKASK_A2A_SECRET" => {
            let bytes = hkask_keystore::keychain::resolve_a2a_secret()?;
            Ok(hex::encode(&*bytes))
        }
        "HKASK_MCP_SECRET" => {
            let bytes = hkask_keystore::keychain::resolve_mcp_secret()?;
            Ok(hex::encode(&*bytes))
        }
        "HKASK_MCP_SECURITY_KEY" => {
            // Reserved for the MCP security gateway; resolution is wired here
            // even though the gateway auth path is not yet integrated.
            let bytes = hkask_keystore::keychain::resolve_mcp_security_key()?;
            Ok(hex::encode(&*bytes))
        }
        "HKASK_CAPABILITY_KEY" => {
            let bytes = hkask_keystore::keychain::resolve_capability_key()?;
            Ok(hex::encode(&*bytes))
        }
        _ => {
            // Unrecognized credential — try keychain, then env var.
            let val = hkask_keystore::Keychain::default()
                .retrieve_by_key(env_var)
                .or_else(|_| std::env::var(env_var))
                .map_err(|_| {
                    hkask_keystore::KeystoreError::NotFound(format!(
                        "Credential '{}' not found in keychain or environment",
                        env_var
                    ))
                })?;
            tracing::debug!(
                credential = env_var,
                "Credential resolved via keychain or environment"
            );
            Ok(val)
        }
    }
}
