//! Last-Writer-Wins Map — key-value store with timestamp-based conflict resolution.
//!
//! The one place wall-clock timestamps are used for cross-server conflict resolution.
//! Acceptable because profiles are metadata, not regulation record-grounded observations.

use std::collections::HashMap;
use std::hash::Hash;

use chrono::{DateTime, Utc};

use crate::ReplicaId;

/// An entry in an LWW-Map, carrying a timestamp and replica for conflict resolution.
#[derive(Debug, Clone)]
struct LwwEntry<V> {
    value: V,
    timestamp: DateTime<Utc>,
    replica: ReplicaId,
}

impl<V: Clone> LwwEntry<V> {
    fn merge(&self, other: &Self) -> Self {
        match self.timestamp.cmp(&other.timestamp) {
            std::cmp::Ordering::Greater => self.clone(),
            std::cmp::Ordering::Less => other.clone(),
            std::cmp::Ordering::Equal => {
                // Tiebreak: higher replica_id wins (deterministic, arbitrary)
                if self.replica > other.replica {
                    self.clone()
                } else {
                    other.clone()
                }
            }
        }
    }
}

/// Last-Writer-Wins Map — concurrent writes resolve by highest timestamp.
pub struct LWWMap<K: Hash + Eq + Clone, V: Clone> {
    entries: HashMap<K, LwwEntry<V>>,
}

/// Public entry type for get_entry accessor.
pub struct LwwMapEntry<V> {
    pub value: V,
    pub timestamp: DateTime<Utc>,
    pub replica: ReplicaId,
}

impl<K: Hash + Eq + Clone, V: Clone> LWWMap<K, V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert a value with a timestamp and replica.
    pub fn insert(&mut self, key: K, value: V, timestamp: DateTime<Utc>, replica: ReplicaId) {
        let entry = LwwEntry {
            value,
            timestamp,
            replica,
        };
        self.entries.insert(key, entry);
    }

    /// Get the current value for a key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).map(|e| &e.value)
    }

    /// Get the full entry for a key (value + metadata).
    pub fn get_entry(&self, key: &K) -> Option<LwwMapEntry<V>> {
        self.entries.get(key).map(|e| LwwMapEntry {
            value: e.value.clone(),
            timestamp: e.timestamp,
            replica: e.replica.clone(),
        })
    }

    /// Remove a key from the map.
    pub fn remove(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, e)| (k, &e.value))
    }

    /// Merge another LWW-Map's state. LWW: highest timestamp wins.
    pub fn merge(&mut self, other: &Self) {
        for (key, other_entry) in &other.entries {
            match self.entries.get(key) {
                Some(my_entry) => {
                    let winner = my_entry.merge(other_entry);
                    self.entries.insert(key.clone(), winner);
                }
                None => {
                    self.entries.insert(key.clone(), other_entry.clone());
                }
            }
        }
    }
}

impl<K: Hash + Eq + Clone, V: Clone> Default for LWWMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_timestamp_wins() {
        let mut a: LWWMap<String, String> = LWWMap::new();
        let mut b: LWWMap<String, String> = LWWMap::new();

        a.insert("k".into(), "a-value".into(), Utc::now(), "alpha".into());
        // B writes later
        b.insert(
            "k".into(),
            "b-value".into(),
            Utc::now() + chrono::Duration::seconds(10),
            "beta".into(),
        );

        a.merge(&b);
        assert_eq!(a.get(&"k".into()), Some(&"b-value".to_string()));
    }

    #[test]
    fn equal_timestamp_replica_tiebreak() {
        let t = Utc::now();
        let mut a: LWWMap<String, String> = LWWMap::new();
        let mut b: LWWMap<String, String> = LWWMap::new();

        a.insert("k".into(), "alpha-value".into(), t, "alpha".into());
        b.insert("k".into(), "beta-value".into(), t, "beta".into());

        // "beta" > "alpha" lexicographically → beta wins
        a.merge(&b);
        assert_eq!(a.get(&"k".into()), Some(&"beta-value".to_string()));
    }

    #[test]
    fn merge_commutative_for_distinct_keys() {
        let mut a: LWWMap<String, String> = LWWMap::new();
        let mut b: LWWMap<String, String> = LWWMap::new();

        a.insert("a".into(), "val-a".into(), Utc::now(), "alpha".into());
        b.insert("b".into(), "val-b".into(), Utc::now(), "beta".into());

        let mut a1 = LWWMap::new();
        a1.merge(&a);
        a1.merge(&b);

        let mut b1 = LWWMap::new();
        b1.merge(&b);
        b1.merge(&a);

        assert_eq!(a1.get(&"a".into()), b1.get(&"a".into()));
        assert_eq!(a1.get(&"b".into()), b1.get(&"b".into()));
    }

    #[test]
    fn remove_then_merge_keeps_value() {
        let mut a: LWWMap<String, String> = LWWMap::new();
        let mut b: LWWMap<String, String> = LWWMap::new();

        a.insert("k".into(), "val".into(), Utc::now(), "alpha".into());
        a.remove(&"k".into());
        b.insert("k".into(), "new-val".into(), Utc::now(), "beta".into());

        a.merge(&b);
        assert_eq!(a.get(&"k".into()), Some(&"new-val".to_string()));
    }
}
