//! Behavioral contract tests for hkask-keystore.
//!
//! Covering encryption/decryption, key derivation, internal secrets,
//! and Ed25519 spec signing. OS keychain operations are excluded —
//! these tests are deterministic and require no keychain or network.

use hkask_keystore::{
    Ed25519SpecSigner, derive_all_internal_secrets_with_version, derive_key,
    encryption::{EncryptionError, EncryptionService},
    keychain::resolve_db_passphrase,
    master_key::InternalSecrets,
    spec_signer::SpecSignatureError,
};

// ---------------------------------------------------------------------------
// 1. EncryptionService encrypt/decrypt roundtrip
// ---------------------------------------------------------------------------

#[test]
fn encrypt_decrypt_roundtrip() {
    let passphrase = "roundtrip-test-passphrase";
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new(passphrase, &salt).expect("create service");

    let plaintext = b"The quick brown fox jumps over the lazy dog";
    let ciphertext = svc.encrypt(plaintext).expect("encrypt");
    let decrypted = svc.decrypt(&ciphertext).expect("decrypt");

    assert_eq!(
        decrypted, plaintext,
        "decrypted text must match original plaintext"
    );
}

#[test]
fn encrypt_decrypt_empty_plaintext() {
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new("empty-test-passphrase", &salt).expect("create service");

    let ciphertext = svc.encrypt(b"").expect("encrypt empty");
    let decrypted = svc.decrypt(&ciphertext).expect("decrypt empty");

    assert_eq!(decrypted, b"", "empty roundtrip must produce empty output");
}

#[test]
fn encrypt_decrypt_binary_data() {
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new("binary-test-passphrase", &salt).expect("create service");

    let plaintext: Vec<u8> = (0..=255).collect();
    let ciphertext = svc.encrypt(&plaintext).expect("encrypt binary");
    let decrypted = svc.decrypt(&ciphertext).expect("decrypt binary");

    assert_eq!(
        decrypted, plaintext,
        "binary roundtrip must preserve all 256 byte values"
    );
}

// ---------------------------------------------------------------------------
// 2. EncryptionService tampered ciphertext rejection
// ---------------------------------------------------------------------------

#[test]
fn decrypt_rejects_tampered_ciphertext() {
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new("tamper-test", &salt).expect("create service");

    let plaintext = b"sensitive data";
    let mut ciphertext = svc.encrypt(plaintext).expect("encrypt");

    // Flip a byte in the encrypted portion (after the 12-byte nonce)
    let flip_index = ciphertext.len() / 2;
    ciphertext[flip_index] ^= 0x01;

    let result = svc.decrypt(&ciphertext);
    assert!(
        result.is_err(),
        "tampered ciphertext must be rejected (AES-GCM auth tag)"
    );
}

#[test]
fn decrypt_rejects_truncated_ciphertext() {
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new("truncation-test", &salt).expect("create service");

    let plaintext = b"some data";
    let ciphertext = svc.encrypt(plaintext).expect("encrypt");

    // Feed only the 12-byte nonce — not enough for nonce + auth tag
    let too_short = &ciphertext[..12];
    let result = svc.decrypt(too_short);
    assert!(
        result.is_err(),
        "truncated ciphertext (< 12 bytes) must be rejected"
    );
}

// ---------------------------------------------------------------------------
// 3. EncryptionService wrong key rejection
// ---------------------------------------------------------------------------

#[test]
fn decrypt_rejects_wrong_key() {
    let salt = EncryptionService::generate_salt();
    let svc_a = EncryptionService::new("passphrase-alpha", &salt).expect("create svc A");
    let svc_b = EncryptionService::new("passphrase-bravo", &salt).expect("create svc B");

    let plaintext = b"top secret";
    let ciphertext = svc_a.encrypt(plaintext).expect("encrypt with A");

    let result = svc_b.decrypt(&ciphertext);
    assert!(
        result.is_err(),
        "decrypt with a different key must fail (AES-GCM auth tag)"
    );
}

// ---------------------------------------------------------------------------
// 4. derive_key properties
// ---------------------------------------------------------------------------

#[test]
fn derive_key_deterministic() {
    let passphrase = "deterministic-test";
    let salt = EncryptionService::generate_salt();

    let k1 = derive_key(passphrase, &salt).expect("first derivation");
    let k2 = derive_key(passphrase, &salt).expect("second derivation");

    assert_eq!(
        &*k1, &*k2,
        "same passphrase + salt must produce the same key"
    );
}

#[test]
fn derive_key_salt_dependent() {
    let passphrase = "salt-test-passphrase";
    let salt_a = EncryptionService::generate_salt();
    let salt_b = EncryptionService::generate_salt();

    // Salts are random — in the extremely unlikely event they collide, retry
    if salt_a == salt_b {
        return; // skip; astronomically improbable
    }

    let k1 = derive_key(passphrase, &salt_a).expect("derivation with salt A");
    let k2 = derive_key(passphrase, &salt_b).expect("derivation with salt B");

    assert_ne!(&*k1, &*k2, "different salts must produce different keys");
}

#[test]
fn derive_key_rejects_empty_passphrase() {
    let salt = EncryptionService::generate_salt();
    let result = derive_key("", &salt);
    assert!(
        matches!(result, Err(EncryptionError::InvalidPassphrase)),
        "empty passphrase must be rejected with InvalidPassphrase"
    );
}

// ---------------------------------------------------------------------------
// 5. resolve_db_passphrase env precedence
// ---------------------------------------------------------------------------

#[test]
fn resolve_db_passphrase_from_env() {
    // SAFETY: test-only env var mutation.
    unsafe {
        std::env::set_var("HKASK_DB_PASSPHRASE", "test-db-passphrase");
    }
    let passphrase = resolve_db_passphrase().expect("resolve from env");
    assert_eq!(&*passphrase, b"test-db-passphrase");
}

// ---------------------------------------------------------------------------
// 6. derive_all_internal_secrets field independence
// ---------------------------------------------------------------------------

#[test]
fn internal_secrets_all_fields_present() {
    let secrets: InternalSecrets =
        derive_all_internal_secrets_with_version("field-presence-test", 1);

    // master_key_hex is 64 hex chars for 32 bytes
    assert!(
        !secrets.master_key_hex.is_empty(),
        "master_key_hex must be non-empty"
    );
    assert_eq!(
        secrets.master_key_hex.len(),
        64,
        "master_key_hex must be 64 hex chars (32 bytes)"
    );
    assert!(
        secrets
            .master_key_hex
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "master_key_hex must be valid hex"
    );

    // a2a_secret
    assert!(
        !secrets.a2a_secret.is_empty(),
        "a2a_secret must be non-empty"
    );
    assert_eq!(
        secrets.a2a_secret.len(),
        64,
        "a2a_secret must be 64 hex chars (32 bytes)"
    );
    assert!(
        secrets.a2a_secret.chars().all(|c| c.is_ascii_hexdigit()),
        "a2a_secret must be valid hex"
    );

    // capability_key
    assert!(
        !secrets.capability_key.is_empty(),
        "capability_key must be non-empty"
    );
    assert_eq!(
        secrets.capability_key.len(),
        64,
        "capability_key must be 64 hex chars (32 bytes)"
    );
    assert!(
        secrets
            .capability_key
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "capability_key must be valid hex"
    );

    // mcp_security_key
    assert!(
        !secrets.mcp_security_key.is_empty(),
        "mcp_security_key must be non-empty"
    );
    assert_eq!(
        secrets.mcp_security_key.len(),
        64,
        "mcp_security_key must be 64 hex chars (32 bytes)"
    );
    assert!(
        secrets
            .mcp_security_key
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "mcp_security_key must be valid hex"
    );

    // ocap_secret
    assert!(
        !secrets.ocap_secret.is_empty(),
        "ocap_secret must be non-empty"
    );
    assert_eq!(
        secrets.ocap_secret.len(),
        64,
        "ocap_secret must be 64 hex chars (32 bytes)"
    );
    assert!(
        secrets.ocap_secret.chars().all(|c| c.is_ascii_hexdigit()),
        "ocap_secret must be valid hex"
    );
}

#[test]
fn internal_secrets_fields_distinct() {
    let secrets: InternalSecrets =
        derive_all_internal_secrets_with_version("distinct-fields-test", 1);

    // All sub-secrets must be distinct from each other (cryptographic domain separation)
    assert_ne!(
        secrets.a2a_secret, secrets.capability_key,
        "a2a_secret must differ from capability_key"
    );
    assert_ne!(
        secrets.a2a_secret, secrets.mcp_security_key,
        "a2a_secret must differ from mcp_security_key"
    );
    assert_ne!(
        secrets.a2a_secret, secrets.ocap_secret,
        "a2a_secret must differ from ocap_secret"
    );
    assert_ne!(
        secrets.capability_key, secrets.mcp_security_key,
        "capability_key must differ from mcp_security_key"
    );
    assert_ne!(
        secrets.capability_key, secrets.ocap_secret,
        "capability_key must differ from ocap_secret"
    );
    assert_ne!(
        secrets.mcp_security_key, secrets.ocap_secret,
        "mcp_security_key must differ from ocap_secret"
    );

    // Also distinct from master_key_hex
    assert_ne!(
        secrets.a2a_secret, secrets.master_key_hex,
        "a2a_secret must differ from master_key_hex"
    );
    assert_ne!(
        secrets.capability_key, secrets.master_key_hex,
        "capability_key must differ from master_key_hex"
    );
    assert_ne!(
        secrets.mcp_security_key, secrets.master_key_hex,
        "mcp_security_key must differ from master_key_hex"
    );
    assert_ne!(
        secrets.ocap_secret, secrets.master_key_hex,
        "ocap_secret must differ from master_key_hex"
    );
}

// ---------------------------------------------------------------------------
// 6. Ed25519SpecSigner sign/verify
// ---------------------------------------------------------------------------

fn canonical_test_spec() -> Vec<u8> {
    // A minimal canonical JSON payload (sorted keys, no whitespace).
    br#"{"id":"test-spec","name":"test","version":1}"#.to_vec()
}

fn modified_spec() -> Vec<u8> {
    br#"{"id":"test-spec","name":"malicious","version":1}"#.to_vec()
}

fn make_signer() -> Ed25519SpecSigner {
    let master_secret: [u8; 32] = [
        0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0xaa,
        0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc,
        0xba, 0x98,
    ];
    Ed25519SpecSigner::from_master_secret(&master_secret)
}

#[test]
fn sign_verify_roundtrip() {
    let signer = make_signer();
    let canonical_json = canonical_test_spec();

    let hex_sig = signer.sign_spec(&canonical_json);
    assert_eq!(
        hex_sig.len(),
        128,
        "signature must be 128 hex chars (64 bytes)"
    );

    let result = signer.verify_spec(&canonical_json, &hex_sig);
    assert!(
        result.is_ok(),
        "signature must verify against the original canonical JSON"
    );
}

#[test]
fn verify_rejects_modified_payload() {
    let signer = make_signer();
    let original_json = canonical_test_spec();
    let modified_json = modified_spec();

    let hex_sig = signer.sign_spec(&original_json);

    let result = signer.verify_spec(&modified_json, &hex_sig);
    assert!(
        matches!(result, Err(SpecSignatureError::VerificationFailed)),
        "modified payload must fail verification"
    );
}

#[test]
fn verify_rejects_invalid_hex_signature() {
    let signer = make_signer();
    let canonical_json = canonical_test_spec();

    let result = signer.verify_spec(&canonical_json, "not-valid-hex-XXXX");
    assert!(
        matches!(result, Err(SpecSignatureError::InvalidHex)),
        "invalid hex signature must be rejected with InvalidHex"
    );
}

// ---------------------------------------------------------------------------
// 7. Nonce uniqueness
// ---------------------------------------------------------------------------

#[test]
fn encrypt_produces_different_ciphertext_each_time() {
    let salt = EncryptionService::generate_salt();
    let svc = EncryptionService::new("nonce-uniqueness-test", &salt).expect("create service");

    let plaintext = b"same plaintext, different nonce";

    let ct1 = svc.encrypt(plaintext).expect("first encrypt");
    let ct2 = svc.encrypt(plaintext).expect("second encrypt");

    assert_ne!(
        ct1, ct2,
        "each encryption must use a unique nonce, producing different ciphertext"
    );

    // Sanity: both must decrypt successfully
    let d1 = svc.decrypt(&ct1).expect("decrypt ct1");
    let d2 = svc.decrypt(&ct2).expect("decrypt ct2");
    assert_eq!(d1, plaintext);
    assert_eq!(d2, plaintext);
}
