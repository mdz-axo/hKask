# Public Surface Justification — hkask-keystore

**Crate:** `hkask-keystore`  
**Public items in lib.rs:** 11  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-keystore` is the **OS keychain integration** — AES-256-GCM encrypted secret storage with HKDF-SHA256 key derivation. Its surface is large because it provides both storage and derivation:

1. **Keychain operations** — `set`, `get`, `delete`, `load` (bulk) for the OS keychain.
2. **Secret resolution** — `resolve()` for `SecretRef` types used by wallet, OCAP, and storage.
3. **Key derivation** — HKDF-SHA256 domain-separated key derivation for per-agent secrets.
4. **KeystoreError** — Error type for keychain failures.

## Mitigations

- **Single backend:** Currently Linux-only (secret-service), keeping the implementation surface small.
- **SecretRef abstraction:** Consumers use `SecretRef` rather than raw keychain access.

## Deletion Test

Delete `hkask-keystore` and encrypted secret storage, key derivation, and provider API key management reappear in every crate that needs secure credential storage. The crate earns its existence.
