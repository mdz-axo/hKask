//! A2A template dispatch handler
//!
//! Provides the `TemplateDispatchHandler` for processing template dispatch
//! requests and responses between agents.

use std::sync::Arc;

use tracing::info;

use super::{A2AMessage, AcpError, AcpRuntime};

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
    /// * `Err(AcpError)` — Dispatch error
    pub async fn dispatch(
        &self,
        from: hkask_types::WebID,
        to: Option<hkask_types::WebID>,
        template_id: String,
        input: serde_json::Value,
    ) -> Result<String, AcpError> {
        // Verify sender is registered
        if !self.acp_runtime.is_registered(&from).await {
            return Err(AcpError::AgentNotFound(from));
        }

        // Verify recipient if specified
        if let Some(recipient) = to
            && !self.acp_runtime.is_registered(&recipient).await
        {
            return Err(AcpError::AgentNotFound(recipient));
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
    ) -> Result<(), AcpError> {
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
        producer: hkask_types::WebID,
        artifact_type: String,
        artifact_id: String,
        visibility: String,
    ) -> Result<(), AcpError> {
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
