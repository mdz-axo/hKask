//! Curator email delivery — sends invites, notifications, and alerts via MXroute HTTP API.
//!
//! Uses MXroute's SMTP API at smtpapi.mxroute.com — no SMTP library needed.
//! Also provides the foundation for replicant email access (future: per-replicant credentials).
//!
//! expect: "The Curator can send email on behalf of the server"

/// Errors that can occur during email delivery.
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Email delivery is not configured: {0}")]
    NotConfigured(String),
    #[error("MXroute API request failed: {0}")]
    ApiRequest(String),
    #[error("MXroute API returned HTTP {0}")]
    ApiStatus(u16),
    #[error("MXroute API error: {0}")]
    ApiError(String),
}

/// Result type for email operations.
pub type EmailResult<T> = std::result::Result<T, EmailError>;

/// Shared HTTP client for MXroute API calls.
fn email_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Send an email via MXroute's HTTP API.
///
/// Reads credentials from environment variables:
/// - HKASK_MXROUTE_SERVER — MXroute server hostname (e.g., "tuesday.mxrouting.net")
/// - HKASK_SMTP_USERNAME — full email address for auth
/// - HKASK_SMTP_PASSWORD — email account password
/// - HKASK_CURATOR_EMAIL — from address (default: HKASK_SMTP_USERNAME)
///
/// Returns Ok(()) on success, Err(message) on failure.
pub async fn send_email(to: &str, subject: &str, body: &str) -> EmailResult<()> {
    let server = std::env::var("HKASK_MXROUTE_SERVER")
        .map_err(|_| EmailError::NotConfigured("HKASK_MXROUTE_SERVER not set".into()))?;
    let username = std::env::var("HKASK_SMTP_USERNAME")
        .map_err(|_| EmailError::NotConfigured("HKASK_SMTP_USERNAME not set".into()))?;
    let password = std::env::var("HKASK_SMTP_PASSWORD")
        .map_err(|_| EmailError::NotConfigured("HKASK_SMTP_PASSWORD not set".into()))?;
    let from = std::env::var("HKASK_CURATOR_EMAIL").unwrap_or_else(|_| username.clone());

    let payload = serde_json::json!({
        "server": server,
        "username": username,
        "password": password,
        "from": from,
        "to": to,
        "subject": subject,
        "body": body,
    });

    let response = email_client()
        .post("https://smtpapi.mxroute.com/")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| EmailError::ApiRequest(e.to_string()))?;

    if !response.status().is_success() {
        return Err(EmailError::ApiStatus(response.status().as_u16()));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| EmailError::ApiRequest(format!("parse error: {e}")))?;

    if result["success"].as_bool() != Some(true) {
        let msg = result["message"].as_str().unwrap_or("unknown error");
        return Err(EmailError::ApiError(msg.to_string()));
    }

    tracing::info!(
        target = "cns.email.sent",
        to = %to,
        subject = %subject,
        "CNS"
    );
    Ok(())
}

/// Send an invite email to a prospective member.
///
/// Includes the invite code, acceptance link, and server info.
pub async fn send_invite_email(
    to_email: &str,
    to_name: &str,
    invite_code: &str,
) -> EmailResult<()> {
    let domain = std::env::var("HKASK_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    let scheme = if domain == "localhost" {
        "http"
    } else {
        "https"
    };
    let accept_url = format!("{scheme}://{domain}/api/v1/auth/accept-invite?code={invite_code}");

    let subject = format!("You're invited to hKask — {domain}");
    let body = format!(
        r#"<h2>Welcome to hKask, {to_name}!</h2>
<p>You've been invited to join the hKask server at <strong>{domain}</strong>.</p>

<p>To get started:</p>
<ol>
  <li>Click the link below to accept your invite</li>
  <li>Sign in with your GitHub account</li>
  <li>Create your replicant — your personal AI agent</li>
  <li>Join the team chat and start collaborating</li>
</ol>

<p style="margin: 20px 0;">
  <a href="{accept_url}" style="background:#238636;color:#fff;padding:14px 32px;
     border-radius:8px;text-decoration:none;font-weight:600;display:inline-block;">
     Accept Invitation
  </a>
</p>

<p style="color:#8b949e;font-size:0.9rem;">
  Or copy this link: <code>{accept_url}</code>
</p>

<p style="color:#8b949e;font-size:0.85rem;margin-top:24px;">
  Your invite code: <strong>{invite_code}</strong><br>
  This code expires in 7 days.
</p>

<hr style="border:none;border-top:1px solid #30363d;margin:24px 0;">
<p style="color:#8b949e;font-size:0.8rem;">
  Sent by the hKask Curator · <a href="https://hkask.org">hkask.org</a>
</p>"#,
        to_name = to_name,
        domain = domain,
        accept_url = accept_url,
        invite_code = invite_code,
    );

    send_email(to_email, &subject, &body).await
}
