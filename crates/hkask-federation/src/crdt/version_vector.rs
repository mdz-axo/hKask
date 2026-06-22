//! Version vector — causal ordering across replicas.
//!
//! A version vector maps each replica to its highest-seen counter.
//! Merge is element-wise MAX. Used by OR-Set, LWW-Map to track causal dependencies.
//! No wall-clock dependency — pure causal ordering.

use std::collections::HashMap;

use crate::ReplicaId;

/// Causal ordering: replica → counter. Merge is element-wise MAX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionVector {
    entries: HashMap<ReplicaId, u64>,
}

impl VersionVector {
    /// Create an empty version vector.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get the counter for a replica (0 if unseen).
    pub fn get(&self, replica: &ReplicaId) -> u64 {
        self.entries.get(replica).copied().unwrap_or(0)
    }

    /// Increment the counter for a replica and return the new value.
    pub fn increment(&mut self, replica: ReplicaId) -> u64 {
        let next = self.get(&replica) + 1;
        self.entries.insert(replica, next);
        next
    }

    /// Does `self` dominate `other`? ∀r: self[r] ≥ other[r] and ∃r: self[r] > other[r].
    pub fn dominates(&self, other: &VersionVector) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let mut strictly_greater = false;
        // Check all keys in both vectors
        let all_keys: std::collections::HashSet<_> =
            self.entries.keys().chain(other.entries.keys()).collect();
        for key in all_keys {
            let mine = self.get(key);
            let theirs = other.get(key);
            if mine < theirs {
                return false;
            }
            if mine > theirs {
                strictly_greater = true;
            }
        }
        strictly_greater
    }

    /// Merge: element-wise MAX of both vectors.
    pub fn merge(&self, other: &VersionVector) -> VersionVector {
        let mut merged = self.entries.clone();
        for (replica, counter) in &other.entries {
            let existing = merged.entry(replica.clone()).or_insert(0);
            *existing = (*existing).max(*counter);
        }
        VersionVector { entries: merged }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vv(pairs: &[(&str, u64)]) -> VersionVector {
        let mut v = VersionVector::new();
        for (r, c) in pairs {
            v.entries.insert(r.to_string(), *c);
        }
        v
    }

    #[test]
    fn merge_is_commutative() {
        let a = vv(&[("alpha", 3), ("beta", 1)]);
        let b = vv(&[("alpha", 2), ("beta", 4)]);
        assert_eq!(a.merge(&b), b.merge(&a));
    }

    #[test]
    fn merge_is_associative() {
        let a = vv(&[("alpha", 3)]);
        let b = vv(&[("beta", 4)]);
        let c = vv(&[("alpha", 2), ("gamma", 1)]);
        assert_eq!(a.merge(&b).merge(&c), a.merge(&b.merge(&c)));
    }

    #[test]
    fn merge_is_idempotent() {
        let a = vv(&[("alpha", 3), ("beta", 1)]);
        assert_eq!(a.merge(&a), a);
    }

    #[test]
    fn dominates_strict_greater() {
        let a = vv(&[("alpha", 3)]);
        let same = vv(&[("alpha", 3)]);
        assert!(!a.dominates(&same)); // equal → not strictly dominating
    }

    #[test]
    fn dominates_self_is_not_strict() {
        let a = vv(&[("alpha", 3)]);
        assert!(!a.dominates(&a)); // identical → not strictly dominating
    }

    #[test]
    fn dominates_transitive() {
        let a = vv(&[("alpha", 5)]);
        let b = vv(&[("alpha", 3)]);
        let c = vv(&[("alpha", 1)]);
        assert!(a.dominates(&b));
        assert!(b.dominates(&c));
    }

    #[test]
    fn merge_advances_both() {
        let a = vv(&[("alpha", 3)]);
        let b = vv(&[("beta", 4)]);
        let m = a.merge(&b);
        assert!(m.dominates(&a));
        assert!(m.dominates(&b));
    }

    #[test]
    fn empty_dominated_by_all() {
        let empty = VersionVector::new();
        let nonempty = vv(&[("alpha", 1)]);
        assert!(!empty.dominates(&nonempty));
    }
}
