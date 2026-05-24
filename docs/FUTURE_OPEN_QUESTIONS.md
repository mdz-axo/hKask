# Future Open Questions

**Version:** 0.21.0
**Last Updated:** 2026-05-24
**Status:** Pre-alpha — decisions deferred post-MVP

---

This document catalogs architectural and design decisions that remain open for hKask. These questions are explicitly out of scope for v0.21.0 MVP but must be resolved before production deployment or federation.

---

## 1. Streaming Inference Protocol

**Question:** Should hKask adopt a streaming inference protocol for long-running LLM generations?

**Context:** The current `InferencePort::generate()` returns a complete `InferenceResult`. For templates that produce large outputs or need incremental processing (e.g., code generation, document drafting), a streaming interface would reduce time-to-first-token.

**Design decisions needed:**
- Streaming trait signature: `async fn generate_stream(&self, prompt: &str, params: &LLMParameters) -> impl Stream<Item = InferenceChunk>`
- Backpressure strategy when consumer is slower than producer
- How streaming interacts with `CircuitBreaker` and `RateLimiter`
- CNS span emission granularity: per-chunk vs. per-completion
- Okapi SSE/WebSocket protocol selection
- Curator evaluation of partial outputs for early termination

**Dependencies:** Okapi streaming support, async runtime constraints

---

## 2. MCP Server Process Lifecycle

**Question:** How should hKask manage the lifecycle of MCP server processes?

**Context:** The 16 MCP servers (inference, condenser, web, scholar, ocap, keystore, cns, git, registry, gml, spec, github, fmp, telnyx, fal, rss-reader) each need process management — startup, health checks, graceful shutdown, restart on failure.

**Design decisions needed:**
- Process supervision model: supervisor tree vs. process pool
- Health check protocol: heartbeat, liveness probe, readiness probe
- Restart policy: exponential backoff, max restart count, circuit breaker integration
- Resource limits: memory, CPU, file descriptors per server
- Configuration: per-server startup args, environment variables
- CNS integration: lifecycle spans (`cns.mcp.started`, `cns.mcp.crashed`, `cns.mcp.restarted`)
- Dependency ordering between servers (e.g., keystore before ocap)

**Dependencies:** Process management library selection, CNS alerting pipeline

---

## 3. Embedding Model Versioning

**Question:** How should hKask handle embedding model upgrades without invalidating existing vector indices?

**Context:** `hkask-storage` uses `sqlite-vec` for vector similarity search. If the embedding model changes, all stored vectors become incompatible. The semantic memory pipeline (`hkask-memory`) depends on consistent embedding dimensions.

**Design decisions needed:**
- Versioning scheme: model name + version tag (e.g., `text-embedding-3-small@v1`)
- Migration strategy: re-embed on upgrade vs. dual-index during transition
- Storage format: per-vector model metadata in `sqlite-vec` auxiliary columns
- Backward compatibility: support queries across multiple embedding model versions
- Cost model: re-embedding full corpus on model change
- A/B testing support for model comparison

**Dependencies:** `hkask-storage` schema evolution, Okapi embeddings endpoint

---

## 4. Russell ↔ hKask Bidirectional ACP

**Question:** Should the Russell mapping be bidirectional, allowing hKask templates to be exported back to Russell format?

**Context:** The current `RussellMapper` is one-directional (Russell → hKask). A bidirectional mapping would enable template portability between hKask and Russell ecosystems, but requires solving semantic equivalence for constructs that don't have 1:1 mappings.

**Design decisions needed:**
- Reverse mapping: hKask `TemplateType` → Russell skill category
- Fidelity levels: lossless round-trip vs. best-effort approximation
- Lexicon translation: hLexicon ↔ Russell symptom vocabulary
- Version compatibility: hKask v0.21.x ↔ Russell versions
- ACP (Agent Communication Protocol) for cross-system template exchange
- OCAP implications: capability tokens don't transfer between systems

**Dependencies:** Russell API stability, hLexicon governance model

---

## 5. Distributed CNS Architecture

**Question:** How should CNS observability scale across multiple hKask instances?

**Context:** CNS currently operates within a single process. Multi-agent scenarios (ensemble, distributed pods) and future federation require cross-instance span aggregation, variety counter synchronization, and algedonic alert propagation.

**Design decisions needed:**
- Span transport: in-process → network (gRPC, message queue, shared database)
- Span aggregation: centralized collector vs. peer-to-peer gossip
- Clock synchronization: logical clocks (Lamport) vs. hybrid logical clocks (HLC)
- Variety counter federation: global variety deficit across instances
- Algedonic alert propagation: broadcast vs. hierarchical escalation
- Sampling strategy: high-volume span sampling to control overhead
- Retention policy: hot/warm/cold tier storage for CNS events

**Dependencies:** Network transport selection, `hkask-cns` serialization format

---

## 6. Capability Token Persistence

**Question:** How should OCAP capability tokens survive process restarts?

**Context:** `CapabilityToken` instances are currently in-memory. If the hKask process restarts, all granted capabilities are lost, requiring re-authorization. This breaks long-running bot workflows and agent delegation chains.

**Design decisions needed:**
- Persistence backend: SQLCipher table in `hkask-storage` vs. OS keychain via `hkask-keystore`
- Token format: JWT-style signed tokens vs. opaque database references
- Revocation: immediate invalidation on restart vs. grace period
- Encryption: AES-256-GCM via keystore for stored tokens
- Attenuation chain preservation: stored tokens maintain delegation provenance
- Garbage collection: expired/revoked token cleanup

**Dependencies:** `hkask-keystore` API stability, `hkask-storage` schema

---

## 7. Template Hot-Reload via Git CAS

**Question:** Can hKask hot-reload templates from Git content-addressable storage without process restart?

**Context:** Templates are stored in the unified registry with `template_type` discriminator. Git CAS provides content-addressable, verifiable storage. Hot-reload would enable template updates without downtime.

**Design decisions needed:**
- Watch mechanism: Git hook (post-receive) vs. filesystem watcher vs. polling
- Atomicity: how to swap templates without mid-request inconsistency
- Validation: pre-reload validation to prevent corrupt templates from being loaded
- Rollback: automatic rollback on validation failure
- CNS integration: `cns.template.reloaded`, `cns.template.rollback` spans
- Cache invalidation: compiled template cache, lexicon index, registry index
- Concurrency: read-write lock during reload vs. copy-on-write

**Dependencies:** `hkask-mcp-git` API, registry locking strategy

---

## 8. Formal OCAP Attenuation Proof

**Question:** Can hKask provide a formal proof that capability attenuation is sound — i.e., that a derived capability never exceeds the permissions of its parent?

**Context:** The OCAP model allows capability attenuation (reducing permissions) but not escalation. This property is currently enforced by code convention. A formal proof would enable security audits and compliance requirements.

**Design decisions needed:**
- Formalism: type-level proof (Rust type system) vs. runtime verification vs. external proof assistant
- Attenuation algebra: partial order on `CapabilityResource` × `CapabilityAction`
- Proof strategy: structural induction on delegation chain
- Runtime enforcement: capability check on every `invoke` and `generate`
- Audit trail: ν-event recording of all capability derivations
- Composition: proving attenuation holds across template composition chains

**Dependencies:** `hkask-types` capability model stability, formal methods expertise

---

## 9. Multi-Okapi Federation Routing

**Question:** How should hKask route inference requests across multiple Okapi instances?

**Context:** A production deployment may have multiple Okapi instances with different models, latency profiles, and cost structures. hKask needs a routing layer that selects the optimal instance per request.

**Design decisions needed:**
- Routing strategy: model-affinity, latency-based, cost-based, load-balanced
- Service discovery: static configuration vs. dynamic registry
- Failover: primary/backup vs. anycast
- Model catalog federation: unified catalog across Okapi instances
- Circuit breaker per instance: independent breaker state per endpoint
- CNS integration: `cns.connector.route_selected` spans
- Rate limiting: per-instance quotas vs. global quota with distributed token bucket

**Dependencies:** `OkapiConfig` federation extensions, `hkask-cns` routing spans

---

## 10. hLexicon Governance Automation

**Question:** How should the hLexicon vocabulary evolve — who approves new terms, and how are they propagated?

**Context:** The hLexicon (`hkask-types/src/lexicon.rs`) defines the canonical vocabulary for template discovery and semantic search. Currently, terms are added manually. As the template corpus grows, automated governance becomes necessary.

**Design decisions needed:**
- Governance model: Curator-only vs. proposal-vote vs. meritocratic
- Proposal format: structured YAML with term, domain, synonyms, relationships
- Validation: consistency checks (no duplicate terms, no circular synonyms)
- Propagation: how new terms reach running instances (hot-reload, restart required)
- Deprecation: sunset process for obsolete terms
- Versioning: hLexicon schema version, backward compatibility
- CNS integration: `cns.lexicon.term_added`, `cns.lexicon.term_deprecated` spans
- Automation: Curator-driven term extraction from new templates

**Dependencies:** hLexicon schema stability, registry hot-reload capability

---

## Decision Template

For each open question, resolution should produce:

1. **Decision Record** — ADR (Architecture Decision Record) in `docs/adr/`
2. **Implementation Plan** — Crate-level changes, dependency updates
3. **CNS Spans** — New span definitions for observability
4. **Test Strategy** — Unit + integration test coverage plan

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*These questions are deferred by design — MVP first, federation later.*
