# Security Architecture

**ADR-021** — Security Hardening Implementation  
**Date:** 2026-05-21  
**Status:** Implemented

## Context

hKask requires security hardening for production deployment:
- Input validation for all pod operations
- Rate limiting to prevent abuse
- OCAP enhancement with attenuation tracking

## Decision

Implement three-layer defense-in-depth security:

### Layer 1: Input Validation

**Location:** `crates/hkask-agents/src/security.rs`

- `InputValidator<T>` trait for schema-based validation
- `AgentPersonaInput` struct with field constraints
- `ValidationError` enum for detailed error reporting

### Layer 2: Rate Limiting

**Location:** `crates/hkask-agents/src/security.rs`

- `TokenBucket` — Token bucket algorithm implementation
- `RateLimiter` — Per-key rate limiting with async locking
- Default: 10 requests burst, 1 request/second refill

### Layer 3: OCAP Enhancement

**Location:** `crates/hkask-agents/src/ocap.rs`

- `OCAP` struct with attenuation history tracking
- `AttenuationHistory` — Delegation chain recording
- `ExpiryEnforcer` — Configurable token lifetime

## Implementation

### Security Module Structure

```
hkask-agents/src/security.rs
├── ValidationError (enum)
├── InputValidator<T> (trait)
├── AgentPersonaInput (struct)
├── TokenBucket (struct)
├── RateLimiter (struct)
├── AttenuationHistory (struct)
├── AttenuationEntry (struct)
├── ExpiryEnforcer (struct)
└── SecurityContext (struct)
```

### OCAP Module Structure

```
hkask-agents/src/ocap.rs
└── OCAP (struct)
    ├── attenuation_history: HashMap<String, AttenuationHistory>
    ├── expiry_enforcer: ExpiryEnforcer
    ├── record_attenuation()
    ├── verify_attenuation()
    ├── is_expired()
    ├── validate_expiry()
    ├── attenuate_with_history()
    └── get_attenuation_history()
```

### Integration Points

**PodManager** (`crates/hkask-agents/src/pod.rs`):

```rust
pub struct PodManager {
    // ... existing fields ...
    security_context: SecurityContext,
}

impl PodManager {
    pub async fn create_pod(&self, ...) -> AgentPodResult<PodID> {
        // Rate limit pod creation
        self.security_context.rate_limiter.acquire(&rate_key, 1.0).await?;
        
        // Validate persona input
        input.validate(&input)?;
        
        // Create pod...
    }
    
    pub fn security_context(&self) -> &SecurityContext {
        &self.security_context
    }
}
```

**PodManagerBuilder**:

```rust
pub struct PodManagerBuilder {
    // ... existing fields ...
    security_context: Option<SecurityContext>,
}

impl PodManagerBuilder {
    pub fn security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }
}
```

## Consequences

### Positive

- **Defense in depth:** Multiple security layers
- **Configurable:** Rate limits and expiry via builder pattern
- **Traceable:** Attenuation history for audit trails
- **Testable:** Full test coverage (41 tests in hkask-agents)

### Negative

- **Line budget:** +788 lines (security.rs: 330, ocap.rs: 95, pod.rs: 363)
- **Complexity:** Additional async locking for rate limiter
- **Dependencies:** Requires tokio for async primitives

## Compliance

**P1 (No trait without two consumers):** ✓
- `InputValidator<T>` used by `AgentPersonaInput` and tests

**P2 (No generic without two instantiations):** ✓
- `InputValidator<T>` instantiated for `AgentPersonaInput`

**C5 (Every error variant is unique recovery path):** ✓
- `ValidationError` has 5 distinct variants with recovery paths

## Testing

```bash
# Security module tests
cargo test -p hkask-agents security

# OCAP module tests
cargo test -p hkask-agents ocap

# All hkask-agents tests
cargo test -p hkask-agents
```

**Test Coverage:**
- Input validation: 6 tests
- Rate limiting: 2 tests
- Expiry enforcement: 2 tests
- Attenuation history: 3 tests
- OCAP integration: 5 tests
- Pod integration: 2 tests

## References

- `docs/user-guides/SECURITY.md` — User-facing security documentation
- `crates/hkask-agents/src/security.rs` — Security module implementation
- `crates/hkask-agents/src/ocap.rs` — OCAP enhancement implementation
- `crates/hkask-types/src/capability.rs` — Capability token with `attenuate_with_expiry()`

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
