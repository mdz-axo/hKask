//! hKask Storage Core — SQLite foundation for all storage crates.
//!
//! Provides the `Database` connection manager, driver store macros,
//! and path sanitization.
//! This crate has no dependencies on any service or domain crate
//! (only `hkask-types` for `InfrastructureError`).

#[macro_use]
pub mod store_macros;
pub mod database;
pub mod security;

pub use database::{Database, DatabaseError, check_passphrase, open_database, open_or_repair};
pub use security::sanitize_path;
pub use store_macros::DatabaseDriverTrait;
