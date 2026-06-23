//! Loop 5: Curation — metacognitive observer (thin re-export module).
//!
//! Re-exports `CuratorHandle` and `CuratorDirective` from `hkask-types::curator`.
//!
//! Essential subloops:
//! - 5.1 Escalation Routing (ROUTE) — signal → classify → deliver to consumer
//! - 5.2 Metacognitive Adaptation (ADAPT) — outcome → compare to desired → adjust parameter

// Re-export CuratorHandle and CuratorDirective from their canonical home.
pub use hkask_types::curator::{CuratorDirective, CuratorHandle};
