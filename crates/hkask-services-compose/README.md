# hkask-services-compose — Prompt Composition

Prompt composition service with cognition configuration: embedding-based selection, retrieval augmentation, validation, and prompt assembly.

**Version:** v0.31.0 | **Crate:** `hkask-services-compose`

## Key Types

| Type | Purpose |
|------|---------|
| `CognitionConfig` | Configuration for the cognition pipeline (embedding, retrieval, validation) |
| `EmbeddingSection` | Embedding model selection and parameters |
| `RetrievalSection` | Retrieval strategy and augmentation settings |
| `ValidationSection` | Output validation criteria |
| `ComposeRequest` | Full composition request with context and configuration |
| `ComposeResult` | Assembled prompt with validation results |
| `CentroidValidation` | Centroid-based validation of composed output |
| `ComposeService` | Primary service struct |

## Dependencies

- `hkask-types` — CNS spans, nu-event, WebID
- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-inference` — Inference router for embedding/retrieval
- `hkask-memory` — Semantic memory for context retrieval
- `hkask-storage` — Persistent storage
- `hkask-ports` — Hexagonal port traits
- `minijinja` — Template rendering
