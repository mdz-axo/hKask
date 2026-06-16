//! API key authentication middleware — Ed25519 Bearer token verification.
//!
//! Authenticates requests using hKask-issued API keys (Ed25519 keypairs).
//! The private key IS the API key — presented as a hex-encoded Bearer token.
//!
//! # Flow
//! 1. Extract Bearer token from Authorization header
//! 2. Parse as Ed25519 private key (hex-encoded 32 bytes → 64 hex chars)
//! 3. Derive public key from private key
//! 4. Look up ApiKeyCapability by public key in WalletStore
//! 5. Verify: not revoked, not expired, spending limit not exceeded
//! 6. Attach WalletContext to request extensions
//!
//! # OCAP alignment (P4)
//! The API key IS a capability token. The middleware verifies the capability
//! and extracts its attenuation (spending limit). Downstream handlers use
//! the attached wallet context for gas→rJoule billing.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ed25519_dalek::SigningKey;
use hkask_services::WalletService;
use hkask_storage::WalletStore;
use hkask_types::WebID;
use hkask_types::wallet::{ApiKeyId, RJoule, WalletId};
use std::sync::Arc;

/// Wallet context attached to authenticated requests.
#[derive(Debug, Clone)]
pub struct WalletContext {
    pub wallet_id: WalletId,
    pub key_id: ApiKeyId,
    pub spending_limit_rj: RJoule,
    pub spent_rj: RJoule,
}

// SAFETY: all fields are Copy + Send + Sync (UUIDs and u64s)
unsafe impl Send for WalletContext {}
unsafe impl Sync for WalletContext {}

/// Middleware state for API key authentication.
#[derive(Clone)]
pub struct ApiKeyAuthService {
    wallet_store: Arc<WalletStore>,
    wallet_service: Arc<WalletService>,
    system_webid: WebID,
}

impl ApiKeyAuthService {
    /// Create a new API key auth service backed by a WalletStore and WalletService.
    pub fn new(
        wallet_store: Arc<WalletStore>,
        wallet_service: Arc<WalletService>,
        system_webid: WebID,
    ) -> Self {
        Self {
            wallet_store,
            wallet_service,
            system_webid,
        }
    }

    /// Authenticate a request using an Ed25519 API key Bearer token.
    fn authenticate(&self, request: &Request<Body>) -> Result<WalletContext, ApiKeyAuthError> {
        let header = request
            .headers()
            .get("Authorization")
            .ok_or(ApiKeyAuthError::MissingAuthorization)?;

        let header_str = header
            .to_str()
            .map_err(|_| ApiKeyAuthError::InvalidAuthorizationFormat)?;

        let token = header_str
            .strip_prefix("Bearer ")
            .ok_or(ApiKeyAuthError::InvalidAuthorizationFormat)?;

        // Parse hex-encoded Ed25519 private key (64 hex chars = 32 bytes)
        let private_key_bytes =
            hex::decode(token).map_err(|_| ApiKeyAuthError::InvalidKeyFormat)?;

        if private_key_bytes.len() != 32 {
            return Err(ApiKeyAuthError::InvalidKeyFormat);
        }

        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&private_key_bytes);

        let signing_key = SigningKey::from_bytes(&key_arr);
        let public_key_bytes = signing_key.verifying_key().to_bytes();

        // Look up the capability by public key.
        // The store query already filters `WHERE revoked_at IS NULL`,
        // so revoked keys are never returned.
        let capability = self
            .wallet_store
            .get_api_key_by_public_key(&public_key_bytes)
            .map_err(|_| ApiKeyAuthError::StoreError)?
            .ok_or(ApiKeyAuthError::UnknownApiKey)?;

        // Verify key is not expired
        if let Some(expiry) = capability.expiry
            && chrono::Utc::now() > expiry
        {
            return Err(ApiKeyAuthError::KeyExpired);
        }

        // Verify spending limit not exceeded
        if capability.spent_rj.as_u64() >= capability.spending_limit_rj.as_u64() {
            return Err(ApiKeyAuthError::SpendingLimitExceeded);
        }

        // Verify encumbrance: the key must have rJoules allocated
        let encumbrance = self
            .wallet_store
            .get_encumbrance(capability.key_id)
            .map_err(|_| ApiKeyAuthError::StoreError)?;

        match encumbrance {
            Some(ref enc) if enc.is_active() && enc.remaining_rj() > 0 => {
                // Key has allocated rJoules — proceed
            }
            Some(ref enc) if enc.is_active() => {
                // Encumbrance exists but is exhausted
                return Err(ApiKeyAuthError::PaymentRequired(
                    "API key encumbrance exhausted — allocate more rJoules".into(),
                ));
            }
            Some(_) => {
                // Encumbrance exists but is consumed/released
                return Err(ApiKeyAuthError::PaymentRequired(
                    "API key encumbrance is not active — re-encumber rJoules".into(),
                ));
            }
            None => {
                // No encumbrance at all — key has no allocated rJoules
                return Err(ApiKeyAuthError::PaymentRequired(
                    "API key has no rJoules allocated — use `kask wallet encumber` first".into(),
                ));
            }
        }

        // Verify scope: the request path must match the key's declared scope.
        // An empty scope means unrestricted access. Otherwise, at least one
        // scope entry must be a prefix of the request URI path.
        if !capability.scope.is_empty() {
            let path = request.uri().path();
            let scope_matched = capability.scope.iter().any(|s| path.starts_with(s));
            if !scope_matched {
                return Err(ApiKeyAuthError::ScopeViolation {
                    path: path.to_string(),
                    allowed_scopes: capability.scope.clone(),
                });
            }
        }

        Ok(WalletContext {
            wallet_id: capability.wallet_id,
            key_id: capability.key_id,
            spending_limit_rj: capability.spending_limit_rj,
            spent_rj: capability.spent_rj,
        })
    }
}

/// Errors returned by API key authentication.
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyAuthError {
    #[error("Missing Authorization header")]
    MissingAuthorization,
    #[error("Invalid Authorization header format (expected: Bearer <hex-key>)")]
    InvalidAuthorizationFormat,
    #[error("Invalid API key format (expected: 64-character hex string)")]
    InvalidKeyFormat,
    #[error("Unknown API key")]
    UnknownApiKey,
    #[error("API key has expired")]
    KeyExpired,
    #[error("API key spending limit exceeded")]
    SpendingLimitExceeded,
    #[error("Payment required: {0}")]
    PaymentRequired(String),
    #[error("Scope violation: {path} not in allowed scopes {allowed_scopes:?}")]
    ScopeViolation {
        path: String,
        allowed_scopes: Vec<String>,
    },
    #[error("Wallet store error")]
    StoreError,
}

impl IntoResponse for ApiKeyAuthError {
    fn into_response(self) -> Response {
        let (status, message): (StatusCode, String) = match self {
            ApiKeyAuthError::MissingAuthorization => (
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".into(),
            ),
            ApiKeyAuthError::InvalidAuthorizationFormat => (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format".into(),
            ),
            ApiKeyAuthError::InvalidKeyFormat => {
                (StatusCode::UNAUTHORIZED, "Invalid API key format".into())
            }
            ApiKeyAuthError::UnknownApiKey => (StatusCode::UNAUTHORIZED, "Unknown API key".into()),
            ApiKeyAuthError::KeyExpired => (StatusCode::FORBIDDEN, "API key has expired".into()),
            ApiKeyAuthError::SpendingLimitExceeded => (
                StatusCode::FORBIDDEN,
                "API key spending limit exceeded".into(),
            ),
            ApiKeyAuthError::PaymentRequired(msg) => (StatusCode::PAYMENT_REQUIRED, msg),
            ApiKeyAuthError::ScopeViolation {
                path,
                allowed_scopes,
            } => (
                StatusCode::FORBIDDEN,
                format!(
                    "Scope violation: '{}' not in allowed scopes {:?}",
                    path, allowed_scopes
                ),
            ),
            ApiKeyAuthError::StoreError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal authentication error".into(),
            ),
        };
        (status, message).into_response()
    }
}

/// Axum middleware function for API key authentication.
///
/// Pass-through: if no `Authorization: Bearer` header is present, the request
/// proceeds without API key auth (capability token auth still applies from the
/// global `auth_middleware`). This allows wallet and non-wallet routes to coexist.
///
/// When a valid Bearer token is present, registers a wallet-backed energy budget
/// in the CNS so that subsequent tool/inference calls consume rJoules from the
/// key's encumbrance.
pub async fn api_key_auth_middleware(
    State(auth): State<Arc<ApiKeyAuthService>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiKeyAuthError> {
    // Pass-through: if no Bearer token, skip API key auth.
    // Capability token auth is handled by the global auth_middleware.
    let has_bearer = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.starts_with("Bearer "))
        .unwrap_or(false);

    if !has_bearer {
        return Ok(next.run(request).await);
    }

    let ctx = auth.authenticate(&request)?;

    // Register wallet-backed budget so GovernedTool/GovernedInference
    // debit from this key's encumbrance during the request.
    let _ = auth
        .wallet_service
        .register_wallet_budget_for_key(
            auth.system_webid,
            ctx.wallet_id,
            ctx.key_id,
            ctx.spending_limit_rj,
        )
        .await;

    // Attach wallet context to request extensions for downstream handlers
    let mut request = request;
    request.extensions_mut().insert(ctx);

    Ok(next.run(request).await)
}
