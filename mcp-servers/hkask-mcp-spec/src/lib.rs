//! F-SYN-020: the `hkask-mcp-spec` library target.
//!
//! This file exists so that integration tests in `tests/` can
//! `use hkask_mcp_spec::types::...` to import the request types
//! for fuzz testing. The actual implementation lives in
//! `src/main.rs`; this lib just re-exports the public surface.

pub mod types;
