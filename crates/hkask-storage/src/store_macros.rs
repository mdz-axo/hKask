//! Store trait and declarative macros for store boilerplate elimination
//!
//! `Store` provides the shared `conn_arc()` and `lock_conn()` methods
//! over `Arc<Mutex<Connection>>`. Every store struct implements this trait.
//! `define_store!` generates the struct + `Store` impl for each store.
//! `impl_from_rusqlite!` generates the canonical `From<rusqlite::Error>` impl.

use hkask_types::InfrastructureError;
use std::sync::{Arc, Mutex, MutexGuard};

// P4.3: `now_rfc3339` lives in `hkask-types` (the foundation crate) so
// that non-storage crates (CLI, agents) can also use it without
// pulling in the entire storage dependency tree. Re-export it from this
// module for backward compatibility with `hkask_storage::now_rfc3339`.
// The actual implementation is in `hkask_types::time`.
//
// (Kept as a re-export inside the macro module so the path
// `crate::store_macros::now_rfc3339` still resolves for any caller that
// reaches for the canonical implementation directly.)
pub use hkask_types::time::now_rfc3339;

/// Shared trait for all SQLite-backed stores.
///
/// Provides the standard `conn_arc()` and `lock_conn()` methods over
/// `Arc<Mutex<Connection>>`. Every store struct implements this trait.
///
/// `lock_conn()` is a required method (not a default) because the
/// `MutexGuard` borrows from the `Mutex` inside the `Arc`, and the
/// borrow checker cannot verify safety through a default-body call
/// to `conn_arc()` — the temporary `Arc` would be dropped while the
/// guard still references the `Mutex`.
pub trait Store {
    /// Get a clone of the inner connection Arc for direct SQL access.
    fn conn_arc(&self) -> Arc<Mutex<rusqlite::Connection>>;

    /// Acquire the mutex lock on the shared connection.
    ///
    /// Returns `InfrastructureError::LockPoisoned` if another thread
    /// panicked while holding the lock.
    fn lock_conn(&self) -> Result<MutexGuard<'_, rusqlite::Connection>, InfrastructureError>;
}

/// Define a store struct with the standard `Arc<Mutex<Connection>>` pattern.
///
/// Generates:
/// - `pub struct $name { conn: Arc<Mutex<Connection>> }`
/// - `pub fn new(conn: Arc<Mutex<Connection>>) -> Self`
/// - `impl Store for $name` (provides `conn_arc()` and `lock_conn()`)
///
/// # Example
/// ```ignore
/// define_store!(TripleStore);
/// define_store!(AgentRegistryStore);
/// ```
#[macro_export]
macro_rules! define_store {
    ($name:ident) => {
        /// SQLite-backed store sharing an encrypted connection.
        #[derive(Clone)]
        pub struct $name {
            conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
        }

        impl $name {
            /// Create a new store backed by the given connection.
            pub fn new(conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
                Self { conn }
            }
        }

        impl $crate::Store for $name {
            fn conn_arc(&self) -> std::sync::Arc<std::sync::Mutex<rusqlite::Connection>> {
                std::sync::Arc::clone(&self.conn)
            }

            fn lock_conn(
                &self,
            ) -> std::result::Result<
                std::sync::MutexGuard<'_, rusqlite::Connection>,
                hkask_types::InfrastructureError,
            > {
                $crate::lock_helpers::lock_mutex(&self.conn)
            }
        }
    };
}

/// Implement `From<rusqlite::Error>` for a store error type, mapping to
/// `XxxError::Infra(InfrastructureError::Database(e.to_string()))`.
///
/// This eliminates the copy-pasted `From<rusqlite::Error>` impl that every store
/// defines identically.
///
/// # Example
/// ```ignore
/// impl_from_rusqlite!(TripleError, Infra);
/// impl_from_rusqlite!(GoalRepositoryError, Infra);
/// ```
#[macro_export]
macro_rules! impl_from_rusqlite {
    ($error:ident, $infra_variant:ident) => {
        impl From<rusqlite::Error> for $error {
            fn from(e: rusqlite::Error) -> Self {
                $error::$infra_variant(e.into())
            }
        }
    };
}

/// Implement `From<serde_json::Error>` for a store error type, mapping to
/// `XxxError::Infra(InfrastructureError::from(e))`.
///
/// Six stores currently hand-write this identical impl. The macro eliminates
/// that boilerplate (Fowler C2: Duplicated Code).
///
/// # Example
/// ```ignore
/// impl_from_serde_json!(TripleError, Infra);
/// ```
#[macro_export]
macro_rules! impl_from_serde_json {
    ($error:ident, $infra_variant:ident) => {
        impl From<serde_json::Error> for $error {
            fn from(e: serde_json::Error) -> Self {
                $error::$infra_variant(hkask_types::InfrastructureError::from(e))
            }
        }
    };
}

/// Collect mapped rows, propagating the **first error** instead of logging and skipping.
///
/// Use for targeted single-result queries (e.g., `get_by_id`) where a malformed
/// row is a hard error, not graceful degradation. The two-function form matches
/// the `collect_rows!` signature for `row_mapper → converter` pipelines.
///
/// # Example
/// ```ignore
/// // Note: no `?` on the call — the macro propagates errors internally
/// // via `?`, returning `Vec<T>` on success or early-returning from
/// // the enclosing function on the first error.
/// let triples = collect_rows_strict!(
///     stmt,
///     rusqlite::params![id],
///     TripleStore::row_to_triple_row,
///     TripleStore::row_to_triple
/// );
/// Ok(triples.into_iter().next())
/// ```
#[macro_export]
macro_rules! collect_rows_strict {
    ($stmt:expr, $params:expr, $mapper:expr, $convert:expr) => {{
        let mapped: Vec<std::result::Result<_, rusqlite::Error>> =
            $stmt.query_map($params, $mapper)?.collect();
        let mut results = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            let row = row_result?;
            let item = $convert(row)?;
            results.push(item);
        }
        results
    }};
}

/// Collect mapped rows into a result vector with graceful error logging.
///
/// Eliminates the repeated `collect → for row_result in mapped { match Ok/Err }`
/// pattern that appears ~15 times across all stores (Fowler C1: Duplicated Code).
///
/// The closure `$mapper` receives a `&rusqlite::Row` and should return
/// `Result<T, rusqlite::Error>`. Each row is mapped and collected; rows that
/// fail mapping or domain conversion are logged as warnings and skipped
/// rather than failing the entire query.
///
/// # Example
/// ```ignore
/// let triples: Vec<Triple> = collect_rows!(
///     stmt,
///     rusqlite::params![entity],
///     |row: &rusqlite::Row<'_>| -> rusqlite::Result<TripleRow> {
///         Ok(TripleRow { ... })
///     },
///     TripleStore::row_to_triple
/// )?;
/// ```
#[macro_export]
macro_rules! collect_rows {
    ($stmt:expr, $params:expr, $mapper:expr, $convert:expr) => {{
        let mapped: Vec<std::result::Result<_, rusqlite::Error>> =
            $stmt.query_map($params, $mapper)?.collect();
        let mut results = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match $convert(row) {
                    Ok(item) => results.push(item),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed database row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }
        results
    }};
    ($stmt:expr, $params:expr, $mapper:expr) => {{
        let mapped: Vec<std::result::Result<_, rusqlite::Error>> =
            $stmt.query_map($params, $mapper)?.collect();
        let mut results = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(item) => results.push(item),
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }
        results
    }};
}
