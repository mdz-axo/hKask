//! Canonical keychain key constants — single source of truth.
//!
//! All keychain keys used across hKask are defined here. Using bare string
//! literals for keychain keys in call sites is a P5 violation (duplicated
//! source of truth) and a risk vector — a typo in a keychain key silently
//! breaks authentication at runtime with no compiler feedback.
//!
//! Added 2026-06-21 after audit found 21 distinct keychain keys, all bare strings.

/// Keychain key for the A2A communication secret.
pub const KEY_A2A_SECRET: &str = "a2a-secret";

/// Keychain key for the database passphrase.
pub const KEY_DB_PASSPHRASE: &str = "hkask-db-passphrase";

/// Keychain key for the master passphrase (user-facing credential).
pub const KEY_MASTER_PASSPHRASE: &str = "hkask-master-passphrase";

/// Keychain key for the master key hex (derived from passphrase).
pub const KEY_MASTER_KEY: &str = "HKASK_MASTER_KEY";

/// Keychain key for the default inference provider.
pub const KEY_DEFAULT_PROVIDER: &str = "HKASK_DEFAULT_PROVIDER";

/// Keychain key for Matrix human account username.
pub const KEY_MATRIX_HUMAN_USERNAME: &str = "matrix-human-username";

/// Keychain key for Matrix human account password.
pub const KEY_MATRIX_HUMAN_PASSWORD: &str = "matrix-human-password";

/// Keychain key for Matrix replicant account username.
pub const KEY_MATRIX_REPLICANT_USERNAME: &str = "matrix-replicant-username";

/// Keychain key for Matrix replicant account password.
pub const KEY_MATRIX_REPLICANT_PASSWORD: &str = "matrix-replicant-password";

/// Keychain key for the Matrix bot Curator credentials.
pub const KEY_MATRIX_BOT_CURATOR: &str = "matrix-bot-curator";

/// Keychain key for Matrix pending-recovery flag (set when Conduit was down during onboarding).
pub const KEY_MATRIX_PENDING_RECOVERY: &str = "matrix-pending-recovery";

/// Keychain key for the homeserver URL stored alongside the pending-recovery flag.
pub const KEY_MATRIX_PENDING_HOMESERVER: &str = "matrix-pending-homeserver";

/// Keychain key prefix for per-replicant Matrix credentials (format with display_name).
pub const KEY_MATRIX_REPLICANT_PREFIX: &str = "matrix-replicant-";

/// Keychain key prefix for per-pod Matrix credentials (format with pod_name).
pub const KEY_MATRIX_POD_PREFIX: &str = "matrix-pod-";

/// Keychain key prefix for per-pod Matrix pending-recovery URL (format with pod_name).
pub const KEY_MATRIX_POD_PENDING_PREFIX: &str = "matrix-pod-pending-";

/// Keychain key prefix for Matrix bot credentials (format with bot_name).
pub const KEY_MATRIX_BOT_PREFIX: &str = "matrix-bot-";

/// Keychain key for the OAuth GitHub client ID.
pub const KEY_OAUTH_GITHUB_CLIENT_ID: &str = "hkask-oauth-github-client-id";

/// Keychain key for the OAuth GitHub client secret.
pub const KEY_OAUTH_GITHUB_CLIENT_SECRET: &str = "hkask-oauth-github-client-secret";

/// Keychain key for the MCP security key.
pub const KEY_MCP_SECRET: &str = "mcp-secret";

/// Keychain key for the capability probe (internal diagnostics).
pub const KEY_CAPABILITY_PROBE: &str = "__hkask_capability_probe__";

/// Keychain key prefix for Matrix admin token.
pub const KEY_MATRIX_ADMIN_TOKEN: &str = "HKASK_MATRIX_ADMIN_TOKEN";

/// Keychain key prefix for Matrix agent username (format with agent_name).
pub const KEY_MATRIX_AGENT_USERNAME_PREFIX: &str = "HKASK_MATRIX_AGENT_USERNAME_";

/// Keychain key prefix for Matrix agent password (format with agent_name).
pub const KEY_MATRIX_AGENT_PASSWORD_PREFIX: &str = "HKASK_MATRIX_AGENT_PASSWORD_";
