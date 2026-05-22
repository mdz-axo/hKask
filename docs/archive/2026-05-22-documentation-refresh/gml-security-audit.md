# GML Security Audit

**Date:** May 2026  
**Auditor:** hKask Security Review  
**Scope:** GML templates and manifests

---

## Security Principles (Schneier)

| Principle | Status | Notes |
|-----------|--------|-------|
| Defense in depth | ✓ | Validation + capability + audit logging |
| Least privilege | ✓ | OCAP capability tokens |
| Fail safely | ✓ | Error templates for all failure modes |
| Complete mediation | ✓ | Every operation checks capability |
| Audit trail | ✓ | CNS instrumentation on all operations |
| Input validation | ✓ | JSON schema + runtime validation |

---

## Threat Model

### Threat 1: Unauthorized Concept Access

**Attack:** User attempts to access concepts outside their capability scope.

**Mitigation:**
- Capability tokens checked in `bind-effector.j2`
- `check_capability()` macro validates operation permission
- Error displayed via `error-capability.j2`

**Status:** ✓ Mitigated

---

### Threat 2: Invalid Parameter Injection

**Attack:** User provides invalid L, c, n, α values to corrupt computation.

**Mitigation:**
- JSON schema validation (`schema.json`)
- Runtime validation (`validate-inputs.j2`)
- Parameter range checks (L > 0, c > 0, n ≥ 1, α ≥ 0)
- Error displayed via `error-parameters.j2`

**Status:** ✓ Mitigated

---

### Threat 3: Effector Budget Exhaustion

**Attack:** User applies excessive effector concentration to destabilize interpretations.

**Mitigation:**
- Capability `effector_budget` field limits concentration
- `check_effector_budget()` macro validates
- Error displayed via `error-budget-exceeded.j2`

**Status:** ✓ Mitigated

---

### Threat 4: Port Shape Mismatch

**Attack:** User attempts to bind effector to incompatible port.

**Mitigation:**
- `check_port_compatibility()` macro validates shape match
- Error displayed via `error-port-compatibility.j2`
- No computation performed on mismatch

**Status:** ✓ Mitigated

---

### Threat 5: Audit Trail Evasion

**Attack:** User attempts to hide state-changing operations.

**Mitigation:**
- CNS instrumentation on all templates
- `audit_log()` macro emits immutable log entries
- Logs include capability hash for accountability

**Status:** ✓ Mitigated

---

### Threat 6: Missing Input Exploit

**Attack:** User provides incomplete inputs to trigger undefined behavior.

**Mitigation:**
- `validate_concept()` checks required fields
- `validate_effectors()` validates effector array
- All templates check for missing inputs
- Error displayed via `error-missing-input.j2`

**Status:** ✓ Mitigated

---

## Capability Composition (Miller/OCAP)

| Property | Enforcement |
|----------|-------------|
| No ambient authority | ✓ Explicit capability token required |
| Least privilege | ✓ Default capability = Recognize only |
| Attenuation | ✓ Child capabilities are subsets |
| End-to-end | ✓ Checked at template level |
| Audit | ✓ All checks logged via CNS |

---

## Remaining Concerns

| Concern | Severity | Remediation |
|---------|----------|-------------|
| Rate limiting on template invocations | Medium | Implement at infrastructure layer |
| Encryption of stored concept data | High | SQLCipher already in hkask-storage |
| Capability token cryptographic signing | Medium | Add signature verification |
| Template sandboxing | Low | Jinja2 autoescape enabled |

---

## Conclusion

GML templates implement defense-in-depth security:
1. Schema validation at input
2. Capability enforcement at operation
3. Audit logging on all state changes
4. Error handling for all failure modes

**Audit result:** PASS (with recommendations above)

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*