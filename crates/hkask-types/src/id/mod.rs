//! ID types for hKask entities — re-export facade.
//!
pub mod core;
pub mod webid;

// Re-export everything for backward compatibility
pub use core::*;
pub use webid::*;
