//! ID types for hKask entities — re-export facade.
//!
//! G2 Justification: 2 public submodules (core, webid). The id module is a
//! re-export facade — all types live in sub-files with ≤7 public items each.

pub mod core;
pub mod webid;

// Re-export everything for backward compatibility
pub use core::*;
pub use webid::*;
