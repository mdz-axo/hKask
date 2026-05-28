---
title: "ADR-023: Master Key Derivation via HKDF-SHA256"
audience: [architects, security engineers, developers]
last_updated: 2026-05-27
version: "0.21.0"
status: "Accepted"
---

# ADR-023: Master Key Derivation via HKDF-SHA256

## Context

Three subsystems (`AcpRuntime`, `SecurityGateway`, `SoapInferenceConfig`) silently generated **random, non-reproducible secrets** when environment variables were not set. This meant:

1. **Restart invalidation** — Every process restart generated new secrets, invalidating all previously-issued capability tokens, HMAC signatures, and sessions.
2. **No cluster safety** — Different processes or nodes could never agree on the same secrets.
3. **Silent failure** — No error or warning was produced; the system appeared to work but tokens were unverifiable after restart.

Additionally, the MCP keystore server (`hkask-mcp-keystore`) stored entries only in-memory (`Arc<RwLock<HashMap<...>>>`), meaning all `keystore_set` entries were lost on restart.

## Decision

### 1. Master Key Derivation (HKDF-SHA256)

Introduce a `SecretRef::Derived` variant that deterministically derives sub-keys from a single master passphrase using HKDF-SHA256 (RFC 5869).

**Derivation chain:**

```
Master passphrase → Argon2id(passphrase, fixed_salt) → 256-bit master key
Master key → HKDF-SHA256(master_key, "hkask:acp-secret")      → ACP secret
Master key → HKDF-SHA256(master_key, "hkask:capability-key")    → Capability key
Master key → HKDF-SHA256(master_key, "hkask:mcp-security-key")  → MCP security key
Master key → HKDF-SHA256(master_key, "hkask:ocap-secret")       → OCAP secret
```

**Resolution priority:**

1. `SecretRef::Derived` — HKDF from master key (preferred, deterministic)
2. `SecretRef::Env` — Direct environment variable (override)
3. `SecretRef::Keychain` — OS keychain (override)
4. `SecretRef::Generated` — Random (only for salts/nonces, never for signing keys)

### 2. Eliminate Random Secret Generation

Replace all `unwrap_or_else(|| random...)` patterns in `AcpRuntime::default()`, `SecurityGateway::default()`, `SoapInferenceConfig::default()`, and `resolve_acp_secret()` with `SecretRef::Derived` as the primary resolution path. Fail with a clear error message instead of silently generating random secrets.

### 3. Keystore Persistence

Replace the in-memory `HashMap` in `KeystoreServer` with a file-backed vault at `~/.hkask/keystore/vault.json` (configurable via `HKASK_KEYSTORE_DIR`). Entries are loaded at startup, saved after each mutation using atomic writes (temp file + rename).

## Consequences

### Positive

- **Restart-safe secrets** — Same passphrase → same secrets, always.
- **Cluster-safe** — Multiple processes sharing `HKASK_MASTER_KEY` produce identical secrets.
- **Single secret to manage** — One master passphrase derives all four internal signing keys.
- **No silent failures** — Missing secrets produce clear error messages instead of random fallbacks.
- **Keystore survives restarts** — `keystore_set` entries are persisted to disk.
- **Performance** — Argon2id runs once (~100ms); HKDF expansions are ~1μs each.

### Negative

- **`HKASK_MASTER_KEY` must be set** — Systems that previously ran with random secrets now require explicit configuration. The `HKASK_INSECURE_DEV` escape hatch in `resolve_acp_secret()` preserves local development ergonomics.
- **Vault file permissions** — The vault.json file contains encrypted entries but the OS keychain is still recommended for sensitive external API keys.
- **Fixed salts** — The Argon2id salt for master key derivation is fixed (`hkask-master-2026`), which is acceptable because the passphrase provides the entropy. This is standard practice for deterministic key derivation.

## Implementation

| Component | File | Change |
|-----------|------|--------|
| `SecretRef::Derived` variant | `crates/hkask-types/src/secret.rs` | New enum variant + `derivation_contexts` module |
| `derive_all_internal_secrets()` | `crates/hkask-keystore/src/master_key.rs` | New module: HKDF-SHA256 derivation |
| `resolve()` extended | `crates/hkask-keystore/src/keychain.rs` | Handle `Derived` variant |
| `AcpRuntime::default()` | `crates/hkask-agents/src/acp.rs` | Derived → Env → Keychain chain |
| `SecurityGateway::default()` | `crates/hkask-mcp/src/security.rs` | Derived → Env → Keychain chain |
| `SoapInferenceConfig::default()` | `crates/hkask-api/src/lib.rs` | Derived → Env → Keychain chain |
| `resolve_acp_secret()` | `crates/hkask-cli/src/commands.rs` | Derived → Env → Keychain → InsecureDev chain |
| `KeystoreServer` persistence | `mcp-servers/hkask-mcp-keystore/src/main.rs` | File-backed vault with atomic writes |

## Verification

```bash
cargo test -p hkask-keystore    # Master key derivation tests
cargo test -p hkask-types       # SecretRef serialization
cargo test -p hkask-agents      # ACP runtime tests
cargo check --workspace          # Full build verification
```