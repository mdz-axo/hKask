---
title: "Database Connection Lifecycle"
diataxis: reference
---

# Database Connection Lifecycle

Reference flowchart tracing the path from environment variable to constructed stores. Covers SQLite (stable) and PostgreSQL (planned v0.32) paths.

Related: [ADR-043](../architecture/ADR-043-database-driver.md), [Class Diagram](class-database-driver.md)

```mermaid
flowchart TD
    A([Start: kask init]) --> B[Read HKASK_DB_PROVIDER]
    B --> C{Provider?}
    C -->|sqlite| D[Read HKASK_DB_PATH<br/>+ HKASK_DB_PASSPHRASE]
    C -->|postgres| E{PostgresDriver<br/>implemented?}
    E -->|No| F[Error: not yet<br/>implemented]
    E -->|Yes| G[Parse connection URL]
    D --> H[parse_db_provider]
    G --> H
    H --> I[ServiceConfig::db_config]
    I --> J[Build DbConfig]
    J --> K{DbConfig variant?}
    K -->|Sqlite| L[open_or_repair<br/>verify SQLCipher database]
    K -->|Postgres| M[Connect via sqlx<br/>pgvector]
    L --> N[Database::conn_arc<br/>Arc Mutex Connection]
    M --> N
    N --> O[SqliteDriver::new<br/>Arc dyn DatabaseDriver]
    O --> P[HMemStore::from_driver]
    P --> Q[driver.as_any<br/>downcast_ref extract conn]
    Q --> R[Self::new raw_conn]
    O --> S[EmbeddingStore::from_driver]
    S --> T[driver.as_any<br/>downcast_ref extract conn]
    T --> U[VectorBackend conn + dim]
    R --> V[MemoryLoopForwarder<br/>from_connection]
    U --> V
    V --> W([Stores Ready])
    F --> X([Error Exit])
```
