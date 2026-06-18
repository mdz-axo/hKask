//! OAuth authentication routes — GitHub/Google sign-in for hKask cloud deployment.
//!
//! # REQ: P1-deploy-oauth-login — P1 User Sovereignty: OAuth sign-in with session cookie.
//! expect: "I can sign in via OAuth to access my hKask server" [P1]
//! # REQ: P12-deploy-oauth-attribution — P12 Anonymous Agency: every action tied to authenticated WebID.
//! expect: "Every OAuth session is tied to my authenticated WebID" [P12]
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
use hkask_rsolidity as rs;
use hkask_types::identity::OAuthProvider;
use serde::Deserialize;
use tracing;

use crate::ApiState;
use crate::middleware::session::extract_cookie;

/// Query parameters for OAuth login initiation.
#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub provider: Option<String>,
}

/// Query parameters for OAuth callback.
#[derive(Debug, Deserialize)]
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
    /// Load OAuth config from environment variables.
    /// expect: "My API access is scoped to my sovereignty boundaries" [P1]
    fn from_env(provider: &OAuthProvider) -> Result<Self, String> {
        match provider {
            OAuthProvider::GitHub => {
                let client_id = std::env::var("HKASK_OAUTH_GITHUB_CLIENT_ID")
                    .map_err(|_| "HKASK_OAUTH_GITHUB_CLIENT_ID not set".to_string())?;
                let client_secret = std::env::var("HKASK_OAUTH_GITHUB_CLIENT_SECRET")
                    .map_err(|_| "HKASK_OAUTH_GITHUB_CLIENT_SECRET not set".to_string())?;
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
                let client_id = std::env::var("HKASK_OAUTH_GOOGLE_CLIENT_ID")
                    .map_err(|_| "HKASK_OAUTH_GOOGLE_CLIENT_ID not set".to_string())?;
                let client_secret = std::env::var("HKASK_OAUTH_GOOGLE_CLIENT_SECRET")
                    .map_err(|_| "HKASK_OAUTH_GOOGLE_CLIENT_SECRET not set".to_string())?;
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
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
/// pre:  provider query param is "github" or "google"
/// post: redirects to provider's OAuth authorize URL
/// post: sets state cookie for CSRF verification
pub async fn login(
    State(_state): State<ApiState>,
    Query(query): Query<LoginQuery>,
) -> Result<Response, (StatusCode, String)> {
    let provider_str = query.provider.as_deref().unwrap_or("github");
    let provider: OAuthProvider = provider_str
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let config =
        OAuthConfig::from_env(&provider).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

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

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, authorize_url)
        .header(header::SET_COOKIE, state_cookie)
        .body(axum::body::Body::empty())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
}

/// GET /api/v1/auth/callback
///
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
/// pre:  code is a valid OAuth authorization code; state matches cookie
/// post: session created, session cookie set, redirected to /terminal
/// post: new HumanUser + ReplicantIdentity created on first sign-in
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

    // Verify CSRF state
    let expected_state = query.state.as_deref().unwrap_or("");
    let cookie_state = extract_cookie(&headers, "hkask_oauth_state");
    if expected_state.is_empty() || cookie_state != Some(expected_state.to_string()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid OAuth state (CSRF check failed)".to_string(),
        ));
    }

    let config =
        OAuthConfig::from_env(&provider).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

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
    let user_store = state.agent_service.user_store();
    let user_store = user_store.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock error: {e}"),
        )
    })?;

    let (_user, replicant) = user_store
        .find_or_create_oauth_user(&provider, &provider_user_id, &email, &display_name)
        .map_err(|e| {
            tracing::error!(target: "hkask.api.oauth", error = %e, "Failed to find/create OAuth user");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("User creation failed: {e}"))
        })?;

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
        replicant = %replicant.replicant_name,
        webid = %replicant.replicant_webid,
        "OAuth sign-in complete"
    );

    // Clear state cookie and set session cookie
    let clear_state = "hkask_oauth_state=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0";
    let session_cookie = format!(
        "hkask_session={}; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age={}",
        session.session_id,
        86400 * 7 // 7 days
    );

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/terminal")
        .header(header::SET_COOKIE, clear_state)
        .header(header::SET_COOKIE, session_cookie)
        .body(axum::body::Body::empty())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
}

/// GitHub token exchange response.
#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: String,
    #[serde(default)]
    #[allow(dead_code)]
    scope: String,
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
        .header("User-Agent", "hKask/0.28.0")
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
                .header("User-Agent", "hKask/0.28.0")
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
                .unwrap_or_else(|| format!("{provider_user_id}@github.users.noreply"))
        }
    };

    Ok((provider_user_id, display_name, email))
}

/// POST /api/v1/auth/logout — destroys the current session.
///
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
pub async fn logout(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    if let Some(session_id) = extract_cookie(&headers, "hkask_session") {
        let user_store = state.agent_service.user_store();
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Lock error: {e}"),
            )
        })?;
        let _ = store.logout(&session_id);
    }
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/")
        .header(
            header::SET_COOKIE,
            "hkask_session=; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=0",
        )
        .body(axum::body::Body::empty())
        .unwrap())
}

/// GET /api/v1/auth/session — returns current session info.
///
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
pub async fn session_info(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let session_id = extract_cookie(&headers, "hkask_session")
        .ok_or((StatusCode::UNAUTHORIZED, "No session".to_string()))?;
    let user_store = state.agent_service.user_store();
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
        "replicant_name": session.replicant_name,
        "webid": session.replicant_webid.to_string(),
        "expires_at": session.expires_at,
        "last_active": session.last_active
    })))
}

/// POST /api/v1/auth/accept-invite
///
/// REQ: P2-multi-accept-invite-route
/// expect: "I can accept an invite code to join a server" [P2]
/// pre:  code is a valid invite code
/// post: if not authenticated: redirect to OAuth with invite code in state
/// post: if authenticated: accept invite, link user, return success
pub async fn accept_invite(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<AcceptInviteBody>,
) -> Result<Response, (StatusCode, String)> {
    let user_store = state.agent_service.user_store();
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
        let redirect_url = format!(
            "/api/v1/auth/login?provider=github&state=invite:{}",
            body.code
        );
        return Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, redirect_url)
            .body(axum::body::Body::empty())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }
    let session_id = session_cookie.unwrap();
    let session = user_store
        .get_session(&session_id)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Session invalid: {e}")))?
        .ok_or((StatusCode::UNAUTHORIZED, "Session expired".into()))?;
    let now = chrono::Utc::now().timestamp();
    if session.expires_at <= now {
        return Err((StatusCode::UNAUTHORIZED, "Session expired".into()));
    }
    let replicant = user_store
        .get_replicant_by_webid(&session.replicant_webid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))?
        .ok_or((StatusCode::UNAUTHORIZED, "Replicant not found".into()))?;
    user_store
        .accept_invite(&body.code, &replicant.user_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Accept failed: {e}")))?;
    let body = serde_json::json!({
        "status": "accepted",
        "code": body.code,
        "replicant": replicant.replicant_name,
    });
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
}

#[derive(Deserialize)]
pub struct AcceptInviteBody {
    code: String,
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
/// expect: "My API access is scoped to my sovereignty boundaries" [P1]
pub fn auth_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new()
        .route("/api/v1/auth/login", axum::routing::get(login))
        .route("/api/v1/auth/callback", axum::routing::get(callback))
        .route("/api/v1/auth/logout", axum::routing::post(logout))
        .route("/api/v1/auth/session", axum::routing::get(session_info))
        .route(
            "/api/v1/auth/accept-invite",
            axum::routing::post(accept_invite),
        )
}
