# Security Architecture — hKask v0.21.0

Minimal security architecture documentation reflecting ADV-REVIEW-F2 remediation.

## Core Security Principles

1. **Zero-trust defaults**: No hardcoded secrets, no ambient authority
2. **Single capability primitive**: `CapabilityToken` with caveats (Miller-style)
3. **OCAP enforcement**: Token-based access control at all boundaries
4. **Deterministic identity**: WebIDs derived from persona content (UUID v5)
5. **Secure memory**: `Arc<Zeroizing<Vec<u8>>>` for secrets (no byte copying on Clone)

## Capability System

### Unified Primitive

All capabilities use `hkask_types::CapabilityToken`:
- HMAC-signed tokens with resource/action scoping
- Caveats for additional restrictions (expiration, operation, template, visibility)
- Attenuation chains with configurable depth limits
- Revocation tracking with persistent storage

### Enforcement Points

| Boundary | Enforcement | Location |
|----------|-------------|----------|
| MCP tools | `verify_tool_capability` | `hkask-mcp/dispatch.rs` |
| Template execution | `CapabilityAwareValidator` | `hkask-templates/capability_validator.rs` |
| ACP messaging | `AcpRuntime::verify_capability` | `hkask-agents/acp.rs` |
| Memory storage | `MemoryStoragePort` checks | `hkask-agents/adapters/memory_storage.rs` |

## Secret Management

### Okapi Integration

- **No hardcoded keys**: Removed `OKAPI_DEV_KEY` constant
- **Keystore resolution**: Environment → Keychain → Generate
- **Rotation**: Documented in `docs/architecture/security-architecture.md`

### ACP Secrets

- **Environment variable**: `HKASK_ACP_SECRET`
- **Keychain fallback**: `acp-secret` key
- **Auto-generation**: 32-byte hex string if not configured

## Observability

### CNS Spans

All capability mutations emit spans:
- `cns.cap.minted` — Token creation
- `cns.cap.attenuated` — Delegation with reduced authority
- `cns.cap.revoked` — Token revocation
- `cns.cap.verified_ok` / `cns.cap.verified_denied` — Verification outcomes

### Audit Trail

- `AuditLogPort` writes to both in-memory cache and SQLite storage
- Tracks A2A messages, capability operations, and lifecycle events
- Queryable by WebID and time range

## Federation Security

### Russell ACP Bridge

- JSON-RPC over stdio with macaroon authentication
- CNS spans emitted for cross-system capability translation
- Bidirectional ACP communication via `RussellAcpAdapter`

## Known Limitations

1. **No cross-machine ACP**: Transport layer designed for single-machine deployment
2. **No CRDT merge**: Revocation is centralized per runtime instance
3. **No hardware keystore**: Uses OS keychain (not TPM/SE)

## See Also

- `docs/plans/ADV-REVIEW-F2.md` — Adversarial review findings
- `docs/plans/IMPLEMENTATION-PLAN-F2.md` — Remediation tasks
- `docs/architecture/ports-inventory.md` — Hexagonal port inventory
