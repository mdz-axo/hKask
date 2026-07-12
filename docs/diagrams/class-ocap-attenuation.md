---
title: "OCAP Delegation Token Attenuation Chain"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Capability"
mds_categories: [domain, security, lifecycle]
diataxis: "reference"
---

# OCAP Delegation Token Attenuation Chain

## Description

The OCAP (Object Capability) delegation system in `hkask-capability` uses Ed25519-signed `DelegationToken` instances with cryptographic attenuation. A root token at depth 0 is minted by a trusted issuer (e.g., the A2A root authority). Each delegation step calls `attenuate()` / `attenuate_with_expiry()`, incrementing `attenuation_level` and chaining `context_nonce` (e.g. `root-attenuated-uuid-...`). The `CapabilityChecker` verifies token integrity, root trust, holder match, and resource/action alignment. Attenuation is bounded by `SYSTEM_MAX_ATTENUATION` (7) — a token at depth 7 cannot be further attenuated. Each level reduces authority: the attenuated token inherits parent caveats and gains a 1-hour default expiry.

**Key source:** `crates/hkask-capability/src/token_types.rs:17` (`SYSTEM_MAX_ATTENUATION = 7`), `token_types.rs:347-398` (`attenuate`, `attenuate_with_expiry`), `verification/checker.rs:20-33` (`CapabilityChecker`), `resources.rs` (`capabilities_match`).

```mermaid
classDiagram
    class DelegationToken {
        +WebID delegated_from
        +WebID delegated_to
        +DelegationResource resource
        +String resource_id
        +DelegationAction action
        +u8 attenuation_level
        +u8 max_attenuation
        +String context_nonce
        +Vec~Caveat~ caveats
        +Option~i64~ expires_at
        +verify() bool
        +attenuate(WebID, SigningKey, i64) Option~DelegationToken~
        +can_attenuate() bool
        +is_valid_for(DelegationResource, String, DelegationAction) bool
        +is_expired(i64) bool
        +holder() WebID
        +issuer() WebID
        +caveat_ids() Vec~str~
        +verify_attenuation_chain(String, u8) bool
        +grants_resource(DelegationResource) bool
    }

    class CapabilityChecker {
        -Option~SigningKey~ signing_key
        -Vec~Ed25519PublicKey~ trusted_roots
        -bool enforce_roots
        +check(DelegationToken, WebID, DelegationResource, String, DelegationAction) bool
        +verify(DelegationToken) bool
        +verify_with_time(DelegationToken, i64) bool
        +grant(DelegationResource, String, DelegationAction, WebID, WebID) DelegationToken
        +attenuate(DelegationToken, WebID, i64) Option~DelegationToken~
        +grant_tool(String, WebID, WebID) DelegationToken
        +grant_registry(DelegationAction, WebID, WebID) DelegationToken
    }

    class RootToken {
        attenuation_level: 0
        max_attenuation: 7
        "full authority"
    }
    class Depth1 {
        attenuation_level: 1
        "1hr default expiry"
    }
    class Depth2 {
        attenuation_level: 2
    }
    class Depth7Max {
        attenuation_level: 7
        "cannot attenuate further"
    }

    CapabilityChecker --> DelegationToken : verifies
    CapabilityChecker --> DelegationToken : grants/attenuates

    RootToken --> Depth1 : attenuate()
    Depth1 --> Depth2 : attenuate()
    Depth2 --> "..." : attenuate()
    "..." --> Depth7Max : attenuate()

    note for RootToken "Root token minted by\ntrusted issuer (A2A root)"
    note for Depth1 "Authority reduced:\nexpiry enforced\ncaveats inherited"
    note for Depth7Max "Terminal depth:\ncan_attenuate() → false\nSYSTEM_MAX_ATTENUATION = 7"
```

## Attenuation Chain Rules

| Depth | `can_attenuate()` | Expiry Behavior | Context Nonce |
|-------|-------------------|-----------------|---------------|
| 0 (Root) | `true` (0 < 7) | None by default | `root-uuid` |
| 1 | `true` (1 < 7) | Default 1hr from `current_time` | `root-attenuated-uuid1` |
| 2–6 | `true` | Inherits parent expiry if set | Chains `-attenuated-` separator |
| 7 (Max) | **`false`** | N/A — cannot attenuate | N/A |

## Verification Flow

1. **Ed25519 signature check** (`token.verify()`) — verifies self-signature integrity.
2. **Root trust enforcement** (`enforce_roots`) — if enabled, token's `public_key` must be in `trusted_roots`. Fail-closed: empty `trusted_roots` rejects all tokens.
3. **Expiry check** — `token.is_expired(current_time)` gate.
4. **Holder match** — `token.delegated_to == holder`.
5. **Resource + Action match** — `token.is_valid_for(resource, resource_id, action)`.

---

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DC-004
verified_date: 2026-07-01
verified_against: crates/hkask-capability/src/token_types.rs (SYSTEM_MAX_ATTENUATION:17, DelegationToken:51-68, can_attenuate:336-338, attenuate:347-354, attenuate_with_expiry:364-398, verify_attenuation_chain:447-464, is_valid_for:405-412, grants_resource:416-418), crates/hkask-capability/src/verification/checker.rs (CapabilityChecker:20-33, check:155-166, verify:126-134, grant:195-207, attenuate:243-251), crates/hkask-capability/src/resources.rs (capabilities_match)
status: VERIFIED
-->

## Cross-Reference

- [`hKask-architecture-master.md` § OCAP Capability Model](../architecture/hKask-architecture-master.md#ocap-capability-model)
- [`token_types.rs`](crates/hkask-capability/src/token_types.rs) — `DelegationToken`, `SYSTEM_MAX_ATTENUATION`, `attenuate()`, `verify_attenuation_chain()`
- [`verification/checker.rs`](crates/hkask-capability/src/verification/checker.rs) — `CapabilityChecker`, `check()`, `verify()`, `grant()`
- [`resources.rs`](crates/hkask-capability/src/resources.rs) — `capabilities_match()`, `CapabilitySpec`
