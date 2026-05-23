//! Per-step capability tokens for manifest execution
//!
//! Implements fine-grained capability attenuation for individual manifest steps.
//! Each step receives minimally-scoped authority (Mark Miller OCAP principle).
//!
//! **Design Principles:**
//! - Least authority: each step gets only what it needs
//! - Monotonic attenuation: can only remove authority, never add
//! - Step isolation: capability for step N cannot access step N+1
//! - TOCTOU prevention: capability checked at use time, not just grant time

use crate::ports::ManifestStep;
use chrono::{DateTime, Utc};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Step-specific capability with minimal authority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCapability {
    /// Unique step capability identifier
    pub step_id: String,
    /// Parent capability ID (for audit trail)
    pub parent_capability: Option<String>,
    /// Allowed actions for this step
    pub allowed_actions: Vec<StepAction>,
    /// Allowed template references
    pub allowed_templates: Vec<String>,
    /// Allowed MCP targets
    pub allowed_mcps: Vec<String>,
    /// Expiration time
    pub expires_at: DateTime<Utc>,
    /// Attenuation level (0 = root, increases with each delegation)
    pub attenuation_level: u8,
    /// Maximum attenuation depth (Mark Miller limit)
    pub max_attenuation: u8,
    /// Holder of this capability
    pub holder: WebID,
    /// Issuer of this capability
    pub issuer: WebID,
}

/// Step action types (mirrors manifest actions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepAction {
    Select,
    Populate,
    Execute,
}

impl StepAction {
    pub fn from_manifest_action(action: crate::ports::Action) -> Self {
        match action {
            crate::ports::Action::Select => StepAction::Select,
            crate::ports::Action::Populate => StepAction::Populate,
            crate::ports::Action::Execute => StepAction::Execute,
        }
    }
}

/// Authorization error for step capabilities
#[derive(Debug, thiserror::Error)]
pub enum StepAuthError {
    #[error("Step action not authorized: {action:?}")]
    StepActionNotAuthorized { action: StepAction },

    #[error("Template not authorized: {template_ref}")]
    TemplateNotAuthorized { template_ref: String },

    #[error("MCP not authorized: {mcp}")]
    McpNotAuthorized { mcp: String },

    #[error("Capability expired at {expired_at}")]
    CapabilityExpired { expired_at: DateTime<Utc> },

    #[error("Maximum attenuation depth reached ({max})")]
    MaxAttenuationReached { max: u8 },

    #[error("No valid operations for attenuation (subset must be non-empty)")]
    NoValidOperations,

    #[error("Holder mismatch: expected {expected}, got {actual}")]
    HolderMismatch { expected: WebID, actual: WebID },

    #[error("Capability usage exceeded: max {max}, current {current}")]
    UsageExceeded { max: u64, current: u64 },
}

impl StepCapability {
    /// Create root step capability
    pub fn new(
        step_id: &str,
        allowed_actions: Vec<StepAction>,
        allowed_templates: Vec<String>,
        allowed_mcps: Vec<String>,
        expires_at: DateTime<Utc>,
        holder: WebID,
        issuer: WebID,
    ) -> Self {
        Self {
            step_id: step_id.to_string(),
            parent_capability: None,
            allowed_actions,
            allowed_templates,
            allowed_mcps,
            expires_at,
            attenuation_level: 0,
            max_attenuation: 7, // Matroshka limit
            holder,
            issuer,
        }
    }

    /// Create attenuated capability for specific step
    pub fn attenuate_for_step(
        &self,
        step: &ManifestStep,
        new_holder: WebID,
    ) -> Result<StepCapability, StepAuthError> {
        // Check attenuation depth
        if self.attenuation_level >= self.max_attenuation {
            return Err(StepAuthError::MaxAttenuationReached {
                max: self.max_attenuation,
            });
        }

        // Convert step action to StepAction
        let step_action = StepAction::from_manifest_action(step.action);

        // Verify step action is in allowed_actions
        if !self.allowed_actions.contains(&step_action) {
            return Err(StepAuthError::StepActionNotAuthorized {
                action: step_action,
            });
        }

        // Verify template_ref is in allowed_templates (if allowlist is non-empty)
        if !self.allowed_templates.is_empty()
            && !self.allowed_templates.contains(&step.template_ref)
        {
            return Err(StepAuthError::TemplateNotAuthorized {
                template_ref: step.template_ref.clone(),
            });
        }

        // Verify MCP is in allowed_mcps (if allowlist is non-empty)
        if let Some(mcp) = &step.mcp
            && !self.allowed_mcps.is_empty()
            && !self.allowed_mcps.contains(mcp)
        {
            return Err(StepAuthError::McpNotAuthorized { mcp: mcp.clone() });
        }

        // Create attenuated capability with minimal authority
        Ok(StepCapability {
            step_id: format!("{}.{}", self.step_id, step.ordinal),
            parent_capability: Some(self.step_id.clone()),
            allowed_actions: vec![step_action], // Further attenuated to single action
            allowed_templates: vec![step.template_ref.clone()],
            allowed_mcps: step.mcp.iter().cloned().collect(),
            expires_at: self.expires_at, // Cannot extend expiration
            attenuation_level: self.attenuation_level + 1,
            max_attenuation: self.max_attenuation,
            holder: new_holder,
            issuer: self.issuer,
        })
    }

    /// Check if capability authorizes a step
    pub fn authorize_step(&self, step: &ManifestStep) -> Result<(), StepAuthError> {
        // Check expiration
        if Utc::now() > self.expires_at {
            return Err(StepAuthError::CapabilityExpired {
                expired_at: self.expires_at,
            });
        }

        // Verify action
        let step_action = StepAction::from_manifest_action(step.action);
        if !self.allowed_actions.contains(&step_action) {
            return Err(StepAuthError::StepActionNotAuthorized {
                action: step_action,
            });
        }

        // Verify template (if allowlist is non-empty)
        if !self.allowed_templates.is_empty()
            && !self.allowed_templates.contains(&step.template_ref)
        {
            return Err(StepAuthError::TemplateNotAuthorized {
                template_ref: step.template_ref.clone(),
            });
        }

        // Verify MCP (if allowlist is non-empty)
        if let Some(mcp) = &step.mcp
            && !self.allowed_mcps.is_empty()
            && !self.allowed_mcps.contains(mcp)
        {
            return Err(StepAuthError::McpNotAuthorized { mcp: mcp.clone() });
        }

        Ok(())
    }

    /// Check if holder matches
    pub fn verify_holder(&self, holder: &WebID) -> Result<(), StepAuthError> {
        if &self.holder != holder {
            return Err(StepAuthError::HolderMismatch {
                expected: self.holder,
                actual: *holder,
            });
        }
        Ok(())
    }
}

/// Atomic capability with check-and-use semantics (TOCTOU prevention)
pub struct AtomicCapability {
    inner: Arc<RwLock<StepCapability>>,
    usage_count: Arc<std::sync::atomic::AtomicU64>,
    max_usage: u64,
}

impl AtomicCapability {
    /// Create new atomic capability
    pub fn new(capability: StepCapability, max_usage: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(capability)),
            usage_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            max_usage,
        }
    }

    /// Check capability and execute atomically
    pub async fn check_and_execute<F, T>(&self, operation: F) -> Result<T, StepAuthError>
    where
        F: FnOnce(&StepCapability) -> Result<T, StepAuthError>,
    {
        // Atomic: check usage, increment, execute
        let current = self
            .usage_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if current >= self.max_usage {
            self.usage_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst); // Rollback
            return Err(StepAuthError::UsageExceeded {
                max: self.max_usage,
                current,
            });
        }

        let cap = self.inner.read().await;

        // Check expiration atomically with usage
        if Utc::now() > cap.expires_at {
            return Err(StepAuthError::CapabilityExpired {
                expired_at: cap.expires_at,
            });
        }

        operation(&cap)
    }

    /// Get current usage count
    pub fn usage_count(&self) -> u64 {
        self.usage_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get capability reference (read-only)
    pub async fn get_capability(&self) -> tokio::sync::RwLockReadGuard<'_, StepCapability> {
        self.inner.read().await
    }
}

impl Clone for AtomicCapability {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            usage_count: Arc::clone(&self.usage_count),
            max_usage: self.max_usage,
        }
    }
}






