//! Store trait and declarative macros for store boilerplate elimination
//!
//! `Store` provides the shared `conn_arc()` and `lock_conn()` methods
//! over `Arc<Mutex<Connection>>`. Every store struct implements this trait.
//! `define_store!` generates the struct + `Store` impl for each store.
//! `impl_from_rusqlite!` generates the canonical `From<rusqlite::Error>` impl.

use hkask_types::InfrastructureError;
use std::sync::{Arc, Mutex, MutexGuard};

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
                self.conn
                    .lock()
                    .map_err(|_| hkask_types::InfrastructureError::LockPoisoned)
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
