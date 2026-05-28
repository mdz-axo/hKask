//! GML type definitions — error types, MWC model types, and request/response types

use chrono::{DateTime, Utc};
use ed25519_dalek::SignatureError;
use hex::FromHexError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    HexError(#[from] FromHexError),
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
