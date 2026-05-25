# Security Architecture — hKask v0.21.0

Comprehensive security architecture documentation reflecting ADV-REVIEW-F2 adversarial review remediation.

## Table of Contents

1. [Core Security Principles](#core-security-principles)
2. [Threat Model](#threat-model)
3. [Capability System](#capability-system)
4. [OCAP Enforcement](#ocap-enforcement)
5. [Secret Management](#secret-management)
6. [Identity and WebIDs](#identity-and-webids)
7. [Observability and Audit](#observability-and-audit)
8. [Federation Security](#federation-security)
9. [Transport Security](#transport-security)
10. [Security Invariants](#security-invariants)
11. [Known Limitations](#known-limitations)

---

## Core Security Principles

hKask implements a **zero-trust, capability-based security model** inspired by Mark Miller's object-capability (OCAP) security and Bruce Schneier's defense-in-depth principles.

### 1. Zero-Trust Defaults

- **No hardcoded secrets**: All cryptographic keys loaded from environment or keystore
- **No ambient authority**: Every operation requires explicit capability presentation
- **Fail-closed**: Denied by default; capabilities must be explicitly granted
- **No wildcards**: Capability strings like `"*"` are rejected at registration

### 2. Single Capability Primitive

All access control uses `hkask_types::CapabilityToken`:
- HMAC-SHA256 signed tokens (constant-time comparison via `subtle::ConstantTimeEq`)
- Resource/action scoping (e.g., `tool:execute`, `template:render`)
- Caveats for additional restrictions (expiration, operation, template, visibility)
- Attenuation chains with configurable depth limits (default: 7 levels)
- Revocation tracking with persistent SQLite storage

### 3. OCAP Enforcement

Token-based access control at all boundaries:
- MCP tool invocation
- Template execution
- ACP message routing
- Memory storage operations

### 4. Deterministic Identity

WebIDs derived from persona content using UUID v5:
- Same persona YAML → same WebID (across processes)
- Enables audit trail continuity and capability binding
- Root authority WebID derived from fixed `"hkask-root-authority"` persona

### 5. Secure Memory

Secrets wrapped in `Arc<Zeroizing<Vec<u8>>>`:
- `Clone` shares the `Arc`, not the bytes (no secret duplication)
- Memory zeroized on drop via `zeroize` crate
- Prevents secret leakage through core dumps or swap

---

## Threat Model

### Attack Surfaces

| Surface | Threat | Mitigation |
|---------|--------|------------|
| **Capability forgery** | Attacker creates fake tokens | HMAC-SHA256 signatures with shared secret; constant-time comparison |
| **Capability replay** | Reuse of revoked tokens | Persistent `RevocationStore` checked on every verification |
| **Privilege escalation** | Attenuation chain abuse | `max_attenuation` limit (default 7); chain verification |
| **Secret extraction** | Memory dump or swap | `Zeroizing<Vec<u8>>` with `Arc` sharing; no byte copying |
| **Identity spoofing** | Fake WebIDs | Deterministic derivation from persona content (UUID v5) |
| **DoS via messaging** | Message flood | Per-agent rate limiting (default: 100 msg/min) |
| **Wildcard abuse** | Overly broad capabilities | Wildcards rejected at registration; explicit capabilities only |
| **Cross-system attack** | Russell bridge compromise | Macaroon authentication; CNS span emission for federation |

### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                      hKask Runtime                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Agent Pods  │  │  ACP Runtime │  │ MCP Dispatch │      │
│  │  (Bots/Rep)  │  │  (Messaging) │  │  (Tools)     │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                 │               │
│         └─────────────────┼─────────────────┘               │
│                           │                                 │
│                    ┌──────▼───────┐                         │
│                    │  Capability  │                         │
│                    │   Checker    │                         │
│                    └──────┬───────┘                         │
│                           │                                 │
│                    ┌──────▼───────┐                         │
│                    │  Root Auth   │                         │
│                    │  (Secret)    │                         │
│                    └──────────────┘                         │
└─────────────────────────────────────────────────────────────┘
                           │
                    ┌──────▼───────┐
                    │   Russell    │  (External ACP)
                    │   Bridge     │
                    └──────────────┘
```

---

## Capability System

### CapabilityToken Structure

```rust
pub struct CapabilityToken {
    pub id: String,                          // Unique token ID (content-addressed)
    pub resource: CapabilityResource,        // Tool, Template, Manifest, Registry, Cascade, Spec
    pub resource_id: String,                 // Specific resource (e.g., "tool:search")
    pub action: CapabilityAction,            // Read, Write, Execute, Render, Compose, Attenuate, Validate
    pub delegated_from: WebID,               // Issuer WebID
    pub delegated_to: WebID,                 // Holder WebID
    pub signature: String,                   // HMAC-SHA256 over all fields
    pub expires_at: Option<i64>,             // Unix timestamp (None = no expiry)
    pub attenuation_level: u8,               // Current depth (0 = root)
    pub max_attenuation: u8,                 // Maximum allowed depth (default: 7)
    pub context_nonce: String,               // Execution context binding
    pub caveats: Vec<Caveat>,                // Additional restrictions
}
```

### Caveat Types

Caveats are additive restrictions that limit capability scope:

| Caveat Type | Purpose | Example |
|-------------|---------|---------|
| `expiration` | Time-based expiry | `Caveat::expiration(1735689600)` |
| `operation` | Specific operation allowed | `Caveat::operation("generate")` |
| `template` | Template ID scope | `Caveat::template("template:greeting")` |
| `visibility` | Visibility level | `Caveat::visibility("private")` |

### Capability Lifecycle

```
1. MINT
   RootAuthority.create_root_token(resource, resource_id, action, holder)
   → CapabilityToken with attenuation_level=0

2. ATTENUATE (Delegate)
   parent_token.attenuate(new_holder, secret, current_time)
   → Child token with attenuation_level=parent+1, caveats preserved

3. VERIFY
   AcpRuntime.verify_capability(token)
   → Check signature, expiry, revocation, attenuation chain

4. REVOKE
   AcpRuntime.revoke_capability(token_id)
   → Add to RevocationStore (persistent SQLite)
   → Emit cns.cap.revoked span

5. ENFORCE
   verify_tool_capability(token, holder, resource, resource_id, action)
   → OCAP-idiomatic: holder presents token, checker verifies
```

### Verification Flow

```rust
// OCAP-idiomatic verification (T02)
pub fn verify_tool_capability(
    &self,
    token: &CapabilityToken,
    expected_holder: &WebID,
    resource: CapabilityResource,
    resource_id: &str,
    action: CapabilityAction,
) -> bool {
    // 1. Verify signature (constant-time comparison)
    if !self.verify_with_time(token, current_time) {
        return false;
    }
    
    // 2. Verify holder matches
    if token.delegated_to != *expected_holder {
        return false;
    }
    
    // 3. Verify resource/action match
    if !token.is_valid_for(resource, resource_id, action) {
        return false;
    }
    
    // 4. Check revocation (persistent store)
    if self.revocation_store.is_revoked(&token.id).await? {
        return false;
    }
    
    true
}
```

---

## OCAP Enforcement

### Enforcement Points

| Boundary | Enforcement Function | Location |
|----------|---------------------|----------|
| **MCP tools** | `verify_tool_capability` | `hkask-mcp/src/dispatch.rs` |
| **Template execution** | `CapabilityAwareValidator` | `hkask-templates/src/capability_validator.rs` |
| **ACP messaging** | `AcpRuntime::verify_capability` | `hkask-agents/src/acp.rs` |
| **Memory storage** | `MemoryStoragePort::store_artifact` | `hkask-agents/src/adapters/memory_storage.rs` |
| **Curator pipeline** | `CuratorPipeline::evaluate` | `hkask-templates/src/curator_pipeline.rs` |

### MCP Tool Enforcement

```rust
// hkask-mcp/src/dispatch.rs
pub async fn invoke_tool(
    &self,
    server_id: &str,
    tool_name: &str,
    arguments: Value,
    token: &CapabilityToken,
) -> Result<Value, McpError> {
    // 1. Verify capability token
    if !self.capability_checker.verify_tool_capability(
        token,
        &token.delegated_to,
        CapabilityResource::Tool,
        tool_name,
        CapabilityAction::Execute,
    ) {
        return Err(McpError::CapabilityDenied(
            "Invalid capability token for tool invocation".to_string(),
        ));
    }
    
    // 2. Dispatch to transport
    let transport = self.get_transport(server_id)?;
    transport.call(server_id, tool_name, arguments).await
}
```

### Template Execution Enforcement

```rust
// hkask-templates/src/capability_validator.rs
pub fn validate_capability(
    &self,
    token: &CapabilityToken,
    template_id: &str,
) -> Result<(), TemplateError> {
    // Verify token grants template:render for this template_id
    if !self.checker.verify_tool_capability(
        token,
        &token.delegated_to,
        CapabilityResource::Template,
        template_id,
        CapabilityAction::Render,
    ) {
        return Err(TemplateError::CapabilityDenied(
            format!("No capability for template: {}", template_id),
        ));
    }
    Ok(())
}
```

---

## Secret Management

### Okapi Integration

**Before (T05):** Hardcoded 32-byte key in source code.

**After:** Keystore resolution chain:

```rust
// hkask-ensemble/src/okapi_integration.rs
fn load_okapi_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    // 1. Try environment variable (CI/testing)
    if let Ok(key) = std::env::var("HKASK_OKAPI_KEY") {
        return Ok(Zeroizing::new(key.into_bytes()));
    }
    
    // 2. Try OS keychain
    let keychain = Keychain::default();
    match keychain.retrieve_by_key("okapi-cap-key") {
        Ok(secret) => Ok(Zeroizing::new(secret.into_bytes())),
        Err(KeychainError::NotFound(_)) => {
            // 3. Generate new key and store
            let generated: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            let hex_key = hex::encode(&generated);
            keychain.store_by_key("okapi-cap-key", &hex_key)?;
            Ok(Zeroizing::new(generated))
        }
        Err(e) => Err(e),
    }
}
```

### ACP Secrets

```rust
// hkask-agents/src/acp.rs
pub fn from_env(rate_limit_config: Option<RateLimitConfig>) -> Result<Self, AcpError> {
    let secret_str = std::env::var("HKASK_ACP_SECRET")
        .map_err(|_| AcpError::SecretNotConfigured)?;
    
    let secret = Arc::new(Zeroizing::new(secret_str.into_bytes()));
    // ...
}
```

### Secret Rotation

**Okapi key rotation procedure:**
1. Generate new key: `openssl rand -hex 32`
2. Store in keychain: `kask keystore store okapi-cap-key <new-key>`
3. Restart hKask services
4. Old tokens will fail verification (expected)
5. Re-issue tokens to active agents

---

## Identity and WebIDs

### Deterministic Derivation (T06)

```rust
// hkask-types/src/id.rs
pub fn from_persona(persona_bytes: &[u8]) -> Self {
    let namespace = Uuid::parse_str("686b6173-6b2d-7065-7273-6f6e612d6e73")
        .expect("Invalid namespace UUID");
    Self(Uuid::new_v5(&namespace, persona_bytes))
}
```

**Properties:**
- Same persona YAML → same WebID (across processes, restarts)
- Different personas → different WebIDs
- Enables audit trail continuity and capability binding

### Root Authority

```rust
// hkask-agents/src/acp.rs
pub fn new(secret: &[u8], rate_limit_config: Option<RateLimitConfig>) -> Self {
    let root_persona = b"hkask-root-authority";
    let root_webid = WebID::from_persona(root_persona);
    let root_authority = Arc::new(RootAuthority::new(root_webid, secret));
    // ...
}
```

---

## Observability and Audit

### CNS Spans (T13)

All capability mutations emit spans for debugging and monitoring:

| Span | Emitted On | Data |
|------|-----------|------|
| `cns.cap.minted` | Token creation | `token_id`, `holder`, `resource`, `action` |
| `cns.cap.attenuated` | Delegation | `parent_id`, `child_id`, `attenuation_level`, `holder` |
| `cns.cap.revoked` | Revocation | `token_id` |
| `cns.cap.verified_ok` | Successful verification | `token_id`, `holder`, `resource` |
| `cns.cap.verified_denied` | Failed verification | `token_id`, `holder`, `resource` |

### Audit Trail

**AuditLogPort** writes to both in-memory cache and SQLite storage:

```rust
// hkask-agents/src/acp.rs
impl AuditLogPort for AuditLog {
    async fn log(&self, entry: AuditLogEntry) {
        // Write to persistent storage if available
        if let Some(ref store) = self.store {
            let storage_entry = hkask_storage::AuditEntry::new(
                &entry.from.to_string(),
                &entry.message_type,
                &entry.event_type,
                &entry.correlation_id,
            );
            let _ = store.insert(&storage_entry);
        }
        
        // Write to in-memory cache
        let mut entries = self.entries.write().await;
        entries.push(entry);
        // ...
    }
}
```

**Queryable by:**
- WebID (sender or recipient)
- Time range
- Event type (sent, received, verified, denied)

---

## Federation Security

### Russell ACP Bridge (T14)

**Architecture:**
- JSON-RPC over stdio with macaroon authentication
- Russell spawned as child process
- Bidirectional ACP communication via `RussellAcpAdapter`

**Security properties:**
- Macaroon token required for all requests
- CNS spans emitted for cross-system capability translation
- Loopback-only communication (no network exposure)

```rust
// hkask-agents/src/adapters/russell_acp.rs
pub struct RussellAcpAdapter {
    child: Mutex<Option<Child>>,
    russell_binary: String,
    macaroon_token: Option<String>,
    cns_emitter: Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>,
}
```

---

## Transport Security

### MCP Transport Layer (T09)

Three transport implementations:

| Transport | Use Case | Security |
|-----------|----------|----------|
| `InProcessMcpTransport` | Co-located servers | No network; in-process handlers |
| `StdioMcpTransport` | Child process servers | JSON-RPC over stdin/stdout; no network |
| `HttpMcpTransport` | Remote servers | HTTPS with capability tokens |

### Loopback HTTP Transport

**Security constraints:**
- Binds only to loopback addresses (127.0.0.1, ::1)
- Rejects non-loopback addresses at construction
- No network exposure

```rust
// hkask-agents/src/adapters/loopback_http_transport.rs
pub fn new(addr: SocketAddr) -> Result<Self, AcpError> {
    if !addr.ip().is_loopback() {
        return Err(AcpError::NonLoopbackRefused(addr.ip()));
    }
    // ...
}
```

---

## Security Invariants

### Guaranteed Properties

1. **No wildcard capabilities**: `AcpRuntime::register_agent` rejects `"*"`
2. **No ambient authority**: Every operation requires capability presentation
3. **Constant-time signature comparison**: Prevents timing attacks
4. **Persistent revocation**: Revoked tokens remain revoked across restarts
5. **Deterministic identity**: Same persona → same WebID
6. **Secure memory**: Secrets zeroized on drop; no byte copying on Clone
7. **Async purity**: No `block_in_place`/`block_on` in library code
8. **Typed errors**: No `unwrap()` on hot paths

### Invariant Checks

| Invariant | Check Location | Enforcement |
|-----------|---------------|-------------|
| No wildcards | `AcpRuntime::register_agent` | `AcpError::WildcardCapabilityNotAllowed` |
| Signature valid | `CapabilityToken::verify` | `subtle::ConstantTimeEq` |
| Not revoked | `AcpRuntime::verify_capability` | `RevocationStore::is_revoked` |
| Attenuation limit | `CapabilityToken::attenuate` | `attenuation_level < max_attenuation` |
| Loopback only | `LoopbackHttpTransport::new` | `AcpError::NonLoopbackRefused` |

---

## Known Limitations

1. **No cross-machine ACP**: Transport layer designed for single-machine deployment
2. **No CRDT merge**: Revocation is centralized per runtime instance (no gossip protocol)
3. **No hardware keystore**: Uses OS keychain (not TPM/SE)
4. **No capability delegation across systems**: Russell bridge uses separate macaroon auth
5. **Rate limiting is per-instance**: No distributed rate limiting across multiple hKask instances

---

## See Also

- `docs/plans/ADV-REVIEW-F2.md` — Adversarial review findings
- `docs/plans/IMPLEMENTATION-PLAN-F2.md` — Remediation tasks
- `docs/architecture/ports-inventory.md` — Hexagonal port inventory
- `docs/architecture/hKask-architecture-master.md` — System architecture
