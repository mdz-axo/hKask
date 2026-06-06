//! Standing Session Port — Hexagonal boundary for standing session persistence
//
//! Canonical types (`SessionRecord`, `MessageRecord`, `SessionStoreError`)
//! live in `hkask_types::ports`. This module re-exports them for convenience.

pub use hkask_types::ports::{
    MessageRecord, SessionRecord, SessionStoreError as StandingSessionPortError,
};
