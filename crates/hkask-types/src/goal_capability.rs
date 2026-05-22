//! hKask Goal Capability — OCAP-gated capability tokens for goal delegation
//!
//! **Design Principles:**
//! - Capability tokens grant authority to act on behalf of a goal
//! - Delegation attenuates (reduces) capabilities
//! - HMAC-SHA256 signatures prevent tampering
//! - Expiration limits token lifetime

use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

use crate::goal::GoalId;

type HmacSha256 = Hmac<Sha256>;

/// Capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub uuid::Uuid);

impl CapabilityId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

/// Goal-specific actions (OCAP authority)
///
/// Each action type can be independently granted or denied.
/// Delegation attenuates by removing write permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum GoalAction {
    /// Call a specific MCP tool
    ToolCall {
        mcp_server: String,
        tool_name: String,
    },
    /// Read files matching pattern
    ReadFile {
        path_pattern: String,
    },
    /// Write files matching pattern
    WriteFile {
        path_pattern: String,
    },
    /// Execute specific commands
    ExecuteCommand {
        allowed_commands: Vec<String>,
    },
    /// Delegate goal to another agent
    DelegateGoal {
        max_attenuation: u8,
    },
}

/// Goal capability token with attenuation
///
/// **Structure:**
/// - `id`: Unique capability identifier
/// - `goal_id`: Goal this capability is for
/// - `owner_webid`: Original goal owner
/// - `holder_webid`: Current capability holder
/// - `allowed_actions`: Granted actions
/// - `attenuation_level`: Increases on each delegation (0 = original)
/// - `max_attenuation`: Maximum allowed attenuation level
/// - `expiration`: Unix timestamp when capability expires
/// - `hmac_signature`: HMAC-SHA256 over (id, goal_id, holder, expiration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCapability {
    pub id: CapabilityId,
    pub goal_id: GoalId,
    pub owner_webid: String,
    pub holder_webid: String,
    pub allowed_actions: Vec<GoalAction>,
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub expiration: i64,
    pub hmac_signature: Vec<u8>,
}

impl GoalCapability {
    /// Create new capability with HMAC signature
    pub fn new(
        goal_id: GoalId,
        owner_webid: String,
        holder_webid: String,
        allowed_actions: Vec<GoalAction>,
        max_attenuation: u8,
        expiration: i64,
        secret_key: &[u8],
    ) -> Result<Self, CapabilityError> {
        let id = CapabilityId::new();
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .map_err(|_| CapabilityError::InvalidKey)?;
        
        // Sign: id || goal_id || holder_webid || expiration
        mac.update(id.0.as_bytes());
        mac.update(goal_id.0.as_bytes());
        mac.update(holder_webid.as_bytes());
        mac.update(&expiration.to_be_bytes());
        
        let signature = mac.finalize().into_bytes().to_vec();
        
        Ok(Self {
            id,
            goal_id,
            owner_webid,
            holder_webid,
            allowed_actions,
            attenuation_level: 0,
            max_attenuation,
            expiration,
            hmac_signature: signature,
        })
    }
    
    /// Verify HMAC signature
    pub fn verify(&self, secret_key: &[u8]) -> Result<(), CapabilityError> {
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .map_err(|_| CapabilityError::InvalidKey)?;
        
        mac.update(self.id.0.as_bytes());
        mac.update(self.goal_id.0.as_bytes());
        mac.update(self.holder_webid.as_bytes());
        mac.update(&self.expiration.to_be_bytes());
        
        mac.verify_slice(&self.hmac_signature)
            .map_err(|_| CapabilityError::InvalidSignature)
    }
    
    /// Check if capability is expired
    pub fn is_expired(&self) -> bool {
        self.expiration <= get_current_timestamp()
    }
    
    /// Delegate capability with attenuation
    ///
    /// **Attenuation rules:**
    /// - Remove write permissions (keep read-only)
    /// - Halve remaining expiration time
    /// - Increment attenuation level
    pub fn delegate(
        &self,
        new_holder: String,
        secret_key: &[u8],
    ) -> Result<Self, DelegationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(DelegationError::MaxAttenuationReached);
        }
        
        if self.is_expired() {
            return Err(DelegationError::Expired);
        }
        
        // Attenuate: keep only read-only actions
        let attenuated_actions = self
            .allowed_actions
            .iter()
            .filter(|action| {
                matches!(
                    action,
                    GoalAction::ReadFile { .. } | GoalAction::ToolCall { .. }
                )
            })
            .cloned()
            .collect();
        
        // Halve remaining time
        let now = get_current_timestamp();
        let remaining = self.expiration - now;
        let new_expiration = now + (remaining / 2);
        
        Self::new(
            self.goal_id,
            self.owner_webid.clone(),
            new_holder,
            attenuated_actions,
            self.max_attenuation,
            new_expiration,
            secret_key,
        )
        .map(|mut child| {
            child.attenuation_level = self.attenuation_level + 1;
            child
        })
    }
    
    /// Check if action is allowed
    pub fn allows(&self, action: &GoalAction) -> bool {
        self.allowed_actions.iter().any(|a| {
            std::mem::discriminant(a) == std::mem::discriminant(action)
        })
    }
}

/// Capability verification errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    #[error("invalid HMAC key")]
    InvalidKey,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("capability expired")]
    Expired,
}

/// Delegation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DelegationError {
    #[error("maximum attenuation level reached")]
    MaxAttenuationReached,
    #[error("capability expired")]
    Expired,
}

fn get_current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
