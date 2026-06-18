//! Admin role-gating middleware — restricts admin endpoints to Admin role.
//!
//! REQ: P1-multi-admin-middleware — Admin role gates admin endpoints.
//! After session middleware injects AuthContext, this middleware checks
//! the authenticated user's role against the UserStore.

use crate::ApiState;
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use hkask_types::identity::Role;
use hkask_types::capability::auth::AuthContext;

/// Admin-only paths that require the Admin role.
/// These are checked after session middleware injects AuthContext.
const ADMIN_PATH_PREFIXES: &[&str] = &["/api/v1/admin"];

/// Admin middleware: reject non-Admin requests to admin endpoints.
///
/// REQ: P1-multi-admin-gate
/// expect: "As an admin I am the only one who can access admin configuration" [P1]
/// pre:  AuthContext is present in request extensions (injected by session middleware)
/// post: 200 if path not admin-prefixed or role is Admin; 403 otherwise
pub async fn admin_middleware(
    state: axum::extract::State<ApiState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let is_admin_path = ADMIN_PATH_PREFIXES.iter().any(|prefix| path.starts_with(prefix));
    if !is_admin_path {
        return Ok(next.run(req).await);
    }
    let auth = req.extensions().get::<AuthContext>();
    let user_id = match auth {
        Some(ctx) => ctx.webid,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let user_store = state.agent_service.user_store();
    let user_store = user_store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = user_store.get_user(&user_id).map_err(|_| StatusCode::FORBIDDEN)?;
    if user.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
