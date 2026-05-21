//! Capability-based access control for MCP tool invocation
//!
//! Implements OCAP (Object-Capability) security model for tool access.
//! Each bot must hold a capability token to invoke tools.

use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Helper to convert WebID to string
fn to_string(webid: &WebID) -> String {
    webid.to_string()
}

/// Capability token for tool access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Unique token identifier
    pub id: String,
    /// Tool name this capability grants access to
    pub tool_name: String,
    /// WebID that delegated this capability
    pub delegated_from: WebID,
    /// WebID that received this capability
    pub delegated_to: WebID,
    /// Token signature (HMAC over fields)
    pub signature: String,
}

impl CapabilityToken {
    /// Create a new capability token
    pub fn new(
        tool_name: String,
        delegated_from: WebID,
        delegated_to: WebID,
        secret: &[u8],
    ) -> Self {
        let id = Self::generate_id(&tool_name, &delegated_from, &delegated_to);
        let signature = Self::sign(&id, &tool_name, &delegated_from, &delegated_to, secret);

        Self {
            id,
            tool_name,
            delegated_from,
            delegated_to,
            signature,
        }
    }

    /// Generate unique token ID
    fn generate_id(tool_name: &str, from: &WebID, to: &WebID) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(to_string(from).as_bytes());
        hasher.update(to_string(to).as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Sign the token
    fn sign(id: &str, tool_name: &str, from: &WebID, to: &WebID, secret: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(id.as_bytes());
        mac.update(tool_name.as_bytes());
        mac.update(to_string(from).as_bytes());
        mac.update(to_string(to).as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Verify the token signature
    pub fn verify(&self, secret: &[u8]) -> bool {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let expected_signature = Self::sign(
            &self.id,
            &self.tool_name,
            &self.delegated_from,
            &self.delegated_to,
            secret,
        );

        // Constant-time comparison
        let result = HmacSha256::new_from_slice(secret);
        if result.is_err() {
            return false;
        }

        self.signature == expected_signature
    }

    /// Check if this token is valid for a given bot and tool
    pub fn is_valid_for(&self, bot_id: &WebID, tool_name: &str) -> bool {
        self.delegated_to == *bot_id && self.tool_name == tool_name
    }
}

/// Capability checker for MCP invocations
pub struct CapabilityChecker {
    secret: Vec<u8>,
}

impl CapabilityChecker {
    /// Create a new capability checker with the given secret
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    /// Verify a capability token
    pub fn verify(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.secret)
    }

    /// Check if a bot has capability to invoke a tool
    pub fn check(&self, token: &CapabilityToken, bot_id: &WebID, tool_name: &str) -> bool {
        self.verify(token) && token.is_valid_for(bot_id, tool_name)
    }

    /// Create a capability token for a bot
    pub fn grant(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        CapabilityToken::new(tool_name, from, to, &self.secret)
    }
}

/// Bot capability manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCapabilities {
    /// Bot's WebID
    pub bot_id: WebID,
    /// List of tool capabilities this bot holds
    pub capabilities: Vec<String>,
}

impl BotCapabilities {
    pub fn new(bot_id: WebID) -> Self {
        Self {
            bot_id,
            capabilities: vec![],
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<&str>) -> Self {
        self.capabilities = caps.into_iter().map(String::from).collect();
        self
    }

    pub fn has_capability(&self, tool_name: &str) -> bool {
        self.capabilities.iter().any(|cap| cap == tool_name)
    }
}


