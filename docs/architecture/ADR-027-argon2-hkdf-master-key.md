---
title: "ADR-027: Argon2id + HKDF-SHA256 Master Key Derivation"
audience: [architects, security engineers]
last_updated: 2026-05-29
version: "1.0.0"
status: "Active"
domain: "Technology"
ddmvss_categories: [trust]
---

# ADR-027: Argon2id + HKDF-SHA256 Master Key Derivation

**Date:** 2026-05-29 (retroactive)  
**Status:** Implemented  
**Supersedes:** ADR-023 (archived — expanded derivation chain rationale now in this ADR)

## Context

hKask requires multiple internal secrets (ACP signing key, capability token key, MCP security key, OCAP secret). Generating each independently creates four failure modes: (1) random secrets change on restart, invalidating all issued tokens; (2) users must manage four separate passphrases; (3) no cryptographically guaranteed independence between secrets; (4) key storage requires four OS keychain entries.

## Decision

**Single master passphrase → Argon2id → 256-bit master key → HKDF-SHA256 → four independent sub-keys.**

```
Master passphrase (user-provided)
  │
  ├── Argon2id(passphrase, fixed_salt) → 256-bit master key  [~100ms, memory-hard]
  │
  ├── HKDF-SHA256(master_key, "hkask:acp-secret")     → ACP HMAC signing secret
  ├── HKDF-SHA256(master_key, "hkask:capability-key")  → API capability token key
  ├── HKDF-SHA256(master_key, "hkask:mcp-security-key") → MCP security gateway key
  └── HKDF-SHA256(master_key, "hkask:ocap-secret")     → OCAP signing secret
```

Resolution priority: `SecretRef::Derived` (preferred) → `SecretRef::Env` → `SecretRef::Keychain` → `SecretRef::Generated` (salts/nonces only).

## Rationale

1. **Argon2id memory-hardness.** [^argon2] Argon2id with OWASP parameters (64 MiB, 3 iterations, 4 lanes) resists GPU and ASIC attacks. The ~100ms derivation cost is paid once at startup.

2. **HKDF cryptographic independence.** [^hkdf] HKDF-SHA256 (RFC 5869) provides cryptographic domain separation: different `info` strings yield completely independent sub-keys. Compromising one sub-key (e.g., the MCP security key) reveals nothing about the others or the master key.

3. **Restart safety.** The same passphrase always produces the same secrets. Previously issued capability tokens remain valid across process restarts and pod migrations.

4. **Schneier principle.** [^schneier-secrets] "Secrets should be derived, not generated." Generating random secrets on each boot silently invalidates all previously issued tokens — a failure mode that manifests as mysterious "invalid token" errors rather than a clear key management error.

5. **One secret to remember.** The user provides one passphrase. All internal secrets derive from it. This reduces key management surface from 4 secrets to 1.

## Consequences

### Positive

- Same passphrase → same secrets (restart-safe, cluster-safe)
- Compromise of one sub-key does not compromise the master or other sub-keys
- One passphrase to manage instead of four
- ~100ms startup cost (one Argon2id) + ~4μs (four HKDF expansions) vs ~400ms (four Argon2id calls)

### Negative

- Changing the master passphrase invalidates all derived secrets (by design — this is key rotation)
- Argon2id with 64 MiB memory cost is non-trivial on constrained devices
- `SecretRef::Generated` is prohibited in production code paths (salts and nonces only)

## Compliance

| Principle | Compliance |
|-----------|-----------|
| C5 (Every error variant is unique recovery path) | ✅ `KeystoreError::NotFound`, `NotSupported`, `KeychainError` |
| Schneier (Zero-trust) | ✅ No secrets in source, all derived from passphrase |

## References

[^argon2]: Biryukov, A., Dinu, D., & Khovratovich, D. (2016). *Argon2: The Memory-Hard Function for Password Hashing*. IETF.
[^hkdf]: Krawczyk, H., & Eronen, P. (2010). *HMAC-based Extract-and-Expand Key Derivation Function (HKDF)*. RFC 5869.
[^schneier-secrets]: Schneier, B. (2015). *Secrets and Lies: Digital Security in a Networked World*. Wiley. "Secrets should be generated infrequently and checked carefully."

---

*ℏKask - A Minimal Viable Container for Agents — ADR-027 — v0.23.0*
