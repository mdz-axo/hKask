//! hKask CLI — Command-line interface

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

/// Block on a future using a tokio Runtime, panicking with an error message on failure.
#[macro_export]
macro_rules! block_on {
    ($rt:expr, $future:expr, $msg:expr $(,)?) => {
        match $rt.block_on($future) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", $msg, e);
                std::process::exit(1);
            }
        }
    };
}

pub mod archival;
pub mod cli;
pub mod cloud;
pub mod commands;
pub mod experience;
pub mod onboarding;
pub mod onboarding_session;
pub mod repl;
#[cfg(feature = "tui")]
pub mod transcript_viewer;
