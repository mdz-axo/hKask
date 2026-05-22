# Dependency Decision Log

## Workspace Dependencies

| Dependency | Version | Added | Justification | Used By |
|------------|---------|-------|---------------|---------|
| `tokio` | 1.51 | 2026-05-19 | Async runtime for all async operations | 21 crates |
| `rmcp` | 1 | 2026-05-19 | MCP protocol implementation | 21 crates |
| `rusqlite` | 0.39 | 2026-05-19 | SQLite with SQLCipher for encrypted storage | hkask-storage |
| `sqlite-vec` | 0.1 | 2026-05-19 | Vector similarity search | hkask-storage |
| `minijinja` | 2 | 2026-05-19 | Jinja2 templating for templates | hkask-templates |
| `regex-lite` | 0.1 | 2026-05-22 | Lightweight regex for template security (83% smaller than regex) | hkask-templates |
| `keyring` | 3 | 2026-05-19 | Cross-platform OS keychain access | hkask-keystore |
| `gix` | 0.81 | 2026-05-19 | Git operations for template registry | hkask-templates |
| `blake3` | 1 | 2026-05-19 | Fast cryptographic hashing | hkask-types |
| `ed25519-dalek` | 2 | 2026-05-19 | Digital signatures for WebID | hkask-types |
| `aes-gcm` | 0.10 | 2026-05-19 | AES-256-GCM encryption | hkask-keystore |
| `argon2` | 0.5 | 2026-05-19 | Password/key derivation | hkask-keystore |
| `acp-runtime` | 0.1 | 2026-05-19 | Agent communication protocol | hkask-agents |
| `axum` | 0.8 | 2026-05-19 | HTTP API framework | hkask-api |
| `utoipa` | 5.5 | 2026-05-19 | OpenAPI documentation | hkask-api |
| `serde` | 1 | 2026-05-19 | Serialization framework | All crates |
| `tracing` | 0.1 | 2026-05-19 | Observability and logging | All crates |

## Removed Dependencies

| Dependency | Removed | Reason | Alternative |
|------------|---------|--------|-------------|
| `nalgebra` | 2026-05-22 | Never used, vector ops handled by sqlite-vec | `sqlite-vec` |
| `ndarray` | 2026-05-22 | Never used, vector ops handled by sqlite-vec | `sqlite-vec` |
| `once_cell` | 2026-05-22 | Not needed in Rust 2024 | `std::sync::OnceLock` |
| `secret-service` | 2026-05-22 | Redundant with keyring | `keyring` |
| `tokio-util` | 2026-05-22 | Not yet needed | `tokio::sync` |
| `base64` | 2026-05-22 | Not implemented, hex encoding used | `hex` |
| `url` | 2026-05-22 | Not yet needed | Built-in via reqwest/axum |

## Decision Criteria

### Add When
1. **Two or more crates** need the dependency
2. **Security critical** (crypto, hashing) — prefer established crates
3. **Performance critical** — benchmark required
4. **No std alternative** — justify why std is insufficient

### Remove When
1. **Zero usage** detected by `cargo udeps`
2. **Superseded** by better alternative
3. **Security vulnerability** with no patch
4. **Feature not implemented** within 30 days

### Retain When
1. **Single crate** but security critical
2. **Planned feature** with implementation timeline
3. **Platform abstraction** (e.g., keyring for OS keychain)

## Review Cadence

| Review | Frequency | Owner |
|--------|-----------|-------|
| **Unused deps** | Weekly (CI) | Automated |
| **Security advisories** | Weekly (CI) | Automated |
| **Decision log** | Monthly | Human |
| **Major version upgrades** | Quarterly | Human |

---
*Document generated: 2026-05-22*
*Last reviewed: 2026-05-22*
*Part of hKask Dependency Governance (Phase 2)*