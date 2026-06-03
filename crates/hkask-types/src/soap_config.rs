//! SOAP inference configuration — shared between CLI and API layers
//!
//! This module owns the config struct and its environment-variable parsing,
//! while keystore secret resolution is left to the caller. This keeps the
//! config logic reusable without introducing a keystore dependency in the
//! types crate.

/// SOAP inference configuration
#[derive(Clone, Debug)]
pub struct SoapInferenceConfig {
    /// Capability secret for token verification (resolved externally via keystore)
    pub capability_secret: [u8; 32],
    /// Maximum number of events per request
    pub max_events: usize,
    /// Maximum subjective text length
    pub max_subjective_len: usize,
    /// Maximum event message length
    pub max_message_len: usize,
    /// Inference timeout in seconds.
    ///
    /// Simple config value for `tokio::time::timeout` — not Cybernetics logic.
    /// A true CNS energy budget would track cumulative spend and adapt; this is
    /// just a per-request wall-clock limit, which is standard HTTP resilience.
    pub timeout_secs: u64,
    /// Model to use for inference
    pub model: String,
    /// Inference temperature (0.0-1.0)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Path to Jack persona file (loaded at runtime)
    pub jack_persona_path: String,
}

impl SoapInferenceConfig {
    /// Build configuration from environment variables and a pre-resolved
    /// capability secret.
    ///
    /// The caller is responsible for resolving `capability_secret` through
    /// the keystore chain (master-key derivation → env var → OS keychain).
    /// This separation keeps the config parsing shareable across CLI and API
    /// without pulling keystore logic into the types crate.
    pub fn from_env(capability_secret: [u8; 32]) -> Result<Self, String> {
        let mut config = Self {
            capability_secret,
            max_events: 100,
            max_subjective_len: 4096,
            max_message_len: 1024,
            timeout_secs: 30,
            model: "qwen3:8b".to_string(),
            temperature: 0.2,
            max_tokens: 2048,
            jack_persona_path: "hkask-templates/personas/jack-nurse.md".to_string(),
        };

        if let Ok(val) = std::env::var("HKASK_SOAP_MODEL") {
            config.model = val;
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TEMPERATURE") {
            config.temperature = val.parse().unwrap_or(config.temperature);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_MAX_TOKENS") {
            config.max_tokens = val.parse().unwrap_or(config.max_tokens);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_TIMEOUT_SECS") {
            config.timeout_secs = val.parse().unwrap_or(config.timeout_secs);
        }
        if let Ok(val) = std::env::var("HKASK_SOAP_PERSONA_PATH") {
            config.jack_persona_path = val;
        }

        Ok(config)
    }

    /// Load Jack persona from file at runtime
    pub fn load_jack_persona(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(&self.jack_persona_path)
    }
}
