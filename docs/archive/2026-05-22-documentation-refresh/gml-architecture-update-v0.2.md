# GML Architecture Update — MCP Server Integration

**Version:** 0.2.0  
**Date:** May 22, 2026  
**Status:** Phase 1 Complete

---

## Overview

This document updates the GML architecture to reflect the implementation of `hkask-mcp-gml`, a dedicated MCP server that extracts domain logic from templates and provides OCAP enforcement with CNS integration.

---

## Architecture Changes (v0.1.0 → v0.2.0)

### Before (v0.1.0 — Templates-Only)

```
┌─────────────────────────────────────────┐
│      GML Templates (Jinja2)             │
│  - Input validation                     │
│  - Capability checking (template-side)  │
│  - MWC computations (inline)            │
│  - CNS instrumentation (macros)         │
│  - Output rendering                     │
└─────────────────────────────────────────┘
```

**Problems:**
- Duplicate MWC computations across templates
- No cryptographic capability enforcement
- Hard to test domain logic
- Security checks in presentation layer

### After (v0.2.0 — MCP Server + Templates)

```
┌─────────────────────────────────────┐
│      GML Templates (Jinja2)         │
│  - Input validation                 │
│  - CNS instrumentation (macros)     │
│  - Output rendering                 │
└──────────────┬──────────────────────┘
               │ MCP calls
┌──────────────▼──────────────────────┐
│         hkask-mcp-gml               │
│  ┌──────────────┬────────────────┐  │
│  │ MWC Engine   │ Capability Mgr │  │
│  │ - compute    │ - verify       │  │
│  │ - apply      │ - sign         │  │
│  │ - delta_g    │ - enforce      │  │
│  └──────────────┴────────────────┘  │
│          ▲                          │
│          │ CNS spans                │
└──────────┼──────────────────────────┘
           │
┌──────────▼──────────────────────────┐
│         hkask-cns                   │
│  - Span emission                    │
│  - Audit logging                    │
│  - Algedonic alerts                 │
└─────────────────────────────────────┘
```

**Benefits:**
- Single source of truth for MWC computations
- Cryptographic capability enforcement
- Testable domain logic (8 unit tests)
- Security in middleware layer
- CNS observability pipeline

---

## Component Boundaries

### hkask-mcp-gml (MCP Server)

**Responsibilities:**
1. MWC mathematical kernel
2. Capability token management
3. OCAP enforcement
4. CNS span emission

**Tools (6 MCP endpoints):**
- `gml_compute_equilibrium` — MWC equilibrium calculation
- `gml_bind_effector` — Effector binding with capability enforcement
- `gml_create_capability` — Token creation with Ed25519 signing
- `gml_verify_capability` — Token verification
- `gml_compute_hill` — Hill coefficient calculation
- `gml_assess_cooperativity` — Cooperativity assessment

**Dependencies:**
- `hkask-types` — WebID for CNS emitter
- `hkask-cns` — SpanEmitter for observability
- `ed25519-dalek` — Cryptographic signing
- `rmcp` — MCP protocol

### GML Templates (Jinja2)

**Responsibilities:**
1. Input validation
2. CNS instrumentation (via macros)
3. Output rendering (markdown)

**Templates (5 core + 3 utility + 2 error):**
- `recognize-ensemble.j2` — Parse concept into states/ports
- `bind-effector.j2` — Apply effector, infer state-shift
- `compute-equilibrium.j2` — Calculate R̄, n_H, distribution
- `assess-coherence.j2` — Evaluate network homeostasis
- `reframe-concept.j2` — Generate alternative frames
- `macros.j2` — MWC computation macros
- `validate-inputs.j2` — Input validation macros
- `cns-instrument.j2` — CNS span emission macros
- `error-generic.j2` — Generic error template
- `error-validation.j2` — Validation error template

**Dependencies:**
- `hkask-templates` — Template rendering engine
- Jinja2 — Template language

### hkask-cns (CNS Runtime)

**Responsibilities:**
1. Span emission
2. Audit logging
3. Algedonic alerts
4. Variety monitoring

**Integration Points:**
- `SpanEmitter` — Used by hkask-mcp-gml for span emission
- `AlgedonicManager` — Monitors variety deficit
- `AuditLogger` — Records capability enforcement events

---

## Data Flow

### 1. Concept Recognition Flow

```
User Input
    ↓
recognize-ensemble.j2 (validate inputs)
    ↓
gml_compute_hill (MCP call)
    ↓
CNS span: cns.prompt.compute_hill.start
    ↓
MwcEngine::compute_r_bar()
MwcEngine::compute_hill()
    ↓
CNS span: cns.prompt.compute_hill.success
    ↓
Template renders markdown output
```

### 2. Effector Binding Flow (with OCAP)

```
User Input + Capability Token
    ↓
bind-effector.j2 (validate inputs)
    ↓
gml_bind_effector (MCP call)
    ↓
CNS span: cns.prompt.bind_effector.start
    ↓
CapabilityManager::verify_capability()
    ↓
[if valid] MwcEngine::apply_effectors()
    ↓
CNS span: cns.prompt.bind_effector.success
    ↓
Template renders markdown output

[if invalid]
    ↓
CNS span: cns.prompt.bind_effector.error
    ↓
Error template renders
```

---

## Security Model

### OCAP Enforcement Points

| Operation | Capability Required | Enforcement Point |
|-----------|---------------------|-------------------|
| `compute_equilibrium` | Optional | MCP server |
| `bind_effector` | **Required** | MCP server |
| `create_capability` | None | MCP server |
| `verify_capability` | None | MCP server |

### Capability Token Structure

```json
{
  "id": "gml_abc123...",
  "issuer": "did:webid:curator",
  "subject": "did:webid:researcher",
  "operations": ["bind_effector", "compute_equilibrium"],
  "scope": ["concept:freedom"],
  "effector_budget": 50.0,
  "issued_at": "2026-05-22T00:00:00Z",
  "expires_at": "2026-05-23T00:00:00Z",
  "signature": "ed25519:..."
}
```

### Cryptographic Guarantees

- **Ed25519 signatures** — 128-bit security level
- **SHA-256 hashing** — Preimage resistance
- **OS RNG** — Unpredictable key generation
- **Constant-time verification** — Timing attack resistant

---

## CNS Integration

### Span Categories

All GML operations emit CNS spans under `cns.prompt.*`:

| Operation | Span Name | Phase |
|-----------|-----------|-------|
| Compute equilibrium | `cns.prompt.compute_equilibrium.*` | Observe/Regulate |
| Bind effector | `cns.prompt.bind_effector.*` | Observe/Regulate |
| Create capability | `cns.prompt.create_capability.*` | Observe |
| Verify capability | `cns.prompt.verify_capability.*` | Observe |

### Event Types

Each operation emits structured events:
- `*.start` — Operation initiated
- `*.success` — Operation completed successfully
- `*.error` — Operation failed (with reason)
- `*.outcome` — Verification result

### Audit Log Structure

```json
{
  "event": "span_start",
  "span": "cns.prompt.bind_effector.start",
  "concept": "Freedom",
  "effector": "Security Crisis",
  "port_index": 0,
  "timestamp": "2026-05-22T00:00:00Z",
  "actor": "did:webid:curator"
}
```

### Algedonic Alerts

- **Variety deficit** — Triggered when same interpretation repeated >100 times
- **Channel:** `cns.algedonic.variety_deficit`
- **Escalation:** Curator/human intervention

---

## Testing Strategy

### Unit Tests (8 tests in hkask-mcp-gml)

**MWC Mathematical Kernel (4 tests):**
- `test_compute_r_bar_l_100_alpha_0` — L=100, α=0 → R̄≈0.01
- `test_compute_r_bar_l_1_alpha_0` — L=1, α=0 → R̄=0.5
- `test_compute_r_bar_invalid_l` — L≤0 errors
- `test_compute_delta_g` — ΔG calculation

**Capability Management (4 tests):**
- `test_create_capability_token` — Token creation
- `test_verify_capability_valid` — Valid token verification
- `test_verify_capability_wrong_operation` — Operation mismatch
- `test_check_effector_budget` — Budget enforcement

### Integration Tests (Future)

- Template → MCP → CNS pipeline
- Multi-step workflows
- Token lifecycle (create → use → expire)

---

## Performance Characteristics

### Computational Complexity

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| `compute_r_bar` | O(1) | <100ns |
| `compute_hill` | O(1) | <100ns |
| `apply_effectors` | O(m) | <1μs |
| `sign_token` | O(1) | <10μs |
| `verify_signature` | O(1) | <10μs |

### Memory Usage

- **Stateless computations** — No heap allocation
- **Signing key** — 64 bytes (in-memory)
- **Token cache** — Not implemented (optional future optimization)

---

## Future Enhancements

### Phase 2 (Planned)

1. **Key Persistence** — Store signing key in hkask-keystore
2. **Delegation Chains** — Support token delegation (A → B → C)
3. **Revocation Lists** — Token revocation mechanism
4. **Rate Limiting** — Per-subject operation limits
5. **Multi-signature** — Require multiple issuers for sensitive operations

### Phase 3 (Research)

1. **RDF Runtime Binding** — SPARQL endpoint for concept graphs
2. **Monad Laws Verification** — Formal verification of GML as monad
3. **Temporal Dynamics** — Time-dependent effector binding
4. **Multi-ligand Interactions** — Competitive/allosteric binding
5. **Collective Allostery** — Multi-agent conceptual analysis

---

## References

- **Original Architecture:** `docs/gml/gml-architecture.md` (v0.1.0)
- **MCP Server Docs:** `docs/gml/gml-mcp-server.md`
- **Security Audit:** `docs/gml/gml-security-audit.md`
- **Implementation Summary:** `docs/gml/gml-implementation-summary.md`
- **Remediation Progress:** `docs/gml/gml-remediation-progress.md`

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.2.0*  
*Architecture Update: MCP Server + OCAP + CNS Integration*
