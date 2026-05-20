//! MCP security gateway
//!
//! Provides security middleware for MCP tool invocations:
//! - Capability verification (OCAP)
//! - Rate limiting
//! - Input validation
//! - Audit logging

use hkask_agents::{CapabilityChecker, CapabilityToken};
use hkask_cns::rate_limit::RateLimiter;
use hkask_templates::TemplateError;
use hkask_types::WebID;
use serde_json::Value;
use std::collections::HashSet;
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
        let input_size = serde_json::to_string(input).map(|s| s.len()).unwrap_or(0);

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
        use hkask_types::{CapabilityAction, CapabilityResource};
        self.capability_checker.check(
            token,
            bot_id,
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
        )
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
        self.capability_checker.grant_tool(tool_name, from, to)
    }
}

impl Default for SecurityGateway {
    fn default() -> Self {
        Self::with_default_policy(b"default-secret-key-change-in-production")
    }
}

