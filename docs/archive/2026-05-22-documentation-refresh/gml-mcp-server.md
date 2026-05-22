# hkask-mcp-gml — GML Allosteric Thinking MCP Server

**Version:** 0.1.0  
**Type:** MCP Server  
**Status:** Implementation complete (Tasks 2, 3, 4)

---

## Overview

`hkask-mcp-gml` is the MCP server implementation of GML (Allosteric Thinking) domain logic and capability enforcement. It extracts MWC computations from templates and provides OCAP enforcement with Ed25519-signed capability tokens.

---

## Responsibilities

### 1. MWC Domain Logic (Task 2)

The server implements the mathematical kernel of GML:

```rust
// MWC equilibrium: R̄ = (1+α)ⁿ/((1+α)ⁿ + L·(1+cα)ⁿ)
pub fn compute_r_bar(l: f64, c: f64, n: u32, alpha: f64) -> Result<f64, GmlError>

// Hill coefficient: n_H = d(ln(R̄/(1-R̄)))/d(ln(α))
pub fn compute_hill(l: f64, c: f64, n: u32, alpha: f64, _r_bar: f64) -> f64

// Free energy: ΔG = -RT·ln(R̄/(1-R̄))
pub fn compute_delta_g(r_bar: f64, temperature: f64) -> f64

// Effector application: α' = α + Σ[effector]
pub fn apply_effectors(concept: &Concept, effectors: &[Effector]) -> Result<(f64, f64, f64), GmlError>
```

**Benefits:**
- Single source of truth for MWC computations
- Type-safe parameter validation
- Reusable across templates, adapters, and tests
- Templates delegate computation to middleware

### 2. Capability Enforcement (Task 3)

All state-changing operations require capability tokens:

```rust
// Capability verification
pub fn verify_capability(
    &self,
    request: VerifyCapabilityRequest,
) -> Result<TokenVerification, GmlError>

// Effector budget checking
pub fn check_effector_budget(
    &self,
    token: &CapabilityToken,
    concentration: f64,
) -> Result<bool, GmlError>
```

**Enforcement points:**
- `gml_bind_effector` — requires `bind_effector` operation
- `gml_compute_equilibrium` — requires `compute_equilibrium` operation (optional)
- Budget constraints enforced per operation

### 3. Unforgeable Tokens (Task 4)

Capability tokens are cryptographically signed with Ed25519:

```rust
// Token creation with signature
pub fn create_capability(
    &self,
    request: CreateCapabilityRequest,
) -> Result<CapabilityToken, GmlError>

// Signature verification
fn verify_signature(
    &self,
    token_data: &str,
    signature_hex: &str,
) -> Result<bool, GmlError>
```

**Token structure:**
```json
{
  "id": "gml_abc123...",
  "issuer": "did:webid:user123",
  "subject": "did:webid:agent456",
  "operations": ["bind_effector", "compute_equilibrium"],
  "scope": ["concept:freedom"],
  "effector_budget": 100.0,
  "issued_at": "2026-05-22T00:00:00Z",
  "expires_at": "2026-05-23T00:00:00Z",
  "signature": "ed25519:..."
}
```

---

## Tools

| Tool | Description | Capability Required |
|------|-------------|---------------------|
| `gml_compute_equilibrium` | Compute MWC equilibrium for concept | Optional |
| `gml_bind_effector` | Bind effector to concept port | **Required** |
| `gml_create_capability` | Create new capability token | None |
| `gml_verify_capability` | Verify capability token validity | None |
| `gml_compute_hill` | Compute Hill coefficient | None |
| `gml_assess_cooperativity` | Assess cooperativity level | None |

---

## Usage Examples

### Create Capability Token

```json
{
  "issuer": "did:webid:curator",
  "subject": "did:webid:researcher",
  "operations": ["bind_effector", "compute_equilibrium"],
  "scope": ["concept:freedom"],
  "effector_budget": 50.0,
  "expires_in_seconds": 86400
}
```

### Bind Effector

```json
{
  "concept": {
    "name": "Freedom",
    "t_state": {"description": "Negative liberty", "energy": -10.0},
    "r_state": {"description": "Positive liberty", "energy": -5.0},
    "l": 100.0,
    "ports": [
      {"name": "Threat Response", "effector_shape": "SecurityThreat", "affinity_c": 0.1}
    ],
    "current_alpha": 0.0
  },
  "effector": {
    "name": "Security Crisis",
    "concentration": 10.0,
    "effect_type": "Activator",
    "shape": "SecurityThreat"
  },
  "port_index": 0,
  "capability": {...}
}
```

### Compute Equilibrium

```json
{
  "concept": {...},
  "effectors": [
    {"name": "Economic Pressure", "concentration": 5.0, "effect_type": "Inhibitor", "shape": "EconomicCondition"}
  ],
  "capability": {...}
}
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    GML Templates                         │
│  (recognize-ensemble.j2, bind-effector.j2, etc.)        │
│  - Input validation                                      │
│  - CNS instrumentation                                   │
│  - Output rendering                                      │
└────────────────────┬────────────────────────────────────┘
                     │ MCP calls
┌────────────────────▼────────────────────────────────────┐
│                  hkask-mcp-gml                           │
│  ┌──────────────────┬────────────────────────────────┐  │
│  │  MWC Engine      │  Capability Manager            │  │
│  │  - compute_r_bar │  - create_capability           │  │
│  │  - compute_hill  │  - verify_capability           │  │
│  │  - delta_g       │  - sign tokens (Ed25519)       │  │
│  │  - apply_effectors│  - verify signatures          │  │
│  └──────────────────┴────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│              hkask-cns (CNS instrumentation)            │
│  - Span emission (cns.gml.*)                            │
│  - Audit logging                                        │
│  - Algedonic alerts                                     │
└─────────────────────────────────────────────────────────┘
```

---

## Security Model

### OCAP Enforcement

1. **Capability tokens** are required for state-changing operations
2. **Signature verification** ensures tokens are unforgeable
3. **Scope restrictions** limit operations to specific concepts
4. **Budget constraints** prevent resource exhaustion
5. **Expiration** limits token lifetime

### Cryptographic Guarantees

- **Ed25519 signatures** — 128-bit security level
- **SHA-256 hashing** — message digest before signing
- **OS RNG** — cryptographically secure random number generation
- **Constant-time verification** — timing attack resistant

---

## CNS Integration

### Spans Emitted

- `cns.gml.compute_equilibrium` — equilibrium computation
- `cns.gml.bind_effector` — effector binding
- `cns.gml.create_capability` — token creation
- `cns.gml.verify_capability` — token verification

### Audit Log Events

```json
{
  "event": "audit_log",
  "operation": "bind_effector",
  "concept_id": "freedom-001",
  "capability_hash": "sha256:abc123...",
  "before_r_bar": 0.01,
  "after_r_bar": 0.73,
  "result": "success",
  "timestamp": "2026-05-22T00:00:00Z"
}
```

### Algedonic Alerts

- **Variety deficit** — detected when same interpretation repeated >100 times
- **Channel:** `cns.algedonic.variety_deficit`

---

## Error Handling

### Error Types

```rust
pub enum GmlError {
    InvalidMwcParameters(String),      // L ≤ 0, c ≤ 0, etc.
    CapabilityDenied(String),          // Token invalid or insufficient
    SignatureError(SignatureError),    // Ed25519 verification failed
    KeystoreError(String),             // Key storage failure
    InvalidInput(String),              // Missing required fields
    HexError(FromHexError),            // Signature decoding failed
}
```

### Error Responses

```json
{
  "success": false,
  "error": "Capability denied",
  "reason": "Operation 'bind_effector' not allowed"
}
```

---

## Testing

### Unit Tests (in hkask-testing)

- MWC computation accuracy
- Signature generation/verification
- Capability validation logic
- Effector budget enforcement
- Error handling

### Integration Tests

- Template → MCP server → CNS pipeline
- Multi-step workflows (recognize → bind → equilibrate → assess → reframe)
- Token lifecycle (create → use → expire)

---

## Performance

### Benchmarks (target)

- `compute_r_bar`: <100ns
- `bind_effector`: <1ms (including capability verification)
- `create_capability`: <10ms (including signature generation)
- `verify_capability`: <1ms (signature verification)

### Optimization Strategies

- Stateless computations (no heap allocation)
- Pre-computed signing keys (generated at startup)
- Signature caching (optional, for repeated verifications)

---

## Future Work

1. **Key persistence** — store signing key in hkask-keystore
2. **Delegation chains** — support token delegation (A → B → C)
3. **Revocation** — token revocation list
4. **Multi-signature** — require multiple issuers for sensitive operations
5. **Rate limiting** — per-subject operation limits
6. **Monadic structure** — formalize GML as monad (bind = effector application)

---

## References

- **MWC Model:** Monod, Wyman, Changeux (1965) — "On the Nature of Allosteric Transitions"
- **OCAP:** Miller, "Robust Composition: Foundations of Object-Capability Security"
- **Ed25519:** Bernstein et al. (2012) — "High-speed high-security signatures"
- **GML Architecture:** `docs/gml/gml-architecture.md`
- **Security Audit:** `docs/gml/gml-security-audit.md`

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
