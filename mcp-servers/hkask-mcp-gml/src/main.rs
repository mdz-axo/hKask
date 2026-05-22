//! hKask MCP GML — Allosteric Thinking with MWC model and OCAP enforcement

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, SignatureError};
use hkask_keystore::KeystoreClient;
use rand::rngs::OsRng;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;

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
    pub effect_type: String, // "Activator", "Inhibitor", "Neutral"
    pub shape: String, // Matches port.effector_shape
    pub affinity_c: Option<f64>, // Optional override for port affinity
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
    /// Unique token ID
    pub id: String,
    /// Token issuer (WebID)
    pub issuer: String,
    /// Token subject (holder WebID)
    pub subject: String,
    /// Allowed operations
    pub operations: Vec<String>,
    /// Scope restrictions (e.g., concept IDs)
    pub scope: Option<Vec<String>>,
    /// Effector concentration budget
    pub effector_budget: Option<f64>,
    /// Issuance timestamp
    pub issued_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Ed25519 signature (hex-encoded)
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

pub struct MwcEngine;

impl MwcEngine {
    /// Compute R̄ = (1+α)ⁿ/((1+α)ⁿ + L·(1+cα)ⁿ)
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

    /// Compute Hill coefficient: n_H = (n·(1+α)ⁿ·L·(1+cα)ⁿ·(c-1)·α) / ((1+α)ⁿ + L·(1+cα)ⁿ)²
    pub fn compute_hill(l: f64, c: f64, n: u32, alpha: f64, r_bar: f64) -> f64 {
        if alpha == 0.0 || c == 1.0 {
            return 0.0;
        }

        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;

        let numerator = (n as f64) * one_plus_alpha.powi(n as i32)
            * l * one_plus_c_alpha.powi(n as i32)
            * (c - 1.0) * alpha;

        let denominator = (one_plus_alpha.powi(n as i32) + l * one_plus_c_alpha.powi(n as i32)).powi(2);

        if denominator == 0.0 {
            return 0.0;
        }

        // Hill coefficient from derivative: n_H = d(ln(R̄/(1-R̄)))/d(ln(α))
        // Simplified: n_H = n · (c-1) · α · (1+α)ⁿ⁻¹ · (1+cα)ⁿ⁻¹ · L / ((1+α)ⁿ + L·(1+cα)ⁿ)²
        let hill = numerator / denominator;
        hill.abs() // Hill coefficient is always positive
    }

    /// Compute free energy: ΔG = -RT·ln(R̄/(1-R̄))
    pub fn compute_delta_g(r_bar: f64, temperature: f64) -> f64 {
        const R: f64 = 8.314; // Gas constant J/(mol·K)

        if r_bar <= 0.0 || r_bar >= 1.0 {
            return 0.0;
        }

        let ratio = r_bar / (1.0 - r_bar);
        -R * temperature * ratio.ln()
    }

    /// Compute equilibrium shift from effector binding
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

        let old_hill = Self::compute_hill(concept.l, avg_c, n, old_alpha, old_r_bar);
        let new_hill = Self::compute_hill(concept.l, avg_c, n, new_alpha, new_r_bar);

        Ok((new_r_bar, new_hill, new_alpha))
    }
}

// ============================================================================
// Capability Token Manager
// ============================================================================

pub struct CapabilityManager {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    keystore: Arc<RwLock<KeystoreClient>>,
}

impl CapabilityManager {
    pub fn new(keystore: Arc<RwLock<KeystoreClient>>) -> Result<Self, GmlError> {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok(Self {
            signing_key,
            verifying_key,
            keystore,
        })
    }

    /// Generate token ID from content hash
    fn generate_token_id(issuer: &str, subject: &str, issued_at: DateTime<Utc>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(issuer.as_bytes());
        hasher.update(subject.as_bytes());
        hasher.update(issued_at.to_rfc3339().as_bytes());
        let hash = hasher.finalize();
        format!("gml_{}", hex::encode(&hash[..8]))
    }

    /// Sign capability token with Ed25519
    fn sign_token(&self, token_data: &str) -> Result<String, GmlError> {
        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        let signature = self.signing_key.sign_prehash(&message_hash)?;
        Ok(hex::encode(signature.to_bytes()))
    }

    /// Verify Ed25519 signature
    fn verify_signature(&self, token_data: &str, signature_hex: &str) -> Result<bool, GmlError> {
        let signature_bytes = hex::decode(signature_hex)?;
        let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());

        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        match self.verifying_key.verify_prehash(&message_hash, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Create new capability token
    pub fn create_capability(
        &self,
        request: CreateCapabilityRequest,
    ) -> Result<CapabilityToken, GmlError> {
        let now = Utc::now();
        let expires_at = request.expires_in_seconds.map(|secs| now + chrono::Duration::seconds(secs));

        let token_id = Self::generate_token_id(&request.issuer, &request.subject, now);

        // Create unsigned token data
        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token_id,
            request.issuer,
            request.subject,
            request.operations.join(","),
            now.to_rfc3339(),
            expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| "never".to_string())
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

    /// Verify capability token
    pub fn verify_capability(
        &self,
        request: VerifyCapabilityRequest,
    ) -> Result<TokenVerification, GmlError> {
        let token = &request.token;

        // Check expiration
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

        // Verify signature
        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token.id,
            token.issuer,
            token.subject,
            token.operations.join(","),
            token.issued_at.to_rfc3339(),
            token.expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| "never".to_string())
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

        // Check operation permission
        if !token.operations.contains(&request.operation) {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some(format!("Operation '{}' not allowed", request.operation)),
            });
        }

        // Check scope
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

    /// Check effector budget
    pub fn check_effector_budget(
        &self,
        token: &CapabilityToken,
        concentration: f64,
    ) -> Result<bool, GmlError> {
        if let Some(budget) = token.effector_budget {
            Ok(concentration <= budget)
        } else {
            Ok(true) // No budget constraint
        }
    }
}

// ============================================================================
// GML MCP Server
// ============================================================================

#[derive(Debug, Default)]
pub struct GmlServer {
    mwc_engine: MwcEngine,
    capability_manager: Arc<RwLock<Option<CapabilityManager>>>,
    keystore: Arc<RwLock<KeystoreClient>>,
}

impl GmlServer {
    pub fn new() -> Self {
        Self {
            mwc_engine: MwcEngine,
            capability_manager: Arc::new(RwLock::new(None)),
            keystore: Arc::new(RwLock::new(KeystoreClient::new())),
        }
    }

    async fn init_capability_manager(&self) -> Result<(), GmlError> {
        let mut manager = self.capability_manager.write().await;
        if manager.is_none() {
            let keystore_clone = Arc::clone(&self.keystore);
            let cap_manager = CapabilityManager::new(keystore_clone)?;
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
        // Verify capability if provided
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
                        return format!(
                            r#"{{"error":"Capability denied","reason":"{}"}}"#,
                            result.error.unwrap_or_default()
                        );
                    }
                    Err(e) => {
                        return format!(r#"{{"error":"Capability verification failed","reason":"{}"}}"#, e);
                    }
                    _ => {}
                }
            }
        }

        // Compute equilibrium
        let result = if let Some(effectors) = effectors {
            MwcEngine::apply_effectors(&concept, &effectors)
        } else {
            let n = concept.ports.len() as u32;
            let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);
            MwcEngine::compute_r_bar(concept.l, avg_c, n, concept.current_alpha).map(|r_bar| (r_bar, 0.0, concept.current_alpha))
        };

        match result {
            Ok((r_bar, n_h, alpha)) => {
                let delta_g = MwcEngine::compute_delta_g(r_bar, 298.0); // 298K = 25°C
                format!(
                    r#"{{"success":true,"r_bar":{},"n_h":{},"alpha":{},"delta_g":{}}}"#,
                    r_bar, n_h, alpha, delta_g
                )
            }
            Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e),
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
        // Verify capability
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
                        return format!(
                            r#"{{"success":false,"error":"Capability denied","reason":"{}"}}"#,
                            result.error.unwrap_or_default()
                        );
                    }
                    Err(e) => {
                        return format!(r#"{{"success":false,"error":"Capability verification failed","reason":"{}"}}"#, e);
                    }
                    _ => {}
                }

                // Check effector budget
                match mgr.check_effector_budget(token, effector.concentration) {
                    Ok(true) => {}
                    Ok(false) => {
                        return format!(
                            r#"{{"success":false,"error":"Effector budget exceeded","concentration":{},"budget":{}}}"#,
                            effector.concentration, token.effector_budget.unwrap()
                        );
                    }
                    Err(e) => {
                        return format!(r#"{{"success":false,"error":"{}"}}"#, e);
                    }
                }
            } else {
                return r#"{"success":false,"error":"Capability token required"}"#.to_string();
            }
        }

        // Validate port index
        if port_index >= concept.ports.len() {
            return format!(
                r#"{{"success":false,"error":"Invalid port index","max":{}}}"#,
                concept.ports.len() - 1
            );
        }

        // Check shape compatibility
        let port = &concept.ports[port_index];
        if port.effector_shape != effector.shape {
            return format!(
                r#"{{"success":false,"error":"Shape mismatch","port_shape":"{}","effector_shape":"{}"}}"#,
                port.effector_shape, effector.shape
            );
        }

        // Compute binding effect
        let effectors = vec![effector.clone()];
        let result = MwcEngine::apply_effectors(&concept, &effectors);

        match result {
            Ok((r_bar, n_h, alpha)) => {
                format!(
                    r#"{{"success":true,"bound":true,"port":"{}","effector":"{}","r_bar":{},"n_h":{},"alpha":{}}}"#,
                    port.name, effector.name, r_bar, n_h, alpha
                )
            }
            Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e),
        }
    }

    #[tool(description = "Create a capability token")]
    async fn gml_create_capability(
        &self,
        Parameters(request): Parameters<CreateCapabilityRequest>,
    ) -> String {
        self.init_capability_manager().await.unwrap();
        let manager = self.capability_manager.read().await;

        match manager.as_ref().unwrap().create_capability(request) {
            Ok(token) => {
                format!(
                    r#"{{"success":true,"token_id":"{}","issuer":"{}","subject":"{}","operations":{},"expires_at":{}}}"#,
                    token.id,
                    token.issuer,
                    token.subject,
                    serde_json::to_string(&token.operations).unwrap(),
                    token.expires_at.map(|dt| format!("\"{}\"", dt.to_rfc3339())).unwrap_or_else(|| "null".to_string())
                )
            }
            Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e),
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
        self.init_capability_manager().await.unwrap();
        let manager = self.capability_manager.read().await;

        match manager.as_ref().unwrap().verify_capability(VerifyCapabilityRequest {
            token,
            operation,
            scope,
        }) {
            Ok(verification) => {
                format!(
                    r#"{{"valid":{},"token_id":"{}","subject":"{}","operations":{},"error":{}}}"#,
                    verification.valid,
                    verification.token_id,
                    verification.subject,
                    serde_json::to_string(&verification.operations).unwrap(),
                    verification.error.map(|e| format!("\"{}\"", e)).unwrap_or_else(|| "null".to_string())
                )
            }
            Err(e) => format!(r#"{{"valid":false,"error":"{}"}}"#, e),
        }
    }

    #[tool(description = "Compute Hill coefficient for a concept")]
    async fn gml_compute_hill(
        &self,
        Parameters(concept): Parameters<Concept>,
    ) -> String {
        let n = concept.ports.len() as u32;
        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        match MwcEngine::compute_r_bar(concept.l, avg_c, n, concept.current_alpha) {
            Ok(r_bar) => {
                let n_h = MwcEngine::compute_hill(concept.l, avg_c, n, concept.current_alpha, r_bar);
                format!(
                    r#"{{"success":true,"r_bar":{},"n_h":{},"l":{},"c_avg":{},"alpha":{}}}"#,
                    r_bar, n_h, concept.l, avg_c, concept.current_alpha
                )
            }
            Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e),
        }
    }

    #[tool(description = "Assess cooperativity of a concept")]
    async fn gml_assess_cooperativity(
        &self,
        Parameters(concept): Parameters<Concept>,
    ) -> String {
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
    let service = server.serve(stdio());

    tracing::info!("hkask-mcp-gml started (v{})", SERVER_VERSION);
    service.await?;

    Ok(())
}
