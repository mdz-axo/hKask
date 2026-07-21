//! Matrix registration — Conduit homeserver account creation and health management.

use hkask_keystore::Keychain;
use hkask_services_core::{DomainKind, ErrorKind, ServiceError};
use hkask_types::agent_registry::UserProfile;

use super::OnboardingService;

/// Result of Matrix account registration during onboarding.
#[derive(Debug, Clone)]
pub struct MatrixRegistrationResult {
    /// Full Matrix user ID for the human (e.g., "@alice-smith:localhost").
    pub human_user_id: String,
    /// Full Matrix user ID for the replicant (e.g., "@assistant-rsmith-bot:localhost").
    pub replicant_user_id: String,
}

// ── Matrix helpers ──────────────────────────────────────────────────────

/// Derive Matrix username localparts from display and replicant names.
///
/// Uses the same formula as OnboardingService::register_oauth_matrix_accounts.
/// Returns (human_localpart, replicant_localpart) for use in Matrix APIs.
pub fn derive_matrix_localparts(display_name: &str, userpod_name: &str) -> (String, String) {
    let sanitize = |s: &str| -> String {
        s.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    };
    let (first, last) = {
        let mut parts = display_name.splitn(2, ' ');
        let f = sanitize(parts.next().unwrap_or("user"));
        let l = sanitize(parts.next().unwrap_or("user"));
        (f, l)
    };
    let human_localpart = format!("{}-{}", first, last);
    let replicant_localpart = format!("{}-bot", sanitize(userpod_name).replace(' ', "-"));
    (human_localpart, replicant_localpart)
}

/// Derive a Matrix username from the human's UserProfile.
/// Format: "@firstname-lastname:localhost" (lowercase, hyphenated).
fn matrix_username_from_human(profile: &UserProfile) -> String {
    let first = profile.first_name.to_lowercase();
    let last = profile.last_name.to_lowercase();
    format!("{}-{}", first, last)
}

/// Derive a Matrix username from the replicant's display name.
/// Format: "@displayname-bot:localhost" (lowercase, hyphenated, " r" → "-r").
fn matrix_username_from_replicant(display_name: &str) -> String {
    let slug = display_name.to_lowercase().replace(' ', "-");
    format!("{}-bot", slug)
}

/// Register a user on a Conduit homeserver via the Matrix API.
///
/// POST /_matrix/client/v3/register with username, password, and
/// m.login.registration_token auth. The registration token is read from
/// the HKASK_MATRIX_REGISTRATION_TOKEN env var (default: "hkask-dev").
///
/// The Curator (@curator:localhost) is the Matrix admin and manages
/// account creation, deletion, and moderation on the server.
/// System bots auto-register during bootstrap using this function.
///
/// Returns the full Matrix user ID on success (e.g., "@alice-smith:localhost").
async fn register_on_conduit(
    homeserver_url: &str,
    localpart: &str,
    password: &str,
) -> Result<String, ServiceError> {
    let url = format!(
        "{}/_matrix/client/v3/register",
        homeserver_url.trim_end_matches('/')
    );

    let registration_token = std::env::var("HKASK_MATRIX_REGISTRATION_TOKEN")
        .unwrap_or_else(|_| "hkask-dev".to_string());

    let body = serde_json::json!({
        "username": localpart,
        "password": password,
        "initial_device_display_name": "hKask",
        "auth": {"type": "m.login.registration_token", "token": registration_token}
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("HTTP request failed: {}", e);
            ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;

    let status = response.status();
    let response_body: serde_json::Value = response.json().await.map_err(|e| {
        let msg = format!("Failed to parse response: {}", e);
        ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Infrastructure,
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    if !status.is_success() {
        let error_msg = response_body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown error");
        return Err(ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Infrastructure,
            source: None,
            message: format!(
                "Registration failed (HTTP {}): {}",
                status.as_u16(),
                error_msg
            ),
        });
    }

    let user_id = response_body
        .get("user_id")
        .and_then(|u| u.as_str())
        .ok_or_else(|| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Infrastructure,
            source: None,
            message: "Response missing user_id field".to_string(),
        })?;

    Ok(user_id.to_string())
}

/// Attempt to recover a Conduit container that is stopped or missing.
///
/// Checks container state before acting:
/// - Running → no action needed (caller's health poll handles the wait)
/// - Stopped → try `docker start` / `podman start`
/// - Missing → run `conduit-docker.sh start` to create and start
/// - No container runtime → return false
async fn try_conduit_recovery() -> bool {
    use std::process::Command;

    tracing::info!(target: "hkask.communication.matrix.recovery", "Attempting Conduit container recovery");

    // Check whether a hkask-conduit container matches (docker or podman).
    // `show_all` includes stopped containers; without it we only see running ones.
    let container_matches = |show_all: bool| -> bool {
        let args: &[&str] = if show_all {
            &[
                "ps",
                "-a",
                "--format",
                "{{.Names}}",
                "--filter",
                "name=hkask-conduit",
            ]
        } else {
            &[
                "ps",
                "--format",
                "{{.Names}}",
                "--filter",
                "name=hkask-conduit",
            ]
        };
        for runtime in &["docker", "podman"] {
            if let Ok(out) = Command::new(runtime).args(args).output()
                && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
            {
                return true;
            }
        }
        false
    };

    // Container is already running — nothing to do.
    if container_matches(false) {
        tracing::info!(target: "hkask.communication.matrix.recovery", "Conduit container already running — awaiting health check");
        return true;
    }

    // Container exists but is stopped — try to start it.
    if container_matches(true) {
        for runtime in &["docker", "podman"] {
            if let Ok(out) = Command::new(runtime)
                .args(["start", "hkask-conduit"])
                .output()
                && out.status.success()
            {
                tracing::info!(target: "hkask.communication.matrix.recovery", runtime = runtime, "Started existing hkask-conduit container");
                return true;
            }
        }
        tracing::warn!(target: "hkask.communication.matrix.recovery", "Container exists but could not be started");
        return false;
    }

    // Container doesn't exist — run the setup script to create and start it.
    let script_candidates = [
        "scripts/conduit/conduit-docker.sh",
        "../scripts/conduit/conduit-docker.sh",
    ];
    for candidate in &script_candidates {
        if std::path::Path::new(candidate).exists()
            && let Ok(out) = Command::new("bash").args([candidate, "start"]).output()
            && out.status.success()
        {
            tracing::info!(
                target: "hkask.communication.matrix.recovery",
                script = %candidate,
                "Conduit started via conduit-docker.sh"
            );
            return true;
        }
    }

    tracing::warn!(target: "hkask.communication.matrix.recovery", "All Conduit recovery attempts failed — no container runtime or script available");
    false
}

/// Ensure Conduit is healthy, attempting recovery if needed.
///
/// 1. Check health via `/_matrix/client/versions`
/// 2. If unhealthy, attempt container recovery
/// 3. Wait up to 30s for Conduit to become healthy
/// 4. Return whether Conduit is now healthy
///
/// \[P9\] Constraining: Homeostatic Self-Regulation — the system heals its own transport.
/// pre:  homeserver_url must be a valid HTTP URL
/// post: returns true if Conduit is healthy (either already was, or recovered); false if recovery failed
pub async fn conduit_ensure_healthy(homeserver_url: &str) -> bool {
    if conduit_health_check(homeserver_url).await {
        return true;
    }

    tracing::warn!(
        target: "hkask.communication.matrix.recovery",
        url = %homeserver_url,
        "Conduit unhealthy — attempting recovery"
    );

    try_conduit_recovery().await;

    // Wait for Conduit to become healthy (up to 30 attempts, 1s apart)
    for attempt in 1..=30 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if conduit_health_check(homeserver_url).await {
            tracing::info!(
                target: "hkask.communication.matrix.recovery",
                attempt = attempt,
                "Conduit recovered and healthy"
            );
            return true;
        }
    }

    tracing::error!(
        target: "hkask.communication.matrix.recovery",
        url = %homeserver_url,
        "Conduit recovery failed after 30s — container may need manual intervention"
    );
    false
}

/// Check whether the Conduit homeserver is healthy and responding.
///
/// Performs a GET to `/_matrix/client/versions`. Returns `true` if the
/// server responds with a successful HTTP status.
///
/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  homeserver_url must be a valid HTTP URL
/// post: returns true if server responds with 2xx; false on connection error or non-2xx status
pub async fn conduit_health_check(homeserver_url: &str) -> bool {
    // P9: CNS span
    tracing::info!(target: "hkask.onboarding", operation = "conduit_health_check", url = %homeserver_url, "CNS");
    let url = format!(
        "{}/_matrix/client/versions",
        homeserver_url.trim_end_matches('/')
    );

    match reqwest::Client::new().get(&url).send().await {
        Ok(response) => {
            let healthy = response.status().is_success();
            if healthy {
                tracing::debug!(
                    target: "hkask.communication.matrix.health",
                    url = %homeserver_url,
                    "Conduit healthy"
                );
            } else {
                tracing::warn!(
                    target: "hkask.communication.matrix.health",
                    url = %homeserver_url,
                    status = %response.status().as_u16(),
                    "Conduit responded with error status"
                );
            }
            healthy
        }
        Err(e) => {
            tracing::warn!(
                target: "hkask.communication.matrix.health",
                url = %homeserver_url,
                error = %e,
                "Conduit unreachable"
            );
            false
        }
    }
}

/// Ensure Conduit is healthy, or store a pending-recovery marker for later retry.
///
/// Called by Matrix account registration functions. If Conduit is unreachable
/// and recovery fails, stores `matrix-pending-recovery` + homeserver URL in the
/// keychain so `retry_pending_matrix` can retry on the next `kask chat` session.
///
/// On success, clears any stale pending-recovery marker.
async fn ensure_conduit_or_store_pending(
    homeserver_url: &str,
    error_detail: &str,
) -> Result<(), ServiceError> {
    if conduit_ensure_healthy(homeserver_url).await {
        let _ = Keychain::default()
            .delete_by_key(hkask_types::keychain_keys::KEY_MATRIX_PENDING_RECOVERY);
        return Ok(());
    }

    let keychain = Keychain::default();
    let _ = keychain.store_by_key(
        hkask_types::keychain_keys::KEY_MATRIX_PENDING_RECOVERY,
        "true",
    );
    let _ = keychain.store_by_key(
        hkask_types::keychain_keys::KEY_MATRIX_PENDING_HOMESERVER,
        homeserver_url,
    );
    Err(ServiceError::Domain {
        kind: ErrorKind::BadRequest,
        domain: DomainKind::Infrastructure,
        source: None,
        message: format!(
            "Conduit at {} is unreachable and recovery failed{}",
            homeserver_url, error_detail
        ),
    })
}

impl OnboardingService {
    /// Register Matrix accounts for the human user and their replicant on
    /// the local Conduit homeserver.
    ///
    /// Called during onboarding after replicant registration succeeds.
    /// Creates two accounts:
    /// - Human: `@firstname-lastname:localhost`
    /// - Replicant: `@displayname-bot:localhost`
    ///
    /// Both use the master passphrase as their initial password.
    /// Credentials are stored in the OS keychain.
    ///
    /// Returns the created user IDs for display in the onboarding summary.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  user_profile must have first_name and last_name; replicant_display_name must be non-empty; passphrase must be non-empty; homeserver_url must be valid
    /// post: returns MatrixRegistrationResult with human and replicant user IDs; credentials stored in keychain; Err(Matrix) on registration failure
    pub async fn register_matrix_accounts(
        user_profile: &UserProfile,
        replicant_display_name: &str,
        passphrase: &str,
        homeserver_url: &str,
    ) -> Result<MatrixRegistrationResult, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "hkask.onboarding", operation = "register_matrix_accounts", replicant = %replicant_display_name, "CNS");

        // ── Ensure Conduit is healthy before attempting registration ──
        ensure_conduit_or_store_pending(
            homeserver_url,
            ". Start it manually: ./scripts/conduit/conduit-docker.sh start",
        )
        .await?;
        let human_username = matrix_username_from_human(user_profile);
        let replicant_username = matrix_username_from_replicant(replicant_display_name);

        // Register human account
        let human_id = register_on_conduit(homeserver_url, &human_username, passphrase)
            .await
            .map_err(|e| {
                let msg = format!("Human account registration failed: {}", e);
                ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })?;

        // Register replicant account
        let replicant_id = register_on_conduit(homeserver_url, &replicant_username, passphrase)
            .await
            .map_err(|e| {
                let msg = format!("Replicant account registration failed: {}", e);
                ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })?;

        // Store credentials in keychain
        let keychain = Keychain::default();
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_MATRIX_HUMAN_USERNAME,
                &human_id,
            )
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: "Failed to store matrix-human-username".into(),
            })?;
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_MATRIX_HUMAN_PASSWORD,
                passphrase,
            )
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: "Failed to store matrix-human-password".into(),
            })?;
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_USERNAME,
                &replicant_id,
            )
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: "Failed to store matrix-replicant-username".into(),
            })?;
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_PASSWORD,
                passphrase,
            )
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: "Failed to store matrix-replicant-password".into(),
            })?;

        tracing::info!(
            target: "hkask.communication.matrix.onboarding",
            human = %human_id,
            replicant = %replicant_id,
            "Matrix accounts registered during onboarding"
        );

        Ok(MatrixRegistrationResult {
            human_user_id: human_id,
            replicant_user_id: replicant_id,
        })
    }

    /// Register Matrix accounts for an OAuth user (web sign-in path).
    ///
    /// Unlike `register_matrix_accounts` (CLI path), this:
    /// - Splits a display_name (e.g., "Alice Smith") into first/last
    /// - Generates its own passphrase (OAuth users don't have one)
    /// - Is non-blocking-friendly (returns errors instead of blocking on health checks)
    ///
    /// Returns the Matrix user IDs for both accounts.
    ///
    /// \[P5\] Motivating: Essentialism — single call from auth module, no raw Matrix logic leaked.
    /// pre:  display_name is non-empty; userpod_name is non-empty; homeserver_url is valid
    /// post: returns MatrixRegistrationResult with human and replicant user IDs; credentials stored in keychain
    pub async fn register_oauth_matrix_accounts(
        display_name: &str,
        userpod_name: &str,
        homeserver_url: &str,
    ) -> Result<MatrixRegistrationResult, ServiceError> {
        // Split display name into first/last for Matrix username derivation
        let (first_name, last_name) = {
            let mut parts = display_name.splitn(2, ' ');
            let first = parts.next().unwrap_or("user");
            let last = parts.next().unwrap_or("user");
            (first.to_string(), last.to_string())
        };
        let human_localpart = format!("{}-{}", first_name.to_lowercase(), last_name.to_lowercase());
        let replicant_localpart =
            format!("{}-bot", userpod_name.to_lowercase().replace(' ', "-"));

        let passphrase = uuid::Uuid::new_v4().to_string();

        // Register human account
        let human_id = register_on_conduit(homeserver_url, &human_localpart, &passphrase)
            .await
            .map_err(|e| {
                let msg = format!("Human Matrix registration failed: {e}");
                ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })?;

        // Register replicant account
        let replicant_id =
            register_on_conduit(homeserver_url, &replicant_localpart, &passphrase).await?;

        // Store credentials in keychain
        let keychain = Keychain::default();
        let _ = keychain.store_by_key(
            hkask_types::keychain_keys::KEY_MATRIX_HUMAN_USERNAME,
            &human_id,
        );
        let _ = keychain.store_by_key(
            hkask_types::keychain_keys::KEY_MATRIX_HUMAN_PASSWORD,
            &passphrase,
        );
        let _ = keychain.store_by_key(
            hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_USERNAME,
            &replicant_id,
        );
        let _ = keychain.store_by_key(
            hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_PASSWORD,
            &passphrase,
        );

        tracing::info!(
            target = "cns.communication.matrix.oauth",
            human = %human_id,
            replicant = %replicant_id,
            "CNS"
        );

        Ok(MatrixRegistrationResult {
            human_user_id: human_id,
            replicant_user_id: replicant_id,
        })
    }

    /// Register a single replicant Matrix account on Conduit.
    ///
    /// Used by `kask onboard` when adding replicants to an existing installation.
    /// The human account already exists; only the replicant account is created.
    /// Uses a generated UUID password (replicant auth is daemon-managed, not human-facing).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence.
    /// pre:  display_name must be non-empty; homeserver_url must be valid and reachable
    /// post: returns the full Matrix user ID on success; Err(Matrix) on registration failure
    pub async fn register_replicant_matrix_account(
        display_name: &str,
        homeserver_url: &str,
    ) -> Result<String, ServiceError> {
        // ── Ensure Conduit is healthy ──
        ensure_conduit_or_store_pending(homeserver_url, "").await?;

        let localpart = display_name.to_lowercase().replace(' ', "-");
        let full_username = format!("@{}-bot:localhost", localpart);
        let password = uuid::Uuid::new_v4().to_string();

        register_on_conduit(homeserver_url, &format!("{}-bot", localpart), &password).await?;

        let keychain = Keychain::default();
        let _ = keychain.store_by_key(
            &format!(
                "{}{}",
                hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_PREFIX,
                display_name
            ),
            &password,
        );

        tracing::info!(
            target: "hkask.communication.matrix.onboarding",
            replicant = %full_username,
            "Replicant Matrix account registered"
        );

        Ok(full_username)
    }

    /// Register Matrix accounts for system bots (Curator, 7R7) on Conduit.
    ///
    /// Called during bootstrap. Creates accounts with generated passwords
    /// stored in the OS keychain. These are passive listeners — they monitor
    /// rooms and escalate via CNS, not active chat participants.
    ///
    /// Returns the created user IDs keyed by bot name.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  homeserver_url must be valid and reachable
    /// post: returns Hash`Map<String, String>` of bot_name → user_id for successfully registered bots; failed registrations are silently skipped
    pub async fn register_system_accounts(
        homeserver_url: &str,
    ) -> Result<std::collections::HashMap<String, String>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "hkask.onboarding", operation = "register_system_accounts", "CNS");
        let system_bots = ["curator"];

        let mut registered = std::collections::HashMap::new();
        let keychain = Keychain::default();

        for bot_name in &system_bots {
            let localpart = format!("hkask-{}", bot_name);
            let password = uuid::Uuid::new_v4().to_string();

            match register_on_conduit(homeserver_url, &localpart, &password).await {
                Ok(user_id) => {
                    keychain
                        .store_by_key(
                            &format!(
                                "{}{}",
                                hkask_types::keychain_keys::KEY_MATRIX_BOT_PREFIX,
                                bot_name
                            ),
                            &password,
                        )
                        .map_err(|e| ServiceError::Domain {
                            kind: ErrorKind::BadRequest,
                            domain: DomainKind::Infrastructure,
                            source: Some(Box::new(e)),
                            message: format!("Failed to store matrix-bot-{}", bot_name),
                        })?;
                    tracing::info!(
                        target: "hkask.communication.matrix.bootstrap",
                        bot = %bot_name,
                        user_id = %user_id,
                        "System bot Matrix account registered"
                    );
                    registered.insert(bot_name.to_string(), user_id);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.communication.matrix.bootstrap",
                        bot = %bot_name,
                        error = %e,
                        "Failed to register system bot Matrix account — Conduit may not be running"
                    );
                }
            }
        }

        Ok(registered)
    }
}
