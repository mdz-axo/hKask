//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

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
