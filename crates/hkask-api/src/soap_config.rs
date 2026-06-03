//! SOAP inference configuration — API layer wrapper
//!
//! Wraps the pure-data `InferenceConfig` from `hkask_types` with API-specific
//! fields (capability secret, persona path) and I/O methods (env parsing,
//! file loading). These concerns don't belong in the types crate.

use hkask_types::InferenceConfig;

/// SOAP inference configuration — API layer
///
/// Extends the serializable `InferenceConfig` with runtime-only fields
/// that require I/O or secret resolution.
pub struct SoapInferenceConfig {
    /// Pure inference parameters (serializable)
    pub inference: InferenceConfig,
    /// Capability secret for token verification (resolved via keystore)
    pub(crate) capability_secret: [u8; 32],
    /// Path to Jack persona file (loaded at runtime)
    pub jack_persona_path: String,
}

impl SoapInferenceConfig {
    /// Build configuration from environment variables and a pre-resolved
    /// capability secret.
    ///
    /// The caller is responsible for resolving `capability_secret` through
    /// the keystore chain (master-key derivation → env var → OS keychain).
    pub fn from_env(capability_secret: [u8; 32]) -> Result<Self, String> {
        let mut inference = InferenceConfig {
            model: "qwen3:8b".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            timeout_secs: 30,
            max_events: 100,
            max_subjective_len: 4096,
            max_message_len: 1024,
        };
        let mut jack_persona_path = "hkask-templates/personas/jack-nurse.md".to_string();

        if let Ok(val) = std::env::var("HKASK_SOAP_MODEL") {
            inference.model = val;
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TEMPERATURE") {
            inference.temperature = val.parse().unwrap_or(inference.temperature);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_MAX_TOKENS") {
            inference.max_tokens = val.parse().unwrap_or(inference.max_tokens);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TIMEOUT_SECS") {
            inference.timeout_secs = val.parse().unwrap_or(inference.timeout_secs);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_PERSONA_PATH") {
            jack_persona_path = val;
        }

        Ok(Self {
            inference,
            capability_secret,
            jack_persona_path,
        })
    }

    /// Load Jack persona from file at runtime
    pub fn load_jack_persona(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(&self.jack_persona_path)
    }
}
