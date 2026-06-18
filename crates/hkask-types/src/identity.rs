//! Human identity and authentication types — Loop 6 (Cybernetics): Access Guard
//!
//! Cybernetics subloop 6.1 (Access Guard) governs who can access what.
//! Human users, replicant identities, and sessions are verified at this boundary.

use hkask_rsolidity::contract;

use crate::WebID;
use crate::id::UserID;
use crate::wallet::WalletId;
use serde::{Deserialize, Serialize};

/// User role for multi-user access control.
///
/// expect: "I can assign roles to users to control what they can access" [P1]
/// [P1] Goal: User Sovereignty — roles enforce who can manage the server
/// [P2] Constraining: Affirmative Consent — admin role is explicitly granted, never default
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full administrative access: invite, sessions, config, all realms.
    Admin,
    /// Standard user access: own replicants, own sessions, own resources.
    Member,
}

impl Default for Role {
    fn default() -> Self {
        Role::Member
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => f.write_str("admin"),
            Role::Member => f.write_str("member"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            other => Err(format!("Unknown role: {other}")),
        }
    }
}

/// OAuth identity provider for human user sign-in.
///
/// expect: "System types preserve semantic identity and are provenance-aware" [P8]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    GitHub,
    Google,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::GitHub => f.write_str("github"),
            OAuthProvider::Google => f.write_str("google"),
        }
    }
}

impl std::str::FromStr for OAuthProvider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(OAuthProvider::GitHub),
            "google" => Ok(OAuthProvider::Google),
            other => Err(format!("Unknown OAuth provider: {other}")),
        }
    }
}

/// Loop: Cybernetics
/// Human user — owns contact info (email, phone for recovery only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanUser {
    pub user_id: UserID,
    pub email_enc: Vec<u8>,
    pub phone_enc: Option<Vec<u8>>,
    pub passphrase_hash: String,
    pub salt: String,
    pub master_salt: String,
    pub created_at: i64,
    pub last_active: Option<i64>,
    pub passphrase_set_at: Option<i64>,
    /// User role for access control (defaults to Member).
    /// expect: "Every user has a role that controls their access level" [P1]
    pub role: Role,
    /// OAuth provider used for sign-in (None = passphrase-only registration).
    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    pub oauth_provider: Option<OAuthProvider>,
    /// External user ID from the OAuth provider (e.g., GitHub user ID).
    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    pub oauth_provider_user_id: Option<String>,
    /// Display name from the OAuth provider (e.g., GitHub username).
    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    pub oauth_display_name: Option<String>,
}

impl HumanUser {
    #[contract(id = "P1-multi-role-field", principle = "P1")]
    pub fn new(
        email_enc: Vec<u8>,
        phone_enc: Option<Vec<u8>>,
        passphrase_hash: String,
        salt: String,
        master_salt: String,
    ) -> Self {
        Self {
            user_id: UserID::new(),
            email_enc,
            phone_enc,
            passphrase_hash,
            salt,
            master_salt,
            created_at: chrono::Utc::now().timestamp(),
            last_active: None,
            passphrase_set_at: None,
            oauth_provider: None,
            oauth_provider_user_id: None,
            oauth_display_name: None,
            role: Role::Member,
        }
    }
}

/// Loop: Cybernetics
/// Replicant identity — the in-system persona users log in AS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicantIdentity {
    pub replicant_name: String,
    pub user_id: UserID,
    pub replicant_webid: WebID,
    /// Wallet ID for rJoule payments. Created during replicant registration.
    /// Each replicant gets their own wallet for deposit/withdrawal isolation.
    pub wallet_id: Option<WalletId>,
    pub first_name_enc: Vec<u8>,
    pub last_name_enc: Vec<u8>,
    pub persona_yaml: Option<String>,
    pub is_primary: bool,
    pub created_at: i64,
    pub last_login: Option<i64>,
}

impl ReplicantIdentity {
    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  replicant_name is a non-empty string (1–64 alphanumeric/hyphen/underscore chars)
    /// post: returns a deterministic WebID with the "replicant" namespace;
    ///       same replicant_name always produces the same WebID
    #[contract(id = "P1-multi-role-type", principle = "P1")]
    pub fn derive_webid(replicant_name: &str) -> WebID {
        WebID::from_persona_with_namespace(replicant_name.as_bytes(), "replicant")
    }

    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  replicant_name is non-empty; user_id is a valid UserID;
    ///       first_name_enc and last_name_enc are encrypted byte vectors
    /// post: returns a ReplicantIdentity with derived webid, wallet_id=None,
    ///       persona_yaml=None, created_at set to current Unix timestamp,
    ///       last_login=None
    pub fn new(
        replicant_name: String,
        user_id: UserID,
        first_name_enc: Vec<u8>,
        last_name_enc: Vec<u8>,
        is_primary: bool,
    ) -> Self {
        Self {
            replicant_webid: Self::derive_webid(&replicant_name),
            replicant_name,
            user_id,
            wallet_id: None,
            first_name_enc,
            last_name_enc,
            persona_yaml: None,
            is_primary,
            created_at: chrono::Utc::now().timestamp(),
            last_login: None,
        }
    }
}

/// Loop: Cybernetics
/// Active user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub session_id: String,
    pub replicant_name: String,
    pub replicant_webid: WebID,
    pub user_id: UserID,
    pub session_key_salt: String,
    pub expires_at: i64,
    pub last_active: i64,
}

impl UserSession {
    /// expect: "System types preserve semantic identity and are provenance-aware" [P8]
    /// pre:  now is a Unix timestamp (i64); self.expires_at is a valid
    ///       expiry timestamp set at session creation
    /// post: returns true if now > self.expires_at (session has expired);
    ///       returns false if now <= self.expires_at (session still valid)
    pub fn is_expired(&self, now: i64) -> bool {
        now > self.expires_at
    }
}

/// Invite status for multi-user onboarding.
///
/// expect: "I can send invites to bring other users onto my server" [P2]
/// [P2] Goal: Affirmative Consent — invite requires explicit admin action
/// [P1] Constraining: User Sovereignty — invite is scoped to a specific server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

impl std::fmt::Display for InviteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InviteStatus::Pending => f.write_str("pending"),
            InviteStatus::Accepted => f.write_str("accepted"),
            InviteStatus::Revoked => f.write_str("revoked"),
            InviteStatus::Expired => f.write_str("expired"),
        }
    }
}

impl std::str::FromStr for InviteStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(InviteStatus::Pending),
            "accepted" => Ok(InviteStatus::Accepted),
            "revoked" => Ok(InviteStatus::Revoked),
            "expired" => Ok(InviteStatus::Expired),
            other => Err(format!("Unknown invite status: {other}")),
        }
    }
}

/// Multi-user invitation record.
///
/// expect: "I can track who was invited and whether they've joined" [P2]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub invite_id: String,
    pub created_by: UserID,
    pub code: String,
    pub status: InviteStatus,
    pub created_at: i64,
    pub expires_at: i64,
    pub accepted_at: Option<i64>,
    pub accepted_user_id: Option<UserID>,
}

/// Loop: Cybernetics
/// Registration request for new replicant identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub replicant_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub passphrase: String,
}

#[derive(Debug, Clone, thiserror::Error)]
/// Loop: Cybernetics
///
/// Variants are grouped by shared recovery path:
/// - `InvalidReplicantName` — \[NORMATIVE\] name must be 1–64 alphanumeric/hyphen/underscore chars (P6 — Space for Replicants).
/// - `EmptyName` — required name field is missing
/// - `InvalidContact` — email or phone format is wrong
/// - `InvalidPassphrase` — passphrase doesn't meet requirements
pub enum RegistrationError {
    #[error("Invalid replicant name: must be 1-64 chars, alphanumeric with hyphens/underscores")]
    InvalidReplicantName,
    #[error("Required name field is empty")]
    EmptyName,
    #[error("Invalid contact information format")]
    InvalidContact,
    #[error("Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case")]
    InvalidPassphrase,
}
