//! hKask Capability — OCAP delegation token system
//!
//! Ed25519-signed delegation tokens with cryptographic attenuation.
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are Ed25519-signed tokens for inter-agent delegation.

pub mod auth;
pub mod resources;
pub mod token_types;
pub mod tokens;
pub mod verification;

pub use auth::{derive_signing_key, AuthContext};
pub use resources::{
    capabilities_match, capability_from_server_id, CapabilityParseError, CapabilitySpec,
    DelegationAction, DelegationResource,
};
pub use token_types::{
    DelegationToken, DelegationTokenBuilder, SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_RECURSION,
};
pub use tokens::ConsolidationToken;
pub use verification::{
    require_read_access, require_write_access, token_err_insufficient_access,
    token_err_tool_access_denied, verify_delegation_token, verify_delegation_token_now,
    CapabilityChecker, VerificationOutcome, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE,
    TOKEN_ERR_NO_CHECKER,
};
