//! OAuth authentication routes — GitHub/Google sign-in for hKask cloud deployment.
//!
//! # REQ: P1-deploy-oauth-login — P1 User Sovereignty: OAuth sign-in with session cookie.
//! expect: "I can sign in via OAuth to access my hKask server"
//! # REQ: P12-deploy-oauth-attribution — P12 Anonymous Agency: every action tied to authenticated WebID.
//! expect: "Every OAuth session is tied to my authenticated WebID"
//!
//! Flow:
//! 1. `GET /api/v1/auth/login?provider=github` → redirect to provider OAuth
//! 2. `GET /api/v1/auth/callback?provider=github&code=...&state=...` → exchange code, create session, redirect to /terminal

use axum::{
    Json,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use hkask_types::identity::OAuthProvider;
use serde::Deserialize;
use tracing;
use utoipa::IntoParams;
use utoipa::ToSchema;

use crate::ApiState;
use crate::error::ApiError;
use crate::middleware::session::extract_cookie;
use hkask_types::server_config::ServerConfig;

/// Query parameters for OAuth login initiation.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct LoginQuery {
    pub provider: Option<String>,
}

/// Query parameters for OAuth callback.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct CallbackQuery {
    pub provider: Option<String>,
    pub code: Option<String>,
    pub state: Option<String>,
}

/// OAuth configuration for a provider.
#[derive(Debug, Clone)]
struct OAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl OAuthConfig {
    /// Load OAuth config — prefers OS keychain (set by `kask init`), falls back to env vars.
    /// expect: "My API access is scoped to my sovereignty boundaries"
    fn from_env(provider: &OAuthProvider) -> Result<Self, ApiError> {
        let keychain = hkask_keystore::keychain::Keychain::new("hkask");
        match provider {
            OAuthProvider::GitHub => {
                let client_id = keychain
                    .retrieve_by_key(hkask_types::keychain_keys::KEY_OAUTH_GITHUB_CLIENT_ID)
                    .or_else(|_| std::env::var("HKASK_OAUTH_GITHUB_CLIENT_ID"))
                    .map_err(|_| ApiError::Internal {
                        message: "GitHub OAuth Client ID not found in keychain or env. Run 'kask init' first.".into(),
                    })?;
                let client_secret = keychain
                    .retrieve_by_key(hkask_types::keychain_keys::KEY_OAUTH_GITHUB_CLIENT_SECRET)
                    .or_else(|_| std::env::var("HKASK_OAUTH_GITHUB_CLIENT_SECRET"))
                    .map_err(|_| ApiError::Internal {
                        message: "GitHub OAuth Client Secret not found in keychain or env. Run 'kask init' first.".into(),
                    })?;
                let domain =
                    std::env::var("HKASK_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
                let scheme = if domain == "localhost" {
                    "http"
                } else {
                    "https"
                };
                Ok(Self {
                    client_id,
                    client_secret,
                    redirect_uri: format!(
                        "{scheme}://{domain}/api/v1/auth/callback?provider=github"
                    ),
                })
            }
            OAuthProvider::Google => {
                let client_id = keychain
                    .retrieve_by_key("hkask-oauth-google-client-id")
                    .or_else(|_| std::env::var("HKASK_OAUTH_GOOGLE_CLIENT_ID"))
                    .map_err(|_| ApiError::Internal {
                        message:
                            "Google OAuth Client ID not found. Set HKASK_OAUTH_GOOGLE_CLIENT_ID."
                                .into(),
                    })?;
                let client_secret = keychain
                    .retrieve_by_key("hkask-oauth-google-client-secret")
                    .or_else(|_| std::env::var("HKASK_OAUTH_GOOGLE_CLIENT_SECRET"))
                    .map_err(|_| ApiError::Internal {
                        message: "Google OAuth Client Secret not found. Set HKASK_OAUTH_GOOGLE_CLIENT_SECRET.".into(),
                    })?;
                let domain =
                    std::env::var("HKASK_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
                let scheme = if domain == "localhost" {
                    "http"
                } else {
                    "https"
                };
                Ok(Self {
                    client_id,
                    client_secret,
                    redirect_uri: format!(
                        "{scheme}://{domain}/api/v1/auth/callback?provider=google"
                    ),
                })
            }
        }
    }

    fn authorize_url(&self, provider: &OAuthProvider, state: &str) -> String {
        match provider {
            OAuthProvider::GitHub => format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=user:email",
                self.client_id,
                urlencoding(&self.redirect_uri),
                state
            ),
            OAuthProvider::Google => format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&state={}&response_type=code&scope=openid%20email%20profile",
                self.client_id,
                urlencoding(&self.redirect_uri),
                state
            ),
        }
    }
}

/// GitHub user info response from /user API.
#[derive(Debug, Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

/// GitHub email response from /user/emails API.
#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

/// URL-encode a string (basic implementation — only encodes special chars).
/// GET /api/v1/auth/login
///
/// expect: "My API access is scoped to my sovereignty boundaries"
/// pre:  provider query param is "github" or "google"
/// post: redirects to provider's OAuth authorize URL
/// post: sets state cookie for CSRF verification
#[utoipa::path(
    get,
    path = "/api/v1/auth/login",
    tag = "auth",
    params(LoginQuery),
    responses(
        (status = 302, description = "Redirect to OAuth provider"),
        (status = 400, description = "Invalid OAuth provider"),
    ),
)]
pub async fn login(
    State(_state): State<ApiState>,
    Query(query): Query<LoginQuery>,
) -> Result<Response, (StatusCode, String)> {
    let provider_str = query.provider.as_deref().unwrap_or("github");
    let provider: OAuthProvider = provider_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let config = OAuthConfig::from_env(&provider)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Generate CSRF state
    let csrf_state = uuid::Uuid::new_v4().to_string();
    let authorize_url = config.authorize_url(&provider, &csrf_state);

    tracing::info!(
        target = "hkask.api.oauth",
        provider = %provider,
        "Initiating OAuth flow"
    );

    // Build response with state cookie (5-minute expiry, HttpOnly, SameSite=Lax, Secure)
    let state_cookie = format!(
        "hkask_oauth_state={}; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=300",
        csrf_state
    );

    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, authorize_url)
        .header(header::SET_COOKIE, state_cookie)
        .body(axum::body::Body::empty())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// GET /api/v1/auth/callback
///
/// expect: "My API access is scoped to my sovereignty boundaries"
/// pre:  code is a valid OAuth authorization code; state matches cookie
/// post: session created, session cookie set, redirected to /terminal
/// post: new HumanUser + UserPod created on first sign-in
#[utoipa::path(
    get,
    path = "/api/v1/auth/callback",
    tag = "auth",
    params(CallbackQuery),
    responses(
        (status = 302, description = "Redirect to /terminal after successful authentication"),
        (status = 400, description = "Invalid callback parameters"),
    ),
)]
pub async fn callback(
    State(state): State<ApiState>,
    Query(query): Query<CallbackQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let provider_str = query.provider.as_deref().unwrap_or("github");
    let provider: OAuthProvider = provider_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let code = query.code.as_deref().ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'code' parameter".to_string(),
    ))?;

    // Verify CSRF state (skip for invite flow — invite code is the anti-forgery token)
    let expected_state = query.state.as_deref().unwrap_or("");
    let cookie_state = extract_cookie(&headers, "hkask_oauth_state");

    // Detect invite flow: check for invite code cookie (set by accept_invite redirect)
    let invite_code = extract_cookie(&headers, "hkask_invite_code");

    if invite_code.is_none() {
        // Normal flow: verify CSRF state
        if expected_state.is_empty() || cookie_state != Some(expected_state.to_string()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid OAuth state (CSRF check failed)".to_string(),
            ));
        }
    }

    let config = OAuthConfig::from_env(&provider)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Exchange code for access token
    let token_response = exchange_code(&config, code, &provider).await?;

    let (provider_user_id, display_name, email) = match provider {
        OAuthProvider::GitHub => fetch_github_user(&token_response.access_token).await?,
        OAuthProvider::Google => {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                "Google OAuth not yet supported".to_string(),
            ));
        }
    };

    // Find or create user
    let user_store = state.agent_service.storage().users.clone();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;

    // Registration mode guard — check if this is a closed server.
    // Uses load_or_default(): missing config → default (closed), corrupted config → deny.
    match ServerConfig::load_or_default() {
        Ok(config) => {
            if config.registration == hkask_types::server_config::ServerRegistration::Closed {
                match &invite_code {
                    Some(code) => {
                        let invite = user_store.lookup_invite(code).map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Invite lookup failed: {e}"),
                            )
                        })?;
                        if invite.is_none() {
                            return Response::builder()
                                .status(StatusCode::FOUND)
                                .header(header::LOCATION, "/invite-required")
                                .body(axum::body::Body::empty())
                                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
                        }
                        tracing::info!(
                            target = "hkask.api.oauth",
                            invite_code = %code,
                            "Invite validated for closed server"
                        );
                    }
                    None => {
                        return Response::builder()
                            .status(StatusCode::FOUND)
                            .header(header::LOCATION, "/invite-required")
                            .body(axum::body::Body::empty())
                            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
                    }
                }
            }
        }
        Err(e) => {
            // Config exists but is corrupted — fail closed for safety.
            tracing::error!(target = "hkask.api.oauth", error = %e, "Server config corrupted — denying registration");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server configuration error. Please contact the administrator.".to_string(),
            ));
        }
    }

    let (_user, replicant) = user_store
        .find_or_create_oauth_user(&provider, &provider_user_id, &email, &display_name)
        .map_err(|e| {
            tracing::error!(target: "hkask.api.oauth", error = %e, "Failed to find/create OAuth user");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("User creation failed: {e}"))
        })?;

    // If this was an invite flow, accept the invite now
    if let Some(ref code) = invite_code {
        if let Err(e) = user_store.accept_invite(code, &replicant.user_id) {
            tracing::warn!(
                target = "hkask.api.oauth",
                invite_code = %code,
                error = %e,
                "Failed to accept invite after OAuth — user created but invite not linked"
            );
        } else {
            tracing::info!(
                target = "cns.deploy.invite",
                operation = "invite_accepted",
                code = %code,
                webid = %replicant.webid,
                "CNS"
            );
        }
    }

    // Create session
    let session = user_store.create_oauth_session(&replicant).map_err(|e| {
        tracing::error!(target: "hkask.api.oauth", error = %e, "Failed to create session");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Session creation failed: {e}"),
        )
    })?;

    tracing::info!(
        target = "hkask.api.oauth",
        provider = %provider,
        replicant = %replicant.userpod_name,
        webid = %replicant.webid,
        "OAuth sign-in complete"
    );

    // CNS: SessionOpen span
    tracing::info!(
        target = "cns.deploy.session",
        operation = "session_open",
        provider = %provider,
        webid = %replicant.webid,
        "CNS"
    );
    // CNS: member activity — emitted on every sign-in so Curator can track server population
    tracing::info!(
        target = "cns.multi_user.member_active",
        operation = "member_sign_in",
        replicant = %replicant.userpod_name,
        webid = %replicant.webid,
        provider = %provider,
        is_invite_flow = invite_code.is_some(),
        "CNS"
    );

    // Fire-and-forget: register Matrix accounts on Conduit and join chat room.
    // Non-blocking — if Conduit is unavailable, the user can still use the system.
    let userpod_name = replicant.userpod_name.clone();
    let display = display_name.clone();
    tokio::spawn(async move {
        onboard_matrix(&userpod_name, &display).await;
    });

    // Clear state cookie and set session cookie
    let clear_state = "hkask_oauth_state=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0";
    let clear_invite = "hkask_invite_code=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0";
    let session_cookie = format!(
        "hkask_session={}; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age={}",
        session.session_id,
        86400 * 7 // 7 days
    );

    let mut builder = Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/terminal")
        .header(header::SET_COOKIE, clear_state)
        .header(header::SET_COOKIE, clear_invite)
        .header(header::SET_COOKIE, session_cookie);

    // If invite flow, redirect to onboarding instead of terminal.
    // Pass user info as query params so the page can personalize the welcome.
    if invite_code.is_some() {
        let onboarding_url = format!(
            "/onboarding?name={}&replicant={}",
            urlencoding(&display_name),
            urlencoding(&replicant.userpod_name),
        );
        builder = builder.header(header::LOCATION, onboarding_url);
    }

    builder
        .body(axum::body::Body::empty())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// GitHub token exchange response.
#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    #[serde(default)]
    _token_type: String,
    #[serde(default)]
    _scope: String,
}

/// Exchange OAuth code for access token.
async fn exchange_code(
    config: &OAuthConfig,
    code: &str,
    provider: &OAuthProvider,
) -> Result<GitHubTokenResponse, (StatusCode, String)> {
    let client = reqwest::Client::new();
    let resp = match provider {
        OAuthProvider::GitHub => {
            client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .json(&serde_json::json!({
                    "client_id": config.client_id,
                    "client_secret": config.client_secret,
                    "code": code,
                    "redirect_uri": config.redirect_uri,
                }))
                .send()
                .await
        }
        OAuthProvider::Google => {
            client
                .post("https://oauth2.googleapis.com/token")
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", config.client_id.as_str()),
                    ("client_secret", config.client_secret.as_str()),
                    ("code", code),
                    ("grant_type", "authorization_code"),
                    ("redirect_uri", config.redirect_uri.as_str()),
                ])
                .send()
                .await
        }
    }
    .map_err(|e| {
        tracing::error!(target: "hkask.api.oauth", error = %e, "Token exchange request failed");
        (
            StatusCode::BAD_GATEWAY,
            format!("Token exchange failed: {e}"),
        )
    })?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(target: "hkask.api.oauth", body = %body, "Token exchange returned error");
        return Err((StatusCode::BAD_GATEWAY, "Token exchange failed".to_string()));
    }

    resp.json::<GitHubTokenResponse>().await.map_err(|e| {
        tracing::error!(target: "hkask.api.oauth", error = %e, "Failed to parse token response");
        (
            StatusCode::BAD_GATEWAY,
            format!("Token response parse error: {e}"),
        )
    })
}

/// Fetch GitHub user info (id, login, email).
async fn fetch_github_user(
    access_token: &str,
) -> Result<(String, String, String), (StatusCode, String)> {
    let client = reqwest::Client::new();

    // Fetch user profile
    let user_resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", concat!("hKask/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| {
            tracing::error!(target: "hkask.api.oauth", error = %e, "GitHub /user request failed");
            (StatusCode::BAD_GATEWAY, format!("GitHub API error: {e}"))
        })?;

    if !user_resp.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "GitHub API returned error".to_string(),
        ));
    }

    let github_user = user_resp.json::<GitHubUser>().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("GitHub user parse error: {e}"),
        )
    })?;
    let provider_user_id = github_user.id.to_string();
    let display_name = github_user.name.unwrap_or(github_user.login);

    // Try to get email from profile; fall back to /user/emails
    let email = match github_user.email.filter(|e| !e.is_empty()) {
        Some(e) => e,
        None => {
            // Fetch verified emails
            let email_resp = client
                .get("https://api.github.com/user/emails")
                .header("Authorization", format!("Bearer {access_token}"))
                .header("User-Agent", concat!("hKask/", env!("CARGO_PKG_VERSION")))
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("GitHub emails API error: {e}"),
                    )
                })?;

            if !email_resp.status().is_success() {
                return Err((
                    StatusCode::BAD_GATEWAY,
                    "GitHub emails API returned error".to_string(),
                ));
            }

            let emails: Vec<GitHubEmail> = email_resp.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("GitHub emails parse error: {e}"),
                )
            })?;

            emails
                .into_iter()
                .find(|e| e.primary && e.verified)
                .map(|e| e.email)
                .ok_or_else(|| {
                    (
                        StatusCode::UNAUTHORIZED,
                        format!(
                            "GitHub user '{}' has no verified primary email — cannot establish identity",
                            provider_user_id
                        ),
                    )
                })?
        }
    };

    Ok((provider_user_id, display_name, email))
}

/// POST /api/v1/auth/logout — destroys the current session.
///
/// expect: "My API access is scoped to my sovereignty boundaries"
pub async fn logout(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    if let Some(session_id) = extract_cookie(&headers, "hkask_session") {
        let user_store = state.agent_service.storage().users.clone();
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        // CNS: SessionClose span (before destroying session, so we can log webid)
        if let Ok(Some(session)) = store.get_session(&session_id) {
            tracing::info!(
                target = "cns.deploy.session",
                operation = "session_close",
                webid = %session.webid,
                "CNS"
            );
        }
        let _ = store.logout(&session_id);
    }
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/")
        .header(
            header::SET_COOKIE,
            "hkask_session=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0",
        )
        .body(axum::body::Body::empty())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// GET /api/v1/auth/session — returns current session info.
///
/// expect: "My API access is scoped to my sovereignty boundaries"
pub async fn session_info(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let session_id = extract_cookie(&headers, "hkask_session")
        .ok_or((StatusCode::UNAUTHORIZED, "No session".to_string()))?;
    let user_store = state.agent_service.storage().users.clone();
    let store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let session = store
        .get_session(&session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid session".to_string()))?;
    let now = chrono::Utc::now().timestamp();
    if session.is_expired(now) {
        return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
    }
    Ok(Json(serde_json::json!({
        "userpod_name": session.userpod_name,
        "webid": session.webid.to_string(),
        "expires_at": session.expires_at,
        "last_active": session.last_active
    })))
}

/// POST /api/v1/auth/accept-invite
///
/// REQ: P2-multi-accept-invite-route
/// expect: "I can accept an invite code to join a server"
/// pre:  code is a valid invite code
/// post: if not authenticated: redirect to OAuth with invite code in state
/// post: if authenticated: accept invite, link user, return success
pub async fn accept_invite(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<AcceptInviteBody>,
) -> Result<Response, (StatusCode, String)> {
    let user_store = state.agent_service.storage().users.clone();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;
    let _invite = user_store
        .lookup_invite(&body.code)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lookup failed: {e}"),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Invite not found or expired".into()))?;
    let session_cookie = extract_cookie(&headers, "hkask_session");
    if session_cookie.is_none() {
        let redirect_url = "/api/v1/auth/login?provider=github";
        let invite_cookie = format!(
            "hkask_invite_code={}; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=1800",
            body.code
        );
        return Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, redirect_url)
            .header(header::SET_COOKIE, invite_cookie)
            .body(axum::body::Body::empty())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }
    let session_id = session_cookie
        .ok_or((StatusCode::UNAUTHORIZED, "No session cookie".into()))?
        .trim_matches('"')
        .to_string();
    let session = user_store
        .get_session(&session_id)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Session invalid: {e}")))?
        .ok_or((StatusCode::UNAUTHORIZED, "Session expired".into()))?;
    let now = chrono::Utc::now().timestamp();
    if session.expires_at <= now {
        return Err((StatusCode::UNAUTHORIZED, "Session expired".into()));
    }
    let replicant = user_store
        .get_replicant_by_webid(&session.webid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .ok_or((StatusCode::UNAUTHORIZED, "Replicant not found".into()))?;
    user_store
        .accept_invite(&body.code, &replicant.user_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Accept failed: {e}")))?;
    let body = serde_json::json!({
        "status": "accepted",
        "code": body.code,
        "replicant": replicant.userpod_name,
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Deserialize)]
pub struct AcceptInviteBody {
    code: String,
}

/// GET /api/v1/auth/accept-invite?code=XYZ
///
/// Browser-friendly invite acceptance — redirects through OAuth if not authenticated.
/// expect: "I can click an invite link to join a server"
pub async fn accept_invite_get(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<AcceptInviteQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Delegate to POST handler with the code from query params
    let body = AcceptInviteBody { code: query.code };
    accept_invite(State(state), headers, Json(body)).await
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteQuery {
    code: String,
}

// ── Matrix onboarding (fire-and-forget, non-blocking) ────────────────────

/// Shared HTTP client for Matrix API calls (connection pooling).
fn matrix_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Register Matrix accounts on Conduit and join the server chat room.
///
/// Delegates account registration to OnboardingService (shared with CLI path).
/// Called as a fire-and-forget task after OAuth sign-in. Non-blocking.
async fn onboard_matrix(userpod_name: &str, display_name: &str) {
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());

    // Delegate account registration to the onboarding service
    let result = hkask_services_onboarding::OnboardingService::register_oauth_matrix_accounts(
        display_name,
        userpod_name,
        &homeserver_url,
    )
    .await;

    let (human_id, replicant_id) = match result {
        Ok(r) => {
            tracing::info!(
                target = "hkask.api.matrix",
                human = %r.human_user_id,
                replicant = %r.replicant_user_id,
                "Matrix accounts registered via onboarding service"
            );
            (Some(r.human_user_id), Some(r.replicant_user_id))
        }
        Err(e) => {
            tracing::warn!(target = "hkask.api.matrix", error = %e, "Matrix registration failed (Conduit may be offline)");
            (None, None)
        }
    };

    // Ensure server chat room exists and invite users
    if human_id.is_some() || replicant_id.is_some() {
        ensure_chat_room(
            &homeserver_url,
            human_id.as_deref(),
            replicant_id.as_deref(),
        )
        .await;
    }
}

/// Ensure the server-wide chat room exists and invite new users to it.
///
/// Uses the curator's Matrix credentials to create the room if it doesn't exist.
/// Stores the room ID in ServerConfig for future invites.
async fn ensure_chat_room(
    homeserver_url: &str,
    human_id: Option<&str>,
    replicant_id: Option<&str>,
) {
    // Load or create the server chat room
    let room_id = match get_or_create_server_room(homeserver_url).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(target: "hkask.api.matrix", error = %e, "Could not ensure chat room");
            return;
        }
    };

    // Invite users using the Matrix admin API (/_synapse/admin — Conduit-compatible)
    for user_id in [human_id, replicant_id].into_iter().flatten() {
        match matrix_invite_to_room(homeserver_url, &room_id, user_id).await {
            Ok(()) => {
                tracing::info!(target: "hkask.api.matrix", user = %user_id, room = %room_id, "User invited to chat room");
                tracing::info!(target: "hkask.communication.matrix.room.invite", operation = "user_invited", room = %room_id, user = %user_id, "CNS");
            }
            Err(e) => {
                tracing::warn!(target: "hkask.api.matrix", user = %user_id, error = %e, "Failed to invite user to chat room");
            }
        }
    }
}

/// Get or create the server-wide chat room using the curator's Matrix account.
async fn get_or_create_server_room(homeserver_url: &str) -> anyhow::Result<String> {
    // Check if room already exists in ServerConfig
    #[allow(clippy::collapsible_if)]
    if let Ok(config) = hkask_types::server_config::ServerConfig::load() {
        if let Some(ref room_id) = config.conduit_room_id {
            if !room_id.is_empty() {
                return Ok(room_id.clone());
            }
        }
    }

    // Create room via Matrix API
    let url = format!(
        "{}/_matrix/client/v3/createRoom",
        homeserver_url.trim_end_matches('/')
    );

    let curator_token = get_curator_access_token(homeserver_url).await?;

    let body = serde_json::json!({
        "name": "hKask Server Chat",
        "topic": "Welcome to hKask — team chat for this server",
        "preset": "trusted_private_chat",
    });

    let response = matrix_client()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {curator_token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Create room HTTP error: {e}"))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Create room failed: HTTP {}",
            response.status()
        ));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Create room parse error: {e}"))?;

    let room_id = result["room_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Create room response missing room_id"))?
        .to_string();

    // Store in ServerConfig for future lookups.
    // Handle TOCTOU race: another concurrent sign-in may have just created a room.
    // Reload config after room creation — if another task beat us, use their room_id.
    let final_room_id = if let Ok(mut config) = hkask_types::server_config::ServerConfig::load() {
        if config.conduit_room_id.as_deref() == Some("") {
            config.conduit_room_id = None;
        }
        match &config.conduit_room_id {
            Some(existing) if !existing.is_empty() => {
                // Another task already created and stored a room. Use theirs.
                tracing::info!(
                    target = "hkask.api.matrix",
                    existing_room = %existing,
                    our_room = %room_id,
                    "Another task already created the chat room — using existing"
                );
                existing.clone()
            }
            _ => {
                // We're first — store our room_id.
                config.conduit_room_id = Some(room_id.clone());
                if let Err(e) = config.save() {
                    tracing::warn!(target: "hkask.api.matrix", error = %e, "Failed to save room_id to config");
                }
                room_id
            }
        }
    } else {
        room_id
    };

    tracing::info!(target: "hkask.api.matrix", room_id = %final_room_id, "Server chat room ensured");
    tracing::info!(target: "hkask.communication.matrix.room.created", operation = "room_ensured", room_id = %final_room_id, "CNS");
    Ok(final_room_id)
}

/// Get the curator's Matrix access token by logging in.
async fn get_curator_access_token(homeserver_url: &str) -> anyhow::Result<String> {
    let keychain = hkask_keystore::Keychain::default();
    let curator_password = keychain
        .retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_BOT_CURATOR)
        .or_else(|_| {
            std::env::var("HKASK_CURATOR_MATRIX_PASSWORD")
                .map_err(|_| anyhow::anyhow!("Curator Matrix password not found in keychain or HKASK_CURATOR_MATRIX_PASSWORD env var. Run 'kask init' or set the env var."))
        })?;

    let url = format!(
        "{}/_matrix/client/v3/login",
        homeserver_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "type": "m.login.password",
        "identifier": {
            "type": "m.id.user",
            "user": "@hkask-curator:localhost"
        },
        "password": curator_password,
    });

    let response = matrix_client()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Curator login HTTP error: {e}"))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Curator login failed: HTTP {}",
            response.status()
        ));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Curator login parse error: {e}"))?;

    result["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Curator login response missing access_token"))
}

/// Invite a user to a Matrix room using an admin access token.
async fn matrix_invite_to_room(
    homeserver_url: &str,
    room_id: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    let curator_token = get_curator_access_token(homeserver_url).await?;

    let encoded_room = urlencoding(room_id);
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/invite",
        homeserver_url.trim_end_matches('/'),
        encoded_room
    );

    let body = serde_json::json!({"user_id": user_id});

    let response = matrix_client()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {curator_token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Invite HTTP error: {e}"))?;

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or_default();
        let msg = error_body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown");
        return Err(anyhow::anyhow!("Invite failed ({msg})"));
    }

    Ok(())
}

/// URL-encode a string (basic implementation — only encodes special chars).
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            '?' => "%3F".to_string(),
            '#' => "%23".to_string(),
            '[' => "%5B".to_string(),
            ']' => "%5D".to_string(),
            '@' => "%40".to_string(),
            '!' => "%21".to_string(),
            '$' => "%24".to_string(),
            '&' => "%26".to_string(),
            '\'' => "%27".to_string(),
            '(' => "%28".to_string(),
            ')' => "%29".to_string(),
            '*' => "%2A".to_string(),
            '+' => "%2B".to_string(),
            ',' => "%2C".to_string(),
            ';' => "%3B".to_string(),
            '=' => "%3D".to_string(),
            '%' => "%25".to_string(),
            ' ' => "+".to_string(),
            other => other.to_string(),
        })
        .collect()
}

/// Build the auth router.
///
/// expect: "My API access is scoped to my sovereignty boundaries"
pub fn auth_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    use utoipa_axum::routes;
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(callback))
        .route("/api/v1/auth/logout", axum::routing::post(logout))
        .route("/api/v1/auth/session", axum::routing::get(session_info))
        .route(
            "/api/v1/auth/accept-invite",
            axum::routing::post(accept_invite),
        )
        .route(
            "/api/v1/auth/accept-invite",
            axum::routing::get(accept_invite_get),
        )
}
