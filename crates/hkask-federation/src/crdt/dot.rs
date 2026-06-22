//! CRDT dot — uniquely identifies a write in causal order.
//!
//! No timestamp dependency. Pure causal ordering: `(replica, counter)`.

use crate::ReplicaId;

/// CRDT dot — uniquely identifies a write event.
/// Combined with version vectors to establish causal ordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dot {
    /// Which replica performed the write.
    pub replica: ReplicaId,
    /// Monotonic counter — increments on each write at this replica.
    pub counter: u64,
}

impl Dot {
    /// Create a new dot for the given replica with the given counter.
    pub fn new(replica: ReplicaId, counter: u64) -> Self {
        Self { replica, counter }
    }
}
