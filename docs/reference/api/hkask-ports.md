---
title: "hkask-ports — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-ports — API Reference

Hexagonal port traits for infrastructure abstractions. Domain crates depend on these port traits rather than on concrete implementations, enforcing the Authority DAG dependency direction.

## Public Modules

| Module | Description |
|---|---|
| `cns` | CNS-related port traits: `CircuitBreakerPort`, `CnsObserver`, `CnsStoragePort`, and types (`ConsolidationRequest`, `ConsolidationOutcome`, `DepletionSignal`, `BackpressureSignal`, `WeightedEvent`, `DecayConfig`) |
| `consent_port` | Consent record persistence: `ConsentPort`, `StoredConsentRecord` |
| `embedding` | Embedding generation errors: `EmbeddingGenerationError` |
| `embedding_port` | Embedding storage operations: `EmbeddingPort`, `StoredEmbedding` |
| `escalation` | Escalation port traits |
| `federation` | Federation networking: `FederationDispatch`, `FederationTransport`, `FederationSyncPort`, `FederationMessage`, `FederationDelta`, `FederatedTriple`, `ReplicaId` |
| `flowdef_validation` | FlowDef validation: `FlowDefValidationFinding`, `FlowDefValidationReport`, `validate_convergence_field()`, `validate_step_input_mapping()` |
| `git_cas` | Git CAS (Content-Addressable Storage) port |
| `inference_port` | LLM invocation boundary: `InferencePort`, `InferenceStreamChunk` |
| `inference_types` | Inference result types: `InferenceResult`, `InferenceError`, `InferenceUsage`, `ChatToolDefinition`, `ChatToolFunction`, `StructuredToolCall`, `TokenProb`, `TokenProbability`, `compute_confidence()` |
| `registry` | Registry types: `RegistryEntry`, `RegistryError`, `RegistryIndex`, `Skill`, `SkillRegistryIndex`, `SkillZone` |
| `registry_port` | Registry port trait |
| `tool` | Tool governance: `ToolPort`, `ToolPortError`, `ToolInfo` |

## Key Public Traits

### `InferencePort`

LLM invocation boundary. Uses `Pin<Box<dyn Future>>` for object safety (not `async_trait`). Implemented by `InferenceRouter` (hkask-inference) and blanket-implemented for `Arc<dyn InferencePort>`.

**Methods:**

```
fn generate(&self, prompt: &str, parameters: &LLMParameters, tools: Option<&[ChatToolDefinition]>)
    -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>

fn generate_with_model(&self, prompt: &str, parameters: &LLMParameters, model_override: Option<&str>, tools: Option<&[ChatToolDefinition]>)
    -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>

fn generate_n(&self, prompt: &str, parameters: &LLMParameters, n: usize)
    -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>

fn generate_stream(&self, prompt: &str, parameters: &LLMParameters, tools: Option<&[ChatToolDefinition]>)
    -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>>

fn generate_stream_with_model(&self, prompt: &str, parameters: &LLMParameters, model_override: Option<&str>, tools: Option<&[ChatToolDefinition]>)
    -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>>

fn generate_vision(&self, prompt: &str, images: &[String], parameters: &LLMParameters, model_override: Option<&str>)
    -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>
```

`generate_with_model` and `generate_vision` default to `generate()`. `generate_n` defaults to parallel `generate()` calls via `join_all`. `generate_stream` and `generate_stream_with_model` default to single-chunk wrappers.

### `ToolPort`

Governance membrane for MCP tool invocation. Implemented by `McpDispatcher` (hkask-mcp) and `GovernedTool` (hkask-cns).

**Methods:**

```
fn invoke(&self, server: &str, tool: &str, args: Value, token: &DelegationToken)
    -> impl Future<Output = Result<Value, ToolPortError>> + Send

fn discover_tools(&self) -> impl Future<Output = Vec<String>> + Send

fn get_tool_info(&self, tool_name: &str)
    -> impl Future<Output = Option<ToolInfo>> + Send
```

`invoke()` requires a `DelegationToken` (OCAP-gated). `discover_tools()` and `get_tool_info()` are intentionally unauthenticated — tool schemas are public metadata.

### `ToolPortError`

Error enum: `CapabilityDenied(String)`, `EnergyBudgetExceeded(String)`, `NotFound(String)`, `InvocationFailed(String)`.

### `CircuitBreakerPort`

Circuit breaker boundary trait. Methods: `allow_request() -> bool`, `record_success()`, `record_failure()`, `state() -> CircuitState`. Implemented by `CircuitBreaker` in hkask-cns.

### `CnsObserver`

Subscribes to CNS events by span namespace. Methods: `interest_mask() -> Vec<SpanNamespace>`, `on_event(&self, event: &NuEvent)`, `on_depletion(&self, signal: &DepletionSignal)`, `on_backpressure(&self, signal: &BackpressureSignal)`.

### `CnsStoragePort`

Storage port for CNS event queries. Abstracts the `NuEventStore` behind a trait.

**Methods:**
- `query_algedonic(since: DateTime<Utc>, limit: u64) -> Result<Vec<NuEvent>, InfrastructureError>`
- `replay_weighted(since: DateTime<Utc>, limit: u64, config: &DecayConfig) -> Result<Vec<WeightedEvent>, InfrastructureError>`
- `persist_cursor(key: &str, value: i64) -> Result<(), InfrastructureError>`
- `load_cursor(key: &str) -> Result<Option<i64>, InfrastructureError>`

### `ConsentPort`

Consent record persistence trait. Methods: `initialize_schema()`, `store(record: &StoredConsentRecord)`, `list_active() -> Vec<StoredConsentRecord>`.

### `FederationDispatch`

Federation lifecycle operations trait (`#[async_trait]`). Implemented by `FederationLinkManager` in hkask-federation.

**Methods:** `register_peer()`, `invite()`, `accept()`, `reject()`, `pause()`, `resume()`, `revoke()`, `leave()`, `dissolve()`, `linked_peers()`, `link_state()`.

### `EmbeddingPort`

Embedding storage operations. Methods: `store(entity_ref, embedding)`, `get(entity_ref)`, `search(query_embedding, limit)`, `delete(entity_ref)`.

## Key Public Types

### `InferenceStreamChunk`

Single chunk of streaming inference output.

**Fields:** `text_delta: String`, `model: String`, `finish_reason: Option<String>`, `usage: Option<InferenceUsage>`, `tool_calls: Vec<StructuredToolCall>`.

Implements `From<InferenceResult>`.

### `ToolInfo`

Canonical tool metadata for OCAP capability matching.

**Fields:** `name: String`, `description: String`, `input_schema: Value`, `server_id: String`, `required_capability: Option<String>`.

### `ConsolidationRequest` / `ConsolidationOutcome`

Request: `limit: usize`, `confidence_floor: Option<f64>`, `max_semantic_triples: Option<usize>`. Default limit: 100.

Outcome: `consolidated_count: usize`, `deleted_count: usize`, `failed_count: usize`.

### `DepletionSignal`

Emitted when an agent's energy budget is depleted: `agent: WebID`, `remaining: u64`, `cap: u64`, `usage_ratio: f64`.

### `BackpressureSignal`

Backpressure from a loop: `source: LoopId`, `reason: String`, `severity: f64`.

### `DecayConfig`

Per-domain decay constants for weighted event replay. Fields: `cybernetics_lambda`, `curation_lambda`, `inference_lambda`, `episodic_lambda`, `weight_threshold`. Default half-lives: cybernetics 300s, curation 900s, inference 120s, episodic 600s.

### `FederationMessage`

Enum: `SyncRequest { version_vector }`, `SyncResponse { deltas, version_vector }`, `InvitationRequest { from_replica, server_domain, matrix_domain, curator_matrix_id, message }`, `InvitationResponse { accepted, from_replica, reason }`.

### `FederatedTriple`

Minimal h_mem representation for federation sync: `entity: String`, `attribute: String`, `value: Value`, `confidence: f64`.

### `EmbeddingGenerationError`

Enum: `Connection(String)`, `Api(u16, String)`, `Json(String)`, `EmptyResponse`, `DimensionMismatch { expected, actual }`.

## Re-exports from Crate Root

`BackpressureSignal`, `CircuitBreakerPort`, `CnsObserver`, `CnsStoragePort`, `ConsolidationOutcome`, `ConsolidationRequest`, `DecayConfig`, `DepletionSignal`, `WeightedEvent`, `EmbeddingGenerationError`, `FlowDefValidationFinding`, `FlowDefValidationReport`, `validate_convergence_field()`, `validate_step_input_mapping()`, `InferencePort`, `InferenceStreamChunk`, `ChatToolDefinition`, `ChatToolFunction`, `InferenceError`, `InferenceResult`, `InferenceUsage`, `StructuredToolCall`, `TokenProb`, `TokenProbability`, `compute_confidence()`, `RegistryEntry`, `RegistryError`, `RegistryIndex`, `Skill`, `SkillRegistryIndex`, `SkillZone`, `ToolInfo`, `ToolPort`, `ToolPortError`.
