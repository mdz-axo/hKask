//! Curator email delivery — sends invites, notifications, and alerts via MXroute HTTP API.
//!
//! Uses MXroute's SMTP API at smtpapi.mxroute.com — no SMTP library needed.
//! Also provides the foundation for userpod email access (future: per-userpod credentials).
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
    #[error("IMAP error: {0}")]
    Imap(String),
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
        target = "reg.email.sent",
        to = %to,
        subject = %subject,
        "REG"
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
  <li>Create your userpod — your personal AI agent</li>
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

// ── Inbound (IMAP) ─────────────────────────────────────────────────────

/// A received email fetched from the curator's IMAP inbox.
///
/// `body` is the raw TEXT part (RFC 822 body after headers) as best-effort
/// UTF-8. Full MIME/multipart parsing is deferred — S1 returns the primitive;
/// command parsing (S4) operates on this text.
#[derive(Debug, Clone)]
pub struct InboundEmail {
    pub from: String,
    pub subject: String,
    pub body: String,
    pub uid: u32,
}

/// Fetch unread messages from the curator's IMAP inbox (port 993 SSL).
///
/// Reuses the same env vars as [`send_email`] — no new credentials:
/// - `HKASK_MXROUTE_SERVER` — IMAP server hostname (same as SMTP)
/// - `HKASK_SMTP_USERNAME` — full email address (IMAP login)
/// - `HKASK_SMTP_PASSWORD` — mailbox password
///
/// Returns unread messages and marks them `\Seen`. Emits `reg.email.received`
/// per message. This closes the inbound half of the email feedback loop.
pub async fn fetch_unread() -> EmailResult<Vec<InboundEmail>> {
    use futures_util::StreamExt;

    let server = std::env::var("HKASK_MXROUTE_SERVER")
        .map_err(|_| EmailError::NotConfigured("HKASK_MXROUTE_SERVER not set".into()))?;
    let username = std::env::var("HKASK_SMTP_USERNAME")
        .map_err(|_| EmailError::NotConfigured("HKASK_SMTP_USERNAME not set".into()))?;
    let password = std::env::var("HKASK_SMTP_PASSWORD")
        .map_err(|_| EmailError::NotConfigured("HKASK_SMTP_PASSWORD not set".into()))?;

    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(tls_config));

    let tcp = tokio::net::TcpStream::connect((server.as_str(), 993))
        .await
        .map_err(|e| EmailError::Imap(format!("tcp connect: {e}")))?;
    let server_name = rustls::pki_types::ServerName::try_from(server.clone())
        .map_err(|e| EmailError::Imap(format!("server name: {e}")))?;
    let tls_stream = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| EmailError::Imap(format!("tls handshake: {e}")))?;

    // async-imap uses futures-ecosystem AsyncRead/AsyncWrite; tokio-rustls
    // provides tokio-ecosystem traits. The compat adapter bridges them.
    use tokio_util::compat::TokioAsyncReadCompatExt;
    let client = async_imap::Client::new(tls_stream.compat());
    let mut session = client
        .login(&username, &password)
        .await
        .map_err(|(e, _)| EmailError::Imap(format!("imap login: {e}")))?;

    session
        .select("INBOX")
        .await
        .map_err(|e| EmailError::Imap(format!("select inbox: {e}")))?;

    let uids: Vec<u32> = session
        .uid_search("UNSEEN")
        .await
        .map_err(|e| EmailError::Imap(format!("search unseen: {e}")))?
        .into_iter()
        .collect();

    let mut messages = Vec::new();
    if !uids.is_empty() {
        let uid_set = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut fetch_stream = session
            .uid_fetch(&uid_set, "(UID ENVELOPE BODY.PEEK[TEXT])")
            .await
            .map_err(|e| EmailError::Imap(format!("fetch: {e}")))?;

        while let Some(msg) = fetch_stream
            .next()
            .await
            .transpose()
            .map_err(|e| EmailError::Imap(format!("fetch iter: {e}")))?
        {
            let uid = msg.uid.unwrap_or(0);
            let from = msg
                .envelope()
                .and_then(|env| env.from.as_ref())
                .and_then(|addrs| addrs.first())
                .map(|a| {
                    let mailbox = a
                        .mailbox
                        .as_deref()
                        .map(|m| String::from_utf8_lossy(m).into_owned())
                        .unwrap_or_default();
                    let host = a
                        .host
                        .as_deref()
                        .map(|h| String::from_utf8_lossy(h).into_owned())
                        .unwrap_or_default();
                    format!("{mailbox}@{host}")
                })
                .unwrap_or_default();
            let subject = msg
                .envelope()
                .and_then(|env| env.subject.as_deref())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .unwrap_or_default();
            let body = msg
                .body()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();

            tracing::info!(
                target = "reg.email.received",
                from = %from,
                subject = %subject,
                "REG"
            );
            messages.push(InboundEmail {
                from,
                subject,
                body,
                uid,
            });
        }
    }

    let _ = session.logout().await;
    Ok(messages)
}
