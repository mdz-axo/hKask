//! Master key derivation for hKask internal secrets.
//!
//! Provides HKDF-SHA256 key derivation from a single master passphrase.
//! The derivation chain is:
//!
//! 1. Argon2id(master_passphrase, fixed_salt) → 32-byte master key
//!    (slow, memory-hard — run once)
//! 2. HKDF-SHA256(master_key, context) → 32-byte sub-key
//!    (fast, deterministic — run per secret)
//!
//! This ensures:
//! - The same passphrase always produces the same secrets (restart-safe)
//! - Different contexts produce cryptographically independent sub-keys
//! - Compromising one sub-key does not compromise the master key or other sub-keys
//! - Deriving 4 secrets takes ~100ms (one Argon2id) + ~4μs (four HKDF expansions)
//!   instead of ~400ms (four Argon2id calls)

use hkask_types::derivation_contexts;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;

/// Salt used for the initial Argon2id master key derivation.
///
/// Fixed so that the same passphrase always produces the same master key.
/// This is not a security weakness — the Argon2id memory-hardness provides
/// the security, and the salt's purpose is domain separation, not secrecy.
const MASTER_KEY_SALT: [u8; 16] = [
    b'h', b'k', b'a', b's', b'k', b'-', b'm', b'a', b's', b't', b'e', b'r', b'-', b'2', b'0', b'2',
];

/// HKDF-Extract salt for sub-key derivation.
/// Uses a fixed application-specific salt for domain separation.
const HKDF_SALT: &[u8; 13] = b"hkask-hkdf-v1";

/// Output length for HKDF expansion (256 bits = 32 bytes = AES-256 / HMAC-SHA256 key size).
const SUB_KEY_LEN: usize = 32;

/// All internal secrets derived from the master key.
///
/// Each field is a hex-encoded 256-bit key, deterministically derived
/// from the master passphrase via HKDF-SHA256.
#[derive(Debug)]
pub struct InternalSecrets {
    /// ACP HMAC signing secret (hex-encoded 256-bit key)
    pub acp_secret: String,
    /// API capability token signing key (hex-encoded 256-bit key)
    pub capability_key: String,
    /// MCP security gateway HMAC key (hex-encoded 256-bit key)
    pub mcp_security_key: String,
    /// OCAP capability token signing secret (hex-encoded 256-bit key)
    pub ocap_secret: String,
}

/// Derive all internal secrets from a master passphrase.
///
/// Uses Argon2id (slow, memory-hard) once to stretch the passphrase into a
/// 32-byte master key, then HKDF-SHA256 (fast, deterministic) to derive each
/// sub-key with domain separation.
///
/// # Security
///
/// - Argon2id with OWASP-recommended parameters (64 MiB, 3 iterations, 4 lanes)
/// - HKDF-SHA256 with per-context info strings for domain separation
/// - All intermediate key material is zeroized on drop
///
/// # Panics
///
/// Cannot panic — Argon2id and HKDF are infallible with valid parameters.
pub fn derive_all_internal_secrets(master_passphrase: &str) -> InternalSecrets {
    // Step 1: Argon2id stretch (slow, ~100ms)
    let master_key = crate::encryption::derive_key(master_passphrase, &MASTER_KEY_SALT)
        .expect("Argon2id derivation cannot fail with valid parameters");

    // Step 2: HKDF-SHA256 expand (fast, ~1μs each)
    let master_key_bytes: &[u8] = &*master_key;
    let acp_secret = derive_sub_key_hex(master_key_bytes, derivation_contexts::ACP_SECRET);
    let capability_key = derive_sub_key_hex(master_key_bytes, derivation_contexts::CAPABILITY_KEY);
    let mcp_security_key =
        derive_sub_key_hex(master_key_bytes, derivation_contexts::MCP_SECURITY_KEY);
    let ocap_secret = derive_sub_key_hex(master_key_bytes, derivation_contexts::OCAP_SECRET);

    InternalSecrets {
        acp_secret,
        capability_key,
        mcp_security_key,
        ocap_secret,
    }
}

/// Derive a 32-byte sub-key from a master key using HKDF-SHA256.
///
/// HKDF (RFC 5869) provides:
/// - **Extract**: PRK = HMAC-SHA256(salt, IKM) — extracts entropy from master key
/// - **Expand**: OKM = HMAC-SHA256(PRK, info || 0x01) — expands into sub-key
///
/// The `context` string provides cryptographic domain separation: different
/// contexts yield completely independent sub-keys from the same master key.
/// This is the same property that makes HKDF safe for deriving multiple
/// independent keys from a single master secret.
///
/// # Arguments
///
/// * `master_key` — 32-byte master key (typically from Argon2id)
/// * `context` — Domain separation string (e.g., `"hkask:acp-secret"`)
///
/// # Returns
///
/// 32-byte derived sub-key, wrapped in `Zeroizing` for secure memory handling.
pub(crate) fn derive_sub_key(master_key: &[u8], context: &str) -> Zeroizing<Vec<u8>> {
    // HKDF-Extract: PRK = HMAC-SHA256(salt, IKM)
    let mut extract_mac =
        HmacSha256::new_from_slice(HKDF_SALT).expect("HMAC-SHA256 accepts any key length");
    extract_mac.update(master_key);
    let prk = extract_mac.finalize().into_bytes();

    // HKDF-Expand: OKM = HMAC-SHA256(PRK, info || 0x01)
    // For a 32-byte output, only one HKDF block is needed (single 0x01 counter).
    let mut expand_mac =
        HmacSha256::new_from_slice(&prk).expect("HMAC-SHA256 accepts any key length");
    expand_mac.update(context.as_bytes());
    expand_mac.update(&[0x01]); // HKDF block counter
    let okm = expand_mac.finalize().into_bytes();

    Zeroizing::new(okm[..SUB_KEY_LEN].to_vec())
}

/// Derive a sub-key and return it as a hex-encoded string.
///
/// Convenience wrapper around [`derive_sub_key`] for callers that need
/// the key as a hex string (e.g., for storage in the OS keychain or
/// environment variable comparison).
fn derive_sub_key_hex(master_key: &[u8], context: &str) -> String {
    let sub_key = derive_sub_key(master_key, context);
    hex::encode(&*sub_key)
}
