//! hKask MCP GML — Allosteric Thinking with MWC model and OCAP enforcement

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, SignatureError, SigningKey, VerifyingKey};
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum GmlError {
    #[error("Invalid MWC parameters: {0}")]
    InvalidMwcParameters(String),
    #[error("Capability validation failed: {0}")]
    CapabilityDenied(String),
    #[error("Signature verification failed: {0}")]
    SignatureError(#[from] SignatureError),
    #[error("Keystore error: {0}")]
    KeystoreError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Hex decoding error: {0}")]
    HexError(#[from] hex::FromHexError),
}

// ============================================================================
// MWC Model Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MwcParameters {
    /// Allosteric constant (L = [T0]/[R0])
    pub l: f64,
    /// Selectivity factor (c = KR/KT)
    pub c: f64,
    /// Number of binding sites (cooperativity)
    pub n: u32,
    /// Reduced concentration (α = [S]/KR)
    pub alpha: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MwcState {
    /// Fraction in R-state: R̄ = (1+α)ⁿ/((1+α)ⁿ + L·(1+cα)ⁿ)
    pub r_bar: f64,
    /// Hill coefficient at current α
    pub n_h: f64,
    /// Free energy difference: ΔG = -RT·ln(R̄/(1-R̄))
    pub delta_g: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Effector {
    pub name: String,
    pub concentration: f64,
    pub effect_type: String,
    pub shape: String,
    pub affinity_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AllostericPort {
    pub name: String,
    pub effector_shape: String,
    pub affinity_c: f64,
    pub bound_effector: Option<Effector>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Concept {
    pub id: Option<String>,
    pub name: String,
    pub t_state: StateDescription,
    pub r_state: StateDescription,
    pub l: f64,
    pub ports: Vec<AllostericPort>,
    pub current_alpha: f64,
    pub current_r_bar: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StateDescription {
    pub description: String,
    pub energy: f64,
}

// ============================================================================
// Capability Token Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityToken {
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub operations: Vec<String>,
    pub scope: Option<Vec<String>>,
    pub effector_budget: Option<f64>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenVerification {
    pub valid: bool,
    pub token_id: String,
    pub subject: String,
    pub operations: Vec<String>,
    pub error: Option<String>,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComputeEquilibriumRequest {
    pub concept: Concept,
    pub effectors: Option<Vec<Effector>>,
    pub capability: Option<CapabilityToken>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BindEffectorRequest {
    pub concept: Concept,
    pub effector: Effector,
    pub port_index: usize,
    pub capability: Option<CapabilityToken>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateCapabilityRequest {
    pub issuer: String,
    pub subject: String,
    pub operations: Vec<String>,
    pub scope: Option<Vec<String>>,
    pub effector_budget: Option<f64>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyCapabilityRequest {
    pub token: CapabilityToken,
    pub operation: String,
    pub scope: Option<String>,
}

// ============================================================================
// MWC Computation Engine
// ============================================================================

#[derive(Debug, Default, Clone)]
pub struct MwcEngine;

impl MwcEngine {
    pub fn compute_r_bar(l: f64, c: f64, n: u32, alpha: f64) -> Result<f64, GmlError> {
        if l <= 0.0 {
            return Err(GmlError::InvalidMwcParameters("L must be > 0".into()));
        }
        if c <= 0.0 {
            return Err(GmlError::InvalidMwcParameters("c must be > 0".into()));
        }

        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;

        let numerator = one_plus_alpha.powi(n as i32);
        let denominator = numerator + l * one_plus_c_alpha.powi(n as i32);

        if denominator == 0.0 {
            return Err(GmlError::InvalidMwcParameters("Denominator is zero".into()));
        }

        Ok(numerator / denominator)
    }

    pub fn compute_hill(l: f64, c: f64, n: u32, alpha: f64, _r_bar: f64) -> f64 {
        if alpha == 0.0 || c == 1.0 {
            return 0.0;
        }

        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;

        let numerator = (n as f64)
            * one_plus_alpha.powi(n as i32)
            * l
            * one_plus_c_alpha.powi(n as i32)
            * (c - 1.0)
            * alpha;

        let denominator =
            (one_plus_alpha.powi(n as i32) + l * one_plus_c_alpha.powi(n as i32)).powi(2);

        if denominator == 0.0 {
            return 0.0;
        }

        let hill = numerator / denominator;
        hill.abs()
    }

    pub fn compute_delta_g(r_bar: f64, temperature: f64) -> f64 {
        const R: f64 = 8.314;

        if r_bar <= 0.0 || r_bar >= 1.0 {
            return 0.0;
        }

        let ratio = r_bar / (1.0 - r_bar);
        -R * temperature * ratio.ln()
    }

    pub fn apply_effectors(
        concept: &Concept,
        effectors: &[Effector],
    ) -> Result<(f64, f64, f64), GmlError> {
        let n = concept.ports.len() as u32;
        if n == 0 {
            return Err(GmlError::InvalidInput("No allosteric ports".into()));
        }

        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        let old_alpha = concept.current_alpha;
        let new_alpha = old_alpha + effectors.iter().map(|e| e.concentration).sum::<f64>();

        let old_r_bar = Self::compute_r_bar(concept.l, avg_c, n, old_alpha)?;
        let new_r_bar = Self::compute_r_bar(concept.l, avg_c, n, new_alpha)?;

        let _old_hill = Self::compute_hill(concept.l, avg_c, n, old_alpha, old_r_bar);
        let new_hill = Self::compute_hill(concept.l, avg_c, n, new_alpha, new_r_bar);

        Ok((new_r_bar, new_hill, new_alpha))
    }
}

// ============================================================================
// Capability Token Manager
// ============================================================================

#[derive(Debug)]
pub struct CapabilityManager {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl CapabilityManager {
    pub fn new() -> Result<Self, GmlError> {
        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    fn generate_token_id(issuer: &str, subject: &str, issued_at: DateTime<Utc>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(issuer.as_bytes());
        hasher.update(subject.as_bytes());
        hasher.update(issued_at.to_rfc3339().as_bytes());
        let hash = hasher.finalize();
        format!("gml_{}", hex::encode(&hash[..8]))
    }

    fn sign_token(&self, token_data: &str) -> Result<String, GmlError> {
        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        use ed25519_dalek::Signer;
        let signature = self.signing_key.sign(&message_hash);
        Ok(hex::encode(signature.to_bytes()))
    }

    fn verify_signature(&self, token_data: &str, signature_hex: &str) -> Result<bool, GmlError> {
        let signature_bytes = hex::decode(signature_hex)?;
        let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());

        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        use ed25519_dalek::Verifier;
        match self.verifying_key.verify(&message_hash, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn create_capability(
        &self,
        request: CreateCapabilityRequest,
    ) -> Result<CapabilityToken, GmlError> {
        let now = Utc::now();
        let expires_at = request
            .expires_in_seconds
            .map(|secs| now + chrono::Duration::seconds(secs));

        let token_id = Self::generate_token_id(&request.issuer, &request.subject, now);

        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token_id,
            request.issuer,
            request.subject,
            request.operations.join(","),
            now.to_rfc3339(),
            expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        );

        let signature = self.sign_token(&token_data)?;

        Ok(CapabilityToken {
            id: token_id,
            issuer: request.issuer,
            subject: request.subject,
            operations: request.operations,
            scope: request.scope,
            effector_budget: request.effector_budget,
            issued_at: now,
            expires_at,
            signature,
        })
    }

    pub fn verify_capability(
        &self,
        request: VerifyCapabilityRequest,
    ) -> Result<TokenVerification, GmlError> {
        let token = &request.token;

        if let Some(expires) = token.expires_at {
            if Utc::now() > expires {
                return Ok(TokenVerification {
                    valid: false,
                    token_id: token.id.clone(),
                    subject: token.subject.clone(),
                    operations: token.operations.clone(),
                    error: Some("Token expired".into()),
                });
            }
        }

        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token.id,
            token.issuer,
            token.subject,
            token.operations.join(","),
            token.issued_at.to_rfc3339(),
            token
                .expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        );

        let signature_valid = self.verify_signature(&token_data, &token.signature)?;

        if !signature_valid {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some("Invalid signature".into()),
            });
        }

        if !token.operations.contains(&request.operation) {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some(format!("Operation '{}' not allowed", request.operation)),
            });
        }

        if let Some(scope) = request.scope {
            if let Some(token_scope) = &token.scope {
                if !token_scope.contains(&scope) {
                    return Ok(TokenVerification {
                        valid: false,
                        token_id: token.id.clone(),
                        subject: token.subject.clone(),
                        operations: token.operations.clone(),
                        error: Some(format!("Scope '{}' not allowed", scope)),
                    });
                }
            }
        }

        Ok(TokenVerification {
            valid: true,
            token_id: token.id.clone(),
            subject: token.subject.clone(),
            operations: token.operations.clone(),
            error: None,
        })
    }

    pub fn check_effector_budget(
        &self,
        token: &CapabilityToken,
        concentration: f64,
    ) -> Result<bool, GmlError> {
        if let Some(budget) = token.effector_budget {
            Ok(concentration <= budget)
        } else {
            Ok(true)
        }
    }
}

// ============================================================================
// GML MCP Server
// ============================================================================

pub struct GmlServer {
    capability_manager: Arc<RwLock<Option<CapabilityManager>>>,
    cns_emitter: SpanEmitter,
}

impl GmlServer {
    pub fn new() -> Self {
        Self {
            capability_manager: Arc::new(RwLock::new(None)),
            cns_emitter: SpanEmitter::new(WebID::new()),
        }
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
                        return format!(
                            r#"{{"error":"Capability denied","reason":"{}"}}"#,
                            result.error.unwrap_or_default()
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
                        return format!(
                            r#"{{"error":"Capability verification failed","reason":"{}"}}"#,
                            e
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
                format!(
                    r#"{{"success":true,"r_bar":{},"n_h":{},"alpha":{},"delta_g":{}}}"#,
                    r_bar, n_h, alpha, delta_g
                )
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "compute_equilibrium.error",
                    serde_json::json!({
                        "reason": "computation_failed",
                        "error": e.to_string()
                    }),
                );
                format!(r#"{{"success":false,"error":"{}"}}"#, e)
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
                        return format!(
                            r#"{{"success":false,"error":"Capability denied","reason":"{}"}}"#,
                            result.error.unwrap_or_default()
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
                        return format!(
                            r#"{{"success":false,"error":"Capability verification failed","reason":"{}"}}"#,
                            e
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
                        return format!(
                            r#"{{"success":false,"error":"Effector budget exceeded","concentration":{},"budget":{}}}"#,
                            effector.concentration,
                            token.effector_budget.unwrap()
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
                        return format!(r#"{{"success":false,"error":"{}"}}"#, e);
                    }
                }
            } else {
                self.cns_emitter.emit_prompt(
                    "bind_effector.error",
                    serde_json::json!({
                        "reason": "capability_missing"
                    }),
                );
                return r#"{"success":false,"error":"Capability token required"}"#.to_string();
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
            return format!(
                r#"{{"success":false,"error":"Invalid port index","max":{}}}"#,
                concept.ports.len() - 1
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
            return format!(
                r#"{{"success":false,"error":"Shape mismatch","port_shape":"{}","effector_shape":"{}"}}"#,
                port.effector_shape, effector.shape
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
                format!(
                    r#"{{"success":true,"bound":true,"port":"{}","effector":"{}","r_bar":{},"n_h":{},"alpha":{}}}"#,
                    port.name, effector.name, r_bar, n_h, alpha
                )
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "bind_effector.error",
                    serde_json::json!({
                        "reason": "computation_failed",
                        "error": e.to_string()
                    }),
                );
                format!(r#"{{"success":false,"error":"{}"}}"#, e)
            }
        }
    }

    #[tool(description = "Create a capability token")]
    async fn gml_create_capability(
        &self,
        Parameters(request): Parameters<CreateCapabilityRequest>,
    ) -> String {
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
                format!(
                    r#"{{"success":true,"token_id":"{}","issuer":"{}","subject":"{}","operations":{},"expires_at":{}}}"#,
                    token.id,
                    token.issuer,
                    token.subject,
                    serde_json::to_string(&token.operations).unwrap(),
                    token
                        .expires_at
                        .map(|dt| format!("\"{}\"", dt.to_rfc3339()))
                        .unwrap_or_else(|| "null".to_string())
                )
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "create_capability.error",
                    serde_json::json!({
                        "reason": "creation_failed",
                        "error": e.to_string()
                    }),
                );
                format!(r#"{{"success":false,"error":"{}"}}"#, e)
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
                format!(
                    r#"{{"valid":{},"token_id":"{}","subject":"{}","operations":{},"error":{}}}"#,
                    verification.valid,
                    verification.token_id,
                    verification.subject,
                    serde_json::to_string(&verification.operations).unwrap(),
                    verification
                        .error
                        .map(|e| format!("\"{}\"", e))
                        .unwrap_or_else(|| "null".to_string())
                )
            }
            Err(e) => {
                self.cns_emitter.emit_prompt(
                    "verify_capability.error",
                    serde_json::json!({
                        "reason": "verification_failed",
                        "error": e.to_string()
                    }),
                );
                format!(r#"{{"valid":false,"error":"{}"}}"#, e)
            }
        }
    }

    #[tool(description = "Compute Hill coefficient for a concept")]
    async fn gml_compute_hill(&self, Parameters(concept): Parameters<Concept>) -> String {
        let n = concept.ports.len() as u32;
        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        match MwcEngine::compute_r_bar(concept.l, avg_c, n, concept.current_alpha) {
            Ok(r_bar) => {
                let n_h =
                    MwcEngine::compute_hill(concept.l, avg_c, n, concept.current_alpha, r_bar);
                format!(
                    r#"{{"success":true,"r_bar":{},"n_h":{},"l":{},"c_avg":{},"alpha":{}}}"#,
                    r_bar, n_h, concept.l, avg_c, concept.current_alpha
                )
            }
            Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e),
        }
    }

    #[tool(description = "Assess cooperativity of a concept")]
    async fn gml_assess_cooperativity(&self, Parameters(concept): Parameters<Concept>) -> String {
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

        format!(
            r#"{{"success":true,"cooperativity":"{}","sensitivity":"{}","ports":{},"c_avg":{},"l":{}}}"#,
            cooperativity_level, sensitivity, n, avg_c, concept.l
        )
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = GmlServer::new();
    tracing::info!("hkask-mcp-gml started (v{})", SERVER_VERSION);
    let service = server.serve(rmcp::transport::stdio());
    service.await?;

    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_r_bar_l_100_alpha_0() {
        let r_bar = MwcEngine::compute_r_bar(100.0, 0.1, 4, 0.0).unwrap();
        assert!((r_bar - 0.01).abs() < 0.001, "Expected R̄ ≈ 0.01, got {}", r_bar);
    }

    #[test]
    fn test_compute_r_bar_l_1_alpha_0() {
        let r_bar = MwcEngine::compute_r_bar(1.0, 0.1, 4, 0.0).unwrap();
        assert!((r_bar - 0.5).abs() < 0.001, "Expected R̄ = 0.5, got {}", r_bar);
    }

    #[test]
    fn test_compute_r_bar_invalid_l() {
        assert!(MwcEngine::compute_r_bar(0.0, 0.1, 4, 1.0).is_err());
        assert!(MwcEngine::compute_r_bar(-1.0, 0.1, 4, 1.0).is_err());
    }

    #[test]
    fn test_compute_delta_g() {
        let delta_g = MwcEngine::compute_delta_g(0.5, 298.0);
        assert!((delta_g - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_create_capability_token() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: Some(50.0),
            expires_in_seconds: Some(86400),
        };
        let token = manager.create_capability(request).unwrap();
        assert_eq!(token.issuer, "did:webid:curator");
        assert!(!token.signature.is_empty());
    }

    #[test]
    fn test_verify_capability_valid() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: None,
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        let verification = manager.verify_capability(VerifyCapabilityRequest {
            token: token.clone(),
            operation: "bind_effector".to_string(),
            scope: None,
        }).unwrap();
        assert!(verification.valid);
    }

    #[test]
    fn test_verify_capability_wrong_operation() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: None,
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        let verification = manager.verify_capability(VerifyCapabilityRequest {
            token,
            operation: "compute_equilibrium".to_string(),
            scope: None,
        }).unwrap();
        assert!(!verification.valid);
    }

    #[test]
    fn test_check_effector_budget() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec![],
            scope: None,
            effector_budget: Some(50.0),
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        assert!(manager.check_effector_budget(&token, 30.0).unwrap());
        assert!(!manager.check_effector_budget(&token, 100.0).unwrap());
    }
}
