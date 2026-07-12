---
title: "How to Audit Sovereignty — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Audit Sovereignty

**Goal:** Inspect and verify that hKask's Magna Carta principles (P1–P4) are being enforced, including delegation tokens, consent records, pod boundaries, and OCAP enforcement.

Sovereignty is hKask's foundational guarantee: users own their data and control how it is accessed. All access requires explicit, scoped, version-aware, revocable consent. No ambient authority exists — every tool invocation passes through the OCAP gate.

---

## 1. What Sovereignty Means in hKask

The Magna Carta defines four principles, each enforced by specific code paths:

| Principle | Core Guarantee | Enforcement | Fail Mode |
|-----------|---------------|-------------|-----------|
| **P1 — User Sovereignty** | Users own their data and delegation boundaries | `SovereigntyChecker::can_access()` + `require_sovereignty()` | All access denied if no checker wired |
| **P2 — Affirmative Consent** | Default is deny; access requires explicit, scoped, revocable consent | `ConsentManager::has_consent()` (`unwrap_or(false)`) | Storage errors → deny |
| **P3 — Generative Space** | No hidden control plane; content safety is a mandatory floor, not a ceiling | `hkask-guard` at every LLM boundary; no `is_admin` flag | Structurally impossible to hide settings |
| **P4 — Clear Boundaries (OCAP)** | All resource access is capability-gated; no ambient authority | `CapabilityChecker::verify()` + `GovernedTool::invoke()` | Empty roots → reject all; no "god token" |
| **P4.1 — Pod Boundaries** | Pods cannot structurally reach other pods' MCP servers | Type system enforcement via `PerPodToolBinding` | Always enforced (structural) |

---

## 2. Viewing Sovereignty Status

Get the full sovereignty picture for the current user:

```bash
kask sovereignty status
```

Output shows:

```
Sovereignty Status
==================

Consent State:
  WebID: webid://cli-user
  • episodic_memory: GRANTED
  • personal_context: DENIED
  • capability_tokens: GRANTED
  • ocap_boundaries: DENIED
  • semantic_memory: GRANTED
  • template_invocations: DENIED
  • template_registry: GRANTED (public)

Data Boundaries:
  • Sovereign: episodic_memory, personal_context, capability_tokens, ocap_boundaries
  • Shared: semantic_memory, template_invocations
  • Public: template_registry

Affirmative Consent:
  • Requires Affirmative Consent: true
```

### Data Categories

| Category | Classification | Access Rule |
|----------|---------------|-------------|
| `episodic_memory` | Sovereign | Consent **AND** owner match required |
| `personal_context` | Sovereign | Consent **AND** owner match required |
| `capability_tokens` | Sovereign | Consent **AND** owner match required |
| `ocap_boundaries` | Sovereign | Consent **AND** owner match required |
| `semantic_memory` | Shared | Consent required (any WebID) |
| `template_invocations` | Shared | Consent required (any WebID) |
| `template_registry` | Public | No consent required |

---

## 3. Inspecting Delegation Tokens

OCAP (Object Capability) enforcement uses Ed25519-signed `DelegationToken` objects. List tokens:

```bash
kask token list
```

Output:

```
curator — tool:*, inference:*, memory:read — 2026-06-15T10:30:00Z
replicant-alice — tool:web_search, tool:condenser — 2026-06-20T14:22:00Z
```

Issue a new token for a replicant:

```bash
kask token issue \
  --replicant my-replicant \
  --capabilities "tool:web_search,tool:condenser,inference:*" \
  --ttl 24h
```

Output:

```json
{
  "token_id": "...",
  "capabilities": [...],
  "expires_at": 1234567890,
  ...
}
```

Use the token in an IDE or deployment:

```bash
export HKASK_DELEGATION_TOKEN='{"token_id":"...","capabilities":[...],"expires_at":1234567890}'
```

Revoke a token:

```bash
kask token revoke --token-id <token_id>
```

Check capability enforcement at the pod level:

```bash
kask pod status <pod_id> --verbose
```

This shows per-pod capability bindings — which tokens authorize which tools.

---

## 4. Verifying Consent Records

### Grant Consent

Grant consent for a data category. Use `--agent curator` to authorize the Curator daemon:

```bash
# Grant curator access to episodic memory
kask sovereignty grant --category episodic_memory --agent curator

# Grant for the CLI user
kask sovereignty grant --category semantic_memory
```

### Revoke Consent

Revoke ALL consent for the current user:

```bash
kask sovereignty revoke
```

### Check Access for a Specific Category

```bash
kask sovereignty check --category episodic_memory
```

Output:

```
Data Access Check
=================
  Category: episodic_memory
  Classification: SOVEREIGN
  Access required: CONSENT + OWNER_MATCH
  Access: DENIED
  Use 'kask sovereignty grant --category episodic_memory' to grant.
```

---

## 5. Auditing Pod Boundaries

List all active agent pods:

```bash
kask pod list
```

Output:

```
Agent pods (2):
  curator-primary (active)
    WebID: webid://curator
    Name:  curator
  replicant-alice (active)
    WebID: webid://alice
    Name:  alice
```

Inspect a pod's tool bindings and OCAP state:

```bash
kask pod status curator-primary --verbose
```

Each pod has its own `PerPodToolBinding`, dedicated SQLCipher file, and per-pod variety counters. Pods are structurally isolated — cross-pod dispatch is impossible at the type level.

---

## 6. Checking OCAP Enforcement Status

OCAP enforcement runs through the `GovernedTool` membrane. Every tool call passes five gates:

```
Caller → GovernedTool.invoke(server, tool, args, token)
           ├─ Step 0: token.verify() — cryptographic authenticity
           ├─ Step 1: verify_capability_exact(token, tool) || verify_capability_domain_fallback(token, tool)
           ├─ Step 2: cybernetics.can_proceed(agent, estimated_cost) — gas budget check
           ├─ Step 3: emit cns.tool.invoked span
           ├─ Step 4: inner.invoke(server, tool, args, token) → delegate
           └─ Step 5: settle_gas(agent, reserved, actual) → refund if over-estimated
```

Verify that OCAP enforcement is active:

```bash
# Check tool spans to see OCAP gate activity
kask cns query --target cns.tool

# Check MCP startup gates
kask cns query --target cns.mcp
```

Three startup gates control MCP server access:
- **Gate 1 (auth):** Server refuses to start on failure → `McpError::Auth`
- **Gate 2 (assignment):** Server refuses to start on failure → `McpError::RoleAssignment`
- **Gate 3 (capability per tool):** Non-fatal — server starts in degraded mode with denied tools unavailable

---

## 7. Exporting Sovereignty State for External Audit

### Magna Carta Verification

Run structural audits against the codebase to verify P1–P4 enforcement:

```bash
# Full verification report
kask sovereignty verify

# Verify a specific principle
kask sovereignty verify --principle user_sovereignty
kask sovereignty verify --principle affirmative_consent
kask sovereignty verify --principle generative_space
kask sovereignty verify --principle clear_boundaries

# JSON output for CI/automation
kask sovereignty verify --json
```

Sample output:

```
Magna Carta Verification Report
==============================

## User Sovereignty (P1)

  ✓ P1-001 sovereignty_checker_configured check: pass
    → SovereigntyChecker found in crate hkask-agents
  ✓ P1-002 require_sovereignty_enforced check: pass
    → All pod accesses route through require_sovereignty()
  △ P1-003 data_portability_export check: gap
    → Export endpoint exists but not tested
    ⚑ Add integration test for kask sovereignty export

  Principle summary: 2 pass, 0 fail, 1 gap
```

### CNS Span Audit for Sovereignty

Track sovereignty enforcement through CNS spans:

```bash
# View consent-related spans
kask cns query --target cns.sovereignty

# Key events:
#   consent_granted — consent was granted for a category
#   consent_revoked — consent was revoked
#   consent_checked — a consent check was performed; check result field
```

### API Consent Status

If the API server is running:

```bash
curl -H "Authorization: Bearer $HKASK_API_KEY" \
  http://localhost:3000/sovereignty
```

---

## 8. Understanding Denial Events

When access is denied, CNS emits spans that help trace the root cause:

### `cns.tool.denied` (CNS Span)

Emitted when a tool invocation is blocked by OCAP enforcement. Look for:

- **Token signature invalid** → `ToolPortError::CapabilityDenied("Token failed cryptographic verification")` — check token validity and trusted roots
- **No capability for tool** → `ToolPortError::CapabilityDenied("Token does not authorize tool: X")` — issue a token with the required capability
- **Gas budget exceeded** → `ToolPortError::EnergyBudgetExceeded(...)` — increase the energy cap or reduce consumption
- **No CapabilityChecker configured** → `AgentPodError::CapabilityDenied` (fail-closed)

### `cns.sovereignty.denied` (CNS Span)

Emitted when consent is denied:

- **No consent grant** → `has_consent()` returns `false` — run `kask sovereignty grant`
- **Storage error** → `unwrap_or(false)` in `has_consent()` — check database connectivity
- **Consent revoked** → `ConsentRecord::active = false` — re-grant if appropriate
- **Sovereign data without owner match** → Even with consent, sovereign data requires owner match

### Common Denial Scenarios

| Scenario | Error | Fix |
|----------|-------|-----|
| No `SovereigntyChecker` wired | `AgentPodError::SovereigntyDenied` | Wired automatically by `AgentService` |
| No `CapabilityChecker` wired | `AgentPodError::CapabilityDenied` | Wired automatically by `AgentService` |
| No `ConsentManager` wired | `DenyAllConsent` returns `false` | Wired automatically by `AgentService` |
| Storage error in consent check | Consent denied | Check `HKASK_DB_PATH` and `HKASK_DB_PASSPHRASE` |
| Token expired | `CapabilityChecker::verify_with_time()` returns `false` | Re-issue token with a longer TTL |
| API key without budget | `cns.gas.depleted` span emitted | Increase energy budget |

---

## Related

- [Magna Carta Reference](../reference/magna-carta.md) — Full principle text, enforcement traces, failure modes
- [CNS Span Registry](../reference/cns-spans.md) — CNS span taxonomy
- [Read CNS Alerts](read-cns-alerts.md) — Interpreting algedonic signals
