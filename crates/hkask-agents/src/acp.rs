//! ACP (Agent Communication Protocol) Runtime Integration
//!
//! This module provides ACP runtime adapters for agent registration,
//! A2A message handling, and capability-gated communication.
//!
//! # A2A Message Flow
//!
//! ```text
//! Agent Pod A → ACP Message (template:dispatch) → hKask Router → Agent Pod B
//!                                                              ↓
//!                                                   Capability Verification
//!                                                              ↓
//!                                                   Template Execution
//!                                                              ↓
//!                                                   Memory Artifact Generation
//!                                                              ↓
//!                                                   CNS Span Emission
//!                                                              ↓
//!                                                   Response to Agent A
//! ```

use hkask_types::{CapabilityToken, CapabilityResource, CapabilityAction, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// ACP agent registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpAgent {
    /// Agent WebID
    pub webid: WebID,
    /// Agent type (Bot or Replicant)
    pub agent_type: String,
    /// Registered capabilities
    pub capabilities: Vec<String>,
    /// Registration timestamp (Unix epoch)
    pub registered_at: i64,
    /// Active status
    pub active: bool,
}

/// A2A message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum A2AMessage {
    /// Template dispatch request
    TemplateDispatch {
        /// Sender WebID
        from: WebID,
        /// Recipient WebID (or broadcast)
        to: Option<WebID>,
        /// Template ID to invoke
        template_id: String,
        /// Input data
        input: serde_json::Value,
        /// Correlation ID for response matching
        correlation_id: String,
    },
    /// Template dispatch response
    TemplateResponse {
        /// Original correlation ID
        correlation_id: String,
        /// Response data
        result: serde_json::Value,
        /// Error message (if any)
        error: Option<String>,
    },
    /// Memory artifact notification
    MemoryArtifact {
        /// Producer WebID
        producer: WebID,
        /// Artifact type
        artifact_type: String,
        /// Artifact ID
        artifact_id: String,
        /// Visibility setting
        visibility: String,
    },
}

/// ACP runtime for agent registration and message routing
pub struct AcpRuntime {
    /// Registered agents
    agents: Arc<RwLock<HashMap<WebID, AcpAgent>>>,
    /// Pending messages (correlation_id -> message)
    pending_messages: Arc<RwLock<HashMap<String, A2AMessage>>>,
    /// Capability tokens indexed by holder WebID
    capability_tokens: Arc<RwLock<HashMap<WebID, Vec<CapabilityToken>>>>,
    /// Secret for HMAC signing
    secret: Vec<u8>,
}

impl AcpRuntime {
    /// Create new ACP runtime
    pub fn new(secret: &[u8]) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
            capability_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret: secret.to_vec(),
        }
    }

    /// Register an agent with the ACP runtime
    ///
    /// # Arguments
    /// * `webid` — Agent's WebID
    /// * `agent_type` — "Bot" or "Replicant"
    /// * `capabilities` — List of capability strings
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Primary capability token for the agent
    /// * `Err(String)` — Registration error
    pub async fn register_agent(
        &self,
        webid: WebID,
        agent_type: String,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&webid) {
            return Err(format!("Agent {:?} already registered", webid));
        }

        let agent = AcpAgent {
            webid,
            agent_type: agent_type.clone(),
            capabilities: capabilities.clone(),
            registered_at: current_timestamp(),
            active: true,
        };

        // Create primary capability token
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "*".to_string(),
            CapabilityAction::Execute,
            webid,
            webid,
            &self.secret,
        );

        // Store agent and capabilities
        agents.insert(webid, agent);

        // Store capability token
        let mut tokens = self.capability_tokens.write().await;
        tokens.entry(webid).or_insert_with(Vec::new).push(token.clone());

        info!(
            target: "hkask.acp",
            webid = %webid,
            agent_type = %agent_type,
            capabilities = ?capabilities,
            "Agent registered with ACP runtime"
        );

        Ok(token)
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), String> {
        let mut agents = self.agents.write().await;

        if agents.remove(webid).is_none() {
            return Err(format!("Agent {:?} not found", webid));
        }

        // Remove capability tokens
        let mut tokens = self.capability_tokens.write().await;
        tokens.remove(webid);

        info!(
            target: "hkask.acp",
            webid = %webid,
            "Agent unregistered from ACP runtime"
        );

        Ok(())
    }

    /// Get agent by WebID
    pub async fn get_agent(&self, webid: &WebID) -> Option<AcpAgent> {
        let agents = self.agents.read().await;
        agents.get(webid).cloned()
    }

    /// Check if agent is registered
    pub async fn is_registered(&self, webid: &WebID) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(webid)
    }

    /// Send A2A message
    pub async fn send_message(&self, message: A2AMessage) -> Result<String, String> {
        let correlation_id = match &message {
            A2AMessage::TemplateDispatch { correlation_id, .. } => correlation_id.clone(),
            A2AMessage::TemplateResponse { correlation_id, .. } => correlation_id.clone(),
            A2AMessage::MemoryArtifact { artifact_id, .. } => {
                format!("artifact:{}", artifact_id)
            }
        };

        let mut pending = self.pending_messages.write().await;
        pending.insert(correlation_id.clone(), message);

        info!(
            target: "hkask.acp",
            correlation_id = %correlation_id,
            "A2A message sent"
        );

        Ok(correlation_id)
    }

    /// Get pending message by correlation ID
    pub async fn get_message(&self, correlation_id: &str) -> Option<A2AMessage> {
        let pending = self.pending_messages.read().await;
        pending.get(correlation_id).cloned()
    }

    /// Remove pending message
    pub async fn remove_message(&self, correlation_id: &str) -> Option<A2AMessage> {
        let mut pending = self.pending_messages.write().await;
        pending.remove(correlation_id)
    }

    /// Verify capability token
    pub fn verify_capability(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.secret)
    }

    /// Store capability token for agent
    pub async fn store_capability(&self, webid: WebID, token: CapabilityToken) {
        let mut tokens = self.capability_tokens.write().await;
        tokens.entry(webid).or_insert_with(Vec::new).push(token);
    }

    /// Get all capability tokens for agent
    pub async fn get_capabilities(&self, webid: &WebID) -> Vec<CapabilityToken> {
        let tokens = self.capability_tokens.read().await;
        tokens.get(webid).cloned().unwrap_or_default()
    }

    /// Check if agent has capability for tool
    pub async fn has_capability(&self, webid: &WebID, tool_name: &str) -> bool {
        let tokens = self.capability_tokens.read().await;
        if let Some(agent_tokens) = tokens.get(webid) {
            agent_tokens.iter().any(|token| {
                token.resource == CapabilityResource::Tool
                    && (token.resource_id == "*" || token.resource_id == tool_name)
                    && token.action == CapabilityAction::Execute
                    && token.verify(&self.secret)
            })
        } else {
            false
        }
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AcpAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl Default for AcpRuntime {
    fn default() -> Self {
        Self::new(b"acp-default-secret-key")
    }
}

/// A2A template dispatch handler
pub struct TemplateDispatchHandler {
    acp_runtime: Arc<AcpRuntime>,
}

impl TemplateDispatchHandler {
    /// Create new dispatch handler
    pub fn new(acp_runtime: Arc<AcpRuntime>) -> Self {
        Self { acp_runtime }
    }

    /// Process template dispatch request
    ///
    /// # Arguments
    /// * `from` — Sender WebID
    /// * `to` — Recipient WebID (optional for broadcast)
    /// * `template_id` — Template to invoke
    /// * `input` — Input data
    ///
    /// # Returns
    /// * `Ok(correlation_id)` — Message correlation ID
    /// * `Err(String)` — Dispatch error
    pub async fn dispatch(
        &self,
        from: WebID,
        to: Option<WebID>,
        template_id: String,
        input: serde_json::Value,
    ) -> Result<String, String> {
        // Verify sender is registered
        if !self.acp_runtime.is_registered(&from).await {
            return Err(format!("Sender {:?} not registered", from));
        }

        // Verify recipient if specified
        if let Some(recipient) = to {
            if !self.acp_runtime.is_registered(&recipient).await {
                return Err(format!("Recipient {:?} not registered", recipient));
            }
        }

        let correlation_id = uuid::Uuid::new_v4().to_string();
        let template_id_clone = template_id.clone();

        let message = A2AMessage::TemplateDispatch {
            from,
            to,
            template_id,
            input,
            correlation_id: correlation_id.clone(),
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            from = %from,
            to = ?to,
            template_id = %template_id_clone,
            correlation_id = %correlation_id,
            "Template dispatch sent"
        );

        Ok(correlation_id)
    }

    /// Process template dispatch response
    pub async fn respond(
        &self,
        correlation_id: String,
        result: serde_json::Value,
        error: Option<String>,
    ) -> Result<(), String> {
        let message = A2AMessage::TemplateResponse {
            correlation_id: correlation_id.clone(),
            result,
            error,
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            correlation_id = %correlation_id,
            "Template dispatch response sent"
        );

        Ok(())
    }

    /// Notify memory artifact creation
    pub async fn notify_artifact(
        &self,
        producer: WebID,
        artifact_type: String,
        artifact_id: String,
        visibility: String,
    ) -> Result<(), String> {
        let artifact_id_clone = artifact_id.clone();
        let artifact_type_clone = artifact_type.clone();
        
        let message = A2AMessage::MemoryArtifact {
            producer,
            artifact_type,
            artifact_id,
            visibility,
        };

        self.acp_runtime.send_message(message).await?;

        info!(
            target: "hkask.acp",
            producer = %producer,
            artifact_id = %artifact_id_clone,
            artifact_type = %artifact_type_clone,
            "Memory artifact notification sent"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acp_runtime_register_agent() {
        let runtime = AcpRuntime::new(b"test-secret");
        let webid = WebID::new();

        let token = runtime
            .register_agent(webid, "Bot".to_string(), vec!["inference:call".to_string()])
            .await
            .unwrap();

        assert!(runtime.is_registered(&webid).await);
        assert!(runtime.verify_capability(&token));

        let agent = runtime.get_agent(&webid).await.unwrap();
        assert_eq!(agent.webid, webid);
        assert_eq!(agent.agent_type, "Bot");
        assert_eq!(agent.capabilities.len(), 1);
    }

    #[tokio::test]
    async fn test_acp_runtime_unregister_agent() {
        let runtime = AcpRuntime::new(b"test-secret");
        let webid = WebID::new();

        runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await
            .unwrap();

        assert!(runtime.is_registered(&webid).await);

        runtime.unregister_agent(&webid).await.unwrap();

        assert!(!runtime.is_registered(&webid).await);
    }

    #[tokio::test]
    async fn test_acp_runtime_duplicate_registration() {
        let runtime = AcpRuntime::new(b"test-secret");
        let webid = WebID::new();

        runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let result = runtime
            .register_agent(webid, "Bot".to_string(), vec![])
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already registered"));
    }

    #[tokio::test]
    async fn test_acp_runtime_send_message() {
        let runtime = AcpRuntime::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();

        runtime
            .register_agent(from, "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(to, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let handler = TemplateDispatchHandler::new(Arc::new(runtime));

        let correlation_id = handler
            .dispatch(from, Some(to), "test/template".to_string(), serde_json::json!({"test": "data"}))
            .await
            .unwrap();

        assert!(!correlation_id.is_empty());
    }

    #[tokio::test]
    async fn test_acp_runtime_capability_check() {
        let runtime = AcpRuntime::new(b"test-secret");
        let webid = WebID::new();

        // Register agent - creates wildcard capability token
        runtime
            .register_agent(webid, "Bot".to_string(), vec!["inference:call".to_string()])
            .await
            .unwrap();

        // Wildcard token allows any tool
        assert!(runtime.has_capability(&webid, "inference:call").await);
        assert!(runtime.has_capability(&webid, "memory:write").await);
        
        // Unregistered agent has no capabilities
        let other_webid = WebID::new();
        assert!(!runtime.has_capability(&other_webid, "inference:call").await);
    }

    #[tokio::test]
    async fn test_acp_runtime_list_agents() {
        let runtime = AcpRuntime::new(b"test-secret");

        runtime
            .register_agent(WebID::new(), "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(WebID::new(), "Replicant".to_string(), vec![])
            .await
            .unwrap();

        let agents = runtime.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_template_dispatch_handler() {
        let runtime = Arc::new(AcpRuntime::new(b"test-secret"));
        let from = WebID::new();
        let to = WebID::new();

        runtime
            .register_agent(from, "Bot".to_string(), vec![])
            .await
            .unwrap();
        runtime
            .register_agent(to, "Bot".to_string(), vec![])
            .await
            .unwrap();

        let handler = TemplateDispatchHandler::new(runtime.clone());

        // Dispatch
        let correlation_id = handler
            .dispatch(from, Some(to), "test/template".to_string(), serde_json::json!({"input": "test"}))
            .await
            .unwrap();

        // Get message
        let message = runtime.get_message(&correlation_id).await.unwrap();
        assert!(matches!(message, A2AMessage::TemplateDispatch { .. }));

        // Respond
        handler
            .respond(correlation_id.clone(), serde_json::json!({"result": "success"}), None)
            .await
            .unwrap();

        // Get response
        let response = runtime.get_message(&correlation_id).await.unwrap();
        assert!(matches!(response, A2AMessage::TemplateResponse { .. }));
    }
}
