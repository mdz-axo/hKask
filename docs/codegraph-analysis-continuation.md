# Code Graph Analysis — Continuation Prompt

## Context

A comprehensive RDF code graph of the entire hKask codebase has been constructed at `hKask/docs/hKask-codegraph.rdf`. It covers all 26 crates (11 core + 15 MCP servers), the registry (10 bots, 16+ manifests, 10+ templates), 30+ API routes, and 35+ port traits. The analysis below represents findings from deep exploration of every source file.

**The task:** Analyze the code graph for opportunities to simplify the codebase without impairing any essential or required functionality. Produce actionable recommendations organized by: structural redundancy, security hardening, transparency improvements, and program efficiency.

## Findings So Far

### 1. STRUCTURAL REDUNDANCY — Duplicate Types & Overlapping Systems

**1a. Dual `DataCategory` enums (hkask-types)**
Two `DataCategory` enums exist in `hkask-types`:
- `sovereignty.rs::DataCategory` — simple unit variants (`EpisodicMemory`, `SemanticMemory`, `PersonalContext`, etc.)
- `sovereignty/category.rs::DataCategory` — tagged-union variants (`Episodic { owner, encrypted }`, `Semantic { scope }`, etc.)

Both are re-exported from `lib.rs` via `pub use sovereignty::*`. The tagged-union version is strictly more expressive; the simple version is used in `DataSovereigntyBoundary` but the tagged version isn't used anywhere in storage. This creates confusion about which is canonical.

**Recommendation:** Unify to the tagged-union `DataCategory` in `category.rs`. Add conversion `From<simple::DataCategory> -> category::DataCategory` if needed, then deprecate and remove the simple version. Update `DataSovereigntyBoundary` to use the tagged version.

**1b. Triple span/categorization system (3 enums, 2 crates)**
Three overlapping span categorization enums exist:
- `hkask_types::event::Span` — 10 variants with `String` payloads (Prompt, Tool, AgentPod, Connector, Pipeline, Energy, Review, Sovereignty, Goal, Spec)
- `hkask_types::cns::CnsSpan` — 11 unit variants (Tool, Prompt, AgentPod, Connector, Template, Curation, Variety, KillZone, Sovereignty, Goal, Spec)
- `hkask_cns::spans::SpanCategory` — 10 unit variants (Connector, Pipeline, Tool, Prompt, AgentPod, Energy, Review, Sovereignty, Goal, Spec)

The mapping between them is done ad-hoc with `span_to_category()` and `span_from_columns()` in `hkask-storage::nu_event_store`. This is fragile — adding a span category requires updating three enums and two conversion functions.

**Recommendation:** Unify to a single `Span` enum in `hkask-types` that carries both the category and the path string. Remove `CnsSpan` and `SpanCategory` entirely. Add a `category()` method that returns the category portion. The CNS module should consume `Span` directly.

**1c. Six separate `RetryConfig` structs**
Retry configuration is defined independently in 6 places:
- `hkask_types::cns::RetryConfig` — `max_retries, initial_delay_ms, max_delay_ms, multiplier, retryable_status`
- `hkask_templates::okapi_config::OkapiRetryConfig` — `max_retries, backoff_base_ms, max_delay_ms, retryable_status`
- `hkask_templates::csp::CspCspRetryConfig` — `max_retries, initial_delay_ms, max_delay_ms`
- `hkask_templates::error::ErrorErrorRetryConfig` — `max_retries, base_delay_ms, max_delay_ms`
- `hkask_mcp::dispatch::McpMcpRetryConfig` — `max_retries, backoff_base, retryable_status`
- `hkask_ensemble::resilience::EnsembleEnsembleRetryConfig` — `max_retries, initial_delay, max_delay, multiplier`

These are all the same concept with slightly different field names.

**Recommendation:** Unify into a single `RetryConfig` in `hkask-types` (it's already there as `cns::RetryConfig`). Have all other crates reference it. Remove the 5 duplicates. If domain-specific fields are needed, extend via composition rather than re-declaration.

**1d. Two `RateLimiter` implementations**
- `hkask_cns::rate_limit::RateLimiter<K>` — generic, token-bucket, uses `parking_lot::Mutex`
- `hkask_mcp_web::types::rate_limit::RateLimiter` — string-keyed, sliding window, uses `tokio::sync::Mutex`

Different algorithms (token bucket vs sliding window) but serving the same conceptual purpose.

**Recommendation:** Keep both but extract a shared `RateLimitStrategy` trait to `hkask-types` or `hkask-cns`, so the web server can use the existing CNS rate limiter with a configurable strategy. At minimum, consolidate the configuration types.

**1e. Two `TokenBucket` implementations**
- `hkask_types::cns::TokenBucket` — `tokens: f64, max_tokens: f64, refill_rate: f64, last_refill: Instant`
- `hkask_cns::rate_limit::CnsTokenBucket` — `tokens: u32, last_refill: Instant, config: RateLimitConfig`

One uses `f64` tokens with a refill rate; the other uses `u32` tokens with a refill interval. Both are token-bucket rate limiters.

**Recommendation:** Remove `TokenBucket` from `hkask-types::cns` (it's not used outside the type definition). Keep the one in `hkask-cns::rate_limit` as the implementation, which is the actual runtime.

**1f. Duplicate `RussellMapper` (two crates)**
Two nearly identical `RussellMapper` structs exist:
- `hkask-cli/russell_mapper.rs::RussellMapper` — has `config + SpanEmitter`, used at CLI level
- `hkask-templates/russell_mapper.rs::RussellMapper` — has `config` only, used in template pipeline

Both have their own `RussellMappingConfig`, `FieldMappings`, `MappedTemplate`, etc. The CLI version adds `cns: SpanEmitter`.

**Recommendation:** Move the canonical `RussellMapper` to `hkask-templates` (where it logically belongs). Make `SpanEmitter` optional or inject it. Remove the CLI copy and import from `hkask-templates`.

**1g. Two mock adapter sets in `hkask-testing`**
- `ports/mock_adapter.rs` — `MockInferenceAdapter`, `MockMcpAdapter`, `MockCnsAdapter`, `MockCnsAdapterMut`
- `test_harnesses/mocks.rs` — `MockInferencePort`, `MockMcpPort`, `MockCnsPort`, `TestMocks`

Both implement the same port traits (`SyncInferencePort`, `McpPort`, `CnsPort`). The first set uses `Cell<usize>` for call counts; the second uses `Arc<RwLock<HashMap>>`.

**Recommendation:** Consolidate to a single set in `test_harnesses/mocks.rs` (the more complete implementation). Remove `ports/mock_adapter.rs` or make it a thin re-export.

**1h. Orphaned code in `hkask-testing`**
- `security/test_capability.rs` exists but has no `security/mod.rs` and isn't declared in `lib.rs` — unreachable
- 7 integration test files not declared in `integration_tests/mod.rs` — unreachable
- Many workspace dependencies declared in `Cargo.toml` but with no `use` statements in reachable source files

**Recommendation:** Either integrate the orphaned files or delete them. Remove unused `Cargo.toml` dependencies.

**1i. Deprecated `evaluate_access` function**
`hkask_types::visibility::evaluate_access` is marked `#[deprecated]` with note to use `AccessEvaluator::evaluate_request`. But it's still exported and callable.

**Recommendation:** Remove the deprecated function entirely. It's been superseded by the more capable `AccessEvaluator` system.

---

### 2. SECURITY HARDENING

**2a. Fixed master key derivation salt**
`hkask_keystore::master_key::MASTER_KEY_SALT = b"hkask-master-202"` — a fixed, known salt for Argon2id. This means any two users with the same master passphrase derive the same key, which weakens the system against precomputation attacks.

**Recommendation:** Generate a random salt per user and store it alongside the encrypted vault. The `Database` struct already stores a random salt (`[u8; 16]`); apply the same pattern to the master key derivation. This is a breaking change to the key format, so add a version byte and migration path.

**2b. Three separate capability token systems with different signing**
- `hkask_types::capability::CapabilityToken` — HMAC-SHA256, 7-level attenuation
- `hkask_types::goal_capability::GoalCapabilityToken` — HMAC-SHA256, separate HMAC context
- `hkask_ensemble::okapi_capability` — custom HMAC-SHA256 operations (`create_okapi_capability`, `verify_okapi_capability`)
- `hkask_mcp_gml::capability::CapabilityToken` — Ed25519 signing

Four separate capability systems, two different signing algorithms, with overlapping purposes.

**Recommendation:** Unify capability token creation/verification into `hkask-types::capability` as the single canonical system. The `GoalCapabilityToken` should be replaced by `CapabilityToken` with appropriate `CaveatContext`. The Okapi-specific operations in `hkask-ensemble` should call into `CapabilityTokenBuilder`. The GML server's Ed25519 signing is appropriate for its MWC domain but should be clearly documented as a separate trust domain.

**2c. `OcapServer` stores secret as plain `Vec<u8>`**
In `hkask-mcp-ocap`, `OcapServer { secret: Vec<u8>, ... }` — the HMAC key is stored as a plain heap allocation without `Zeroize` protection. If the server is dumped, the key is exposed.

**Recommendation:** Use `Zeroizing<Vec<u8>>` (from the `zeroize` crate, already a workspace dependency) for the secret field. This is the same pattern used in `AcpRuntime` and `PodManager`.

**2d. Multiple `.expect()` calls for secret derivation**
Five places call `.expect()` on key derivation, which will panic at runtime:
- `AcpRuntime::default()` — ACP secret
- `SoapInferenceConfig::from_env()` — capability key  
- `config.rs::resolve_acp_secret()` — ACP secret
- `SecurityGateway::default()` — MCP security key
- `OcapServer` creation — OCAP secret

Runtime panics in production are a security-adjacent reliability risk.

**Recommendation:** Replace all `.expect()` with proper error propagation. The bootstrap sequence in `hkask-cli` should handle key derivation failures gracefully and report them to the user. Add a `KeyDerivationError` variant to `CliError`.

---

### 3. TRANSPARENCY IMPROVEMENTS

**3a. `GoalMemory` is in-memory only — data loss risk**
`GoalMemory` stores semantic and episodic data in `Arc<RwLock<HashMap<String, GoalMemory>>>`. This means goal data is lost on process restart. Meanwhile, `EpisodicMemory` and `SemanticMemory` properly persist to `TripleStore`/`EmbeddingStore` (SQLite).

The `GoalMemoryPort` trait exists but `GoalMemory` doesn't implement persistence.

**Recommendation:** Implement a `SqliteGoalMemory` that implements `GoalMemoryPort` using `GoalSemanticMemory`/`GoalEpisodicMemory` stored as triples in the `TripleStore`. This gives goal memory the same durability as other memory systems.

**3b. Storage layer uses coarse `Arc<Mutex<Connection>>` everywhere**
Every storage struct (`TripleStore`, `EmbeddingStore`, `BlobStore`, `AgentRegistryStore`, `AuditLogStore`, `NuEventStore`, `SovereigntyBoundaryStore`, `StandingSessionStore`, `SqliteGoalRepository`, `MetacognitionStore`, `SqliteSpecStore`, `UserStore`) wraps the SQLite connection in `Arc<Mutex<Connection>>`. This serializes all database access for each store, which is a bottleneck for concurrent agent pods.

**Recommendation:** Migrate to `rusqlite::Connection` with WAL mode enabled and connection pooling. Use `r2d2` or `deadpool` for connection pooling. The `Database` struct should expose a `Connection` pool rather than a single `Arc<Mutex<Connection>>`. This is a P2 optimization that can be done incrementally.

**3c. `EnsembleChatManager` double-nested locking**
`EnsembleChatManager { chats: Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>> }` — this is a RwLock of a HashMap of Arc-RwLock-EnsembleChat. Accessing a chat requires acquiring two locks.

**Recommendation:** Use `dashmap::DashMap<String, EnsembleChat>` or a simple `HashMap` behind a single lock. The `Arc<RwLock<EnsembleChat>>` pattern creates unnecessary indirection.

**3d. Port trait proliferation — 35+ traits across crates**
The codebase defines 35+ port traits. Some overlap significantly:
- `InferencePort` (async, `hkask-templates`) vs `SyncInferencePort` (sync, `hkask-templates`) vs `OkapiClientTrait` (async, `hkask-ensemble`) vs `InferenceClient` (async, `hkask-ensemble`) — 4 inference interfaces
- `CnsEmit` (`hkask-cns`) vs `CnsQueryPort` (`hkask-agents`) vs `CnsPort` (re-export from `hkask-templates`) vs `ObservabilityPort` (`hkask-types`) — 4 CNS interfaces
- `McpPort` (`hkask-templates`) vs `McpTransport` (`hkask-mcp`) vs `MCPRuntimePort` (`hkask-agents`) — 3 MCP interfaces
- `MemoryPort` (`hkask-templates`) vs `MemoryStoragePort` (`hkask-agents`) — 2 memory interfaces
- `CapabilityProviderPort` (`hkask-templates`) vs `CapabilityQueryPort` (`hkask-ensemble`) — 2 capability query interfaces

**Recommendation:** Consolidate to canonical interfaces:
- `InferencePort` should be the single async inference trait; `SyncInferencePort` can be a blanket impl
- `CnsEmit` should be the single CNS emission trait; `CnsQueryPort` should extend it
- `McpPort` should be the single MCP port trait; `McpTransport` is the transport layer beneath it
- `MemoryStoragePort` should subsume `MemoryPort`

This is a P3 refactor that reduces cognitive load without changing behavior.

---

### 4. PROGRAM EFFICIENCY

**4a. Template cascade `MAX_CASCADE_DEPTH = 7` is hardcoded**
Both `hkask-templates::cascade::MAX_CASCADE_DEPTH` and `hkask-templates::ports::DEFAULT_MATROSHKA_LIMIT` are `7`. The cascade engine checks depth per-invocation. The matroshka limit on `CompositionTemplate` is a different concept (template nesting) but uses the same value.

**Recommendation:** Make `MAX_CASCADE_DEPTH` configurable via `CascadeConfig` (it already is — `cascade_limits.max_depth`). The `DEFAULT_MATROSHKA_LIMIT` in `ports.rs` should reference the cascade config value rather than duplicating the constant.

**4b. `OkapiInference` in `hkask-templates` duplicates circuit breaker from `hkask-ensemble`**
`OkapiInference` has its own `CircuitBreaker` and retry logic. `hkask-ensemble::resilience` also has a `CircuitBreaker` with `CircuitBreakerConfig`. Both use `AtomicU32` for state.

**Recommendation:** The `CircuitBreaker` in `hkask-templates::resilience` is the canonical implementation (it's re-exported). `OkapiInference` should use it directly rather than potentially duplicating.

**4c. `hkask-mcp-web` has its own `RateLimiter` and `CacheKey/ResponseCache`**
The web server implements its own rate limiting and response caching, which are conceptually the same as what `hkask-cns::rate_limit` and a simple LRU would provide.

**Recommendation:** For the rate limiter, use `hkask-cns::RateLimiter` (or its `StringRateLimiter` type alias). For the cache, consider a shared `moka` or `mini-moka` crate if response caching becomes a cross-server need. For now, this is P4.

**4d. `hkask-mcp-web` is 5x larger than any other MCP server**
The web MCP server has 6 provider implementations (Brave, Exa, Firecrawl, Tavily, SerAPI, Browserbase, plus raw_fetch), its own rate limiter, cache, freshness normalization, RRF ranking, URL validation, and HTML stripping. At ~2500 lines, it's by far the largest MCP server.

**Recommendation:** Extract the provider pool, caching, and ranking into a shared `hkask-mcp-web-core` or into `hkask-mcp` itself. The URL validation (`validate_url`, `UrlValidationConfig`) is already in `hkask-mcp::security` — use it from there instead of duplicating.

---

### 5. PRIORITY RANKING

| Priority | Finding | Impact | Effort |
|----------|---------|--------|--------|
| **P1** | Dual `DataCategory` enums | Confusion, bugs | Low |
| **P1** | Triple span system | Fragility, maintenance | Medium |
| **P1** | Fixed master key salt | Security vulnerability | Medium |
| **P1** | `OcapServer` secret not zeroized | Key exposure risk | Low |
| **P1** | `.expect()` on secret derivation | Runtime panics | Medium |
| **P2** | 6 duplicate RetryConfig structs | Maintenance burden | Low |
| **P2** | Dual RussellMapper | Code duplication | Low |
| **P2** | Two TokenBucket impls | Confusion | Low |
| **P2** | Deprecated `evaluate_access` | Dead code | Trivial |
| **P2** | Orphaned testing modules | Dead code | Trivial |
| **P2** | GoalMemory not persisted | Data loss risk | Medium |
| **P2** | `Arc<Mutex<Connection>>` bottleneck | Concurrency ceiling | High |
| **P3** | 35+ port traits, 4 inference interfaces | Cognitive load | Medium |
| **P3** | Two RateLimiter impls | Algorithm divergence | Low |
| **P3** | Duplicate mock adapters in testing | Maintenance | Low |
| **P3** | 4 capability token systems | Security surface area | Medium |
| **P3** | Double-nested lock in EnsembleChatManager | Performance | Low |
| **P4** | Web MCP server size | Maintainability | High |
| **P4** | OkapiInference circuit breaker duplication | Minor duplication | Low |
| **P4** | Hardcoded cascade depth constants | Config flexibility | Trivial |

---

### NEXT STEPS

For the continuing agent:

1. **Read the RDF file** at `hKask/docs/hKask-codegraph.rdf` for the full graph
2. **Read the architecture spec** at `hKask/docs/architecture/hKask-architecture-master.md` for design constraints
3. **Read the principles** at `hKask/docs/architecture/PRINCIPLES.md` for the P1-P7 and C1-C7 constraints
4. **Verify each finding** by checking the actual source files before proposing changes
5. **Produce concrete patches** (or at minimum, detailed change specifications) for the P1 items first
6. **Ensure no change violates** the headless-system constraint (no visual UI), the OCAP security model, or the CNS observability model
7. **Write a final report** at `hKask/docs/codegraph-simplification-plan.md` with:
   - Each finding with file paths and line numbers
   - Proposed change with before/after code sketches
   - Impact analysis (what breaks, what improves)
   - Priority ranking

The most impactful and safest changes to start with are: removing the deprecated `evaluate_access`, removing the orphaned test modules, unifying the `DataCategory` enums, and zeroizing the `OcapServer` secret. These are all low-risk, high-clarity improvements.