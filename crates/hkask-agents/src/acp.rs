//! ACP Runtime — Simplified
//!
//! Generic ACP processor reading configuration from acp-runtime.yaml.
//! Rust is the loom. YAML is the thread.
//! ℏKask v0.21.2

use hkask_cns::rate_limit::{RateLimitConfig, RateLimiter};
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

/// ACP configuration (loaded from YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfig {
    pub root_authority: RootAuthorityConfig,
    #[serde(default)]
    pub agent_registration: AgentRegistrationConfig,
    #[serde(default)]
    pub messaging: MessagingConfig,
    #[serde(default)]
    pub rate_limiting: RateLimitingConfig,
    #[serde(default)]
    pub capability_verification: CapabilityVerificationConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub cns: CnsIntegrationConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootAuthorityConfig {
    pub webid: String,
    pub secret: SecretConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretConfig {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub env_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRegistrationConfig {
    #[serde(default)]
    pub requirements: RegistrationRequirements,
    #[serde(default)]
    pub validation: RegistrationValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrationRequirements {
    #[serde(default)]
    pub require_webid: bool,
    #[serde(default)]
    pub require_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrationValidation {
    #[serde(default)]
    pub reject_wildcard_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagingConfig {
    #[serde(default)]
    pub correlation: CorrelationConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub delivery: DeliveryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorrelationConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeliveryConfig {
    #[serde(default)]
    pub guarantee: String,
    #[serde(default)]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub defaults: RateLimitDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitDefaults {
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u64,
    #[serde(default = "default_burst_size")]
    pub burst_size: u64,
}

fn default_messages_per_minute() -> u64 {
    60
}

fn default_burst_size() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityVerificationConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub token_validation: TokenValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenValidationConfig {
    #[serde(default)]
    pub check_signature: bool,
    #[serde(default)]
    pub check_expiry: bool,
    #[serde(default = "default_max_depth")]
    pub max_chain_depth: u8,
}

fn default_max_depth() -> u8 {
    7
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub log: AuditLogging,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogging {
    #[serde(default)]
    pub message_send: bool,
    #[serde(default)]
    pub message_receive: bool,
    #[serde(default)]
    pub capability_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CnsIntegrationConfig {
    #[serde(default)]
    pub spans: SpanEmissionConfig,
    #[serde(default)]
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpanEmissionConfig {
    #[serde(default)]
    pub emit_on_message_send: bool,
    #[serde(default)]
    pub emit_on_capability_denied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub injection_prevention: InjectionPrevention,
    #[serde(default)]
    pub dos_prevention: DosPrevention,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InjectionPrevention {
    #[serde(default)]
    pub sanitize_message_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DosPrevention {
    #[serde(default)]
    pub max_message_size_bytes: usize,
}

/// ACP error types
#[derive(Debug, Error)]
pub enum AcpError {
    #[error("Agent {0:?} already registered")]
    AgentAlreadyRegistered(WebID),
    #[error("Agent {0:?} not found")]
    AgentNotFound(WebID),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Invalid capability: wildcards not allowed")]
    WildcardCapabilityNotAllowed,
    #[error("Secret not configured")]
    SecretNotConfigured,
}

/// A2A message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub from: WebID,
    pub to: WebID,
    pub content: serde_json::Value,
    pub correlation_id: Option<String>,
}

/// ACP Agent
#[derive(Debug, Clone)]
pub struct AcpAgent {
    pub webid: WebID,
    pub capabilities: Vec<String>,
}

/// Template dispatch handler trait
pub trait TemplateDispatchHandler: Send + Sync {
    fn dispatch(&self, message: A2AMessage) -> Result<serde_json::Value, AcpError>;
}

/// ACP Runtime
pub struct AcpRuntime {
    config: AcpConfig,
    emitter: SpanEmitter,
    agents: Arc<RwLock<HashMap<WebID, AgentInfo>>>,
    rate_limiter: RateLimiter,
}

#[derive(Debug, Clone)]
struct AgentInfo {
    capabilities: Vec<String>,
}

impl AcpRuntime {
    pub fn new(config: AcpConfig) -> Self {
        let observer = WebID::new();
        let rate_config = RateLimitConfig {
            max_tokens: config.rate_limiting.defaults.burst_size as u32,
            refill_interval: std::time::Duration::from_millis(
                60000 / config.rate_limiting.defaults.messages_per_minute,
            ),
        };
        Self {
            config,
            emitter: SpanEmitter::new(observer),
            agents: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: RateLimiter::new(rate_config),
        }
    }

    /// Register agent
    pub async fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
    ) -> Result<(), AcpError> {
        if self
            .config
            .agent_registration
            .validation
            .reject_wildcard_capabilities
        {
            for cap in &capabilities {
                if cap.contains("*") {
                    return Err(AcpError::WildcardCapabilityNotAllowed);
                }
            }
        }

        let mut agents = self.agents.write().await;
        if agents.contains_key(&webid) {
            return Err(AcpError::AgentAlreadyRegistered(webid));
        }

        agents.insert(webid, AgentInfo { capabilities });

        info!("Agent {:?} registered", webid);
        Ok(())
    }

    /// Send A2A message
    pub async fn send_message(
        &self,
        from: WebID,
        to: WebID,
        content: serde_json::Value,
    ) -> Result<serde_json::Value, AcpError> {
        // Rate limit check
        if self.config.rate_limiting.enabled && !self.rate_limiter.check(&from) {
            self.emitter.emit_tool(
                "cns.agent.acp.rate_limit",
                serde_json::json!({"from": from.to_string(), "to": to.to_string()}),
            );
            return Err(AcpError::RateLimitExceeded);
        }

        // Capability verification
        if self.config.capability_verification.required {
            self.verify_capability(&from, "tool:execute").await?;
        }

        self.emitter.emit_tool(
            "cns.agent.acp.message.send",
            serde_json::json!({
                "from": from.to_string(),
                "to": to.to_string(),
            }),
        );

        Ok(content)
    }

    async fn verify_capability(&self, webid: &WebID, required: &str) -> Result<(), AcpError> {
        let agents = self.agents.read().await;
        let agent = agents.get(webid).ok_or(AcpError::AgentNotFound(*webid))?;

        if !agent.capabilities.iter().any(|c| c == required) {
            return Err(AcpError::CapabilityDenied(format!(
                "Agent lacks {}",
                required
            )));
        }
        Ok(())
    }
}

/// Load ACP config from YAML
pub fn load_acp_config(yaml_path: &str) -> Result<AcpConfig, AcpError> {
    let content = std::fs::read_to_string(yaml_path).map_err(|_| AcpError::SecretNotConfigured)?;

    serde_yaml::from_str(&content).map_err(|_| AcpError::SecretNotConfigured)
}
