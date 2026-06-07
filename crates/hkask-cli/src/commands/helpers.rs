//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

use hkask_storage::{Database, lock_mutex};
use std::path::Path;

/// Unwrap a `Result` or print an error message and exit.
pub fn or_exit<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {}", label, e);
            std::process::exit(1);
        }
    }
}

/// Write content to a file or print to stdout.
pub fn write_or_print(content: &str, output: Option<&Path>, label: &str) {
    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, content) {
                eprintln!("Failed to write {}: {}", label, e);
                std::process::exit(1);
            }
            println!("{} written to {}", label, path.display());
        }
        None => println!("{}", content),
    }
}

/// Open the user store with the registry database.
pub fn open_user_store() -> std::sync::Arc<std::sync::Mutex<hkask_storage::user_store::UserStore>> {
    use super::config::{registry_db_path, resolve_db_passphrase};

    let db_path = registry_db_path();
    let passphrase = or_exit(resolve_db_passphrase(), "Failed to resolve DB passphrase");

    let db = or_exit(
        if db_path == ":memory:" {
            Database::in_memory()
        } else {
            Database::open(&db_path, &passphrase)
        },
        "Failed to open user database",
    );

    let store = hkask_storage::user_store::UserStore::new(db.conn_arc());
    let store = std::sync::Arc::new(std::sync::Mutex::new(store));
    or_exit(
        lock_mutex(&store).and_then(|mut g| {
            g.initialize_schema()
                .map_err(|e| hkask_types::InfrastructureError::Database(e.to_string()))
        }),
        "Failed to initialize user store schema",
    );
    store
}

/// Run an async future on the tokio runtime and exit on error.
///
/// Shorthand for `or_exit(rt.block_on($fut), $label)`.
/// Eliminates the repeated `or_exit(rt.block_on(...), "...")` boilerplate
/// across command handlers.
#[macro_export]
macro_rules! block_on {
    ($rt:expr, $fut:expr, $label:literal) => {
        $crate::commands::helpers::or_exit($rt.block_on($fut), $label)
    };
}
