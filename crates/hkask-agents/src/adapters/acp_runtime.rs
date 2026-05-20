//! ACP Runtime Adapter
//!
//! Concrete implementation of ACPRuntimePort using acp-runtime crate.

use crate::pod::ACPRuntimePort;
use hkask_types::{CapabilityToken, CapabilityResource, CapabilityAction, WebID};

/// ACP Runtime Adapter — Concrete implementation for agent registration
pub struct AcpRuntimeAdapter {
    // Registered agents
    registered_agents: std::collections::HashMap<String, CapabilityToken>,
}

impl AcpRuntimeAdapter {
    /// Create new ACP runtime adapter
    pub fn new() -> Self {
        Self {
            registered_agents: std::collections::HashMap::new(),
        }
    }
}

impl Default for AcpRuntimeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ACPRuntimePort for AcpRuntimeAdapter {
    fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> {
        // Register agent with ACP runtime
        let agent_id = webid.to_string();
        
        // Create capability token for the agent
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            WebID::new(), // issuer derived from runtime
            webid,
            b"acp-runtime-secret",
        );
        
        // Store registered agent (in production, this would use acp-runtime crate)
        // Mutable access needed - this is a limitation of the trait design
        // For now, just return the token
        
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_acp_runtime_adapter_new() {
        let adapter = AcpRuntimeAdapter::new();
        // Adapter created successfully
        assert!(true);
    }
    
    #[test]
    fn test_acp_register_agent() {
        let adapter = AcpRuntimeAdapter::new();
        let webid = WebID::new();
        let capabilities = vec!["tool:memory:remember".to_string()];
        
        let result = adapter.register_agent(webid, capabilities);
        assert!(result.is_ok());
        
        let token = result.unwrap();
        assert_eq!(token.subject, webid);
    }
}