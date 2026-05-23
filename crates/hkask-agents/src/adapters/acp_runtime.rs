//! ACP Runtime Adapter
//!
//! Concrete implementation of ACPRuntimePort using acp-runtime crate.

use crate::pod::ACPRuntimePort;
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, SecretRef, WebID};

/// ACP Runtime Adapter — Concrete implementation for agent registration
pub struct AcpRuntimeAdapter {
    #[allow(dead_code)]
    registered_agents: std::collections::HashMap<String, CapabilityToken>,
    secret: Vec<u8>,
}

impl AcpRuntimeAdapter {
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            registered_agents: std::collections::HashMap::new(),
            secret,
        }
    }
}

impl Default for AcpRuntimeAdapter {
    fn default() -> Self {
        let secret = hkask_keystore::resolve(&SecretRef::env("HKASK_ACP_SECRET_KEY"))
            .unwrap_or_else(|_| {
                tracing::warn!("HKASK_ACP_SECRET_KEY not set, using generated secret");
                hkask_keystore::resolve(&SecretRef::generated(32))
                    .expect("generated secret cannot fail")
            });
        Self::new(secret.to_vec())
    }
}

impl ACPRuntimePort for AcpRuntimeAdapter {
    fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> {
        // Use the first capability as the resource ID, or "agent" as default
        let resource_id = capabilities
            .first()
            .cloned()
            .unwrap_or_else(|| "agent".to_string());
        
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            resource_id,
            CapabilityAction::Execute,
            WebID::new(),
            webid,
            &self.secret,
        );

        Ok(token)
    }
}
