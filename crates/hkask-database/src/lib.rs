//! hkask-database — Provider-agnostic database driver abstraction.
//!
//! Enables hKask to work with SQLite, PostgreSQL, and future database
//! providers. User sovereignty requires database optionality — users
//! must be able to choose their infrastructure.
//!
//! # Architecture
//!
//! ```text
//! Store (HMemStore, EmbeddingStore, ...)
//!   └── Arc<dyn DatabaseDriver>           ← provider-agnostic handle
//!         ├── SqliteDriver(r2d2::Pool)    ← rusqlite + WAL + SQLCipher
//!         └── PostgresDriver(sqlx::PgPool) ← sqlx + pgvector + pgcrypto
//! ```rust,no_run
//!
//! # Provider evolution
//!
//! | Provider     | v0.31 | v0.32 | Backend     | Pool      | Vector     | Encryption |
//! |-------------|-------|-------|-------------|-----------|------------|------------|
//! | SQLite      | ✅    | ✅    | rusqlite    | r2d2 (8)  | sqlite-vec | SQLCipher  |
//! | PostgreSQL  | —     | ✅    | sqlx        | sqlx pool | pgvector   | pgcrypto   |

pub(crate) mod regulation;
pub mod driver;
pub mod encrypt;
pub mod postgres;
pub mod sqlite;
pub mod transaction;
pub mod types;
pub mod value;

pub use driver::DatabaseDriver;
pub use postgres::PostgresDriver;
pub use sqlite::{SqliteDriver, WAL_PRAGMA_BATCH, init_wal_pragmas};
pub use types::DbProvider;
