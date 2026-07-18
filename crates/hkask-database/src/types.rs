//! Core database types — provider enum and error type.

use hkask_types::InfrastructureError;
use hkask_types::error::DatabaseErrorKind;

/// Supported database providers.
///
/// New providers are added as enum variants. The `DatabaseDriver` trait
/// dispatches to the correct implementation at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DbProvider {
    /// SQLite / SQLCipher via rusqlite (default, stable).
    Sqlite,
    /// PostgreSQL via sqlx + pgvector.
    Postgres,
}

impl std::fmt::Display for DbProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite => write!(f, "sqlite"),
            Self::Postgres => write!(f, "postgres"),
        }
    }
}

/// Database operation errors — provider-agnostic.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database: {0}")]
    Database(String),

    #[error("constraint violation: {0}")]
    Constraint(String),

    #[error("connection: {0}")]
    Connection(String),

    #[error("serialization: {0}")]
    Serialization(String),

    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("migration: {0}")]
    Migration(String),
}

impl From<DbError> for InfrastructureError {
    fn from(e: DbError) -> Self {
        match &e {
            DbError::Connection(_) => InfrastructureError::Database {
                message: e.to_string(),
                kind: DatabaseErrorKind::Connection,
            },
            DbError::Constraint(_) => InfrastructureError::Database {
                message: e.to_string(),
                kind: DatabaseErrorKind::Constraint,
            },
            DbError::Migration(_) => InfrastructureError::Database {
                message: e.to_string(),
                kind: DatabaseErrorKind::Migration,
            },
            _ => InfrastructureError::Database {
                message: e.to_string(),
                kind: DatabaseErrorKind::Other,
            },
        }
    }
}

impl From<InfrastructureError> for DbError {
    fn from(e: InfrastructureError) -> Self {
        DbError::Database(e.to_string())
    }
}
