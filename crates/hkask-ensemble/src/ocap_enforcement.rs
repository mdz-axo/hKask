//! OCAP Enforcement Middleware
//!
//! Provides capability-based authorization at Okapi and MCP boundaries.
//! Enforces principle of least authority (Mark Miller / Bruce Schneier).

use crate::okapi_capability::{OkapiCapabilityError, OkapiOperation};
use crate::webid_registry::WebIDCapabilityRegistry;
use hkask_types::{CapabilityToken, Visibility, WebID};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Port for capability queries (hexagonal architecture)
#[async_trait::async_trait]
pub trait CapabilityQueryPort: Send + Sync {
    /// Check if WebID has capability for operation
    async fn has_capability(&self, webid: WebID, operation: OkapiOperation) -> bool;

    /// Get all capabilities for WebID
    async fn get_capabilities(&self, webid: WebID) -> Option<Vec<CapabilityToken>>;
}

/// Port for security metrics (hexagonal architecture)
pub trait SecurityMetricPort: Send + Sync {
    /// Record OCAP enforcement metric
    fn record_ocap_event(
        &self,
        granted: bool,
        requester: &str,
        operation: &str,
        error: Option<&str>,
    );
}

/// OCAP enforcement result
#[derive(Debug, Clone)]
pub struct OcapEnforcementResult {
    /// Whether access was granted
    pub granted: bool,
    /// WebID of the requester
    pub requester: WebID,
    /// Operation requested
    pub operation: OkapiOperation,
    /// Capability used (if granted)
    pub capability: Option<CapabilityToken>,
    /// Error message (if denied)
    pub error: Option<String>,
}

/// OCAP enforcement context
#[derive(Debug, Clone)]
pub struct OcapContext {
    /// Requester WebID
    pub requester: WebID,
    /// Operation being requested
    pub operation: OkapiOperation,
    /// Required visibility level
    pub required_visibility: Visibility,
    /// Resource being accessed (optional)
    pub resource: Option<String>,
}

impl OcapContext {
    pub fn new(requester: WebID, operation: OkapiOperation) -> Self {
        Self {
            requester,
            operation,
            required_visibility: Visibility::Private,
            resource: None,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.required_visibility = visibility;
        self
    }

    pub fn with_resource(mut self, resource: &str) -> Self {
        self.resource = Some(resource.to_string());
        self
    }
}

/// OCAP enforcement engine with CNS metrics
pub struct OcapEnforcer {
    registry: Arc<WebIDCapabilityRegistry>,
    metrics: Option<Arc<dyn SecurityMetricPort>>,
}

impl OcapEnforcer {
    /// Create new enforcer without metrics
    pub fn new(registry: Arc<WebIDCapabilityRegistry>) -> Self {
        Self {
            registry,
            metrics: None,
        }
    }

    /// Create new enforcer with metrics adapter
    pub fn with_metrics(
        registry: Arc<WebIDCapabilityRegistry>,
        metrics: Arc<dyn SecurityMetricPort>,
    ) -> Self {
        Self {
            registry,
            metrics: Some(metrics),
        }
    }

    /// Enforce capability for an operation
    pub async fn enforce(
        &self,
        context: OcapContext,
    ) -> Result<OcapEnforcementResult, OkapiCapabilityError> {
        debug!(
            "Enforcing OCAP: requester={}, operation={:?}, visibility={:?}",
            context.requester, context.operation, context.required_visibility
        );

        // Check if requester has capability
        let has_cap = self
            .registry
            .has_capability(context.requester, context.operation)
            .await;

        if !has_cap {
            warn!(
                "OCAP denied: requester={} lacks capability for operation={:?}",
                context.requester, context.operation
            );

            let result = OcapEnforcementResult {
                granted: false,
                requester: context.requester,
                operation: context.operation,
                capability: None,
                error: Some(format!(
                    "Capability not found for operation {:?}",
                    context.operation
                )),
            };
            self.record_ocap_metric(&result);
            return Ok(result);
        }

        // Get the capability
        let capabilities = self.registry.get_capabilities(context.requester).await;

        if let Some(caps) = capabilities
            && let Some(cap) = caps.into_iter().find(|c| {
                crate::okapi_capability::has_operation(c, context.operation)
                    && !crate::okapi_capability::is_expired(c)
            })
        {
            // Verify visibility
            let cap_visibility = cap
                .get_caveat_data("visibility")
                .and_then(|v| hkask_types::Visibility::parse_str(v))
                .unwrap_or(hkask_types::Visibility::Private);

            if cap_visibility != context.required_visibility {
                warn!(
                    "OCAP denied: visibility mismatch. Required={:?}, Capability={:?}",
                    context.required_visibility, cap_visibility
                );

                return Ok(OcapEnforcementResult {
                    granted: false,
                    requester: context.requester,
                    operation: context.operation,
                    capability: None,
                    error: Some(format!(
                        "Visibility mismatch: required {:?}, capability has {:?}",
                        context.required_visibility, cap_visibility
                    )),
                });
            }

            info!(
                "OCAP granted: requester={}, operation={:?}",
                context.requester, context.operation
            );

            return Ok(OcapEnforcementResult {
                granted: true,
                requester: context.requester,
                operation: context.operation,
                capability: Some(cap),
                error: None,
            });
        }

        Ok(OcapEnforcementResult {
            granted: false,
            requester: context.requester,
            operation: context.operation,
            capability: None,
            error: Some("Capability not found".to_string()),
        })
    }

    /// Record OCAP enforcement metric
    fn record_ocap_metric(&self, result: &OcapEnforcementResult) {
        if let Some(metrics) = &self.metrics {
            metrics.record_ocap_event(
                result.granted,
                &result.requester.to_string(),
                &format!("{:?}", result.operation),
                result.error.as_deref(),
            );
            info!(
                target: "cns",
                granted = result.granted,
                "OCAP enforcement event recorded"
            );
        }
    }
    /// Authorize Okapi generate operation
    pub async fn authorize_generate(
        &self,
        requester: WebID,
    ) -> Result<OcapEnforcementResult, OkapiCapabilityError> {
        let context = OcapContext::new(requester, OkapiOperation::Generate);
        self.enforce(context).await
    }

    /// Authorize Okapi chat operation
    pub async fn authorize_chat(
        &self,
        requester: WebID,
    ) -> Result<OcapEnforcementResult, OkapiCapabilityError> {
        let context = OcapContext::new(requester, OkapiOperation::Chat);
        self.enforce(context).await
    }

    /// Authorize Okapi embed operation
    pub async fn authorize_embed(
        &self,
        requester: WebID,
    ) -> Result<OcapEnforcementResult, OkapiCapabilityError> {
        let context = OcapContext::new(requester, OkapiOperation::Embed);
        self.enforce(context).await
    }

    /// Authorize metrics read operation
    pub async fn authorize_read_metrics(
        &self,
        requester: WebID,
    ) -> Result<OcapEnforcementResult, OkapiCapabilityError> {
        let context = OcapContext::new(requester, OkapiOperation::ReadMetrics);
        self.enforce(context).await
    }
}

/// Helper function to enforce OCAP at Okapi boundary
pub async fn enforce_okapi_ocap(
    enforcer: Arc<OcapEnforcer>,
    requester: WebID,
    operation: OkapiOperation,
) -> Result<CapabilityToken, OkapiCapabilityError> {
    let context = OcapContext::new(requester, operation);
    let result = enforcer.enforce(context).await?;

    if result.granted {
        result.capability.ok_or(OkapiCapabilityError::Unauthorized {
            requested: operation.to_string(),
            granted: vec![],
        })
    } else {
        Err(OkapiCapabilityError::Unauthorized {
            requested: operation.to_string(),
            granted: vec![],
        })
    }
}
