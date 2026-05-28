//! PodContext — Runtime context for an active pod

use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use std::sync::Arc;

use super::AgentPodError;
use super::manager::PodManager;
use super::types::PodID;
use crate::ports::{MCPRuntimePort, MemoryStoragePort};

/// PodContext — Runtime context for an active pod
///
/// Provides access to all ports (inference, memory, MCP, CNS) for a specific pod.
/// This is the unit of access that enforces the pod invariant: all interactions
/// with memory, inference, and tools must go through a pod context.
pub struct PodContext {
    pub pod_id: PodID,
    pub webid: WebID,
    pub capability_token: CapabilityToken,
    inference_port: Option<Arc<dyn hkask_templates::InferencePort>>,
    memory_storage: Arc<dyn MemoryStoragePort>,
    mcp_runtime: Arc<dyn MCPRuntimePort>,
    cns_emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>,
}

impl PodContext {
    pub async fn from_manager(manager: &PodManager, pod_id: &PodID) -> Result<Self, AgentPodError> {
        let pods = manager.pods.read().await;
        let pod = pods
            .get(pod_id)
            .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

        if pod.state != super::types::PodLifecycleState::Activated {
            return Err(AgentPodError::ACPRegistrationError(
                "Pod must be activated before creating context".to_string(),
            ));
        }

        Ok(Self {
            pod_id: *pod_id,
            webid: pod.webid,
            capability_token: pod.capability_token.clone(),
            inference_port: manager.inference_port.clone(),
            memory_storage: Arc::clone(&manager.memory_storage),
            mcp_runtime: Arc::clone(&manager.mcp_runtime),
            cns_emitter: Arc::clone(&manager.cns_emitter),
        })
    }

    fn require_capability(
        &self,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> Result<(), AgentPodError> {
        if !self
            .capability_token
            .is_valid_for(resource, resource_id, action)
        {
            return Err(AgentPodError::CapabilityDenied { resource, action });
        }
        Ok(())
    }

    pub fn inference_port(&self) -> Result<Arc<dyn hkask_templates::InferencePort>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Template,
            "inference",
            CapabilityAction::Render,
        )?;
        self.inference_port.clone().ok_or_else(|| {
            AgentPodError::InferenceUnavailable("No inference port configured".to_string())
        })
    }

    pub async fn recall_memory(
        &self,
        query: &str,
    ) -> Result<Vec<serde_json::Value>, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "memory",
            CapabilityAction::Read,
        )?;
        self.memory_storage
            .recall(query, &self.capability_token)
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    pub async fn store_memory(
        &self,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
    ) -> Result<String, AgentPodError> {
        self.require_capability(
            CapabilityResource::Manifest,
            "memory",
            CapabilityAction::Write,
        )?;
        self.memory_storage
            .store_artifact(
                self.webid,
                artifact_type,
                content,
                visibility,
                &self.capability_token,
            )
            .map_err(|e| AgentPodError::MemoryError(e.to_string()))
    }

    pub fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AgentPodError> {
        self.require_capability(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
        )?;
        self.emit_span(
            &format!("cns.tool.{}", tool_name),
            "invoked",
            serde_json::json!({ "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>()) }),
        );
        let result = self
            .mcp_runtime
            .invoke_tool(tool_name, input, &self.capability_token)
            .map_err(|e| AgentPodError::ToolError(e.to_string()));
        match &result {
            Ok(_) => self.emit_span(
                &format!("cns.tool.{}.completed", tool_name),
                "completed",
                serde_json::json!({}),
            ),
            Err(_) => self.emit_span(
                &format!("cns.tool.{}.failed", tool_name),
                "failed",
                serde_json::json!({}),
            ),
        }
        result
    }

    pub fn emit_span(&self, span_type: &str, action: &str, data: serde_json::Value) {
        self.cns_emitter.emit_event(
            span_type,
            "action",
            &serde_json::json!({
                "pod_id": self.pod_id.to_string(),
                "webid": self.webid.to_string(),
                "action": action,
                "data": data,
            }),
            1.0,
        );
    }
}
