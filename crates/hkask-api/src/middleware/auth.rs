//! API authentication middleware — Capability token verification
//!
//! Extracts `Authorization: Bearer <token>` from incoming requests,
//! verifies the HMAC-SHA256 signature using the MCP security key derived
//! from the master key via HKDF-SHA256, checks expiry, and attaches
//! the validated `CapabilityToken` and `WebID` to request extensions.
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
use hkask_types::{CapabilityToken, SYSTEM_MAX_ATTENUATION, SecretRef, WebID, derivation_contexts};
use std::sync::Arc;

/// Routes that bypass authentication (health checks, model listing).
const PUBLIC_PATHS: &[&str] = &["/api/cns/health", "/api/models", "/api/models/search"];

/// Service holding the master key derivation context for capability token
/// verification.
#[derive(Debug, Clone)]
pub struct AuthService {
    /// Resolved HMAC key derived from the master key via HKDF-SHA256.
    secret: Arc<Vec<u8>>,
}

impl AuthService {
    /// Create a new `AuthService` by deriving the MCP security key from
    /// the master key environment variable.
    ///
    /// Resolution chain: `SecretRef::Derived` → env var → keychain.
    pub fn new() -> Result<Self, String> {
        let secret = hkask_keystore::resolve(&SecretRef::derived(
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::MCP_SECURITY_KEY,
        ))
        .or_else(|_| hkask_keystore::resolve(&SecretRef::env("HKASK_MCP_SECURITY_KEY")))
        .or_else(|_| hkask_keystore::resolve(&SecretRef::Keychain("mcp-security-key".to_string())))
        .map_err(|e| format!("MCP security key not available: {}", e))?;

        Ok(Self {
            secret: Arc::new((*secret).clone()),
        })
    }

    /// Create an `AuthService` from an explicit secret (useful for tests).
    pub fn from_secret(secret: Vec<u8>) -> Self {
        Self {
            secret: Arc::new(secret),
        }
    }

    /// Verify a capability token cryptographically and check expiry.
    pub fn verify_token(&self, token: &CapabilityToken) -> TokenVerification {
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
}

/// Axum layer wrapping `AuthService`.
#[derive(Clone)]
pub struct AuthLayer {
    #[allow(dead_code)] // Available for alternative wiring patterns
    service: Arc<AuthService>,
}

impl AuthLayer {
    /// Create a new auth layer from an `AuthService`.
    pub fn new(service: AuthService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

/// Extracted auth context attached to validated requests.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The verified capability token.
    pub token: CapabilityToken,
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
    let token = match CapabilityToken::from_base64(token_str) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{CapabilityAction, CapabilityResource, WebID};

    fn make_test_token(secret: &[u8]) -> CapabilityToken {
        let from = WebID::from_persona(b"test-issuer");
        let to = WebID::from_persona(b"test-holder");
        CapabilityToken::new(
            CapabilityResource::Tool,
            "test-tool".to_string(),
            CapabilityAction::Execute,
            from,
            to,
            secret,
        )
    }

    #[test]
    fn test_valid_token_verifies() {
        let secret = b"test-secret-key-for-hmac-123456";
        let service = AuthService::from_secret(secret.to_vec());
        let token = make_test_token(secret);
        assert_eq!(service.verify_token(&token), TokenVerification::Valid);
    }

    #[test]
    fn test_wrong_secret_fails() {
        let service = AuthService::from_secret(b"correct-secret".to_vec());
        let token = make_test_token(b"wrong-secret");
        assert_eq!(service.verify_token(&token), TokenVerification::Invalid);
    }
}
