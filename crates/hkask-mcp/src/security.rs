//! MCP security gateway
//!
//! Provides security middleware for MCP tool invocations:
//! - Capability verification (OCAP)
//! - Input validation
//! - Audit logging
//! - URL validation (SSRF protection)

use hkask_templates::TemplateError;
use hkask_types::WebID;
use hkask_types::{CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken};
use serde_json::Value;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Security policy for MCP tool invocation
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Maximum input size in bytes
    pub max_input_size: usize,
    /// Allowed tool prefixes (empty = all allowed)
    pub allowed_tools: HashSet<String>,
    /// Denied tool prefixes
    pub denied_tools: HashSet<String>,
    /// Require capability tokens
    pub require_capabilities: bool,
    /// Enable rate limiting (energy budget enforcement)
    pub enable_rate_limiting: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_input_size: 1024 * 1024, // 1MB default
            allowed_tools: HashSet::new(),
            denied_tools: ["admin:", "system:", "internal:"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            require_capabilities: true,
            enable_rate_limiting: true,
        }
    }
}

/// Security gateway for MCP
pub struct SecurityGateway {
    /// Capability checker
    capability_checker: Arc<CapabilityChecker>,
    /// Security policy
    policy: SecurityPolicy,
    /// Audit log (persistent via AuditLogStore adapter, in-memory fallback)
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
    /// Optional persistent audit store (wired when database is available)
    audit_store: Option<Arc<dyn hkask_types::AuditLogPort + Send + Sync>>,
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub bot_id: WebID,
    pub tool_name: String,
    pub action: AuditAction,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Audit action type
#[derive(Debug, Clone)]
pub enum AuditAction {
    CapabilityCheck,
    RateLimitCheck,
    InputValidation,
    ToolInvocation,
}

impl SecurityGateway {
    /// Create new security gateway
    pub fn new(secret: &[u8], policy: SecurityPolicy) -> Self {
        Self {
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            policy,
            audit_log: Arc::new(RwLock::new(Vec::new())),
            audit_store: None,
        }
    }

    /// Create with default policy
    pub fn with_default_policy(secret: &[u8]) -> Self {
        Self::new(secret, SecurityPolicy::default())
    }

    /// Set the persistent audit store for durable audit logging
    pub fn with_audit_store(
        mut self,
        store: Arc<dyn hkask_types::AuditLogPort + Send + Sync>,
    ) -> Self {
        self.audit_store = Some(store);
        self
    }

    /// Validate input size
    pub fn validate_input_size(&self, input: &Value) -> Result<(), TemplateError> {
        let input_size = serde_json::to_vec(input).map(|v| v.len()).unwrap_or(0);

        if input_size > self.policy.max_input_size {
            return Err(TemplateError::Validation(format!(
                "Input size {} exceeds maximum {}",
                input_size, self.policy.max_input_size
            )));
        }

        Ok(())
    }

    /// Check if tool is allowed by policy
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // Check denied list first
        for denied in &self.policy.denied_tools {
            if tool_name.starts_with(denied) {
                return false;
            }
        }

        // If allowed list is empty, all non-denied tools are allowed
        if self.policy.allowed_tools.is_empty() {
            return true;
        }

        // Check allowed list
        for allowed in &self.policy.allowed_tools {
            if tool_name.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Verify capability token
    pub fn verify_capability(
        &self,
        token: &CapabilityToken,
        bot_id: &WebID,
        tool_name: &str,
    ) -> bool {
        let result = self.capability_checker.check(
            token,
            bot_id,
            hkask_types::CapabilityResource::Tool,
            tool_name,
            hkask_types::CapabilityAction::Execute,
        );

        result
    }

    /// Authorize a capability token for tool invocation.
    ///
    /// Performs full OCAP verification: cryptographic signature, expiry, holder
    /// identity, and resource/action match. Returns the verified token (or an
    /// attenuated copy) on success, or a descriptive `SecurityError` on denial.
    pub fn authorize(
        &self,
        token: &CapabilityToken,
        bot_id: &WebID,
        tool_name: &str,
    ) -> Result<CapabilityToken, SecurityError> {
        // 1. Cryptographic signature verification
        if !self.capability_checker.verify(token) {
            return Err(SecurityError::CapabilityDenied {
                webid: *bot_id,
                tool: tool_name.to_string(),
                reason: "cryptographic signature verification failed".to_string(),
            });
        }

        // 2. Expiry check
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if token.is_expired(current_time) {
            return Err(SecurityError::TokenExpired {
                token_id: token.id.clone(),
            });
        }

        // 3. Holder (delegated_to) must match the requesting bot
        if token.delegated_to != *bot_id {
            return Err(SecurityError::CapabilityDenied {
                webid: *bot_id,
                tool: tool_name.to_string(),
                reason: format!(
                    "token holder mismatch: token delegates to {}, but {} requested",
                    token.delegated_to, bot_id
                ),
            });
        }

        // 4. Resource/action match — must grant Execute on the requested Tool
        let required = format!("{}/{}", CapabilityResource::Tool.as_str(), tool_name);
        if !token.is_valid_for(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
        ) {
            let provided = format!("{}/{}", token.resource.as_str(), token.resource_id);
            return Err(SecurityError::InsufficientCapability { required, provided });
        }

        // All checks passed — return the verified token
        Ok(token.clone())
    }

    /// Check rate limit (energy budget enforcement)
    pub fn check_rate_limit(&self, bot_id: &WebID) -> bool {
        if !self.policy.enable_rate_limiting {
            return true;
        }
        // Energy budget handles rate limiting at the pod level.
        // This is a policy gate that always allows when rate limiting is disabled.
        // When enabled, the actual enforcement happens via energy budget depletion.
        let _ = bot_id;
        true
    }

    /// Record audit entry
    pub async fn audit(&self, entry: AuditEntry) {
        // Persist to durable store if available
        if let Some(ref store) = self.audit_store {
            let canonical = hkask_types::AuditEntry {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: entry.timestamp,
                actor: entry.bot_id,
                action: format!("{:?}", entry.action),
                resource: entry.tool_name.clone(),
                outcome: if entry.success {
                    hkask_types::AuditOutcome::Success
                } else {
                    hkask_types::AuditOutcome::Denied
                },
                context: hkask_types::AuditContext {
                    correlation_id: None,
                    recipient: None,
                    ip_address: None,
                    error_message: entry.error_message.clone(),
                    metadata: serde_json::Value::Null,
                },
            };
            store.log(canonical);
        }

        // Maintain in-memory cache for fast querying
        let mut log = self.audit_log.write().await;
        log.push(entry);

        // Trim old entries (keep last 10,000)
        if log.len() > 10000 {
            let drain_count = log.len() - 10000;
            log.drain(0..drain_count);
        }
    }

    /// Get recent audit entries
    pub async fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.read().await;
        log.iter().rev().take(limit).cloned().collect()
    }

    /// Get audit entries for a bot
    pub async fn get_bot_audit(&self, bot_id: &WebID, limit: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.read().await;
        log.iter()
            .filter(|e| e.bot_id == *bot_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get security policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Update security policy
    pub fn set_policy(&mut self, policy: SecurityPolicy) {
        self.policy = policy;
    }

    /// Get capability checker
    pub fn capability_checker(&self) -> &CapabilityChecker {
        &self.capability_checker
    }

    /// Issue capability token
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }
}

impl Default for SecurityGateway {
    fn default() -> Self {
        let secret = hkask_keystore::resolve(&hkask_types::SecretRef::derived(
            hkask_types::derivation_contexts::MASTER_KEY_ENV,
            hkask_types::derivation_contexts::MCP_SECURITY_KEY,
        ))
        .or_else(|_| {
            hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_MCP_SECURITY_KEY"))
        })
        .or_else(|_| {
            hkask_keystore::resolve(&hkask_types::SecretRef::Keychain(
                "mcp-security-key".to_string(),
            ))
        })
        .expect(
            "MCP security key not available: set HKASK_MASTER_KEY or HKASK_MCP_SECURITY_KEY, \
             or store 'mcp-security-key' in the OS keychain",
        );
        Self::with_default_policy(&secret)
    }
}

/// URL validation error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Non-HTTP(S) scheme not allowed: {0}")]
    DisallowedScheme(String),

    #[error("URL contains embedded credentials (user:pass@host): {0}")]
    EmbeddedCredentials(String),

    #[error("Private IP address not allowed: {0}")]
    PrivateIpNotAllowed(String),

    #[error("Loopback address not allowed: {0}")]
    LoopbackNotAllowed(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("capability denied for {webid} on {tool}: {reason}")]
    CapabilityDenied {
        webid: WebID,
        tool: String,
        reason: String,
    },

    #[error("capability token expired: {token_id}")]
    TokenExpired { token_id: String },

    #[error("insufficient capability: required {required}, provided {provided}")]
    InsufficientCapability { required: String, provided: String },
}

/// URL validation configuration
#[derive(Debug, Clone, Default)]
pub struct UrlValidationConfig {
    /// Allow private IP addresses (10.x, 172.16-31.x, 192.168.x)
    pub allow_private_ips: bool,
    /// Allow loopback addresses (127.x.x.x, ::1)
    pub allow_loopback: bool,
}

/// Validate a URL for use in MCP web/scholar requests.
///
/// Checks:
/// - Rejects non-HTTP(S) schemes
/// - Rejects URLs with embedded credentials (user:pass@host)
/// - Rejects private IPs unless explicitly permitted
/// - Rejects loopback addresses unless explicitly permitted
pub fn validate_url(raw_url: &str, config: &UrlValidationConfig) -> Result<(), SecurityError> {
    let scheme_end = raw_url
        .find("://")
        .ok_or_else(|| SecurityError::InvalidUrl("No scheme separator '://' found".to_string()))?;
    let scheme = &raw_url[..scheme_end];
    if scheme != "http" && scheme != "https" {
        return Err(SecurityError::DisallowedScheme(scheme.to_string()));
    }

    let after_scheme = &raw_url[scheme_end + 3..];
    let authority = after_scheme.split('/').next().unwrap_or(after_scheme);
    let host_part = authority.split('@').next_back().unwrap_or(authority);
    if host_part != authority {
        return Err(SecurityError::EmbeddedCredentials(raw_url.to_string()));
    }

    let host = host_part.split(':').next().unwrap_or(host_part);
    let bracket_close = host.rfind(']');
    let hostname = if host.starts_with('[') {
        bracket_close
            .map(|i| &host[1..i])
            .ok_or_else(|| SecurityError::InvalidUrl("Malformed IPv6 address".to_string()))?
    } else {
        host
    };

    let ip: Option<IpAddr> = hostname.parse().ok();

    if let Some(ip) = ip {
        if ip.is_loopback() && !config.allow_loopback {
            return Err(SecurityError::LoopbackNotAllowed(ip.to_string()));
        }
        if is_private_ip(&ip) && !config.allow_private_ips {
            return Err(SecurityError::PrivateIpNotAllowed(ip.to_string()));
        }
    }

    Ok(())
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 10
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
                || octets[0] == 127
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            segments[0] == 0xfc00 || segments[0] == 0xfd00 || is_ipv6_loopback(v6)
        }
    }
}

fn is_ipv6_loopback(v6: &Ipv6Addr) -> bool {
    v6.segments() == [0, 0, 0, 0, 0, 0, 0, 1]
}
