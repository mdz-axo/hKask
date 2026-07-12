---
title: "hkask-capability — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-capability` provides the OCAP (Object-Capability) delegation token system with Ed25519-signed tokens and cryptographic attenuation. Two token kinds exist: **loop authority tokens** (ZST tokens proving loop-authorized operations) and **delegation tokens** (Ed25519-signed tokens for inter-agent delegation).

## Public Modules

| Module | Purpose |
|---|---|
| `auth` | `AuthContext` and `derive_signing_key` — authentication context and signing key derivation |
| `resources` | `DelegationResource`, `DelegationAction`, `CapabilitySpec`, `CapabilityParseError`, `capabilities_match`, `capability_from_server_id` |
| `token_types` | `DelegationToken`, `DelegationTokenBuilder`, `CapabilityError`, `TokenRegistry`, `NoOpTokenRegistry`, `TokenRegistryError`, `SYSTEM_MAX_ATTENUATION`, `SYSTEM_MAX_RECURSION` |
| `tokens` | ZST loop authority tokens for internal loop-authorized operations |
| `verification` | `CapabilityChecker`, `VerificationOutcome`, `require_read_access`, `require_write_access`, `verify_delegation_token`, `verify_delegation_token_now`, error constants |

## Key Types

### `DelegationToken`

Ed25519-signed OCAP token for inter-agent capability delegation. Fields:

| Field | Type | Purpose |
|---|---|---|
| `id` | `String` | Unique token identifier |
| `resource` | `DelegationResource` | Target resource descriptor |
| `resource_id` | `String` | Specific resource instance ID |
| `action` | `DelegationAction` | Permitted action (read/write/execute) |
| `delegated_from` | `WebID` | Issuer identity |
| `delegated_to` | `WebID` | Recipient identity |
| `signature` | `TokenSignature` | Ed25519 signature over the token payload (64 bytes, hex-encoded) |
| `public_key` | `Ed25519PublicKey` | Ed25519 public key for signature verification |
| `expires_at` | `Option<i64>` | Unix epoch expiration; `None` = no expiry |
| `attenuation_level` | `u8` | 0 = full authority; increases with each delegation step |
| `max_attenuation` | `u8` | Hard cap on attenuation depth |
| `context_nonce` | `String` | Nonce binding token to a specific invocation context |
| `caveats` | `Vec<Caveat>` | Additive restrictions on the capability |

### `DelegationTokenBuilder`

Builder pattern for constructing and signing `DelegationToken` instances. Supports setting all token fields, then `.build(signing_key)` to produce a signed token.

### `TokenSignature`

Wraps a 64-byte Ed25519 signature (`[u8; 64]`). Serialized via `hex::serde` for JSON transport. Verification uses the token's `public_key` field — no shared secret is required.

### `Caveat`

Additive restriction on a capability token. Fields: `caveat_id` (`String`), `data` (`String`). Caveats narrow token scope; they cannot expand it.

### `AuthContext`

Verified authentication context carrying the caller's identity and capability token. Both API middleware and CLI keystore produce this type. Fields: `token` (`Option<DelegationToken>`) — `Some` for capability-token auth, `None` for session-cookie auth — and `webid` (`WebID`). Constructed via `AuthContext::from_session(webid)` or `AuthContext::from_token(token, webid)`.

### `CapabilityChecker`

Token verification facility from `verification` module. Validates signature, expiry, and attenuation constraints. Exposes constants `TOKEN_ERR_EXPIRED`, `TOKEN_ERR_INVALID_SIGNATURE`, `TOKEN_ERR_NO_CHECKER`.

### `VerificationOutcome`

Result of token verification indicating pass/fail and reason.

### `TokenRegistry`

Trait for token storage and lifecycle. `NoOpTokenRegistry` provides a pass-through implementation for contexts where token persistence is not required.

### `DelegationResource` / `DelegationAction`

Resource and action descriptors for capability specification. `capabilities_match()` checks whether a `CapabilitySpec` satisfies a required resource+action pair. `capability_from_server_id()` constructs a `CapabilitySpec` from a server identifier.

## Constants

| Constant | Value | Purpose |
|---|---|---|
| `SYSTEM_MAX_RECURSION` | `7` | Shared structural bound for capability attenuation, cascade depth, and subgoal nesting |
| `SYSTEM_MAX_ATTENUATION` | `SYSTEM_MAX_RECURSION` (7) | Capability-domain alias for maximum attenuation |

## Key Functions

| Function | Signature | Purpose |
|---|---|---|
| `require_read_access` | `(checker, token, resource_id) -> bool` | Verify read access for a resource |
| `require_write_access` | `(checker, token, resource_id) -> bool` | Verify write access for a resource |
| `verify_delegation_token` | `(token, checker) -> VerificationOutcome` | Full token verification (signature + expiry + attenuation) |
| `verify_delegation_token_now` | `(token, checker) -> VerificationOutcome` | Verify with current time as reference |
| `derive_signing_key` | `(secret: &[u8]) -> SigningKey` | Derive Ed25519 signing key from arbitrary bytes (SHA-256 → 32-byte seed) |
| `capabilities_match` | See `resources` module | Check capability spec against a requirement |
| `capability_from_server_id` | See `resources` module | Construct capability spec from server identifier |

## Error Types

### `CapabilityError`

Enum with a single variant: `Other(String)` — a generic error wrapper for capability-domain failures.

### `TokenRegistryError`

Token registry persistence errors.

### `CapabilityParseError`

Error parsing capability specifications from strings.

## Feature Flags

No feature flags are defined. This crate is a core dependency.
