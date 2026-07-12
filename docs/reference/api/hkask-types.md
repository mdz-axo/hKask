---
title: "hkask-types — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-types — API Reference

Foundation types for the hKask agent platform. Provides canonical ID types, event primitives, visibility classifications, error taxonomies, macros, and span observability traits shared across all downstream crates.

## Public Modules

| Module | Description |
|---|---|
| `agent` | Agent kind and persona constraint types |
| `agent_paths` | Filesystem path conventions for agent storage |
| `agent_registry` | Agent registration records: `AgentDefinition`, `Charter`, `Contact`, `RegisteredAgent`, `Responsibility`, `Right`, `ScheduledTask`, `UserProfile` |
| `cns` | CNS span types (`CnsSpan`) and circuit state (`CircuitState`) |
| `crypto` | Cryptographic primitives including `Ed25519PublicKey` |
| `curation` | Sovereignty boundary types: `BoundaryClassification`, `DataCategory`, `DataSovereigntyBoundary`, `UserSovereigntyState` |
| `curator` | Curation configuration: `CurationThresholdConfig`, `CuratorDirective`, `CuratorHandle`, `EscalationSeverity` |
| `error` | Cross-cutting error types: `InfrastructureError`, `McpErrorKind`, `DatabaseErrorKind`, `NotFound`, `CapabilityDenied`, `DimensionMismatch` |
| `event` | Event primitives: `NuEvent`, `NuEventSink`, `Span`, `SpanKind`, `SpanNamespace`, `SpanCategory`, `CyclePhase` |
| `goal` | Goal state tracking: `GoalState` |
| `id` | Type-safe UUID identifier system: `Id<T>`, `IdKind`, `WebID`, and all concrete ID type aliases |
| `identity` | Agent identity types |
| `keychain_keys` | Keychain key storage |
| `loops` | Loop identifier: `LoopId` |
| `macros` | Shared macros: `enum_str_ops!` |
| `observable_span` | `ObservableSpan` trait for decoupled CNS observability |
| `retry` | Retry policy types |
| `secret` | Secret reference handling |
| `skill` | Skill polarity: `SkillPolarity` |
| `template` | LLM parameter types: `LLMParameters` |
| `template_type` | Template type classification: `TemplateType` |
| `time` | Time utilities |
| `transcript` | Transcript primitives: `TimedWord`, `TranscriptBundle`, `TranscriptSegment` |
| `visibility` | Access-control visibility types: `Visibility`, `Confidence`, `Dimension`, `AccessControl` |
| `sql_impls` | SQL support (feature-gated behind `sql`) |

## Key Public Types

### `Id<T: IdKind>`

Generic UUID-based identifier with phantom type parameter. `Id<BotKind>` and `Id<TemplateKind>` are distinct types — type-safe discrimination prevents accidental cross-type assignment.

**Methods:**
- `new() -> Self` — creates a new random UUID v4
- `from_uuid(uuid: Uuid) -> Self` — wraps an existing UUID
- `from_name(name: &str) -> Self` — deterministic UUID v5 derivation from a name
- `as_uuid(&self) -> Uuid` — returns the inner UUID

Implements `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `FromStr`, `Default`, `Display`.

### `IdKind`

Sealed marker trait for ID kind discrimination. Implemented by empty enum types (`TemplateKind`, `BotKind`, `TripleKind`, `EventKind`, `GoalKind`, `EmbeddingKind`, `UserKind`, `SovereigntyKind`, `PodIdKind`, `WalletKind`, `ApiKeyKind`, `EscalationKind`, `PhaseKind`, `CommentKind`, `BoardKind`, `ColumnKind`, `TaskKind`).

### Concrete ID Type Aliases

| Alias | Kind |
|---|---|
| `TemplateID` | `Id<TemplateKind>` |
| `BotID` | `Id<BotKind>` |
| `HMemId` | `Id<TripleKind>` |
| `EventID` | `Id<EventKind>` |
| `GoalID` | `Id<GoalKind>` |
| `EmbeddingID` | `Id<EmbeddingKind>` |
| `UserID` | `Id<UserKind>` |
| `SovereigntyId` | `Id<SovereigntyKind>` |
| `PodID` | `Id<PodIdKind>` |
| `WalletId` | `Id<WalletKind>` |
| `ApiKeyId` | `Id<ApiKeyKind>` |
| `EscalationID` | `Id<EscalationKind>` |
| `PhaseId` | `Id<PhaseKind>` |
| `CommentId` | `Id<CommentKind>` |
| `BoardId` | `Id<BoardKind>` |
| `ColumnId` | `Id<ColumnKind>` |
| `TaskId` | `Id<TaskKind>` |

### `WebID`

Unique identifier for agents (bots and replicants). Newtype wrapper over `Uuid`.

**Fields:** `WebID(Uuid)`

**Methods:**
- `new() -> Self` — random UUID v4
- `from_uuid(uuid: Uuid) -> Self` — wrap existing UUID
- `as_uuid(&self) -> Uuid` — extract inner UUID
- `for_agent_name(agent_name: &str) -> Self` — deterministic WebID from agent name
- `from_persona(persona_bytes: &[u8]) -> Self` — UUID v5 from persona bytes
- `from_persona_with_namespace(persona_bytes: &[u8], namespace: &str) -> Self` — UUID v5 with namespace isolation
- `redacted_display(&self) -> String` — first 8 hex chars with "..." suffix

Implements `From<BotID> for WebID`, `FromStr`, `Default`, `Display`.

### `NuEvent`

The canonical ν-event — the atomic unit of CNS observability.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `id` | `Uuid` | Unique event identifier |
| `timestamp` | `DateTime<Utc>` | Event timestamp |
| `observer_webid` | `WebID` | Observer agent identity |
| `span` | `Span` | CNS span (namespace + path) |
| `phase` | `CyclePhase` | Loop cycle phase when emitted |
| `observation` | `Option<String>` | Observed data |
| `regulation` | `Option<String>` | Regulatory action taken |
| `outcome` | `Option<String>` | Outcome classification |
| `recursion_depth` | `u8` | Nesting depth for recursive loops |
| `parent_event` | `Option<Uuid>` | Parent event ID for causal chains |
| `visibility` | `Visibility` | Access visibility |

**Constructors:** `new()`, with builder methods: `with_outcome()`, `with_regulation()`, `with_parent()`, `with_visibility()`.

### `NuEventSink`

Trait for persisting NuEvents. Single method: `fn persist(&self, event: &NuEvent)`.

### `ObservableSpan`

Trait for typed observability spans. Dyn-compatible (`Display + Debug + Send + Sync + 'static`).

**Methods:**
- `as_str(&self) -> &'static str` — canonical dot-separated namespace string (e.g., `"cns.tool.web_search"`)
- `emit(&self, operation: &str)` — emit a structured tracing event with `target = "cns"`

### `SpanKind`

Enum of CNS span kinds. Variants include: `ToolInvoked`, `ToolCompleted`, `ToolError`, `GasReserved`, `GasSettled`, `GasDepleted`, `CurationDirectiveAcknowledged`, `CurationEscalation`, `AgentPodRegistered`, `AgentPodActivated`, `AgentPodDeactivated`, `VarietyAlgedonicAlert`, `DepositCredited`, `ImpactVerified`, `ActionSubstituted`, `ActionBlocked`, `RegulatoryPlateauDetected`, `LoopQualityTelemetry`.

### `SpanNamespace`

Validated string wrapper for canonical CNS span namespaces (e.g., `"cns.tool"`, `"cns.inference"`). Constructed via `new()` (fallible) or `parse()` (returns `None` for invalid namespaces). Implements `From<CnsSpan>`.

### `CyclePhase`

Loop cycle phase enum: `Sense`, `Compute`, `Compare`, `Act`, `Verify`.

### `Visibility`

Three-tier access classification: `Private` (agent-specific, default), `Shared` (consent-bound), `Public` (universal). Methods: `as_str()`, `parse_str()`.

### `Confidence`

Newtype over `f64` clamped to `[0.0, 1.0]`. Methods: `new(value)`, `full()` (returns 1.0), `value()`, `memory_decay(days_since_recall, memory_life_days)` — applies Wozniak-Gorzelanczyk forgetting curve.

### `Dimension`

5W1H dimension enum: `Who`, `What`, `Where`, `When`, `Why`, `How`. Methods: `as_str()`.

### `AccessControl`

Value object bundling perspective, visibility, and owner WebID. Replaces repeated (perspective, visibility, owner) parameter triples.

**Fields:** `perspective: Option<WebID>`, `visibility: Visibility`, `owner_webid: WebID`

**Constructors:** `new(owner)`, `episodic(perspective, owner)`, `semantic(owner)`, `public(owner)`. Builder: `with_perspective()`, `with_visibility()`. Queries: `is_episodic()`, `is_semantic()`. Transformations: `to_semantic()`, `to_public()`, `without_perspective()`.

## Error Types

### `InfrastructureError`

Cross-cutting transport-layer error enum (`#[non_exhaustive]`).

**Variants:** `Database { message, kind }`, `Serialization(String)`, `LockPoisoned`, `NotFound(String)`, `Io(String)`.

Implements `From<serde_json::Error>`, `From<std::io::Error>`, `From<PoisonError<T>>`. Feature-gated `From<rusqlite::Error>` behind `sql`.

### `DatabaseErrorKind`

Recovery-path discrimination: `Connection`, `Query`, `Constraint`, `Migration`, `Other`.

### `McpErrorKind`

Semantic MCP tool error taxonomy (`#[non_exhaustive]`): `Internal`, `Unavailable`, `Timeout`, `NotFound`, `InvalidArgument`, `PermissionDenied`, `RateLimited`, `FailedPrecondition`. Methods: `is_retryable()`, `requires_intervention()`.

### Canonical Domain Error Types

- `NotFound { entity_type: &'static str, id: String }`
- `CapabilityDenied { reason: String }`
- `DimensionMismatch { expected: usize, actual: usize }`

## Macros

### `enum_str_ops!`

Canonical location: `crates/hkask-types/src/macros.rs`. Generates `as_str()` (returns PascalCase) and `parse_str()` (accepts both PascalCase and snake_case) for an enum. Usage:

```ignore
enum_str_ops!(SkillPolarity, {
    Generative => ("Generative", "generative"),
    Evaluative => ("Evaluative", "evaluative"),
});
```

## Feature Flags

| Flag | Effect |
|---|---|
| `sql` | Enables `sql_impls` module and `From<rusqlite::Error> for InfrastructureError` |

## Re-exports from Crate Root

`AgentKind`, `PersonaConstraints`, `AgentDefinition`, `Charter`, `Contact`, `RegisteredAgent`, `Responsibility`, `Right`, `ScheduledTask`, `UserProfile`, `CircuitState`, `Ed25519PublicKey`, `BoundaryClassification`, `DataCategory`, `DataSovereigntyBoundary`, `UserSovereigntyState`, `CurationThresholdConfig`, `CuratorDirective`, `CuratorHandle`, `EscalationSeverity`, `InfrastructureError`, `McpErrorKind`, `NuEvent`, `NuEventSink`, `GoalState`, all 17 ID type aliases, `WebID`, `LoopId`, `ObservableSpan`, `SkillPolarity`, `LLMParameters`, `TemplateType`, `TimedWord`, `TranscriptBundle`, `TranscriptSegment`, `Confidence`, `Dimension`, `Visibility`.
