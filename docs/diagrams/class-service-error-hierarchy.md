---
title: "Class Diagram — ServiceError Hierarchy"
audience: [architects, developers]
last_updated: 2026-07-09
version: "0.31.0"
status: "Active"
domain: "Composition"
mds_categories: ["composition", "domain"]
diataxis: reference
source: "crates/hkask-services-core/src/error/mod.rs, crates/hkask-api/src/error.rs, crates/hkask-types/src/error.rs"
---

# ServiceError Hierarchy

The hKask error architecture follows a three-layer Miller separation design (see `crates/hkask-types/src/error.rs` doc comment). At the base, `InfrastructureError` carries transport-level failures (Database, Serialization, LockPoisoned, Io, NotFound) shared across all crates. Domain crate errors (InferenceError, EmbeddingGenerationError, WalletError) compose from `InfrastructureError` via `#[from]`. At the top, `ServiceError` unifies 49 domain-tagged variants into a single canonical error vocabulary. `ServiceError::domain()` returns a `DomainKind` for routing and observability; `ServiceError::kind()` returns a semantic `ErrorKind` (NotFound, Forbidden, etc.) for HTTP status mapping to `ApiError`.

```mermaid
classDiagram
    direction TB

    %% ── Foundation Types (hkask-types) ──────────────────────────────────
    namespace types {
        class InfrastructureError {
            <<enumeration>>
            Database(message, kind)
            Serialization(String)
            LockPoisoned
            NotFound(String)
            Io(String)
        }
        class DatabaseErrorKind {
            <<enumeration>>
            Connection
            Query
            Constraint
            Migration
            Other
        }
        class McpErrorKind {
            <<enumeration>>
            Internal
            Unavailable
            Timeout
            NotFound
            InvalidArgument
            PermissionDenied
            RateLimited
            FailedPrecondition
        }
    }

    %% ── ServiceError Core (hkask-services-core) ────────────────────────
    namespace core {
        class ServiceError {
            <<enumeration>>
            +domain() DomainKind
            +kind() ErrorKind
        }
        class ErrorKind {
            <<enumeration>>
            NotFound
            Conflict
            Forbidden
            BadRequest
            ServiceUnavailable
        }
        class DomainKind {
            <<enumeration>>
            Agent
            Consent
            Curator
            Federation
            Inference
            Infrastructure
            Memory
            Pod
            Storage
            User
            Wallet
        }
    }

    %% ── ServiceError Variants by Domain ─────────────────────────────────
    namespace curator {
        class EscalationNotFound {
            source: Option~BoxErr~
            message: String
        }
        class Escalation {
            source: Option~BoxErr~
            message: String
        }
        class Metacognition {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace agent {
        class AgentNotFound {
            source: Option~BoxErr~
            message: String
        }
        class InvalidAgentType {
            source: Option~BoxErr~
            message: String
        }
        class AgentRegistrationFailed {
            source: Option~BoxErr~
            message: String
        }
        class A2A {
            source: Option~BoxErr~
            message: String
        }
        class AgentRegistry {
            source: Option~BoxErr~
            message: String
        }
        class AgentRegistryStore {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace consent {
        class Consent {
            source: Option~BoxErr~
            message: String
            kind: Option~ErrorKind~
        }
        class ConsentDenied {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace storage {
        class Storage {
            source: Option~BoxErr~
            message: String
        }
        class Registry {
            source: Option~BoxErr~
            message: String
        }
        class Template {
            source: Option~BoxErr~
            message: String
            kind: Option~ErrorKind~
        }
        class GoalRepo {
            source: Option~BoxErr~
            message: String
        }
        class UserStore {
            source: Option~BoxErr~
            message: String
        }
        class ConsentStore {
            source: Option~BoxErr~
            message: String
        }
        class SovereigntyStore {
            source: Option~BoxErr~
            message: String
        }
        class Archival {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace memory {
        class HMem {
            source: Option~BoxErr~
            message: String
        }
        class EpisodicMemory {
            source: Option~BoxErr~
            message: String
        }
        class SemanticMemory {
            source: Option~BoxErr~
            message: String
        }
        class Consolidation {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace infrastructure {
        class Infra {
            wraps: InfrastructureError
        }
        class Config {
            source: Option~BoxErr~
            message: String
        }
        class RegistryInitFailed {
            source: Option~BoxErr~
            message: String
        }
        class RegistryLoadFailed {
            source: Option~BoxErr~
            message: String
        }
        class Matrix {
            source: Option~BoxErr~
            message: String
        }
        class RateLimited {
            source: Option~BoxErr~
            message: String
        }
        class Keystore {
            source: Option~BoxErr~
            message: String
        }
        class Gas {
            source: Option~BoxErr~
            message: String
        }
        class Cns {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace pod {
        class PodNotFound {
            source: Option~BoxErr~
            message: String
        }
        class Pod {
            source: Option~BoxErr~
            message: String
            kind: Option~ErrorKind~
        }
    }

    namespace inference {
        class InferencePort {
            source: Option~BoxErr~
            message: String
            retryable: bool
        }
        class Embedding {
            source: Option~BoxErr~
            message: String
            retryable: bool
        }
    }

    namespace user {
        class UserNotFound {
            source: Option~BoxErr~
            message: String
        }
        class LoginFailed {
            source: Option~BoxErr~
            message: String
        }
        class InvalidPassphrase {
            source: Option~BoxErr~
            message: String
        }
        class ValidationError {
            source: Option~BoxErr~
            message: String
        }
        class InvalidWebID {
            source: Option~uuid::Error~
            message: String
        }
        class Forbidden {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace wallet {
        class Wallet {
            source: Option~BoxErr~
            message: String
        }
        class McpTool {
            kind: McpErrorKind
            server: String
            tool: String
            message: String
        }
        class Embed {
            source: Option~BoxErr~
            message: String
        }
        class Compose {
            source: Option~BoxErr~
            message: String
        }
        class Skill {
            source: Option~BoxErr~
            message: String
        }
        class Verification {
            source: Option~BoxErr~
            message: String
        }
    }

    namespace federation {
        class Federation {
            source: Option~BoxErr~
            message: String
        }
    }

    %% ── API Layer (hkask-api) ───────────────────────────────────────────
    namespace api {
        class ApiError {
            <<enumeration>>
            NotFound(resource, id)
            Unauthorized(reason)
            Forbidden(reason)
            BadRequest(message)
            Conflict(message)
            ServiceUnavailable(reason)
            Internal(message)
        }
        class ServiceErrorResponse {
            +newtype wrapper
            +from(ServiceError)
            +into_response()
        }
    }

    %% ── Composition: variant → ServiceError ─────────────────────────────
    ServiceError *-- curator : "3 variants"
    ServiceError *-- agent : "6 variants"
    ServiceError *-- consent : "2 variants"
    ServiceError *-- storage : "8 variants"
    ServiceError *-- memory : "4 variants"
    ServiceError *-- infrastructure : "9 variants"
    ServiceError *-- pod : "2 variants"
    ServiceError *-- inference : "2 variants"
    ServiceError *-- user : "6 variants"
    ServiceError *-- wallet : "6 variants"
    ServiceError *-- federation : "1 variant"

    %% ── Infra variant wraps InfrastructureError ─────────────────────────
    Infra ..> InfrastructureError : wraps
    InfrastructureError *-- DatabaseErrorKind : classifies
    McpTool ..> McpErrorKind : carries

    %% ── Domain classification ───────────────────────────────────────────
    ServiceError ..> DomainKind : domain()
    ServiceError ..> ErrorKind : kind()

    %% ── Consent/Pod/Template carry optional ErrorKind ───────────────────
    Consent ..> ErrorKind : optional kind field
    Pod ..> ErrorKind : optional kind field
    Template ..> ErrorKind : optional kind field

    %% ── From domain crate errors → ServiceError ─────────────────────────
    ServiceError ..> InferencePort : "From<InferenceError>"
    ServiceError ..> Embedding : "From<EmbeddingGenerationError>"
    ServiceError ..> InvalidWebID : "From<uuid::Error>"
    ServiceError ..> Wallet : "From<WalletError>"
    ServiceError ..> Infra : "From<PoisonError<T>>"
    ServiceError ..> Infra : "From<InfrastructureError> (#[from])"

    %% ── ApiError mapping ────────────────────────────────────────────────
    ServiceErrorResponse ..> ApiError : delegates to
    ServiceError ..> ServiceErrorResponse : "From<ServiceError>"
    ServiceError ..> ApiError : "Into<ApiError> → HTTP status"

    %% ── Infra is transparent via #[error(transparent)] ──────────────────
    ServiceError ..> Infra : "#[error(transparent)]"
```

## Entity Counts

| Layer | Type | Count |
|-------|------|-------|
| `ServiceError` | enum variants | 49 |
| `DomainKind` | enum variants | 11 |
| `ErrorKind` | enum variants | 5 |
| `InfrastructureError` | enum variants | 5 |
| `DatabaseErrorKind` | enum variants | 5 |
| `McpErrorKind` | enum variants | 8 |
| `ApiError` | enum variants | 7 |

## ErrorKind → HTTP Status Mapping (via `ServiceError → ApiError`)

| `ErrorKind` | HTTP Status | Example ServiceError variants |
|-------------|-------------|-------------------------------|
| `NotFound` | 404 | EscalationNotFound, AgentNotFound, PodNotFound, UserNotFound |
| `Conflict` | 409 | AgentRegistrationFailed |
| `Forbidden` | 403 | ConsentDenied, A2A, InvalidWebID |
| `BadRequest` | 400 | InvalidAgentType, InvalidPassphrase, ValidationError |
| `ServiceUnavailable` | 503 | InferencePort(retryable), Embedding(retryable), RateLimited, Keystore |

Variants with an explicit `kind: Option<ErrorKind>` field (Consent, Pod, Template) dispatch to the corresponding HTTP status; when `kind` is `None`, they fall through to `500 Internal Server Error`.

## `From` Impls (Domain → ServiceError)

| Source Error | ServiceError Variant | Notes |
|---|---|---|
| `InferenceError` | `InferencePort` | Sets `retryable` based on variant (Connection/CircuitOpen = true) |
| `EmbeddingGenerationError` | `Embedding` | Sets `retryable` based on variant (Connection/Api = true) |
| `WalletError` | `Wallet` | Wraps source in `Box<dyn Error>` |
| `uuid::Error` | `InvalidWebID` | Preserves source as `Option<uuid::Error>` (typed) |
| `PoisonError<T>` | `Infra(LockPoisoned)` | Generic `T`, delegates to `InfrastructureError::LockPoisoned` |
| `InfrastructureError` | `Infra` | Via `#[from]` attribute — transparent pass-through |

## DIAGRAM_ALIGNMENT

Diagram generated by **diataxis-diagram class** skill from `crates/hkask-services-core/src/error/mod.rs` (v0.31.0). Variant count, domain groupings, and relationships verified against the `domain()`, `kind()`, and `From` impl blocks at time of generation.

### Verification checklist

- [x] 49 `ServiceError` variants match the source enum definition
- [x] 11 `DomainKind` variants correspond to `domain()` match arms
- [x] 5 `ErrorKind` variants: NotFound, Conflict, Forbidden, BadRequest, ServiceUnavailable
- [x] 5 `InfrastructureError` variants: Database, Serialization, LockPoisoned, NotFound, Io
- [x] 8 `McpErrorKind` variants match `crates/hkask-types/src/error.rs`
- [x] 7 `ApiError` variants: NotFound, Unauthorized, Forbidden, BadRequest, Conflict, ServiceUnavailable, Internal
- [x] 6 domain `From` impls verified (InferenceError, EmbeddingGenerationError, uuid::Error, WalletError, PoisonError, InfrastructureError)
- [x] `Consent`, `Pod`, `Template` carry optional `kind: Option<ErrorKind>` for runtime dispatch
- [x] `InferencePort` and `Embedding` carry `retryable: bool`
- [x] `McpTool` variant carries `McpErrorKind` for retryability and observability

## Cross-Reference

- [`FUNCTIONAL_SPECIFICATION.md`](../architecture/core/FUNCTIONAL_SPECIFICATION.md) — §2 error contract definitions, §5 contract anchoring
- [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) — P5 (no pass-through abstractions), P7 (interface minimalism)
- [`crates/hkask-services-core/src/error/mod.rs`](../../crates/hkask-services-core/src/error/mod.rs) — canonical source
- [`crates/hkask-types/src/error.rs`](../../crates/hkask-types/src/error.rs) — InfrastructureError, McpErrorKind
- [`crates/hkask-api/src/error.rs`](../../crates/hkask-api/src/error.rs) — ApiError mapping, ServiceErrorResponse newtype
