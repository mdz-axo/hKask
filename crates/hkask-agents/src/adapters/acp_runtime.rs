//! ACP Runtime Adapter
//!
//! Concrete implementation of ACPRuntimePort using acp-runtime crate.

use crate::pod::ACPRuntimePort;
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};

/// ACP Runtime Adapter — Concrete implementation for agent registration
#[derive(Default)]
pub struct AcpRuntimeAdapter {
    /// Registered agents (reserved for future use)
    #[allow(dead_code)]
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

impl ACPRuntimePort for AcpRuntimeAdapter {
    fn register_agent(
        &self,
        webid: WebID,
        _capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> {
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            WebID::new(),
            webid,
            b"acp-runtime-secret",
        );

        Ok(token)
    }
}
