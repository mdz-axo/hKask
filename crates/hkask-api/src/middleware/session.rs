//! Session cookie middleware — authenticates users via `hkask_session` cookie.
//!
//! # REQ: DEP-020 — session cookie auth coexists with capability token auth.
//!
//! Runs BEFORE the capability token middleware. If a valid session cookie is found,
//! injects an `AuthContext` into request extensions so the capability token middleware
//! can skip Bearer token verification.

use crate::middleware::auth::AuthContext;
use axum::{
    body::Body,
    http::{Request, header},
    middleware::Next,
    response::Response,
};
use hkask_storage::user_store::UserStore;
use std::sync::{Arc, Mutex};

/// Extract a cookie value by name from request headers.
fn get_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Session middleware implementation — called from a closure in `create_router`.
///
/// REQ: DEP-020
/// pre:  user_store is a valid Arc<Mutex<UserStore>>
/// post: if valid session cookie → AuthContext injected into req, next.run called
/// post: if invalid/expired/missing cookie → pass through
pub async fn session_middleware_impl(
    user_store: &Arc<Mutex<UserStore>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Check for session cookie
    let session_id = match get_cookie(req.headers(), "hkask_session") {
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
        Ok(Some(s)) => Some(s),
        _ => None,
    };

    let session = match session {
        Some(s) => s,
        None => return next.run(req).await,
    };

    // Check expiry
    let now = chrono::Utc::now().timestamp();
    if session.is_expired(now) {
        return next.run(req).await;
    }

    // Inject AuthContext
    let webid = session.replicant_webid;
    req.extensions_mut()
        .insert(AuthContext::from_session(webid));

    next.run(req).await
}
