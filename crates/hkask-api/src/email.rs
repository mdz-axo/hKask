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

/// Interaction mode for curator email — tags each message with its cybernetic purpose.
///
/// Each mode closes a different feedback loop:
/// - `Invite` — outbound, one-way (onboarding)
/// - `Alert` — outbound algedonic, closes S1→S5 when the live channel is dead
/// - `Notification` — outbound periodic digest (escalations + reg status)
/// - `Command` — inbound reply → curator MCP tool call (human-in-the-loop)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailMode {
    Invite,
    Alert,
    Notification,
    Command,
}

impl std::fmt::Display for EmailMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invite => write!(f, "invite"),
            Self::Alert => write!(f, "alert"),
            Self::Notification => write!(f, "notification"),
            Self::Command => write!(f, "command"),
        }
    }
}

/// Shared HTTP client for MXroute API calls.
fn email_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Cached IMAP TLS connector — avoids rebuilding the root cert store on every poll.
fn imap_tls_connector() -> tokio_rustls::TlsConnector {
    use std::sync::OnceLock;
    static CONNECTOR: OnceLock<tokio_rustls::TlsConnector> = OnceLock::new();
    CONNECTOR
        .get_or_init(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            tokio_rustls::TlsConnector::from(std::sync::Arc::new(config))
        })
        .clone()
}

/// Extract the text/plain body from a raw RFC 822 message.
/// Falls back to raw UTF-8 lossy conversion if parsing fails.
fn extract_text_body(raw: &[u8]) -> String {
    match mailparse::parse_mail(raw) {
        Ok(parsed) => extract_text_plain_from_mime(&parsed),
        Err(_) => String::from_utf8_lossy(raw).into_owned(),
    }
}

/// Recursively walk the MIME tree to find the first text/plain part.
fn extract_text_plain_from_mime(msg: &mailparse::ParsedMail) -> String {
    if msg.ctype.mimetype == "text/plain" {
        return msg.get_body().map(|(body, _)| body).unwrap_or_default();
    }
    for part in &msg.subparts {
        let text = extract_text_plain_from_mime(part);
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
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
pub async fn send_email(to: &str, subject: &str, body: &str, mode: EmailMode) -> EmailResult<()> {
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
        mode = %mode,
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

    send_email(to_email, &subject, &body, EmailMode::Invite).await
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

    let connector = imap_tls_connector();

    let tcp = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::net::TcpStream::connect((server.as_str(), 993)),
    )
    .await
    .map_err(|_| EmailError::Imap("imap connect timeout (30s)".into()))?
    .map_err(|e| EmailError::Imap(format!("tcp connect: {e}")))?;
    let server_name = rustls::pki_types::ServerName::try_from(server.clone())
        .map_err(|e| EmailError::Imap(format!("server name: {e}")))?;
    let tls_stream = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        connector.connect(server_name, tcp),
    )
    .await
    .map_err(|_| EmailError::Imap("tls handshake timeout (15s)".into()))?
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
            .uid_fetch(&uid_set, "(UID ENVELOPE BODY.PEEK[])")
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
                .map(|b| extract_text_body(b))
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
        drop(fetch_stream);

        // Mark fetched messages as seen (PEEK avoided auto-marking during fetch).
        let _ = session
            .uid_store(&uid_set, "+FLAGS (\\Seen)")
            .await
            .map_err(|e| EmailError::Imap(format!("store seen: {e}")))?;
    }

    if let Err(e) = session.logout().await { tracing::debug!(target: "reg.email.received", error = %e, "IMAP logout failed (non-critical)"); }
    Ok(messages)
}

// ── Alert email sink (S3) ───────────────────────────────────────────────

/// `AlertEmailSink` implementation that sends algedonic alerts via the
/// curator's email channel. Non-blocking — spawns the async send internally
/// so the cybernetics loop is never blocked.
#[derive(Debug)]
pub struct CuratorAlertEmailSink {
    alert_recipient: String,
}

impl CuratorAlertEmailSink {
    /// Create from env: `HKASK_ALERT_EMAIL` or fall back to `HKASK_SMTP_USERNAME`.
    pub fn from_env() -> Self {
        let alert_recipient = std::env::var("HKASK_ALERT_EMAIL")
            .unwrap_or_else(|_| std::env::var("HKASK_SMTP_USERNAME").unwrap_or_default());
        Self { alert_recipient }
    }

    /// Create from env, returning `None` when no recipient is configured.
    pub fn try_from_env() -> Option<std::sync::Arc<dyn hkask_regulation::AlertEmailSink>> {
        let recipient = std::env::var("HKASK_ALERT_EMAIL")
            .or_else(|_| std::env::var("HKASK_SMTP_USERNAME"))
            .ok()?;
        if recipient.is_empty() {
            return None;
        }
        Some(std::sync::Arc::new(Self {
            alert_recipient: recipient,
        }))
    }
}

impl hkask_regulation::AlertEmailSink for CuratorAlertEmailSink {
    fn send_alert_email(&self, alert: &hkask_regulation::RuntimeAlert) {
        if self.alert_recipient.is_empty() {
            tracing::warn!(target: "reg.alert", "Alert email sink has no recipient");
            return;
        }
        let recipient = self.alert_recipient.clone();
        let domain = alert.domain.clone();
        let deficit = alert.deficit;
        let threshold = alert.threshold;
        let message = alert.message.clone();
        let subject = format!("[hKask Alert] {domain} variety deficit {deficit}/{threshold}");
        let body = format!(
            "<h2>Algedonic Alert</h2>\n<p><b>Domain:</b> {domain}</p>\n<p><b>Deficit:</b> {deficit} / {threshold}</p>\n<p><b>Message:</b> {message}</p>\n<p style='color:#8b949e;font-size:0.8rem'>Sent by the hKask Curator cybernetics loop</p>"
        );
        tokio::spawn(async move {
            if let Err(e) = send_email(&recipient, &subject, &body, EmailMode::Alert).await {
                tracing::warn!(target: "reg.alert", error = %e, "Failed to send alert email");
            }
        });
    }
}

// Inbound command parsing (S4) + polling task (S7)

/// A parsed command from an inbound email reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailCommand {
    Resolve { escalation_id: String },
    Dismiss { escalation_id: String },
    Unknown,
}

/// Parse an email body for a command verb (case-insensitive, first match wins).
#[must_use]
pub fn parse_command(body: &str) -> EmailCommand {
    for line in body.lines() {
        let line = line.trim().to_lowercase();
        if let Some(rest) = line.strip_prefix("resolve ") {
            return EmailCommand::Resolve {
                escalation_id: rest.trim().to_string(),
            };
        }
        if let Some(rest) = line.strip_prefix("dismiss ") {
            return EmailCommand::Dismiss {
                escalation_id: rest.trim().to_string(),
            };
        }
    }
    EmailCommand::Unknown
}

/// Check if a sender is authorized (P12). Reads `HKASK_AUTHORIZED_EMAILS`.
#[must_use]
pub fn is_authorized_sender(from: &str) -> bool {
    let allowlist = std::env::var("HKASK_AUTHORIZED_EMAILS").unwrap_or_default();
    !allowlist.is_empty() && allowlist.split(',').any(|e| e.trim() == from)
}

/// Spawn a background IMAP poller that calls `handler` per received email.
pub fn spawn_inbox_poller<F>(interval_secs: u64, handler: F)
where
    F: Fn(InboundEmail, EmailCommand) + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            match fetch_unread().await {
                Ok(messages) => {
                    for msg in messages {
                        let authorized = is_authorized_sender(&msg.from);
                        let cmd = parse_command(&msg.body);
                        if authorized {
                            handler(msg, cmd);
                        } else {
                            tracing::warn!(target = "reg.email.received", from = %msg.from, "Unauthorized sender - command ignored (P12)");
                        }
                    }
                }
                Err(EmailError::NotConfigured(_)) => {}
                Err(e) => {
                    tracing::warn!(target = "reg.email.received", error = %e, "IMAP poll failed")
                }
            }
        }
    });
}

/// Wire the inbox poller to an `AgentService`, dispatching email commands to
/// the governance layer (resolve/dismiss escalations). The poller is a no-op
/// when IMAP is not configured.
///
/// Call after `AgentService::build_with_email()` returns.
pub fn wire_inbox_poller(ctx: &hkask_services_context::AgentService, poll_interval_secs: u64) {
    let escalations = std::sync::Arc::clone(&ctx.governance().escalations);
    let events = ctx.governance().events.clone();
    let userpod = ctx.config().user_name.clone();

    spawn_inbox_poller(poll_interval_secs, move |msg, cmd| match cmd {
        EmailCommand::Resolve { escalation_id } => {
            match hkask_services_context::governance::resolve_direct(
                &escalations,
                &events,
                &escalation_id,
                &userpod,
            ) {
                Ok(()) => tracing::info!(
                    target = "reg.email.received",
                    from = %msg.from,
                    id = %escalation_id,
                    "Escalation resolved via email command"
                ),
                Err(e) => tracing::warn!(
                    target = "reg.email.received",
                    error = %e,
                    id = %escalation_id,
                    "Email resolve command failed"
                ),
            }
        }
        EmailCommand::Dismiss { escalation_id } => {
            match hkask_services_context::governance::dismiss_direct(
                &escalations,
                &events,
                &escalation_id,
                &userpod,
            ) {
                Ok(()) => tracing::info!(
                    target = "reg.email.received",
                    from = %msg.from,
                    id = %escalation_id,
                    "Escalation dismissed via email command"
                ),
                Err(e) => tracing::warn!(
                    target = "reg.email.received",
                    error = %e,
                    id = %escalation_id,
                    "Email dismiss command failed"
                ),
            }
        }
        EmailCommand::Unknown => {
            tracing::debug!(
                target = "reg.email.received",
                from = %msg.from,
                "No command parsed from email"
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Placeholder — real tests below.

    #[test]
    fn parse_command_resolve() {
        assert_eq!(
            parse_command("resolve abc-123"),
            EmailCommand::Resolve {
                escalation_id: "abc-123".into()
            }
        );
    }

    #[test]
    fn parse_command_dismiss() {
        assert_eq!(
            parse_command("dismiss xyz-789"),
            EmailCommand::Dismiss {
                escalation_id: "xyz-789".into()
            }
        );
    }

    #[test]
    fn parse_command_case_insensitive() {
        assert_eq!(
            parse_command("RESOLVE ID-1"),
            EmailCommand::Resolve {
                escalation_id: "id-1".into()
            }
        );
    }

    #[test]
    fn parse_command_multiline_body() {
        let body = "Thanks for the alert.\n\nresolve esc-42\nLet me know when done.";
        assert_eq!(
            parse_command(body),
            EmailCommand::Resolve {
                escalation_id: "esc-42".into()
            }
        );
    }

    #[test]
    fn parse_command_unknown() {
        assert_eq!(
            parse_command("Hello, just checking in."),
            EmailCommand::Unknown
        );
        assert_eq!(parse_command(""), EmailCommand::Unknown);
    }

    #[test]
    fn email_mode_display() {
        assert_eq!(EmailMode::Invite.to_string(), "invite");
        assert_eq!(EmailMode::Alert.to_string(), "alert");
        assert_eq!(EmailMode::Notification.to_string(), "notification");
        assert_eq!(EmailMode::Command.to_string(), "command");
    }
}
