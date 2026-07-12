---
title: "Database Driver Class Diagram"
diataxis: reference
---

# Database Driver Class Diagram

Reference diagram showing the `DatabaseDriver` trait hierarchy, store relationships, and connection management types introduced in v0.31.

Related: [ADR-043](../architecture/ADR-043-database-driver.md), [Database Providers](../architecture/database-providers.md)

```mermaid
classDiagram
    namespace Driver_Trait {
        class DatabaseDriver {
            <<interface>>
            +execute(sql, params) usize
            +execute_batch(sql)
            +query(sql, params) Vec~DbRow~
            +query_optional(sql, params) Option~DbRow~
            +provider() DbProvider
            +transaction() TransactionHandle
            +as_any() &dyn Any
        }
    }
    namespace SQLite_Impl {
        class SqliteDriver {
            -conn: Arc~Mutex~Connection~~
            +new(conn) SqliteDriver
        }
    }
    namespace Transaction {
        class TransactionHandle {
            -driver: &dyn DatabaseDriver
            -committed: bool
            +commit(self) Result
        }
    }
    namespace Connection_Infra {
        class ConnectionFactory {
            <<interface>>
            +open_primary() DbConnection
            +open_domain(config) DbConnection
            +provider() DbProvider
        }
        class DatabaseFactory {
            -db: Arc~Database~
            +new(db) DatabaseFactory
        }
        class DbConnection {
            -inner: Box~dyn Any~
            -provider: DbProvider
            +as_any() &dyn Any
            +provider() DbProvider
        }
    }
    namespace Stores {
        class EmbeddingStore {
            -backend: VectorBackend
            -driver: Arc~dyn DatabaseDriver~
            +from_driver(driver, dim) EmbeddingStore
            +store(ref, vec, model)
            +get(ref) StoredEmbedding
            +search(vec, k) Vec~SimilarityResult~
            +delete(ref)
            +count() usize
        }
        class VectorBackend {
            -conn: Arc~Mutex~Connection~~
            -dim: usize
        }
        class HMemStore {
            -conn: Arc~Mutex~Connection~~
            +from_driver(driver) HMemStore
            +insert(h_mem)
            +query_by_entity(entity) Vec~HMem~
            +update(h_mem)
        }
        class MemoryLoopForwarder {
            -episodic: Arc~EpisodicMemory~
            -semantic: Arc~SemanticMemory~
            +from_connection(conn) Result~Self~
        }
    }
    namespace Types {
        class DbProvider {
            <<enumeration>>
            Sqlite
            Postgres
        }
        class DbConfig {
            <<enumeration>>
            Sqlite { path, passphrase }
            Postgres { url }
        }
    }

    SqliteDriver ..|> DatabaseDriver : implements
    DatabaseFactory ..|> ConnectionFactory : implements
    TransactionHandle o--> DatabaseDriver : borrows &dyn
    EmbeddingStore o--> DatabaseDriver : holds Arc~dyn~
    EmbeddingStore o--> VectorBackend : holds
    HMemStore ..> DatabaseDriver : from_driver extracts conn
    MemoryLoopForwarder ..> HMemStore : creates via from_driver
    MemoryLoopForwarder ..> EmbeddingStore : creates via from_driver
    DatabaseFactory o--> DbConnection : produces
    DbConnection --> DbProvider : typed by
    DbConfig --> DbProvider : selects
```
