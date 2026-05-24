//! MCP security gateway
//!
//! Provides security middleware for MCP tool invocations:
//! - Capability verification (OCAP)
//! - Rate limiting
//! - Input validation
//! - Audit logging
//! - URL validation (SSRF protection)

use hkask_types::{CapabilityChecker, CapabilityToken};
use hkask_cns::rate_limit::RateLimiter;
use hkask_templates::TemplateError;
use hkask_types::WebID;
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
    /// Enable rate limiting
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
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Security policy
    policy: SecurityPolicy,
    /// Audit log (in-memory, replace with persistent storage in production)
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
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
            rate_limiter: RateLimiter::default(),
            policy,
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default policy
    pub fn with_default_policy(secret: &[u8]) -> Self {
        Self::new(secret, SecurityPolicy::default())
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
        self.capability_checker.check(token, bot_id, tool_name)
    }

    /// Check rate limit
    pub fn check_rate_limit(&self, bot_id: &WebID) -> bool {
        if !self.policy.enable_rate_limiting {
            return true;
        }
        self.rate_limiter.check(bot_id)
    }

    /// Get remaining rate limit tokens
    pub fn remaining_rate_limit(&self, bot_id: &WebID) -> u32 {
        self.rate_limiter.remaining(bot_id)
    }

    /// Record audit entry
    pub async fn audit(&self, entry: AuditEntry) {
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
        self.capability_checker.grant(tool_name, from, to)
    }
}

impl Default for SecurityGateway {
    fn default() -> Self {
        let secret =
            hkask_keystore::resolve(&hkask_types::SecretRef::env("HKASK_MCP_SECURITY_KEY"))
                .unwrap_or_else(|_| {
                    tracing::warn!("HKASK_MCP_SECURITY_KEY not set, using generated secret");
                    hkask_keystore::resolve(&hkask_types::SecretRef::generated(32))
                        .expect("generated secret cannot fail")
                });
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
    let scheme_end = raw_url.find("://").ok_or_else(|| {
        SecurityError::InvalidUrl("No scheme separator '://' found".to_string())
    })?;
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

#[cfg(test)]
mod url_validation_tests {
    use super::*;

    #[test]
    fn test_rejects_ftp_scheme() {
        let result = validate_url("ftp://example.com/file", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::DisallowedScheme(_))));
    }

    #[test]
    fn test_accepts_https() {
        let result = validate_url("https://example.com/path", &UrlValidationConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_accepts_http() {
        let result = validate_url("http://example.com/path", &UrlValidationConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_embedded_credentials() {
        let result = validate_url(
            "https://user:pass@example.com/path",
            &UrlValidationConfig::default(),
        );
        assert!(matches!(result, Err(SecurityError::EmbeddedCredentials(_))));
    }

    #[test]
    fn test_rejects_private_ip_default() {
        let result = validate_url("http://192.168.1.1/path", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::PrivateIpNotAllowed(_))));
    }

    #[test]
    fn test_allows_private_ip_when_configured() {
        let config = UrlValidationConfig {
            allow_private_ips: true,
            ..Default::default()
        };
        let result = validate_url("http://192.168.1.1/path", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_10_ip() {
        let result = validate_url("http://10.0.0.1/path", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::PrivateIpNotAllowed(_))));
    }

    #[test]
    fn test_rejects_172_ip() {
        let result = validate_url("http://172.16.0.1/path", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::PrivateIpNotAllowed(_))));
    }

    #[test]
    fn test_rejects_loopback_default() {
        let result = validate_url("http://127.0.0.1/path", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::LoopbackNotAllowed(_))));
    }

    #[test]
    fn test_allows_loopback_when_configured() {
        let config = UrlValidationConfig {
            allow_loopback: true,
            allow_private_ips: true,
        };
        let result = validate_url("http://127.0.0.1/path", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_no_scheme() {
        let result = validate_url("example.com/path", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::InvalidUrl(_))));
    }

    #[test]
    fn test_rejects_file_scheme() {
        let result = validate_url("file:///etc/passwd", &UrlValidationConfig::default());
        assert!(matches!(result, Err(SecurityError::DisallowedScheme(_))));
    }
}
