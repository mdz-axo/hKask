//! hKask CLI — Command-line interface

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

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
