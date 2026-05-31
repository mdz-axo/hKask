---
title: "hKask Trust, Security & Observability Specification"
audience: [architects, security engineers, developers]
last_updated: 2026-05-29
version: "2.2.1"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [trust, observability]
---

# hKask Trust, Security & Observability Specification

**Purpose:** Authoritative specification for security model, OCAP enforcement, encryption, CNS observability, and threat model. Single source of truth for DDMVSS categories **Trust & Security** and **Observability**.

**Related:** [`domain-and-capability.md`](domain-and-capability.md), [`interface-and-composition.md`](interface-and-composition.md), [`persistence-and-lifecycle.md`](persistence-and-lifecycle.md), [`magna-carta.md`](magna-carta.md)

**Verification:** `cargo check --workspace && cargo test -p hkask-types && cargo test -p hkask-cns`

---

## 1. Security Model

### 1.1 Zero-Trust Defaults

hKask implements a **zero-trust, capability-based security model**:[^miller-robust]

- **No hardcoded secrets** — all keys from environment or keystore
- **No ambient authority** — every operation requires explicit capability
- **Fail-closed** — denied by default
- **No wildcards** — `"*"` rejected at registration

[^miller-robust]: Miller, M. S. (2006). *Robust Composition*. Johns Hopkins University.

### 1.2 Single Capability Primitive

All access control uses `CapabilityToken` (`crates/hkask-types/src/capability/mod.rs:223`):

| Property | Implementation |
|----------|---------------|
| **Signing** | HMAC-SHA256 + `subtle::ConstantTimeEq` |
| **Scoping** | Resource + action pairs (`CapabilityResource`, `CapabilityAction`) |
| **Caveats** | Expiration, operation, template, visibility |
| **Attenuation** | Max depth 7 (configurable) |
| **Revocation** | Persistent SQLite via `RevocationStore` (`hkask-agents/src/revocation_store.rs:16`) |
| **Secure memory** | Arc-wrapped, `Zeroizing` on drop |

**Full capability model:** [`domain-and-capability.md`](domain-and-capability.md) §5

### 1.3 Deterministic Identity

WebIDs derived from persona content via UUID v5:
- Same persona → same WebID (across processes, restarts)
- Root authority from fixed `"hkask-root-authority"` persona
- Namespace UUID: `686b6173-6b2d-7065-7273-6f6e612d6e73`

**Implementation:** `WebID::from_persona()` (`crates/hkask-types/src/id.rs:91`)

### 1.4 Encryption Stack

| Layer | Algorithm | Crate | Purpose |
|-------|-----------|-------|--------|
| Database at rest | SQLCipher (AES-256-CBC) | `rusqlite` + `bundled-sqlcipher` | Encrypted storage |
| Master key derivation | Argon2id → HKDF-SHA256 | `argon2` v0.5 + `hmac`/`sha2` | One passphrase → all internal secrets[^master-key] |
| Key derivation | Argon2id | `argon2` v0.5 | Passphrase → key[^argon2] |
| Sub-key expansion | HKDF-SHA256 (RFC 5869) | `hmac` + `sha2` | Master key → independent sub-keys |
| Capability signing | HMAC-SHA256 | `hmac` + `sha2` | Token integrity |
| Manifest signing | Ed25519 | `ed25519-dalek` v2 | Template provenance |
| Symmetric encryption | AES-256-GCM | `aes-gcm` v0.10 | Secret encryption |
| Content hashing | BLAKE3 | `blake3` v1 | Git CAS addressing |
| Memory protection | Zeroize on drop | `zeroize` + `zeroize_derive` | Prevent leakage |
| Secret wrapping | `secrecy` | `secrecy` crate | No accidental logging |

[^master-key]: The master key derivation chain uses Argon2id once (slow, memory-hard, ~100ms) to stretch the user's passphrase into a 256-bit master key, then HKDF-SHA256 (fast, deterministic, ~1μs each) to derive each internal secret. This ensures the same passphrase always produces the same secrets across restarts.

[^argon2]: Biryukov, A., Dinu, D., & Khovratovich, D. (2016). *Argon2: The Memory-Hard Function for Password Hashing*. Selected for GPU/ASIC resistance.

### 1.5 OCAP Enforcement Points

| Boundary | Enforcement | Implementation |
|----------|------------|----------------|
| MCP tool invocation | `SecurityGateway` | `hkask-mcp/src/security.rs:51` |
| Template execution | `CapabilityAwareValidator` | `hkask-templates/src/capability_validator.rs:21` |
| ACP message routing | `SovereigntyPort` | `hkask-agents/src/ports/sovereignty.rs:79` |
| Memory storage | `MemoryStoragePort` | `hkask-agents/src/pod/context.rs:50` |
| API requests | Capability in Authorization header | `hkask-api/src/lib.rs` |
| Pod creation | Root capability required | `hkask-agents/src/pod/manager.rs:266` |

### 1.6 Security Invariants

| Invariant | Enforcement |
|-----------|-------------|
| No wildcard capabilities | `AcpRuntime::register_agent` rejects `"*"` |
| No ambient authority | Every operation requires capability |
| Constant-time comparison | `subtle::ConstantTimeEq` |
| Persistent revocation | `RevocationStore` survives restarts |
| Attenuation limit | `attenuation_level < max_attenuation` |
| Deterministic identity | UUID v5 from persona |
| Deterministic secrets | All internal secrets derived from master key via HKDF-SHA256 |
| No random secret fallback | `SecretRef::Generated` prohibited in production code paths |
| Secure memory | Secrets zeroized on drop |
| Async purity | No `block_in_place`/`block_on` |
| Typed errors | No `unwrap()` on hot paths |

### 1.7 Master Key Derivation

All internal secrets (ACP signing key, capability token key, MCP security key, OCAP secret) are derived deterministically from a single master passphrase using HKDF-SHA256. This eliminates the previous class of bugs where secrets were silently generated at random on each process start, invalidating all previously-issued tokens.

**Derivation chain:**

```
Master passphrase (user-provided, stored in OS keychain or env var HKASK_MASTER_KEY)
  │
  ├── Argon2id(passphrase, fixed_salt) → 256-bit master key  [~100ms, memory-hard]
  │
  ├── HKDF-SHA256(master_key, "hkask:acp-secret")     → ACP HMAC signing secret
  ├── HKDF-SHA256(master_key, "hkask:capability-key")  → API capability token key
  ├── HKDF-SHA256(master_key, "hkask:mcp-security-key") → MCP security gateway key
  └── HKDF-SHA256(master_key, "hkask:ocap-secret")     → OCAP signing secret
```

**Resolution priority:**

1. `SecretRef::Derived` — HKDF-SHA256 from master key (preferred, deterministic)
2. `SecretRef::Env` — Direct environment variable (for override)
3. `SecretRef::Keychain` — OS keychain entry (for override)
4. `SecretRef::Generated` — Random bytes (⚠️ not restart-safe; only for salts/nonces)

**Implementation:**

| Component | Location |
|-----------|----------|
| `SecretRef::Derived` variant | `crates/hkask-types/src/secret.rs` |
| `derivation_contexts` constants | `crates/hkask-types/src/secret.rs` |
| `derive_all_internal_secrets()` | `crates/hkask-keystore/src/master_key.rs` |
| `derive_sub_key()` (HKDF-SHA256) | `crates/hkask-keystore/src/master_key.rs` |
| `resolve()` extended for `Derived` | `crates/hkask-keystore/src/keychain.rs` |
| Call-site updates | `crates/hkask-agents/src/acp.rs`, `crates/hkask-mcp/src/security.rs`, `crates/hkask-api/src/lib.rs` |

**Security properties:**

- Same passphrase → same secrets (restart-safe, cluster-safe)
- Different contexts → cryptographically independent sub-keys
- Compromise of one sub-key does not compromise the master key or other sub-keys (HKDF extraction step)
- Master key never stored; only derived sub-keys are held in memory with `Zeroizing` protection
- Argon2id with OWASP parameters (64 MiB, 3 iterations, 4 lanes) resists GPU/ASIC attacks

### 1.8 Keystore Persistence

The MCP keystore server persists encrypted entries to a file-based vault at `~/.hkask/keystore/vault.json` (configurable via `HKASK_KEYSTORE_DIR`). Each entry is AES-256-GCM encrypted with a per-entry salt and serialized as JSON. The vault is loaded at startup and saved after each mutation using atomic writes (temp file + rename).

| Property | Implementation |
|----------|---------------|
| Encryption | AES-256-GCM with per-entry Argon2id-derived key |
| Access control | OCAP-gated: only owner WebID can read |
| Persistence | JSON vault file, atomic writes |
| Vault location | `~/.hkask/keystore/vault.json` or `HKASK_KEYSTORE_DIR` |
| Schema versioning | `Vault.version` field for forward compatibility |

---

## 2. STRIDE-lite Threat Model

| Threat | Category | Mitigation | hKask Primitive |
|--------|----------|-----------|-----------------|
| Template injection | Tampering | Jinja2 sandbox | `minijinja` sandboxing |
| Capability forgery | Spoofing | HMAC-SHA256 + constant-time | `CapabilityToken` integrity |
| Capability escalation | Elevation | Attenuation enforcement | `CapabilityTokenBuilder` attenuation |
| Replay attacks | Spoofing | Context nonce + expiry | `CapabilityToken.context_nonce` |
| Data at rest exposure | Info Disclosure | SQLCipher | `hkask-storage` |
| Supply chain compromise | Tampering | Pinned versions, `cargo deny` | `Cargo.toml` |
| Path traversal | Elevation | Path validation | `hkask-storage` guards |
| Spec tampering | Tampering | Ed25519 signing | `hkask-keystore` |
| Master key compromise | Info Disclosure | Argon2id memory-hardness, `Zeroizing` protection | `hkask-keystore/master_key.rs` |
| Vault file read | Info Disclosure | AES-256-GCM encryption at rest | `hkask-mcp-keystore` |
| Audit log tampering | Repudiation | Append-only + git CAS | `GitCas` + `NuEventStore` |

[^shostack-threat]: Shostack, A. (2014). *Threat Modeling: Designing for Security*. Wiley. STRIDE methodology.

---

## 3. User Sovereignty (Magna Carta)

The Magna Carta principle enforces user sovereignty:[^westin-data]

| Right | Implementation |
|-------|---------------|
| **Data ownership** | All data local, SQLCipher encrypted |
| **No cross-machine sync** | Local-first, git backup only |
| **Capability revocation** | User can revoke any granted capability |
| **Visibility control** | Private/public gating per data category |
| **Consent management** | `ConsentManager` tracks authorization |
| **Acquisition resistance** | Default `Maximum` resistance level |
| **Kill-zone detection** | VC investment < 0.5 after acquisition attempt → CNS alert |

**SovereigntyPort** (`crates/hkask-agents/src/ports/sovereignty.rs`):

```rust
pub trait SovereigntyPort {
    fn check(&self, data_category: DataCategory, operation: SovereigntyOperation, requester: &WebID) -> SovereigntyCheckResult;
    fn can_access(&self, data_category: DataCategory, requester: &WebID) -> bool;
    fn mark_acquisition_attempt(&mut self, details: &Value);
    fn update_vc_investment(&mut self, vc_investment: f32);
    fn is_compromised(&self) -> bool;
    fn grant_consent(&mut self);
    fn revoke_consent(&mut self);
    fn owner_webid(&self) -> WebID;
}
```

[^westin-data]: Westin, A. F. (1967). *Privacy and Freedom*. Atheneum. Informational self-determination.

---

## 4. CNS — Cybernetic Loop (7)

### 4.1 Cybernetic Nervous System

The CNS (`hkask-cns`, 2,039 LOC) provides runtime observability following Beer's Viable System Model:[^beer-vsm]

The Cybernetic loop (Loop 7) manages the Observability→Governance feedback cycle:
- **Observability** (Loop 4) is the sensing half — it detects anomalies and generates alerts
- **Governance** (Loop 3) is the acting half — it enforces policy changes in response
- The Curation loop (5) provides the decision-making agent (Curator) that reads Observability and writes Governance policy

```mermaid
graph TB
    subgraph CNS["CNS Runtime"]
        RT["CnsRuntime<br/>orchestrator"]
        ALG["AlgedonicManager<br/>alert escalation"]
        UVT["UnifiedVarietyTracker<br/>variety + bot metrics + sovereignty + goals"]
        RL["RateLimiter<br/>token bucket"]
        EN["EnergyBudget<br/>resource tracking"]
    end

    RT --> ALG
    RT --> UVT
    RT --> RL
    RT --> EN
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-TSO-001
verified_date: 2026-05-30
verified_against: crates/hkask-cns/src/runtime.rs:39-55; unified_tracker.rs
status: VERIFIED
-->

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Wiley.

### 4.2 Span Namespaces

Every capability invocation emits a `NuEvent` with typed `Span` (`event.rs:92-106`):

| Span | Variant | Covers |
|------|---------|--------|
| `cns.prompt.*` | `Prompt` | Template render, validate, outcome |
| `cns.tool.*` | `Tool` | Tool governance, invocation |
| `cns.agent_pod.*` | `AgentPod` | Pod lifecycle, delegation |
| `cns.connector.*` | `Connector` | External I/O (LLM, embeddings) |
| `cns.pipeline.*` | `Pipeline` | Memory pipeline operations |
| `cns.energy.*` | `Energy` | Energy budget tracking |
| `cns.review.*` | `Review` | Review queue operations |
| `cns.template.*` | `Template` | Template lifecycle |
| `cns.curation.*` | `Curation` | Curation operations |
| `cns.variety.*` | `Variety` | Variety counter tracking |
| `cns.killzone.*` | `KillZone` | User sovereignty kill-zone events |
| `cns.sovereignty.*` | `Sovereignty` | User sovereignty enforcement |
| `cns.goal.*` | `Goal` | Goal lifecycle operations |
| `cns.spec.*` | `Spec` | DDMVSS specification operations |
| `cns.memory.*` | (Pattern A) | Memory pipeline: encode, budget, decay, retract |

**Event structure:** `NuEvent` (`event.rs:27`) — id, timestamp, observer_webid, span, phase (Observe/Regulate/Outcome), observation, regulation, outcome, recursion_depth, parent_event, visibility.

### 4.3 Variety Counters

Following Ashby's Law of Requisite Variety:[^ashby-law]

| Counter | Type | Purpose |
|---------|------|---------|
| `VarietyCounter` | `u64` wrapper | Unique element count per category |
| `UnifiedVarietyTracker` | struct | Single SENSE point for domain variety (4.1), bot metrics (4.3), sovereignty events (4.4), and goal variety |

**Implementation:** `UnifiedVarietyTracker` (`unified_tracker.rs`), `VarietyMonitor` (`variety.rs`)

[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Wiley. "Only variety can absorb variety."

### 4.4 Algedonic Alerts

When variety deficit exceeds threshold, CNS escalates:

```mermaid
sequenceDiagram
    participant OBS as Observer
    participant RT as CnsRuntime
    participant ALG as AlgedonicManager
    participant GOV as Governance
    participant CUR as Curator/Human

    OBS->>RT: emit NuEvent (variety_count)
    RT->>ALG: check_algedonic(counters)
    alt deficit > threshold
        ALG->>ALG: create RuntimeAlert
        ALG->>GOV: escalate (severity)
        GOV->>CUR: notify Curator
        CUR->>GOV: issue directive (CalibrateThreshold)
        GOV->>RT: apply threshold change
    else within bounds
        ALG-->>RT: OK
    end
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-TSO-002
verified_date: 2026-05-31
verified_against: crates/hkask-cns/src/algedonic.rs:79; types/cns.rs:62; types/loops/cybernetics.rs
status: VERIFIED
-->

| Severity | Trigger | Action |
|----------|---------|--------|
| Info | Variety approaching threshold | Log |
| Warning | Deficit > 100 | Escalate to Curator |
| Critical | Deficit > 500 | Escalate to Human |

### 4.5 CNS Health

`CnsHealth` (`algedonic.rs:175`) provides aggregate status:
- Runtime status (active/degraded/down)
- Variety counter summary
- Active algedonic alerts
- Rate limiter status
- Review queue depth

**Accessible via:** `kask cns health` (CLI), `GET /api/v1/cns/health` (API), `cns_health()` (MCP)

### 4.6 Rate Limiting

Token bucket rate limiting prevents resource exhaustion:
- `CnsTokenBucket` (`rate_limit.rs:32`) — configurable capacity and refill
- `RateLimiter` (`rate_limit.rs:79`) — per-operation enforcement

### 4.7 Energy Budget

Energy tracking for resource-conscious execution:
- `EnergyBudget` (`energy.rs:55`) — allocation and consumption
- `EnergySpanType` (`energy.rs:25`) — operation categorization

---

## 5. Audit Trail

### 5.1 NuEvent Store

All CNS events persisted in `NuEventStore` (`hkask-storage/src/nu_event_store.rs:21`):
- Append-only event log with observer identity
- Queryable by span, time range, observer
- SQLCipher-encrypted SQLite

### 5.2 Git CAS Backup

Content-addressed storage via git (`hkask-storage/src/git_cas.rs:15`):
- BLAKE3 hashing for content addressing
- Git objects for immutable storage
- Provenance tracking via git history

### 5.3 Template Execution Audit

`AuditTrail` (`hkask-templates/src/audit.rs:87`) records:
- Template ID, version, rendering context
- Execution timing and outcome
- Capability tokens used
- Inference calls and model tier

---

## References

[^miller-robust]: Miller, M. S. (2006). *Robust Composition*. Johns Hopkins University.
[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Wiley.
[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Wiley.
[^shostack-threat]: Shostack, A. (2014). *Threat Modeling*. Wiley.
[^argon2]: Biryukov, A., et al. (2016). *Argon2*.
[^westin-data]: Westin, A. F. (1967). *Privacy and Freedom*. Atheneum.
