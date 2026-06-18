//! Admin role-gating middleware — restricts admin endpoints to Admin role.
//!
//! REQ: P1-multi-admin-middleware — Admin role gates admin endpoints.

use hkask_rsolidity::contract;

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use hkask_types::identity::Role;
use hkask_types::capability::auth::AuthContext;
use hkask_types::WebID;
use std::sync::{Arc, Mutex};
use hkask_storage::user_store::UserStore;

const ADMIN_PATH_PREFIXES: &[&str] = &["/api/v1/admin"];

/// Admin middleware: reject non-Admin requests to admin endpoints.
///
/// expect: "As an admin I am the only one who can access admin configuration" [P1]
    #[contract(id = "P1-multi-admin-gate", principle = "P1")]
pub async fn admin_middleware(
    store: Arc<Mutex<UserStore>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let is_admin_path = ADMIN_PATH_PREFIXES.iter().any(|prefix| path.starts_with(prefix));
    if !is_admin_path {
        return Ok(next.run(req).await);
    }
    let auth = req.extensions().get::<AuthContext>();
    let webid = match auth {
        Some(ctx) => ctx.webid,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let is_admin = {
        let store = store.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let replicant = store.get_replicant_by_webid(&webid)
            .map_err(|_| StatusCode::FORBIDDEN)?
            .ok_or(StatusCode::FORBIDDEN)?;
        let user = store.get_user(&replicant.user_id)
            .map_err(|_| StatusCode::FORBIDDEN)?;
        user.role == Role::Admin
    };
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
