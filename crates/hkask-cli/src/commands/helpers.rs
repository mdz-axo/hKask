//! Shared helper functions for CLI command handlers
//!
//! Utility functions used across multiple command modules for error handling,
//! output, and common setup.

use std::path::Path;

/// Unwrap a `Result` or print an error message and exit.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  result is a `Result<T, E>`; label is a human-readable context string
/// post: returns Ok value or prints "{label}: {error}" to stderr and exits with code 1
pub fn or_exit<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {}", label, e);
            std::process::exit(1);
        }
    }
}

/// Build an AgentService from environment config. Shared across all commands
/// that previously duplicated `build_service_context()`.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  service config must be resolvable from environment
/// post: builds and returns an AgentService from environment config; exits on failure
pub fn build_service_context() -> hkask_services::AgentService {
    let config = or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve service config",
    );
    let rt = tokio::runtime::Runtime::new().expect("runtime should start");
    or_exit(
        rt.block_on(hkask_services::AgentService::build(config)),
        "Failed to build service context",
    )
}

/// Write content to a file or print to stdout.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  content is a non-empty string; output is an optional file path; label is a human-readable description
/// post: writes content to the file path if provided, or prints to stdout; exits on write failure
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

pub fn resolve_user_webid() -> hkask_types::WebID {
    if let Ok(uuid_str) = std::env::var("HKASK_WEBID")
        && let Ok(webid) = uuid_str.parse::<hkask_types::WebID>()
    {
        return webid;
    }
    hkask_types::WebID::from_persona(b"cli-user")
}
