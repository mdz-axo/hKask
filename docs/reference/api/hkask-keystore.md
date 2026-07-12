---
title: "hkask-keystore — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-keystore` provides OS keychain integration, AES-256-GCM encryption with Argon2 key derivation, and master key derivation via HKDF. Secrets are stored in the platform keychain, never in plaintext files.

## Public Modules

| Module | Purpose |
|---|---|
| `encryption` | `derive_key` — AES-256-GCM encryption with Argon2id key derivation |
| `error` | `KeystoreError` — unified keystore error type |
| `keychain` | `Keychain`, `KeychainError`, `resolve` — OS keychain integration |
| `master_key` | `derive_all_internal_secrets_with_version` — master key derivation via HKDF |
| `version_file` | Keystore version file management |

## Key Types

### `Keychain`

OS keychain service for secure credential storage. Invariant: secrets are stored in the OS keychain, never in plaintext files.

| Method | Purpose |
|---|---|
| `new(service_name: &str) -> Self` | Create a new Keychain for the given service name |
| `store(webid: &WebID, secret: &str) -> Result<(), KeychainError>` | Store a secret keyed by WebID |

Secrets are keyed under `service_name + webid.uuid`. The keychain backend uses the `keyring` crate to integrate with the platform's native keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager).

### `KeychainError`

| Variant | Source | Purpose |
|---|---|---|
| `Platform(String)` | KeyringError non-NoEntry errors | Platform keychain is unavailable or errored |
| `NotFound(String)` | KeyringError::NoEntry | Secret was not found in the keychain |

Implements `From<KeyringError>` — `NoEntry` maps to `NotFound`, all others to `Platform`.

### `KeystoreError`

Unified error type for all keystore operations. Re-exported from the `error` module.

### `resolve`

Function exposed from `keychain` module for resolving keychain entries by WebID.

## Encryption

### `derive_key`

Derives an AES-256 key from a passphrase using Argon2id with:

| Parameter | Value | Rationale |
|---|---|---|
| **Algorithm** | Argon2id | Hybrid resistance to side-channel and GPU attacks |
| **Memory cost** | 64 MiB (65536 KiB) | OWASP recommendation for high-security |
| **Time cost** | 3 iterations | Balanced for interactive use |
| **Parallelism** | 4 lanes | Matches typical CPU core count |
| **Salt size** | 16 bytes (128 bits) | Standard salt length |
| **Nonce size** | 12 bytes (96 bits) | Standard AES-GCM nonce |

The derived key is used with AES-256-GCM for authenticated encryption. All secret key material uses `Zeroizing` wrappers.

### `EncryptionError`

Non-exhaustive error enum with variants: `KeyDerivation(String)`, `Encryption(String)`, `Decryption(String)`, `InvalidPassphrase`.

## Master Key

### `derive_all_internal_secrets_with_version`

Derives all internal secrets (database passphrase, capability key, MCP secrets) from the master key using HKDF-SHA256. Returns versioned secrets matching the current keystore version.

## Known Keychain Entries

From `hkask_types::keychain_keys`:

| Constant | Purpose |
|---|---|
| `KEY_A2A_SECRET` | A2A root-authority secret |
| `KEY_DB_PASSPHRASE` | Database passphrase |
| `KEY_OCAP_SECRET` | OCAP signing secret |

## Feature Flags

No feature flags are defined. This crate is a core dependency.
