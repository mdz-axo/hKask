//! API authentication middleware — Capability token verification
//!
//! Extracts `Authorization: Bearer <token>` from incoming requests,
//! verifies the HMAC-SHA256 signature using the MCP security key derived
//! from the master key via HKDF-SHA256, checks expiry, and attaches
//! the validated `DelegationToken` and `WebID` to request extensions.
//!
//! Routes that don't require auth (health checks, model listing) are
//! excluded from authentication.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use hkask_types::{DelegationToken, SYSTEM_MAX_ATTENUATION, WebID};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Routes that bypass authentication (health checks, model listing).
const PUBLIC_PATHS: &[&str] = &["/api/cns/health", "/api/models", "/api/models/search"];

/// Service holding the master key derivation context for capability token
/// verification.
#[derive(Debug, Clone)]
pub struct AuthService {
    /// Resolved HMAC key derived from the master key via HKDF-SHA256.
    secret: Arc<Vec<u8>>,
    /// Revoked capability token IDs (sync RwLock for use in sync verify_token)
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl AuthService {
    /// Create a new `AuthService` by deriving the MCP security key from
    /// the master key environment variable.
    ///
    /// Delegates to the keystore's domain-specific resolution chain.
    pub fn new() -> Result<Self, String> {
        let secret = hkask_keystore::resolve_mcp_security_key()
            .map_err(|e| format!("MCP security key not available: {}", e))?;

        Ok(Self {
            secret: Arc::new((*secret).clone()),
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Create an `AuthService` from an explicit secret (useful for tests).
    pub fn from_secret(secret: Vec<u8>) -> Self {
        Self {
            secret: Arc::new(secret),
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Revoke a capability token by its ID.
    pub fn revoke_token(&self, token_id: String) {
        if let Ok(mut revoked) = self.revoked_tokens.write() {
            revoked.insert(token_id);
        }
    }

    /// Check whether a capability token has been revoked.
    pub fn is_token_revoked(&self, token_id: &str) -> bool {
        self.revoked_tokens
            .read()
            .map(|set| set.contains(token_id))
            .unwrap_or(false)
    }

    /// Verify a capability token cryptographically and check expiry.
    pub fn verify_token(&self, token: &DelegationToken) -> TokenVerification {
        // 1. Verify HMAC-SHA256 signature
        if !token.verify_cryptographic(&self.secret) {
            return TokenVerification::Invalid;
        }

        // 2. Check expiry
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if token.is_expired(current_time) {
            return TokenVerification::Expired;
        }

        // 3. Verify attenuation chain (root nonce matches, level valid)
        if !token.verify_attenuation_chain(token.root_context_nonce(), SYSTEM_MAX_ATTENUATION) {
            return TokenVerification::Invalid;
        }

        // 4. Check revocation
        if self.is_token_revoked(&token.id) {
            return TokenVerification::Revoked;
        }

        TokenVerification::Valid
    }
}

/// Result of token verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenVerification {
    /// Token is valid and not expired.
    Valid,
    /// Signature is invalid or attenuation chain is broken.
    Invalid,
    /// Signature is valid but token has expired.
    Expired,
    /// Token has been revoked.
    Revoked,
}

/// Extracted auth context attached to validated requests.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The verified capability token.
    pub token: DelegationToken,
    /// The WebID of the token holder.
    pub webid: WebID,
}

/// Middleware function that performs capability token authentication.
///
/// Returns:
/// - `401 Unauthorized` for missing or invalid tokens
/// - `403 Forbidden` for expired tokens
/// - Passes through for routes listed in `PUBLIC_PATHS`
pub async fn auth_middleware(
    State(service): State<Arc<AuthService>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // Allow public routes without authentication
    if PUBLIC_PATHS.iter().any(|prefix| path.starts_with(prefix)) {
        return next.run(req).await;
    }

    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token_str = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(_) | None => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Missing or malformed Authorization header"))
                .unwrap();
        }
    };

    // Base64-decode the token
    let token = match DelegationToken::from_base64(token_str) {
        Ok(t) => t,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("Invalid token encoding"))
                .unwrap();
        }
    };

    // Verify the token
    match service.verify_token(&token) {
        TokenVerification::Valid => {
            // Double-check revocation via async-safe method
            if service.is_token_revoked(&token.id) {
                return Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Token has been revoked"))
                    .unwrap();
            }

            let webid = token.holder();

            // Attach auth context to request extensions
            let mut req = req;
            req.extensions_mut().insert(AuthContext { token, webid });

            next.run(req).await
        }
        TokenVerification::Expired => Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Token expired"))
            .unwrap(),
        TokenVerification::Invalid => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("Invalid capability token"))
            .unwrap(),
        TokenVerification::Revoked => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("Token has been revoked"))
            .unwrap(),
    }
}
