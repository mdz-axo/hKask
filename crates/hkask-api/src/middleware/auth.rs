//! API authentication middleware — Capability token verification
//!
//! Extracts `Authorization: Bearer <token>` from incoming requests,
//! verifies the Ed25519 signature using the token's embedded public key,
//! checks expiry, and attaches the validated `DelegationToken` and `WebID`
//! to request extensions.
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
use hkask_types::{DelegationToken, SYSTEM_MAX_ATTENUATION};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Routes that bypass authentication (health checks, model listing).
const PUBLIC_PATHS: &[&str] = &["/api/cns/health", "/api/models", "/api/models/search"];

/// Service for capability token verification and revocation tracking.
#[derive(Debug, Clone)]
pub struct AuthService {
    /// Revoked capability token IDs (sync RwLock for use in sync verify_token)
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl AuthService {
    /// Create an `AuthService` from a ServiceConfig.
    ///
    /// REQ: API-022
    /// pre:  _config is a valid ServiceConfig (currently unused, reserved for future)
    /// post: returns AuthService with empty revocation set
    pub fn from_config(_config: &hkask_services::ServiceConfig) -> Self {
        Self {
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Revoke a capability token by its ID.
    ///
    /// REQ: API-023
    /// pre:  token_id is a valid token identifier string
    /// post: token_id is added to the revocation set (best-effort, RwLock write may fail silently)
    pub fn revoke_token(&self, token_id: String) {
        if let Ok(mut revoked) = self.revoked_tokens.write() {
            revoked.insert(token_id);
        }
    }

    /// Check whether a capability token has been revoked.
    ///
    /// REQ: API-024
    /// pre:  token_id is a valid token identifier string
    /// post: returns true iff token_id is in the revocation set
    /// post: returns false if RwLock read fails (conservative: assume not revoked)
    pub fn is_token_revoked(&self, token_id: &str) -> bool {
        self.revoked_tokens
            .read()
            .map(|set| set.contains(token_id))
            .unwrap_or(false)
    }

    /// Verify a capability token cryptographically and check expiry.
    ///
    /// REQ: API-025
    /// pre:  token is a valid DelegationToken
    /// post: returns TokenVerification::Invalid if signature or attenuation chain fails
    /// post: returns TokenVerification::Expired if token is past its expiry
    /// post: returns TokenVerification::Revoked if token_id is in revocation set
    /// post: returns TokenVerification::Valid iff all checks pass
    pub fn verify_token(&self, token: &DelegationToken) -> TokenVerification {
        // 1. Verify Ed25519 cryptographic signature
        if !token.verify_cryptographic() {
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
///
/// This is a type alias for the domain-level `AuthContext` in `hkask_types`,
/// which carries the verified capability token and the caller's WebID.
/// Both API (middleware) and CLI (keystore) paths produce the same type.
pub type AuthContext = hkask_types::AuthContext;

/// Build a response safely, falling back to a minimal status-only response if
/// the body cannot be constructed (e.g., header size overflow).
///
/// This avoids `.unwrap()` panics on `Response::builder().body(...)` which can
/// fail in edge cases (e.g., very large status codes or header values).
fn build_response(status: StatusCode, body: Body) -> Response {
    Response::builder()
        .status(status)
        .body(body)
        .unwrap_or_else(|_| {
            // Minimal fallback: status line only, no body
            Response::new(Body::empty())
        })
}

/// Middleware function that performs capability token authentication.
///
/// Returns:
/// - `401 Unauthorized` for missing or invalid tokens
/// - `403 Forbidden` for expired tokens
/// - Passes through for routes listed in `PUBLIC_PATHS`
///
/// REQ: API-026
/// pre:  service is a valid AuthService
/// post: if path in PUBLIC_PATHS → pass-through (next.run)
/// post: if missing Authorization header → 401
/// post: if invalid/expired/revoked token → 401 or 403
/// post: if valid token → AuthContext injected, next.run
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
            return build_response(
                StatusCode::UNAUTHORIZED,
                Body::from("Missing or malformed Authorization header"),
            );
        }
    };

    // Base64-decode the token
    let token = match DelegationToken::from_base64(token_str) {
        Ok(t) => t,
        Err(_) => {
            return build_response(
                StatusCode::UNAUTHORIZED,
                Body::from("Invalid token encoding"),
            );
        }
    };

    // Verify the token
    match service.verify_token(&token) {
        TokenVerification::Valid => {
            // Double-check revocation via async-safe method
            if service.is_token_revoked(&token.id) {
                return build_response(
                    StatusCode::UNAUTHORIZED,
                    Body::from("Token has been revoked"),
                );
            }

            let webid = token.holder();

            // Attach auth context to request extensions
            let mut req = req;
            req.extensions_mut().insert(AuthContext { token, webid });

            next.run(req).await
        }
        TokenVerification::Expired => {
            build_response(StatusCode::FORBIDDEN, Body::from("Token expired"))
        }
        TokenVerification::Invalid => build_response(
            StatusCode::UNAUTHORIZED,
            Body::from("Invalid capability token"),
        ),
        TokenVerification::Revoked => build_response(
            StatusCode::UNAUTHORIZED,
            Body::from("Token has been revoked"),
        ),
    }
}
