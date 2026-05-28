//! GML MCP Server — tool handlers for allosteric thinking operations

use hkask_cns::spans::SpanEmitter;
use hkask_mcp::server::{McpToolError, McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::capability::CapabilityManager;
use crate::engine::MwcEngine;
use crate::types::*;

pub struct GmlServer {
    capability_manager: Arc<RwLock<Option<CapabilityManager>>>,
    cns_emitter: SpanEmitter,
    webid: WebID,
}

impl GmlServer {
    pub fn new(webid: WebID) -> anyhow::Result<Self> {
        Ok(Self {
            capability_manager: Arc::new(RwLock::new(None)),
            cns_emitter: SpanEmitter::new(webid.clone()),
            webid,
        })
    }

    async fn init_capability_manager(&self) -> Result<(), GmlError> {
        let mut manager = self.capability_manager.write().await;
        if manager.is_none() {
            let cap_manager = CapabilityManager::new()?;
            *manager = Some(cap_manager);
        }
        Ok(())
    }
}

#[tool_router(server_handler)]
impl GmlServer {
    #[tool(description = "Compute MWC equilibrium for a concept")]
    async fn gml_compute_equilibrium(
        &self,
        Parameters(ComputeEquilibriumRequest {
            concept,
            effectors,
            capability,
        }): Parameters<ComputeEquilibriumRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gml_compute_equilibrium", &self.webid);

        self.cns_emitter.emit_prompt(
            "compute_equilibrium.start",
            serde_json::json!({
                "concept": concept.name,
                "effectors_count": effectors.as_ref().map(|e| e.len()).unwrap_or(0)
            }),
        );

        if let Some(token) = &capability {
            self.init_capability_manager().await.unwrap();
            let manager = self.capability_manager.read().await;
            if let Some(mgr) = manager.as_ref() {
                let verification = mgr.verify_capability(VerifyCapabilityRequest {
                    token: token.clone(),
                    operation: "compute_equilibrium".into(),
                    scope: concept.id.clone(),
                });

                match verification {
                    Ok(result) if !result.valid => {
                        self.cns_emitter.emit_prompt(
                            "compute_equilibrium.error",
                            serde_json::json!({
                                "reason": "capability_denied",
                                "error": result.error
                            }),
                        );
                        return span.error(
                            McpErrorKind::PermissionDenied,
                            McpToolError::permission_denied(format!(
                                "Capability denied: {}",
                                result.error.unwrap_or_default()
                            ))
                            .to_json_string(),
                        );
                    }
                    Err(e) => {
                        self.cns_emitter.emit_prompt(
                            "compute_equilibrium.error",
                            serde_json::json!({
                                "reason": "verification_failed",
                                "error": e.to_string()
                            }),
                        );
                        return span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(format!(
                                "Capability verification failed: {}",
                                e
                            ))
                            .to_json_string(),
                        );
                    }
                    _ => {}
                }
            }
        }

        let result = if let Some(effectors) = effectors {
            MwcEngine::apply_effectors(&concept, &effectors)
        } else {
            let n = concept.ports.len() as u32;
            let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);
            MwcEngine::compute_r_bar(concept.l, avg_c, n, concept.current_alpha)
                .map(|r_bar| (r_bar, 0.0, concept.current_alpha))
        };

        match result {
            Ok((r_bar, n_h, alpha)) => {
                let delta_g = MwcEngine::compute_delta_g(r_bar, 298.0);
                self.cns_emitter.emit_prompt(
                    "compute_equilibrium.success",
                    serde_json::json!({
                        "r_bar": r_bar,
                        "n_h": n_h,
                        "delta_g": delta_g
                    }),
                );
                span.ok(McpToolOutput::new(json!({
                    "success": true,
                    "r_bar": r_bar,
                    "n_h": n_h,
                    "alpha": alpha,
                    "delta_g": delta_g
                }))
                .to_json_string())
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "compute_equilibrium.error",
                    serde_json::json!({
                        "reason": "computation_failed",
                        "error": e.to_string()
                    }),
                );
                span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e.to_string()).to_json_string(),
                )
            }
        }
    }

    #[tool(description = "Bind effector to concept port")]
    async fn gml_bind_effector(
        &self,
        Parameters(BindEffectorRequest {
            concept,
            effector,
            port_index,
            capability,
        }): Parameters<BindEffectorRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gml_bind_effector", &self.webid);

        self.cns_emitter.emit_prompt(
            "bind_effector.start",
            serde_json::json!({
                "concept": concept.name,
                "effector": effector.name,
                "port_index": port_index
            }),
        );

        self.init_capability_manager().await.unwrap();
        let manager = self.capability_manager.read().await;
        if let Some(mgr) = manager.as_ref() {
            if let Some(token) = &capability {
                let verification = mgr.verify_capability(VerifyCapabilityRequest {
                    token: token.clone(),
                    operation: "bind_effector".into(),
                    scope: concept.id.clone(),
                });

                match verification {
                    Ok(result) if !result.valid => {
                        self.cns_emitter.emit_prompt(
                            "bind_effector.error",
                            serde_json::json!({
                                "reason": "capability_denied",
                                "error": result.error
                            }),
                        );
                        return span.error(
                            McpErrorKind::PermissionDenied,
                            McpToolError::permission_denied(format!(
                                "Capability denied: {}",
                                result.error.unwrap_or_default()
                            ))
                            .to_json_string(),
                        );
                    }
                    Err(e) => {
                        self.cns_emitter.emit_prompt(
                            "bind_effector.error",
                            serde_json::json!({
                                "reason": "verification_failed",
                                "error": e.to_string()
                            }),
                        );
                        return span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(format!(
                                "Capability verification failed: {}",
                                e
                            ))
                            .to_json_string(),
                        );
                    }
                    _ => {}
                }

                match mgr.check_effector_budget(token, effector.concentration) {
                    Ok(true) => {}
                    Ok(false) => {
                        self.cns_emitter.emit_prompt(
                            "bind_effector.error",
                            serde_json::json!({
                                "reason": "budget_exceeded",
                                "concentration": effector.concentration,
                                "budget": token.effector_budget.unwrap()
                            }),
                        );
                        return span.error(
                            McpErrorKind::PermissionDenied,
                            McpToolError::permission_denied(format!(
                                "Effector budget exceeded: concentration {} exceeds budget {}",
                                effector.concentration,
                                token.effector_budget.unwrap()
                            ))
                            .to_json_string(),
                        );
                    }
                    Err(e) => {
                        self.cns_emitter.emit_prompt(
                            "bind_effector.error",
                            serde_json::json!({
                                "reason": "budget_check_failed",
                                "error": e.to_string()
                            }),
                        );
                        return span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(e.to_string()).to_json_string(),
                        );
                    }
                }
            } else {
                self.cns_emitter.emit_prompt(
                    "bind_effector.error",
                    serde_json::json!({
                        "reason": "capability_missing"
                    }),
                );
                return span.error(
                    McpErrorKind::PermissionDenied,
                    McpToolError::permission_denied("Capability token required").to_json_string(),
                );
            }
        }

        if port_index >= concept.ports.len() {
            self.cns_emitter.emit_prompt(
                "bind_effector.error",
                serde_json::json!({
                    "reason": "invalid_port_index",
                    "provided": port_index,
                    "max": concept.ports.len() - 1
                }),
            );
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Invalid port index: max {}",
                    concept.ports.len() - 1
                ))
                .to_json_string(),
            );
        }

        let port = &concept.ports[port_index];
        if port.effector_shape != effector.shape {
            self.cns_emitter.emit_prompt(
                "bind_effector.error",
                serde_json::json!({
                    "reason": "shape_mismatch",
                    "port_shape": port.effector_shape,
                    "effector_shape": effector.shape
                }),
            );
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Shape mismatch: port expects '{}' but effector has '{}'",
                    port.effector_shape, effector.shape
                ))
                .to_json_string(),
            );
        }

        let effectors = vec![effector.clone()];
        let result = MwcEngine::apply_effectors(&concept, &effectors);

        match result {
            Ok((r_bar, n_h, alpha)) => {
                self.cns_emitter.emit_prompt(
                    "bind_effector.success",
                    serde_json::json!({
                        "bound": true,
                        "port": port.name,
                        "effector": effector.name,
                        "r_bar": r_bar,
                        "n_h": n_h,
                        "alpha": alpha
                    }),
                );
                span.ok(McpToolOutput::new(json!({
                    "success": true,
                    "bound": true,
                    "port": port.name,
                    "effector": effector.name,
                    "r_bar": r_bar,
                    "n_h": n_h,
                    "alpha": alpha
                }))
                .to_json_string())
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "bind_effector.error",
                    serde_json::json!({
                        "reason": "computation_failed",
                        "error": e.to_string()
                    }),
                );
                span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e.to_string()).to_json_string(),
                )
            }
        }
    }

    #[tool(description = "Create a capability token")]
    async fn gml_create_capability(
        &self,
        Parameters(request): Parameters<CreateCapabilityRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gml_create_capability", &self.webid);

        if let Err(e) = validate_identifier("issuer", &request.issuer, 256) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("subject", &request.subject, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        self.cns_emitter.emit_prompt(
            "create_capability.start",
            serde_json::json!({
                "issuer": request.issuer,
                "subject": request.subject,
                "operations": request.operations
            }),
        );

        self.init_capability_manager().await.unwrap();
        let manager = self.capability_manager.read().await;

        match manager.as_ref().unwrap().create_capability(request) {
            Ok(token) => {
                self.cns_emitter.emit_prompt(
                    "create_capability.success",
                    serde_json::json!({
                        "token_id": token.id,
                        "expires_at": token.expires_at
                    }),
                );
                span.ok(McpToolOutput::new(json!({
                    "success": true,
                    "token_id": token.id,
                    "issuer": token.issuer,
                    "subject": token.subject,
                    "operations": token.operations,
                    "expires_at": token.expires_at
                }))
                .to_json_string())
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "create_capability.error",
                    serde_json::json!({
                        "reason": "creation_failed",
                        "error": e.to_string()
                    }),
                );
                span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(e.to_string()).to_json_string(),
                )
            }
        }
    }

    #[tool(description = "Verify a capability token")]
    async fn gml_verify_capability(
        &self,
        Parameters(VerifyCapabilityRequest {
            token,
            operation,
            scope,
        }): Parameters<VerifyCapabilityRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("gml_verify_capability", &self.webid);

        if let Err(e) = validate_identifier("token_id", &token.id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        self.cns_emitter.emit_prompt(
            "verify_capability.start",
            serde_json::json!({
                "token_id": token.id,
                "operation": operation,
                "scope": scope
            }),
        );

        self.init_capability_manager().await.unwrap();
        let manager = self.capability_manager.read().await;

        match manager
            .as_ref()
            .unwrap()
            .verify_capability(VerifyCapabilityRequest {
                token,
                operation,
                scope,
            }) {
            Ok(verification) => {
                self.cns_emitter.emit_prompt(
                    "verify_capability.outcome",
                    serde_json::json!({
                        "valid": verification.valid,
                        "token_id": verification.token_id,
                        "error": verification.error
                    }),
                );
                span.ok(McpToolOutput::new(json!({
                    "valid": verification.valid,
                    "token_id": verification.token_id,
                    "subject": verification.subject,
                    "operations": verification.operations,
                    "error": verification.error
                }))
                .to_json_string())
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "verify_capability.error",
                    serde_json::json!({
                        "reason": "verification_failed",
                        "error": e.to_string()
                    }),
                );
                span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(e.to_string()).to_json_string(),
                )
            }
        }
    }

    #[tool(description = "Compute Hill coefficient for a concept")]
    async fn gml_compute_hill(&self, Parameters(concept): Parameters<Concept>) -> String {
        let span = ToolSpanGuard::new("gml_compute_hill", &self.webid);

        let n = concept.ports.len() as u32;
        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        match MwcEngine::compute_r_bar(concept.l, avg_c, n, concept.current_alpha) {
            Ok(r_bar) => {
                let n_h =
                    MwcEngine::compute_hill(concept.l, avg_c, n, concept.current_alpha, r_bar);
                span.ok(McpToolOutput::new(json!({
                    "success": true,
                    "r_bar": r_bar,
                    "n_h": n_h,
                    "l": concept.l,
                    "c_avg": avg_c,
                    "alpha": concept.current_alpha
                }))
                .to_json_string())
            }
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(e.to_string()).to_json_string(),
            ),
        }
    }

    #[tool(description = "Assess cooperativity of a concept")]
    async fn gml_assess_cooperativity(&self, Parameters(concept): Parameters<Concept>) -> String {
        let span = ToolSpanGuard::new("gml_assess_cooperativity", &self.webid);

        let n = concept.ports.len();
        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        let cooperativity_level = if n == 1 {
            "non-cooperative"
        } else if n == 2 {
            "weakly cooperative"
        } else if n <= 4 {
            "moderately cooperative"
        } else {
            "highly cooperative"
        };

        let sensitivity = if avg_c < 0.1 {
            "high sensitivity (strong activator bias)"
        } else if avg_c > 10.0 {
            "low sensitivity (strong inhibitor bias)"
        } else {
            "moderate sensitivity"
        };

        span.ok(McpToolOutput::new(json!({
            "success": true,
            "cooperativity": cooperativity_level,
            "sensitivity": sensitivity,
            "ports": n,
            "c_avg": avg_c,
            "l": concept.l
        }))
        .to_json_string())
    }
}
