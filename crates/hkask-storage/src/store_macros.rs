//! Store trait and declarative macros for store boilerplate elimination
//!
//! `Store` provides the shared `conn_arc()` and `lock_conn()` methods
//! over `Arc<Mutex<Connection>>`. Every store struct implements this trait.
//! `define_store!` generates the struct + `Store` impl for each store.
//! `impl_from_rusqlite!` generates the canonical `From<rusqlite::Error>` impl.

use crate::lock_helpers::lock_mutex;
use hkask_types::InfrastructureError;
use std::sync::{Arc, Mutex, MutexGuard};

/// Produce an RFC 3339 timestamp string for the current moment.
///
/// Consolidates the repeated `Utc::now().to_rfc3339()` pattern across
/// stores and agents (P4.3).
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

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
            ) -> Result<
                std::sync::MutexGuard<'_, rusqlite::Connection>,
                hkask_types::InfrastructureError,
            > {
                $crate::lock_helpers::lock_mutex(&self.conn)
            }
        };
    };
}

/// Define a store struct with an optional CAS port for write-through.
///
/// Generates the same struct as `define_store!` plus a `cas_port` field
/// and a `with_cas()` builder method for injecting a `GitCASPort`.
///
/// # Example
/// ```ignore
/// define_store_cas!(AgentRegistryStore);
/// ```
#[macro_export]
macro_rules! define_store_cas {
    ($name:ident) => {
        /// SQLite-backed store sharing an encrypted connection, with optional CAS write-through.
        #[derive(Clone)]
        pub struct $name {
            conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
            cas_port: Option<std::sync::Arc<dyn hkask_types::ports::git_cas::GitCASPort>>,
        }

        impl $name {
            /// Create a new store backed by the given connection (no CAS port).
            pub fn new(conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
                Self {
                    conn,
                    cas_port: None,
                }
            }

            /// Attach a CAS port for write-through. Consumes and returns self.
            #[must_use = "builder returns the configured store"]
            pub fn with_cas(
                mut self,
                port: std::sync::Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
            ) -> Self {
                self.cas_port = Some(port);
                self
            }
        }

        impl $crate::Store for $name {
            fn conn_arc(&self) -> std::sync::Arc<std::sync::Mutex<rusqlite::Connection>> {
                std::sync::Arc::clone(&self.conn)
            }

            fn lock_conn(
                &self,
            ) -> Result<
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
                $error::$infra_variant(hkask_types::InfrastructureError::Database(e.to_string()))
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
/// impl_from_serde_json!(NuEventError, Infra);
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
