# hKask Security Model

**Version:** 0.21.0  
**Last Updated:** 2026-05-21

## Overview

hKask implements defense-in-depth security through three layers:

1. **Input Validation** — Schema-based validation for all pod operations
2. **Rate Limiting** — Token bucket algorithm for abuse prevention
3. **OCAP** — Object-capability security with attenuation history tracking

## Input Validation

All agent persona inputs are validated before pod creation:

| Field | Constraints |
|-------|-------------|
| `name` | 1-64 chars, alphanumeric + hyphens + underscores |
| `agent_type` | Must be `Bot` or `Replicant` |
| `version` | 1-32 chars, semver-like format |
| `description` | Max 1000 chars |
| `editor` | 1-256 chars |
| `capabilities` | Max 20 entries, each max 128 chars |

**Example:**

```rust
use hkask_agents::security::{AgentPersonaInput, InputValidator};

let input = AgentPersonaInput {
    name: "my-bot".to_string(),
    agent_type: "bot".to_string(),
    version: "0.1.0".to_string(),
    description: "A test bot".to_string(),
    editor: "developer".to_string(),
    capabilities: vec!["tool:inference".to_string()],
};

input.validate(&input)?; // Returns ValidationResult<()>
```

## Rate Limiting

Pod operations use token bucket rate limiting:

- **Default:** 10 requests burst, 1 request/second refill
- **Per-key:** Rate limits are tracked per operation key (e.g., `pod_creation:{template_name}`)
- **Configurable:** Via `SecurityContext` in `PodManagerBuilder`

**Example:**

```rust
use hkask_agents::pod::PodManagerBuilder;
use hkask_agents::security::{SecurityContext, RateLimiter, ExpiryEnforcer};
use std::time::Duration;

let security_context = SecurityContext {
    rate_limiter: RateLimiter::new(20.0, 2.0), // 20 burst, 2/sec refill
    expiry_enforcer: ExpiryEnforcer::new(Duration::from_secs(7200)), // 2 hour max lifetime
};

let manager = PodManagerBuilder::new()
    .security_context(security_context)
    .build();
```

## OCAP (Object-Capability Security)

### Capability Tokens

All pod operations require valid capability tokens:

- **Cryptographic:** HMAC-SHA256 signatures
- **Attenuation:** Max 7 levels of delegation
- **Expiry:** Configurable lifetime (default 1 hour)
- **Context Nonce:** Traceable delegation chains

### Attenuation History

OCAP tracks capability delegation chains:

```rust
use hkask_agents::ocap::OCAP;
use hkask_types::{CapabilityToken, WebID};

let ocap = OCAP::new();

// Record attenuation event
ocp.record_attenuation(
    "root-nonce",
    &delegated_from,
    &delegated_to,
    timestamp,
    attenuation_level,
).await;

// Verify attenuation chain
let valid = ocap.verify_attenuation(&token).await;

// Create attenuated token with history tracking
let child = ocap.attenuate_with_history(
    &parent,
    new_holder,
    secret,
    current_time,
).await;
```

### Expiry Enforcement

Capability tokens have configurable expiry:

- **Default:** 1 hour max lifetime
- **Attenuated tokens:** Inherit parent's remaining lifetime
- **Enforcement:** Checked on every operation

```rust
use hkask_agents::ocap::OCAP;
use std::time::Duration;

// Custom expiry (2 hours)
let ocap = OCAP::with_expiry(ExpiryEnforcer::new(Duration::from_secs(7200)));

// Check if token is expired
let expired = ocap.is_expired(&token, current_time);

// Validate token expiry
let valid = ocap.validate_expiry(&token, current_time);
```

## Security Context

`SecurityContext` unifies rate limiting and expiry enforcement:

```rust
use hkask_agents::security::SecurityContext;

// Default configuration
let ctx = SecurityContext::default();

// Custom configuration
let ctx = SecurityContext::new(
    RateLimiter::new(10.0, 1.0),
    ExpiryEnforcer::new(Duration::from_secs(3600)),
);

// Access from PodManager
let manager = PodManager::new_mock();
let ctx = manager.security_context();
```

## Error Handling

Security violations return `ValidationError`:

```rust
use hkask_agents::security::ValidationError;

match result {
    Ok(_) => { /* Success */ }
    Err(ValidationError::InvalidInput(msg)) => { /* Invalid input */ }
    Err(ValidationError::MissingField(field)) => { /* Missing required field */ }
    Err(ValidationError::FieldTooLong { field, max }) => { /* Field exceeds limit */ }
    Err(ValidationError::InvalidFormat { field }) => { /* Invalid format */ }
    Err(ValidationError::RateLimitExceeded) => { /* Rate limit hit */ }
}
```

## Architecture

Security is implemented at the adapter layer (hexagonal architecture):

```
┌─────────────────────────────────────────────┐
│              PodManager                      │
├─────────────────────────────────────────────┤
│  SecurityContext                            │
│  ├─ RateLimiter (token bucket)              │
│  └─ ExpiryEnforcer (max lifetime)           │
├─────────────────────────────────────────────┤
│  OCAP Manager                               │
│  ├─ Attenuation History                     │
│  ├─ Delegation Verification                 │
│  └─ Expiry Tracking                         │
└─────────────────────────────────────────────┘
```

## Testing

Run security tests:

```bash
cargo test -p hkask-agents security
cargo test -p hkask-agents ocap
```

All security modules have full test coverage:
- Input validation (valid, invalid, edge cases)
- Rate limiting (burst, refill, exhaustion)
- Expiry enforcement (valid, expired, custom lifetime)
- Attenuation history (recording, verification, chain integrity)

---

*ℏKask Security Model — v0.21.0*
*Defense in depth. OCAP security. User sovereignty.*
