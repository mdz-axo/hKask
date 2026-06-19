//! Session cookie middleware — authenticates users via `hkask_session` cookie.
//!
//! # REQ: P1-deploy-session-middleware — session cookie auth coexists with capability token auth.
//! Runs BEFORE the capability token middleware. If a valid session cookie is found,
//! injects an `AuthContext` into request extensions so the capability token middleware
//! can skip Bearer token verification.

use crate::middleware::auth::AuthContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use hkask_rsolidity as rs;
use hkask_storage::user_store::UserStore;
use std::sync::{Arc, Mutex};

/// Extract a cookie value by name from request headers.
pub(crate) fn extract_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == name
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

/// Session middleware implementation — called from a closure in `create_router`.
///
pub async fn session_middleware_impl(
    user_store: &Arc<Mutex<UserStore>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Check for session cookie — if absent, pass through to capability token auth
    let session_id = match extract_cookie(req.headers(), "hkask_session") {
        Some(id) => id,
        None => return next.run(req).await,
    };

    // Load session from UserStore — drop lock before any await
    let session_result = {
        let store = user_store.lock();
        match store {
            Ok(s) => s.get_session(&session_id),
            Err(_) => Err(hkask_storage::user_store::UserStoreError::Infra(
                hkask_types::InfrastructureError::LockPoisoned,
            )),
        }
    }; // MutexGuard dropped here

    let session = match session_result {
        Ok(Some(s)) => s,
        _ => {
            // Invalid or missing session — return 401 with clear message
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    header::SET_COOKIE,
                    "hkask_session=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0",
                )
                .body(Body::from(
                    "Session invalid or expired. Please sign in again.",
                ))
                .expect("static response builder");
        }
    };

    // Check expiry — clear expired session cookie and return 401
    let now = chrono::Utc::now().timestamp();
    if session.is_expired(now) {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(
                header::SET_COOKIE,
                "hkask_session=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0",
            )
            .body(Body::from(
                "Session invalid or expired. Please sign in again.",
            ))
            .expect("static response builder");
    }

    // Inject AuthContext
    let webid = session.replicant_webid;
    req.extensions_mut()
        .insert(AuthContext::from_session(webid));

    next.run(req).await
}
